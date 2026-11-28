"""Write the game's own GPU-compressed data (DXT1 / DXT5, with its mip chain) next to exported
model and map textures, so the renderer can upload them compressed: 4-8x less GPU memory than the
decoded RGBA PNGs, which stay (other platforms, CPU uses, every export kept).

    python tools/assets/export_compressed.py [--dirs models maps ...] [--force] [--game <install>]
    python tools/assets/export_compressed.py --mips      (adds mip levels to .bct files without)

Every texture PNG under local_extracted/<dirs> (tex/ and shared_textures/) is matched by its pixels:
each of the game's DXT1 / DXT5 Texture2D assets of a size some PNG has is decoded (after the flip
below) and looked up by a hash of its pixels (RGB for DXT1, RGBA for DXT5); every PNG with exactly
those pixels gets the blocks as <same name>.bct. Blocks are flipped top-down (Unity stores the bottom row first; the PNGs and
the renderer's UVs are top-first), losslessly: block rows reversed, each block's pixel rows
reversed in its index bits. (The match itself is the check: the flipped
blocks decode to exactly the PNG's pixels.)

--mips: the game's textures have no mip levels (1,148 of 1,153); the renderer samples model textures
through mips (smaller on screen than their size: without, the GPU reads far more texels). Each level
is the one above averaged 2x2 (render3d::half_size, what glGenerateMipmap gives for the PNG), DXT
encoded here (bounding-box endpoints, nearest palette entry); level 0 stays the game's own blocks.

.bct layout (little endian): b"RBCT", u32 version 1, u32 format (1 = DXT1 RGB, 5 = DXT5 RGBA),
u32 width, u32 height, u32 levels, then per level u32 width, u32 height, u32 bytes, then the
levels' blocks in order.
"""
import argparse
import os
import re
import struct
import sys
from collections import defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pathlib import Path

import numpy as np
import texture2ddecoder
import UnityPy
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "local_extracted"
GAME = Path(os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)")) / "Steam/steamapps/common/BloonsTD6"
AA = GAME / "BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64"
DXT1, DXT5 = 10, 12
BLOCK = {DXT1: 8, DXT5: 16}
CODE = {DXT1: 1, DXT5: 5}


def flip_color(b: np.ndarray, rows: int) -> None:
    """BC1 colour blocks (n x 8 bytes) flipped vertically within their first `rows` pixel rows."""
    idx = b[:, 4:8].copy()
    for r in range(rows):
        b[:, 4 + r] = idx[:, rows - 1 - r]


def flip_alpha(b: np.ndarray, rows: int) -> None:
    """BC3 alpha blocks (n x 8 bytes): 2 endpoints, then 16 3-bit indices (12 bits a row)."""
    bits = np.zeros(len(b), dtype=np.uint64)
    for k in range(6):
        bits |= b[:, 2 + k].astype(np.uint64) << np.uint64(8 * k)
    out = bits & ~np.uint64((1 << (12 * rows)) - 1)
    for r in range(rows):
        row = (bits >> np.uint64(12 * (rows - 1 - r))) & np.uint64(0xFFF)
        out |= row << np.uint64(12 * r)
    for k in range(6):
        b[:, 2 + k] = ((out >> np.uint64(8 * k)) & np.uint64(0xFF)).astype(np.uint8)


def flip_level(data: bytes, fmt: int, w: int, h: int) -> bytes:
    bw, bh, size = (w + 3) // 4, (h + 3) // 4, BLOCK[fmt]
    blocks = np.frombuffer(data, dtype=np.uint8).reshape(bh, bw, size)[::-1].reshape(-1, size).copy()
    rows = min(h, 4)
    if fmt == DXT1:
        flip_color(blocks, rows)
    else:
        flip_alpha(blocks[:, 0:8], rows)
        sub = blocks[:, 8:16].copy()
        flip_color(sub, rows)
        blocks[:, 8:16] = sub
    return blocks.tobytes()


def decode(data: bytes, fmt: int, w: int, h: int) -> np.ndarray:
    raw = (texture2ddecoder.decode_bc1 if fmt == DXT1 else texture2ddecoder.decode_bc3)(data, w, h)
    bgra = np.frombuffer(raw, dtype=np.uint8).reshape(h, w, 4)
    return bgra[..., [2, 1, 0, 3]]


def levels(t, fmt: int):
    """(width, height, bytes) per mip level of the texture's data, bottom row first."""
    data = bytes(t.get_image_data() or b"")
    w, h, at, out = t.m_Width, t.m_Height, 0, []
    for _ in range(max(1, t.m_MipCount)):
        n = ((w + 3) // 4) * ((h + 3) // 4) * BLOCK[fmt]
        if at + n > len(data):
            break
        out.append((w, h, data[at:at + n]))
        at += n
        if w == 1 and h == 1:
            break
        w, h = max(1, w // 2), max(1, h // 2)
    return out


def half(px: np.ndarray) -> np.ndarray:
    """The next mip of an RGBA image: each texel the rounded mean of a 2x2 block."""
    h, w = max(1, px.shape[0] // 2), max(1, px.shape[1] // 2)
    # (A side already 1 texel counts its texel twice.)
    a = px.astype(np.uint32)
    if a.shape[0] > 1:
        a = a[0:h * 2:2] + a[1:h * 2:2]
    else:
        a = a * 2
    if a.shape[1] > 1:
        a = a[:, 0:w * 2:2] + a[:, 1:w * 2:2]
    else:
        a = a * 2
    return ((a + 2) // 4).astype(np.uint8)


def to565(c: np.ndarray) -> np.ndarray:
    c = c.astype(np.int32)
    return ((c[..., 0] * 31 + 127) // 255 << 11) | ((c[..., 1] * 63 + 127) // 255 << 5) | ((c[..., 2] * 31 + 127) // 255)


def from565(v: np.ndarray) -> np.ndarray:
    r, g, b = (v >> 11) & 31, (v >> 5) & 63, v & 31
    return np.stack([(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2)], -1).astype(np.int32)


def blocks_of(px: np.ndarray) -> np.ndarray:
    """(n, 16, 4) pixels of the 4x4 blocks, rows of blocks top first (edges repeated)."""
    h, w = px.shape[:2]
    H, W = (h + 3) // 4 * 4, (w + 3) // 4 * 4
    pad = np.pad(px, ((0, H - h), (0, W - w), (0, 0)), mode="edge")
    return pad.reshape(H // 4, 4, W // 4, 4, 4).transpose(0, 2, 1, 3, 4).reshape(-1, 16, 4)


def encode_color(b: np.ndarray) -> np.ndarray:
    """BC1 colour blocks (4-colour mode) for (n, 16, 3+) pixels: (n, 8) bytes."""
    rgb = b[..., :3].astype(np.int32)
    lo, hi = rgb.min(1), rgb.max(1)
    c0, c1 = to565(hi), to565(lo)
    swap = c0 < c1
    c0, c1 = np.where(swap, c1, c0), np.where(swap, c0, c1)
    e0, e1 = from565(c0), from565(c1)
    pal = np.stack([e0, e1, (2 * e0 + e1) // 3, (e0 + 2 * e1) // 3], 1)
    d = ((rgb[:, :, None, :] - pal[:, None, :, :]) ** 2).sum(-1)
    idx = d.argmin(-1).astype(np.uint32)
    idx[c0 == c1] = 0
    bits = (idx << (2 * np.arange(16, dtype=np.uint32))).sum(1, dtype=np.uint64).astype(np.uint32)
    out = np.zeros((len(b), 8), np.uint8)
    out[:, 0:2] = c0.astype("<u2").view(np.uint8).reshape(-1, 2)
    out[:, 2:4] = c1.astype("<u2").view(np.uint8).reshape(-1, 2)
    out[:, 4:8] = bits.astype("<u4").view(np.uint8).reshape(-1, 4)
    return out


def encode_alpha(b: np.ndarray) -> np.ndarray:
    """BC3 alpha blocks (8-value mode) for (n, 16, 4) pixels: (n, 8) bytes."""
    a = b[..., 3].astype(np.int32)
    a0, a1 = a.max(1), a.min(1)
    k = np.arange(8)
    w0 = np.array([7, 0, 6, 5, 4, 3, 2, 1])
    w1 = 7 - w0
    w1[0], w1[1] = 0, 7
    pal = (a0[:, None] * w0 + a1[:, None] * w1 + 3) // 7
    idx = np.abs(a[:, :, None] - pal[:, None, :]).argmin(-1).astype(np.uint64)
    idx[a0 == a1] = 0
    bits = (idx << (3 * np.arange(16, dtype=np.uint64))).sum(1, dtype=np.uint64)
    out = np.zeros((len(b), 8), np.uint8)
    out[:, 0], out[:, 1] = a0, a1
    for i in range(6):
        out[:, 2 + i] = (bits >> np.uint64(8 * i)) & np.uint64(0xFF)
    return out


def encode(px: np.ndarray, code: int) -> bytes:
    b = blocks_of(px)
    if code == 1:
        return encode_color(b).tobytes()
    return np.concatenate([encode_alpha(b), encode_color(b)], 1).tobytes()


def add_mips(bct: Path) -> bool:
    data = bct.read_bytes()
    magic, ver, code, w, h, n = struct.unpack("<4s5I", data[:24])
    if magic != b"RBCT" or n != 1 or w < 8 or h < 8:
        return False
    lw, lh, ln = struct.unpack("<3I", data[24:36])
    base = data[36:36 + ln]
    px = np.asarray(Image.open(bct.with_suffix(".png")).convert("RGBA"))
    levels = [(w, h, base)]
    while px.shape[0] > 1 or px.shape[1] > 1:
        px = half(px)
        levels.append((px.shape[1], px.shape[0], encode(px, code)))
    header = struct.pack("<4s5I", b"RBCT", 1, code, w, h, len(levels))
    header += b"".join(struct.pack("<3I", a, b_, len(d)) for a, b_, d in levels)
    bct.write_bytes(header + b"".join(d for _, _, d in levels))
    return True


def load_bundle(path: Path):
    """UnityPy.load, or (too big to decompress in memory) its decompressed files in
    local_extracted/_bundle_cache/<name>-*/ (export_model.load_bundle)."""
    try:
        return UnityPy.load(str(path))
    except MemoryError:
        for d in sorted((OUT / "_bundle_cache").glob(path.name + "-*")):
            if d.is_dir() and (d / ".complete").exists():
                print("(loading", path.name, "from the bundle cache)", flush=True)
                return UnityPy.load(*[str(f) for f in sorted(d.iterdir()) if f.name.startswith("CAB-")])
        raise


def pixel_key(rgba: np.ndarray, alpha: bool) -> bytes:
    import hashlib
    h = hashlib.sha256()
    h.update(struct.pack("<2I", rgba.shape[1], rgba.shape[0]))
    h.update(np.ascontiguousarray(rgba if alpha else rgba[..., :3]).tobytes())
    return h.digest()


def png_keys(p):
    """(path, RGB key or None, RGBA key, size) of one exported PNG, or None (too small)."""
    with Image.open(p) as im:
        if im.size[0] < 64 or im.size[1] < 64:
            return None
        px = np.asarray(im.convert("RGBA"))
    # (DXT1 goes up as opaque RGB, as Unity samples it: only for PNGs with no transparency.)
    rgb = pixel_key(px, False) if (px[..., 3] == 255).all() else None
    return p, rgb, pixel_key(px, True), (px.shape[1], px.shape[0])


_WANT = None


def _init(want):
    global _WANT
    _WANT = want


def bundle_matches(bundle):
    """The bundle's DXT textures whose pixels match an exported PNG: [(fmt, key, blob)] in object
    order (main applies them in bundle order, so the result doesn't depend on which worker ran)."""
    rgb_keys, rgba_keys, sizes = _WANT
    try:
        env = load_bundle(bundle)
    except Exception as e:
        return [], f"skip bundle {bundle.name} {type(e).__name__} {e}"
    out = []
    for obj in env.objects:
        if obj.type.name != "Texture2D":
            continue
        try:
            t = obj.read()
            fmt = int(t.m_TextureFormat)
            if fmt not in BLOCK or (t.m_Width, t.m_Height) not in sizes:
                continue
            lv = levels(t, fmt)
        except Exception:
            continue
        if not lv:
            continue
        w, h, _ = lv[0]
        flipped = [(lw, lh, flip_level(d, fmt, lw, lh)) for lw, lh, d in lv]
        mine = decode(flipped[0][2], fmt, w, h)
        key = pixel_key(mine, False) if fmt == DXT1 else pixel_key(mine, True)
        if key not in (rgb_keys if fmt == DXT1 else rgba_keys):
            continue
        header = struct.pack("<4s5I", b"RBCT", 1, CODE[fmt], w, h, len(flipped))
        header += b"".join(struct.pack("<3I", lw, lh, len(d)) for lw, lh, d in flipped)
        out.append((fmt, key, header + b"".join(d for _, _, d in flipped)))
    return out, None


def workers():
    """Processes for the independent pieces: by free memory, like the model stages."""
    from model_batch import worker_count
    return max(1, min(worker_count(per_worker=1 << 30, max_workers=12), int(os.environ.get("RRBTD6_SETUP_WORKERS", "12"))))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dirs", nargs="*", default=["models"])
    ap.add_argument("--force", action="store_true")
    ap.add_argument("--mips", action="store_true")
    ap.add_argument("--game", help="the BTD6 install (default: Steam's)")
    args = ap.parse_args()
    global AA
    if args.game:
        AA = Path(args.game) / "BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64"
    from concurrent.futures import ProcessPoolExecutor
    if args.mips:
        files = sorted(b for d in args.dirs for b in (OUT / d).rglob("*.bct"))
        with ProcessPoolExecutor(workers()) as pool:
            done = sum(pool.map(add_mips, files, chunksize=32))
        print(f"mips added to {done} of {len(files)} compressed textures")
        return
    # Every exported texture PNG (per-model tex/ and the deduplicated shared_textures/), keyed by
    # its pixels: RGB (a DXT1 match) and RGBA (a DXT5 match). Copies of one image share a key.
    by_rgb, by_rgba, sizes = defaultdict(list), defaultdict(list), set()
    pngs = sorted(p for d in args.dirs for p in (OUT / d).rglob("*.png")
                  if p.parent.name in ("tex", "shared_textures") and (args.force or not p.with_suffix(".bct").exists()))
    n = 0
    with ProcessPoolExecutor(workers()) as pool:
        for found in pool.map(png_keys, pngs, chunksize=16):
            if found is None:
                continue
            p, rgb, rgba, size = found
            if rgb is not None:
                by_rgb[rgb].append(p)
            by_rgba[rgba].append(p)
            sizes.add(size)
            n += 1
    print(f"{n} PNGs to match, {len(by_rgba)} distinct images, {len(sizes)} sizes", flush=True)
    done = set()
    written = 0
    bundles = [b for b in sorted(AA.rglob("*.bundle")) if "Half" not in b.parts]
    # Bundles are read in parallel; their matches are written in bundle order, as one process did.
    with ProcessPoolExecutor(workers(), initializer=_init, initargs=((set(by_rgb), set(by_rgba), sizes),)) as pool:
        for bi, (bundle, (matches, note)) in enumerate(zip(bundles, pool.map(bundle_matches, bundles))):
            if note:
                print(note, flush=True)
            for fmt, key, blob in matches:
                targets = by_rgb.get(key, []) if fmt == DXT1 else by_rgba.get(key, [])
                for p in [p for p in targets if p not in done]:
                    p.with_suffix(".bct").write_bytes(blob)
                    done.add(p)
                    written += 1
            print(f"[{bi + 1}/{len(bundles)}] {bundle.name[:60]}: {written} written", flush=True)
    print(f"done: {written} compressed textures written; {n - written} PNGs without a DXT match (kept as PNG only)")


if __name__ == "__main__":
    sys.exit(main())
