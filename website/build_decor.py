"""Cut single silhouettes out of the hero's captures for the page's decorations: public/decor/.

Usage (from the repository root), with the capture directory build_hero_silhouette.py kept:
    tools/.venv/Scripts/python.exe website/build_decor.py <capture dir>

Each image is a white shape on transparent; the page uses it as a CSS mask and picks the colour.
cloud-a.webp and cloud-b.webp drift beside the sections; island-far.webp and island-near.webp are the footer's horizon: a near island's gentle curve
with palms rooted on it, and a distant one behind (same size, stacked).
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageOps

# Name, capture layer, index among that layer's whole shapes (largest first; ones cut by the
# frame's edge don't count), output height in px (None: only drawn into the island).
SHAPES = [
    ("palm", "palms", 1, None),
    ("cloud-a", "_clouds", 0, 160),
    ("cloud-b", "_clouds", 1, 160),
]
PAD = 8
# The island, in px of a 2560-wide image whose bottom edge is the footer's top. Each island is one
# smooth hump (centre x, half-width, height above the bottom; a flat edge of EDGE px either side),
# with palms (shape, x of the trunk's foot, height, mirrored) rooted on it.
ISLAND_WIDTH, ISLAND_DEPTH = 2560, 600
EDGE = 24
FAR = (1750, 1250, 150)
FAR_PALMS = [("palm", 1880, 150, False), ("palm", 1990, 115, True)]
NEAR = (900, 1500, 110)
NEAR_PALMS = [("palm", 330, 290, False), ("palm", 440, 220, True), ("palm", 610, 170, False)]
SS = 2


def masks(cap):
    load = lambda name: np.asarray(Image.open(cap / f"{name}.png").convert("RGBA"), np.int16)
    base = load("_base")
    out = {name: np.abs(base - load(name)).max(-1) > 6 for name in {layer for _, layer, _, _ in SHAPES}
           if not name.startswith("_")}
    out["_clouds"] = load("_clouds")[..., 3] > 127
    return out


def shapes(mask):
    """Each connected shape that the frame doesn't cut: its bounding box and mask, largest first."""
    image = Image.fromarray(mask.astype(np.uint8) * 255).copy()
    found = []
    while True:
        ys, xs = np.nonzero(np.array(image) == 255)
        if not len(ys):
            break
        ImageDraw.floodfill(image, (int(xs[0]), int(ys[0])), 1)
        filled = np.array(image) == 1
        rows, cols = np.nonzero(filled.any(1))[0], np.nonzero(filled.any(0))[0]
        box = (cols[0], rows[0], cols[-1] + 1, rows[-1] + 1)
        if box[0] > 0 and box[1] > 0 and box[2] < mask.shape[1] and box[3] < mask.shape[0]:
            found.append((filled.sum(), box, filled[box[1]:box[3], box[0]:box[2]]))
        image = Image.fromarray(np.where(filled, 0, np.array(image)).astype(np.uint8)).copy()
    return [(box, m) for _, box, m in sorted(found, key=lambda f: -f[0])]


def white(alpha):
    image = Image.new("RGBA", alpha.size, (255, 255, 255, 0))
    image.putalpha(alpha)
    return image


def save(image, path):
    image.save(path, quality=90, method=6)
    print(path, image.size, path.stat().st_size)


def foot(mask):
    """Where a shape's trunk meets the ground: the middle of its lowest rows, as a fraction of its width."""
    rows = np.nonzero(mask.any(1))[0]
    xs = np.nonzero(mask[rows[-1] - max(2, len(rows) // 40):rows[-1] + 1].any(0))[0]
    return (xs.min() + xs.max()) / 2 / mask.shape[1]


def island(hump, palms, cut):
    """One smooth hump (a raised cosine), and palms with their trunks' feet a little into it."""
    centre, half, height = hump
    x = (np.arange(ISLAND_WIDTH * SS) + .5) / SS
    t = np.clip((x - centre) / half, -1, 1)
    surface = (ISLAND_DEPTH - EDGE - height * (1 + np.cos(np.pi * t)) / 2) * SS
    canvas = Image.fromarray(((np.arange(ISLAND_DEPTH * SS)[:, None] >= surface[None, :]) * 255).astype(np.uint8))
    for name, x0, h, mirrored in palms:
        f = foot(cut[name])
        shape = Image.fromarray(cut[name].astype(np.uint8) * 255)
        h *= SS
        shape = shape.resize((round(shape.width * h / shape.height), h), Image.LANCZOS)
        if mirrored:
            shape, f = ImageOps.mirror(shape), 1 - f
        root = int(surface[x0 * SS]) + 14 * SS + round(PAD * h / cut[name].shape[0])
        canvas.paste(255, (round(x0 * SS - f * shape.width), root - h), shape)
    return canvas


def main():
    cap = Path(sys.argv[1])
    out = Path(__file__).resolve().parent / "public" / "decor"
    out.mkdir(exist_ok=True)
    m = masks(cap)
    found, cut = {}, {}
    for name, layer, index, height in SHAPES:
        if layer not in found:
            found[layer] = shapes(m[layer])
        cut[name] = np.pad(found[layer][index][1], PAD)
        if height:
            alpha = Image.fromarray(cut[name].astype(np.uint8) * 255)
            save(white(alpha.resize((round(alpha.width * height / alpha.height), height), Image.BOX)), out / f"{name}.webp")
    layers = {"far": island(FAR, FAR_PALMS, cut), "near": island(NEAR, NEAR_PALMS, cut)}
    rows = np.nonzero(np.logical_or.reduce([np.array(c).any(1) for c in layers.values()]))[0]
    top = max(0, rows[0] - 4 * SS)
    for name, canvas in layers.items():
        canvas = canvas.crop((0, top, canvas.width, canvas.height))
        canvas = canvas.resize((ISLAND_WIDTH, canvas.height // SS), Image.BOX)
        save(white(canvas), out / f"island-{name}.webp")


if __name__ == "__main__":
    main()
