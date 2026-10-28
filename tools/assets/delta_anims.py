"""Lossless temporal prediction for already packed baked animation files.

Each byte stores its difference from the same byte in the previous frame,
modulo 256. Zlib compresses those differences. Restoring them is exact and
happens once during staging; frame sampling has no new decoding work.
"""
from concurrent.futures import ThreadPoolExecutor
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import tempfile
import time
import zlib

import numpy as np

MAGIC = b"RRDIF001"
CHUNK = 1024 * 1024


def chunks(path, skip=0):
    """Bounded-memory, checksum-checked reading of a zlib file."""
    decoder = zlib.decompressobj()
    with Path(path).open("rb") as f:
        f.seek(skip)
        while data := f.read(CHUNK):
            if decoder.eof:
                raise ValueError("trailing animation data")
            while data and not decoder.eof:
                out = decoder.decompress(data, CHUNK)
                data = decoder.unconsumed_tail
                if out:
                    yield out
    if not decoder.eof or decoder.unused_data:
        raise ValueError("incomplete animation stream")


def frames(source, stride):
    pending = bytearray()
    for data in source:
        pending.extend(data)
        end = len(pending) // stride * stride
        for offset in range(0, end, stride):
            yield bytes(pending[offset:offset + stride])
        del pending[:end]
    if pending:
        raise ValueError("partial animation frame")


def restored_frames(path):
    with Path(path).open("rb") as f:
        header = f.read(16)
    if len(header) != 16 or header[:8] != MAGIC:
        raise ValueError("invalid delta animation header")
    stride, = struct.unpack("<Q", header[8:])
    if not 0 < stride <= 64 * 1024 * 1024:
        raise ValueError("invalid animation frame stride")
    previous = np.zeros(stride, dtype=np.uint8)
    for frame in frames(chunks(path, 16), stride):
        previous = np.frombuffer(frame, dtype=np.uint8) + previous
        yield previous.tobytes()


def read_bytes(path):
    return b"".join(restored_frames(path))


def signature(path):
    s = Path(path).stat()
    return s.st_size, s.st_mtime_ns, s.st_ino


def pack(path, keep_original=False):
    path = Path(path)
    if path.name not in ("anim.bin.z", "anim_q.bin.z") or path.is_symlink():
        raise ValueError("expected a packed animation")
    target = path.with_suffix(".dz")
    if target.is_symlink():
        raise ValueError("refusing linked output")
    meta_path = path.with_name("meta.json")
    meta_signature = signature(meta_path)
    meta = json.loads(meta_path.read_bytes())
    stride = meta["vertex_count"] * (10 if path.name == "anim_q.bin.z" else 24)
    count = max(a["offset"] + a["frames"] for a in meta["anims"])
    if stride <= 0 or stride > 64 * 1024 * 1024 or count <= 1:
        return path.stat().st_size, path.stat().st_size
    expected = stride * count
    before = signature(path)
    pending = None
    try:
        digest = hashlib.blake2b()
        seen = 0
        encoder = zlib.compressobj(1)
        previous = np.zeros(stride, dtype=np.uint8)
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=target.name + ".", suffix=".pending", delete=False) as out:
            pending = Path(out.name)
            out.write(MAGIC + struct.pack("<Q", stride))
            for frame in frames(chunks(path), stride):
                digest.update(frame)
                seen += len(frame)
                if seen > expected:
                    raise ValueError("animation exceeds metadata length")
                current = np.frombuffer(frame, dtype=np.uint8)
                out.write(encoder.compress((current - previous).tobytes()))
                previous = current
            out.write(encoder.flush())
            out.flush()
            os.fsync(out.fileno())
        if seen != expected:
            raise ValueError("animation length differs from metadata")
        after = pending.stat().st_size
        if after >= before[0]:
            return before[0], before[0]
        restored_hash = hashlib.blake2b()
        restored_size = 0
        for frame in restored_frames(pending):
            restored_hash.update(frame)
            restored_size += len(frame)
        if restored_size != expected or restored_hash.digest() != digest.digest():
            raise ValueError("delta animation verification failed")
        if signature(path) != before or signature(meta_path) != meta_signature:
            raise RuntimeError("animation export changed while packing")
        os.replace(pending, target)
        pending = None
        if not keep_original and signature(path) == before:
            path.unlink()
        return before[0], after
    finally:
        if pending is not None:
            pending.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("models", type=Path)
    parser.add_argument("--keep-original", action="store_true")
    parser.add_argument("--workers", type=int, default=2)
    args = parser.parse_args()
    if not 1 <= args.workers <= 16:
        parser.error("workers must be between 1 and 16")
    if args.models.is_symlink() or os.path.isjunction(args.models):
        parser.error("refusing linked model root")
    root = args.models.resolve(strict=True)
    paths = []
    for directory, dirs, names in os.walk(root, followlinks=False):
        dirs[:] = [n for n in dirs if not Path(directory, n).is_symlink() and not os.path.isjunction(Path(directory, n))]
        for name in ("anim.bin.z", "anim_q.bin.z"):
            if name in names:
                path = Path(directory, name)
                if root not in path.resolve().parents:
                    raise ValueError("animation outside model root")
                paths.append(path)
    before = after = 0
    start = time.perf_counter()
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        for index, (b, a) in enumerate(pool.map(lambda p: pack(p, args.keep_original), sorted(paths)), 1):
            before += b
            after += a
            if index % 50 == 0 or index == len(paths):
                print(f"predicted {index}/{len(paths)}: {before/1e9:.3f} -> {after/1e9:.3f} GB ({time.perf_counter()-start:.1f}s)", flush=True)
    print(f"saved {(before-after)/1e9:.3f} GB; every changed payload verified", flush=True)


if __name__ == "__main__":
    main()
