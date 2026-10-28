"""Lossless JSON token preservation and safe asset migration checks."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("compact_json", Path(__file__).parent / "assets/compact_json.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class CompactJsonTests(unittest.TestCase):
    def test_preserves_exact_strings_number_tokens_and_duplicate_keys(self):
        raw = (b'{ \n "a": -0.0, "a": 1.234567890123456789e+012, '
               b'"big": 123456789012345678901234567890, '
               b'"text": "spaces  and\\t tabs \\" quoted \\" \\\\ slash", '
               b'"unicode": "\\u00e9", "raw": "\xc3\xa9", "list": [true, null, false] }\r\n')
        expected = (b'{"a":-0.0,"a":1.234567890123456789e+012,'
                    b'"big":123456789012345678901234567890,'
                    b'"text":"spaces  and\\t tabs \\" quoted \\" \\\\ slash",'
                    b'"unicode":"\\u00e9","raw":"\xc3\xa9","list":[true,null,false]}')
        self.assertEqual(module.compact_bytes(raw), expected)

    def test_invalid_source_is_never_repaired_or_replaced(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder, "invalid.json")
            raw = b'{"invalid": 1 2}'
            path.write_bytes(raw)
            with self.assertRaises(ValueError):
                module.compact(path)
            self.assertEqual(path.read_bytes(), raw)
            self.assertEqual(list(Path(folder).glob("*.pending")), [])

    def test_repeat_pass_does_not_rewrite_file(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder, "data.json")
            path.write_bytes(b'{ "value": [1, 2, 3] }\n')
            old, new = module.compact(path)
            self.assertLess(new, old)
            stamp = module.signature(path)
            self.assertEqual(module.compact(path), (new, new))
            self.assertEqual(module.signature(path), stamp)

    def test_atomic_failure_retains_source_and_cleans_temporary(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder, "data.json")
            raw = b'{ "value": 1 }'
            path.write_bytes(raw)
            with patch.object(module.os, "replace", side_effect=OSError("fixture")):
                with self.assertRaises(OSError):
                    module.compact(path)
            self.assertEqual(path.read_bytes(), raw)
            self.assertEqual(list(Path(folder).glob("*.pending")), [])

    def test_concurrent_export_is_kept(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder, "data.json")
            path.write_bytes(b'{ "value": 1 }')
            original = module.compact_bytes
            def change(raw):
                path.write_bytes(b'{ "value": 9 }')
                return original(raw)
            with patch.object(module, "compact_bytes", side_effect=change):
                with self.assertRaisesRegex(RuntimeError, "changed"):
                    module.compact(path)
            self.assertEqual(json.loads(path.read_bytes()), {"value": 9})
            self.assertEqual(list(Path(folder).glob("*.pending")), [])

    def test_replacing_one_hardlink_preserves_other_names(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder, "data.json")
            other = Path(folder, "other.json")
            raw = b'{ "value": 1 }'
            path.write_bytes(raw)
            os.link(path, other)
            module.compact(path)
            self.assertEqual(other.read_bytes(), raw)
            self.assertEqual(path.read_bytes(), b'{"value":1}')

    def test_inventory_excludes_settings_accounts_and_research(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            names = ["settings.json", "account/profile.json", "saves/game.json", "research/data.json",
                     "ui/prefabs/menu.json", "models/tower/meta.json", "game/game.json"]
            for name in names:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('{ "value": 1 }')
            self.assertEqual({str(p.relative_to(root)).replace("\\", "/") for p in module.files(root)}, set(names[-3:]))


if __name__ == "__main__":
    unittest.main()
