"""Animated models stored as skinning data (skin.bin) instead of baked vertex frames.

export_animated.py used to write every vertex's position and normal for every frame
(anim.bin), which the finish passes then quantized and compressed (anim_q.bin.wz). Every
animated vertex is a weighted sum of a few matrices (bones, or the transform of a rigid mesh),
so skin.bin stores, once, each vertex's rest position, normal, slot indices and weights, and
per frame each slot's 3x4 matrix. The runtime (renderer/src/animation_storage.rs) rebuilds the
same 10-byte quantized frames as anim_q.bin while staging a model:

    position = sum_k w_k (M_k [v, 1]),  normal = normalize(sum_k w_k (M_k [n, 0]))

in f32, quantized with meta.json "quant" exactly as compact_anims.py does. Against the old f64
bake that is at most one quantization step (1/65535 of the model's bounding box) on under 0.1%
of values; Unity skins in f32 as well. About 5x smaller than anim_q.bin.wz.

skin.bin: "RRSKIN01", u32 vertices, slots, frames, influences K, u64 body length, then one
Zstandard frame (with checksum) of: positions (vertices x 3 f32), normals (vertices x 3 f32),
slot indices (vertices x K u16), weights (vertices x K f32), matrices (frames x slots x 12 f32,
row-major 3x4; each frame's bytes stored minus the previous frame's, wrapping).
"""
import json
import os
import struct
from pathlib import Path

import numpy as np

MAGIC = b"RRSKIN01"
HEADER = struct.Struct("<8sIIIIQ")


def write(out, v, n, idx, w, mats):
    """skin.bin in `out`. v, n: (vertices, 3); idx, w: (vertices, K); mats: (frames, slots, 3, 4)."""
    import zstandard
    v, n = np.ascontiguousarray(v, "<f4"), np.ascontiguousarray(n, "<f4")
    idx, w = np.ascontiguousarray(idx, "<u2"), np.ascontiguousarray(w, "<f4")
    mats = np.ascontiguousarray(mats, "<f4")
    frames, slots = mats.shape[:2]
    rows = mats.view(np.uint8).reshape(frames, -1)
    delta = rows.copy()
    delta[1:] -= rows[:-1]  # uint8: wrapping
    body = b"".join([v.tobytes(), n.tobytes(), idx.tobytes(), w.tobytes(), delta.tobytes()])
    # Level 19 is only about 7% smaller (28 MB for the whole game) and 19x slower.
    packed = zstandard.ZstdCompressor(level=9, write_checksum=True).compress(body)
    path = Path(out) / "skin.bin"
    pending = path.with_name(path.name + ".pending")
    pending.write_bytes(HEADER.pack(MAGIC, len(v), slots, frames, idx.shape[1], len(body)) + packed)
    os.replace(pending, path)


def read(folder):
    """(v, n, idx, w, mats) of a model's skin.bin."""
    import zstandard
    data = (Path(folder) / "skin.bin").read_bytes()
    magic, nv, slots, frames, k, size = HEADER.unpack_from(data)
    if magic != MAGIC:
        raise ValueError("not a skin.bin")
    body = zstandard.ZstdDecompressor().decompress(data[HEADER.size:], max_output_size=size)
    if len(body) != size:
        raise ValueError("skin.bin body length")
    at = 0

    def take(dtype, count):
        nonlocal at
        a = np.frombuffer(body, dtype=dtype, count=count, offset=at)
        at += a.nbytes
        return a
    v, n = take("<f4", nv * 3).reshape(nv, 3), take("<f4", nv * 3).reshape(nv, 3)
    idx, w = take("<u2", nv * k).reshape(nv, k), take("<f4", nv * k).reshape(nv, k)
    rows = take(np.uint8, frames * slots * 48).reshape(frames, -1)
    mats = np.cumsum(rows, axis=0, dtype=np.uint8).view("<f4").reshape(frames, slots, 3, 4)
    if at != size or (idx >= slots).any():
        raise ValueError("skin.bin layout")
    return v, n, idx, w, mats


def poses(v, n, idx, w, mats, chunk=64, dtype=np.float32, normals=True):
    """(frame index, positions, unit normals or None) of every frame, in f32 as the runtime
    rebuilds them (or `dtype`, for a reference), computed for `chunk` frames at a time."""
    one = dtype(1e-9)
    v, n, w = v.astype(dtype), n.astype(dtype), w.astype(dtype)
    # Only the influences that carry weight (most vertices use one or two of the K slots).
    used = [(k, np.flatnonzero(w[:, k])) for k in range(idx.shape[1])]
    for start in range(0, len(mats), chunk):
        m = mats[start:start + chunk].astype(dtype)
        p = np.zeros((len(m), len(v), 3), dtype)
        q = np.zeros((len(m), len(v), 3), dtype) if normals else None
        for k, rows in used:
            if not len(rows):
                continue
            M = m[:, idx[rows, k]]  # frames, vertices, 3, 4
            wk = w[rows, k, None]
            p[:, rows] += wk * (np.einsum("fnij,nj->fni", M[..., :3], v[rows]) + M[..., 3])
            if normals:
                q[:, rows] += wk * np.einsum("fnij,nj->fni", M[..., :3], n[rows])
        if normals:
            length = np.sqrt((q * q).sum(-1, keepdims=True, dtype=dtype))
            q /= np.maximum(length, one)
        for f in range(len(m)):
            yield start + f, p[f], q[f] if normals else None


def check(v, n, idx, w, mats, exact, every=8):
    """meta.json "quant" bounds of the f32 rebuild, and its largest position difference, in
    steps of the 1/65535 grid, from one in f64 with the unrounded matrices `exact` (what the
    vertex-frame bake computed). The difference comes from the size of a model's matrices, not
    single frames (under 0.05 steps on every game model but one, at 5.7), so a sample of the
    frames decides it."""
    lo, hi = bounds(v, n, idx, w, mats)
    error = np.zeros(3)
    sampled = zip(poses(v, n, idx, w, mats[::every], normals=False), poses(v, n, idx, w, exact[::every], dtype=np.float64, normals=False))
    for (_, p, _), (_, e, _) in sampled:
        error = np.maximum(error, np.abs(p - e).max(0))
    span = np.where(hi - lo > 1e-9, hi - lo, 1.0)
    return lo, hi, float((error / span).max() * 65535.0) if len(mats) else 0.0


def bounds(v, n, idx, w, mats):
    """meta.json "quant" {min, max}: the animated positions' bounding box."""
    lo, hi = np.full(3, np.inf, np.float32), np.full(3, -np.inf, np.float32)
    for _, p, _ in poses(v, n, idx, w, mats, normals=False):
        lo, hi = np.minimum(lo, p.min(0)), np.maximum(hi, p.max(0))
    return lo, hi


def float_frames(folder):
    """(frames, vertices, 6) f32 positions + normals, as anim.bin held them."""
    v, n, idx, w, mats = read(folder)
    out = np.empty((len(mats), len(v), 6), np.float32)
    for f, p, q in poses(v, n, idx, w, mats):
        out[f, :, :3], out[f, :, 3:] = p, q
    return out


def quantized(folder):
    """anim_q.bin bytes rebuilt from skin.bin with meta.json "quant" (what the runtime does)."""
    meta = json.loads((Path(folder) / "meta.json").read_text(encoding="utf-8"))
    lo, hi = np.array(meta["quant"]["min"], np.float32), np.array(meta["quant"]["max"], np.float32)
    span = np.where(hi - lo > 1e-9, hi - lo, np.float32(1)).astype(np.float32)
    v, n, idx, w, mats = read(folder)
    out = np.zeros((len(mats), len(v)), dtype=[("p", "<u2", 3), ("n", "i1", 3), ("pad", "i1")])
    for f, p, q in poses(v, n, idx, w, mats):
        out["p"][f] = np.round((p - lo) / span * np.float32(65535)).clip(0, 65535).astype(np.uint16)
        out["n"][f] = np.round(q.clip(-1, 1) * np.float32(127)).astype(np.int8)
    return out.tobytes()
