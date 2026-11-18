import Foundation

/// A key press, as the keyboard sees it.
enum Key: Equatable {
    case space
    case backspace
    case enter
    case escape
    /// Down arrow or Tab: highlight the next suggestion.
    case next
    /// Up arrow or Shift+Tab.
    case previous
    /// Left, Right, Home, End, Page Up/Down, Forward Delete.
    case motion
    /// A letter, digit or punctuation mark.
    case text(String)
    /// With Command, Control or Option held: always the app's.
    case shortcut
    /// Anything else: function keys, characters from other layouts.
    case other

    /// Mac virtual key codes (Carbon's kVK_ constants).
    static func from(keyCode: UInt16, characters: String?, shift: Bool, command: Bool, control: Bool, option: Bool) -> Key {
        if command || control || option {
            return .shortcut
        }
        switch keyCode {
        case 49: return .space
        case 51: return .backspace
        case 36, 76: return .enter
        case 53: return .escape
        case 48: return shift ? .previous : .next
        case 125: return .next
        case 126: return .previous
        case 123, 124, 115, 119, 116, 121, 117: return .motion
        default: break
        }
        // Only plain ASCII goes to the keyboard. Function and arrow keys come
        // as private-use characters (U+F700 and up) and are left to the app.
        guard let characters = characters, !characters.isEmpty,
              characters.unicodeScalars.allSatisfy({ (0x21...0x7E).contains($0.value) })
        else {
            return .other
        }
        return .text(characters)
    }
}

/// Where the text goes: the app's text field.
protocol TextTarget: AnyObject {
    /// Inserts text in place of the marked text.
    func insert(_ text: String)
    /// Shows text marked (underlined) at the caret; "" takes it away.
    func mark(_ text: String)
}

/// What the keyboard does with each key, apart from the Mac's text system
/// (InputController) and the popup (CandidatePanel), so it can be tested.
final class Session {
    let engine = Engine()
    private(set) var highlight = 0
    private var marked = ""

    /// Called after every change: the popup redraws.
    var onChange: () -> Void = {}
    /// Called with the words corrected by picking another suggestion.
    var onCorrections: ([(String, String)]) -> Void = { _ in }

    var isComposing: Bool { engine.isComposing }
    var candidates: [String] { engine.isComposing ? engine.candidates : [] }
    var typed: String { engine.typed }

    /// Returns true when the key was the keyboard's, false to give it to the app.
    @discardableResult
    func handle(_ key: Key, in target: TextTarget) -> Bool {
        let composing = engine.isComposing
        switch key {
        case .shortcut, .other:
            // The app gets the key. A phrase being typed is sent first, so a
            // shortcut such as Command+Enter acts on all of it.
            if composing { finish(in: target) }
            engine.reset()
            return false
        case .motion:
            if composing { return true }
            engine.reset()
            return false
        case .space:
            if composing && highlight > 0 {
                apply(engine.select(highlight), to: target)
            } else {
                apply(engine.key(UInt32(32)), to: target)
            }
        case .backspace:
            guard composing else {
                engine.reset()
                return false
            }
            apply(engine.key(Engine.backspace), to: target)
        case .enter:
            guard composing else {
                engine.reset()
                return false
            }
            let chosen = engine.select(highlight)
            apply(chosen.commit.isEmpty ? engine.commit() : chosen, to: target)
        case .escape:
            guard composing else { return false }
            cancel(in: target)
        case .next, .previous:
            guard composing else {
                engine.reset()
                return false
            }
            let count = max(engine.candidates.count, 1)
            highlight = (highlight + (key == .next ? 1 : count - 1)) % count
            onChange()
        case .text(let text):
            var commit = ""
            var preedit = engine.isComposing ? marked : ""
            for scalar in text.unicodeScalars {
                let output = engine.key(scalar.value)
                commit += output.commit
                preedit = output.preedit
            }
            apply(Engine.Output(commit: commit, preedit: preedit, handled: true), to: target)
        }
        return true
    }

    /// Sends the phrase as it is shown: the app or the Mac wants it finished.
    func finish(in target: TextTarget) {
        guard engine.isComposing else { return }
        apply(engine.commit(), to: target)
    }

    /// Sends suggestion `index`: a click in the popup.
    func pick(_ index: Int, in target: TextTarget) {
        guard engine.isComposing else { return }
        apply(engine.select(index), to: target)
    }

    /// Throws the phrase away.
    func cancel(in target: TextTarget) {
        engine.reset()
        apply(Engine.Output(commit: "", preedit: "", handled: true), to: target)
    }

    /// Forgets the phrase without touching the app: the app is gone.
    func forget() {
        engine.reset()
        marked = ""
        highlight = 0
        onChange()
    }

    private func apply(_ output: Engine.Output, to target: TextTarget) {
        highlight = 0
        let corrections = engine.takeCorrections()
        if !corrections.isEmpty { onCorrections(corrections) }
        if !output.commit.isEmpty {
            target.insert(output.commit)
            marked = ""
        }
        if output.preedit != marked {
            target.mark(output.preedit)
            marked = output.preedit
        }
        onChange()
    }
}
