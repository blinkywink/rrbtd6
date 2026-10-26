#!/usr/bin/env bash
# Build a portable copy on the Desktop: static-CRT exe + the runtime data, zipped.
# Private use only (contains art/data extracted from your own BTD6 install).
set -euo pipefail
cd "$(dirname "$0")/.."
RUSTFLAGS="-C target-feature=+crt-static" cargo build -p btd6-game --release --target-dir target/dist-static
OUT="$USERPROFILE/Desktop/BTD6-rrbtd6"
rm -rf "$OUT" "$OUT.zip"
mkdir -p "$OUT/local_extracted"
cp target/dist-static/release/btd6-game.exe "$OUT/"
for d in game maps sprites bloon_displays models effects ui fonts text audio; do cp -r "local_extracted/$d" "$OUT/local_extracted/"; done
(cd "$USERPROFILE/Desktop" && "$WINDIR/System32/tar.exe" -a -c -f BTD6-rrbtd6.zip BTD6-rrbtd6)
echo "packaged: $OUT.zip"
