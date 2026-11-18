import AppKit
import SwiftUI

/// "Wrong word?": what the user typed and what the keyboard gave for it.
struct Report {
    var typed: String
    var got: String
}

/// The My words window, the same as on Windows (windows/my-words.ps1):
///
/// * My words: the user's own Singkhmer spellings for Khmer words.
/// * Share: words to send to Bunny Keyboard's word list through a Google
///   Form. Nothing is sent until the user presses Submit on the form.
///
/// Opened from the popup's buttons and from "My words…" in the input menu.
enum MyWordsWindow {
    private static var window: NSWindow?

    static func show(spelling: String = "", report: Report? = nil) {
        let model = MyWordsModel(spelling: spelling, report: report)
        let host = NSHostingController(rootView: MyWordsView(model: model))
        let window = self.window ?? {
            let made = NSWindow(contentViewController: host)
            made.title = "Bunny Keyboard - My words"
            made.styleMask = [.titled, .closable, .miniaturizable, .resizable]
            made.isReleasedWhenClosed = false
            made.setContentSize(NSSize(width: 540, height: 600))
            made.center()
            self.window = made
            return made
        }()
        window.contentViewController = host
        NSApp.activate(ignoringOtherApps: true)
        window.makeKeyAndOrderFront(nil)
    }
}

struct ShareRow: Identifiable {
    var pair: Pair
    var from: String
    var checked = true
    var id: String { pair.line }
}

final class MyWordsModel: ObservableObject {
    @Published var words: [Pair]
    @Published var spelling: String { didSet { if spelling != oldValue { message = ""; guess() } } }
    @Published var khmer: String { didSet { if khmer != oldValue { message = "" } } }
    @Published var shareToo: Bool
    @Published var message = ""
    @Published var messageIsError = false
    @Published var guesses: [String] = []
    @Published var rows: [ShareRow] = []
    @Published var shareMessage = ""
    @Published var shareMessageIsGood = false
    @Published var tab = 0

    let report: Report?

    init(spelling: String, report: Report?) {
        words = MyWords.read(MyWords.wordsFile)
        self.spelling = spelling.lowercased()
        khmer = ""
        self.report = report
        shareToo = report != nil && ShareForm.isOn
        guess()
        showShareable()
    }

    var about: String {
        if let report = report {
            return "You typed:  \(report.typed)\nThe keyboard gave:  \(report.got)\n\n"
                + "Write the spelling and the Khmer word you wanted. It will come first from now on."
        }
        return "When you type the spelling, your word comes first in the suggestions."
    }

    /// The keyboard's own readings of the spelling, to pick from.
    private func guess() {
        let letters = spelling.trimmingCharacters(in: .whitespaces)
        guesses = letters.isEmpty ? [] : Array(Engine.convert(letters).prefix(3))
    }

    private func say(_ text: String, error: Bool) {
        message = text
        messageIsError = error
    }

    func add() {
        let spelling = self.spelling.trimmingCharacters(in: .whitespaces).lowercased()
        let khmer = self.khmer.trimmingCharacters(in: .whitespaces)
        if let problem = Engine.check(spelling: spelling, word: khmer) {
            say("Not added: \(problem).", error: true)
            return
        }
        let pair = Pair(spelling: spelling, khmer: khmer)
        let known = words.contains(pair)
        if !known {
            // The newest word for a spelling goes first: it is the one wanted now.
            words.insert(pair, at: 0)
            do {
                try MyWords.save(words)
                MyWords.changed()
            } catch {
                words.removeFirst()
                say("Could not save: \(error.localizedDescription)", error: true)
                return
            }
        }
        var done = known ? "'\(pair.line)' is already in your words." : "Added. Type '\(spelling)' to get \(khmer)."
        if shareToo {
            var line = pair.line
            if let report = report, !report.got.isEmpty {
                line += " (keyboard gave: \(report.got); typed: \(report.typed))"
            }
            send([line], pairs: [pair])
            done += " The form is open in your browser: press Submit there to send it."
        }
        say(done, error: false)
        self.spelling = ""
        self.khmer = ""
        shareToo = false
        showShareable()
    }

    func remove(_ chosen: Set<Pair>) {
        guard !chosen.isEmpty else { return }
        words.removeAll { chosen.contains($0) }
        do {
            try MyWords.save(words)
            MyWords.changed()
            say("Removed.", error: false)
        } catch {
            say("Could not save: \(error.localizedDescription)", error: true)
        }
        showShareable()
    }

    func showShareable() {
        let sent = Set(MyWords.read(MyWords.sharedFile).map(\.line))
        var order: [String] = []
        var found: [String: (pair: Pair, mine: Bool, times: Int)] = [:]
        for pair in words where found[pair.line] == nil {
            order.append(pair.line)
            found[pair.line] = (pair, true, 0)
        }
        for pair in MyWords.read(MyWords.learnedFile) {
            if found[pair.line] == nil {
                order.append(pair.line)
                found[pair.line] = (pair, false, 0)
            }
            found[pair.line]?.times += 1
        }
        rows = order.filter { !sent.contains($0) }.compactMap { line -> ShareRow? in
            guard let row = found[line] else { return nil }
            let from = row.mine ? "your word" : (row.times == 1 ? "picked" : "picked \(row.times)x")
            return ShareRow(pair: row.pair, from: from)
        }
        shareMessageIsGood = false
        if !ShareForm.isOn {
            shareMessage = "Sending is not switched on in this version yet."
        } else if rows.isEmpty {
            shareMessage = "Nothing new to send. Words you add or correct will show up here."
        } else {
            shareMessage = "\(rows.count) word(s) not sent yet."
        }
    }

    func sendChosen() {
        let chosen = rows.filter(\.checked).prefix(ShareForm.mostPerSend).map(\.pair)
        guard !chosen.isEmpty else {
            shareMessage = "Tick at least one word to send."
            shareMessageIsGood = false
            return
        }
        send(chosen.map(\.line), pairs: Array(chosen))
        showShareable()
        shareMessage = "The form is open in your browser with \(chosen.count) word(s): press Submit there to send them. Thank you!"
        shareMessageIsGood = true
    }

    private func send(_ lines: [String], pairs: [Pair]) {
        guard let link = ShareForm.link(lines) else { return }
        NSWorkspace.shared.open(link)
        MyWords.append(pairs, to: MyWords.sharedFile)
    }

    func openFile() {
        if !FileManager.default.fileExists(atPath: MyWords.wordsFile.path) {
            try? MyWords.save(words)
        }
        NSWorkspace.shared.open(MyWords.wordsFile)
    }
}

private let khmerFont = Font.custom("Khmer Sangam MN", size: 16)

struct MyWordsView: View {
    @ObservedObject var model: MyWordsModel

    var body: some View {
        TabView(selection: $model.tab) {
            MyWordsTab(model: model).tabItem { Text("My words") }.tag(0)
            ShareTab(model: model).tabItem { Text("Share") }.tag(1)
        }
        .padding(12)
        .frame(minWidth: 480, minHeight: 520)
    }
}

struct MyWordsTab: View {
    @ObservedObject var model: MyWordsModel
    @State private var selection = Set<Pair>()

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(model.about)
                .foregroundColor(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            HStack(alignment: .top, spacing: 10) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Singkhmer spelling")
                    TextField("bong", text: $model.spelling)
                }
                VStack(alignment: .leading, spacing: 4) {
                    Text("Khmer word")
                    TextField("បង", text: $model.khmer).font(khmerFont)
                }
            }
            if !model.guesses.isEmpty {
                HStack(spacing: 6) {
                    Text("The keyboard's guesses:").foregroundColor(.secondary)
                    ForEach(model.guesses, id: \.self) { guess in
                        Button(guess) { model.khmer = guess }.font(khmerFont)
                    }
                }
            }
            Text("Pick a guess, paste the word, or type it with the Mac's Khmer keyboard.")
                .font(.caption)
                .foregroundColor(.secondary)
            Toggle("Also send it to Bunny Keyboard, to fix it for everyone", isOn: $model.shareToo)
                .disabled(!ShareForm.isOn)
            if !model.message.isEmpty {
                Text(model.message)
                    .foregroundColor(model.messageIsError ? .red : .green)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Button("Add word") { model.add() }
                .keyboardShortcut(.defaultAction)
            Divider()
            List(selection: $selection) {
                ForEach(model.words, id: \.self) { word in
                    HStack {
                        Text(word.spelling).frame(width: 170, alignment: .leading)
                        Text(word.khmer).font(khmerFont)
                    }
                }
            }
            .frame(minHeight: 140)
            HStack {
                Button("Remove selected") {
                    model.remove(selection)
                    selection.removeAll()
                }
                .disabled(selection.isEmpty)
                Spacer()
                Button("Open the file") { model.openFile() }
            }
        }
        .padding(12)
    }
}

struct ShareTab: View {
    @ObservedObject var model: MyWordsModel

    static let explanation: String = "These are your own words, and words you corrected by picking another "
        + "suggestion. Only the words are kept, never what you wrote. Tick the ones to send, then press "
        + "Send: a form opens in your browser with them filled in, and nothing is sent until you press "
        + "Submit there. Sent words are given to the public domain (CC0), so anyone may use them."

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(ShareTab.explanation)
                .foregroundColor(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            List {
                ForEach($model.rows) { $row in
                    Toggle(isOn: $row.checked) {
                        HStack {
                            Text(row.pair.spelling).frame(width: 140, alignment: .leading)
                            Text(row.pair.khmer).font(khmerFont)
                            Spacer()
                            Text(row.from).foregroundColor(.secondary)
                        }
                    }
                }
            }
            .frame(minHeight: 200)
            Text(model.shareMessage)
                .foregroundColor(model.shareMessageIsGood ? .green : .secondary)
                .fixedSize(horizontal: false, vertical: true)
            Button("Send…") { model.sendChosen() }
                .disabled(!ShareForm.isOn || model.rows.isEmpty)
        }
        .padding(12)
    }
}
