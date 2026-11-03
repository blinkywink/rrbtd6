#!/bin/sh
# Builds RustTD6.app for this Mac from the working tree (personal use; the app is git-ignored).
#
#     sh tools/build_macos.sh [path/to/local_extracted] [--link]
#
# The game's data comes from a Windows install: copy its local_extracted folder to this Mac
# (by default the one in this checkout, ./local_extracted). The app gets a copy of the folders
# the game reads (about 3 GB), so it is self-contained; --link links to the data folder
# instead (small app, but keep the folder where it is). Settings and Monkey XP are saved in
# the app's data, as on Windows.
#
# Needs: Xcode command line tools (xcode-select --install) and Rust (https://rustup.rs).
set -e
cd "$(dirname "$0")/.."

data=local_extracted
copy=1
for arg in "$@"; do
  case "$arg" in
    --link) copy=0 ;;
    *) data="$arg" ;;
  esac
done

[ "$(uname)" = Darwin ] || { echo "build_macos.sh: run this on macOS" >&2; exit 1; }
xcode-select -p >/dev/null 2>&1 || { echo "Xcode command line tools are missing: run xcode-select --install" >&2; exit 1; }
command -v cargo >/dev/null 2>&1 || [ -x "$HOME/.cargo/bin/cargo" ] || {
  echo "Rust is missing: install it from https://rustup.rs, then run this again" >&2; exit 1; }
command -v cargo >/dev/null 2>&1 || PATH="$HOME/.cargo/bin:$PATH"
[ -d "$data" ] && [ -f "$data/game/game.json" ] || {
  echo "No game data at '$data' (expected $data/game/game.json)." >&2
  echo "Copy local_extracted from your Windows install and pass its path." >&2; exit 1; }
data=$(cd "$data" && pwd)

# The same build as tools/build_exe.sh, for this Mac's CPU.
CARGO_TARGET_DIR=target/game cargo build --profile game -p btd6-game

app=RustTD6.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/game/game/btd6-game "$app/Contents/MacOS/RustTD6"
# The game loads local_extracted from beside its executable.
if [ "$copy" = 1 ]; then
  # Only what the game reads: extraction caches and research leftovers stay behind.
  echo "Copying the game data into the app..."
  dest="$app/Contents/MacOS/local_extracted"
  mkdir -p "$dest"
  find "$data" -maxdepth 1 -type f -exec cp {} "$dest/" \;
  for d in ui game models maps text data shaders effects audio fonts sprites engine_data online bloon_displays brand; do
    [ -d "$data/$d" ] && cp -R "$data/$d" "$dest/$d"
  done
else
  ln -s "$data" "$app/Contents/MacOS/local_extracted"
fi

# The window icon (brand/icon.png) as the app icon.
icon=""
if [ -f "$data/brand/icon.png" ]; then
  set_dir=$(mktemp -d)/RustTD6.iconset
  mkdir -p "$set_dir"
  for s in 16 32 128 256 512; do
    sips -z $s $s "$data/brand/icon.png" --out "$set_dir/icon_${s}x${s}.png" >/dev/null
    d=$((s * 2))
    sips -z $d $d "$data/brand/icon.png" --out "$set_dir/icon_${s}x${s}@2x.png" >/dev/null
  done
  iconutil -c icns "$set_dir" -o "$app/Contents/Resources/RustTD6.icns" && icon=RustTD6
  rm -rf "$(dirname "$set_dir")"
fi

cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>RustTD6</string>
  <key>CFBundleDisplayName</key><string>RustTD6</string>
  <key>CFBundleIdentifier</key><string>com.rusttd6.game</string>
  <key>CFBundleExecutable</key><string>RustTD6</string>
  <key>CFBundleIconFile</key><string>$icon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF

# Built here, so not quarantined; an ad-hoc signature lets Apple Silicon run it (the
# executable only: the data link points outside the bundle).
codesign --force -s - "$app/Contents/MacOS/RustTD6" >/dev/null 2>&1 || true
echo "built $(pwd)/$app (open it from Finder, or: open $app)"
