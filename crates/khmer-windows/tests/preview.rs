//! Draws the suggestion window into a picture, to look at.
//!
//! The picture is saved as `suggestion-window.bmp` in the build folder's `tmp`
//! (the test prints the full path). Look at it after changing `candidates.rs`.
#![cfg(windows)]

use std::path::PathBuf;

use khmer_core::{Engine, Mode};
use khmer_tsf::candidates::{create_font, measure, metrics, paint};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GdiFlush, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, DIB_RGB_COLORS,
};

/// The suggestions the keyboard shows for these keys.
fn suggestions(keys: &str) -> Vec<String> {
    let mut engine = Engine::new();
    engine.set_mode(Mode::Romanized);
    for key in keys.chars() {
        engine.on_key(key);
    }
    engine.candidates()
}

/// A .bmp file of `pixels`: rows from the top, four bytes a pixel (blue,
/// green, red, unused).
fn bmp(width: i32, height: i32, pixels: &[u8]) -> Vec<u8> {
    let mut file = Vec::with_capacity(54 + pixels.len());
    file.extend(b"BM");
    file.extend((54 + pixels.len() as u32).to_le_bytes());
    file.extend(0u32.to_le_bytes());
    file.extend(54u32.to_le_bytes());
    file.extend(40u32.to_le_bytes());
    file.extend(width.to_le_bytes());
    // A negative height means the rows run from the top.
    file.extend((-height).to_le_bytes());
    file.extend(1u16.to_le_bytes());
    file.extend(32u16.to_le_bytes());
    file.extend([0u8; 24]);
    file.extend(pixels);
    file
}

#[test]
fn the_suggestion_window_is_drawn() {
    let items = suggestions("nh jg tv");
    assert!(items.len() > 1, "there are suggestions to draw: {items:?}");
    // 144 is a screen set to 150%, a common laptop setting.
    let dpi = 144;
    let highlighted = 1;

    let (width, height, pixels) = unsafe {
        let hdc = CreateCompatibleDC(None);
        let font = create_font(dpi);
        let sizes = metrics(hdc, font, dpi);
        let size = measure(hdc, font, sizes, &items);
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
        paint(hdc, font, sizes, size, &items, highlighted);
        let _ = GdiFlush();
        let pixels = std::slice::from_raw_parts(bits as *const u8, (size.cx * size.cy * 4) as usize).to_vec();
        SelectObject(hdc, old);
        let _ = DeleteObject(bitmap);
        let _ = DeleteObject(font);
        let _ = DeleteDC(hdc);
        (size.cx, size.cy, pixels)
    };

    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("suggestion-window.bmp");
    std::fs::write(&path, bmp(width, height, &pixels)).expect("the picture is saved");
    println!("saved {}", path.display());

    // Not a blank picture: there is white background, the highlight colour
    // behind one row, and dark text.
    let colours: Vec<[u8; 3]> = pixels.chunks_exact(4).map(|pixel| [pixel[2], pixel[1], pixel[0]]).collect();
    let count = |wanted: [u8; 3]| colours.iter().filter(|&&colour| colour == wanted).count();
    let all = colours.len();
    assert!(count([0xFF, 0xFF, 0xFF]) > all / 4, "the background is white");
    assert!(count([0xD9, 0xEB, 0xFB]) > all / 20, "one row is highlighted");
    assert!(count([0x20, 0x20, 0x20]) > 50, "text is drawn");
}
