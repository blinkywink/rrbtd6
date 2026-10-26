"""Quick software preview of an exported model (painter's algorithm, per-triangle
texture colour). Camera: orthographic, pitched `--pitch` degrees (BTD6 in-game: 60).

Usage: tools/.venv/Scripts/python.exe tools/assets/preview_model.py <model_dir> <out.png> [--pitch 60] [--size 800]
"""
import json, math, os, sys
import numpy as np
from PIL import Image, ImageDraw

d, out = sys.argv[1:3]
pitch = float(sys.argv[sys.argv.index("--pitch") + 1]) if "--pitch" in sys.argv else 60.0
size = int(sys.argv[sys.argv.index("--size") + 1]) if "--size" in sys.argv else 800
meta = json.load(open(os.path.join(d, "meta.json")))
V = np.fromfile(os.path.join(d, "verts.bin"), dtype=np.float32).reshape(-1, 5)
I = np.fromfile(os.path.join(d, "idx.bin"), dtype=np.uint32)
a = math.radians(pitch)
# Camera looks along fwd = (0, -sin a, cos a); screen up = (0, cos a, sin a).
fwd = np.array([0, -math.sin(a), math.cos(a)]); up = np.array([0, math.cos(a), math.sin(a)])
P = V[:, :3]
sx, sy, depth = P[:, 0], P @ up, P @ fwd
lo, hi = np.array([sx.min(), sy.min()]), np.array([sx.max(), sy.max()])
scale = (size - 20) / max(hi - lo)
img = Image.new("RGB", (size, size), (40, 60, 40)); dr = ImageDraw.Draw(img)
texs = {}
tris = []
for p in meta["parts"]:
    tex = None
    if p["texture"]:
        if p["texture"] not in texs:
            texs[p["texture"]] = Image.open(os.path.join(d, p["texture"])).convert("RGB")
        tex = texs[p["texture"]]
    idx = I[p["ioff"]:p["ioff"] + p["icount"]].reshape(-1, 3) + p["voff"]
    for t in idx:
        tris.append((depth[t].mean(), t, tex))
tris.sort(key=lambda x: -x[0])
for _, t, tex in tris:
    pts = [(10 + (sx[k] - lo[0]) * scale, size - 10 - (sy[k] - lo[1]) * scale) for k in t]
    if tex is not None:
        u, v = V[t, 3].mean() % 1.0, V[t, 4].mean() % 1.0
        col = tex.getpixel((min(int(u * tex.width), tex.width - 1), min(int((1 - v) * tex.height), tex.height - 1)))
    else:
        col = (200, 200, 200)
    dr.polygon(pts, fill=col)
img.save(out); print("saved", out, len(tris), "tris")
