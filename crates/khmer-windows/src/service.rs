//! The text service: the object Windows talks to.
//!
//! Windows (its Text Services Framework, TSF) makes one of these for each
//! thread that has a text field, and tells it about every key press. The
//! service passes the key to the Rust engine and does what the engine says:
//!
//! * finished text is written into the app,
//! * the text still being typed is kept in a "composition": text the app
//!   shows as not yet final, which this service may still change.
//!
//! It never reorders or converts Khmer text itself. All of that is in
//! `khmer-core`.
//!
//! One rule keeps this file safe: the state in `inner` is never borrowed while
//! calling into Windows, because Windows may call straight back into this
//! object (a second borrow would be a panic).

use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::Once;

use khmer_core::{Engine, Mode, Romanizer, BACKSPACE};
use windows::core::{implement, ComObject, Error, IUnknownImpl, Interface, Result, BSTR, GUID, VARIANT};
use windows::Win32::Foundation::{
    BOOL, E_FAIL, E_INVALIDARG, FALSE, HWND, LPARAM, POINT, RECT, S_FALSE, TRUE, WPARAM,
};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, MapVirtualKeyW, MAPVK_VK_TO_VSC};
use windows::Win32::UI::TextServices::{
    IEnumTfDisplayAttributeInfo, IEnumTfDisplayAttributeInfo_Impl, ITfCategoryMgr, ITfCompartmentMgr,
    ITfComposition, ITfCompositionSink, ITfCompositionSink_Impl, ITfContext, ITfContextComposition,
    ITfDisplayAttributeInfo, ITfDisplayAttributeInfo_Impl, ITfDisplayAttributeProvider,
    ITfDisplayAttributeProvider_Impl, ITfDocumentMgr, ITfEditSession, ITfEditSession_Impl,
    ITfInputProcessorProfileMgr, ITfInsertAtSelection, ITfKeyEventSink, ITfKeyEventSink_Impl,
    ITfKeystrokeMgr, ITfRange, ITfSource, ITfTextInputProcessorEx, ITfTextInputProcessorEx_Impl,
    ITfTextInputProcessor_Impl, ITfThreadMgr, ITfThreadMgrEventSink, ITfThreadMgrEventSink_Impl,
    CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, GUID_COMPARTMENT_EMPTYCONTEXT,
    GUID_COMPARTMENT_KEYBOARD_DISABLED, GUID_PROP_ATTRIBUTE, GUID_TFCAT_TIP_KEYBOARD,
    INSERT_TEXT_AT_SELECTION_FLAGS, TF_AE_NONE, TF_ANCHOR_END, TF_ATTR_INPUT, TF_DISPLAYATTRIBUTE,
    TF_ES_ASYNCDONTCARE, TF_ES_READWRITE, TF_ES_SYNC, TF_IAS_QUERYONLY, TF_INPUTPROCESSORPROFILE,
    TF_LS_DOT, TF_SD_READONLY, TF_SELECTION, TF_SELECTIONSTYLE, TF_TMAE_SECUREMODE,
};
use windows::Win32::UI::WindowsAndMessaging::{GetGUIThreadInfo, GUITHREADINFO};

use crate::candidates::CandidateWindow;
use crate::keys::{self, KeyAction, Modifiers};
use crate::{DISPLAY_ATTRIBUTE_INPUT, PROFILE_KHMER};

/// Runs `action`, turning a panic into an error. A panic must not leave this
/// DLL: it would close whichever app is being typed into.
pub(crate) fn guard<T>(action: impl FnOnce() -> Result<T>) -> Result<T> {
    catch_unwind(AssertUnwindSafe(action)).unwrap_or_else(|_| Err(Error::from(E_FAIL)))
}

fn utf16(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// Starts reading the Singkhmer dictionary, on another thread, the first time
/// it is called in a program.
///
/// The engine would read it by itself at the first Singkhmer key press, but
/// that takes a moment, and the app would stand still meanwhile. A key pressed
/// before the reading is done still waits for it.
fn warm_up() {
    static STARTED: Once = Once::new();
    STARTED.call_once(|| {
        let _ = std::thread::Builder::new().name("khmer keyboard dictionary".into()).spawn(|| {
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
    /// The number Windows knows this service by on this thread.
    client_id: u32,
    /// Tells which of the two keyboards (Singkhmer, Khmer) is selected.
    profiles: Option<ITfInputProcessorProfileMgr>,
    thread_sink_cookie: Option<u32>,
    /// The number Windows knows the "being typed" underline by. 0 for none.
    display_atom: u32,

    engine: Engine,
    /// The mode the engine is in.
    mode: Mode,
    /// The text being typed, while there is some.
    composition: Option<ITfComposition>,
    /// The text field the composition is in.
    context: Option<ITfContext>,
    /// The composition's text, as last written.
    preedit: String,
    /// Which suggestion is highlighted.
    highlight: usize,
    /// Where on screen the text being typed is.
    anchor: RECT,

    window: Option<CandidateWindow>,
    /// False where no window may be shown: the sign-in screen, and tests.
    show_window: bool,

    /// Set by tests in place of the selected keyboard and the real key state.
    test_mode: Option<Mode>,
    test_modifiers: Option<Modifiers>,
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
                // A new engine starts in Khmer mode.
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
            }),
        }
    }

    /// A service for tests: it shows no window, and its mode and the state of
    /// Shift and Ctrl are set by the test rather than read from Windows.
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

    /// The suggestions for the phrase being typed, and which is highlighted.
    pub fn suggestions(&self) -> (Vec<String>, usize) {
        let inner = self.inner.borrow();
        (inner.engine.candidates(), inner.highlight)
    }

    /// Remembers the thread this service works for. `Activate` does this and
    /// then asks Windows for the key presses; tests call only this, and pass
    /// the key presses in themselves.
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

    /// Forgets the text being typed without touching the app: for when the
    /// app has already ended the composition, or writing to it failed.
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

    /// Does something with the suggestion window, which is taken out of
    /// `inner` meanwhile so that `inner` is not borrowed during window calls.
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

    /// The mode of the keyboard the user has selected.
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

    /// Writes into the app. Runs inside an edit session: `ec` is the app's
    /// permission to change its text.
    ///
    /// `commit` becomes finished text, in place of the text being typed, and
    /// `preedit` becomes the new text being typed.
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
            (inner.composition.take(), inner.display_atom, std::mem::take(&mut inner.preedit))
        };
        unsafe {
            if !commit.is_empty() {
                match composition.take() {
                    // The text being typed turns into the finished text.
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
                // Nothing is being typed any more. If a composition is still
                // open, all of it was deleted or cancelled.
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
                    // Start one where the cursor is.
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
            // An app that cannot say where the text is leaves this empty, and
            // the suggestions go by the app's cursor instead.
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

/// Puts the cursor at the end of `range`.
unsafe fn caret_to_end(ec: u32, context: &ITfContext, range: &ITfRange) -> Result<()> {
    let end = range.Clone()?;
    end.Collapse(ec, TF_ANCHOR_END)?;
    let selection = TF_SELECTION {
        range: std::mem::ManuallyDrop::new(Some(end)),
        style: TF_SELECTIONSTYLE { ase: TF_AE_NONE, fInterimChar: FALSE },
    };
    let result = context.SetSelection(ec, &[selection.clone()]);
    // TF_SELECTION does not release its range by itself.
    drop(std::mem::ManuallyDrop::into_inner(selection.range));
    result
}

/// Marks `range` as text being typed, which apps draw with a dotted underline.
unsafe fn set_attribute(ec: u32, context: &ITfContext, range: &ITfRange, atom: u32) {
    if atom == 0 {
        return;
    }
    if let Ok(property) = context.GetProperty(&GUID_PROP_ATTRIBUTE) {
        let _ = property.SetValue(ec, range, &VARIANT::from(atom as i32));
    }
}

unsafe fn clear_attribute(ec: u32, context: &ITfContext, range: &ITfRange) {
    if let Ok(property) = context.GetProperty(&GUID_PROP_ATTRIBUTE) {
        let _ = property.Clear(ec, range);
    }
}

/// Where on screen `range` is, if the app can say.
unsafe fn text_place(ec: u32, context: &ITfContext, range: &ITfRange) -> Option<RECT> {
    let view = context.GetActiveView().ok()?;
    let mut place = RECT::default();
    let mut clipped = FALSE;
    view.GetTextExt(ec, range, &mut place, &mut clipped).ok()?;
    (place != RECT::default()).then_some(place)
}

/// Where the app's own cursor is, for apps that cannot say where the text is.
fn caret_place() -> Option<RECT> {
    unsafe {
        let mut info = GUITHREADINFO { cbSize: std::mem::size_of::<GUITHREADINFO>() as u32, ..Default::default() };
        GetGUIThreadInfo(0, &mut info).ok()?;
        if info.hwndCaret == HWND::default() {
            return None;
        }
        let mut top_left = POINT { x: info.rcCaret.left, y: info.rcCaret.top };
        let mut bottom_right = POINT { x: info.rcCaret.right, y: info.rcCaret.bottom };
        let _ = ClientToScreen(info.hwndCaret, &mut top_left);
        let _ = ClientToScreen(info.hwndCaret, &mut bottom_right);
        Some(RECT { left: top_left.x, top: top_left.y, right: bottom_right.x, bottom: bottom_right.y })
    }
}

/// True when the app does not want a keyboard like this one in this field.
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
        [GUID_COMPARTMENT_KEYBOARD_DISABLED, GUID_COMPARTMENT_EMPTYCONTEXT].iter().any(|guid| {
            compartments
                .GetCompartment(guid)
                .and_then(|compartment| compartment.GetValue())
                .map(|value| i32::try_from(&value).unwrap_or(0) != 0)
                .unwrap_or(false)
        })
    }
}

// ---- edit sessions ----

/// Windows lets a service change an app's text only inside an "edit session":
/// the service hands over an object, and Windows calls it back when the app
/// is ready, with a cookie that stands for the permission.
#[implement(ITfEditSession)]
struct EditSession {
    run: Box<dyn Fn(u32) -> Result<()>>,
}

impl ITfEditSession_Impl for EditSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        guard(|| (self.run)(ec))
    }
}

/// Runs `run` in an edit session on `context`: at once if the app allows it,
/// otherwise as soon as it does.
fn edit(client_id: u32, context: &ITfContext, run: impl Fn(u32) -> Result<()> + 'static) -> Result<()> {
    let session: ITfEditSession = EditSession { run: Box::new(run) }.into();
    unsafe {
        match context.RequestEditSession(client_id, &session, TF_ES_SYNC | TF_ES_READWRITE) {
            Ok(result) => result.ok(),
            Err(_) => context
                .RequestEditSession(client_id, &session, TF_ES_ASYNCDONTCARE | TF_ES_READWRITE)?
                .ok(),
        }
    }
}

/// Writes the engine's answer into the app and brings the suggestion window
/// up to date.
fn apply(this: &ComObject<TextService>, context: &ITfContext, commit: String, preedit: String) {
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

/// Finishes whatever is being typed, with the engine's first suggestion.
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

/// Shows the suggestions for the phrase being typed, or hides the window when
/// there is nothing to choose from.
fn refresh_window(this: &ComObject<TextService>) {
    let (items, highlight, mut anchor, window, allowed) = {
        let mut inner = this.inner.borrow_mut();
        let items = if inner.mode == Mode::Romanized && inner.composition.is_some() {
            inner.engine.candidates()
        } else {
            Vec::new()
        };
        (items, inner.highlight, inner.anchor, inner.window.take(), inner.show_window)
    };
    if items.is_empty() || !allowed {
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
        Some(window)
    });
    if anchor == RECT::default() {
        anchor = caret_place().unwrap_or_default();
    }
    this.put_back(window, |window| window.show(items, highlight, anchor));
}

/// Commits suggestion number `index`: a click in the suggestion window.
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

// ---- key presses ----

/// Decides what a key press does, and, unless this is only a question
/// (`test`), does it. Returns whether the key is ours: if it is, the app
/// never sees it.
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

    // The user may have switched between the two keyboards.
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
        // A key press made by a program can come without its position.
        scan = unsafe { MapVirtualKeyW(u32::from(vk), MAPVK_VK_TO_VSC) } as u8;
    }
    let composing = this.is_composing();
    let action = keys::action(mode, composing, vk, scan, this.modifiers());

    if let KeyAction::Pass { forget } = action {
        if composing {
            // In Khmer mode the text being typed is one syllable, which the
            // user thinks of as already written: finish it, so that Enter or
            // an arrow key acts on finished text.
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

    // Text being typed stays in the field it was started in.
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
                // With another suggestion highlighted, Space picks it.
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

// ---- what Windows calls ----

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
                // Dropping the window also drops its hold on this service.
                inner.window = None;
                inner.composition = None;
                inner.context = None;
                inner.preedit.clear();
                inner.profiles = None;
                inner.engine.reset();
                (inner.thread_mgr.take(), inner.client_id, inner.thread_sink_cookie.take())
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
                // Ask for the key presses, and to be told when the user moves
                // to another text field.
                let keystrokes: ITfKeystrokeMgr = thread_mgr.cast()?;
                keystrokes.AdviseKeyEventSink(client_id, &self.to_interface::<ITfKeyEventSink>(), TRUE)?;
                let cookie = thread_mgr
                    .cast::<ITfSource>()
                    .and_then(|source| {
                        source.AdviseSink(&ITfThreadMgrEventSink::IID, &self.to_interface::<ITfThreadMgrEventSink>())
                    })
                    .ok();
                let profiles = CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER).ok();

                let mut inner = self.inner.borrow_mut();
                inner.thread_sink_cookie = cookie;
                inner.profiles = profiles;
                // No windows on the sign-in screen.
                if flags & TF_TMAE_SECUREMODE != 0 {
                    inner.show_window = false;
                }
            }
            // The Khmer layout needs no dictionary, so a program where only
            // that one is used never reads it.
            if self.selected_mode() == Mode::Romanized {
                warm_up();
            }
            Ok(())
        });
        if started.is_err() {
            // Windows turned the service down: keep nothing of the attempt.
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
    /// The app ended the composition itself: the user clicked somewhere else,
    /// say. The text stays as it was showing.
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

    /// The user moved to another text field, or away from the app.
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

// ---- the underline under text being typed ----

impl ITfDisplayAttributeProvider_Impl for TextService_Impl {
    fn EnumDisplayAttributeInfo(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        Ok(AttributeList { position: Cell::new(0) }.into())
    }

    fn GetDisplayAttributeInfo(&self, guid: *const GUID) -> Result<ITfDisplayAttributeInfo> {
        if !guid.is_null() && unsafe { *guid } == DISPLAY_ATTRIBUTE_INPUT {
            Ok(InputAttribute.into())
        } else {
            Err(Error::from(E_INVALIDARG))
        }
    }
}

/// How text being typed looks: a dotted underline, colours left alone.
#[implement(ITfDisplayAttributeInfo)]
struct InputAttribute;

impl ITfDisplayAttributeInfo_Impl for InputAttribute_Impl {
    fn GetGUID(&self) -> Result<GUID> {
        Ok(DISPLAY_ATTRIBUTE_INPUT)
    }

    fn GetDescription(&self) -> Result<BSTR> {
        Ok(BSTR::from("Khmer Keyboard: text being typed"))
    }

    fn GetAttributeInfo(&self, attribute: *mut TF_DISPLAYATTRIBUTE) -> Result<()> {
        if attribute.is_null() {
            return Err(Error::from(E_INVALIDARG));
        }
        unsafe {
            *attribute = TF_DISPLAYATTRIBUTE { lsStyle: TF_LS_DOT, bAttr: TF_ATTR_INPUT, ..Default::default() };
        }
        Ok(())
    }

    fn SetAttributeInfo(&self, _attribute: *const TF_DISPLAYATTRIBUTE) -> Result<()> {
        Ok(())
    }

    fn Reset(&self) -> Result<()> {
        Ok(())
    }
}

/// The list of this service's display attributes: just the one.
#[implement(IEnumTfDisplayAttributeInfo)]
struct AttributeList {
    position: Cell<u32>,
}

impl IEnumTfDisplayAttributeInfo_Impl for AttributeList_Impl {
    fn Clone(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        Ok(AttributeList { position: Cell::new(self.position.get()) }.into())
    }

    fn Next(&self, count: u32, items: *mut Option<ITfDisplayAttributeInfo>, fetched: *mut u32) -> Result<()> {
        let mut given = 0;
        if count > 0 && !items.is_null() && self.position.get() == 0 {
            unsafe { items.write(Some(InputAttribute.into())) };
            self.position.set(1);
            given = 1;
        }
        if !fetched.is_null() {
            unsafe { *fetched = given };
        }
        // "Fewer than asked for" is not a failure, but has its own code.
        if given == count {
            Ok(())
        } else {
            Err(Error::from(S_FALSE))
        }
    }

    fn Reset(&self) -> Result<()> {
        self.position.set(0);
        Ok(())
    }

    fn Skip(&self, count: u32) -> Result<()> {
        self.position.set(self.position.get().saturating_add(count));
        Ok(())
    }
}
