import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("canonical_anims", Path(__file__).parent / "assets/canonical_anims.py")
canonical = importlib.util.module_from_spec(spec)
spec.loader.exec_module(canonical)


class CanonicalTests(unittest.TestCase):
    def test_matching_shipped_client_receives_explicit_models_path(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            models = root / "models"
            models.mkdir()
            (root / "rrbtd6.exe").write_bytes(b"fixture")
            with patch.object(canonical.subprocess, "run") as run:
                canonical.run(root, models)
            run.assert_called_once_with([str(root / "rrbtd6.exe"), "--canonicalize-anims", str(models)], check=True, cwd=root)

    def test_missing_client_fails_instead_of_recording_completion(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            with self.assertRaises(FileNotFoundError): canonical.run(root, root)


if __name__ == "__main__":
    unittest.main()
