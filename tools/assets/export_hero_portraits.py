"""Export hero portraits to local_extracted/ui/portraits/<HeroId>/ with uniform names.

The game's files vary ("StrikerJonesLv10", "SilasPortraitsLvl20", "ObynGreenFoot"); the HUD
looks for "<HeroId>Portrait" and "<HeroId>PortraitLvl<N>", so this renames them. Skins (sub-
folders) and extras (totems, idols, alternate forms) are skipped.

Usage: tools/.venv/Scripts/python.exe tools/assets/export_hero_portraits.py <game install> <HeroId>...
"""
import json
import os
import re
import subprocess
import sys

GAME, HEROES = sys.argv[1], sys.argv[2:]
# Portrait folders not named after the hero id.
DIRS = {"ObynGreenfoot": "Obyn"}
# Files that aren't the hero's own portraits.
SKIP = re.compile(r"Totem|Spiritless|SunGod|Masqued|Creepy|Genie|Icon|Idol", re.I)

paths = json.load(open("local_extracted/all_container_paths.json"))
for hero in HEROES:
    d = f"Assets/ResizedImages/MonkeyPortraits/{DIRS.get(hero, hero)}/"
    files = [p for p in paths if p.startswith(d) and "/" not in p[len(d):] and p.endswith(".png")
             and not SKIP.search(os.path.basename(p))
             and os.path.basename(p).lower().startswith(DIRS.get(hero, hero).lower())]
    out = f"local_extracted/ui/portraits/{hero}"
    if os.path.isdir(out):
        for f in os.listdir(out):
            os.remove(os.path.join(out, f))
    if not files:
        print(hero, "no portraits")
        continue
    subprocess.run([sys.executable, "tools/assets/export_textures.py", GAME, out, *files], capture_output=True)
    n = 0
    for f in os.listdir(out):
        m = re.search(r"Lvl?(\d+)\.png$", f)
        new = f"{hero}PortraitLvl{m.group(1)}.png" if m else f"{hero}Portrait.png"
        if f != new:
            os.replace(os.path.join(out, f), os.path.join(out, new))
        n += 1
    print(hero, n, "portraits")
