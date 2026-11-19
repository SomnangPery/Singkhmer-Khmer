import UIKit

@main
final class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?

    func application(_ application: UIApplication,
                     didFinishLaunchingWithOptions options: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        Engine.warmUp()
        let window = UIWindow(frame: UIScreen.main.bounds)
        window.rootViewController = SetupViewController()
        window.makeKeyAndVisible()
        self.window = window
        return true
    }
}

/// The screen behind the app icon: how to turn the keyboard on, and a box to
/// try it in before that, with the same keys.
final class SetupViewController: UIViewController {
    private let tryField = TryField()
    private lazy var typist = Typist(field: tryField)
    private let keyboard = KeyboardView()

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .systemBackground

        let title = UILabel()
        title.text = "Bunny Keyboard"
        title.font = .systemFont(ofSize: 28, weight: .bold)

        let steps = UILabel()
        steps.numberOfLines = 0
        steps.font = .preferredFont(forTextStyle: .body)
        steps.text = """
        Type Khmer the way you write it in chat: nh jg tv sala rean becomes ខ្ញុំចង់ទៅសាលារៀន.

        To turn it on:
        1. Open Settings, then General → Keyboard → Keyboards.
        2. Tap Add New Keyboard… and pick Bunny Keyboard.
        3. In any app, hold 🌐 under the keys and pick Bunny Keyboard.

        Bunny Keyboard does not need Full Access: nothing you type leaves the phone.
        """

        let settings = UIButton(type: .system)
        settings.setTitle("Open Settings", for: .normal)
        settings.titleLabel?.font = .systemFont(ofSize: 17, weight: .semibold)
        settings.contentHorizontalAlignment = .leading
        settings.addTarget(self, action: #selector(openSettings), for: .touchUpInside)

        let tryTitle = UILabel()
        tryTitle.text = "Try it here:"
        tryTitle.font = .preferredFont(forTextStyle: .headline)

        tryField.label.numberOfLines = 0
        tryField.label.font = .systemFont(ofSize: 22)
        tryField.label.backgroundColor = .secondarySystemBackground
        tryField.label.layer.cornerRadius = 8
        tryField.label.clipsToBounds = true
        tryField.label.heightAnchor.constraint(greaterThanOrEqualToConstant: 60).isActive = true
        tryField.show()

        let stack = UIStackView(arrangedSubviews: [title, steps, settings, tryTitle, tryField.label])
        stack.axis = .vertical
        stack.spacing = 14
        stack.setCustomSpacing(4, after: tryTitle)

        let scroll = UIScrollView()
        scroll.alwaysBounceVertical = true
        for subview in [scroll, keyboard] {
            subview.translatesAutoresizingMaskIntoConstraints = false
            view.addSubview(subview)
        }
        stack.translatesAutoresizingMaskIntoConstraints = false
        scroll.addSubview(stack)
        let guide = view.safeAreaLayoutGuide
        NSLayoutConstraint.activate([
            scroll.topAnchor.constraint(equalTo: guide.topAnchor),
            scroll.leadingAnchor.constraint(equalTo: guide.leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: guide.trailingAnchor),
            scroll.bottomAnchor.constraint(equalTo: keyboard.topAnchor),
            stack.topAnchor.constraint(equalTo: scroll.contentLayoutGuide.topAnchor, constant: 20),
            stack.bottomAnchor.constraint(equalTo: scroll.contentLayoutGuide.bottomAnchor, constant: -20),
            stack.leadingAnchor.constraint(equalTo: scroll.frameLayoutGuide.leadingAnchor, constant: 20),
            stack.trailingAnchor.constraint(equalTo: scroll.frameLayoutGuide.trailingAnchor, constant: -20),
            keyboard.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            keyboard.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            keyboard.bottomAnchor.constraint(equalTo: guide.bottomAnchor),
            keyboard.heightAnchor.constraint(equalToConstant: KeyboardView.height(compact: false)),
        ])

        keyboard.keys.onKey = { [weak self] action in self?.typist.tap(action) }
        keyboard.suggestions.onPick = { [weak self] index in self?.typist.pick(index) }
        typist.onChange = { [weak self] in self?.refresh() }
        refresh()

        // For a picture of the keyboard at work (ios/build.sh --screenshot):
        //   -type "nh jg tv"  taps those keys at launch.
        if let at = CommandLine.arguments.firstIndex(of: "-type"), at + 1 < CommandLine.arguments.count {
            for letter in CommandLine.arguments[at + 1] {
                typist.tap(letter == " " ? .space : .text(String(letter)))
            }
        }
    }

    private func refresh() {
        keyboard.show(typist, globe: false, enterLabel: "return")
    }

    @objc private func openSettings() {
        if let url = URL(string: UIApplication.openSettingsURLString) {
            UIApplication.shared.open(url)
        }
    }
}

/// The "Try it" box: a label that shows the text, with the phrase being
/// typed underlined, as an app would show it.
final class TryField: Field {
    let label = UILabel()
    private var text = ""
    private var marked = ""

    func insert(_ inserted: String) {
        text += inserted
        marked = ""
        show()
    }

    func mark(_ shown: String) {
        marked = shown
        show()
    }

    func deleteBackward() {
        if !text.isEmpty { text.removeLast() }
        show()
    }

    func type(_ typed: String) {
        text += typed
        show()
    }

    func show() {
        let shown = NSMutableAttributedString(string: " " + text, attributes: [.foregroundColor: UIColor.label])
        shown.append(NSAttributedString(string: marked, attributes: [
            .foregroundColor: UIColor.label,
            .underlineStyle: NSUnderlineStyle.single.rawValue,
        ]))
        shown.append(NSAttributedString(string: "|", attributes: [.foregroundColor: UIColor.systemBlue]))
        label.attributedText = shown
    }
}
