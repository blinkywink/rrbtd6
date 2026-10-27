"""Reference-resolution regression checks, without UnityPy or proprietary files."""
import ast
from pathlib import Path
from types import SimpleNamespace as Obj
import unittest
from unittest.mock import Mock

source = ast.parse((Path(__file__).parent / "assets/export_model.py").read_text())
resolver = next(n for n in source.body if isinstance(n, ast.FunctionDef) and n.name == "resolve")


def bundle(*files):
    return Obj(files={"bundle": Obj(files={str(i): file for i, file in enumerate(files)})})


class ReferenceTests(unittest.TestCase):
    def test_scene_shared_assets_use_the_owning_bundle_even_with_stale_index(self):
        owner = Obj(name="scene", objects={}, externals=[Obj(path="archive:/same.sharedAssets")])
        correct, stale = object(), object()
        current = Obj(name="same.sharedAssets", objects={35: correct})
        old = Obj(name="same.sharedAssets", objects={35: stale, 99: stale})
        load = Mock(side_effect=AssertionError("must not load the stale indexed bundle"))
        scope = {"_envs": {"old": bundle(old), "current": bundle(owner, current)},
                 "CAB_INDEX": {"same.sharedAssets": "old"}, "env_for_bundle": load}
        exec(compile(ast.Module(body=[resolver], type_ignores=[]), "resolve-fixture", "exec"), scope)
        self.assertIs(scope["resolve"](owner, {"m_FileID": 1, "m_PathID": 35}), correct)
        self.assertIsNone(scope["resolve"](owner, {"m_FileID": 1, "m_PathID": 99}))
        load.assert_not_called()

    def test_true_external_bundle_and_local_references_still_resolve(self):
        expected = object()
        owner = Obj(name="scene", objects={7: expected}, externals=[Obj(path="archive:/external")])
        load = Mock(return_value=bundle(Obj(name="external", objects={35: expected})))
        scope = {"_envs": {"current": bundle(owner)}, "CAB_INDEX": {"external": "external.bundle"}, "env_for_bundle": load}
        exec(compile(ast.Module(body=[resolver], type_ignores=[]), "resolve-fixture", "exec"), scope)
        self.assertIs(scope["resolve"](owner, {"m_FileID": 0, "m_PathID": 7}), expected)
        self.assertIsNone(scope["resolve"](owner, {"m_FileID": 0, "m_PathID": 0}))
        self.assertIs(scope["resolve"](owner, {"m_FileID": 1, "m_PathID": 35}), expected)
        load.assert_called_once_with("external.bundle")


if __name__ == "__main__":
    unittest.main()
