"""The author SDK emits plain data; no scripts run in the client or on peers."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
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
