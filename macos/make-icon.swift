// Draws the keyboard's icon for the menu bar: the Khmer letter ក in a
// rounded square, black on clear, which macOS tints to match the menu bar.
// Used by build.sh:  make-icon <out.tiff>
import AppKit

func draw(pixels: Int) -> NSBitmapImageRep {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels,
                               bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                               colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    rep.size = NSSize(width: 16, height: 16)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    NSColor.black.set()
    let frame = NSBezierPath(roundedRect: NSRect(x: 0.75, y: 0.75, width: 14.5, height: 14.5), xRadius: 3, yRadius: 3)
    frame.lineWidth = 1.25
    frame.stroke()
    let font = NSFont(name: "Khmer Sangam MN", size: 11) ?? NSFont.systemFont(ofSize: 11)
    let letter = NSAttributedString(string: "ក", attributes: [.font: font, .foregroundColor: NSColor.black])
    let bounds = letter.boundingRect(with: NSSize(width: 100, height: 100), options: [.usesLineFragmentOrigin, .usesDeviceMetrics])
    letter.draw(at: NSPoint(x: (16 - bounds.width) / 2 - bounds.minX, y: (16 - bounds.height) / 2 - bounds.minY))
    NSGraphicsContext.restoreGraphicsState()
    return rep
}

guard CommandLine.arguments.count == 2 else {
    print("usage: make-icon <out.tiff>")
    exit(2)
}
let image = NSImage(size: NSSize(width: 16, height: 16))
image.addRepresentation(draw(pixels: 16))
image.addRepresentation(draw(pixels: 32))
guard let data = image.tiffRepresentation else { exit(1) }
try data.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
