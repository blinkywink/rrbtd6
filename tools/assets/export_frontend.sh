#!/usr/bin/env bash
# Frontend scenes, runtime controller data and hero appearance metadata: extract_all.py's steps.
set -euo pipefail
GAME="${1:?usage: export_frontend.sh GAME}"
cd "$(dirname "$0")/../.."
exec tools/.venv/Scripts/python.exe -I tools/extract_all.py --game "$GAME" --root . --only frontend home menu_additions
