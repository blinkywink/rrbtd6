"""Store byte-identical runtime asset files once, as hard links to a single copy.

Usage:
    tools/.venv/Scripts/python.exe -I tools/assets/link_duplicates.py <local_extracted> [folder ...]
    tools/.venv/Scripts/python.exe -I tools/assets/link_duplicates.py --split <local_extracted> [folder ...]

--split turns every linked file back into its own copy. The extraction driver runs it before
any exporter, since exporters overwrite files in place, which would change every linked copy.

Paths and contents stay exactly the same, so nothing that reads the assets changes; only the
disk stores each distinct file once (e.g. a tower's animation repeated across upgrade tiers, the
same shadow.png in hundreds of model folders). Files are grouped by size, then BLAKE2 hash, then
compared byte for byte before linking. Each duplicate is replaced atomically (a hard link made
next to it, then os.replace). Where hard links aren't supported (another volume, exFAT/FAT) the
copy is simply kept. Writers elsewhere replace files with os.replace, which breaks a link
instead of changing the shared copy. Repeat runs change nothing.
"""
import collections
import filecmp
import hashlib
import os
import sys

FOLDERS = ["models", "ui", "effects", "sprites", "audio", "maps", "bloon_displays", "screens"]
MIN_SIZE = 4096
# Binary assets only: metadata (.json) is rewritten in place by later tools.
EXTENSIONS = {".png", ".dz", ".z", ".wz", ".bin", ".flac", ".wav", ".ogg"}


def digest(path):
    h = hashlib.blake2b(digest_size=20)
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.digest()


def files(root, folders):
    for folder in folders:
        base = os.path.join(root, folder)
        if not os.path.isdir(base) or os.path.islink(base):
            continue
        for directory, dirs, names in os.walk(base, followlinks=False):
            dirs[:] = [d for d in dirs if not os.path.islink(os.path.join(directory, d))]
            for name in names:
                if name.endswith(".pending") or os.path.splitext(name)[1].lower() not in EXTENSIONS:
                    continue
                path = os.path.join(directory, name)
                if not os.path.islink(path):
                    yield path


def split(root, folders):
    count = 0
    for path in files(root, folders):
        if os.stat(path).st_nlink < 2:
            continue
        pending = path + ".split.pending"
        with open(path, "rb") as src, open(pending, "wb") as out:
            for chunk in iter(lambda: src.read(1 << 20), b""):
                out.write(chunk)
        os.replace(pending, path)
        count += 1
    print(f"split {count} linked files into separate copies", flush=True)


def main():
    args = sys.argv[1:]
    splitting = args[:1] == ["--split"]
    if splitting:
        args = args[1:]
    root = os.path.abspath(args[0])
    folders = args[1:] or FOLDERS
    if splitting:
        split(root, folders)
        return
    by_size = collections.defaultdict(list)
    for path in files(root, folders):
        size = os.path.getsize(path)
        if size >= MIN_SIZE:
            by_size[size].append(path)
    linked = saved = kept = 0
    for size, paths in by_size.items():
        if len(paths) < 2:
            continue
        groups = collections.defaultdict(list)
        for path in paths:
            groups[digest(path)].append(path)
        for group in groups.values():
            if len(group) < 2:
                continue
            group.sort()
            keep = group[0]
            keep_stat = os.stat(keep)
            for path in group[1:]:
                st = os.stat(path)
                if (st.st_dev, st.st_ino) == (keep_stat.st_dev, keep_stat.st_ino):
                    continue
                if not filecmp.cmp(keep, path, shallow=False):
                    continue
                pending = path + ".link.pending"
                try:
                    if os.path.exists(pending):
                        os.remove(pending)
                    os.link(keep, pending)
                    os.replace(pending, path)
                    linked += 1
                    saved += size
                except OSError:
                    kept += 1
                    if os.path.exists(pending):
                        os.remove(pending)
    print(f"linked {linked} duplicate files, {saved / 1e9:.3f} GB stored once"
          + (f"; {kept} kept as copies (links unsupported)" if kept else ""), flush=True)


if __name__ == "__main__":
    main()
