import Foundation

/// The user's own words, in ~/Library/Application Support/Bunny Keyboard,
/// in the same files and format as on Windows:
///
/// * my-words.txt: words the user added ("bong = បង"), which come first in
///   the suggestions. Written by the My words window, or by hand in TextEdit.
///   Read again whenever it changes, so a new word works at the next key.
/// * learned.txt: words the user corrected by picking a suggestion other
///   than the first. Only the word pairs, never the text around them.
/// * shared.txt: the words already sent through the Share tab.
struct Pair: Hashable {
    var spelling: String
    var khmer: String

    var line: String { "\(spelling) = \(khmer)" }
}

enum MyWords {
    static let header = [
        "# My words for Bunny Keyboard (Singkhmer).",
        "# One per line: the spelling, = and the Khmer word. For example: bong = បង",
        "# The keyboard reads this file again whenever it changes.",
    ]

    /// Set by the tests, to keep away from the user's own files.
    static var folderOverride: URL?

    static var folder: URL {
        if let folder = folderOverride { return folder }
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? URL(fileURLWithPath: NSHomeDirectory()).appendingPathComponent("Library/Application Support")
        return support.appendingPathComponent("Bunny Keyboard", isDirectory: true)
    }

    static var wordsFile: URL { folder.appendingPathComponent("my-words.txt") }
    static var learnedFile: URL { folder.appendingPathComponent("learned.txt") }
    static var sharedFile: URL { folder.appendingPathComponent("shared.txt") }

    static func read(_ file: URL) -> [Pair] {
        guard let text = try? String(contentsOf: file, encoding: .utf8) else { return [] }
        return text.components(separatedBy: .newlines).compactMap { line -> Pair? in
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if trimmed.isEmpty || trimmed.hasPrefix("#") { return nil }
            let separator: Character = trimmed.contains("\t") ? "\t" : "="
            let parts = trimmed.split(separator: separator, maxSplits: 1).map {
                $0.trimmingCharacters(in: .whitespaces)
            }
            guard parts.count == 2, !parts[0].isEmpty, !parts[1].isEmpty else { return nil }
            return Pair(spelling: parts[0].lowercased(), khmer: parts[1])
        }
    }

    static func save(_ words: [Pair]) throws {
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let lines = header + words.map(\.line)
        try (lines.joined(separator: "\n") + "\n").write(to: wordsFile, atomically: true, encoding: .utf8)
    }

    static func append(_ pairs: [Pair], to file: URL) {
        guard !pairs.isEmpty else { return }
        try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let text = pairs.map { $0.line + "\n" }.joined()
        guard let data = text.data(using: .utf8) else { return }
        if let handle = try? FileHandle(forWritingTo: file) {
            handle.seekToEndOfFile()
            handle.write(data)
            handle.closeFile()
        } else {
            try? data.write(to: file)
        }
    }

    /// Notes corrections in learned.txt. Failing to write loses only these.
    static func remember(_ corrections: [(String, String)]) {
        append(corrections.map { Pair(spelling: $0.0, khmer: $0.1) }, to: learnedFile)
    }

    /// What was read last: nil before the first read, then the file's date
    /// (.some(nil) when there was no file).
    private static var lastRead: Date??

    /// Hands my-words.txt to the engine again if it changed since it was last
    /// read, or was deleted. Cheap when nothing changed: one look at its date.
    static func refresh() {
        let date = (try? FileManager.default.attributesOfItem(atPath: wordsFile.path))?[.modificationDate] as? Date
        if let last = lastRead, last == date { return }
        let text = date == nil ? "" : ((try? String(contentsOf: wordsFile, encoding: .utf8)) ?? "")
        Engine.setUserWords(text)
        lastRead = .some(date)
    }

    /// Reads the file again at the next refresh, whatever its date: the My
    /// words window just saved it, possibly within the same second.
    static func changed() {
        lastRead = nil
        refresh()
    }
}

/// The Google Form that shared words go to, the same one as on Windows
/// (windows/my-words.ps1). An empty `url` switches sharing off.
enum ShareForm {
    static let url = "https://docs.google.com/forms/d/e/1FAIpQLSe2-on-wyrh2fevqq8J9xHY5pr8sWre5k0pcGvKpGgDJTcXRA/viewform"
    /// The Khmer question (a paragraph): one "spelling = Khmer" line per word.
    static let wordsField = "entry.1030374765"
    /// The Singkhmer question: the spellings on one line.
    static let spellingsField = "entry.1067540385"
    /// A pre-filled link holds this many words at most.
    static let mostPerSend = 25

    static var isOn: Bool { !url.isEmpty && !wordsField.isEmpty }

    /// The link that opens the form with these lines filled in.
    static func link(_ lines: [String]) -> URL? {
        let spellings = lines.map {
            $0.components(separatedBy: "=").first?.trimmingCharacters(in: .whitespaces) ?? ""
        }.joined(separator: ", ")
        var link = url + "?usp=pp_url&" + wordsField + "=" + escape(lines.joined(separator: "\n"))
        if !spellingsField.isEmpty {
            link += "&" + spellingsField + "=" + escape(spellings)
        }
        return URL(string: link)
    }

    /// Percent-encodes everything but ASCII letters, digits and - . _ ~, as
    /// Uri.EscapeDataString does on Windows.
    static func escape(_ text: String) -> String {
        let plain = Set("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~".utf8)
        return text.utf8.map { byte -> String in
            plain.contains(byte) ? String(Character(UnicodeScalar(byte))) : String(format: "%%%02X", UInt32(byte))
        }.joined()
    }
}
