"""The Python author SDK emits data and optional embedded event source; Python never runs in the client."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("modsdk", ROOT / "tools/modsdk.py")
sdk = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sdk)


class ModSdkTests(unittest.TestCase):
    def test_quickstart_builds_with_no_third_party_dependencies(self):
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "mod.json"
            subprocess.run([sys.executable, "-I", str(ROOT / "tools/modsdk.py"), "build",
                            str(ROOT / "website/public/mods/quickstart.py"), "--out", str(destination)], check=True,
                           stdout=subprocess.DEVNULL)
            manifest = json.loads(destination.read_text(encoding="utf-8"))
            self.assertEqual(manifest["api_version"], 1)
            self.assertEqual(manifest["content"][0]["id"], "quickstart-dart")
            self.assertEqual(manifest["content"][1]["path"], "/tower_tiers/quickstart-dart/tiers/000/attacks/0/weapons/0/projectile/damage")
            self.assertNotIn("script", manifest)

    def test_reusable_projectile_templates_emit_native_copy_and_patch_operations(self):
        mod = sdk.Mod("test", "Test")
        bomb = mod.projectile("big-bomb", base_tower="BombShooter", damage=10)
        tower = mod.tower("dart")
        tower.projectile(bomb, pierce=30)
        edits = mod.manifest["content"]
        self.assertEqual(edits[1]["op"], "copy")
        self.assertIn("/BombShooter/", edits[1]["from"])
        self.assertTrue(edits[2]["path"].endswith("/projectile/damage"))
        self.assertEqual(edits[3]["value"], 30)

    def test_upgrade_helper_targets_a_real_upgrade_and_rejects_unsupported_paths(self):
        mod=sdk.Mod("test","Test")
        tower=mod.tower("dart")
        tower.upgrade(0,1,cost=150,stats={"range":40},weapon={"rate":.5},projectile={"damage":5})
        edit=mod.manifest["content"][1]
        self.assertEqual(edit["op"],"upgrade_patch");self.assertEqual(edit["tier"],1)
        self.assertEqual(edit["projectile"]["damage"],5)
        tower.upgrade(1,2,projectile=mod.projectile("bomb",base_tower="BombShooter",damage=10))
        self.assertIn("BombShooter",mod.manifest["content"][-1]["projectile_from"])
        self.assertEqual(mod.manifest["content"][-1]["projectile"]["damage"],10)
        with self.assertRaises(ValueError):tower.upgrade(3,1)

    def test_existing_towers_shop_and_embedded_scripts(self):
        mod=sdk.Mod("test","Test")
        mod.existing_tower("DartMonkey").shop(after="SniperMonkey",category="military")
        mod.script("mechanics",source="fn on_round_end(ctx) { [] }")
        edits=mod.manifest["content"]
        self.assertEqual(edits[0],dict(op="shop",tower="DartMonkey",after="SniperMonkey",category=2))
        self.assertEqual(edits[1]["op"],"script")
        self.assertEqual(edits[1]["tick_interval"],60)
        with self.assertRaises(ValueError):mod.existing_tower("DartMonkey").shop(before="A",after="B")
        with self.assertRaises(ValueError):mod.script("bad",source="",tick_interval=0)

    def test_downloadable_entity_lab_builds_with_embedded_source(self):
        with tempfile.TemporaryDirectory() as directory:
            destination=Path(directory)/"mod.json"
            subprocess.run([sys.executable,"-I",str(ROOT/"website/public/mods/modsdk.py"),"build",
                            str(ROOT/"website/public/mods/entity-lab.py"),"--out",str(destination)],check=True,stdout=subprocess.DEVNULL)
            manifest=json.loads(destination.read_text(encoding="utf-8"))
            self.assertEqual(manifest["id"],"entity-lab")
            self.assertIn("fn before_damage",manifest["content"][0]["source"])
            self.assertIn("fn on_query",manifest["content"][0]["source"])
            self.assertEqual((ROOT/"tools/mod_examples/entity-lab/entities.rhai").read_bytes(),
                             (ROOT/"website/public/mods/entities.rhai").read_bytes())

    def test_authoring_errors_do_not_overwrite_existing_manifest(self):
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "mod.json"
            destination.write_text("original")
            mod = sdk.Mod("test", "Test")
            mod.set("/start/cash", float("nan"))
            with self.assertRaises(ValueError):
                mod.write(destination)
            self.assertEqual(destination.read_text(), "original")
            with self.assertRaises(ValueError):
                sdk.Mod("../escape", "Bad")


if __name__ == "__main__":
    unittest.main()
