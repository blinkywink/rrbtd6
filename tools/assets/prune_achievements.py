"""Remove obsolete generated achievement assets, preserving shared art and user files."""
import json
from pathlib import Path
import sys

OBSOLETE = ("ui/runtime/achievements.json", "ui/runtime/AchievementsUI.json", "ui/prefabs/AchievementsUI.json")


def strings(value):
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        for item in value.values():
            yield from strings(item)
    elif isinstance(value, list):
        for item in value:
            yield from strings(item)


def prune(root):
    root = Path(root).resolve()
    catalogue = root / OBSOLETE[0]
    candidates = set()
    if catalogue.is_file():
        for item in json.loads(catalogue.read_text(encoding="utf-8")):
            candidates.update(item[key] for key in ("icon", "hidden_icon") if item.get(key))
    # Shared rewards/currency images remain owned by their surviving menus.
    used = set()
    for path in (root / "ui/runtime").glob("*.json"):
        if path.name in ("achievements.json", "AchievementsUI.json"):
            continue
        try:
            used.update(strings(json.loads(path.read_text(encoding="utf-8"))))
        except (ValueError, OSError):
            pass
    targets = [root / relative for relative in OBSOLETE]
    targets += [root / "ui/menu_icons" / name for name in candidates - used
                if Path(name).name == name and name.lower().endswith(".png")]
    # Only directories explicitly dedicated to achievement sprites are obsolete.
    for directory in (root / "sprites").glob("*Achievement*"):
        if directory.is_dir() and not directory.is_symlink():
            targets.extend(path for path in directory.rglob("*") if path.is_file())
    removed = size = 0
    for path in targets:
        resolved = path.resolve()
        if path.is_symlink() or not resolved.is_relative_to(root) or not path.is_file():
            continue
        size += path.stat().st_size
        path.unlink()
        removed += 1
    print(f"Removed {removed} unused achievement files ({size / 1e6:.1f} MB); shared art and saves kept", flush=True)
    return removed


if __name__ == "__main__":
    prune(sys.argv[1])
