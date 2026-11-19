import UIKit

/// The keyboard, as iOS runs it in every app (a keyboard extension). Its
/// name is set here for Keyboard-Info.plist (NSExtensionPrincipalClass).
@objc(KeyboardViewController)
final class KeyboardViewController: UIInputViewController {
    private lazy var field = ProxyField(proxy: textDocumentProxy)
    private lazy var typist = Typist(field: field)
    private let keyboard = KeyboardView()
    private var height: NSLayoutConstraint?
    /// The kind of text field the keys were last fitted to.
    private var fittedTo: UIKeyboardType?

    override func viewDidLoad() {
        super.viewDidLoad()
        Engine.warmUp()

        keyboard.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(keyboard)
        NSLayoutConstraint.activate([
            keyboard.topAnchor.constraint(equalTo: view.topAnchor),
            keyboard.bottomAnchor.constraint(equalTo: view.bottomAnchor),
            keyboard.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            keyboard.trailingAnchor.constraint(equalTo: view.trailingAnchor),
        ])
        let height = view.heightAnchor.constraint(equalToConstant: KeyboardView.height(compact: false))
        height.priority = .defaultHigh
        height.isActive = true
        self.height = height

        keyboard.keys.onKey = { [weak self] action in self?.typist.tap(action) }
        keyboard.suggestions.onPick = { [weak self] index in self?.typist.pick(index) }
        keyboard.keys.globe.addTarget(self, action: #selector(handleInputModeList(from:with:)), for: .allTouchEvents)
        typist.onChange = { [weak self] in self?.refresh() }
        startField()
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        refresh()
    }

    override func viewWillLayoutSubviews() {
        super.viewWillLayoutSubviews()
        height?.constant = KeyboardView.height(compact: traitCollection.verticalSizeClass == .compact)
    }

    override func viewWillDisappear(_ animated: Bool) {
        // The keyboard is going away: the phrase goes into the app as shown.
        typist.finish()
        super.viewWillDisappear(animated)
    }

    override func textWillChange(_ textInput: UITextInput?) {
        super.textWillChange(textInput)
        guard !field.editedJustNow, typist.isComposing else { return }
        // The app changed the text or moved the caret, or another text field
        // took over. The phrase stays in the app as it was shown.
        field.letGo()
        typist.forget()
    }

    override func textDidChange(_ textInput: UITextInput?) {
        super.textDidChange(textInput)
        guard !field.editedJustNow else { return }
        startField()
    }

    /// Fits the keys to the text field: Latin letters straight in for email
    /// and web addresses, the digits first for numbers.
    private func startField() {
        let numbers: [UIKeyboardType] = [.numberPad, .phonePad, .decimalPad, .numbersAndPunctuation, .asciiCapableNumberPad]
        let plain: [UIKeyboardType] = [.emailAddress, .URL, .webSearch]
        let type = textDocumentProxy.keyboardType ?? .default
        guard type != fittedTo else {
            refresh()
            return
        }
        fittedTo = type
        typist.plain = numbers.contains(type) || plain.contains(type)
        typist.reset(numbers: numbers.contains(type))
    }

    private func refresh() {
        keyboard.show(typist, globe: needsInputModeSwitchKey, enterLabel: enterLabel)
    }

    private var enterLabel: String {
        switch textDocumentProxy.returnKeyType ?? .default {
        case .go: return "go"
        case .search, .google, .yahoo: return "search"
        case .send: return "send"
        case .done: return "done"
        case .next: return "next"
        case .join: return "join"
        case .route: return "route"
        case .continue: return "continue"
        default: return "return"
        }
    }
}

/// The app's text field, through iOS's `textDocumentProxy`. The phrase is
/// shown as marked text, which iOS draws underlined.
final class ProxyField: Field {
    private let proxy: UITextDocumentProxy
    private var marked = false
    private var lastEdit = Date.distantPast

    init(proxy: UITextDocumentProxy) {
        self.proxy = proxy
    }

    /// Whether the keyboard itself changed the text a moment ago, so that a
    /// change iOS reports is ours and not the app's.
    var editedJustNow: Bool { Date().timeIntervalSince(lastEdit) < 0.3 }

    func insert(_ text: String) {
        lastEdit = Date()
        if marked {
            // Replaces the marked phrase, then leaves it as plain text.
            proxy.setMarkedText(text, selectedRange: NSRange(location: (text as NSString).length, length: 0))
            proxy.unmarkText()
            marked = false
        } else {
            proxy.insertText(text)
        }
    }

    func mark(_ text: String) {
        lastEdit = Date()
        if text.isEmpty {
            if marked {
                proxy.setMarkedText("", selectedRange: NSRange(location: 0, length: 0))
                proxy.unmarkText()
            }
            marked = false
        } else {
            proxy.setMarkedText(text, selectedRange: NSRange(location: (text as NSString).length, length: 0))
            marked = true
        }
    }

    /// Leaves the marked phrase in the app as plain text.
    func letGo() {
        if marked { proxy.unmarkText() }
        marked = false
    }

    func deleteBackward() {
        lastEdit = Date()
        proxy.deleteBackward()
    }

    func type(_ text: String) {
        lastEdit = Date()
        proxy.insertText(text)
    }
}
