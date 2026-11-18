// Tests for the Mac keyboard, with the real engine and no Mac text system:
// keys go through Session into a stand-in text field. Run by
// `macos/build.sh --test`.
import Foundation

/// A text field that exists only in memory.
final class Field: TextTarget {
    var text = ""
    var marked = ""

    func insert(_ inserted: String) {
        text += inserted
        marked = ""
    }

    func mark(_ shown: String) {
        marked = shown
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

/// Types `keys` one at a time: a space is the space bar, anything else a key.
@discardableResult
func typeKeys(_ keys: String, _ session: Session, _ field: Field) -> [Bool] {
    keys.map { session.handle($0 == " " ? .space : .text(String($0)), in: field) }
}

func test(_ name: String, _ body: (Session, Field) -> Void) {
    print("-- \(name)")
    body(Session(), Field())
}

Engine.setUserWords("")

test("a phrase is shown while it is typed and sent with a second space") { session, field in
    typeKeys("nh jg tv sala rean ", session, field)
    check(field.text.isEmpty, "nothing is sent yet", field.text)
    check(!field.marked.isEmpty, "the phrase is shown marked", field.marked)
    check(session.candidates.first == "ខ្ញុំចង់ទៅសាលារៀន", "the top suggestion", "\(session.candidates)")
    check(session.typed == "nh jg tv sala rean", "the letters typed", session.typed)
    typeKeys(" ", session, field)
    check(field.text == "ខ្ញុំចង់ទៅសាលារៀន ", "a second space sends it with a space", field.text)
    check(field.marked.isEmpty && !session.isComposing, "and ends the phrase")
}

test("Enter sends the highlighted suggestion, and the correction is noted") { session, field in
    var noted: [(String, String)] = []
    session.onCorrections = { noted += $0 }
    typeKeys("nh jg tv", session, field)
    let suggestions = session.candidates
    check(suggestions.count > 1, "there is a second suggestion", "\(suggestions)")
    check(session.handle(.next, in: field), "Down arrow is the keyboard's while typing")
    check(session.highlight == 1, "it highlights the second suggestion")
    check(session.handle(.previous, in: field) && session.highlight == 0, "Up arrow goes back")
    session.handle(.next, in: field)
    session.handle(.enter, in: field)
    check(field.text == suggestions[1], "Enter sends it", field.text)
    check(!noted.isEmpty, "the corrected words are noted", "\(noted)")
}

test("Backspace undoes one key, and is the app's with no phrase") { session, field in
    typeKeys("nhx", session, field)
    check(session.handle(.backspace, in: field), "Backspace is the keyboard's while typing")
    check(field.marked == "nh", "it takes off the x", field.marked)
    session.handle(.backspace, in: field)
    session.handle(.backspace, in: field)
    check(!session.isComposing && field.marked.isEmpty, "the phrase is gone")
    check(!session.handle(.backspace, in: field), "the next Backspace goes to the app")
}

test("Esc throws the phrase away") { session, field in
    typeKeys("nh jg", session, field)
    check(session.handle(.escape, in: field), "Esc is the keyboard's")
    check(field.text.isEmpty && field.marked.isEmpty, "nothing is left")
    check(!session.handle(.escape, in: field), "the next Esc goes to the app")
}

test("punctuation sends the phrase, then itself") { session, field in
    typeKeys("soksabay te?", session, field)
    check(field.text == "សុខសប្បាយទេ?", "the phrase and the question mark", field.text)
}

test("a shortcut sends the phrase and goes to the app") { session, field in
    typeKeys("nh", session, field)
    check(!session.handle(.shortcut, in: field), "Command+key is the app's")
    check(field.text == "ខ្ញុំ" && field.marked.isEmpty, "the phrase was sent first", field.text)
}

test("arrow keys and Enter with no phrase are the app's") { session, field in
    check(!session.handle(.motion, in: field), "Left arrow")
    check(!session.handle(.next, in: field), "Down arrow")
    check(!session.handle(.enter, in: field), "Enter")
    typeKeys("nh", session, field)
    check(session.handle(.motion, in: field) && field.marked == "nh", "Left arrow is ignored while typing")
}

test("a click on a suggestion sends it") { session, field in
    typeKeys("nh jg tv", session, field)
    let suggestions = session.candidates
    let last = suggestions.count - 1
    session.pick(last, in: field)
    check(last > 0 && field.text == suggestions[last], "the last suggestion", field.text)
}

print("-- Mac key codes")
check(Key.from(keyCode: 49, characters: " ", shift: false, command: false, control: false, option: false) == .space, "space bar")
check(Key.from(keyCode: 0, characters: "a", shift: false, command: false, control: false, option: false) == .text("a"), "a")
check(Key.from(keyCode: 0, characters: "A", shift: true, command: false, control: false, option: false) == .text("A"), "Shift+a")
check(Key.from(keyCode: 0, characters: "a", shift: false, command: true, control: false, option: false) == .shortcut, "Command+a")
check(Key.from(keyCode: 0, characters: "å", shift: false, command: false, control: false, option: true) == .shortcut, "Option+a")
check(Key.from(keyCode: 36, characters: "\r", shift: false, command: false, control: false, option: false) == .enter, "Return")
check(Key.from(keyCode: 76, characters: "\u{3}", shift: false, command: false, control: false, option: false) == .enter, "keypad Enter")
check(Key.from(keyCode: 48, characters: "\t", shift: true, command: false, control: false, option: false) == .previous, "Shift+Tab")
check(Key.from(keyCode: 125, characters: "\u{F701}", shift: false, command: false, control: false, option: false) == .next, "Down arrow")
check(Key.from(keyCode: 123, characters: "\u{F702}", shift: false, command: false, control: false, option: false) == .motion, "Left arrow")
check(Key.from(keyCode: 122, characters: "\u{F704}", shift: false, command: false, control: false, option: false) == .other, "F1")
check(Key.from(keyCode: 18, characters: "1", shift: false, command: false, control: false, option: false) == .text("1"), "1")

print("-- the share link")
let link = ShareForm.link(["bong = បង", "oun = អូន"])?.absoluteString ?? ""
check(link.hasPrefix(ShareForm.url + "?usp=pp_url&"), "goes to the form", link)
check(link.contains("\(ShareForm.wordsField)=bong%20%3D%20%E1%9E%94%E1%9E%84%0Aoun"), "the words, one per line", link)
check(link.contains("\(ShareForm.spellingsField)=bong%2C%20oun"), "the spellings", link)

print("-- checking a word")
check(Engine.check(spelling: "bong", word: "បង") == nil, "bong = បង is fine")
check(Engine.check(spelling: "bo ng", word: "បង")?.contains("a-z") == true, "a space in the spelling is not")
check(Engine.check(spelling: "bong", word: "bong")?.contains("Khmer") == true, "Latin for the word is not")
check(Engine.convert("nh jg tv").first == "ខ្ញុំចង់ទៅ", "guesses for a spelling")

// Last: this changes the words every session gets.
test("my words, in their own folder, come first at once") { session, field in
    let folder = FileManager.default.temporaryDirectory.appendingPathComponent("bunny-keyboard-test-\(getpid())")
    defer { try? FileManager.default.removeItem(at: folder) }
    MyWords.folderOverride = folder
    try? MyWords.save([Pair(spelling: "zzq", khmer: "បង")])
    check(MyWords.read(MyWords.wordsFile) == [Pair(spelling: "zzq", khmer: "បង")], "saved and read back")
    MyWords.changed()
    typeKeys("zzq", session, field)
    check(session.candidates.first == "បង", "the word comes first", "\(session.candidates)")
    MyWords.remember([("srolanh", "ស្រលាញ់")])
    MyWords.remember([("srolanh", "ស្រលាញ់")])
    check(MyWords.read(MyWords.learnedFile).count == 2, "corrections are added to learned.txt")
    try? FileManager.default.removeItem(at: MyWords.wordsFile)
    MyWords.refresh()
    let fresh = Session()
    typeKeys("zzq", fresh, Field())
    check(fresh.candidates.first != "បង", "a deleted file takes the words away", "\(fresh.candidates)")
}

print(failures == 0 ? "\nAll passed." : "\n\(failures) failed.")
exit(failures == 0 ? 0 : 1)
