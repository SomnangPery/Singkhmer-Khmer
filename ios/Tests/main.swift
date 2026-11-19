// Tests for the iPhone keyboard, with the real engine and no iOS: taps go
// through Typist into a text field that exists only in memory. Run on a Mac
// by `ios/build.sh --test`.
import Foundation

/// A text field that exists only in memory.
final class MemoryField: Field {
    var text = ""
    var marked = ""

    func insert(_ inserted: String) {
        text += inserted
        marked = ""
    }

    func mark(_ shown: String) {
        marked = shown
    }

    func deleteBackward() {
        if !text.isEmpty { text.removeLast() }
    }

    func type(_ typed: String) {
        text += typed
    }
}

var failures = 0

func check(_ passed: Bool, _ what: String, _ detail: @autoclosure () -> String = "") {
    if passed {
        print("ok    \(what)")
    } else {
        failures += 1
        print("FAIL  \(what) \(detail())")
    }
}

/// Taps `keys` one at a time: a space is the space bar, anything else a key.
func tap(_ keys: String, _ typist: Typist) {
    for key in keys {
        typist.tap(key == " " ? .space : .text(String(key)))
    }
}

func test(_ name: String, _ body: (Typist, MemoryField) -> Void) {
    print("-- \(name)")
    let field = MemoryField()
    body(Typist(field: field), field)
}

Engine.setUserWords("")

test("every row of keys is as wide as the keyboard") { _, _ in
    for page in [Page.letters, .symbols, .moreSymbols] {
        for globe in [false, true] {
            let rows = Layout.rows(page, shifted: false, globe: globe, spaceLabel: "Singkhmer", enterLabel: "return")
            check(rows.count == 5, "\(page), globe \(globe): five rows")
            for (number, row) in rows.enumerated() where number != 2 || page != .letters {
                let width = row.reduce(0) { $0 + $1.width }
                check(abs(width - Layout.columns) < 0.001, "\(page), globe \(globe), row \(number + 1) fills the width", "\(width)")
            }
        }
    }
    let letters = Layout.rows(.letters, shifted: false, globe: false, spaceLabel: "", enterLabel: "")
    check(letters.flatMap { $0 }.filter { $0.action == .nextKeyboard }.isEmpty, "no globe unless iOS asks for it")
    let shifted = Layout.rows(.letters, shifted: true, globe: false, spaceLabel: "", enterLabel: "")
    check(shifted[1].first?.action == .text("Q"), "Shift shows capitals", "\(shifted[1].first!)")
}

test("a phrase is shown while it is typed and sent with a second space") { typist, field in
    tap("nh jg tv sala rean ", typist)
    check(field.text.isEmpty, "nothing is sent yet", field.text)
    check(!field.marked.isEmpty, "the phrase is shown marked", field.marked)
    check(typist.suggestions.first == "ខ្ញុំចង់ទៅសាលារៀន", "the top suggestion", "\(typist.suggestions)")
    tap(" ", typist)
    check(field.text == "ខ្ញុំចង់ទៅសាលារៀន ", "a second space sends it with a space", field.text)
    check(field.marked.isEmpty && !typist.isComposing, "and ends the phrase")
    check(typist.suggestions.isEmpty, "no suggestions are left")
}

test("a tap on a suggestion sends that one") { typist, field in
    tap("nh jg tv", typist)
    let suggestions = typist.suggestions
    check(suggestions.count > 1, "there is a second suggestion", "\(suggestions)")
    typist.pick(1)
    check(field.text == suggestions[1], "the second one is sent", field.text)
    check(field.marked.isEmpty, "nothing is left marked", field.marked)
}

test("Return sends the phrase and starts a new line") { typist, field in
    tap("nh", typist)
    typist.tap(.enter)
    check(field.text == "ខ្ញុំ\n", "the phrase, then a new line", field.text)
    typist.tap(.enter)
    check(field.text == "ខ្ញុំ\n\n", "with no phrase, only a new line", field.text)
}

test("Backspace takes back a letter, then deletes from the app") { typist, field in
    field.text = "ok"
    tap("nhx", typist)
    typist.tap(.backspace)
    check(field.marked == "nh", "a letter is taken back", field.marked)
    typist.tap(.backspace)
    typist.tap(.backspace)
    check(!typist.isComposing && field.marked.isEmpty, "the phrase is gone", field.marked)
    typist.tap(.backspace)
    check(field.text == "o", "then Backspace deletes in the app", field.text)
}

test("Shift capitalizes one letter, and twice locks it") { typist, field in
    let now = Date()
    typist.tap(.shift, at: now)
    check(typist.shift == .once, "one tap: one capital")
    typist.tap(.text("A"))
    check(typist.shift == .off, "and it goes off after a letter")
    typist.tap(.shift, at: now.addingTimeInterval(10))
    typist.tap(.shift, at: now.addingTimeInterval(10.1))
    check(typist.shift == .locked, "two quick taps lock it")
    typist.tap(.text("B"))
    check(typist.shift == .locked, "and it stays on")
    typist.tap(.shift, at: now.addingTimeInterval(20))
    check(typist.shift == .off, "a third tap turns it off")
}

test("digits and punctuation end the phrase") { typist, field in
    tap("nh", typist)
    typist.tap(.text("."))
    check(field.text.hasPrefix("ខ្ញុំ"), "the phrase is sent before the full stop", field.text)
    check(!typist.isComposing, "and the phrase ends")
}

test("pages of keys") { typist, _ in
    typist.tap(.page(.symbols))
    check(typist.page == .symbols, "?123 shows the digits and symbols")
    typist.tap(.page(.moreSymbols))
    check(typist.page == .moreSymbols, "ឥឧ shows the Khmer digits and letters")
    typist.reset(numbers: false)
    check(typist.page == .letters, "a new text field starts on the letters")
    typist.reset(numbers: true)
    check(typist.page == .symbols, "a number field starts on the digits")
}

test("email and web address fields get the letters as typed") { typist, field in
    typist.plain = true
    tap("nh jg", typist)
    check(field.text == "nh jg", "straight in, in Latin letters", field.text)
    check(typist.suggestions.isEmpty && field.marked.isEmpty, "with no phrase and no suggestions")
    typist.tap(.backspace)
    check(field.text == "nh j", "Backspace deletes", field.text)
}

test("the phrase is sent when the keyboard goes away") { typist, field in
    tap("nh jg", typist)
    typist.finish()
    check(field.text == "ខ្ញុំចង់", "the phrase as shown", field.text)
}

test("the phrase is forgotten when the app changes the text") { typist, field in
    tap("nh", typist)
    typist.forget()
    check(!typist.isComposing && typist.suggestions.isEmpty, "no phrase and no suggestions")
    tap("jg", typist)
    check(field.marked == "jg", "the next letters start a new phrase", field.marked)
}

print(failures == 0 ? "\nAll iPhone keyboard tests passed." : "\n\(failures) failed.")
exit(failures == 0 ? 0 : 1)
