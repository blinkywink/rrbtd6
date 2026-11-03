"""Reuse the loaded bundle when exporting successive static projectile meshes."""
from contextlib import redirect_stdout
from io import StringIO
import sys

_model = None


def release():
    if _model is not None:
        _model._envs.clear()
        for name in ("parts", "verts", "idxs", "vnormals", "vcolors"):
            getattr(_model, name).clear()
        _model.tex_cache.clear()
        getattr(_model, "shader_done", {}).clear()
        getattr(_model, "_shader_names", {}).clear()


def export(game, out, bundle, prefab, env):
    global _model
    previous = sys.argv
    sys.argv = ["export_model.py", game, out, "prefab", bundle.rsplit("_", 1)[0], prefab]
    try:
        if _model is None:
            import export_model
            _model = export_model
        release()
        _model.GAME, _model.OUT, _model.MODE, _model.BUNDLE = game, out, "prefab", bundle.rsplit("_", 1)[0]
        with redirect_stdout(StringIO()):
            _model.main(env)
        return f"mesh {len(_model.parts)} parts"
    finally:
        sys.argv = previous
