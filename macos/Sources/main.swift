import AppKit
import InputMethodKit

// Bunny Keyboard for the Mac: an input method. macOS starts this app by
// itself when the user picks the keyboard, and talks to it through the
// connection named in Info.plist. It has no Dock icon and no window of its
// own, apart from My words.

let arguments = CommandLine.arguments
if arguments.contains("--install") {
    exit(Installer.install() ? 0 : 1)
}
if arguments.contains("--uninstall") {
    exit(Installer.uninstall() ? 0 : 1)
}

let connection = Bundle.main.infoDictionary?["InputMethodConnectionName"] as? String ?? "BunnyKeyboard_Connection"
let server = IMKServer(name: connection, bundleIdentifier: Bundle.main.bundleIdentifier)
if server == nil {
    NSLog("Bunny Keyboard: could not open the input method connection \(connection)")
}
let app = NSApplication.shared
app.setActivationPolicy(.accessory)
app.run()
