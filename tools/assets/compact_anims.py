"""Shrink baked model animations (anim.bin -> anim_q.bin) in place.

export_animated.py bakes every Animator clip to f32 positions + normals per vertex per frame.
Every clip is kept (idle, attack, placement, idle variants, upgrade...) and quantized:
  position: 3 x u16, normalized to the model's animated bounding box (meta "quant")
  normal:   3 x i8 (* 127), plus one pad byte
10 bytes per vertex per frame instead of 24. The renderer decodes back to f32 at load.

Usage: python -I tools/assets/compact_anims.py <models dir>   (recurses; idempotent)
"""
import json
import os
import sys

import numpy as np


def pick(anims):
    """Indices of the clips to keep: all of them (idle, attack, the controller's placement,
    idle-variant and upgrade clips)."""
    return list(range(len(anims)))


def convert(d):
    meta_p, anim_p = os.path.join(d, "meta.json"), os.path.join(d, "anim.bin")
    meta = json.load(open(meta_p))
    n = meta.get("vertex_count", 0)
    anims = meta.get("anims", [])
    if not n or not anims or not os.path.exists(anim_p):
        return 0, 0
    data = np.fromfile(anim_p, dtype=np.float32).reshape(-1, n, 6)
    keep = pick(anims)
    frames, new_anims, off = [], [], 0
    for i in keep:
        a = dict(anims[i])
        frames.append(data[a["offset"]:a["offset"] + a["frames"]])
        a["offset"] = off
        off += a["frames"]
        new_anims.append(a)
    f = np.concatenate(frames)
    pos, nor = f[..., :3], f[..., 3:]
    lo, hi = pos.reshape(-1, 3).min(0), pos.reshape(-1, 3).max(0)
    span = np.where(hi - lo > 1e-9, hi - lo, 1.0)
    q = np.round((pos - lo) / span * 65535.0).clip(0, 65535).astype(np.uint16)
    nq = np.round(nor.clip(-1, 1) * 127.0).astype(np.int8)
    out = np.zeros(f.shape[:2], dtype=[("p", "<u2", 3), ("n", "i1", 3), ("pad", "i1")])
    out["p"], out["n"] = q, nq
    out.tofile(os.path.join(d, "anim_q.bin"))
    meta["anims"] = new_anims
    meta["quant"] = {"min": lo.tolist(), "max": hi.tolist()}
    json.dump(meta, open(meta_p, "w"), indent=1)
    before = os.path.getsize(anim_p)
    os.remove(anim_p)
    return before, os.path.getsize(os.path.join(d, "anim_q.bin"))


def main():
    total_b = total_a = count = 0
    for dirpath, _, files in os.walk(sys.argv[1]):
        if "anim.bin" in files and "meta.json" in files:
            b, a = convert(dirpath)
            total_b += b
            total_a += a
            count += 1
    print(f"{count} models: {total_b / 1e9:.2f} GB -> {total_a / 1e9:.2f} GB")


if __name__ == "__main__":
    main()
