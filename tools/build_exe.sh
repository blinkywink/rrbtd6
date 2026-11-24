#!/bin/sh
# The one playable build: compiles the game from the working tree (everyone's current changes)
# and puts it at BTD6.exe in the project folder, next to local_extracted/ which it loads.
# Build cache: target/game. Close BTD6.exe first, or the copy fails.
#
#     sh tools/build_exe.sh          # fast incremental build (profile game)
#     sh tools/build_exe.sh --dist   # for a build to hand out: whole-program optimized (profile
#                                    # dist, ~4% faster sim, ~8 min to build)
set -e
cd "$(dirname "$0")/.."
profile=game
if [ "${1:-}" = "--dist" ]; then profile=dist; fi
CARGO_TARGET_DIR=target/game cargo build --profile $profile -p btd6-game
cp target/game/$profile/btd6-game.exe BTD6.exe
echo "built $(pwd)/BTD6.exe"
