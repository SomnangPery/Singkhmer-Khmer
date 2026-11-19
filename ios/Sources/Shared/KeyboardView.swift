import UIKit

/// The colours of the Android keyboard (android/.../Palette.kt), light and dark.
enum Palette {
    private static func pick(_ light: UInt32, _ dark: UInt32) -> UIColor {
        UIColor(dynamicProvider: { traits in
            let value = traits.userInterfaceStyle == .dark ? dark : light
            return UIColor(red: CGFloat((value >> 16) & 0xFF) / 255,
                           green: CGFloat((value >> 8) & 0xFF) / 255,
                           blue: CGFloat(value & 0xFF) / 255, alpha: 1)
        })
    }

    static let background = pick(0xE8EAED, 0x1B1C1F)
    static let key = pick(0xFFFFFF, 0x3B3D42)
    static let specialKey = pick(0xCDD1D6, 0x2A2C30)
    static let pressedKey = pick(0xB4B9C0, 0x5A5D64)
    static let text = pick(0x1B1C1F, 0xF1F1F3)
    static let hint = pick(0x6B6F76, 0x9EA1A8)
    static let accent = pick(0x1565C0, 0x8AB4F8)
    static let divider = pick(0xCDD1D6, 0x3B3D42)
}

/// The whole keyboard: the suggestions strip above the keys.
final class KeyboardView: UIView {
    let suggestions = SuggestionBar()
    let keys = KeysView()

    static let barHeight: CGFloat = 40

    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = Palette.background
        for view in [suggestions, keys] {
            view.translatesAutoresizingMaskIntoConstraints = false
            addSubview(view)
        }
        NSLayoutConstraint.activate([
            suggestions.topAnchor.constraint(equalTo: topAnchor),
            suggestions.leadingAnchor.constraint(equalTo: leadingAnchor),
            suggestions.trailingAnchor.constraint(equalTo: trailingAnchor),
            suggestions.heightAnchor.constraint(equalToConstant: KeyboardView.barHeight),
            keys.topAnchor.constraint(equalTo: suggestions.bottomAnchor),
            keys.leadingAnchor.constraint(equalTo: leadingAnchor),
            keys.trailingAnchor.constraint(equalTo: trailingAnchor),
            keys.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    /// The height for the keys and the strip: lower when the phone is on its side.
    static func height(compact: Bool) -> CGFloat {
        barHeight + CGFloat(5) * (compact ? 36 : 48)
    }

    /// Shows what `typist` has: its page of keys and its suggestions.
    func show(_ typist: Typist, globe: Bool, enterLabel: String) {
        keys.rows = Layout.rows(typist.page, shifted: typist.shift != .off, globe: globe,
                                spaceLabel: typist.plain ? "space" : "Singkhmer", enterLabel: enterLabel)
        keys.shift = typist.shift
        suggestions.show(typist.suggestions)
    }
}

/// The strip of suggestions: up to three, the first one in the accent colour.
final class SuggestionBar: UIView {
    var onPick: (Int) -> Void = { _ in }

    private let stack = UIStackView()
    private var shown: [String] = []

    override init(frame: CGRect) {
        super.init(frame: frame)
        stack.axis = .horizontal
        stack.distribution = .fillEqually
        stack.translatesAutoresizingMaskIntoConstraints = false
        addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: topAnchor),
            stack.bottomAnchor.constraint(equalTo: bottomAnchor),
            stack.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 4),
            stack.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -4),
        ])
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    func show(_ suggestions: [String]) {
        let suggestions = Array(suggestions.prefix(3))
        guard suggestions != shown else { return }
        shown = suggestions
        stack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        for (index, suggestion) in suggestions.enumerated() {
            let button = UIButton(type: .system)
            button.setTitle(suggestion, for: .normal)
            button.setTitleColor(index == 0 ? Palette.accent : Palette.text, for: .normal)
            button.titleLabel?.font = .systemFont(ofSize: 18, weight: index == 0 ? .semibold : .regular)
            button.titleLabel?.adjustsFontSizeToFitWidth = true
            button.titleLabel?.minimumScaleFactor = 0.5
            button.titleLabel?.lineBreakMode = .byTruncatingHead
            button.tag = index
            button.accessibilityLabel = suggestion
            button.addTarget(self, action: #selector(picked(_:)), for: .touchUpInside)
            stack.addArrangedSubview(button)
        }
    }

    @objc private func picked(_ button: UIButton) {
        onPick(button.tag)
    }
}

/// The keys, drawn in one view. A touch goes to the nearest key, so the gaps
/// between keys never swallow a tap, and a key is typed when the finger
/// lifts, as on iOS's own keyboards. Holding Backspace repeats it.
final class KeysView: UIView {
    var rows: [[KeyCap]] = [] {
        didSet {
            if rows != oldValue {
                setNeedsLayout()
                setNeedsDisplay()
            }
        }
    }
    var shift = Typist.Shift.off {
        didSet { if shift != oldValue { setNeedsDisplay() } }
    }
    var onKey: (KeyAction) -> Void = { _ in }

    /// The globe key's place, for KeyboardViewController to put iOS's own
    /// keyboard switching on (`handleInputModeList`). Hidden when there is no
    /// globe key.
    let globe = UIButton(type: .custom)

    private var frames: [[CGRect]] = []
    private var pressed: [UITouch: (row: Int, column: Int)] = [:]
    private var repeating: Timer?

    private let gap: CGFloat = 6

    override init(frame: CGRect) {
        super.init(frame: frame)
        isMultipleTouchEnabled = true
        backgroundColor = .clear
        contentMode = .redraw
        globe.isHidden = true
        globe.accessibilityLabel = "Next keyboard"
        addSubview(globe)
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    override func layoutSubviews() {
        super.layoutSubviews()
        let rowHeight = bounds.height / CGFloat(max(rows.count, 1))
        let unit = (bounds.width - gap) / CGFloat(Layout.columns)
        frames = rows.enumerated().map { index, row in
            let rowWidth = CGFloat(row.reduce(0) { $0 + $1.width }) * unit
            var x = (bounds.width - rowWidth) / 2
            return row.map { key in
                let width = CGFloat(key.width) * unit
                defer { x += width }
                return CGRect(x: x + gap / 2, y: CGFloat(index) * rowHeight + gap / 2,
                              width: width - gap, height: rowHeight - gap)
            }
        }
        globe.isHidden = true
        for (row, keys) in rows.enumerated() {
            for (column, key) in keys.enumerated() where key.action == .nextKeyboard {
                globe.frame = frames[row][column]
                globe.isHidden = false
            }
        }
        setNeedsDisplay()
    }

    override func traitCollectionDidChange(_ previous: UITraitCollection?) {
        super.traitCollectionDidChange(previous)
        setNeedsDisplay()
    }

    override func draw(_ rect: CGRect) {
        let isPressed = Set(pressed.values.map { "\($0.row),\($0.column)" })
        for (row, keys) in rows.enumerated() where row < frames.count {
            for (column, key) in keys.enumerated() where column < frames[row].count {
                let frame = frames[row][column]
                let down = isPressed.contains("\(row),\(column)")
                let lit = key.action == .shift && shift != .off
                let fill: UIColor = down ? Palette.pressedKey : (key.special && !lit ? Palette.specialKey : Palette.key)
                fill.setFill()
                UIBezierPath(roundedRect: frame, cornerRadius: 6).fill()
                drawLabel(for: key, in: frame, lit: lit)
            }
        }
    }

    private func drawLabel(for key: KeyCap, in frame: CGRect, lit: Bool) {
        var label = key.label
        if key.action == .shift {
            label = shift == .locked ? "⇪" : "⇧"
        }
        let isLetter: Bool
        if case .text = key.action { isLetter = true } else { isLetter = false }
        let size: CGFloat = isLetter ? min(22, frame.height * 0.5) : 15
        let color = key.action == .space ? Palette.hint : Palette.text
        let text = NSAttributedString(string: label, attributes: [
            .font: UIFont.systemFont(ofSize: size, weight: lit ? .bold : .regular),
            .foregroundColor: color,
        ])
        let measured = text.boundingRect(with: frame.size, options: [.usesLineFragmentOrigin], context: nil)
        text.draw(in: CGRect(x: frame.midX - measured.width / 2, y: frame.midY - measured.height / 2,
                             width: measured.width, height: measured.height))
    }

    /// The key nearest to `point`, by row and then within the row.
    private func key(at point: CGPoint) -> (row: Int, column: Int)? {
        guard !frames.isEmpty, bounds.height > 0 else { return nil }
        let row = min(max(Int(point.y / (bounds.height / CGFloat(frames.count))), 0), frames.count - 1)
        let columns = frames[row]
        guard !columns.isEmpty else { return nil }
        let column = columns.indices.min { abs(columns[$0].midX - point.x) < abs(columns[$1].midX - point.x) }!
        return (row, column)
    }

    private func action(_ at: (row: Int, column: Int)) -> KeyAction {
        rows[at.row][at.column].action
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        for touch in touches {
            guard let at = key(at: touch.location(in: self)), action(at) != .nextKeyboard else { continue }
            pressed[touch] = at
            if action(at) == .backspace {
                onKey(.backspace)
                startRepeating()
            } else if case .page = action(at) {
                // Pages change at once, so a quick "?123 then 1" works.
                onKey(action(at))
                pressed[touch] = nil
            }
        }
        setNeedsDisplay()
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        for touch in touches where pressed[touch] != nil {
            guard let at = key(at: touch.location(in: self)), action(at) != .nextKeyboard else { continue }
            if action(pressed[touch]!) != .backspace {
                pressed[touch] = at
            }
        }
        setNeedsDisplay()
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        for touch in touches {
            guard let at = pressed.removeValue(forKey: touch) else { continue }
            if action(at) == .backspace {
                stopRepeating()
            } else {
                onKey(action(at))
            }
        }
        setNeedsDisplay()
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        for touch in touches { pressed[touch] = nil }
        stopRepeating()
        setNeedsDisplay()
    }

    private func startRepeating() {
        stopRepeating()
        repeating = Timer.scheduledTimer(withTimeInterval: 0.45, repeats: false) { [weak self] _ in
            self?.repeating = Timer.scheduledTimer(withTimeInterval: 0.08, repeats: true) { [weak self] _ in
                self?.onKey(.backspace)
            }
        }
    }

    private func stopRepeating() {
        repeating?.invalidate()
        repeating = nil
    }
}
