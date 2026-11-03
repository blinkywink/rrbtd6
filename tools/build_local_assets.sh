#!/usr/bin/env bash
# Rebuild everything the game loads from your own BTD6 install into local_extracted/ (git-ignored,
# never redistribute): the same extraction players get, so setup can't drift from it.
# Options go to tools/extract_all.py (--only <step> ..., --force, --keep-bundle-cache).
set -euo pipefail
cd "$(dirname "$0")/.."
# The install: $BTD6_DIR, else Steam (the launcher's first choice), else Epic.
GAME="${BTD6_DIR:-}"
[ -n "$GAME" ] || { [ -f "C:/Program Files (x86)/Steam/steamapps/common/BloonsTD6/GameAssembly.dll" ] && GAME="C:/Program Files (x86)/Steam/steamapps/common/BloonsTD6" || GAME="C:/Program Files/Epic Games/BloonsTD6"; }
exec tools/.venv/Scripts/python.exe -I tools/extract_all.py --game "$GAME" --root . "$@"
