"""Exact temporal-prediction migration tests using synthetic frames only."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zlib

spec = importlib.util.spec_from_file_location("delta_anims", Path(__file__).parent / "assets/delta_anims.py")
storage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(storage)


def fixture(root, name="anim_q.bin.z"):
    stride = 240 if name == "anim.bin.z" else 100
    # Slow motion with byte overflow and all byte values represented.
    data = bytes((i % stride * 73 + i // stride) % 256 for i in range(stride * 500))
    p = Path(root) / name
    p.write_bytes(zlib.compress(data, 1))
    p.with_name("meta.json").write_text(json.dumps({"vertex_count": 10,
        "anims": [{"offset": 0, "frames": 200}, {"offset": 200, "frames": 300}]}))
    return p, data


class DeltaTests(unittest.TestCase):
    def test_quantized_and_float_bytes_restore_exactly(self):
        for name in ("anim_q.bin.z", "anim.bin.z"):
            with tempfile.TemporaryDirectory() as d:
                p, data = fixture(d, name)
                before, after = storage.pack(p)
                self.assertLess(after, before)
                self.assertFalse(p.exists())
                self.assertEqual(storage.read_bytes(p.with_suffix(".dz")), data)

    def test_keep_original_and_repeat_migration(self):
        with tempfile.TemporaryDirectory() as d:
            p, data = fixture(d)
            storage.pack(p, keep_original=True)
            self.assertEqual(zlib.decompress(p.read_bytes()), data)
            storage.pack(p)
            self.assertEqual(storage.read_bytes(p.with_suffix(".dz")), data)

    def test_bad_metadata_or_truncated_source_keeps_source(self):
        with tempfile.TemporaryDirectory() as d:
            p, _ = fixture(d)
            original = p.read_bytes()
            p.with_name("meta.json").write_text('{"vertex_count":10,"anims":[{"offset":0,"frames":501}]}')
            with self.assertRaises(ValueError): storage.pack(p)
            self.assertEqual(p.read_bytes(), original)
            self.assertEqual(list(Path(d).glob("*.pending")), [])
            p, _ = fixture(d)
            p.write_bytes(p.read_bytes()[:-1])
            with self.assertRaises(ValueError): storage.pack(p)
            self.assertTrue(p.exists())

    def test_corrupt_header_and_trailing_bytes_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            p, _ = fixture(d)
            storage.pack(p)
            p = p.with_suffix(".dz")
            good = p.read_bytes()
            for bad in (b"bad" + good[3:], good + b"garbage", good[:8] + bytes(8) + good[16:], good[:-1]):
                p.write_bytes(bad)
                with self.assertRaises((ValueError, zlib.error)): storage.read_bytes(p)

    def test_verification_failure_leaves_prior_output_and_source(self):
        with tempfile.TemporaryDirectory() as d:
            p, _ = fixture(d)
            target = p.with_suffix(".dz")
            target.write_bytes(b"prior")
            original = p.read_bytes()
            with patch.object(storage, "restored_frames", side_effect=ValueError("bad data")):
                with self.assertRaises(ValueError): storage.pack(p)
            self.assertEqual(p.read_bytes(), original)
            self.assertEqual(target.read_bytes(), b"prior")
            self.assertEqual(list(Path(d).glob("*.pending")), [])


if __name__ == "__main__":
    unittest.main()
