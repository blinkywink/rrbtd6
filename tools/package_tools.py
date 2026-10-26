"""Create the launcher's ZIP from tracked public tooling, independent of shell/tar.

Usage: python tools/package_tools.py dist/rrbtd6-tools.zip
No extracted data, untracked files, recorder scenarios or private account tools.
"""
from pathlib import Path
import subprocess
import sys
import zipfile


def package(root: Path, destination: Path) -> list[str]:
    exclusions = ["tools/mod"]
    for line in (root / "tools/public_exclude.txt").read_text(encoding="utf-8").splitlines():
        if line.strip() and not line.lstrip().startswith("#"):
            exclusions.append(line.strip())
    tracked = subprocess.check_output(
        ["git", "ls-files", "-z", "--", "tools", *[f":(exclude){p}" for p in exclusions]], cwd=root)
    names = sorted(p.decode("utf-8") for p in tracked.split(b"\0") if p)
    required = {"tools/extract_all.py", "tools/requirements.txt", "tools/sitecustomize.py", "tools/unitypy_lowmem.py"}
    if not required.issubset(names):
        raise ValueError(f"Incomplete extraction tooling: {sorted(required - set(names))}")
    destination = destination.resolve()
    destination.parent.mkdir(parents=True, exist_ok=True)
    names = [name for name in names if (root / name).resolve() != destination]
    with zipfile.ZipFile(destination, "w", zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        for name in names:
            archive.write(root / name, name)
    return names


if __name__ == "__main__":
    root = Path(__file__).resolve().parents[1]
    destination = Path(sys.argv[1])
    print(f"Packaged {len(package(root, destination))} tools files into {destination}")
