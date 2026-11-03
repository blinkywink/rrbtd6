#!/usr/bin/env bash
# Regenerates the site's link preview image, website/public/social.png, with headless Chrome.
# Usage: sh tools/site_social_card.sh   (CHROME=<path> if Chrome isn't in the usual place)
set -euo pipefail
cd "$(dirname "$0")/.."

CHROME="${CHROME:-/c/Program Files/Google/Chrome/Application/chrome.exe}"
PY="${PYTHON:-tools/.venv/Scripts/python.exe}"
PORT=4174
ROOT="$(mktemp -d)"
trap 'kill "${SERVER:-0}" 2>/dev/null || true; rm -rf "$ROOT"' EXIT

# The site plus the card's source, which isn't deployed.
cp -r website/public "$ROOT/site"
cp website/social-card.html "$ROOT/site/"
"$PY" -m http.server "$PORT" --bind 127.0.0.1 --directory "$ROOT/site" >/dev/null 2>&1 &
SERVER=$!
sleep 1

native() { if command -v cygpath >/dev/null; then cygpath -w "$1"; else echo "$1"; fi; }
shot() { # page, output, width, height
  "$CHROME" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=1 \
    --user-data-dir="$(native "$ROOT/profile")" --window-size="$3,$4" \
    --screenshot="$(native "$PWD/$2")" "http://127.0.0.1:$PORT$1" >/dev/null 2>&1
  echo "$2"
}
shot /social-card.html website/public/social.png 1200 630
