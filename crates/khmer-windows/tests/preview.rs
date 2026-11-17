#![cfg(windows)]

use std::path::PathBuf;

use khmer_core::{Engine, Mode};
use khmer_tsf::candidates::{layout, paint, Fonts, Hit, Theme, View, DARK, LIGHT};
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GdiFlush, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
    DIB_RGB_COLORS,
};

fn suggestions(keys: &str) -> Vec<String> {
    let mut engine = Engine::new();
    engine.set_mode(Mode::Romanized);
    for key in keys.chars() {
        engine.on_key(key);
    }
    engine.candidates()
}

fn bmp(width: i32, height: i32, pixels: &[u8]) -> Vec<u8> {
    let mut file = Vec::with_capacity(54 + pixels.len());
    file.extend(b"BM");
    file.extend((54 + pixels.len() as u32).to_le_bytes());
    file.extend(0u32.to_le_bytes());
    file.extend(54u32.to_le_bytes());
    file.extend(40u32.to_le_bytes());
    file.extend(width.to_le_bytes());
    file.extend((-height).to_le_bytes());
    file.extend(1u16.to_le_bytes());
    file.extend(32u16.to_le_bytes());
    file.extend([0u8; 24]);
    file.extend(pixels);
    file
}

fn draw(view: &View, theme: &Theme, dpi: i32) -> (i32, i32, Vec<u8>) {
    unsafe {
        let hdc = CreateCompatibleDC(None);
        let fonts = Fonts::new(dpi);
        let layout = layout(hdc, &fonts, view);
        let size = layout.size;
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size.cx,
                biHeight: -size.cy,
                biPlanes: 1,
                biBitCount: 32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(hdc, &info, DIB_RGB_COLORS, &mut bits, None, 0).expect("a bitmap to draw on");
        let old = SelectObject(hdc, bitmap);
        paint(hdc, &fonts, theme, &layout, view, true);
        let _ = GdiFlush();
        let pixels = std::slice::from_raw_parts(bits as *const u8, (size.cx * size.cy * 4) as usize).to_vec();
        SelectObject(hdc, old);
        let _ = DeleteObject(bitmap);
        drop(fonts);
        let _ = DeleteDC(hdc);
        (size.cx, size.cy, pixels)
    }
}

fn rgb(colour: COLORREF) -> [u8; 3] {
    let value = colour.0;
    [value as u8, (value >> 8) as u8, (value >> 16) as u8]
}

#[test]
fn the_suggestion_window_is_drawn() {
    let items = suggestions("nh jg tv");
    assert!(items.len() > 1, "there are suggestions to draw: {items:?}");
    let view = View {
        header: "nh jg tv".to_string(),
        items,
        highlight: 0,
        footer: Some("Add my word".to_string()),
        hover: Some(Hit::Item(1)),
    };

    for (name, theme) in [("light", LIGHT), ("dark", DARK)] {
        let (width, height, pixels) = draw(&view, &theme, 144);
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("suggestion-window-{name}.bmp"));
        std::fs::write(&path, bmp(width, height, &pixels)).expect("the picture is saved");
        println!("saved {}", path.display());

        let colours: Vec<[u8; 3]> = pixels.chunks_exact(4).map(|pixel| [pixel[2], pixel[1], pixel[0]]).collect();
        let count = |wanted: COLORREF| colours.iter().filter(|&&colour| colour == rgb(wanted)).count();
        let all = colours.len();
        assert!(count(theme.background) > all / 4, "{name}: the background");
        assert!(count(theme.highlight) > all / 30, "{name}: one row is highlighted");
        assert!(count(theme.hover) > all / 30, "{name}: the row under the mouse");
        assert!(count(theme.accent) > 10, "{name}: the highlight's bar");
        assert!(count(theme.text) > 50, "{name}: text is drawn");
    }
}

#[test]
fn clicks_find_the_row_or_the_button() {
    let view = View {
        header: "nh".to_string(),
        items: vec!["ខ្ញុំ".to_string(), "ញ៉ាំ".to_string()],
        highlight: 0,
        footer: Some("Add my word".to_string()),
        hover: None,
    };
    let layout = unsafe {
        let hdc = CreateCompatibleDC(None);
        let fonts = Fonts::new(96);
        let layout = layout(hdc, &fonts, &view);
        drop(fonts);
        let _ = DeleteDC(hdc);
        layout
    };
    let size = layout.size;
    let x = size.cx / 2;
    let hits: Vec<_> = (0..size.cy).filter_map(|y| layout.hit(x, y)).collect();
    assert_eq!(hits.first(), Some(&Hit::Item(0)));
    assert!(hits.contains(&Hit::Item(1)));
    assert_eq!(hits.last(), Some(&Hit::Footer));
    assert_eq!(layout.hit(x, 2), None, "the header is not a button");
}
