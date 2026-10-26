"""Clean-install coverage/failure checks with synthetic game definitions only."""
from contextlib import chdir, redirect_stdout
import importlib.util
import io
import json
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

TOOLS = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("extract_all", TOOLS / "extract_all.py")
driver = importlib.util.module_from_spec(spec)
spec.loader.exec_module(driver)


class Context:
    def __init__(self, root):
        self.out = root
        self.game = "game-fixture"
        self.calls = []
        self.index = {}

    def py(self, *args, **kwargs):
        self.calls.append(tuple(map(str, args)))

    def containers(self):
        return self.index


def fixture(root, data):
    (root / "game").mkdir()
    (root / "game/game.json").write_text(json.dumps(data))
    return Context(root)


class PlanTests(unittest.TestCase):
    def test_new_towers_and_heroes_automatically_receive_assets(self):
        with tempfile.TemporaryDirectory(prefix="rrbtd6-plan-test-") as folder:
            ctx = fixture(Path(folder), {"tower_tiers": {"FutureTower": {"hero": None}, "FutureHero": {"hero": {}}}})
            driver.extended_towers(ctx)
            self.assertEqual(ctx.calls, [
                ("tools/assets/export_tower_assets.py", "game-fixture", "FutureTower", "FutureHero"),
                ("tools/assets/export_hero_portraits.py", "game-fixture", "FutureHero"),
            ])

    def test_dynamic_effects_and_every_actor_and_map_are_included(self):
        with tempfile.TemporaryDirectory(prefix="rrbtd6-plan-test-") as folder:
            root = Path(folder)
            ctx = fixture(root, {"tower_tiers": {"FutureTower": {}}, "place_fx": ["PlacementTest", 1],
                                 "nested": {"expire_fx": ["ExpireTest", 2], "fx": "TransformTest", "fire_fx": "PulseTest"}})
            expected = {"UpgradeEffect", "UpgradeEffectTier5", "HeroUpgradeEffect", "HeroUpgradeEffectLv20", "SellEffect",
                        "PlacementTest", "ExpireTest", "TransformTest", "PulseTest"}
            ctx.index = {f"Assets/Effects/{name}.prefab": "fixture.bundle" for name in expected}
            for name in ["map_one", "map_two"]:
                (root / "maps" / name).mkdir(parents=True)
                (root / "maps" / name / "map.json").write_text("{}")
            driver.particles(ctx)
            self.assertEqual(set(ctx.calls[0][3:]), set(ctx.index))
            self.assertEqual([call[-1] for call in ctx.calls[1:]], ["scene:map_one", "scene:map_two", "tiers:FutureTower", "projectiles"])

    def test_unresolved_effect_stops_before_any_export_is_started(self):
        with tempfile.TemporaryDirectory(prefix="rrbtd6-plan-test-") as folder:
            ctx = fixture(Path(folder), {"tower_tiers": {}, "place_fx": ["MissingEffect", 1]})
            with self.assertRaisesRegex(ValueError, "expected one original prefab"):
                driver.particles(ctx)
            self.assertEqual(ctx.calls, [])

    def test_failed_tier_bake_never_publishes_a_successful_index(self):
        with tempfile.TemporaryDirectory(prefix="rrbtd6-plan-test-") as folder:
            root = Path(folder)
            (root / "local_extracted/display_plans").mkdir(parents=True)
            (root / "local_extracted/display_plans/FutureTower.json").write_text(json.dumps({"towers": {
                "000": {"prefab": "Assets/FutureTower.prefab", "bundle": "generated_assets_all_fixture.bundle"}}}))
            failed = subprocess.CompletedProcess([], 7, stdout="", stderr="bake failed")
            with chdir(root), patch.object(sys, "argv", ["export_tower_tiers.py", "game", "FutureTower"]), \
                    patch("subprocess.run", return_value=failed), redirect_stdout(io.StringIO()):
                with self.assertRaises(SystemExit) as caught:
                    runpy.run_path(str(TOOLS / "assets/export_tower_tiers.py"), run_name="__main__")
            self.assertEqual(caught.exception.code, 7)
            self.assertFalse((root / "local_extracted/models/FutureTower/tiers/index.json").exists())

    def test_failed_nested_export_is_not_reported_as_complete(self):
        failed = subprocess.CompletedProcess([], 5, stdout="", stderr="export failed")
        with patch.object(sys, "argv", ["export_tower_assets.py", "game", "FutureTower"]), \
                patch("subprocess.run", return_value=failed), redirect_stdout(io.StringIO()):
            with self.assertRaises(subprocess.CalledProcessError):
                runpy.run_path(str(TOOLS / "assets/export_tower_assets.py"), run_name="__main__")


if __name__ == "__main__":
    unittest.main()
