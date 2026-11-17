#![cfg(windows)]
#![allow(non_snake_case)]

use std::cell::RefCell;

use khmer_core::Mode;
use khmer_tsf::keys::{Modifiers, VK_BACK, VK_DOWN, VK_ESCAPE, VK_RETURN, VK_SPACE};
use khmer_tsf::service::TextService;
use khmer_tsf::{CLSID_TEXT_SERVICE, DISPLAY_ATTRIBUTE_INPUT};
use windows::core::{implement, ComObject, Error, IUnknown, Interface, Result, GUID, HRESULT, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    BOOL, CLASS_E_CLASSNOTAVAILABLE, E_INVALIDARG, E_NOTIMPL, E_UNEXPECTED, FALSE, HWND, LPARAM, POINT, RECT, S_OK,
    TRUE, WPARAM,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, IClassFactory, IDataObject, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    FORMATETC,
};
use windows::Win32::UI::TextServices::{
    CLSID_TF_ThreadMgr, ITextStoreACP, ITextStoreACPSink, ITextStoreACP_Impl, ITfCompositionView, ITfContext,
    ITfContextComposition, ITfContextOwnerCompositionServices, ITfContextOwnerCompositionSink,
    ITfContextOwnerCompositionSink_Impl, ITfDisplayAttributeProvider, ITfDocumentMgr, ITfKeyEventSink, ITfRange,
    ITfRangeACP, ITfTextInputProcessorEx, ITfThreadMgr, ITfThreadMgrEventSink, TEXT_STORE_LOCK_FLAGS,
    TF_DISPLAYATTRIBUTE, TF_LS_DOT, TF_POPF_ALL, TF_TMAE_SECUREMODE, TS_AE_END, TS_AE_NONE, TS_ATTRVAL,
    TS_DEFAULT_SELECTION, TS_E_NOLOCK, TS_E_SYNCHRONOUS, TS_IAS_NOQUERY, TS_IAS_QUERYONLY, TS_LF_SYNC, TS_RT_PLAIN,
    TS_RUNINFO, TS_SD_READONLY, TS_SELECTIONSTYLE, TS_SELECTION_ACP, TS_STATUS, TS_S_ASYNC, TS_TEXTCHANGE,
};

const MAY_READ: u32 = 2;
const MAY_WRITE: u32 = 4;

#[derive(Default)]
struct Content {
    text: Vec<u16>,
    start: i32,
    end: i32,
    read_only: bool,
    compositions: usize,

    sink: Option<ITextStoreACPSink>,
    lock: u32,
    waiting: Option<u32>,
}

#[implement(ITextStoreACP, ITfContextOwnerCompositionSink)]
#[derive(Default)]
struct Field {
    content: RefCell<Content>,
}

impl Field {
    fn need(&self, permission: u32) -> Result<()> {
        if self.content.borrow().lock & permission == permission {
            Ok(())
        } else {
            Err(Error::from(TS_E_NOLOCK))
        }
    }

    fn replace(&self, start: i32, end: i32, new: &[u16]) -> Result<TS_TEXTCHANGE> {
        let mut content = self.content.borrow_mut();
        let length = content.text.len() as i32;
        if start < 0 || start > end || end > length {
            return Err(Error::from(E_INVALIDARG));
        }
        content.text.splice(start as usize..end as usize, new.iter().copied());
        let new_end = start + new.len() as i32;
        content.start = new_end;
        content.end = new_end;
        Ok(TS_TEXTCHANGE {
            acpStart: start,
            acpOldEnd: end,
            acpNewEnd: new_end,
        })
    }
}

unsafe fn units<'a>(text: &PCWSTR, count: u32) -> &'a [u16] {
    if text.is_null() || count == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(text.0, count as usize)
    }
}

impl ITextStoreACP_Impl for Field_Impl {
    fn AdviseSink(&self, iid: *const GUID, sink: Option<&IUnknown>, _mask: u32) -> Result<()> {
        if iid.is_null() || unsafe { *iid } != ITextStoreACPSink::IID {
            return Err(Error::from(E_INVALIDARG));
        }
        let sink = sink
            .ok_or_else(|| Error::from(E_INVALIDARG))?
            .cast::<ITextStoreACPSink>()?;
        self.content.borrow_mut().sink = Some(sink);
        Ok(())
    }

    fn UnadviseSink(&self, _sink: Option<&IUnknown>) -> Result<()> {
        self.content.borrow_mut().sink = None;
        Ok(())
    }

    fn RequestLock(&self, flags: u32) -> Result<HRESULT> {
        let wanted = flags & !TS_LF_SYNC;
        let sink = {
            let mut content = self.content.borrow_mut();
            let sink = content.sink.clone().ok_or_else(|| Error::from(E_UNEXPECTED))?;
            if content.lock != 0 {
                if flags & TS_LF_SYNC != 0 {
                    return Ok(TS_E_SYNCHRONOUS);
                }
                content.waiting = Some(wanted);
                return Ok(TS_S_ASYNC);
            }
            content.lock = wanted;
            sink
        };
        let granted = unsafe { sink.OnLockGranted(TEXT_STORE_LOCK_FLAGS(wanted)) };
        self.content.borrow_mut().lock = 0;
        loop {
            let Some(next) = self.content.borrow_mut().waiting.take() else {
                break;
            };
            self.content.borrow_mut().lock = next;
            let _ = unsafe { sink.OnLockGranted(TEXT_STORE_LOCK_FLAGS(next)) };
            self.content.borrow_mut().lock = 0;
        }
        Ok(granted.map_or_else(|error| error.code(), |()| S_OK))
    }

    fn GetStatus(&self) -> Result<TS_STATUS> {
        let read_only = self.content.borrow().read_only;
        Ok(TS_STATUS {
            dwDynamicFlags: if read_only { TS_SD_READONLY } else { 0 },
            dwStaticFlags: 0,
        })
    }

    fn QueryInsert(
        &self,
        start: i32,
        end: i32,
        _count: u32,
        result_start: *mut i32,
        result_end: *mut i32,
    ) -> Result<()> {
        unsafe {
            *result_start = start;
            *result_end = end;
        }
        Ok(())
    }

    fn GetSelection(&self, index: u32, count: u32, selection: *mut TS_SELECTION_ACP, fetched: *mut u32) -> Result<()> {
        self.need(MAY_READ)?;
        if selection.is_null() || fetched.is_null() || count == 0 || (index != TS_DEFAULT_SELECTION && index != 0) {
            return Err(Error::from(E_INVALIDARG));
        }
        let content = self.content.borrow();
        unsafe {
            *selection = TS_SELECTION_ACP {
                acpStart: content.start,
                acpEnd: content.end,
                style: TS_SELECTIONSTYLE {
                    ase: if content.start == content.end {
                        TS_AE_NONE
                    } else {
                        TS_AE_END
                    },
                    fInterimChar: FALSE,
                },
            };
            *fetched = 1;
        }
        Ok(())
    }

    fn SetSelection(&self, count: u32, selection: *const TS_SELECTION_ACP) -> Result<()> {
        self.need(MAY_WRITE)?;
        if count != 1 || selection.is_null() {
            return Err(Error::from(E_INVALIDARG));
        }
        let selection = unsafe { *selection };
        let mut content = self.content.borrow_mut();
        let length = content.text.len() as i32;
        if selection.acpStart < 0 || selection.acpStart > selection.acpEnd || selection.acpEnd > length {
            return Err(Error::from(E_INVALIDARG));
        }
        content.start = selection.acpStart;
        content.end = selection.acpEnd;
        Ok(())
    }

    fn GetText(
        &self,
        start: i32,
        end: i32,
        plain: PWSTR,
        plain_room: u32,
        plain_count: *mut u32,
        runs: *mut TS_RUNINFO,
        runs_room: u32,
        runs_count: *mut u32,
        next: *mut i32,
    ) -> Result<()> {
        self.need(MAY_READ)?;
        let content = self.content.borrow();
        let length = content.text.len() as i32;
        let end = if end == -1 { length } else { end };
        if start < 0 || start > end || end > length {
            return Err(Error::from(E_INVALIDARG));
        }
        let mut count = (end - start) as u32;
        let copy = !plain.is_null() && plain_room > 0;
        if copy {
            count = count.min(plain_room);
        }
        unsafe {
            if copy {
                std::ptr::copy_nonoverlapping(content.text[start as usize..].as_ptr(), plain.0, count as usize);
            }
            if !plain_count.is_null() {
                *plain_count = if copy { count } else { 0 };
            }
            let one_run = !runs.is_null() && runs_room > 0 && count > 0;
            if one_run {
                *runs = TS_RUNINFO {
                    uCount: count,
                    r#type: TS_RT_PLAIN,
                };
            }
            if !runs_count.is_null() {
                *runs_count = u32::from(one_run);
            }
            if !next.is_null() {
                *next = start + count as i32;
            }
        }
        Ok(())
    }

    fn SetText(&self, _flags: u32, start: i32, end: i32, text: &PCWSTR, count: u32) -> Result<TS_TEXTCHANGE> {
        self.need(MAY_WRITE)?;
        self.replace(start, end, unsafe { units(text, count) })
    }

    fn GetFormattedText(&self, _start: i32, _end: i32) -> Result<IDataObject> {
        Err(Error::from(E_NOTIMPL))
    }

    fn GetEmbedded(&self, _position: i32, _service: *const GUID, _iid: *const GUID) -> Result<IUnknown> {
        Err(Error::from(E_NOTIMPL))
    }

    fn QueryInsertEmbedded(&self, _service: *const GUID, _format: *const FORMATETC) -> Result<BOOL> {
        Ok(FALSE)
    }

    fn InsertEmbedded(
        &self,
        _flags: u32,
        _start: i32,
        _end: i32,
        _object: Option<&IDataObject>,
    ) -> Result<TS_TEXTCHANGE> {
        Err(Error::from(E_NOTIMPL))
    }

    fn InsertTextAtSelection(
        &self,
        flags: u32,
        text: &PCWSTR,
        count: u32,
        result_start: *mut i32,
        result_end: *mut i32,
        change: *mut TS_TEXTCHANGE,
    ) -> Result<()> {
        let (start, end) = {
            let content = self.content.borrow();
            (content.start, content.end)
        };
        if flags & TS_IAS_QUERYONLY != 0 {
            self.need(MAY_READ)?;
            unsafe {
                if !result_start.is_null() {
                    *result_start = start;
                }
                if !result_end.is_null() {
                    *result_end = end;
                }
            }
            return Ok(());
        }
        self.need(MAY_WRITE)?;
        let done = self.replace(start, end, unsafe { units(text, count) })?;
        unsafe {
            if flags & TS_IAS_NOQUERY == 0 {
                if !result_start.is_null() {
                    *result_start = done.acpStart;
                }
                if !result_end.is_null() {
                    *result_end = done.acpNewEnd;
                }
            }
            if !change.is_null() {
                *change = done;
            }
        }
        Ok(())
    }

    fn InsertEmbeddedAtSelection(
        &self,
        _flags: u32,
        _object: Option<&IDataObject>,
        _start: *mut i32,
        _end: *mut i32,
        _change: *mut TS_TEXTCHANGE,
    ) -> Result<()> {
        Err(Error::from(E_NOTIMPL))
    }

    fn RequestSupportedAttrs(&self, _flags: u32, _count: u32, _attributes: *const GUID) -> Result<()> {
        Ok(())
    }

    fn RequestAttrsAtPosition(&self, _position: i32, _count: u32, _attributes: *const GUID, _flags: u32) -> Result<()> {
        Ok(())
    }

    fn RequestAttrsTransitioningAtPosition(
        &self,
        _position: i32,
        _count: u32,
        _attributes: *const GUID,
        _flags: u32,
    ) -> Result<()> {
        Ok(())
    }

    fn FindNextAttrTransition(
        &self,
        _start: i32,
        _halt: i32,
        _count: u32,
        _attributes: *const GUID,
        _flags: u32,
        next: *mut i32,
        found: *mut BOOL,
        found_offset: *mut i32,
    ) -> Result<()> {
        unsafe {
            if !next.is_null() {
                *next = 0;
            }
            if !found.is_null() {
                *found = FALSE;
            }
            if !found_offset.is_null() {
                *found_offset = 0;
            }
        }
        Ok(())
    }

    fn RetrieveRequestedAttrs(&self, _count: u32, _values: *mut TS_ATTRVAL, fetched: *mut u32) -> Result<()> {
        if !fetched.is_null() {
            unsafe { *fetched = 0 };
        }
        Ok(())
    }

    fn GetEndACP(&self) -> Result<i32> {
        self.need(MAY_READ)?;
        Ok(self.content.borrow().text.len() as i32)
    }

    fn GetActiveView(&self) -> Result<u32> {
        Ok(1)
    }

    fn GetACPFromPoint(&self, _view: u32, _point: *const POINT, _flags: u32) -> Result<i32> {
        Err(Error::from(E_NOTIMPL))
    }

    fn GetTextExt(&self, _view: u32, start: i32, end: i32, place: *mut RECT, clipped: *mut BOOL) -> Result<()> {
        self.need(MAY_READ)?;
        unsafe {
            if !place.is_null() {
                *place = RECT {
                    left: 100 + 10 * start,
                    top: 100,
                    right: 100 + 10 * end.max(start + 1),
                    bottom: 120,
                };
            }
            if !clipped.is_null() {
                *clipped = FALSE;
            }
        }
        Ok(())
    }

    fn GetScreenExt(&self, _view: u32) -> Result<RECT> {
        Ok(RECT {
            left: 0,
            top: 0,
            right: 800,
            bottom: 600,
        })
    }

    fn GetWnd(&self, _view: u32) -> Result<HWND> {
        Ok(HWND::default())
    }
}

impl ITfContextOwnerCompositionSink_Impl for Field_Impl {
    fn OnStartComposition(&self, _composition: Option<&ITfCompositionView>) -> Result<BOOL> {
        self.content.borrow_mut().compositions += 1;
        Ok(TRUE)
    }

    fn OnUpdateComposition(&self, _composition: Option<&ITfCompositionView>, _range: Option<&ITfRange>) -> Result<()> {
        Ok(())
    }

    fn OnEndComposition(&self, _composition: Option<&ITfCompositionView>) -> Result<()> {
        let mut content = self.content.borrow_mut();
        content.compositions = content.compositions.saturating_sub(1);
        Ok(())
    }
}

struct Windows {
    thread_mgr: ITfThreadMgr,
    client_id: u32,
    document: ITfDocumentMgr,
    context: ITfContext,
    field: ComObject<Field>,
}

impl Windows {
    fn with_field(text: &str, cursor: usize) -> Windows {
        let field = ComObject::new(Field::default());
        {
            let mut content = field.content.borrow_mut();
            content.text = text.encode_utf16().collect();
            content.start = cursor as i32;
            content.end = cursor as i32;
        }
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let thread_mgr: ITfThreadMgr =
                CoCreateInstance(&CLSID_TF_ThreadMgr, None, CLSCTX_INPROC_SERVER).expect("the text framework starts");
            let client_id = thread_mgr.Activate().expect("the text framework accepts this thread");
            let document = thread_mgr.CreateDocumentMgr().expect("a document");
            let store: ITextStoreACP = field.to_interface();
            let mut context = None;
            let mut cookie = 0;
            document
                .CreateContext(client_id, 0, &store, &mut context, &mut cookie)
                .expect("a context for the field");
            let context = context.expect("a context for the field");
            document.Push(&context).expect("the field is shown");
            thread_mgr.SetFocus(&document).expect("the field takes the focus");
            Windows {
                thread_mgr,
                client_id,
                document,
                context,
                field,
            }
        }
    }

    fn text(&self) -> String {
        String::from_utf16(&self.field.content.borrow().text).expect("the field holds valid text")
    }

    fn cursor(&self) -> usize {
        let content = self.field.content.borrow();
        assert_eq!(content.start, content.end, "nothing should be selected");
        content.end as usize
    }

    fn composing(&self) -> Option<String> {
        unsafe {
            let compositions: ITfContextComposition = self.context.cast().expect("compositions");
            let list = compositions.EnumCompositions().expect("the list of compositions");
            let mut views = [None];
            let mut fetched = 0;
            let _ = list.Next(&mut views, &mut fetched);
            let view = views[0].take()?;
            let range: ITfRangeACP = view
                .GetRange()
                .expect("the composition's place")
                .cast()
                .expect("as positions");
            let (mut start, mut length) = (0, 0);
            range
                .GetExtent(&mut start, &mut length)
                .expect("the composition's extent");
            let content = self.field.content.borrow();
            assert_eq!(content.compositions, 1, "the field was told about the composition");
            Some(String::from_utf16(&content.text[start as usize..(start + length) as usize]).expect("valid text"))
        }
    }

    fn assert_nothing_is_composing(&self) {
        assert_eq!(self.composing(), None);
        assert_eq!(
            self.field.content.borrow().compositions,
            0,
            "the field was told the composition ended"
        );
    }
}

impl Drop for Windows {
    fn drop(&mut self) {
        unsafe {
            let _ = self.document.Pop(TF_POPF_ALL);
            let _ = self.thread_mgr.Deactivate();
        }
    }
}

struct Desk {
    service: ComObject<TextService>,
    keys: ITfKeyEventSink,
    windows: Windows,
}

impl Desk {
    fn new(mode: Mode) -> Desk {
        Desk::with_field(mode, "", 0)
    }

    fn with_field(mode: Mode, text: &str, cursor: usize) -> Desk {
        let windows = Windows::with_field(text, cursor);
        let service = ComObject::new(TextService::for_tests(mode));
        service.attach(&windows.thread_mgr, windows.client_id);
        let keys = service.to_interface();
        Desk { service, keys, windows }
    }

    fn press(&self, vk: u16, scan: u8) -> bool {
        let wparam = WPARAM(usize::from(vk));
        let lparam = LPARAM(isize::from(scan) << 16 | 1);
        unsafe {
            let wanted = self
                .keys
                .OnTestKeyDown(&self.windows.context, wparam, lparam)
                .expect("an answer");
            wanted.as_bool()
                && self
                    .keys
                    .OnKeyDown(&self.windows.context, wparam, lparam)
                    .expect("an answer")
                    .as_bool()
        }
    }

    fn type_keys(&self, keys: &str) {
        for character in keys.chars() {
            let (vk, scan) = us_key(character);
            assert!(self.press(vk, scan), "the key {character:?} was left to the app");
        }
    }

    fn type_shifted(&self, keys: &str) {
        self.service.set_test_modifiers(Modifiers {
            shift: true,
            ..Modifiers::default()
        });
        self.type_keys(keys);
        self.service.set_test_modifiers(Modifiers::default());
    }

    fn enter(&self) -> bool {
        self.press(VK_RETURN, 0x1C)
    }

    fn backspace(&self) -> bool {
        self.press(VK_BACK, 0x0E)
    }
}

fn us_key(character: char) -> (u16, u8) {
    const ROWS: [(&str, u8); 4] = [
        ("1234567890-=", 0x02),
        ("qwertyuiop[]", 0x10),
        ("asdfghjkl;'", 0x1E),
        ("zxcvbnm,./", 0x2C),
    ];
    if character == ' ' {
        return (VK_SPACE, 0x39);
    }
    for (row, first_scan) in ROWS {
        if let Some(index) = row.find(character) {
            let vk = match character {
                'a'..='z' => character.to_ascii_uppercase() as u16,
                '0'..='9' => character as u16,
                _ => 0xBA,
            };
            return (vk, first_scan + index as u8);
        }
    }
    panic!("no key for {character:?}");
}

#[test]
fn a_singkhmer_phrase_shows_while_it_is_typed_and_enter_writes_it() {
    let desk = Desk::new(Mode::Romanized);
    desk.type_keys("nh jg tv");

    assert_eq!(desk.windows.text(), "ខ្ញុំចង់tv");
    assert_eq!(desk.windows.composing().as_deref(), Some("ខ្ញុំចង់tv"));
    let (suggestions, highlight) = desk.service.suggestions();
    assert_eq!(suggestions[0], "ខ្ញុំចង់ទៅ");
    assert_eq!(highlight, 0);

    assert!(desk.enter());
    assert_eq!(desk.windows.text(), "ខ្ញុំចង់ទៅ");
    assert_eq!(desk.windows.cursor(), "ខ្ញុំចង់ទៅ".encode_utf16().count());
    desk.windows.assert_nothing_is_composing();
    assert!(desk.service.suggestions().0.is_empty());

    assert!(!desk.enter());
}

#[test]
fn space_twice_writes_the_phrase_and_a_space() {
    let desk = Desk::new(Mode::Romanized);
    desk.type_keys("nh ");
    assert_eq!(desk.windows.composing().as_deref(), Some("ខ្ញុំ"));
    desk.type_keys(" ");
    assert_eq!(desk.windows.text(), "ខ្ញុំ ");
    desk.windows.assert_nothing_is_composing();
}

#[test]
fn punctuation_ends_the_phrase_and_is_written_after_it() {
    let desk = Desk::new(Mode::Romanized);
    desk.type_keys("nh jg tv.");
    assert_eq!(desk.windows.text(), "ខ្ញុំចង់ទៅ.");
    desk.windows.assert_nothing_is_composing();
}

#[test]
fn digits_and_spaces_are_latin_when_no_phrase_is_being_typed() {
    let desk = Desk::new(Mode::Romanized);
    desk.type_keys("12 3,");
    assert_eq!(desk.windows.text(), "12 3,");
    desk.windows.assert_nothing_is_composing();
}

#[test]
fn backspace_undoes_one_key_and_then_belongs_to_the_app() {
    let desk = Desk::new(Mode::Romanized);
    desk.type_keys("tv");
    assert!(desk.backspace());
    assert_eq!(desk.windows.composing().as_deref(), Some("t"));
    assert!(desk.backspace());
    assert_eq!(desk.windows.text(), "");
    desk.windows.assert_nothing_is_composing();
    assert!(!desk.backspace());
}

#[test]
fn escape_throws_the_phrase_away() {
    let desk = Desk::with_field(Mode::Romanized, "ab", 2);
    desk.type_keys("nh jg");
    assert!(desk.press(VK_ESCAPE, 0x01));
    assert_eq!(desk.windows.text(), "ab");
    desk.windows.assert_nothing_is_composing();
    assert!(!desk.press(VK_ESCAPE, 0x01));
}

#[test]
fn the_down_arrow_and_enter_pick_another_suggestion() {
    let desk = Desk::new(Mode::Romanized);
    desk.type_keys("tv");
    let (suggestions, _) = desk.service.suggestions();
    assert!(suggestions.len() > 1, "tv has more than one reading: {suggestions:?}");

    assert!(desk.press(VK_DOWN, 0x50));
    assert_eq!(desk.service.suggestions().1, 1);
    assert!(desk.enter());
    assert_eq!(desk.windows.text(), suggestions[1]);
    desk.windows.assert_nothing_is_composing();
}

#[test]
fn a_phrase_is_written_where_the_cursor_is() {
    let desk = Desk::with_field(Mode::Romanized, "ab", 1);
    desk.type_keys("nh");
    assert_eq!(desk.windows.text(), "anhb");
    assert!(desk.enter());
    assert_eq!(desk.windows.text(), "aខ្ញុំb");
    assert_eq!(desk.windows.cursor(), "aខ្ញុំ".encode_utf16().count());
}

#[test]
fn shortcuts_are_left_to_the_app() {
    let desk = Desk::new(Mode::Romanized);
    desk.service.set_test_modifiers(Modifiers {
        ctrl: true,
        ..Modifiers::default()
    });
    let (vk, scan) = us_key('c');
    assert!(!desk.press(vk, scan));
    assert_eq!(desk.windows.text(), "");
}

#[test]
fn a_field_that_cannot_be_changed_gets_no_keys() {
    let desk = Desk::new(Mode::Romanized);
    desk.windows.field.content.borrow_mut().read_only = true;
    let (vk, scan) = us_key('n');
    assert!(!desk.press(vk, scan));
    assert_eq!(desk.windows.text(), "");
}

#[test]
fn a_khmer_syllable_typed_out_of_order_is_stored_in_order() {
    let desk = Desk::new(Mode::Khmer);
    desk.type_keys("s");
    desk.type_shifted("i");
    desk.type_keys("jrjt");
    let syllable = "\u{179F}\u{17D2}\u{178F}\u{17D2}\u{179A}\u{17B8}";
    assert_eq!(desk.windows.text(), syllable);
    assert_eq!(desk.windows.composing().as_deref(), Some(syllable));

    desk.type_keys("k");
    assert_eq!(desk.windows.text(), format!("{syllable}\u{1780}"));
    assert_eq!(desk.windows.composing().as_deref(), Some("\u{1780}"));

    assert!(!desk.enter());
    assert_eq!(desk.windows.text(), format!("{syllable}\u{1780}"));
    desk.windows.assert_nothing_is_composing();
}

#[test]
fn khmer_space_is_the_invisible_word_break_and_shift_space_a_real_one() {
    let desk = Desk::new(Mode::Khmer);
    desk.type_keys("k ");
    desk.type_shifted(" ");
    assert_eq!(desk.windows.text(), "\u{1780}\u{200B} ");
    desk.windows.assert_nothing_is_composing();
}

#[test]
fn khmer_backspace_takes_the_last_part_off_the_syllable() {
    let desk = Desk::new(Mode::Khmer);
    desk.type_keys("ka");
    assert_eq!(desk.windows.text(), "\u{1780}\u{17B6}");
    assert!(desk.backspace());
    assert_eq!(desk.windows.text(), "\u{1780}");
    assert!(desk.backspace());
    assert_eq!(desk.windows.text(), "");
    desk.windows.assert_nothing_is_composing();
    assert!(!desk.backspace());
}

#[test]
fn switching_keyboard_finishes_what_was_being_typed() {
    let desk = Desk::new(Mode::Romanized);
    desk.type_keys("nh");
    desk.service.set_test_mode(Mode::Khmer);
    desk.type_keys("k");
    assert_eq!(desk.windows.text(), "ខ្ញុំ\u{1780}");
    assert_eq!(desk.windows.composing().as_deref(), Some("\u{1780}"));
}

#[test]
fn leaving_the_field_finishes_what_was_being_typed() {
    let desk = Desk::new(Mode::Romanized);
    desk.type_keys("nh jg");
    let focus: ITfThreadMgrEventSink = desk.service.to_interface();
    unsafe {
        focus
            .OnSetFocus(None, &desk.windows.document)
            .expect("the keyboard hears of it")
    };
    assert_eq!(desk.windows.text(), "ខ្ញុំចង់");
    desk.windows.assert_nothing_is_composing();
}

#[test]
fn the_app_may_end_the_phrase_itself() {
    let desk = Desk::new(Mode::Romanized);
    desk.type_keys("nh jg");
    assert_eq!(desk.windows.composing().as_deref(), Some("ខ្ញុំjg"));
    unsafe {
        let app: ITfContextOwnerCompositionServices = desk.windows.context.cast().expect("the app's side");
        app.TerminateComposition(None).expect("the composition ends");
    }
    assert_eq!(desk.windows.text(), "ខ្ញុំjg");
    desk.windows.assert_nothing_is_composing();
    assert!(desk.service.suggestions().0.is_empty());

    desk.type_keys("tv");
    assert_eq!(desk.windows.composing().as_deref(), Some("tv"));
    assert!(desk.enter());
    assert_eq!(desk.windows.text(), "ខ្ញុំjgទៅ");
}

fn made_by_the_dll() -> ITfTextInputProcessorEx {
    unsafe {
        let mut factory = std::ptr::null_mut();
        khmer_tsf::DllGetClassObject(&CLSID_TEXT_SERVICE, &IClassFactory::IID, &mut factory)
            .ok()
            .expect("the DLL knows its text service");
        IClassFactory::from_raw(factory)
            .CreateInstance(None)
            .expect("a text service")
    }
}

#[test]
fn the_dll_makes_a_text_service_that_answers_as_a_keyboard() {
    let service = made_by_the_dll();
    service.cast::<ITfKeyEventSink>().expect("it takes key presses");
    service
        .cast::<ITfThreadMgrEventSink>()
        .expect("it hears of focus changes");

    unsafe {
        let mut factory = std::ptr::null_mut();
        let other = GUID::from_u128(1);
        assert_eq!(
            khmer_tsf::DllGetClassObject(&other, &IClassFactory::IID, &mut factory),
            CLASS_E_CLASSNOTAVAILABLE
        );
        assert!(factory.is_null());
    }
}

#[test]
fn the_text_being_typed_is_underlined_with_dots() {
    let looks: ITfDisplayAttributeProvider = made_by_the_dll().cast().expect("it says how its text looks");
    unsafe {
        let list = looks.EnumDisplayAttributeInfo().expect("the list of looks");
        let mut items = [None, None];
        let mut fetched = 0;
        let _ = list.Next(&mut items, &mut fetched);
        assert_eq!(fetched, 1);
        let look = items[0].take().expect("one look");
        assert_eq!(look.GetGUID().expect("its identity"), DISPLAY_ATTRIBUTE_INPUT);
        let mut attribute = TF_DISPLAYATTRIBUTE::default();
        look.GetAttributeInfo(&mut attribute).expect("the look itself");
        assert_eq!(attribute.lsStyle, TF_LS_DOT);

        let same = looks
            .GetDisplayAttributeInfo(&DISPLAY_ATTRIBUTE_INPUT)
            .expect("found by identity");
        assert_eq!(same.GetGUID().expect("its identity"), DISPLAY_ATTRIBUTE_INPUT);
    }
}

#[test]
fn a_keyboard_windows_does_not_know_cannot_start() {
    let windows = Windows::with_field("", 0);
    let service = made_by_the_dll();
    unsafe {
        assert!(service
            .ActivateEx(&windows.thread_mgr, windows.client_id, TF_TMAE_SECUREMODE)
            .is_err());
        service.Deactivate().expect("turning it off is harmless");
    }
    assert_eq!(windows.text(), "");
}
