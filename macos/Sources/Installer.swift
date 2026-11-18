import Carbon
import Foundation

/// Tells macOS about the keyboard, for macos/install.sh, which runs the app
/// with --install after copying it to ~/Library/Input Methods, and with
/// --uninstall before deleting it.
enum Installer {
    private static var sourceID: String { Bundle.main.bundleIdentifier ?? "" }

    private static func sources() -> [TISInputSource] {
        let filter = [kTISPropertyBundleID as String: sourceID] as CFDictionary
        guard let list = TISCreateInputSourceList(filter, true)?.takeRetainedValue() else { return [] }
        return (list as? [TISInputSource]) ?? []
    }

    static func install() -> Bool {
        let status = TISRegisterInputSource(Bundle.main.bundleURL as CFURL)
        if status != noErr {
            print("macOS did not take the keyboard (error \(status)).")
            return false
        }
        // macOS may take a moment to list a keyboard it was just told about.
        var found = sources()
        for _ in 0..<20 where found.isEmpty {
            usleep(250_000)
            found = sources()
        }
        if found.isEmpty {
            print("The keyboard is installed, but macOS has not listed it yet. Log out and in again.")
            return true
        }
        var enabled = false
        for source in found {
            enabled = TISEnableInputSource(source) == noErr || enabled
        }
        if !enabled {
            print("The keyboard is installed. Add it in System Settings (see below).")
        }
        return true
    }

    static func uninstall() -> Bool {
        for source in sources() {
            TISDisableInputSource(source)
        }
        return true
    }
}
