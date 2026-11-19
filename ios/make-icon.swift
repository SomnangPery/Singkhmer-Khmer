// Draws the iPhone app's icon: the Khmer letter ក in white on the keyboard's
// blue. iOS rounds the corners itself. Used by build.sh:
//   make-icon <size in pixels> <out.png>
import AppKit

guard CommandLine.arguments.count == 3, let pixels = Int(CommandLine.arguments[1]) else {
    print("usage: make-icon <pixels> <out.png>")
    exit(2)
}
let size = CGFloat(pixels)
let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels,
                           bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                           colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
NSColor(red: 0x15 / 255.0, green: 0x65 / 255.0, blue: 0xC0 / 255.0, alpha: 1).setFill()
NSRect(x: 0, y: 0, width: size, height: size).fill()
let font = NSFont(name: "Khmer Sangam MN", size: size * 0.6) ?? NSFont.systemFont(ofSize: size * 0.6)
let letter = NSAttributedString(string: "ក", attributes: [.font: font, .foregroundColor: NSColor.white])
let bounds = letter.boundingRect(with: NSSize(width: size * 2, height: size * 2),
                                 options: [.usesLineFragmentOrigin, .usesDeviceMetrics])
letter.draw(at: NSPoint(x: (size - bounds.width) / 2 - bounds.minX, y: (size - bounds.height) / 2 - bounds.minY))
NSGraphicsContext.restoreGraphicsState()
guard let png = rep.representation(using: .png, properties: [:]) else { exit(1) }
try png.write(to: URL(fileURLWithPath: CommandLine.arguments[2]))
