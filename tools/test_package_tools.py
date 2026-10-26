"""Release packaging checks use a tiny temporary Git index, never real game data."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location("package_tools", Path(__file__).with_name("package_tools.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class PackagingTests(unittest.TestCase):
    def test_zip_uses_public_tracked_files_with_tools_prefix(self):
        with tempfile.TemporaryDirectory(prefix="rrbtd6-package-test-") as temp:
            root = Path(temp)
            subprocess.run(["git", "init", "--quiet", str(root)], check=True)
            for name in ["extract_all.py", "requirements.txt", "sitecustomize.py", "unitypy_lowmem.py", "assets/test.py",
                         "assets/private_account.py", "mod/scenarios/example.json"]:
                path = root / "tools" / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("fixture", encoding="utf-8")
            (root / "tools/public_exclude.txt").write_text("# excluded\ntools/assets/private_account.py\n", encoding="utf-8")
            subprocess.run(["git", "add", "tools"], cwd=root, check=True)
            (root / "tools/untracked.txt").write_text("not shipped", encoding="utf-8")
            destination = root / "dist/tools.zip"
            names = module.package(root, destination)
            with zipfile.ZipFile(destination) as archive:
                self.assertIsNone(archive.testzip())
                self.assertEqual(archive.namelist(), names)
                self.assertIn("tools/assets/test.py", names)
                self.assertEqual(archive.read("tools/extract_all.py"), b"fixture")
                for excluded in ["tools/assets/private_account.py", "tools/untracked.txt", "tools/mod/scenarios/example.json"]:
                    self.assertNotIn(excluded, names)

    def test_missing_required_tooling_cannot_create_release_archive(self):
        with tempfile.TemporaryDirectory(prefix="rrbtd6-package-test-") as temp:
            root = Path(temp)
            subprocess.run(["git", "init", "--quiet", str(root)], check=True)
            (root / "tools").mkdir()
            (root / "tools/public_exclude.txt").write_text("", encoding="utf-8")
            subprocess.run(["git", "add", "tools"], cwd=root, check=True)
            destination = root / "dist/tools.zip"
            with self.assertRaisesRegex(ValueError, "Incomplete extraction tooling"):
                module.package(root, destination)
            self.assertFalse(destination.exists())


if __name__ == "__main__":
    unittest.main()
