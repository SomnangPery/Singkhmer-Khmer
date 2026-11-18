import AppKit

/// What the popup shows.
struct CandidateList: Equatable {
    /// The Latin letters typed ("nh jg tv"), small and grey at the top.
    var header: String
    var items: [String]
    var highlight: Int
    var buttons: [String]
}

/// The suggestion popup under the text being typed. It never takes the
/// keyboard focus from the app; a click on a suggestion or a button is
/// handed to `onPick` or `onButton`.
final class CandidatePanel: NSPanel {
    var onPick: (Int) -> Void = { _ in }
    var onButton: (Int) -> Void = { _ in }

    private let list = CandidateView()

    static func make() -> CandidatePanel {
        let panel = CandidatePanel(contentRect: NSRect(x: 0, y: 0, width: 200, height: 80),
                                   styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: true)
        panel.isFloatingPanel = true
        panel.level = NSWindow.Level(rawValue: Int(CGWindowLevelForKey(.popUpMenuWindow)))
        panel.backgroundColor = .clear
        panel.isOpaque = false
        panel.hasShadow = true
        panel.hidesOnDeactivate = false
        panel.isReleasedWhenClosed = false
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .ignoresCycle]
        panel.contentView = panel.list
        panel.list.onPick = { [weak panel] index in panel?.onPick(index) }
        panel.list.onButton = { [weak panel] index in panel?.onButton(index) }
        return panel
    }

    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }

    /// Shows `content` under `line`, the screen rectangle of the line being
    /// typed in (or above it, when there is no room below).
    func show(_ content: CandidateList, under line: NSRect) {
        list.content = content
        let size = list.fittingSize
        let screen = NSScreen.screens.first { $0.frame.contains(line.origin) } ?? NSScreen.main
        let visible = screen?.visibleFrame ?? NSRect(x: 0, y: 0, width: 10000, height: 10000)
        var origin = NSPoint(x: line.minX, y: line.minY - size.height - 4)
        if origin.y < visible.minY {
            origin.y = line.maxY + 4
        }
        origin.x = min(max(origin.x, visible.minX), visible.maxX - size.width)
        origin.y = min(max(origin.y, visible.minY), visible.maxY - size.height)
        setFrame(NSRect(origin: origin, size: size), display: true)
        orderFrontRegardless()
    }

    func highlight(_ index: Int) {
        list.content.highlight = index
    }

    func hide() {
        orderOut(nil)
    }
}

/// Draws the popup. Uses the Mac's own colours, so it follows light and dark mode.
final class CandidateView: NSView {
    var onPick: (Int) -> Void = { _ in }
    var onButton: (Int) -> Void = { _ in }

    var content = CandidateList(header: "", items: [], highlight: 0, buttons: []) {
        didSet {
            if content != oldValue {
                invalidateIntrinsicContentSize()
                needsDisplay = true
            }
        }
    }

    private let pad: CGFloat = 8
    private let gap: CGFloat = 16
    private let headerFont = NSFont.systemFont(ofSize: 11)
    private let itemFont = NSFont(name: "Khmer Sangam MN", size: 18) ?? NSFont.systemFont(ofSize: 18)
    private let numberFont = NSFont.monospacedDigitSystemFont(ofSize: 12, weight: .regular)
    private let buttonFont = NSFont.systemFont(ofSize: 12)

    override var isFlipped: Bool { true }
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

    private var rowHeight: CGFloat { ceil(itemFont.ascender - itemFont.descender + itemFont.leading) + 10 }
    private var headerHeight: CGFloat { content.header.isEmpty ? 0 : ceil(headerFont.ascender - headerFont.descender) + 6 }
    private var buttonHeight: CGFloat { content.buttons.isEmpty ? 0 : ceil(buttonFont.ascender - buttonFont.descender) + 12 }

    private func width(_ text: String, _ font: NSFont) -> CGFloat {
        ceil((text as NSString).size(withAttributes: [.font: font]).width)
    }

    override var intrinsicContentSize: NSSize {
        let number = width("3", numberFont) + 10
        let items = content.items.map { number + width($0, itemFont) }.max() ?? 0
        let buttons = content.buttons.map { width($0, buttonFont) }.reduce(0, +)
            + gap * CGFloat(max(content.buttons.count - 1, 0))
        let inner = max(items, buttons, width(content.header, headerFont), 180)
        let height = pad + headerHeight + rowHeight * CGFloat(content.items.count) + buttonHeight + pad
        return NSSize(width: inner + pad * 4, height: height)
    }

    override var fittingSize: NSSize { intrinsicContentSize }

    private func rowRect(_ index: Int) -> NSRect {
        NSRect(x: pad, y: pad + headerHeight + rowHeight * CGFloat(index), width: bounds.width - pad * 2, height: rowHeight)
    }

    private func buttonRects() -> [NSRect] {
        var x = pad * 2
        let y = pad + headerHeight + rowHeight * CGFloat(content.items.count) + 4
        return content.buttons.map { title in
            let rect = NSRect(x: x - 4, y: y, width: width(title, buttonFont) + 8, height: buttonHeight - 4)
            x += width(title, buttonFont) + gap
            return rect
        }
    }

    override func draw(_ dirtyRect: NSRect) {
        let frame = bounds.insetBy(dx: 0.5, dy: 0.5)
        let shape = NSBezierPath(roundedRect: frame, xRadius: 8, yRadius: 8)
        NSColor.windowBackgroundColor.setFill()
        shape.fill()
        NSColor.separatorColor.setStroke()
        shape.stroke()

        if !content.header.isEmpty {
            (content.header as NSString).draw(
                at: NSPoint(x: pad * 2, y: pad),
                withAttributes: [.font: headerFont, .foregroundColor: NSColor.secondaryLabelColor])
        }
        for (index, item) in content.items.enumerated() {
            let row = rowRect(index)
            let chosen = index == content.highlight
            if chosen {
                NSColor.selectedContentBackgroundColor.setFill()
                NSBezierPath(roundedRect: row, xRadius: 5, yRadius: 5).fill()
            }
            let text: NSColor = chosen ? .alternateSelectedControlTextColor : .labelColor
            let number = "\(index + 1)" as NSString
            let numberY = row.midY - (numberFont.ascender - numberFont.descender) / 2
            number.draw(at: NSPoint(x: row.minX + pad, y: numberY),
                        withAttributes: [.font: numberFont, .foregroundColor: chosen ? text : NSColor.tertiaryLabelColor])
            let itemY = row.midY - (itemFont.ascender - itemFont.descender) / 2
            (item as NSString).draw(at: NSPoint(x: row.minX + pad + width("3", numberFont) + 10, y: itemY),
                                    withAttributes: [.font: itemFont, .foregroundColor: text])
        }
        if !content.buttons.isEmpty {
            let lineY = pad + headerHeight + rowHeight * CGFloat(content.items.count) + 1
            NSColor.separatorColor.setFill()
            NSRect(x: pad, y: lineY, width: bounds.width - pad * 2, height: 1).fill()
            for (title, rect) in zip(content.buttons, buttonRects()) {
                (title as NSString).draw(at: NSPoint(x: rect.minX + 4, y: rect.minY + 4),
                                         withAttributes: [.font: buttonFont, .foregroundColor: NSColor.linkColor])
            }
        }
    }

    override func mouseDown(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        if let index = content.items.indices.first(where: { rowRect($0).contains(point) }) {
            onPick(index)
        } else if let index = buttonRects().firstIndex(where: { $0.contains(point) }) {
            onButton(index)
        }
    }

    override func resetCursorRects() {
        for rect in buttonRects() {
            addCursorRect(rect, cursor: .pointingHand)
        }
    }
}
