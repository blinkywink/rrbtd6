"""Render the home page's hero: the client's main-menu island in flat silhouettes.

Usage (from the repository root, after `cargo build --profile game -p btd6-game`):
    tools/.venv/Scripts/python.exe website/build_hero_silhouette.py <btd6-game.exe> [capture dir]

The client draws the island (RRBTD6_WORLD_LAYERS, renderer/src/frontend.rs) once whole and once
per layer below with that layer hidden; wherever the two differ, the layer is what shows there.
Each layer is then filled with one colour. The page stacks three images so the clouds can drift:
public/hero/sky.webp (the gradient), clouds-{2560,1920,1280}.webp (whole clouds, also where the
hills hide them) and scene-{2560,1920,1280}.webp (everything else, transparent where it's sky).
"""
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
from PIL import Image

WIDTH, HEIGHT, SCALE = 2560, 1440, 2
W = "MainMenuWorld"
BG = f"{W}/Background/Background"
MID = f"{W}/Background/Midground"
# Left out of the picture entirely, to keep it simple.
DROP = [f"{BG}/City", f"{BG}/Radar", f"{W}/Background/Temple", f"{MID}/Fence", f"{W}/Sky/SkyTerrain/"] \
    + [f"{BG}/{n}" for n in ("Totem", "Huts", "Plants", "Stickdudes", "Bushes")] \
    + [f"{W}/Town/{n}" for n in ("Hut", "Townhall", "Windmill", "Grass")] \
    + [f"{W}/Interactive/{n}" for n in ("FountainAnimation", "BombShooterPopUp", "MonkeyAcePopup",
                                         "MonkeySubPopUp", "NinjaMonkeyPopUp", "DigitalContent")]
# Back to front: name, scene path prefixes, colour. The page's hue (215°, 82% saturation), most
# tones a little darker than the page's blue (57%) so white text stands out, and lightness kept to
# a narrow band (41-59%) so no part jumps out: lighter where the game's colours are light (clouds,
# foam, sand, path), a little darker for what's nearer or darker in the game (jungle, rocks, palms).
LAYERS = [
    ("clouds", [f"{W}/Sky/Clouds"], "#4188ec"),  # 59%
    ("hills", [f"{BG}/Hills"], "#176ee8"),  # 50%
    ("jungle", [f"{BG}/Palms", f"{W}/Background/Trees"], "#1565d5"),  # 46%
    ("sand", [f"{MID}/Sand"], "#2577e9"),  # 53%
    ("sea", [f"{MID}/Sea"], "#166adf"),  # 48%
    ("shore", [f"{MID}/MainMenuShorewaves"], "#3782eb"),  # 57%
    ("rocks", [f"{MID}/Rocks"], "#1461cc"),  # 44%
    ("path", [f"{MID}/MiddleRocks"], "#2e7dea"),  # 55%
    ("grass", [f"{MID}/Grass"], "#166adf"),  # 48%
    ("palms", [f"{W}/Town/PalmTree"], "#135abe"),  # 41%
]
# The sky dome: captured too, only to tell sky apart from bits no layer claims.
SKY = f"{W}/Sky/SkyTerrain$"
SKY_TOP, SKY_HORIZON, HORIZON = "#145fc8", "#1c71e9", 0.42  # 43%, 51%
# Below this (fraction of the height), what no layer covers is wet sand the sea fades over.
SAND_BELOW = 0.6
SIZES = [2560, 1920, 1280]


def colour(h):
    return np.array([int(h[i:i + 2], 16) for i in (1, 3, 5)], np.float32)


def capture(exe, out):
    spec = {
        "out": str(out), "width": WIDTH * SCALE, "height": HEIGHT * SCALE, "wait": 240,
        "base": [W] + [f"-{p}" for p in DROP],
        "layers": [{"name": name, "paths": paths} for name, paths, _ in LAYERS] + [
            {"name": "_sky", "paths": [SKY]},
            # Everything but the clouds hidden: their whole outlines, on transparent.
            {"name": "_clouds", "paths": [f"{W}/Background", f"{W}/Town", f"{W}/Interactive", SKY]}],
    }
    (out / "spec.json").write_text(json.dumps(spec))
    env = dict(os.environ, RRBTD6_WORLD_LAYERS=str(out / "spec.json"), RRBTD6_MENU_BACKGROUND="1",
               RRBTD6_FRONT="main", RRBTD6_FRAMES="100000", RRBTD6_AUTOTEST="dummy")
    subprocess.run([exe], env=env, check=True)


def compose(out):
    load = lambda name: np.asarray(Image.open(out / f"{name}.png").convert("RGBA"), np.int16)
    base = load("_base")
    shows = lambda name: np.abs(base - load(name)).max(-1) > 6
    # Per capture pixel, the frontmost layer showing there (-1: none).
    top = np.full(base.shape[:2], -1, np.int8)
    for i, (name, _, _) in enumerate(LAYERS):
        top[shows(name)] = i
    # Bits no layer claims that aren't sky (the grass skirt at a palm's foot, the same colour as the
    # ground behind it) take the rearmost neighbouring layer, growing in from their edges.
    holes = (top < 0) & ~shows("_sky")
    while holes.any():
        around = np.stack([np.roll(top, s, a) for a in (0, 1) for s in (1, -1)])
        nearest = np.where(around >= 0, around, len(LAYERS)).min(0)
        grow = holes & (nearest < len(LAYERS))
        if not grow.any():
            break
        top[grow] = nearest[grow]
        holes &= ~grow
    shrink = lambda mask: np.asarray(Image.fromarray(mask.astype(np.uint8) * 255).resize((WIDTH, HEIGHT), Image.BOX),
                                     np.float32)[..., None] / 255
    fills = {name: colour(fill) for name, _, fill in LAYERS}
    t = np.clip(np.linspace(0, 1, HEIGHT, dtype=np.float32) / HORIZON, 0, 1)[:, None, None]
    sky = np.broadcast_to(colour(SKY_TOP) * (1 - t) + colour(SKY_HORIZON) * t, (HEIGHT, WIDTH, 3))
    clouds = rgba(fills["clouds"] * np.ones((HEIGHT, WIDTH, 1), np.float32), shrink(load("_clouds")[..., 3] > 127))
    # The scene, premultiplied: wet sand along the bottom, then every layer but the clouds over it.
    colours = np.zeros((HEIGHT, WIDTH, 3), np.float32)
    alpha = np.zeros((HEIGHT, WIDTH, 1), np.float32)
    colours[int(HEIGHT * SAND_BELOW):], alpha[int(HEIGHT * SAND_BELOW):] = fills["sand"], 1
    for i, (name, _, _) in enumerate(LAYERS):
        if name != "clouds":
            m = shrink(top == i)
            colours, alpha = colours * (1 - m) + fills[name] * m, alpha * (1 - m) + m
    # Fully transparent pixels get a blue like their neighbours' rather than black: smaller files.
    scene = rgba(np.where(alpha > 0, colours / np.maximum(alpha, 1e-6), fills["hills"]), alpha)
    return rgba(sky, np.ones((HEIGHT, WIDTH, 1), np.float32)).convert("RGB"), clouds, scene


def rgba(colours, alpha):
    return Image.fromarray(np.clip(np.concatenate([colours, alpha * 255], -1) + 0.5, 0, 255).astype(np.uint8))


def main():
    exe = sys.argv[1]
    hero = Path(__file__).resolve().parent / "public" / "hero"
    if len(sys.argv) > 2:
        # A directory to keep the captures in; when it has them already, only recompose.
        keep = Path(sys.argv[2])
        keep.mkdir(parents=True, exist_ok=True)
        if not (keep / "_base.png").exists():
            capture(exe, keep)
        sky, clouds, scene = compose(keep)
    else:
        with tempfile.TemporaryDirectory() as tmp:
            capture(exe, Path(tmp))
            sky, clouds, scene = compose(Path(tmp))
    # The sky is a plain vertical gradient: a small image scaled up loses nothing.
    outputs = [(hero / "sky.webp", sky, SIZES[-1] // 8)]
    outputs += [(hero / f"{name}-{width}.webp", image, width) for name, image in (("clouds", clouds), ("scene", scene))
                for width in SIZES]
    for path, image, width in outputs:
        small = np.array(image.resize((width, width * HEIGHT // WIDTH), Image.LANCZOS))
        if small.shape[-1] == 4:
            # Lanczos rings faintly around edges; snapping that back keeps the (lossless) alpha small.
            small[..., 3][small[..., 3] < 8], small[..., 3][small[..., 3] > 247] = 0, 255
            # Resizing leaves fully transparent pixels black; a blue compresses far better.
            small[small[..., 3] == 0, :3] = colour(dict((n, c) for n, _, c in LAYERS)["hills"])
        Image.fromarray(small).save(path, quality=90, method=6)
        print(path, path.stat().st_size)


if __name__ == "__main__":
    main()
