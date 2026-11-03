"""Mark a generated projectile complete only after every export step succeeds."""
import os
from pathlib import Path
import tempfile


def complete(folder, signature):
    folder = Path(folder)
    try:
        return (folder / ".export_complete").read_text(encoding="utf-8") == signature and any(
            (folder / name).is_file() for name in ("meta.json", "sprite.json"))
    except (OSError, UnicodeError):
        return False


def begin(folder):
    (Path(folder) / ".export_complete").unlink(missing_ok=True)


def commit(folder, signature):
    folder = Path(folder)
    fd, path = tempfile.mkstemp(prefix="projectile-", suffix=".pending", dir=folder)
    pending = Path(path)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as stream:
            stream.write(signature)
        os.replace(pending, folder / ".export_complete")
    finally:
        pending.unlink(missing_ok=True)
