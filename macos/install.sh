#!/bin/bash
# Installs Bunny Keyboard (Singkhmer) on this Mac, straight from GitHub, with
# no Rust and no copy of the project. In Terminal:
#
#   curl -fsSL https://github.com/SomnangPery/Singkhmer-Khmer/releases/latest/download/install-mac.sh | bash
#
# To take it off again:
#
#   curl -fsSL https://github.com/SomnangPery/Singkhmer-Khmer/releases/latest/download/install-mac.sh | bash -s -- --remove
#
# To install a build made with macos/build.sh instead:
#
#   macos/install.sh --app target/macos/BunnyKeyboard.app
#
# The keyboard goes in ~/Library/Input Methods, for this user only, so no
# administrator password is needed. The user's words, in
# ~/Library/Application Support/Bunny Keyboard, are never touched.
set -euo pipefail

FROM="${BUNNY_FROM:-https://github.com/SomnangPery/Singkhmer-Khmer/releases/latest/download}"
DEST="$HOME/Library/Input Methods"
APP="$DEST/BunnyKeyboard.app"

remove=0
local_app=""
while [ $# -gt 0 ]; do
    case "$1" in
        --remove) remove=1 ;;
        --app) local_app="${2:?--app needs the path to BunnyKeyboard.app}"; shift ;;
        *) echo "Unknown option: $1" >&2; exit 2 ;;
    esac
    shift
done

stop_running() {
    # macOS starts the keyboard again by itself the next time it is needed.
    pkill -x BunnyKeyboard 2>/dev/null || true
}

if [ "$remove" = 1 ]; then
    if [ -x "$APP/Contents/MacOS/BunnyKeyboard" ]; then
        "$APP/Contents/MacOS/BunnyKeyboard" --uninstall || true
    fi
    stop_running
    rm -rf "$APP"
    echo "Bunny Keyboard is removed."
    echo "Your words are kept in ~/Library/Application Support/Bunny Keyboard; delete that folder to remove them too."
    exit 0
fi

major="$(sw_vers -productVersion | cut -d. -f1)"
if [ "$major" -lt 12 ]; then
    echo "Not done: Bunny Keyboard needs macOS 12 (Monterey) or newer. This Mac has $(sw_vers -productVersion)." >&2
    exit 1
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

if [ -n "$local_app" ]; then
    ditto "$local_app" "$work/BunnyKeyboard.app"
else
    echo "Downloading Bunny Keyboard..."
    if ! curl -fsSL "$FROM/BunnyKeyboard-mac.zip" -o "$work/BunnyKeyboard-mac.zip"; then
        echo "Not done: could not download BunnyKeyboard-mac.zip from $FROM." >&2
        echo "Check the internet connection; if it works, there may be no Mac release yet." >&2
        exit 1
    fi
    ditto -x -k "$work/BunnyKeyboard-mac.zip" "$work"
fi
if [ ! -x "$work/BunnyKeyboard.app/Contents/MacOS/BunnyKeyboard" ]; then
    echo "Not done: the download does not hold BunnyKeyboard.app." >&2
    exit 1
fi

mkdir -p "$DEST"
stop_running
rm -rf "$APP"
ditto "$work/BunnyKeyboard.app" "$APP"
xattr -dr com.apple.quarantine "$APP" 2>/dev/null || true
"$APP/Contents/MacOS/BunnyKeyboard" --install

cat <<'DONE'

Bunny Keyboard is installed.

Click the input menu in the menu bar (the flag or letter at the top right),
or press Control+Space, and pick "Singkhmer (Bunny Keyboard)".

Not in the menu? Open System Settings > Keyboard > Text Input > Edit...,
click +, choose Khmer, then Singkhmer (Bunny Keyboard), and click Add.
If it is not in that list either, log out and in again.
DONE
