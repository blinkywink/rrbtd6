"""Make the site's cursor: BTD6's hand cursor traced as a smooth white hand with a navy outline.

Usage (from the repository root):
    tools/.venv/Scripts/python.exe website/build_cursor.py

Reads local_extracted/ui/HandCursor{,Down}.png (64px) and writes public/cursor/hand{,-down}.png
(SIZE px) and hand{,-down}@2x.png. The game's dark outline and the hand inside it become two masks,
blown up, blurred and thresholded so the edges come out smooth rather than as upscaled pixels; the
white glow around the outline is left out.
"""
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter

SIZE = 40
FILL, OUTLINE = (255, 255, 255), (18, 48, 107)
# Luminance below this is the outline (the outline sits around 50, the hand's colours above 120).
DARK = 100
# Masks are traced at this many times the source size; BLUR (source pixels) rounds off the steps.
K, BLUR = 8, .7


def outside(passable):
    """Pixels reachable from the image border through `passable` ones."""
    reached = np.zeros_like(passable)
    reached[[0, -1], :] = passable[[0, -1], :]
    reached[:, [0, -1]] |= passable[:, [0, -1]]
    while True:
        grown = reached.copy()
        grown[1:] |= reached[:-1]
        grown[:-1] |= reached[1:]
        grown[:, 1:] |= reached[:, :-1]
        grown[:, :-1] |= reached[:, 1:]
        grown &= passable
        if (grown == reached).all():
            return reached
        reached = grown


def smooth(mask):
    big = Image.fromarray(mask.astype(np.uint8) * 255).resize((mask.shape[1] * K, mask.shape[0] * K), Image.BICUBIC)
    return np.asarray(big.filter(ImageFilter.GaussianBlur(BLUR * K))) > 127


def trace(path):
    rgba = np.asarray(Image.open(path).convert("RGBA")).astype(np.float32)
    dark = (rgba[..., :3] @ np.array([.299, .587, .114], np.float32) < DARK) & (rgba[..., 3] > 127)
    hand = ~outside(~dark)  # the outline and everything inside it
    body = smooth(hand)
    # The hand inside the outline, without the crease lines drawn on it.
    inner = smooth(hand & ~dark)
    inner = ~outside(~inner) & body
    image = np.zeros((*body.shape, 4), np.uint8)
    image[body] = (*OUTLINE, 255)
    image[inner] = (*FILL, 255)
    return Image.fromarray(image)


root = Path(__file__).resolve().parent
for source, name in (("HandCursor", "hand"), ("HandCursorDown", "hand-down")):
    image = trace(root.parent / "local_extracted" / "ui" / f"{source}.png")
    image.resize((SIZE, SIZE), Image.LANCZOS).save(root / "public" / "cursor" / f"{name}.png", optimize=True)
    image.resize((2 * SIZE, 2 * SIZE), Image.LANCZOS).save(root / "public" / "cursor" / f"{name}@2x.png", optimize=True)
