"""Synthetic storage migration tests; no game assets or services required."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zlib

spec = importlib.util.spec_from_file_location("pack_anims", Path(__file__).parent / "assets/pack_anims.py")
storage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(storage)


class StorageTests(unittest.TestCase):
    def test_pack_roundtrip_and_fresh_export_priority(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "anim_q.bin"
            original = bytes(range(256)) * 10000
            p.write_bytes(original)
            before, after = storage.pack(p)
            self.assertLess(after, before / 10)
            self.assertFalse(p.exists())
            self.assertEqual(storage.read_bytes(p), original)
            p.write_bytes(b"new export")
            self.assertEqual(storage.read_bytes(p), b"new export")

    def test_keep_original_preserves_developer_file(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "anim.bin"
            original = b"float-frames" * 100000
            p.write_bytes(original)
            storage.pack(p, keep_original=True)
            self.assertEqual(p.read_bytes(), original)
            self.assertEqual(zlib.decompress(p.with_name(p.name + ".z").read_bytes()), original)

    def test_verification_failure_preserves_source_and_prior_pack(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "anim.bin"
            p.write_bytes(b"frames" * 10000)
            prior = p.with_name(p.name + ".z")
            prior.write_bytes(b"prior")
            with patch.object(storage, "verify", side_effect=ValueError("bad checksum")):
                with self.assertRaises(ValueError): storage.pack(p)
            self.assertEqual(p.read_bytes(), b"frames" * 10000)
            self.assertEqual(prior.read_bytes(), b"prior")
            self.assertEqual(list(Path(d).glob("*.pending")), [])

    def test_truncation_corruption_and_trailing_bytes_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            source = b"unchanged-frames" * 100000
            packed = zlib.compress(source, 1)
            p = Path(d) / "bad.z"
            for data in [packed[:-1], packed + b"garbage", packed[:-1] + bytes([packed[-1] ^ 1])]:
                p.write_bytes(data)
                with self.assertRaises((ValueError, zlib.error)):
                    storage.verify(p, len(source), hashlib.blake2b(source).digest())

    def test_concurrent_export_is_not_removed(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "anim.bin"
            p.write_bytes(b"old frames" * 10000)
            real_verify = storage.verify
            def reexport(*args):
                real_verify(*args)
                p.write_bytes(b"new frames")
            with patch.object(storage, "verify", side_effect=reexport):
                with self.assertRaises(RuntimeError): storage.pack(p)
            self.assertEqual(p.read_bytes(), b"new frames")

    def test_not_larger_for_tiny_payload(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "anim.bin"
            p.write_bytes(b"a")
            self.assertEqual(storage.pack(p), (1, 1))
            self.assertTrue(p.exists())
            self.assertFalse(p.with_name(p.name + ".z").exists())


if __name__ == "__main__":
    unittest.main()
