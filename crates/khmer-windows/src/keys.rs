use khmer_core::Mode;

pub const VK_BACK: u16 = 0x08;
pub const VK_TAB: u16 = 0x09;
pub const VK_RETURN: u16 = 0x0D;
pub const VK_SHIFT: u16 = 0x10;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_MENU: u16 = 0x12;
pub const VK_CAPITAL: u16 = 0x14;
pub const VK_ESCAPE: u16 = 0x1B;
pub const VK_SPACE: u16 = 0x20;
pub const VK_PRIOR: u16 = 0x21;
pub const VK_DOWN: u16 = 0x28;
pub const VK_INSERT: u16 = 0x2D;
pub const VK_DELETE: u16 = 0x2E;
pub const VK_UP: u16 = 0x26;
const VK_LWIN: u16 = 0x5B;
const VK_RWIN: u16 = 0x5C;
const VK_NUMPAD0: u16 = 0x60;
const VK_DIVIDE: u16 = 0x6F;
const VK_NUMLOCK: u16 = 0x90;
const VK_SCROLL: u16 = 0x91;
const VK_LSHIFT: u16 = 0xA0;
const VK_RMENU: u16 = 0xA5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub caps: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyAction {
    Pass { forget: bool },
    Text(&'static str),
    Space,
    Backspace,
    Commit,
    Cancel,
    Next,
    Previous,
    Ignore,
}

struct KeyRow {
    scan: u8,
    latin: [&'static str; 2],
    khmer: [&'static str; 3],
}

const fn key(scan: u8, latin: [&'static str; 2], khmer: [&'static str; 3]) -> KeyRow {
    KeyRow { scan, latin, khmer }
}

#[rustfmt::skip]
const KEYS: &[KeyRow] = &[
    key(0x29, ["`", "~"], ["«", "»", "\u{200D}"]),
    key(0x02, ["1", "!"], ["១", "!", "\u{200C}"]),
    key(0x03, ["2", "@"], ["២", "ៗ", "@"]),
    key(0x04, ["3", "#"], ["៣", "\"", "៑"]),
    key(0x05, ["4", "$"], ["៤", "៛", "$"]),
    key(0x06, ["5", "%"], ["៥", "%", "€"]),
    key(0x07, ["6", "^"], ["៦", "\u{17CD}", "៙"]),
    key(0x08, ["7", "&"], ["៧", "\u{17D0}", "៚"]),
    key(0x09, ["8", "*"], ["៨", "\u{17CF}", "*"]),
    key(0x0A, ["9", "("], ["៩", "(", "{"]),
    key(0x0B, ["0", ")"], ["០", ")", "}"]),
    key(0x0C, ["-", "_"], ["ឥ", "\u{17CC}", "x"]),
    key(0x0D, ["=", "+"], ["ឲ", "=", "\u{17CE}"]),

    key(0x10, ["q", "Q"], ["ឆ", "ឈ", "ៜ"]),
    key(0x11, ["w", "W"], ["\u{17B9}", "\u{17BA}", "\u{17DD}"]),
    key(0x12, ["e", "E"], ["\u{17C1}", "\u{17C2}", "ឯ"]),
    key(0x13, ["r", "R"], ["រ", "ឬ", "ឫ"]),
    key(0x14, ["t", "T"], ["ត", "ទ", "ឨ"]),
    key(0x15, ["y", "Y"], ["យ", "\u{17BD}", ""]),
    key(0x16, ["u", "U"], ["\u{17BB}", "\u{17BC}", ""]),
    key(0x17, ["i", "I"], ["\u{17B7}", "\u{17B8}", "ឦ"]),
    key(0x18, ["o", "O"], ["\u{17C4}", "\u{17C5}", "ឱ"]),
    key(0x19, ["p", "P"], ["ផ", "ភ", "ឰ"]),
    key(0x1A, ["[", "{"], ["\u{17C0}", "\u{17BF}", "ឩ"]),
    key(0x1B, ["]", "}"], ["ឪ", "ឧ", "ឳ"]),
    key(0x2B, ["\\", "|"], ["ឮ", "ឭ", "\\"]),

    key(0x1E, ["a", "A"], ["\u{17B6}", "\u{17B6}\u{17C6}", ""]),
    key(0x1F, ["s", "S"], ["ស", "\u{17C3}", ""]),
    key(0x20, ["d", "D"], ["ដ", "ឌ", ""]),
    key(0x21, ["f", "F"], ["ថ", "ធ", ""]),
    key(0x22, ["g", "G"], ["ង", "អ", ""]),
    key(0x23, ["h", "H"], ["ហ", "\u{17C7}", ""]),
    key(0x24, ["j", "J"], ["\u{17D2}", "ញ", ""]),
    key(0x25, ["k", "K"], ["ក", "គ", "ឝ"]),
    key(0x26, ["l", "L"], ["ល", "ឡ", "៘"]),
    key(0x27, [";", ":"], ["\u{17BE}", "\u{17C4}\u{17C7}", "៖"]),
    key(0x28, ["'", "\""], ["\u{17CB}", "\u{17C9}", "\u{17C8}"]),

    key(0x2C, ["z", "Z"], ["ឋ", "ឍ", ""]),
    key(0x2D, ["x", "X"], ["ខ", "ឃ", ""]),
    key(0x2E, ["c", "C"], ["ច", "ជ", ""]),
    key(0x2F, ["v", "V"], ["វ", "\u{17C1}\u{17C7}", ""]),
    key(0x30, ["b", "B"], ["ប", "ព", "ឞ"]),
    key(0x31, ["n", "N"], ["ន", "ណ", ""]),
    key(0x32, ["m", "M"], ["ម", "\u{17C6}", ""]),
    key(0x33, [",", "<"], ["\u{17BB}\u{17C6}", "\u{17BB}\u{17C7}", ","]),
    key(0x34, [".", ">"], ["។", "៕", "."]),
    key(0x35, ["/", "?"], ["\u{17CA}", "?", "/"]),
];

const NUMPAD: [&str; 16] = [
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "*", "+", "", "-", ".", "/",
];

fn main_key(scan: u8) -> Option<&'static KeyRow> {
    KEYS.iter().find(|row| row.scan == scan)
}

fn is_letter(text: &str) -> bool {
    text.len() == 1 && text.as_bytes()[0].is_ascii_alphabetic()
}

pub fn action(mode: Mode, composing: bool, vk: u16, scan: u8, held: Modifiers) -> KeyAction {
    if matches!(
        vk,
        VK_SHIFT | VK_CONTROL | VK_MENU | VK_CAPITAL | VK_LWIN | VK_RWIN | VK_NUMLOCK | VK_SCROLL
    ) || (VK_LSHIFT..=VK_RMENU).contains(&vk)
    {
        return KeyAction::Pass { forget: false };
    }
    let altgr = held.ctrl && held.alt;
    if (held.ctrl || held.alt) && !(altgr && mode == Mode::Khmer) {
        return KeyAction::Pass { forget: true };
    }
    let numpad = (VK_NUMPAD0..=VK_DIVIDE).contains(&vk);
    match mode {
        Mode::Romanized => romanized(composing, vk, scan, numpad, held),
        Mode::Khmer => khmer(composing, vk, scan, numpad, held, altgr),
    }
}

fn romanized(composing: bool, vk: u16, scan: u8, numpad: bool, held: Modifiers) -> KeyAction {
    let pass = KeyAction::Pass { forget: true };
    match vk {
        VK_SPACE => {
            return if composing {
                KeyAction::Space
            } else {
                KeyAction::Text(" ")
            }
        }
        VK_BACK => return if composing { KeyAction::Backspace } else { pass },
        VK_RETURN => return if composing { KeyAction::Commit } else { pass },
        VK_ESCAPE => {
            return if composing {
                KeyAction::Cancel
            } else {
                KeyAction::Pass { forget: false }
            }
        }
        VK_TAB if composing => {
            return if held.shift {
                KeyAction::Previous
            } else {
                KeyAction::Next
            }
        }
        VK_TAB => return pass,
        VK_UP if composing => return KeyAction::Previous,
        VK_DOWN if composing => return KeyAction::Next,
        VK_PRIOR..=VK_DOWN | VK_INSERT | VK_DELETE => {
            return if composing { KeyAction::Ignore } else { pass };
        }
        _ => {}
    }
    let text = if numpad {
        NUMPAD[usize::from(vk - VK_NUMPAD0)]
    } else {
        match main_key(scan) {
            Some(row) if is_letter(row.latin[0]) => row.latin[usize::from(held.shift != held.caps)],
            Some(row) => row.latin[usize::from(held.shift)],
            None => "",
        }
    };
    if text.is_empty() {
        KeyAction::Pass { forget: false }
    } else {
        KeyAction::Text(text)
    }
}

fn khmer(composing: bool, vk: u16, scan: u8, numpad: bool, held: Modifiers, altgr: bool) -> KeyAction {
    match vk {
        VK_SPACE => return KeyAction::Text(if held.shift { " " } else { "\u{200B}" }),
        VK_BACK if composing => return KeyAction::Backspace,
        _ => {}
    }
    if !numpad {
        if let Some(row) = main_key(scan) {
            let text = row.khmer[if altgr { 2 } else { usize::from(held.shift) }];
            if !text.is_empty() {
                return KeyAction::Text(text);
            }
        }
    }
    KeyAction::Pass { forget: true }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Modifiers = Modifiers {
        shift: false,
        ctrl: false,
        alt: false,
        caps: false,
    };
    const SHIFT: Modifiers = Modifiers { shift: true, ..NONE };
    const SCAN_A: u8 = 0x1E;
    const SCAN_1: u8 = 0x02;
    const SCAN_COMMA: u8 = 0x33;

    #[test]
    fn every_scan_code_is_listed_once() {
        for (index, row) in KEYS.iter().enumerate() {
            assert!(
                KEYS[..index].iter().all(|other| other.scan != row.scan),
                "scan code {:#04x} is listed twice",
                row.scan
            );
        }
        assert_eq!(KEYS.len(), 47, "a US keyboard has 47 character keys");
    }

    #[test]
    fn khmer_keys_type_khmer() {
        for row in KEYS {
            for text in &row.khmer[..2] {
                assert!(!text.is_empty(), "scan code {:#04x} types nothing", row.scan);
            }
        }
    }

    #[test]
    fn letters_start_a_romanized_phrase() {
        assert_eq!(action(Mode::Romanized, false, 0x41, SCAN_A, NONE), KeyAction::Text("a"));
        assert_eq!(
            action(Mode::Romanized, false, 0x41, SCAN_A, SHIFT),
            KeyAction::Text("A")
        );
        let caps = Modifiers { caps: true, ..NONE };
        assert_eq!(action(Mode::Romanized, false, 0x41, SCAN_A, caps), KeyAction::Text("A"));
        let both = Modifiers { caps: true, ..SHIFT };
        assert_eq!(action(Mode::Romanized, false, 0x41, SCAN_A, both), KeyAction::Text("a"));
    }

    #[test]
    fn digits_and_punctuation_are_latin_with_or_without_a_phrase() {
        for composing in [false, true] {
            assert_eq!(
                action(Mode::Romanized, composing, 0x31, SCAN_1, NONE),
                KeyAction::Text("1")
            );
            assert_eq!(
                action(Mode::Romanized, composing, 0x31, SCAN_1, SHIFT),
                KeyAction::Text("!")
            );
            assert_eq!(
                action(Mode::Romanized, composing, 0xBC, SCAN_COMMA, NONE),
                KeyAction::Text(",")
            );
            let caps = Modifiers { caps: true, ..NONE };
            assert_eq!(
                action(Mode::Romanized, composing, 0x31, SCAN_1, caps),
                KeyAction::Text("1")
            );
        }
    }

    #[test]
    fn romanized_space_is_a_real_space_when_no_phrase_is_being_typed() {
        assert_eq!(
            action(Mode::Romanized, false, VK_SPACE, 0x39, NONE),
            KeyAction::Text(" ")
        );
        assert_eq!(action(Mode::Romanized, true, VK_SPACE, 0x39, NONE), KeyAction::Space);
    }

    #[test]
    fn editing_keys_work_on_the_phrase_only_while_there_is_one() {
        for (vk, during) in [
            (VK_BACK, KeyAction::Backspace),
            (VK_RETURN, KeyAction::Commit),
            (VK_DOWN, KeyAction::Next),
            (VK_UP, KeyAction::Previous),
            (VK_TAB, KeyAction::Next),
            (VK_DELETE, KeyAction::Ignore),
        ] {
            assert_eq!(action(Mode::Romanized, true, vk, 0, NONE), during);
            assert_eq!(
                action(Mode::Romanized, false, vk, 0, NONE),
                KeyAction::Pass { forget: true }
            );
        }
        assert_eq!(action(Mode::Romanized, true, VK_ESCAPE, 0, NONE), KeyAction::Cancel);
        assert_eq!(action(Mode::Romanized, true, VK_TAB, 0, SHIFT), KeyAction::Previous);
    }

    #[test]
    fn shortcuts_belong_to_the_app() {
        let ctrl = Modifiers { ctrl: true, ..NONE };
        let alt = Modifiers { alt: true, ..NONE };
        for mode in [Mode::Romanized, Mode::Khmer] {
            assert_eq!(action(mode, true, 0x41, SCAN_A, ctrl), KeyAction::Pass { forget: true });
            assert_eq!(action(mode, false, 0x41, SCAN_A, alt), KeyAction::Pass { forget: true });
        }
        assert_eq!(
            action(Mode::Romanized, true, VK_SHIFT, 0x2A, SHIFT),
            KeyAction::Pass { forget: false }
        );
    }

    #[test]
    fn khmer_layout_follows_the_key_position() {
        assert_eq!(
            action(Mode::Khmer, false, 0x41, SCAN_A, NONE),
            KeyAction::Text("\u{17B6}")
        );
        assert_eq!(
            action(Mode::Khmer, false, 0x41, SCAN_A, SHIFT),
            KeyAction::Text("\u{17B6}\u{17C6}")
        );
        assert_eq!(action(Mode::Khmer, false, 0x31, SCAN_1, NONE), KeyAction::Text("១"));
        assert_eq!(
            action(Mode::Khmer, false, 0xBC, SCAN_COMMA, NONE),
            KeyAction::Text("\u{17BB}\u{17C6}")
        );
        let altgr = Modifiers {
            ctrl: true,
            alt: true,
            ..NONE
        };
        assert_eq!(action(Mode::Khmer, false, 0x45, 0x12, altgr), KeyAction::Text("ឯ"));
        assert_eq!(
            action(Mode::Khmer, false, 0x41, SCAN_A, altgr),
            KeyAction::Pass { forget: true }
        );
    }

    #[test]
    fn khmer_space_is_the_invisible_word_break() {
        assert_eq!(
            action(Mode::Khmer, false, VK_SPACE, 0x39, NONE),
            KeyAction::Text("\u{200B}")
        );
        assert_eq!(action(Mode::Khmer, false, VK_SPACE, 0x39, SHIFT), KeyAction::Text(" "));
    }

    #[test]
    fn khmer_backspace_and_enter() {
        assert_eq!(action(Mode::Khmer, true, VK_BACK, 0x0E, NONE), KeyAction::Backspace);
        assert_eq!(
            action(Mode::Khmer, false, VK_BACK, 0x0E, NONE),
            KeyAction::Pass { forget: true }
        );
        assert_eq!(
            action(Mode::Khmer, true, VK_RETURN, 0x1C, NONE),
            KeyAction::Pass { forget: true }
        );
    }

    #[test]
    fn the_number_pad_types_digits() {
        assert_eq!(action(Mode::Romanized, false, 0x65, 0x4C, NONE), KeyAction::Text("5"));
        assert_eq!(action(Mode::Romanized, true, 0x65, 0x4C, NONE), KeyAction::Text("5"));
        assert_eq!(
            action(Mode::Romanized, true, VK_DIVIDE, 0x35, NONE),
            KeyAction::Text("/")
        );
        assert_eq!(
            action(Mode::Khmer, false, VK_DIVIDE, 0x35, NONE),
            KeyAction::Pass { forget: true }
        );
    }
}
