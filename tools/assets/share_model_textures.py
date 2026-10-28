"""Share byte-identical model textures through existing relative metadata paths.

No image decoding/re-encoding and no new runtime file format. Run at extraction
finish, after exporters have completed. Interrupted runs retain source textures
until every updated metadata file and shared copy has been verified.
"""
import argparse
from collections import defaultdict
import hashlib
import json
import os
from pathlib import Path
import tempfile

SHARED = "shared_textures"


def signature(path):
    stat = path.stat()
    return stat.st_size, stat.st_mtime_ns, stat.st_ino


def digest(path):
    with path.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def slots(meta):
    """Only fields read as texture paths by Model/AnimModel; never names/text."""
    for part in meta.get("parts", []):
        if isinstance(part.get("texture"), str):
            yield part, "texture"
        for texture in part.get("props", {}).get("textures", {}).values():
            if isinstance(texture, dict) and isinstance(texture.get("file"), str):
                yield texture, "file"


def texture_path(root, directory, name):
    path = directory / name
    if path.is_symlink():
        raise ValueError("refusing linked texture")
    resolved = path.resolve()
    if not resolved.is_relative_to(root):
        raise ValueError("texture path leaves model directory")
    return resolved


def atomic_write(path, data):
    if path.is_symlink():
        raise ValueError("refusing linked output")
    pending = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=path.name + ".", suffix=".pending", delete=False) as f:
            pending = Path(f.name)
            f.write(data)
            f.flush()
            os.fsync(f.fileno())
        os.replace(pending, path)
        pending = None
    finally:
        if pending is not None:
            pending.unlink(missing_ok=True)


def compact(models):
    models = Path(models)
    if models.is_symlink() or os.path.isjunction(models):
        raise ValueError("refusing linked model directory")
    root = models.resolve(strict=True)
    shared = root / SHARED
    if shared.is_symlink() or os.path.isjunction(shared):
        raise ValueError("refusing linked shared directory")
    docs, refs = {}, defaultdict(list)
    for meta_path in sorted(root.rglob("meta.json")):
        if meta_path.is_symlink():
            raise ValueError("refusing linked model metadata")
        stamp = signature(meta_path)
        meta = json.loads(meta_path.read_bytes())
        docs[meta_path] = (meta, stamp)
        for obj, key in slots(meta):
            path = texture_path(root, meta_path.parent, obj[key])
            # Shadows and other implicit filenames remain untouched. Missing
            # pre-existing references are left alone, never invented/repaired.
            if path.suffix.lower() == ".png" and path.parent.name in ("tex", SHARED) and path.is_file():
                refs[path].append((meta_path, obj, key))
    groups, stamps = defaultdict(list), {}
    for path in refs:
        stamps[path] = signature(path)
        groups[(stamps[path][0], digest(path))].append(path)

    changed, removable, created = set(), set(), 0
    saved = 0
    for (size, sha), paths in groups.items():
        if len(paths) < 2:
            continue
        content = paths[0].read_bytes()
        for path in paths:
            if signature(path) != stamps[path] or path.read_bytes() != content:
                raise RuntimeError("texture source changed or digest collision")
        shared.mkdir(exist_ok=True)
        target = shared / (sha + ".png")
        if target.exists():
            if target.is_symlink() or target.read_bytes() != content:
                raise ValueError("shared texture does not match its content hash")
        else:
            atomic_write(target, content)
            created += size
        if target.read_bytes() != content:
            raise ValueError("shared texture verification failed")
        for path in paths:
            for meta_path, obj, key in refs[path]:
                relative = os.path.relpath(target, meta_path.parent).replace(os.sep, "/")
                if obj[key] != relative:
                    obj[key] = relative
                    changed.add(meta_path)
            if path != target:
                removable.add(path)

    # Commit metadata before deleting any originals. A failed/interrupted run
    # leaves both copies readable; it never leaves references without data.
    for path in sorted(changed):
        meta, stamp = docs[path]
        if signature(path) != stamp:
            raise RuntimeError("model metadata changed while compacting")
        encoded = json.dumps(meta, indent=1, ensure_ascii=True).encode()
        atomic_write(path, encoded)
        if json.loads(path.read_bytes()) != meta:
            raise ValueError("model metadata verification failed")

    # Re-read current metadata so any still-referenced original is retained.
    live = set()
    for path in root.rglob("meta.json"):
        for obj, key in slots(json.loads(path.read_bytes())):
            live.add(texture_path(root, path.parent, obj[key]))
    for path in sorted(removable):
        if path in live:
            continue
        if signature(path) != stamps[path]:
            raise RuntimeError("texture source changed before removal")
        size = path.stat().st_size
        path.unlink()
        saved += size
    return {"metadata_updated": len(changed), "original_bytes_removed": saved,
            "shared_bytes_created": created, "net_bytes_saved": saved-created}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("models", type=Path)
    print(json.dumps(compact(parser.parse_args().models)), flush=True)
