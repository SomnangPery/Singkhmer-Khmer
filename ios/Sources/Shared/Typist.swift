import Foundation

/// The text field the keys go into. On the phone this is iOS's
/// `textDocumentProxy`; in the app's "Try it" box and in the tests it is a
/// field of our own.
protocol Field: TextTarget {
    /// Deletes the character before the caret: Backspace with no phrase.
    func deleteBackward()
    /// Types text straight in, with no phrase: Return, and every key in
    /// fields for email addresses, web addresses and numbers.
    func type(_ text: String)
}

/// What each tap does, apart from drawing the keys (KeyboardView) and from
/// iOS (KeyboardViewController), so the tests can tap through it.
///
/// Letters go to the engine through `Session`, as on the Mac. The phrase is
/// shown marked (underlined) in the app while it is typed, and the
/// suggestions show above the keys: a tap on one sends it, the space bar
/// sends the first one when Space is pressed twice, and Return sends the
/// phrase and starts a new line, as on Android.
final class Typist {
    let session = Session()
    let field: Field

    private(set) var page = Page.letters
    private(set) var shift = Shift.off
    /// Fields for email or web addresses, and number pads: keys go straight
    /// into the field, in Latin letters.
    var plain = false {
        didSet {
            if plain != oldValue { finish() }
        }
    }

    enum Shift {
        case off
        /// Capital for one letter.
        case once
        /// Capitals until Shift is tapped again.
        case locked
    }

    /// Called after every tap, to redraw the keys and the suggestions.
    var onChange: () -> Void = {}

    private var lastShiftTap = Date.distantPast
    static let doubleTap: TimeInterval = 0.35

    init(field: Field) {
        self.field = field
    }

    var suggestions: [String] { plain ? [] : session.candidates }
    var isComposing: Bool { session.isComposing }

    func tap(_ action: KeyAction, at time: Date = Date()) {
        switch action {
        case .text(let text):
            if plain {
                field.type(text)
            } else {
                session.handle(.text(text), in: field)
            }
            if shift == .once { shift = .off }
        case .space:
            if plain || !session.handle(.space, in: field) {
                field.type(" ")
            }
        case .backspace:
            if plain || !session.handle(.backspace, in: field) {
                field.deleteBackward()
            }
        case .enter:
            finish()
            field.type("\n")
        case .shift:
            if shift == .locked {
                shift = .off
            } else if time.timeIntervalSince(lastShiftTap) < Typist.doubleTap {
                shift = .locked
            } else {
                shift = shift == .off ? .once : .off
            }
            lastShiftTap = time
        case .page(let newPage):
            page = newPage
            shift = .off
        case .nextKeyboard:
            finish()
        }
        onChange()
    }

    /// Sends suggestion `index`: a tap on the strip above the keys.
    func pick(_ index: Int) {
        session.pick(index, in: field)
        onChange()
    }

    /// Sends the phrase as it is shown: the keyboard is going away.
    func finish() {
        session.finish(in: field)
        onChange()
    }

    /// Forgets the phrase without touching the field: the app changed the
    /// text or moved the caret by itself.
    func forget() {
        session.forget()
        onChange()
    }

    /// Back to the letters, for a new text field. `numbers` starts on the
    /// digits instead.
    func reset(numbers: Bool) {
        session.forget()
        page = numbers ? .symbols : .letters
        shift = .off
        onChange()
    }
}
