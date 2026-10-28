"""Store extracted PCM16 audio as verified lossless FLAC, without resampling.

The renderer decodes once into the same mixer samples. Original BTD6 files are
never touched. --keep-original is useful for decoder equivalence benchmarks.
"""
from concurrent.futures import ThreadPoolExecutor
import argparse
import hashlib
import io
import os
from pathlib import Path
import tempfile
import time

import soundfile as sf

BLOCK = 65536


def encode(source, destination):
    """Atomically write FLAC and verify every decoded sample before replacing."""
    destination = Path(destination)
    if destination.is_symlink():
        raise ValueError("refusing linked audio output")
    pending = None
    try:
        digest = hashlib.blake2b()
        with sf.SoundFile(source) as src:
            if src.subtype != "PCM_16" or src.channels not in (1, 2):
                raise ValueError("lossless audio conversion requires mono/stereo PCM16")
            rate, channels, count = src.samplerate, src.channels, src.frames
            with tempfile.NamedTemporaryFile(dir=destination.parent, prefix=destination.name + ".", suffix=".pending", delete=False) as temp:
                pending = Path(temp.name)
            with sf.SoundFile(str(pending), mode="w", samplerate=rate, channels=channels,
                              format="FLAC", subtype="PCM_16") as out:
                for block in src.blocks(blocksize=BLOCK, dtype="int16", always_2d=True):
                    digest.update(block.astype("<i2", copy=False).tobytes())
                    out.write(block)
        restored = hashlib.blake2b()
        with sf.SoundFile(str(pending)) as check:
            if (check.samplerate, check.channels, check.frames, check.subtype) != (rate, channels, count, "PCM_16"):
                raise ValueError("FLAC audio metadata differs from source")
            for block in check.blocks(blocksize=BLOCK, dtype="int16", always_2d=True):
                restored.update(block.astype("<i2", copy=False).tobytes())
        if restored.digest() != digest.digest():
            raise ValueError("FLAC sample verification failed")
        with pending.open("r+b") as f:
            os.fsync(f.fileno())
        size = pending.stat().st_size
        os.replace(pending, destination)
        pending = None
        return size
    finally:
        if pending is not None:
            pending.unlink(missing_ok=True)


def encode_bytes(wav_bytes, destination):
    return encode(io.BytesIO(wav_bytes), destination)


def compress(path, keep_original=False):
    path = Path(path)
    if path.is_symlink():
        raise ValueError("refusing linked source audio")
    stat = path.stat()
    before = stat.st_size, stat.st_mtime_ns
    size = encode(str(path), path.with_suffix(".flac"))
    stat = path.stat()
    if (stat.st_size, stat.st_mtime_ns) != before:
        raise RuntimeError("audio source changed while encoding")
    if not keep_original:
        path.unlink()
    return before[0], size


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("audio", type=Path)
    parser.add_argument("--keep-original", action="store_true")
    args = parser.parse_args()
    if args.audio.is_symlink() or os.path.isjunction(args.audio):
        parser.error("refusing linked audio directory")
    paths = sorted(args.audio.resolve(strict=True).glob("*.wav"))
    before = after = 0
    start = time.perf_counter()
    with ThreadPoolExecutor(max_workers=min(4, os.cpu_count() or 1)) as pool:
        for index, (b, a) in enumerate(pool.map(lambda p: compress(p, args.keep_original), paths), 1):
            before += b
            after += a
            if index % 100 == 0 or index == len(paths):
                print(f"audio {index}/{len(paths)}: {before/1e9:.3f} -> {after/1e9:.3f} GB ({time.perf_counter()-start:.1f}s)", flush=True)
    print(f"saved {(before-after)/1e9:.3f} GB; all decoded PCM samples verified", flush=True)


if __name__ == "__main__":
    main()
