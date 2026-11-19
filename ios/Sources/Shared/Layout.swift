import Foundation

/// What a key on the screen does.
enum KeyAction: Equatable {
    /// Types its text.
    case text(String)
    case space
    case backspace
    case enter
    case shift
    /// Shows another page of keys.
    case page(Page)
    /// The globe: the next keyboard on the phone. Only shown when iOS asks
    /// for it (on phones with a Home button, or with no other way to switch).
    case nextKeyboard
}

/// The pages of keys: letters, and two pages of digits and symbols.
enum Page: Equatable {
    case letters
    case symbols
    case moreSymbols
}

struct KeyCap: Equatable {
    var label: String
    var action: KeyAction
    /// Width in key widths: a letter key is 1.
    var width: Double = 1
    /// Drawn darker, like the Shift and Backspace keys on iOS's own keyboards.
    var special: Bool = false

    static func letter(_ text: String) -> KeyCap {
        KeyCap(label: text, action: .text(text))
    }
}

/// The keys, row by row, the same as the Android keyboard's Singkhmer
/// layout (android/.../Layouts.kt). Edit this to move a key.
enum Layout {
    /// The keys of `page`. The shift key shows capitals when `shifted`, and
    /// the globe key is left out unless `globe`.
    static func rows(_ page: Page, shifted: Bool, globe: Bool, spaceLabel: String, enterLabel: String) -> [[KeyCap]] {
        let bottom = bottomRow(page, globe: globe, spaceLabel: spaceLabel, enterLabel: enterLabel)
        switch page {
        case .letters:
            let cased: (String) -> [KeyCap] = { row in
                letters(shifted ? row.uppercased() : row)
            }
            return [
                letters("1 2 3 4 5 6 7 8 9 0"),
                cased("q w e r t y u i o p"),
                cased("a s d f g h j k l"),
                [KeyCap(label: "⇧", action: .shift, width: 1.5, special: true)]
                    + cased("z x c v b n m")
                    + [backspace],
                bottom,
            ]
        case .symbols:
            return [
                letters("1 2 3 4 5 6 7 8 9 0"),
                letters("@ # $ ៛ % & - + ( )"),
                letters("* \" ' : ; ! ? / = _"),
                [KeyCap(label: "ឥឧ", action: .page(.moreSymbols), width: 1.5, special: true)]
                    + letters("« » ៖ ៗ ៕ \\ ~")
                    + [backspace],
                bottom,
            ]
        case .moreSymbols:
            return [
                letters("១ ២ ៣ ៤ ៥ ៦ ៧ ៨ ៩ ០"),
                letters("ឥ ឦ ឧ ឩ ឪ ឫ ឬ ឭ ឮ ឯ"),
                letters("ឰ ឱ ឲ ឳ ៌ ៍ ៎ ៏ ័ ៈ"),
                [KeyCap(label: "?123", action: .page(.symbols), width: 1.5, special: true)]
                    + letters("[ ] { } < > |")
                    + [backspace],
                bottom,
            ]
        }
    }

    private static let backspace = KeyCap(label: "⌫", action: .backspace, width: 1.5, special: true)

    /// Split at the spaces by code point, not by letter: a Khmer sign such as
    /// ៍ would otherwise join the space before it.
    private static func letters(_ row: String) -> [KeyCap] {
        row.unicodeScalars.split(separator: " ").map { KeyCap.letter(String(String.UnicodeScalarView($0))) }
    }

    private static func bottomRow(_ page: Page, globe: Bool, spaceLabel: String, enterLabel: String) -> [KeyCap] {
        var row = [
            page == .letters
                ? KeyCap(label: "?123", action: .page(.symbols), width: 1.5, special: true)
                : KeyCap(label: "ABC", action: .page(.letters), width: 1.5, special: true),
        ]
        if globe {
            row.append(KeyCap(label: "🌐", action: .nextKeyboard, width: 1.2, special: true))
        }
        row += [
            KeyCap.letter(","),
            KeyCap(label: spaceLabel, action: .space, width: globe ? 3.8 : 5.0),
            KeyCap.letter("."),
            KeyCap(label: enterLabel, action: .enter, width: 1.5, special: true),
        ]
        return row
    }

    /// How many key widths fit across: every row is laid out against this.
    static let columns = 10.0
}
