#!/bin/sh
# Builds RustTD6.ipa for a sideloaded iPhone or iPad, on Windows (Git Bash). Personal use: the .ipa holds
# your extracted game data, so it is git-ignored and never published.
#
#     sh tools/build_ios.sh path/to/iPhoneOS.sdk [path/to/local_extracted]
#
# Unsigned: the sideloading tool (AltStore, Sideloadly, TrollStore...) signs it. Mods and custom music go in
# the app's folder in the Files app ("On My iPhone > RustTD6 > local_extracted").
#
# Needs:
#   - Rust's iOS target:  rustup target add aarch64-apple-ios
#   - Zig 0.16 (`ZIG=path/to/zig.exe`, or `pip install ziglang==0.16.0` in tools/.venv): the C compiler and
#     linker, through tools/ios_cc.py.
#   - An iPhoneOS SDK folder (from Xcode: Platforms/iPhoneOS.platform/Developer/SDKs). A git checkout made
#     without symlinks leaves them as small text files; this script turns those back into copies.
set -e
cd "$(dirname "$0")/.."

sdk=$1
data=${2:-local_extracted}
[ -n "$sdk" ] && [ -d "$sdk/usr/lib" ] || { echo "usage: sh tools/build_ios.sh path/to/iPhoneOS.sdk [local_extracted]" >&2; exit 1; }
[ -f "$data/game/game.json" ] || { echo "No game data at '$data' (expected $data/game/game.json)." >&2; exit 1; }
py=tools/.venv/Scripts/python.exe
[ -x "$py" ] || { echo "build_ios.sh: tools/.venv is missing (run the setup first)" >&2; exit 1; }
if [ -z "$ZIG" ]; then
  ZIG=$("$py" -I -c "import os, ziglang; print(os.path.join(os.path.dirname(ziglang.__file__), 'zig.exe'))" 2>/dev/null) ||
    { echo "Zig is missing: set ZIG, or run $py -m pip install ziglang==0.16.0" >&2; exit 1; }
fi
rustup target list --installed | grep -qx aarch64-apple-ios || { echo "Run: rustup target add aarch64-apple-ios" >&2; exit 1; }

# Symlinks saved as text files hold just the target's relative path.
"$py" -I - "$sdk" <<'EOF'
import os, shutil, sys
fixed = 0
for _ in range(4):
    for folder, _, files in os.walk(sys.argv[1]):
        for name in files:
            path = os.path.join(folder, name)
            try:  # Some SDK paths are past Windows' length limit.
                if os.path.getsize(path) > 200 or not name.endswith((".tbd", ".h", ".modulemap")):
                    continue
                text = open(path, "rb").read().strip()
            except OSError:
                continue
            if b"\n" in text or not text or text.startswith((b"#", b"/", b"-")):
                continue
            target = os.path.normpath(os.path.join(folder, text.decode("utf-8", "replace")))
            if os.path.isfile(target) and os.path.getsize(target) > 200:
                shutil.copyfile(target, path)
                fixed += 1
if fixed:
    print("restored", fixed, "SDK links")
EOF

# Cargo and the cc crate need an executable for the compiler, linker and archiver.
wrap=target/ios/wrap
mkdir -p "$wrap"
root=$(cygpath -w "$(pwd)")
printf '@"%s" -I "%s\\tools\\ios_cc.py" %%*\r\n' "$(cygpath -w "$(pwd)/$py")" "$root" > "$wrap/cc.cmd"
printf '@"%s" ar %%*\r\n' "$(cygpath -w "$ZIG")" > "$wrap/ar.cmd"
w=$(cygpath -w "$(pwd)/$wrap")
ZIG=$(cygpath -w "$ZIG") SDKROOT=$(cygpath -m "$sdk") IPHONEOS_DEPLOYMENT_TARGET=15.0 \
  CC_aarch64_apple_ios="$w\\cc.cmd" AR_aarch64_apple_ios="$w\\ar.cmd" CARGO_TARGET_AARCH64_APPLE_IOS_LINKER="$w\\cc.cmd" \
  CARGO_TARGET_DIR=target/ios cargo build --profile game -p btd6-game --target aarch64-apple-ios

echo "Packaging RustTD6.ipa (about 3 GB)..."
"$py" -I - "$data" <<'EOF'
import os, sys, zipfile
from PIL import Image

data = sys.argv[1]
app = "Payload/RustTD6.app/"
# What the game reads, as in build_macos.sh. The player's own files are left out: on the phone they live
# in Documents (see ios_root in renderer/src/main.rs).
folders = ["ui", "game", "models", "maps", "text", "data", "shaders", "effects", "audio", "fonts", "sprites",
           "engine_data", "online", "bloon_displays", "brand"]
player = {"settings.json", "local-profile.json", "local-profile.backup.json", "player-name.json", "tower-xp.json"}
plist = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>RustTD6</string>
  <key>CFBundleDisplayName</key><string>RustTD6</string>
  <key>CFBundleIdentifier</key><string>com.rusttd6.game</string>
  <key>CFBundleExecutable</key><string>RustTD6</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>CFBundleSupportedPlatforms</key><array><string>iPhoneOS</string></array>
  <key>CFBundleIcons</key><dict><key>CFBundlePrimaryIcon</key><dict>
    <key>CFBundleIconFiles</key><array><string>AppIcon60x60</string><string>AppIcon76x76</string></array>
  </dict></dict>
  <key>DTPlatformName</key><string>iphoneos</string>
  <key>MinimumOSVersion</key><string>15.0</string>
  <key>UIDeviceFamily</key><array><integer>1</integer><integer>2</integer></array>
  <key>UIRequiredDeviceCapabilities</key><array><string>arm64</string></array>
  <key>UISupportedInterfaceOrientations</key><array>
    <string>UIInterfaceOrientationLandscapeRight</string><string>UIInterfaceOrientationLandscapeLeft</string>
  </array>
  <key>UISupportedInterfaceOrientations~ipad</key><array>
    <string>UIInterfaceOrientationLandscapeRight</string><string>UIInterfaceOrientationLandscapeLeft</string>
  </array>
  <key>UILaunchScreen</key><dict/>
  <key>UIRequiresFullScreen</key><true/>
  <key>UIStatusBarHidden</key><true/>
  <key>UIViewControllerBasedStatusBarAppearance</key><false/>
  <key>UIFileSharingEnabled</key><true/>
  <key>LSSupportsOpeningDocumentsInPlace</key><true/>
  <key>NSLocalNetworkUsageDescription</key><string>Co-op finds and connects to other players.</string>
</dict>
</plist>
"""
with zipfile.ZipFile("target/ios/RustTD6.ipa.tmp", "w", zipfile.ZIP_DEFLATED, compresslevel=1, allowZip64=True) as ipa:
    exe = zipfile.ZipInfo(app + "RustTD6")
    exe.external_attr = 0o100755 << 16
    exe.compress_type = zipfile.ZIP_DEFLATED
    ipa.writestr(exe, open("target/ios/aarch64-apple-ios/game/btd6-game", "rb").read())
    ipa.writestr(app + "Info.plist", plist)
    ipa.writestr(app + "PkgInfo", "APPL????")
    icon = os.path.join(data, "brand/icon.png")
    if os.path.exists(icon):
        image = Image.open(icon).convert("RGB")
        for name, size in [("AppIcon60x60@2x", 120), ("AppIcon60x60@3x", 180), ("AppIcon76x76@2x", 152)]:
            out = os.path.join("target/ios", name + ".png")
            image.resize((size, size), Image.LANCZOS).save(out)
            ipa.write(out, app + name + ".png")
    for name in os.listdir(data):
        path = os.path.join(data, name)
        if os.path.isfile(path) and name.endswith(".json") and name not in player:
            ipa.write(path, app + "local_extracted/" + name)
    for folder in folders:
        for parent, _, files in os.walk(os.path.join(data, folder)):
            for name in files:
                path = os.path.join(parent, name)
                ipa.write(path, app + "local_extracted/" + os.path.relpath(path, data).replace(os.sep, "/"))
os.replace("target/ios/RustTD6.ipa.tmp", "RustTD6.ipa")
print("built", os.path.abspath("RustTD6.ipa"), os.path.getsize("RustTD6.ipa") >> 20, "MB")
EOF
