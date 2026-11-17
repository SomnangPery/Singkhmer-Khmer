//! The small window that lists the suggestions next to the text being typed.
//!
//! It is a plain Win32 window that never takes the keyboard focus, so typing
//! carries on in the app while it shows. The suggestions are drawn one under
//! the other, numbered, with the chosen one highlighted. Clicking one picks it.

use std::rc::Rc;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, EndPaint, FillRect, GetDC,
    GetMonitorInfoW, GetStockObject, GetTextExtentPoint32W, InvalidateRect, MonitorFromPoint, Rectangle,
    ReleaseDC, SelectObject, SetBkMode, SetTextColor, TextOutW, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS,
    DEFAULT_CHARSET, FW_NORMAL, HDC, HFONT, HGDIOBJ, MONITORINFO, MONITOR_DEFAULTTONEAREST, NULL_BRUSH,
    OUT_DEFAULT_PRECIS, PAINTSTRUCT, PS_SOLID, TRANSPARENT,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, LoadCursorW, RegisterClassExW,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, CS_DROPSHADOW, GWLP_USERDATA, HWND_TOPMOST, IDC_ARROW,
    MA_NOACTIVATE, SWP_NOACTIVATE, SW_HIDE, SW_SHOWNOACTIVATE, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_PAINT,
    WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const BACKGROUND: COLORREF = COLORREF(0x00FF_FFFF);
const BORDER: COLORREF = COLORREF(0x00B4_B4B4);
const HIGHLIGHT: COLORREF = COLORREF(0x00FB_EBD9);
const TEXT: COLORREF = COLORREF(0x0020_2020);
const NUMBER: COLORREF = COLORREF(0x0080_8080);

/// Sizes in pixels, worked out from the screen's scaling and the font.
#[derive(Clone, Copy)]
pub struct Metrics {
    row_height: i32,
    padding: i32,
    /// Where the suggestion text starts, after the number.
    text_left: i32,
}

/// What the window shows.
struct Content {
    items: Vec<String>,
    highlight: usize,
    metrics: Metrics,
    font: HFONT,
    on_pick: Option<Rc<dyn Fn(usize)>>,
}

pub struct CandidateWindow {
    hwnd: HWND,
    /// Boxed so the window can keep a pointer to it.
    content: Box<Content>,
}

impl CandidateWindow {
    /// Makes the window, hidden. `instance` is the module that holds the
    /// window's code: the DLL.
    pub fn new(instance: HINSTANCE) -> Option<CandidateWindow> {
        let class_name = w!("KhmerKeyboardCandidates");
        unsafe {
            let class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_DROPSHADOW,
                lpfnWndProc: Some(window_proc),
                hInstance: instance,
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                lpszClassName: class_name,
                ..Default::default()
            };
            // Fails harmlessly when the class is already there from an earlier window.
            RegisterClassExW(&class);
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class_name,
                PCWSTR::null(),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                instance,
                None,
            )
            .ok()?;
            let dpi = GetDpiForWindow(hwnd).max(96) as i32;
            let font = create_font(dpi);
            let hdc = GetDC(hwnd);
            let metrics = metrics(hdc, font, dpi);
            ReleaseDC(hwnd, hdc);
            let mut content =
                Box::new(Content { items: Vec::new(), highlight: 0, metrics, font, on_pick: None });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, &mut *content as *mut Content as isize);
            Some(CandidateWindow { hwnd, content })
        }
    }

    /// Sets what happens when a suggestion is clicked.
    pub fn on_pick(&mut self, on_pick: Rc<dyn Fn(usize)>) {
        self.content.on_pick = Some(on_pick);
    }

    /// Shows `items` under `anchor`, the place on screen of the text being
    /// typed, or above it when there is no room below.
    pub fn show(&mut self, items: Vec<String>, highlight: usize, anchor: RECT) {
        self.content.items = items;
        self.content.highlight = highlight;
        unsafe {
            let hdc = GetDC(self.hwnd);
            let size = measure(hdc, self.content.font, self.content.metrics, &self.content.items);
            ReleaseDC(self.hwnd, hdc);

            let gap = self.content.metrics.padding / 2;
            let mut x = anchor.left;
            let mut y = anchor.bottom + gap;
            let monitor = MonitorFromPoint(POINT { x: anchor.left, y: anchor.bottom }, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            if GetMonitorInfoW(monitor, &mut info).as_bool() {
                let work = info.rcWork;
                if y + size.cy > work.bottom {
                    y = anchor.top - gap - size.cy;
                }
                x = x.min(work.right - size.cx).max(work.left);
                y = y.max(work.top);
            }
            let _ = SetWindowPos(self.hwnd, HWND_TOPMOST, x, y, size.cx, size.cy, SWP_NOACTIVATE);
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            let _ = InvalidateRect(self.hwnd, None, true);
        }
    }

    /// Moves the highlight without moving the window.
    pub fn highlight(&mut self, highlight: usize) {
        self.content.highlight = highlight;
        unsafe {
            let _ = InvalidateRect(self.hwnd, None, true);
        }
    }

    pub fn hide(&mut self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }
}

impl Drop for CandidateWindow {
    fn drop(&mut self) {
        unsafe {
            // The window must stop pointing at the content before it goes.
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.hwnd);
            let _ = DeleteObject(self.content.font);
        }
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let content = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Content;
    if content.is_null() {
        return DefWindowProcW(hwnd, message, wparam, lparam);
    }
    match message {
        WM_PAINT => {
            let content = &*content;
            let mut paint_info = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut paint_info);
            let size = measure(hdc, content.font, content.metrics, &content.items);
            paint(hdc, content.font, content.metrics, size, &content.items, content.highlight);
            let _ = EndPaint(hwnd, &paint_info);
            LRESULT(0)
        }
        // A click must not take the keyboard away from the app.
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_LBUTTONUP => {
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
            // Copy what is needed first: picking changes the content.
            let (index, count, on_pick) = {
                let content = &*content;
                let index = (y - content.metrics.padding / 2) / content.metrics.row_height.max(1);
                (index, content.items.len(), content.on_pick.clone())
            };
            if let (Ok(index), Some(on_pick)) = (usize::try_from(index), on_pick) {
                if index < count {
                    on_pick(index);
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

/// A font with Khmer letters. Leelawadee UI is the one Windows 10 and 11 use
/// for Khmer; an older Windows picks its own Khmer font instead.
pub fn create_font(dpi: i32) -> HFONT {
    unsafe {
        CreateFontW(
            -(15 * dpi / 96),
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET.0 as u32,
            OUT_DEFAULT_PRECIS.0 as u32,
            CLIP_DEFAULT_PRECIS.0 as u32,
            CLEARTYPE_QUALITY.0 as u32,
            0,
            w!("Leelawadee UI"),
        )
    }
}

pub fn metrics(hdc: HDC, font: HFONT, dpi: i32) -> Metrics {
    unsafe {
        let old = SelectObject(hdc, font);
        let mut number = SIZE::default();
        let digits: Vec<u16> = "8.".encode_utf16().collect();
        let _ = GetTextExtentPoint32W(hdc, &digits, &mut number);
        SelectObject(hdc, old);
        let padding = 8 * dpi / 96;
        Metrics {
            // Khmer letters stack marks above and below, so rows are tall.
            row_height: number.cy * 3 / 2,
            padding,
            text_left: padding + number.cx + padding,
        }
    }
}

/// The size the window needs for `items`.
pub fn measure(hdc: HDC, font: HFONT, metrics: Metrics, items: &[String]) -> SIZE {
    let mut widest = 0;
    unsafe {
        let old = SelectObject(hdc, font);
        for item in items {
            let text: Vec<u16> = item.encode_utf16().collect();
            let mut size = SIZE::default();
            let _ = GetTextExtentPoint32W(hdc, &text, &mut size);
            widest = widest.max(size.cx);
        }
        SelectObject(hdc, old);
    }
    SIZE {
        cx: metrics.text_left + widest + metrics.padding * 3 / 2,
        cy: metrics.row_height * items.len() as i32 + metrics.padding,
    }
}

/// Draws the suggestions into `hdc`.
pub fn paint(hdc: HDC, font: HFONT, metrics: Metrics, size: SIZE, items: &[String], highlight: usize) {
    unsafe {
        let background = CreateSolidBrush(BACKGROUND);
        FillRect(hdc, &RECT { left: 0, top: 0, right: size.cx, bottom: size.cy }, background);
        let _ = DeleteObject(background);

        let old_font = SelectObject(hdc, font);
        SetBkMode(hdc, TRANSPARENT);
        let top = metrics.padding / 2;
        for (index, item) in items.iter().enumerate() {
            let row_top = top + index as i32 * metrics.row_height;
            if index == highlight {
                let brush = CreateSolidBrush(HIGHLIGHT);
                let row = RECT { left: 1, top: row_top, right: size.cx - 1, bottom: row_top + metrics.row_height };
                FillRect(hdc, &row, brush);
                let _ = DeleteObject(brush);
            }
            // TextOut puts the top of the font's line at y: centre the line in the row.
            let line_height = metrics.row_height * 2 / 3;
            let y = row_top + (metrics.row_height - line_height) / 2;
            SetTextColor(hdc, NUMBER);
            let number: Vec<u16> = format!("{}.", index + 1).encode_utf16().collect();
            let _ = TextOutW(hdc, metrics.padding, y, &number);
            SetTextColor(hdc, TEXT);
            let text: Vec<u16> = item.encode_utf16().collect();
            let _ = TextOutW(hdc, metrics.text_left, y, &text);
        }
        SelectObject(hdc, old_font);

        let pen = CreatePen(PS_SOLID, 1, BORDER);
        let old_pen = SelectObject(hdc, pen);
        let old_brush = SelectObject(hdc, HGDIOBJ(GetStockObject(NULL_BRUSH).0));
        let _ = Rectangle(hdc, 0, 0, size.cx, size.cy);
        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        let _ = DeleteObject(pen);
    }
}
