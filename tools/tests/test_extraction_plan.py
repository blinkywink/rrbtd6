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
import threading
import time
import unittest
from unittest.mock import MagicMock, patch

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS / "assets"))
spec = importlib.util.spec_from_file_location("extract_all", TOOLS / "extract_all.py")
driver = importlib.util.module_from_spec(spec)
spec.loader.exec_module(driver)


class Context:
    def __init__(self, root):
        self.out = root
        self.game = "game-fixture"
        self.calls = []
        self.index = {}
        self.progress = None

    parallel = driver.Ctx.parallel

    def py(self, *args, **kwargs):
        self.calls.append(tuple(map(str, args)))

    def containers(self):
        return self.index


def fixture(root, data):
    (root / "game").mkdir()
    (root / "game/game.json").write_text(json.dumps(data))
    return Context(root)


class PlanTests(unittest.TestCase):
    def test_maps_run_in_parallel_with_matching_logic_and_geometry_outputs(self):
        with tempfile.TemporaryDirectory() as temp:
            ctx = Context(Path(temp));ctx.aa = Path("fixture-aa")
            guard = threading.Lock();active = 0;peak = 0;ready = set()
            def run(script, *args, **kwargs):
                nonlocal active, peak
                if script.endswith("export_rounds.py"):return
                with guard:
                    active += 1;peak = max(peak, active)
                try:
                    self.assertTrue(script.endswith("export_map_pair.py"))
                    self.assertEqual(Path(args[3]).name, "map_"+args[1])
                    time.sleep(.02)
                    with guard:ready.add(args[1])
                finally:
                    with guard:active -= 1
            ctx.py = run
            with patch.object(driver.glob, "glob", return_value=["map_one_scenes_all_a.bundle", "map_two_scenes_all_b.bundle"]), patch("assets.model_batch.worker_count", return_value=2), redirect_stdout(io.StringIO()):
                driver.rounds_maps(ctx)
            self.assertEqual(peak, 2)
            self.assertEqual(ready, {"one", "two"})

    def test_map_failure_propagates_and_does_not_export_failed_geometry(self):
        with tempfile.TemporaryDirectory() as temp:
            ctx = Context(Path(temp));ctx.aa = Path("fixture-aa");calls = []
            def run(script, *args, **kwargs):
                calls.append(script)
                if script.endswith("export_map_pair.py"):raise RuntimeError("map failed")
            ctx.py = run
            with patch.object(driver.glob, "glob", return_value=["map_one_scenes_all_a.bundle"]), redirect_stdout(io.StringIO()):
                with self.assertRaisesRegex(RuntimeError, "map failed"):driver.rounds_maps(ctx)
            self.assertFalse(any(s.endswith("export_model.py") for s in calls))

    def test_upgrade_art_batches_jobs_and_removes_failed_manifest(self):
        for fail in [False,True]:
            with tempfile.TemporaryDirectory() as temp:
                ctx = Context(Path(temp))
                ctx.index={"Assets/ResizedImages/UI/UpgradeIcons/DartMonkey/100UpgradeIcon.png":"bundle",
                    "Assets/ResizedImages/MonkeyPortraits/Dart/000-DartMonkey.png":"bundle"}
                captured=[]
                def run(script, game, flag, manifest):
                    self.assertTrue(script.endswith("export_textures.py"))
                    self.assertEqual(flag,"--jobs")
                    captured.extend(json.loads(Path(manifest).read_text()))
                    if fail:raise RuntimeError("art interrupted")
                ctx.py=run
                if fail:
                    with self.assertRaisesRegex(RuntimeError,"art interrupted"):driver.upgrade_art(ctx)
                else:driver.upgrade_art(ctx)
                self.assertEqual(len(captured),2)
                self.assertEqual(captured[0]["prefix_numeric"],"DartMonkey")
                self.assertTrue(captured[1]["out"].endswith("portraits/DartMonkey") or captured[1]["out"].endswith("portraits\\DartMonkey"))
                self.assertFalse((ctx.out/".upgrade_art_jobs.json").exists())

    def test_audio_extracts_compressed_and_migrates_existing_wavs(self):
        ctx = Context(Path("fixture"))
        driver.audio(ctx)
        self.assertEqual(ctx.calls, [
            ("tools/assets/export_audio.py", "game-fixture"),
            ("tools/assets/export_jukebox.py", "game-fixture"),
            ("tools/assets/compress_audio.py", str(ctx.out / "audio")),
        ])
        self.assertEqual(driver.STEP_VERSIONS["audio"], 4)

    def test_sprite_atlases_are_exported_once_and_late_animations_are_compacted(self):
        ctx = Context(Path("fixture"))
        ctx.root = Path("fixture-install")
        driver.sprites(ctx)
        self.assertEqual([call for call in ctx.calls if call[0].endswith("export_sprites.py")],
                         [("tools/assets/export_sprites.py", "game-fixture", str(ctx.out / "sprites"))])
        ctx.calls.clear()
        driver.finish(ctx)
        self.assertEqual(ctx.calls[0], ("tools/assets/canonical_anims.py", str(ctx.root), str(ctx.out / "models")))
        self.assertIn(("tools/assets/compact_anims.py", str(ctx.out / "models")), ctx.calls)
        self.assertIn(("tools/assets/pack_anims.py", str(ctx.out / "models")), ctx.calls)
        self.assertIn(("tools/assets/delta_anims.py", str(ctx.out / "models")), ctx.calls)
        self.assertIn(("tools/assets/repack_anims.py", str(ctx.out / "models")), ctx.calls)
        self.assertIn(("tools/assets/word_anims.py", str(ctx.out / "models"), "--root", str(ctx.root)), ctx.calls)
        self.assertIn(("tools/assets/share_model_textures.py", str(ctx.out / "models")), ctx.calls)
        self.assertEqual(ctx.calls[-1], ("tools/assets/compact_json.py", str(ctx.out)))
        self.assertEqual(driver.STEPS[-2:], [driver.finish, driver.dedupe])
        # Models for unbuilt mechanics are exported before finish packs every model's animations.
        self.assertLess(driver.STEPS.index(driver.pending_models), driver.STEPS.index(driver.finish))
        ctx.calls.clear()
        driver.dedupe(ctx)
        self.assertEqual(ctx.calls, [("tools/assets/link_duplicates.py", str(ctx.out))])
        self.assertGreater(driver.STEP_VERSIONS["finish"], 1)

    def test_new_menus_and_daily_chest_hit_volume_are_in_the_launcher_recipe(self):
        ctx = Context(Path("fixture"))
        driver.menu_additions(ctx)
        self.assertTrue(any("DailyRewardUI.unity=DailyRewardUI" in arg for call in ctx.calls for arg in call))
        self.assertTrue(any("JukeboxTrack" in arg for call in ctx.calls for arg in call))
        self.assertEqual(ctx.calls[-1][0], "tools/assets/export_home_buttons.py")

    def test_whitespace_in_native_prefab_name_indexes_the_exported_directory(self):
        with tempfile.TemporaryDirectory(prefix="rrbtd6-plan-test-") as folder:
            root = Path(folder)
            plans = root / "local_extracted/display_plans"
            plans.mkdir(parents=True)
            (plans / "FutureTower.json").write_text(json.dumps({"towers": {"210": {
                "prefab": "Assets/210-FutureTower .prefab", "bundle": "fixture.bundle"}}}))
            target = root / "local_extracted/models/FutureTower/tiers/210-FutureTower"
            target.mkdir(parents=True)
            (target / "meta.json").write_text('{"shadow": []}')
            (target / "outline.bin").write_bytes(b"fixture")
            with chdir(root), patch.object(sys, "argv", ["export_tower_tiers.py", "game", "FutureTower"]), \
                    patch("subprocess.run") as child, redirect_stdout(io.StringIO()):
                runpy.run_path(str(TOOLS / "assets/export_tower_tiers.py"), run_name="__main__")
            child.assert_not_called()
            self.assertEqual(json.loads((target.parent / "index.json").read_text()), {"210": target.name})

    def test_interrupted_checkpoint_write_preserves_completed_steps(self):
        with tempfile.TemporaryDirectory(prefix="rrbtd6-plan-test-") as folder:
            path = Path(folder) / "state.json"
            old = {"fingerprint": "build", "done": {"maps": 1}}
            driver.atomic_write(path, json.dumps(old))
            with patch.object(driver.os, "replace", side_effect=OSError("interrupted")):
                with self.assertRaises(OSError):
                    driver.atomic_write(path, '{"fingerprint":"build","done":{"maps":1,"towers":1}}')
            self.assertEqual(driver.load_state(path, "build"), old)
            driver.atomic_write(path, '{"fingerprint":"build","done":{"maps":1,"towers":1}}')
            self.assertEqual(driver.load_state(path, "build")["done"], {"maps": 1, "towers": 1})

    def test_legacy_damaged_or_different_build_checkpoint_can_restart(self):
        with tempfile.TemporaryDirectory(prefix="rrbtd6-plan-test-") as folder:
            path = Path(folder) / "state.json"
            empty = {"fingerprint": "new-build", "done": {}}
            self.assertEqual(driver.load_state(path, "new-build"), empty)
            for text in ['{"done":', 'null', '[]', '{"fingerprint":"new-build","done":[]}',
                         '{"fingerprint":"old-build","done":{"maps":1}}']:
                path.write_text(text)
                self.assertEqual(driver.load_state(path, "new-build"), empty)

    def test_new_towers_and_heroes_automatically_receive_assets(self):
        with tempfile.TemporaryDirectory(prefix="rrbtd6-plan-test-") as folder:
            ctx = fixture(Path(folder), {"tower_tiers": {"FutureTower": {"hero": None}, "FutureHero": {"hero": {}}}})
            driver.extended_towers(ctx)
            self.assertEqual(ctx.calls, [
                ("tools/assets/build_display_plan.py", "game-fixture", "Geraldo"),
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
            with redirect_stdout(io.StringIO()):
                driver.particles(ctx)
            # The jobs run in parallel: compare what was exported, not the order.
            prefabs = next(call for call in ctx.calls if set(call[3:]) == set(ctx.index))
            self.assertEqual(sorted(call[-1] for call in ctx.calls if call is not prefabs),
                             ["projectiles", "scene:map_one", "scene:map_two", "tiers:FutureTower"])

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
                    patch("model_batch.run", side_effect=subprocess.CalledProcessError(7, "fixture-bake")), redirect_stdout(io.StringIO()):
                with self.assertRaises(subprocess.CalledProcessError) as caught:
                    runpy.run_path(str(TOOLS / "assets/export_tower_tiers.py"), run_name="__main__")
            self.assertEqual(caught.exception.returncode, 7)
            self.assertFalse((root / "local_extracted/models/FutureTower/tiers/index.json").exists())

    def test_failed_nested_export_is_not_reported_as_complete(self):
        # An empty folder and no real child: the script must fail on the child's exit code alone.
        child = MagicMock()
        child.__enter__.return_value = child
        child.stdout = iter([])
        child.wait.return_value = 5
        with tempfile.TemporaryDirectory(prefix="rrbtd6-plan-test-") as folder, chdir(folder),                 patch.object(sys, "argv", ["export_tower_assets.py", "game", "FutureTower"]),                 patch("subprocess.Popen", return_value=child), redirect_stdout(io.StringIO()):
            with self.assertRaises(subprocess.CalledProcessError) as caught:
                runpy.run_path(str(TOOLS / "assets/export_tower_assets.py"), run_name="__main__")
            self.assertEqual(caught.exception.returncode, 5)


class CacheCleanupTests(unittest.TestCase):
    def test_busy_cache_does_not_turn_successful_extraction_into_failure(self):
        with tempfile.TemporaryDirectory(prefix="rrbtd6-cache-test-") as folder:
            root = Path(folder)
            (root / "_bundle_cache").mkdir()
            (root / "_bundle_cache/file").write_bytes(b"busy")
            with patch.object(driver.shutil, "rmtree", side_effect=PermissionError("scanner lock")):
                self.assertEqual(driver.cleanup_bundle_cache(Context(root)), 0)
            self.assertEqual((root / "_bundle_cache/file").read_bytes(), b"busy")

    def test_cleanup_removes_only_the_disposable_cache_and_preserves_runtime_and_settings(self):
        with tempfile.TemporaryDirectory(prefix="rrbtd6-cache-test-") as folder:
            root = Path(folder)
            cache = root / "_bundle_cache/bundle"
            cache.mkdir(parents=True)
            (cache / "CAB-test").write_bytes(b"disposable")
            (cache / ".complete").write_bytes(b"")
            (root / "settings.json").write_text("settings")
            (root / "runtime.bin").write_bytes(b"keep")
            ctx = Context(root)
            self.assertEqual(driver.cleanup_bundle_cache(ctx), len(b"disposable"))
            self.assertFalse((root / "_bundle_cache").exists())
            self.assertEqual((root / "settings.json").read_text(), "settings")
            self.assertEqual((root / "runtime.bin").read_bytes(), b"keep")
            self.assertEqual(driver.cleanup_bundle_cache(ctx), 0)

    def test_linked_cache_and_nested_link_are_refused_before_deletion(self):
        with tempfile.TemporaryDirectory(prefix="rrbtd6-cache-test-") as folder:
            root = Path(folder)
            cache = root / "_bundle_cache"
            cache.mkdir()
            (cache / "keep").write_bytes(b"keep")
            ctx = Context(root)
            with patch.object(driver.Path, "is_symlink", return_value=True):
                self.assertEqual(driver.cleanup_bundle_cache(ctx), 0)
            self.assertTrue((cache / "keep").exists())
            original = driver.Path.is_symlink
            with patch.object(driver.Path, "is_symlink", lambda path: path.name == "keep" or original(path)):
                self.assertEqual(driver.cleanup_bundle_cache(ctx), 0)
            self.assertTrue((cache / "keep").exists())

    def test_full_success_cleans_cache_but_partial_failed_and_developer_runs_keep_it(self):
        def successful(ctx): pass
        def failure(ctx): raise RuntimeError("export interrupted")
        for extra, fail, cleans in [([], False, True), (["--only", "successful"], False, False),
                                     (["--keep-bundle-cache"], False, False), ([], True, False)]:
            with self.subTest(extra=extra, fail=fail), tempfile.TemporaryDirectory(prefix="rrbtd6-cache-plan-") as folder:
                root = Path(folder)
                game = root / "game"
                game.mkdir()
                (game / "GameAssembly.dll").write_bytes(b"fixture")
                steps = [successful, failure] if fail else [successful]
                with patch.object(sys, "argv", ["extract_all.py", "--game", str(game), "--root", str(root), *extra]), \
                     patch.object(driver, "install_hook"), patch.object(driver, "fingerprint", return_value="fixture"), \
                     patch.object(driver, "STEPS", steps), \
                     patch.object(driver, "STEP_VERSIONS", {step.__name__: 1 for step in steps}), \
                     patch.object(driver, "cleanup_bundle_cache") as cleanup, redirect_stdout(io.StringIO()):
                    if fail:
                        with self.assertRaisesRegex(RuntimeError, "interrupted"):
                            driver.main()
                    else:
                        driver.main()
                    self.assertEqual(cleanup.called, cleans)
                    stamp = root / "local_extracted/assets_version.json"
                    self.assertEqual(stamp.exists(), not fail and "--only" not in extra)
                    if stamp.exists():
                        self.assertEqual(json.loads(stamp.read_text())["revision"], driver.asset_revision())



if __name__ == "__main__":
    unittest.main()
