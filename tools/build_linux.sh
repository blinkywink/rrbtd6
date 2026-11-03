#!/bin/sh
# Builds RustTD6-linux for this machine from the working tree (personal use; git-ignored).
#
#     sh tools/build_linux.sh
#
# The game's data comes from a Windows install: copy its local_extracted folder into this
# checkout (next to where RustTD6-linux is written) before running the game.
#
# Needs Rust (https://rustup.rs) and the X11 / OpenGL / ALSA development packages, e.g.
#   Debian/Ubuntu: sudo apt install build-essential pkg-config libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev
#   Fedora:        sudo dnf install gcc pkgconf-pkg-config libX11-devel libXi-devel mesa-libGL-devel alsa-lib-devel
#   Arch:          sudo pacman -S base-devel pkgconf libx11 libxi mesa alsa-lib
set -e
cd "$(dirname "$0")/.."

[ "$(uname)" = Linux ] || { echo "build_linux.sh: run this on Linux" >&2; exit 1; }
command -v cargo >/dev/null 2>&1 || [ -x "$HOME/.cargo/bin/cargo" ] || {
  echo "Rust is missing: install it from https://rustup.rs, then run this again" >&2; exit 1; }
command -v cargo >/dev/null 2>&1 || PATH="$HOME/.cargo/bin:$PATH"
if command -v pkg-config >/dev/null 2>&1; then
  for lib in x11 xi gl alsa; do
    pkg-config --exists "$lib" || echo "warning: development files for '$lib' not found (see the package list at the top of this script)" >&2
  done
fi

# The same build as tools/build_exe.sh, for this machine.
CARGO_TARGET_DIR=target/game cargo build --profile game -p btd6-game
cp target/game/game/btd6-game RustTD6-linux
chmod +x RustTD6-linux
[ -f local_extracted/game/game.json ] || echo "note: copy local_extracted from your Windows install here before playing" >&2
echo "built $(pwd)/RustTD6-linux"
