#!/bin/bash
# Builds Bunny Keyboard for the Mac, for Apple silicon and Intel Macs, on a
# Mac with Rust (https://rustup.rs) and Xcode or its command line tools.
#
#   macos/build.sh           builds target/macos/BunnyKeyboard.app and BunnyKeyboard-mac.zip
#   macos/build.sh --test    runs the tests first
#
# VERSION=0.2.0 macos/build.sh sets the version shown in the app's Info.plist.
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="${VERSION:-0.0.0}"
OUT=target/macos
APP="$OUT/BunnyKeyboard.app"
MIN_MACOS=12.0
export MACOSX_DEPLOYMENT_TARGET=$MIN_MACOS

run_tests=0
for arg in "$@"; do
    case "$arg" in
        --test) run_tests=1 ;;
        *) echo "Unknown option: $arg" >&2; exit 2 ;;
    esac
done

echo "== The engine (Rust), for both kinds of Mac"
rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
for target in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --release --locked -p khmer-macos --target "$target"
done
mkdir -p "$OUT"
lipo -create -output "$OUT/libkhmer_macos.a" \
    target/aarch64-apple-darwin/release/libkhmer_macos.a \
    target/x86_64-apple-darwin/release/libkhmer_macos.a

SWIFT_FLAGS=(-O -swift-version 5 -import-objc-header macos/Sources/khmer.h -L "$OUT" -lkhmer_macos)

if [ "$run_tests" = 1 ]; then
    echo "== Tests"
    cargo test --release --locked -p khmer-macos
    host="$(uname -m)-apple-macos$MIN_MACOS"
    swiftc "${SWIFT_FLAGS[@]}" -target "$host" \
        macos/Sources/Engine.swift macos/Sources/Session.swift macos/Sources/MyWords.swift macos/Tests/main.swift \
        -o "$OUT/keyboard-tests"
    "$OUT/keyboard-tests"
fi

echo "== The keyboard (Swift)"
for arch in arm64 x86_64; do
    swiftc "${SWIFT_FLAGS[@]}" -target "$arch-apple-macos$MIN_MACOS" \
        -framework AppKit -framework Carbon -framework InputMethodKit -framework SwiftUI \
        macos/Sources/*.swift -o "$OUT/BunnyKeyboard-$arch"
done
swiftc -O -swift-version 5 macos/make-icon.swift -o "$OUT/make-icon"

echo "== The app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/en.lproj"
lipo -create -output "$APP/Contents/MacOS/BunnyKeyboard" "$OUT/BunnyKeyboard-arm64" "$OUT/BunnyKeyboard-x86_64"
sed "s/@VERSION@/$VERSION/g" macos/Info.plist > "$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist" >/dev/null
cp macos/InfoPlist.strings "$APP/Contents/Resources/en.lproj/InfoPlist.strings"
"$OUT/make-icon" "$APP/Contents/Resources/icon.tiff"
# Not signed by a developer, only "ad hoc", which Apple silicon Macs need to
# run it at all. install.sh downloads it with curl, which macOS does not mark
# as "from the internet", so Gatekeeper does not stop it.
codesign --force --sign - "$APP"

rm -f "$OUT/BunnyKeyboard-mac.zip"
(cd "$OUT" && ditto -c -k --keepParent BunnyKeyboard.app BunnyKeyboard-mac.zip)
echo "Built $APP and $OUT/BunnyKeyboard-mac.zip (version $VERSION)."
echo "To install this build: macos/install.sh --app $APP"
