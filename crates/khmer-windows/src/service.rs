use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::Once;

use khmer_core::{Engine, Mode, Romanizer, BACKSPACE};
use windows::core::{implement, ComObject, Error, IUnknownImpl, Interface, Result, GUID};
use windows::Win32::Foundation::{BOOL, E_FAIL, E_INVALIDARG, FALSE, LPARAM, RECT, TRUE, WPARAM};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, MapVirtualKeyW, MAPVK_VK_TO_VSC};
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, IEnumTfDisplayAttributeInfo, ITfCategoryMgr, ITfCompartmentMgr, ITfComposition, ITfCompositionSink,
    ITfCompositionSink_Impl, ITfContext, ITfContextComposition, ITfDisplayAttributeInfo,
    ITfDisplayAttributeProvider, ITfDisplayAttributeProvider_Impl, ITfDocumentMgr,
    ITfInputProcessorProfileMgr, ITfInsertAtSelection, ITfKeyEventSink, ITfKeyEventSink_Impl, ITfKeystrokeMgr, ITfSource, ITfTextInputProcessorEx, ITfTextInputProcessorEx_Impl, ITfTextInputProcessor_Impl,
    ITfThreadMgr, ITfThreadMgrEventSink, ITfThreadMgrEventSink_Impl, GUID_COMPARTMENT_EMPTYCONTEXT,
    GUID_COMPARTMENT_KEYBOARD_DISABLED, GUID_TFCAT_TIP_KEYBOARD, INSERT_TEXT_AT_SELECTION_FLAGS,
    TF_IAS_QUERYONLY, TF_INPUTPROCESSORPROFILE, TF_SD_READONLY,
    TF_TMAE_SECUREMODE,
};

use crate::candidates::{CandidateWindow, View};
use crate::my_words;
use crate::display::{clear_attribute, set_attribute, AttributeList, InputAttribute};
use crate::edit::{caret_place, caret_to_end, edit, text_place};
use crate::keys::{self, KeyAction, Modifiers};
use crate::{DISPLAY_ATTRIBUTE_INPUT, PROFILE_KHMER};

pub(crate) fn guard<T>(action: impl FnOnce() -> Result<T>) -> Result<T> {
    catch_unwind(AssertUnwindSafe(action)).unwrap_or_else(|_| Err(Error::from(E_FAIL)))
}

fn utf16(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn warm_up() {
    static STARTED: Once = Once::new();
    STARTED.call_once(|| {
        let _ = std::thread::Builder::new()
            .name("khmer keyboard dictionary".into())
            .spawn(|| {
                let _ = catch_unwind(|| {
                    Romanizer::builtin();
                });
            });
    });
}

#[implement(
    ITfTextInputProcessorEx,
    ITfKeyEventSink,
    ITfCompositionSink,
    ITfThreadMgrEventSink,
    ITfDisplayAttributeProvider
)]
pub struct TextService {
    inner: RefCell<Inner>,
}

struct Inner {
    thread_mgr: Option<ITfThreadMgr>,
    client_id: u32,
    profiles: Option<ITfInputProcessorProfileMgr>,
    thread_sink_cookie: Option<u32>,
    display_atom: u32,

    engine: Engine,
    mode: Mode,
    composition: Option<ITfComposition>,
    context: Option<ITfContext>,
    preedit: String,
    highlight: usize,
    anchor: RECT,

    window: Option<CandidateWindow>,
    show_window: bool,

    test_mode: Option<Mode>,
    test_modifiers: Option<Modifiers>,
    /// What a test typed would have added to learned.txt.
    test_corrections: Vec<(String, String)>,
}

impl TextService {
    pub fn new() -> TextService {
        TextService {
            inner: RefCell::new(Inner {
                thread_mgr: None,
                client_id: 0,
                profiles: None,
                thread_sink_cookie: None,
                display_atom: 0,
                engine: Engine::new(),
                mode: Mode::Khmer,
                composition: None,
                context: None,
                preedit: String::new(),
                highlight: 0,
                anchor: RECT::default(),
                window: None,
                show_window: true,
                test_mode: None,
                test_modifiers: None,
                test_corrections: Vec::new(),
            }),
        }
    }

    pub fn for_tests(mode: Mode) -> TextService {
        let service = TextService::new();
        {
            let mut inner = service.inner.borrow_mut();
            inner.show_window = false;
            inner.test_mode = Some(mode);
            inner.test_modifiers = Some(Modifiers::default());
        }
        service
    }

    pub fn set_test_mode(&self, mode: Mode) {
        self.inner.borrow_mut().test_mode = Some(mode);
    }

    pub fn set_test_modifiers(&self, held: Modifiers) {
        self.inner.borrow_mut().test_modifiers = Some(held);
    }

    /// The corrections noted so far, in a test (a real service writes them
    /// to learned.txt instead).
    pub fn corrections_for_tests(&self) -> Vec<(String, String)> {
        self.inner.borrow().test_corrections.clone()
    }

    pub fn suggestions(&self) -> (Vec<String>, usize) {
        let inner = self.inner.borrow();
        (inner.engine.candidates(), inner.highlight)
    }

    pub fn attach(&self, thread_mgr: &ITfThreadMgr, client_id: u32) {
        let atom = unsafe {
            CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)
                .and_then(|categories| categories.RegisterGUID(&DISPLAY_ATTRIBUTE_INPUT))
                .unwrap_or(0)
        };
        let mut inner = self.inner.borrow_mut();
        inner.thread_mgr = Some(thread_mgr.clone());
        inner.client_id = client_id;
        inner.display_atom = atom;
    }

    fn is_composing(&self) -> bool {
        self.inner.borrow().composition.is_some()
    }

    fn abandon(&self) {
        let window = {
            let mut inner = self.inner.borrow_mut();
            inner.composition = None;
            inner.context = None;
            inner.preedit.clear();
            inner.highlight = 0;
            inner.engine.reset();
            inner.window.take()
        };
        self.put_back(window, |window| window.hide());
    }

    fn put_back(&self, window: Option<CandidateWindow>, action: impl FnOnce(&mut CandidateWindow)) {
        if let Some(mut window) = window {
            action(&mut window);
            self.inner.borrow_mut().window = Some(window);
        }
    }

    fn modifiers(&self) -> Modifiers {
        if let Some(held) = self.inner.borrow().test_modifiers {
            return held;
        }
        let down = |key: u16| unsafe { GetKeyState(i32::from(key)) } < 0;
        Modifiers {
            shift: down(keys::VK_SHIFT),
            ctrl: down(keys::VK_CONTROL),
            alt: down(keys::VK_MENU),
            caps: unsafe { GetKeyState(i32::from(keys::VK_CAPITAL)) } & 1 != 0,
        }
    }

    fn selected_mode(&self) -> Mode {
        let (test_mode, profiles) = {
            let inner = self.inner.borrow();
            (inner.test_mode, inner.profiles.clone())
        };
        if let Some(mode) = test_mode {
            return mode;
        }
        let mut profile = TF_INPUTPROCESSORPROFILE::default();
        let found = profiles
            .map(|profiles| unsafe { profiles.GetActiveProfile(&GUID_TFCAT_TIP_KEYBOARD, &mut profile) }.is_ok())
            .unwrap_or(false);
        if found && profile.guidProfile == PROFILE_KHMER {
            Mode::Khmer
        } else {
            Mode::Romanized
        }
    }

    fn write(
        &self,
        ec: u32,
        context: &ITfContext,
        sink: &ITfCompositionSink,
        commit: &str,
        preedit: &str,
    ) -> Result<()> {
        let (mut composition, atom, shown) = {
            let mut inner = self.inner.borrow_mut();
            (
                inner.composition.take(),
                inner.display_atom,
                std::mem::take(&mut inner.preedit),
            )
        };
        unsafe {
            if !commit.is_empty() {
                match composition.take() {
                    Some(finished) => {
                        let range = finished.GetRange()?;
                        if commit != shown {
                            range.SetText(ec, 0, &utf16(commit))?;
                        }
                        clear_attribute(ec, context, &range);
                        caret_to_end(ec, context, &range)?;
                        finished.EndComposition(ec)?;
                    }
                    None => {
                        let insert: ITfInsertAtSelection = context.cast()?;
                        let range =
                            insert.InsertTextAtSelection(ec, INSERT_TEXT_AT_SELECTION_FLAGS(0), &utf16(commit))?;
                        caret_to_end(ec, context, &range)?;
                    }
                }
            }
            if preedit.is_empty() {
                if let Some(emptied) = composition.take() {
                    let range = emptied.GetRange()?;
                    range.SetText(ec, 0, &[])?;
                    clear_attribute(ec, context, &range);
                    emptied.EndComposition(ec)?;
                }
                self.inner.borrow_mut().context = None;
                return Ok(());
            }
            let composition = match composition {
                Some(open) => open,
                None => {
                    let insert: ITfInsertAtSelection = context.cast()?;
                    let place = insert.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?;
                    let compositions: ITfContextComposition = context.cast()?;
                    compositions.StartComposition(ec, &place, sink)?
                }
            };
            let range = composition.GetRange()?;
            range.SetText(ec, 0, &utf16(preedit))?;
            set_attribute(ec, context, &range, atom);
            caret_to_end(ec, context, &range)?;
            let anchor = text_place(ec, context, &range);

            let mut inner = self.inner.borrow_mut();
            inner.composition = Some(composition);
            inner.context = Some(context.clone());
            inner.preedit = preedit.to_string();
            inner.anchor = anchor.unwrap_or_default();
        }
        Ok(())
    }
}

impl Default for TextService {
    fn default() -> Self {
        TextService::new()
    }
}

fn is_disabled(context: &ITfContext) -> bool {
    unsafe {
        if let Ok(status) = context.GetStatus() {
            if status.dwDynamicFlags & TF_SD_READONLY != 0 {
                return true;
            }
        }
        let Ok(compartments) = context.cast::<ITfCompartmentMgr>() else {
            return false;
        };
        [GUID_COMPARTMENT_KEYBOARD_DISABLED, GUID_COMPARTMENT_EMPTYCONTEXT]
            .iter()
            .any(|guid| {
                compartments
                    .GetCompartment(guid)
                    .and_then(|compartment| compartment.GetValue())
                    .map(|value| i32::try_from(&value).unwrap_or(0) != 0)
                    .unwrap_or(false)
            })
    }
}

fn apply(this: &ComObject<TextService>, context: &ITfContext, commit: String, preedit: String) {
    // A suggestion picked instead of the top one: note the words it
    // corrected, for the user to share later if they want to.
    let corrections = {
        let mut inner = this.inner.borrow_mut();
        let corrections = inner.engine.take_corrections();
        if inner.test_mode.is_some() {
            inner.test_corrections.extend(corrections);
            Vec::new()
        } else {
            corrections
        }
    };
    my_words::remember(&corrections);
    let client_id = this.inner.borrow().client_id;
    let service = this.clone();
    let target = context.clone();
    let result = edit(client_id, context, move |ec| {
        let sink: ITfCompositionSink = service.to_interface();
        let written = service.write(ec, &target, &sink, &commit, &preedit);
        if written.is_err() {
            service.abandon();
        }
        refresh_window(&service);
        written
    });
    if result.is_err() {
        this.abandon();
    }
}

fn finish(this: &ComObject<TextService>) {
    let (context, commit) = {
        let mut inner = this.inner.borrow_mut();
        if inner.composition.is_none() {
            return;
        }
        inner.highlight = 0;
        (inner.context.clone(), inner.engine.commit().commit)
    };
    match context {
        Some(context) => apply(this, &context, commit, String::new()),
        None => this.abandon(),
    }
}

fn refresh_window(this: &ComObject<TextService>) {
    let (view, mut anchor, window, allowed) = {
        let mut inner = this.inner.borrow_mut();
        let items = if inner.mode == Mode::Romanized && inner.composition.is_some() {
            inner.engine.candidates()
        } else {
            Vec::new()
        };
        let view = View {
            header: inner.engine.typed(),
            items,
            highlight: inner.highlight,
            buttons: if my_words::editor().is_some() {
                vec!["+ Add my word".to_string(), "Wrong word?".to_string()]
            } else {
                Vec::new()
            },
            hover: None,
        };
        (view, inner.anchor, inner.window.take(), inner.show_window)
    };
    if view.items.is_empty() || !allowed {
        this.put_back(window, |window| window.hide());
        return;
    }
    let window = window.or_else(|| {
        let mut window = CandidateWindow::new(crate::module_instance())?;
        let service = this.clone();
        window.on_pick(Rc::new(move |index| {
            let _ = guard(|| {
                pick(&service, index);
                Ok(())
            });
        }));
        let service = this.clone();
        window.on_button(Rc::new(move |button| {
            let _ = guard(|| {
                let (typed, got) = {
                    let inner = service.inner.borrow();
                    (inner.engine.typed(), inner.engine.candidates().into_iter().next().unwrap_or_default())
                };
                // The last word typed is the one the user most likely means.
                let spelling = typed.split_whitespace().last().unwrap_or("").to_string();
                if button == 0 {
                    my_words::open_editor(&my_words::Request::Add { spelling });
                } else {
                    my_words::open_editor(&my_words::Request::Report { spelling, typed, got });
                }
                Ok(())
            });
        }));
        Some(window)
    });
    if anchor == RECT::default() {
        anchor = caret_place().unwrap_or_default();
    }
    this.put_back(window, |window| window.show(view, anchor));
}

fn pick(this: &ComObject<TextService>, index: usize) {
    let (context, output) = {
        let mut inner = this.inner.borrow_mut();
        if inner.composition.is_none() {
            return;
        }
        inner.highlight = 0;
        (inner.context.clone(), inner.engine.select(index))
    };
    if let Some(context) = context {
        apply(this, &context, output.commit, output.preedit);
    }
}

fn on_key(
    this: &ComObject<TextService>,
    context: Option<&ITfContext>,
    wparam: WPARAM,
    lparam: LPARAM,
    test: bool,
) -> Result<BOOL> {
    let Some(context) = context else {
        return Ok(FALSE);
    };
    if is_disabled(context) {
        return Ok(FALSE);
    }

    let mode = this.selected_mode();
    if mode != this.inner.borrow().mode {
        finish(this);
        if mode == Mode::Romanized {
            warm_up();
        }
        let mut inner = this.inner.borrow_mut();
        inner.engine.set_mode(mode);
        inner.mode = mode;
    }

    let vk = (wparam.0 & 0xFFFF) as u16;
    let mut scan = ((lparam.0 >> 16) & 0xFF) as u8;
    if scan == 0 {
        scan = unsafe { MapVirtualKeyW(u32::from(vk), MAPVK_VK_TO_VSC) } as u8;
    }
    let composing = this.is_composing();
    let action = keys::action(mode, composing, vk, scan, this.modifiers());

    if let KeyAction::Pass { forget } = action {
        if composing {
            if mode == Mode::Khmer {
                finish(this);
            }
        } else if forget {
            this.inner.borrow_mut().engine.reset();
        }
        return Ok(FALSE);
    }
    if test {
        return Ok(TRUE);
    }
    // A word the user just added works at once. Tests type with the built-in
    // words only, whatever this PC's user added.
    if mode == Mode::Romanized && this.inner.borrow().test_mode.is_none() {
        crate::my_words::refresh();
    }

    let target = this.inner.borrow().context.clone().unwrap_or_else(|| context.clone());
    match action {
        KeyAction::Text(text) => {
            let (commit, preedit) = {
                let mut inner = this.inner.borrow_mut();
                let mut commit = String::new();
                let mut preedit = String::new();
                for key in text.chars() {
                    let output = inner.engine.on_key(key);
                    commit.push_str(&output.commit);
                    preedit = output.preedit;
                }
                inner.highlight = 0;
                (commit, preedit)
            };
            apply(this, &target, commit, preedit);
        }
        KeyAction::Space => {
            let output = {
                let mut inner = this.inner.borrow_mut();
                let highlight = std::mem::take(&mut inner.highlight);
                if highlight > 0 {
                    inner.engine.select(highlight)
                } else {
                    inner.engine.on_key(' ')
                }
            };
            apply(this, &target, output.commit, output.preedit);
        }
        KeyAction::Backspace => {
            let output = {
                let mut inner = this.inner.borrow_mut();
                inner.highlight = 0;
                inner.engine.on_key(BACKSPACE)
            };
            apply(this, &target, output.commit, output.preedit);
        }
        KeyAction::Commit => {
            let output = {
                let mut inner = this.inner.borrow_mut();
                let highlight = std::mem::take(&mut inner.highlight);
                let chosen = inner.engine.select(highlight);
                if chosen.commit.is_empty() {
                    inner.engine.commit()
                } else {
                    chosen
                }
            };
            apply(this, &target, output.commit, output.preedit);
        }
        KeyAction::Cancel => {
            {
                let mut inner = this.inner.borrow_mut();
                inner.highlight = 0;
                inner.engine.reset();
            }
            apply(this, &target, String::new(), String::new());
        }
        KeyAction::Next | KeyAction::Previous => {
            let (highlight, window) = {
                let mut inner = this.inner.borrow_mut();
                let count = inner.engine.candidates().len().max(1);
                inner.highlight = if action == KeyAction::Next {
                    (inner.highlight + 1) % count
                } else {
                    (inner.highlight + count - 1) % count
                };
                (inner.highlight, inner.window.take())
            };
            this.put_back(window, |window| window.highlight(highlight));
        }
        KeyAction::Ignore | KeyAction::Pass { .. } => {}
    }
    Ok(TRUE)
}

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, thread_mgr: Option<&ITfThreadMgr>, client_id: u32) -> Result<()> {
        self.ActivateEx(thread_mgr, client_id, 0)
    }

    fn Deactivate(&self) -> Result<()> {
        guard(|| {
            let this = self.to_object();
            finish(&this);
            let (thread_mgr, client_id, cookie) = {
                let mut inner = self.inner.borrow_mut();
                inner.window = None;
                inner.composition = None;
                inner.context = None;
                inner.preedit.clear();
                inner.profiles = None;
                inner.engine.reset();
                (
                    inner.thread_mgr.take(),
                    inner.client_id,
                    inner.thread_sink_cookie.take(),
                )
            };
            if let Some(thread_mgr) = thread_mgr {
                unsafe {
                    if let Ok(keystrokes) = thread_mgr.cast::<ITfKeystrokeMgr>() {
                        let _ = keystrokes.UnadviseKeyEventSink(client_id);
                    }
                    if let (Some(cookie), Ok(source)) = (cookie, thread_mgr.cast::<ITfSource>()) {
                        let _ = source.UnadviseSink(cookie);
                    }
                }
            }
            Ok(())
        })
    }
}

impl ITfTextInputProcessorEx_Impl for TextService_Impl {
    fn ActivateEx(&self, thread_mgr: Option<&ITfThreadMgr>, client_id: u32, flags: u32) -> Result<()> {
        let started = guard(|| {
            let thread_mgr = thread_mgr.ok_or_else(|| Error::from(E_INVALIDARG))?;
            self.attach(thread_mgr, client_id);
            unsafe {
                let keystrokes: ITfKeystrokeMgr = thread_mgr.cast()?;
                keystrokes.AdviseKeyEventSink(client_id, &self.to_interface::<ITfKeyEventSink>(), TRUE)?;
                let cookie = thread_mgr
                    .cast::<ITfSource>()
                    .and_then(|source| {
                        source.AdviseSink(
                            &ITfThreadMgrEventSink::IID,
                            &self.to_interface::<ITfThreadMgrEventSink>(),
                        )
                    })
                    .ok();
                let profiles = CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER).ok();

                let mut inner = self.inner.borrow_mut();
                inner.thread_sink_cookie = cookie;
                inner.profiles = profiles;
                if flags & TF_TMAE_SECUREMODE != 0 {
                    inner.show_window = false;
                }
            }
            if self.selected_mode() == Mode::Romanized {
                warm_up();
            }
            Ok(())
        });
        if started.is_err() {
            let _ = self.Deactivate();
        }
        started
    }
}

impl ITfKeyEventSink_Impl for TextService_Impl {
    fn OnSetFocus(&self, foreground: BOOL) -> Result<()> {
        guard(|| {
            if !foreground.as_bool() {
                let window = self.inner.borrow_mut().window.take();
                self.put_back(window, |window| window.hide());
            }
            Ok(())
        })
    }

    fn OnTestKeyDown(&self, context: Option<&ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        guard(|| on_key(&self.to_object(), context, wparam, lparam, true))
    }

    fn OnKeyDown(&self, context: Option<&ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        guard(|| on_key(&self.to_object(), context, wparam, lparam, false))
    }

    fn OnTestKeyUp(&self, _context: Option<&ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }

    fn OnKeyUp(&self, _context: Option<&ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }

    fn OnPreservedKey(&self, _context: Option<&ITfContext>, _guid: *const GUID) -> Result<BOOL> {
        Ok(FALSE)
    }
}

impl ITfCompositionSink_Impl for TextService_Impl {
    fn OnCompositionTerminated(&self, ec: u32, composition: Option<&ITfComposition>) -> Result<()> {
        guard(|| {
            let context = self.inner.borrow().context.clone();
            if let (Some(context), Some(composition)) = (context, composition) {
                unsafe {
                    if let Ok(range) = composition.GetRange() {
                        clear_attribute(ec, &context, &range);
                    }
                }
            }
            self.abandon();
            Ok(())
        })
    }
}

impl ITfThreadMgrEventSink_Impl for TextService_Impl {
    fn OnInitDocumentMgr(&self, _document: Option<&ITfDocumentMgr>) -> Result<()> {
        Ok(())
    }

    fn OnUninitDocumentMgr(&self, _document: Option<&ITfDocumentMgr>) -> Result<()> {
        Ok(())
    }

    fn OnSetFocus(&self, _focus: Option<&ITfDocumentMgr>, _previous: Option<&ITfDocumentMgr>) -> Result<()> {
        guard(|| {
            let this = self.to_object();
            finish(&this);
            let window = self.inner.borrow_mut().window.take();
            self.put_back(window, |window| window.hide());
            Ok(())
        })
    }

    fn OnPushContext(&self, _context: Option<&ITfContext>) -> Result<()> {
        Ok(())
    }

    fn OnPopContext(&self, _context: Option<&ITfContext>) -> Result<()> {
        Ok(())
    }
}

impl ITfDisplayAttributeProvider_Impl for TextService_Impl {
    fn EnumDisplayAttributeInfo(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        Ok(AttributeList::new().into())
    }

    fn GetDisplayAttributeInfo(&self, guid: *const GUID) -> Result<ITfDisplayAttributeInfo> {
        if !guid.is_null() && unsafe { *guid } == DISPLAY_ATTRIBUTE_INPUT {
            Ok(InputAttribute.into())
        } else {
            Err(Error::from(E_INVALIDARG))
        }
    }
}
