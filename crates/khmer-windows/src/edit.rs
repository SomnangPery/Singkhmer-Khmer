use windows::core::{implement, Result};
use windows::Win32::Foundation::{FALSE, HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, ITfRange, TF_AE_NONE, TF_ANCHOR_END, TF_ES_ASYNCDONTCARE,
    TF_ES_READWRITE, TF_ES_SYNC, TF_SELECTION, TF_SELECTIONSTYLE,
};
use windows::Win32::UI::WindowsAndMessaging::{GetGUIThreadInfo, GUITHREADINFO};

use crate::service::guard;

#[implement(ITfEditSession)]
struct EditSession {
    run: Box<dyn Fn(u32) -> Result<()>>,
}

impl ITfEditSession_Impl for EditSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        guard(|| (self.run)(ec))
    }
}

pub(crate) fn edit(client_id: u32, context: &ITfContext, run: impl Fn(u32) -> Result<()> + 'static) -> Result<()> {
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

pub(crate) unsafe fn caret_to_end(ec: u32, context: &ITfContext, range: &ITfRange) -> Result<()> {
    let end = range.Clone()?;
    end.Collapse(ec, TF_ANCHOR_END)?;
    let selection = TF_SELECTION {
        range: std::mem::ManuallyDrop::new(Some(end)),
        style: TF_SELECTIONSTYLE {
            ase: TF_AE_NONE,
            fInterimChar: FALSE,
        },
    };
    let result = context.SetSelection(ec, &[selection.clone()]);
    drop(std::mem::ManuallyDrop::into_inner(selection.range));
    result
}

pub(crate) unsafe fn text_place(ec: u32, context: &ITfContext, range: &ITfRange) -> Option<RECT> {
    let view = context.GetActiveView().ok()?;
    let mut place = RECT::default();
    let mut clipped = FALSE;
    view.GetTextExt(ec, range, &mut place, &mut clipped).ok()?;
    (place != RECT::default()).then_some(place)
}

pub(crate) fn caret_place() -> Option<RECT> {
    unsafe {
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        GetGUIThreadInfo(0, &mut info).ok()?;
        if info.hwndCaret == HWND::default() {
            return None;
        }
        let mut top_left = POINT {
            x: info.rcCaret.left,
            y: info.rcCaret.top,
        };
        let mut bottom_right = POINT {
            x: info.rcCaret.right,
            y: info.rcCaret.bottom,
        };
        let _ = ClientToScreen(info.hwndCaret, &mut top_left);
        let _ = ClientToScreen(info.hwndCaret, &mut bottom_right);
        Some(RECT {
            left: top_left.x,
            top: top_left.y,
            right: bottom_right.x,
            bottom: bottom_right.y,
        })
    }
}
