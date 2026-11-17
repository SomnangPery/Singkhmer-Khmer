//! The suggestion popup under the text being typed.
//!
//! ```text
//!  nh jg tv                 the Latin letters typed, small and grey
//!  ─────────────────
//! ▌1  ខ្ញុំចង់ទៅ             the highlighted suggestion
//!  2  ខ្ញុំចង់ទេ
//!  3  ខ្ញុំចង់ទិញ
//!  ─────────────────
//!  +  Add my word            opens the "my words" editor
//! ```
//!
//! It follows the system's light or dark setting, has rounded corners on
//! Windows 11, and is drawn off screen first so it does not flicker.
//! `layout` and `paint` are public so a test can draw it into a picture.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen, CreateSolidBrush,
    DeleteDC, DeleteObject, EndPaint, FillRect, GetDC, GetMonitorInfoW, GetStockObject, GetTextExtentPoint32W,
    GetTextMetricsW, InvalidateRect, MonitorFromPoint, Rectangle, ReleaseDC, RoundRect, SelectObject, SetBkMode,
    SetTextColor, TextOutW, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, FW_NORMAL, FW_SEMIBOLD, HDC,
    HFONT, HGDIOBJ, MONITORINFO, MONITOR_DEFAULTTONEAREST, NULL_BRUSH, NULL_PEN, OUT_DEFAULT_PRECIS, PAINTSTRUCT,
    PS_SOLID, SRCCOPY, TEXTMETRICW, TRANSPARENT,
};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, LoadCursorW, RegisterClassExW,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, CS_DROPSHADOW, GWLP_USERDATA, HWND_TOPMOST, IDC_HAND,
    MA_NOACTIVATE, SWP_NOACTIVATE, SW_HIDE, SW_SHOWNOACTIVATE, WM_ERASEBKGND, WM_LBUTTONUP, WM_MOUSEACTIVATE,
    WM_MOUSEMOVE, WM_PAINT, WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

/// Sent when the mouse leaves the window (asked for with TrackMouseEvent).
const WM_MOUSELEAVE: u32 = 0x02A3;

const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(r as u32 | (g as u32) << 8 | (b as u32) << 16)
}

/// The popup's colours.
#[derive(Clone, Copy)]
pub struct Theme {
    pub background: COLORREF,
    pub text: COLORREF,
    pub muted: COLORREF,
    pub highlight: COLORREF,
    pub hover: COLORREF,
    pub accent: COLORREF,
    pub border: COLORREF,
    pub divider: COLORREF,
}

pub const LIGHT: Theme = Theme {
    background: rgb(252, 252, 252),
    text: rgb(26, 26, 26),
    muted: rgb(112, 112, 112),
    highlight: rgb(229, 239, 252),
    hover: rgb(240, 240, 240),
    accent: rgb(0, 95, 184),
    border: rgb(216, 216, 216),
    divider: rgb(234, 234, 234),
};

pub const DARK: Theme = Theme {
    background: rgb(43, 43, 43),
    text: rgb(242, 242, 242),
    muted: rgb(160, 160, 160),
    highlight: rgb(52, 70, 96),
    hover: rgb(56, 56, 56),
    accent: rgb(118, 185, 255),
    border: rgb(72, 72, 72),
    divider: rgb(60, 60, 60),
};

/// The theme the user picked for apps in Windows settings.
pub fn system_theme() -> Theme {
    let mut light: u32 = 1;
    let mut size = std::mem::size_of::<u32>() as u32;
    let found = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut light as *mut u32 as *mut _),
            Some(&mut size),
        )
    };
    if found.is_ok() && light == 0 {
        DARK
    } else {
        LIGHT
    }
}

/// The two fonts: a large one for Khmer, a small one for everything else.
pub struct Fonts {
    pub khmer: HFONT,
    pub small: HFONT,
    pub small_bold: HFONT,
    dpi: i32,
}

impl Fonts {
    pub fn new(dpi: i32) -> Fonts {
        Fonts {
            // Leelawadee UI comes with every Windows 10 and 11 and has Khmer.
            khmer: font(18, FW_NORMAL.0 as i32, w!("Leelawadee UI"), dpi),
            small: font(12, FW_NORMAL.0 as i32, w!("Segoe UI"), dpi),
            small_bold: font(12, FW_SEMIBOLD.0 as i32, w!("Segoe UI"), dpi),
            dpi,
        }
    }

    fn px(&self, value: i32) -> i32 {
        value * self.dpi / 96
    }
}

impl Drop for Fonts {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.khmer);
            let _ = DeleteObject(self.small);
            let _ = DeleteObject(self.small_bold);
        }
    }
}

fn font(size: i32, weight: i32, face: PCWSTR, dpi: i32) -> HFONT {
    unsafe {
        CreateFontW(
            -(size * dpi / 96),
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET.0 as u32,
            OUT_DEFAULT_PRECIS.0 as u32,
            CLIP_DEFAULT_PRECIS.0 as u32,
            CLEARTYPE_QUALITY.0 as u32,
            0,
            face,
        )
    }
}

/// What the popup shows.
#[derive(Clone, Default)]
pub struct View {
    /// The Latin letters typed. Not shown when empty.
    pub header: String,
    pub items: Vec<String>,
    pub highlight: usize,
    /// The text of the button at the bottom. No button when `None`.
    pub footer: Option<String>,
    /// What the mouse is over.
    pub hover: Option<Hit>,
}

/// A part of the popup that can be clicked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Item(usize),
    Footer,
}

/// Where everything goes, in pixels from the popup's top left corner.
#[derive(Clone, Default)]
pub struct Layout {
    pub size: SIZE,
    header: Option<(i32, i32)>,
    rows: Vec<RECT>,
    footer: Option<RECT>,
    dividers: Vec<i32>,
    number_left: i32,
    text_left: i32,
    khmer_height: i32,
    small_height: i32,
}

impl Layout {
    pub fn hit(&self, x: i32, y: i32) -> Option<Hit> {
        let inside = |rect: &RECT| x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom;
        if let Some(index) = self.rows.iter().position(inside) {
            return Some(Hit::Item(index));
        }
        self.footer.as_ref().filter(|rect| inside(rect)).map(|_| Hit::Footer)
    }
}

fn text_size(hdc: HDC, font: HFONT, text: &str) -> SIZE {
    let mut size = SIZE::default();
    unsafe {
        let old = SelectObject(hdc, font);
        let _ = GetTextExtentPoint32W(hdc, &utf16(text), &mut size);
        SelectObject(hdc, old);
    }
    size
}

fn line_height(hdc: HDC, font: HFONT) -> i32 {
    let mut metrics = TEXTMETRICW::default();
    unsafe {
        let old = SelectObject(hdc, font);
        let _ = GetTextMetricsW(hdc, &mut metrics);
        SelectObject(hdc, old);
    }
    metrics.tmHeight
}

fn utf16(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// Works out the size of the popup and where each part goes.
pub fn layout(hdc: HDC, fonts: &Fonts, view: &View) -> Layout {
    let px = |value| fonts.px(value);
    let edge = px(4);
    let side = px(12);
    let khmer_height = line_height(hdc, fonts.khmer);
    let small_height = line_height(hdc, fonts.small);
    let number_width = text_size(hdc, fonts.small_bold, "8").cx;
    let number_left = side;
    let text_left = side + number_width + px(12);

    let mut widest = text_size(hdc, fonts.small, &view.header).cx + 2 * side;
    for item in &view.items {
        widest = widest.max(text_left + text_size(hdc, fonts.khmer, item).cx + side);
    }
    if let Some(footer) = &view.footer {
        widest = widest.max(text_left + text_size(hdc, fonts.small, footer).cx + side);
    }
    let width = widest.max(px(180));

    let mut layout = Layout {
        number_left,
        text_left,
        khmer_height,
        small_height,
        ..Layout::default()
    };
    let mut y = edge;
    if !view.header.is_empty() {
        layout.header = Some((side, y + px(4)));
        y += small_height + px(8);
        layout.dividers.push(y);
        y += 1 + px(3);
    }
    let row_height = khmer_height + px(8);
    for _ in &view.items {
        layout.rows.push(RECT { left: edge, top: y, right: width - edge, bottom: y + row_height });
        y += row_height;
    }
    if view.footer.is_some() {
        y += px(3);
        layout.dividers.push(y);
        y += 1 + px(3);
        let footer_height = small_height + px(12);
        layout.footer = Some(RECT { left: edge, top: y, right: width - edge, bottom: y + footer_height });
        y += footer_height;
    }
    layout.size = SIZE { cx: width, cy: y + edge };
    layout
}

/// Draws the popup. `border` draws a one-pixel frame, for when Windows does
/// not draw one itself (Windows 10, or a picture in a test).
pub fn paint(hdc: HDC, fonts: &Fonts, theme: &Theme, layout: &Layout, view: &View, border: bool) {
    let px = |value| fonts.px(value);
    let size = layout.size;
    unsafe {
        fill(hdc, RECT { left: 0, top: 0, right: size.cx, bottom: size.cy }, theme.background);
        SetBkMode(hdc, TRANSPARENT);

        if let Some((x, y)) = layout.header {
            text(hdc, fonts.small, theme.muted, x, y, &view.header);
        }
        for &y in &layout.dividers {
            fill(hdc, RECT { left: px(10), top: y, right: size.cx - px(10), bottom: y + 1 }, theme.divider);
        }

        for (index, (row, item)) in layout.rows.iter().zip(&view.items).enumerate() {
            let highlighted = index == view.highlight;
            // A little room between neighbouring shaded rows.
            let row = &RECT { top: row.top + px(1), bottom: row.bottom - px(1), ..*row };
            if highlighted {
                round(hdc, *row, px(6), theme.highlight);
                // A small bar on the left, as Windows 11 marks a selection.
                let bar_height = (row.bottom - row.top) * 2 / 5;
                let top = row.top + (row.bottom - row.top - bar_height) / 2;
                round(
                    hdc,
                    RECT { left: row.left + px(1), top, right: row.left + px(4), bottom: top + bar_height },
                    px(3),
                    theme.accent,
                );
            } else if view.hover == Some(Hit::Item(index)) {
                round(hdc, *row, px(6), theme.hover);
            }
            let middle = (row.top + row.bottom) / 2;
            let (number_font, number_colour) =
                if highlighted { (fonts.small_bold, theme.accent) } else { (fonts.small, theme.muted) };
            text(
                hdc,
                number_font,
                number_colour,
                layout.number_left,
                middle - layout.small_height / 2,
                &(index + 1).to_string(),
            );
            text(hdc, fonts.khmer, theme.text, layout.text_left, middle - layout.khmer_height / 2, item);
        }

        if let (Some(rect), Some(footer)) = (layout.footer, &view.footer) {
            let hovered = view.hover == Some(Hit::Footer);
            if hovered {
                round(hdc, rect, px(6), theme.hover);
            }
            let colour = if hovered { theme.accent } else { theme.muted };
            let y = (rect.top + rect.bottom) / 2 - layout.small_height / 2;
            text(hdc, fonts.small_bold, colour, layout.number_left, y, "+");
            text(hdc, fonts.small, colour, layout.text_left, y, footer);
        }

        if border {
            let pen = CreatePen(PS_SOLID, 1, theme.border);
            let old_pen = SelectObject(hdc, pen);
            let old_brush = SelectObject(hdc, HGDIOBJ(GetStockObject(NULL_BRUSH).0));
            let _ = Rectangle(hdc, 0, 0, size.cx, size.cy);
            SelectObject(hdc, old_brush);
            SelectObject(hdc, old_pen);
            let _ = DeleteObject(pen);
        }
    }
}

unsafe fn fill(hdc: HDC, rect: RECT, colour: COLORREF) {
    let brush = CreateSolidBrush(colour);
    FillRect(hdc, &rect, brush);
    let _ = DeleteObject(brush);
}

unsafe fn round(hdc: HDC, rect: RECT, radius: i32, colour: COLORREF) {
    let brush = CreateSolidBrush(colour);
    let old_brush = SelectObject(hdc, brush);
    let old_pen = SelectObject(hdc, HGDIOBJ(GetStockObject(NULL_PEN).0));
    // With no pen, RoundRect leaves out the last row and column: add them.
    let _ = RoundRect(hdc, rect.left, rect.top, rect.right + 1, rect.bottom + 1, radius * 2, radius * 2);
    SelectObject(hdc, old_pen);
    SelectObject(hdc, old_brush);
    let _ = DeleteObject(brush);
}

unsafe fn text(hdc: HDC, font: HFONT, colour: COLORREF, x: i32, y: i32, text: &str) {
    let old = SelectObject(hdc, font);
    SetTextColor(hdc, colour);
    let _ = TextOutW(hdc, x, y, &utf16(text));
    SelectObject(hdc, old);
}

/// What the window procedure reads. Everything that changes is in a `Cell`
/// or `RefCell`, so the window and its owner only ever share it.
struct Content {
    view: RefCell<View>,
    layout: RefCell<Layout>,
    theme: Cell<Theme>,
    fonts: Fonts,
    /// Windows 11 draws the frame (and rounds the corners) itself.
    system_border: bool,
    tracking_mouse: Cell<bool>,
    on_pick: RefCell<Option<Rc<dyn Fn(usize)>>>,
    on_footer: RefCell<Option<Rc<dyn Fn()>>>,
}

pub struct CandidateWindow {
    hwnd: HWND,
    content: Box<Content>,
}

impl CandidateWindow {
    pub fn new(instance: HINSTANCE) -> Option<CandidateWindow> {
        let class_name = w!("KhmerKeyboardCandidates");
        unsafe {
            let class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_DROPSHADOW,
                lpfnWndProc: Some(window_proc),
                hInstance: instance,
                hCursor: LoadCursorW(None, IDC_HAND).unwrap_or_default(),
                lpszClassName: class_name,
                ..Default::default()
            };
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
            let corner = DWMWCP_ROUND;
            let system_border = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &corner as *const _ as *const _,
                std::mem::size_of_val(&corner) as u32,
            )
            .is_ok();
            let content = Box::new(Content {
                view: RefCell::new(View::default()),
                layout: RefCell::new(Layout::default()),
                theme: Cell::new(LIGHT),
                fonts: Fonts::new(dpi),
                system_border,
                tracking_mouse: Cell::new(false),
                on_pick: RefCell::new(None),
                on_footer: RefCell::new(None),
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, &*content as *const Content as isize);
            Some(CandidateWindow { hwnd, content })
        }
    }

    pub fn on_pick(&mut self, on_pick: Rc<dyn Fn(usize)>) {
        *self.content.on_pick.borrow_mut() = Some(on_pick);
    }

    pub fn on_footer(&mut self, on_footer: Rc<dyn Fn()>) {
        *self.content.on_footer.borrow_mut() = Some(on_footer);
    }

    /// Shows `view` under `anchor`, the place of the text being typed on
    /// screen (or over it, near the bottom of the screen).
    pub fn show(&mut self, mut view: View, anchor: RECT) {
        let content = &*self.content;
        view.hover = content.view.borrow().hover;
        let theme = system_theme();
        content.theme.set(theme);
        unsafe {
            if content.system_border {
                let _ = DwmSetWindowAttribute(
                    self.hwnd,
                    DWMWA_BORDER_COLOR,
                    &theme.border as *const _ as *const _,
                    std::mem::size_of::<COLORREF>() as u32,
                );
            }
            let hdc = GetDC(self.hwnd);
            let layout = layout(hdc, &content.fonts, &view);
            ReleaseDC(self.hwnd, hdc);
            let size = layout.size;
            *content.view.borrow_mut() = view;
            *content.layout.borrow_mut() = layout;

            let gap = content.fonts.px(4);
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
            let _ = InvalidateRect(self.hwnd, None, false);
        }
    }

    pub fn highlight(&mut self, highlight: usize) {
        self.content.view.borrow_mut().highlight = highlight;
        unsafe {
            let _ = InvalidateRect(self.hwnd, None, false);
        }
    }

    pub fn hide(&mut self) {
        self.content.view.borrow_mut().hover = None;
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }
}

impl Drop for CandidateWindow {
    fn drop(&mut self) {
        unsafe {
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

fn set_hover(hwnd: HWND, content: &Content, hover: Option<Hit>) {
    let changed = {
        let mut view = content.view.borrow_mut();
        let changed = view.hover != hover;
        view.hover = hover;
        changed
    };
    if changed {
        unsafe {
            let _ = InvalidateRect(hwnd, None, false);
        }
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let content = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Content;
    if content.is_null() {
        return DefWindowProcW(hwnd, message, wparam, lparam);
    }
    let content = &*content;
    let x = (lparam.0 & 0xFFFF) as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
    match message {
        WM_PAINT => {
            let mut paint_info = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut paint_info);
            let layout = content.layout.borrow().clone();
            let view = content.view.borrow().clone();
            let size = layout.size;
            // Drawn off screen and copied in one go, so it never flickers.
            let memory = CreateCompatibleDC(hdc);
            let bitmap = CreateCompatibleBitmap(hdc, size.cx, size.cy);
            let old = SelectObject(memory, bitmap);
            paint(memory, &content.fonts, &content.theme.get(), &layout, &view, !content.system_border);
            let _ = BitBlt(hdc, 0, 0, size.cx, size.cy, memory, 0, 0, SRCCOPY);
            SelectObject(memory, old);
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(memory);
            let _ = EndPaint(hwnd, &paint_info);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_MOUSEMOVE => {
            if !content.tracking_mouse.get() {
                let mut track = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                content.tracking_mouse.set(TrackMouseEvent(&mut track).is_ok());
            }
            let hover = content.layout.borrow().hit(x, y);
            set_hover(hwnd, content, hover);
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            content.tracking_mouse.set(false);
            set_hover(hwnd, content, None);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            // Nothing is borrowed during the call: it may show this window
            // again, or close it.
            let hit = content.layout.borrow().hit(x, y);
            match hit {
                Some(Hit::Item(index)) => {
                    let on_pick = content.on_pick.borrow().clone();
                    if let Some(on_pick) = on_pick {
                        on_pick(index);
                    }
                }
                Some(Hit::Footer) => {
                    let on_footer = content.on_footer.borrow().clone();
                    if let Some(on_footer) = on_footer {
                        on_footer();
                    }
                }
                None => {}
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}
