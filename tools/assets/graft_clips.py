"""Add named animation clips from one exported model to another with the same vertex order.

The tower tiers draw the game's "Generated" prefabs (flat-skin bakes that only carry the idle
loop); the original prefabs (export_animated.py output, anim.bin) still have every Animator clip
(the Sun Temple opening, True Sun God rising...). Each grafted frame is the clip's motion added to
the target's own idle pose:  target_idle0 + (source_frame - source_idle0),
exact when the meshes match and close when the target is a reshaped variant (SunTempleBase).

With --match the target is the source re-baked (the Sun Temple head: turned 180 degrees about y,
scaled, its vertices reordered): that similarity transform and vertex order are found from the
idle poses, and each source frame is carried through it whole.

Usage: python -I tools/assets/graft_clips.py [--match] <target model dir (anim_q.bin)> <source dir (anim.bin)> <clip>...
Idempotent: a clip already in the target is replaced.
"""
import json
import os
import sys

from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from pack_anims import read_bytes

import numpy as np

QT = [("p", "<u2", 3), ("n", "i1", 3), ("pad", "i1")]


def load_target(d):
    meta = json.load(open(os.path.join(d, "meta.json")))
    n = meta["vertex_count"]
    q = np.frombuffer(read_bytes(Path(d) / "anim_q.bin"), dtype=QT).reshape(-1, n)
    lo, hi = np.array(meta["quant"]["min"]), np.array(meta["quant"]["max"])
    span = np.where(hi - lo > 1e-9, hi - lo, 1.0)
    pos = lo + q["p"].astype(np.float64) / 65535.0 * span
    nor = q["n"].astype(np.float64) / 127.0
    return meta, np.concatenate([pos, nor], axis=-1)


def idle_index(anims):
    return next((i for i, a in enumerate(anims) if a["name"].endswith("Idle")), 0)


def similarity(gi, si):
    """(order, R, k, g_mean, s_mean) with gi[i] ~= R k (si[order[i]] - s_mean) + g_mean, trying a
    turn of 0 or 180 degrees about y; k from the bounding boxes."""
    k = float(np.ptp(gi, 0).mean() / max(np.ptp(si, 0).mean(), 1e-9))
    a, b = gi - gi.mean(0), (si - si.mean(0)) * k
    best = None
    for R in (np.eye(3), np.diag([-1.0, 1.0, -1.0])):
        d = np.linalg.norm((a @ R)[:, None] - b[None], axis=2)  # R is its own inverse
        err = d.min(1).max()
        if best is None or err < best[0]:
            best = (err, d.argmin(1), R)
    err, order, R = best
    # Seam duplicates (same position) may map to one source vertex: they move together.
    if err > 0.05:
        sys.exit(f"no vertex match (max error {err:.3f})")
    return order, R, k, gi.mean(0), si.mean(0)


def main():
    args = sys.argv[1:]
    match = "--match" in args
    args = [a for a in args if a != "--match"]
    tgt, src, names = args[0], args[1], args[2:]
    meta, frames = load_target(tgt)
    smeta = json.load(open(os.path.join(src, "meta.json")))
    n = meta["vertex_count"]
    if smeta["vertex_count"] != n:
        sys.exit(f"vertex counts differ: {n} vs {smeta['vertex_count']}")
    sdata = np.frombuffer(read_bytes(Path(src) / "anim.bin"), dtype=np.float32).reshape(-1, n, 6).astype(np.float64)
    t_idle = meta["anims"][idle_index(meta["anims"])]
    s_idle = smeta["anims"][idle_index(smeta["anims"])]
    base_t, base_s = frames[t_idle["offset"]], sdata[s_idle["offset"]]
    # Drop clips being replaced, keeping the others' frames.
    keep = [a for a in meta["anims"] if a["name"] not in names]
    parts, anims, off = [], [], 0
    for a in keep:
        parts.append(frames[a["offset"]:a["offset"] + a["frames"]])
        anims.append(dict(a, offset=off))
        off += a["frames"]
    for name in names:
        a = next((x for x in smeta["anims"] if x["name"] == name), None)
        if a is None:
            sys.exit(f"{src} has no clip {name}")
        f = sdata[a["offset"]:a["offset"] + a["frames"]].copy()
        if match:
            order, R, k, gm, sm = similarity(base_t[:, :3], base_s[:, :3])
            f = f[:, order]
            f[..., :3] = (f[..., :3] - sm) * k @ R.T + gm
            f[..., 3:] = f[..., 3:] @ R.T
        else:
            f[..., :3] = base_t[:, :3] + (f[..., :3] - base_s[:, :3])
        parts.append(f)
        anims.append(dict(a, offset=off))
        off += a["frames"]
    f = np.concatenate(parts)
    pos, nor = f[..., :3], f[..., 3:]
    lo, hi = pos.reshape(-1, 3).min(0), pos.reshape(-1, 3).max(0)
    span = np.where(hi - lo > 1e-9, hi - lo, 1.0)
    out = np.zeros(f.shape[:2], dtype=QT)
    out["p"] = np.round((pos - lo) / span * 65535.0).clip(0, 65535).astype(np.uint16)
    out["n"] = np.round(nor.clip(-1, 1) * 127.0).astype(np.int8)
    out.tofile(os.path.join(tgt, "anim_q.bin"))
    meta["anims"] = anims
    meta["quant"] = {"min": lo.tolist(), "max": hi.tolist()}
    json.dump(meta, open(os.path.join(tgt, "meta.json"), "w"), indent=1)
    print(tgt, [(a["name"], a["frames"]) for a in anims])


if __name__ == "__main__":
    main()
