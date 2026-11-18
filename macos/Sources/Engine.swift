import Foundation

/// The Rust engine (crates/khmer-macos), in Singkhmer mode.
final class Engine {
    struct Output {
        var commit: String
        var preedit: String
        var handled: Bool
    }

    static let backspace: UInt32 = 8

    private let handle: OpaquePointer?

    init() {
        handle = bk_engine_new()
    }

    deinit {
        bk_engine_free(handle)
    }

    func key(_ key: UInt32) -> Output { Engine.read(bk_on_key(handle, key)) }
    func select(_ index: Int) -> Output { Engine.read(bk_select(handle, UInt32(max(index, 0)))) }
    func commit() -> Output { Engine.read(bk_commit(handle)) }
    func reset() { bk_reset(handle) }
    var isComposing: Bool { bk_is_composing(handle) }
    var candidates: [String] { Engine.lines(bk_candidates(handle)) }
    var typed: String { Engine.take(bk_typed(handle)) }

    /// Words corrected by picking another suggestion, as (spelling, word).
    func takeCorrections() -> [(String, String)] {
        Engine.lines(bk_take_corrections(handle)).compactMap { line in
            let parts = line.components(separatedBy: " = ")
            return parts.count == 2 ? (parts[0], parts[1]) : nil
        }
    }

    /// Reads the dictionary, so the first key does not wait. About a second.
    static func warmUp() {
        DispatchQueue.global(qos: .userInitiated).async { bk_warm_up() }
    }

    @discardableResult
    static func setUserWords(_ text: String) -> Int {
        Int(text.withCString { bk_set_user_words($0) })
    }

    /// nil when "spelling = word" is fine, otherwise what is wrong with it.
    static func check(spelling: String, word: String) -> String? {
        let line = "\(spelling) = \(word)"
        guard let message = line.withCString({ bk_check_word($0) }) else { return nil }
        return take(message)
    }

    /// The best readings of a romanized phrase.
    static func convert(_ phrase: String) -> [String] {
        lines(phrase.withCString { bk_convert($0) })
    }

    private static func take(_ text: UnsafeMutablePointer<CChar>?) -> String {
        guard let text = text else { return "" }
        defer { bk_free(text) }
        return String(cString: text)
    }

    private static func lines(_ text: UnsafeMutablePointer<CChar>?) -> [String] {
        take(text).split(separator: "\n").map(String.init)
    }

    private static func read(_ output: BKOutput) -> Output {
        defer { bk_output_free(output) }
        return Output(
            commit: output.commit.map { String(cString: $0) } ?? "",
            preedit: output.preedit.map { String(cString: $0) } ?? "",
            handled: output.handled)
    }
}
