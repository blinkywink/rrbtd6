"""Contact sheets of what the clone draws for each tower's projectiles, to eyeball wrong art.

One PNG per tower in <out>: every distinct projectile display its tiers use, labelled with the
game's prefab name, the clone folder it loads and the tiers using it. Sprites show their layers;
mesh projectiles show their first texture.

  tools/.venv/Scripts/python.exe tools/assets/display_sheet.py <out dir> [Tower ...]
"""
import glob
import json
import os
import sys
from collections import defaultdict

from PIL import Image, ImageDraw

L = "local_extracted"
out = sys.argv[1]
only = set(sys.argv[2:])
os.makedirs(out, exist_ok=True)
game = json.load(open(f"{L}/game/game.json", encoding="utf-8"))
prefabs = json.load(open(f"{L}/catalog_prefabs.json", encoding="utf-8"))
pindex = json.load(open(f"{L}/models/projectiles/index.json", encoding="utf-8"))


def prefab_name(g):
    p = prefabs.get(g)
    return os.path.splitext(os.path.basename(p))[0].strip() if p else "?"


def projectiles(o):
    if isinstance(o, dict):
        if "display" in o and "pierce" in o:
            yield o
        for v in o.values():
            yield from projectiles(v)
    elif isinstance(o, list):
        for v in o:
            yield from projectiles(v)


def thumb(folder, size=96):
    d = f"{L}/models/projectiles/{folder}"
    img = Image.new("RGBA", (size, size), (60, 60, 60, 255))
    files = []
    if os.path.exists(f"{d}/layers.json"):
        files = [f"{d}/{l['png']}" for l in json.load(open(f"{d}/layers.json"))["layers"]]
    elif os.path.exists(f"{d}/sprite.png"):
        files = [f"{d}/sprite.png"]
    else:
        files = sorted(glob.glob(f"{d}/tex/*.png"))[:1]
        if not files:
            files = sorted(glob.glob(f"{d}/tex/*"))[:1]
    for f in files:
        try:
            im = Image.open(f).convert("RGBA")
        except Exception:
            continue
        im.thumbnail((size, size))
        img.alpha_composite(im, ((size - im.width) // 2, (size - im.height) // 2))
    if not files:
        ImageDraw.Draw(img).text((4, 40), "no image", fill=(255, 80, 80, 255))
    return img


for tower, tt in sorted(game["tower_tiers"].items()):
    if only and tower not in only:
        continue
    if tt.get("hero"):
        continue
    uses = defaultdict(set)
    tiers = dict(tt.get("tiers") or {})
    for tier, spec in tiers.items():
        for pr in projectiles(spec):
            g = pr.get("display")
            if g and g != "#empty":
                uses[g].add(tier)
    if not uses:
        continue
    cell_w, cell_h, cols = 200, 150, 6
    rows = (len(uses) + cols - 1) // cols
    sheet = Image.new("RGBA", (cell_w * cols, cell_h * rows), (30, 30, 30, 255))
    dr = ImageDraw.Draw(sheet)
    for i, (g, ts) in enumerate(sorted(uses.items(), key=lambda kv: min(kv[1]))):
        x, y = (i % cols) * cell_w, (i // cols) * cell_h
        folder = pindex.get(g)
        if folder:
            sheet.alpha_composite(thumb(folder), (x + 52, y + 4))
        want = prefab_name(g)
        col = (255, 255, 255, 255) if folder and folder.lower() == want.lower() else (255, 120, 80, 255)
        dr.text((x + 4, y + 102), f"game: {want}"[:32], fill=col)
        dr.text((x + 4, y + 114), f"clone: {folder}"[:32], fill=col)
        tl = sorted(ts)
        dr.text((x + 4, y + 126), (" ".join(tl[:8]) + (" +" if len(tl) > 8 else ""))[:34], fill=(180, 180, 180, 255))
    sheet.convert("RGB").save(f"{out}/{tower}.png")
    print(tower, len(uses))
