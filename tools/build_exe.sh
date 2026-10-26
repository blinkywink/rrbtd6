#!/bin/sh
# The one playable build: compiles the game from the working tree (everyone's current changes)
# and puts it at BTD6.exe in the project folder, next to local_extracted/ which it loads.
# Build cache: target/game. Close BTD6.exe first, or the copy fails.
#
#     sh tools/build_exe.sh
set -e
cd "$(dirname "$0")/.."
CARGO_TARGET_DIR=target/game cargo build --profile game -p btd6-game
cp target/game/game/btd6-game.exe BTD6.exe
echo "built $(pwd)/BTD6.exe"
