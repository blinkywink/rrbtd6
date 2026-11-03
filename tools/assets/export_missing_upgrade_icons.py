"""Export every upgrade / ability icon in Assets/ResizedImages/UI/UpgradeIcons/ that
local_extracted/ui/upgrades doesn't have yet (hero ability "...AA" icons, Corvus's spells...).

Skips Powers/, the unreleased Gadgeteer and Sheriff folders, and the tier-coded "001UpgradeIcon"
sets (export_tower_assets.py exports those with a tower prefix). Names with non-ASCII letters are
saved ASCII-folded too ("Pouākai" -> "Pouakai"), as the renderer looks them up.

Usage: tools/.venv/Scripts/python.exe tools/assets/export_missing_upgrade_icons.py <game install>
"""
import json
import os
import subprocess
import sys
import unicodedata

GAME = sys.argv[1]
PY = sys.executable
OUT = "local_extracted/ui/upgrades"
with open("local_extracted/all_container_paths.json", encoding="utf-8") as stream:
    paths = json.load(stream)
have = set(os.listdir(OUT))
want = []
for k in paths:
    if not (k.startswith("Assets/ResizedImages/UI/UpgradeIcons/") and k.lower().endswith(".png")):
        continue
    if any(s in k for s in ("/Powers/", "/Gadgeteer/", "/Sheriff/")):
        continue
    base = os.path.basename(k)
    if base[:3].isdigit() or base in have:
        continue
    want.append(k)
print(len(want), "icons to export", flush=True)
jobs = [{"out": OUT, "paths": want}] if want else []
# Names several towers' folders share (FasterShootingUpgradeIcon: Tack Shooter and Buccaneer):
# also saved as "<Folder>__<name>" so each tower gets its own.
import collections
by_name = collections.defaultdict(list)
for k in paths:
    if k.startswith("Assets/ResizedImages/UI/UpgradeIcons/") and k.lower().endswith(".png") and "/Powers/" not in k:
        by_name[os.path.basename(k)].append(k)
for base, ks in by_name.items():
    if len(ks) < 2 or base[:3].isdigit():
        continue
    for k in ks:
        folder = k.split("/")[4]
        dst = os.path.join(OUT, f"{folder}__{base}")
        if os.path.exists(dst):
            continue
        jobs.append({"out": OUT, "paths": [k], "name_prefix": folder + "__"})
if jobs:
    manifest = "local_extracted/.missing_icon_jobs.json"
    try:
        with open(manifest, "w", encoding="utf-8") as stream:
            json.dump(jobs, stream)
        subprocess.run([PY, "tools/assets/export_textures.py", GAME, "--jobs", manifest], check=True)
    finally:
        if os.path.exists(manifest):
            os.unlink(manifest)
for f in os.listdir(OUT):
    folded = unicodedata.normalize("NFKD", f).encode("ascii", "ignore").decode()
    if folded != f and not os.path.exists(os.path.join(OUT, folded)):
        with open(os.path.join(OUT, f), "rb") as a, open(os.path.join(OUT, folded), "wb") as b:
            b.write(a.read())
