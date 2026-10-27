"""Losslessly pack baked animations. Requires a client supporting *.bin.z.

All frames, normals and metadata stay byte-identical. Zlib level 1 keeps setup
fast; the runtime expands the file once in its existing model staging worker.
Use --keep-original for developer benchmarking/old-client compatibility.
"""
from concurrent.futures import ThreadPoolExecutor
import argparse
import hashlib
import os
from pathlib import Path
import tempfile
import time
import zlib

CHUNK = 1024 * 1024


def read_bytes(path):
    path = Path(path)
    if path.exists():
        return path.read_bytes()
    return zlib.decompress(path.with_name(path.name + ".z").read_bytes())


def signature(path):
    s = path.stat()
    return s.st_size, s.st_mtime_ns, s.st_ino


def verify(path, expected_size, expected_hash):
    decoder = zlib.decompressobj()
    digest = hashlib.blake2b()
    size = 0
    with Path(path).open("rb") as f:
        while data := f.read(CHUNK):
            if decoder.eof:
                raise ValueError("trailing packed animation data")
            while data and not decoder.eof:
                decoded = decoder.decompress(data, CHUNK)
                data = decoder.unconsumed_tail
                digest.update(decoded)
                size += len(decoded)
                if size > expected_size:
                    raise ValueError("animation expands beyond source size")
    if not decoder.eof or decoder.unused_data or size != expected_size or digest.digest() != expected_hash:
        raise ValueError("packed animation did not verify")


def pack(path, keep_original=False):
    path = Path(path)
    target = path.with_name(path.name + ".z")
    if path.is_symlink() or target.is_symlink():
        raise ValueError("refusing linked animation")
    before = signature(path)
    pending = None
    try:
        digest = hashlib.blake2b()
        encoder = zlib.compressobj(1)
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=target.name + ".", suffix=".pending", delete=False) as out:
            pending = Path(out.name)
            with path.open("rb") as src:
                while chunk := src.read(CHUNK):
                    digest.update(chunk)
                    out.write(encoder.compress(chunk))
            out.write(encoder.flush())
            out.flush()
            os.fsync(out.fileno())
        after_size = pending.stat().st_size
        if after_size >= before[0]:
            return before[0], before[0]
        verify(pending, before[0], digest.digest())
        if signature(path) != before:
            raise RuntimeError(f"animation changed while packing: {path}")
        os.replace(pending, target)
        pending = None
        # Keep a concurrent re-export instead of removing it.
        if not keep_original and signature(path) == before:
            path.unlink()
        return before[0], after_size
    finally:
        if pending is not None:
            pending.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("models", type=Path)
    parser.add_argument("--keep-original", action="store_true")
    parser.add_argument("--workers", type=int, default=min(4, os.cpu_count() or 1))
    args = parser.parse_args()
    root = args.models.resolve(strict=True)
    if args.workers < 1 or args.workers > 16:
        parser.error("workers must be between 1 and 16")
    if args.models.is_symlink() or os.path.isjunction(args.models):
        parser.error("refusing linked models directory")
    paths = []
    for directory, dirs, names in os.walk(root, followlinks=False):
        dirs[:] = [name for name in dirs if not Path(directory, name).is_symlink() and not os.path.isjunction(Path(directory, name))]
        for name in ("anim.bin", "anim_q.bin"):
            if name in names:
                path = Path(directory, name)
                if root not in path.resolve().parents:
                    raise ValueError("animation outside model root")
                paths.append(path)
    start = time.perf_counter()
    before = after = 0
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        for index, (b, a) in enumerate(pool.map(lambda p: pack(p, args.keep_original), sorted(paths)), 1):
            before += b
            after += a
            if index % 50 == 0 or index == len(paths):
                print(f"packed {index}/{len(paths)}: {before/1e9:.3f} -> {after/1e9:.3f} GB ({time.perf_counter()-start:.1f}s)", flush=True)
    print(f"saved {(before-after)/1e9:.3f} GB; every packed payload verified byte-for-byte", flush=True)


if __name__ == "__main__":
    main()
