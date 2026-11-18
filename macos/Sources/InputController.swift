import AppKit
import InputMethodKit

/// The app's text field, through the Mac's input method connection.
final class ClientTarget: TextTarget {
    private let client: IMKTextInput

    init?(_ sender: Any?) {
        guard let client = sender as? IMKTextInput else { return nil }
        self.client = client
    }

    private static let nowhere = NSRange(location: NSNotFound, length: 0)

    func insert(_ text: String) {
        client.insertText(text, replacementRange: ClientTarget.nowhere)
    }

    func mark(_ text: String) {
        let shown = NSAttributedString(string: text, attributes: [.underlineStyle: NSUnderlineStyle.single.rawValue])
        client.setMarkedText(shown, selectionRange: NSRange(location: (text as NSString).length, length: 0),
                             replacementRange: ClientTarget.nowhere)
    }

    /// The screen rectangle of the line being typed in, or nil when the app
    /// does not say.
    var line: NSRect? {
        var rect = NSRect.zero
        _ = client.attributes(forCharacterIndex: 0, lineHeightRectangle: &rect)
        return rect == .zero ? nil : rect
    }
}

/// The keyboard, for one text field. macOS makes one of these for each
/// field the user types in with Bunny Keyboard; they share one dictionary
/// and one popup.
@objc(BunnyInputController)
final class InputController: IMKInputController {
    static let panel = CandidatePanel.make()
    private static var warmedUp = false

    private lazy var session: Session = {
        let session = Session()
        session.onChange = { [weak self] in self?.refreshPanel() }
        session.onCorrections = { MyWords.remember($0) }
        return session
    }()
    private var lastLine: NSRect?

    override func activateServer(_ sender: Any!) {
        super.activateServer(sender)
        if !InputController.warmedUp {
            InputController.warmedUp = true
            Engine.warmUp()
        }
        MyWords.refresh()
    }

    override func deactivateServer(_ sender: Any!) {
        if let target = ClientTarget(sender) {
            session.finish(in: target)
        } else {
            session.forget()
        }
        InputController.panel.hide()
        super.deactivateServer(sender)
    }

    /// The app or the Mac wants the phrase finished: a click elsewhere in
    /// the text, for example.
    override func commitComposition(_ sender: Any!) {
        if let target = ClientTarget(sender) {
            session.finish(in: target)
        }
    }

    override func recognizedEvents(_ sender: Any!) -> Int {
        Int(NSEvent.EventTypeMask.keyDown.rawValue)
    }

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event = event, event.type == .keyDown, let target = ClientTarget(sender) else { return false }
        let held = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        let key = Key.from(keyCode: event.keyCode, characters: event.characters,
                           shift: held.contains(.shift), command: held.contains(.command),
                           control: held.contains(.control), option: held.contains(.option))
        // A word just added in My words, or by hand in TextEdit, works at once.
        MyWords.refresh()
        lastLine = target.line ?? lastLine
        return session.handle(key, in: target)
    }

    private func refreshPanel() {
        let panel = InputController.panel
        let items = session.candidates
        guard !items.isEmpty else {
            panel.hide()
            return
        }
        panel.onPick = { [weak self] index in
            guard let self = self, let target = ClientTarget(self.client()) else { return }
            self.session.pick(index, in: target)
        }
        panel.onButton = { [weak self] button in self?.openMyWords(button: button) }
        let content = CandidateList(header: session.typed, items: items, highlight: session.highlight,
                                    buttons: ["+ Add my word", "Wrong word?"])
        let mouse = NSEvent.mouseLocation
        let line = ClientTarget(client())?.line ?? lastLine ?? NSRect(x: mouse.x, y: mouse.y - 20, width: 1, height: 20)
        panel.show(content, under: line)
    }

    /// "+ Add my word" (0) or "Wrong word?" (1) in the popup.
    private func openMyWords(button: Int) {
        let typed = session.typed
        let got = session.candidates.first ?? ""
        // The last word typed is the one the user most likely means.
        let spelling = typed.split(separator: " ").last.map(String.init) ?? ""
        // The window takes the focus from the app, which ends the phrase
        // anyway: send it now, as it is shown, rather than lose it.
        if let target = ClientTarget(client()) {
            session.finish(in: target)
        }
        InputController.panel.hide()
        MyWordsWindow.show(spelling: spelling, report: button == 1 ? Report(typed: typed, got: got) : nil)
    }

    override func menu() -> NSMenu! {
        let menu = NSMenu()
        let item = NSMenuItem(title: "My words…", action: #selector(showMyWords(_:)), keyEquivalent: "")
        item.target = self
        menu.addItem(item)
        return menu
    }

    @objc private func showMyWords(_ sender: Any?) {
        MyWordsWindow.show()
    }
}
