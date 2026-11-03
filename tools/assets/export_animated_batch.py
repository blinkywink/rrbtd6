"""Export a bounded group of animated prefabs, sharing loaded Unity bundles."""
import json
import os
from pathlib import Path
import sys


def complete(folder):
    folder = Path(folder)
    try:
        return (folder / "outline.bin").is_file() and "shadow" in json.loads((folder / "meta.json").read_text(encoding="utf-8"))
    except (OSError, ValueError, TypeError):
        return False


def run(game, jobs, compact=True):
    pending = [job for job in jobs if not complete(job[0])]
    if not pending:
        return
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    sys.argv = [sys.argv[0], game, *pending[0][:3]]
    import export_animated as ea
    import compact_anims as ca
    # A job is [out, bundle, prefab], plus environment settings for that export (RRBTD6_ANIM_*).
    for i, (out, bundle, prefab, *settings) in enumerate(pending):
        settings = settings[0] if settings else {}
        ea.OUT, ea.BUNDLE, ea.PREFAB = out, bundle, prefab
        ea.em.OUT = out
        ea.em.tex_cache.clear()
        if not compact:
            getattr(ea.em, "shader_done", {}).clear()
            getattr(ea.em, "_shader_names", {}).clear()
        os.environ.update(settings)
        try:
            ea.main()
            if compact:
                ca.convert(out)
        except Exception:
            Path(out, "meta.json").unlink(missing_ok=True)
            raise
        finally:
            for name in settings:
                os.environ.pop(name, None)
        print("@@RRBTD6_PROGRESS " + json.dumps({"done": i + 1, "total": len(pending),
              "label": "Models: " + Path(out).name}), flush=True)


if __name__ == "__main__":
    run(sys.argv[1], json.loads(Path(sys.argv[2]).read_text(encoding="utf-8")), compact="--keep-floats" not in sys.argv[3:])
