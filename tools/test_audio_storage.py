"""Lossless audio migration tests; fixtures contain no game assets."""
import importlib.util
import io
from pathlib import Path
import tempfile
import unittest
import sys
from unittest.mock import patch

import numpy as np
import soundfile as sf

spec = importlib.util.spec_from_file_location("compress_audio", Path(__file__).parent / "assets/compress_audio.py")
storage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(storage)


def fixture(path, rate=44100, channels=2):
    samples = np.array([-32768, -32767, -1, 0, 1, 32766, 32767] * 20000, dtype=np.int16)
    samples = samples[:len(samples)//channels*channels].reshape(-1, channels)
    sf.write(str(path), samples, rate, subtype="PCM_16")
    return samples


class AudioStorageTests(unittest.TestCase):
    def test_all_formats_and_extremes_exact(self):
        for rate, channels in [(44100, 1), (44100, 2), (48000, 2), (22050, 2)]:
            with self.subTest(rate=rate, channels=channels), tempfile.TemporaryDirectory() as d:
                source = Path(d)/"audio.wav"
                expected = fixture(source, rate, channels)
                before, after = storage.compress(source)
                self.assertFalse(source.exists())
                actual, actual_rate = sf.read(str(source.with_suffix(".flac")), dtype="int16", always_2d=True)
                self.assertEqual(actual_rate, rate)
                np.testing.assert_array_equal(actual, expected)
                self.assertLess(after, before)
                self.assertEqual(list(Path(d).glob("*.pending")), [])

    def test_keep_original_and_direct_export(self):
        with tempfile.TemporaryDirectory() as d:
            source = Path(d)/"audio.wav"
            fixture(source)
            original = source.read_bytes()
            storage.compress(source, keep_original=True)
            self.assertEqual(source.read_bytes(), original)
            direct = Path(d)/"direct.flac"
            storage.encode_bytes(original, direct)
            self.assertEqual(direct.read_bytes(), source.with_suffix(".flac").read_bytes())
            self.assertFalse(direct.with_suffix(".wav").exists())

    def test_failed_verification_preserves_source_and_previous_output(self):
        with tempfile.TemporaryDirectory() as d:
            source = Path(d)/"audio.wav"
            fixture(source)
            original = source.read_bytes()
            destination = source.with_suffix(".flac")
            destination.write_bytes(b"previous")
            real = sf.SoundFile
            def open_file(file, *args, **kwargs):
                if str(file).endswith(".pending") and kwargs.get("mode") != "w":
                    raise ValueError("simulated verification failure")
                return real(file, *args, **kwargs)
            with patch.object(storage.sf, "SoundFile", side_effect=open_file):
                with self.assertRaises(ValueError):
                    storage.compress(source)
            self.assertEqual(source.read_bytes(), original)
            self.assertEqual(destination.read_bytes(), b"previous")
            self.assertEqual(list(Path(d).glob("*.pending")), [])

    def test_source_changed_keeps_wav(self):
        with tempfile.TemporaryDirectory() as d:
            source = Path(d)/"audio.wav"
            fixture(source)
            real = storage.encode
            def encode(*args):
                result = real(*args)
                with source.open("ab") as out:
                    out.write(b"changed")
                return result
            with patch.object(storage, "encode", side_effect=encode):
                with self.assertRaisesRegex(RuntimeError, "source changed"):
                    storage.compress(source)
            self.assertTrue(source.exists())

    def test_unsupported_format_keeps_source(self):
        with tempfile.TemporaryDirectory() as d:
            source = Path(d)/"audio.wav"
            sf.write(str(source), np.array([0., .25, -.5]), 44100, subtype="FLOAT")
            with self.assertRaises(ValueError):
                storage.compress(source)
            self.assertTrue(source.exists())
            self.assertFalse(source.with_suffix(".flac").exists())
            self.assertEqual(list(Path(d).glob("*.pending")), [])


if __name__ == "__main__":
    if len(sys.argv)==3 and sys.argv[1]=="--fixtures":
        root=Path(sys.argv[2]);root.mkdir(parents=True, exist_ok=True)
        for rate, channels in [(44100, 1), (44100, 2), (48000, 2), (22050, 2)]:
            source=root/f"synthetic-{rate}-{channels}.wav"
            fixture(source, rate, channels)
            storage.compress(source, keep_original=True)
        print(f"Wrote four synthetic WAV/FLAC pairs to {root}")
    else:
        unittest.main()
