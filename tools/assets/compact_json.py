"""Remove only JSON whitespace outside strings; preserve every value token exactly.

Run after exporters finish. Files remain ordinary JSON with no runtime decoder.
Only runtime asset folders are visited, never settings, saves or account caches.
"""
import argparse
import json
import os
from pathlib import Path
import re
import stat
import tempfile

FOLDERS = ("models", "ui", "effects", "maps", "game", "text", "bloon_displays", "screens")
# Match whole strings first: whitespace inside them (including escaped quotes) stays intact.
TOKENS = re.compile(rb'"[^"\\]*(?:\\.[^"\\]*)*"|[ \t\r\n]+')


def compact_bytes(raw):
    value = json.loads(raw)
    result = TOKENS.sub(lambda m: m[0] if m[0].startswith(b'"') else b"", raw)
    if json.loads(result) != value:
        raise ValueError("JSON values changed while compacting")
    return result


def signature(path):
    s = path.stat()
    return s.st_size, s.st_mtime_ns, s.st_ino


def compact(path):
    path = Path(path)
    if path.is_symlink() or os.path.isjunction(path) or not path.is_file():
        raise ValueError("expected an unlinked JSON file")
    stamp = signature(path)
    raw = path.read_bytes()
    result = compact_bytes(raw)
    if len(result) >= len(raw):
        return len(raw), len(raw)
    pending = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=path.name + ".", suffix=".pending", delete=False) as out:
            pending = Path(out.name)
            out.write(result)
            out.flush()
            os.fsync(out.fileno())
        if pending.read_bytes() != result:
            raise ValueError("compacted JSON write verification failed")
        if path.is_symlink() or signature(path) != stamp or path.read_bytes() != raw:
            raise RuntimeError("JSON export changed during compaction")
        os.chmod(pending, stat.S_IMODE(path.stat().st_mode))
        # Atomic replacement also leaves any other hard-link names untouched.
        os.replace(pending, path)
        pending = None
        return len(raw), len(result)
    finally:
        if pending is not None:
            pending.unlink(missing_ok=True)


def files(root):
    root = Path(root)
    if root.is_symlink() or os.path.isjunction(root):
        raise ValueError("refusing linked asset root")
    root = root.resolve(strict=True)
    for folder in FOLDERS:
        base = root / folder
        if not base.is_dir() or base.is_symlink() or os.path.isjunction(base):
            continue
        for directory, dirs, names in os.walk(base, followlinks=False):
            dirs[:] = [n for n in dirs if not Path(directory, n).is_symlink() and not os.path.isjunction(Path(directory, n))]
            for name in sorted(names):
                path = Path(directory, name)
                if path.suffix == ".json" and not path.is_symlink() and path.resolve().is_relative_to(root):
                    yield path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("assets", type=Path)
    args = parser.parse_args()
    before = after = changed = count = 0
    # Files are independent: spread over processes (parsing is CPU-bound; one big JSON peaked at
    # ~2.2 GB, so workers are sized by free memory).
    import sys
    from concurrent.futures import ProcessPoolExecutor
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from model_batch import worker_count
    paths = list(files(args.assets))
    workers = max(1, min(worker_count(per_worker=1 << 30, max_workers=12), int(os.environ.get("RRBTD6_SETUP_WORKERS", "12"))))
    with ProcessPoolExecutor(workers) as pool:
        results = list(pool.map(compact, paths, chunksize=64))
    for old, new in results:
        before += old
        after += new
        changed += old != new
        count += 1
    print(json.dumps(dict(files=count, changed=changed, before=before, after=after, saved_bytes=before-after)))


if __name__ == "__main__":
    main()
