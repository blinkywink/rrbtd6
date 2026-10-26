#!/usr/bin/env bash
# Exports every map scene bundle in the game (track, areas, terrain, 3D scene model).
# Usage: bash tools/assets/export_all_maps.sh [map keys...]   (default: every map_<key>_scenes_all bundle)
# Skips maps already exported. Proprietary output: local_extracted/ only.
set -u
GAME="${BTD6_DIR:-C:/Program Files/Epic Games/BloonsTD6}"
PY=tools/.venv/Scripts/python.exe
AA="$GAME/BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full"
if [ $# -gt 0 ]; then KEYS="$*"; else
  KEYS=$(ls "$AA" | grep '^map_.*_scenes_all_' | sed 's/^map_//; s/_scenes_all_.*$//' | sort -u | grep -v '^baseeditormap$')
fi
for K in $KEYS; do
  if [ -f "local_extracted/maps/$K/map.json" ] && [ -f "local_extracted/models/map_$K/meta.json" ]; then continue; fi
  echo "== $K"
  $PY tools/assets/export_map.py "$GAME" "$K" local_extracted/maps 2>&1 | tail -2
  $PY tools/assets/export_model.py "$GAME" "local_extracted/models/map_$K" scene "map_$K" 2>&1 | tail -1
done
