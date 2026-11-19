#!/bin/bash
# Builds Bunny Keyboard for iPhone and iPad (iOS 15 and newer), on a Mac with
# Rust (https://rustup.rs) and Xcode. There is no Xcode project: swiftc builds
# the app and its keyboard extension, as macos/build.sh does for the Mac.
#
#   ios/build.sh                builds target/ios/BunnyKeyboard-ios.ipa (for phones)
#                               and target/ios/sim/BunnyKeyboard.app (for the Simulator)
#   ios/build.sh --test         runs the tests first
#   ios/build.sh --screenshot   then runs it in the Simulator and takes pictures:
#                               target/ios/screenshot-*.png
#
# VERSION=0.5.0 ios/build.sh sets the version in the Info.plist files.
#
# The .ipa is not signed for any phone: iOS installs only apps signed with an
# Apple developer account. A tool such as Sideloadly signs it with the
# phone owner's Apple ID; see docs/DEVELOPMENT.md.
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="${VERSION:-0.0.0}"
OUT=target/ios
MIN_IOS=15.0
export IPHONEOS_DEPLOYMENT_TARGET=$MIN_IOS

run_tests=0
screenshot=0
for arg in "$@"; do
    case "$arg" in
        --test) run_tests=1 ;;
        --screenshot) screenshot=1 ;;
        *) echo "Unknown option: $arg" >&2; exit 2 ;;
    esac
done

# The engine is the Mac keyboard's C library (crates/khmer-macos), built for
# iOS. Its functions are declared for Swift in macos/Sources/khmer.h.
ENGINE_SWIFT=(macos/Sources/Engine.swift macos/Sources/Session.swift)
SHARED=(ios/Sources/Shared/Layout.swift ios/Sources/Shared/Typist.swift)
UIKIT_SHARED=(ios/Sources/Shared/KeyboardView.swift)

echo "== The engine (Rust), for phones and for the Simulator"
rustup target add aarch64-apple-ios aarch64-apple-ios-sim >/dev/null
for target in aarch64-apple-ios aarch64-apple-ios-sim; do
    cargo build --release --locked -p khmer-macos --target "$target"
done
mkdir -p "$OUT"

if [ "$run_tests" = 1 ]; then
    echo "== Tests (on this Mac, with the real engine)"
    cargo build --release --locked -p khmer-macos
    swiftc -O -swift-version 5 -import-objc-header macos/Sources/khmer.h \
        -L target/release -lkhmer_macos \
        "${ENGINE_SWIFT[@]}" "${SHARED[@]}" ios/Tests/main.swift -o "$OUT/keyboard-tests"
    "$OUT/keyboard-tests"
fi

swiftc -O -swift-version 5 ios/make-icon.swift -o "$OUT/make-icon"

# build <sdk> <swift target> <rust target> <platform name> <folder>
build() {
    local sdk="$1" target="$2" rust="$3" platform="$4" folder="$5"
    local sdk_path flags app appex
    sdk_path="$(xcrun --sdk "$sdk" --show-sdk-path)"
    flags=(-O -swift-version 5 -sdk "$sdk_path" -target "$target" -parse-as-library
           -import-objc-header macos/Sources/khmer.h -L "target/$rust/release" -lkhmer_macos)
    app="$folder/BunnyKeyboard.app"
    appex="$app/PlugIns/BunnyKeys.appex"
    rm -rf "$app"
    mkdir -p "$appex"

    echo "== The keyboard extension, for $platform"
    # An app extension has no main(): iOS starts it in NSExtensionMain.
    swiftc "${flags[@]}" -application-extension -module-name BunnyKeys \
        -Xlinker -e -Xlinker _NSExtensionMain \
        "${ENGINE_SWIFT[@]}" "${SHARED[@]}" "${UIKIT_SHARED[@]}" ios/Sources/Keyboard/*.swift \
        -o "$appex/BunnyKeys"
    sed -e "s/@VERSION@/$VERSION/g" -e "s/@PLATFORM@/$platform/g" ios/Keyboard-Info.plist > "$appex/Info.plist"
    plutil -lint "$appex/Info.plist" >/dev/null

    echo "== The app, for $platform"
    swiftc "${flags[@]}" -module-name BunnyKeyboard \
        "${ENGINE_SWIFT[@]}" "${SHARED[@]}" "${UIKIT_SHARED[@]}" ios/Sources/App/*.swift \
        -o "$app/BunnyKeyboard"
    sed -e "s/@VERSION@/$VERSION/g" -e "s/@PLATFORM@/$platform/g" ios/App-Info.plist > "$app/Info.plist"
    plutil -lint "$app/Info.plist" >/dev/null
    printf 'APPL????' > "$app/PkgInfo"
    "$OUT/make-icon" 120 "$app/AppIcon60x60@2x.png"
    "$OUT/make-icon" 180 "$app/AppIcon60x60@3x.png"

    # Signed "ad hoc" only, with no developer account: enough for the
    # Simulator. A phone needs it signed again (Sideloadly, or Xcode).
    codesign --force --sign - "$appex"
    codesign --force --sign - "$app"
}

build iphoneos "arm64-apple-ios$MIN_IOS" aarch64-apple-ios iPhoneOS "$OUT/device"
build iphonesimulator "arm64-apple-ios$MIN_IOS-simulator" aarch64-apple-ios-sim iPhoneSimulator "$OUT/sim"

echo "== The .ipa"
rm -rf "$OUT/Payload" "$OUT/BunnyKeyboard-ios.ipa"
mkdir -p "$OUT/Payload"
cp -R "$OUT/device/BunnyKeyboard.app" "$OUT/Payload/"
(cd "$OUT" && zip -qry BunnyKeyboard-ios.ipa Payload)
rm -rf "$OUT/Payload"
echo "Built $OUT/BunnyKeyboard-ios.ipa and $OUT/sim/BunnyKeyboard.app (version $VERSION)."

if [ "$screenshot" = 1 ]; then
    echo "== In the Simulator"
    device="$(xcrun simctl list devices available -j | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
phones = [d for runtime, list in devices.items() if "iOS" in runtime for d in list if d["name"].startswith("iPhone")]
print(phones[-1]["udid"] if phones else "")')"
    if [ -z "$device" ]; then
        echo "No iPhone simulator found." >&2
        exit 1
    fi
    xcrun simctl boot "$device" 2>/dev/null || true
    xcrun simctl bootstatus "$device" -b >/dev/null
    xcrun simctl install "$device" "$OUT/sim/BunnyKeyboard.app"

    # iOS must see the keyboard extension inside the app.
    if xcrun simctl spawn "$device" pluginkit -m -i com.newkeyboard.bunny.keyboard | grep -q com.newkeyboard.bunny.keyboard; then
        echo "ok    iOS lists the keyboard extension"
    else
        echo "FAIL  iOS does not list the keyboard extension" >&2
        exit 1
    fi

    for look in light dark; do
        xcrun simctl ui "$device" appearance "$look"
        xcrun simctl terminate "$device" com.newkeyboard.bunny 2>/dev/null || true
        xcrun simctl launch "$device" com.newkeyboard.bunny -type "nh jg tv sala rean" >/dev/null
        sleep 6
        xcrun simctl io "$device" screenshot "$OUT/screenshot-$look.png" >/dev/null
        echo "Took $OUT/screenshot-$look.png"
    done
    # Still running after the pictures: the app did not crash.
    if xcrun simctl spawn "$device" launchctl list | grep -q com.newkeyboard.bunny; then
        echo "ok    the app is still running"
    else
        echo "FAIL  the app stopped" >&2
        exit 1
    fi
    xcrun simctl shutdown "$device" || true
fi
