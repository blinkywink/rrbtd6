"""skin_anims.py with synthetic data: round trip, rebuilt frames, and the installer migration."""
import json
from pathlib import Path
import sys
import tempfile
import unittest

import numpy as np

# Import without leaving tools/ on sys.path (other tests check the extraction hook's path).
_path = sys.path[:]
sys.path[:0] = [str(Path(__file__).resolve().parents[1]), str(Path(__file__).resolve().parents[1] / "assets")]
import extract_all as driver
import skin_anims
sys.path[:] = _path

I = [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0]]


def sample():
    v = np.array([[0, 0, 0], [0, 1, 0]], np.float32)
    n = np.array([[0, 0, 2], [0, 0, 1]], np.float32)
    idx = np.array([[0, 0], [0, 1]], np.uint16)
    w = np.array([[1, 0], [.5, .5]], np.float32)
    moved = [[1, 0, 0, 2], [0, 1, 0, 0], [0, 0, 1, 0]]
    return v, n, idx, w, np.array([[I, I], [moved, I]], np.float32)


class SkinTests(unittest.TestCase):
    def test_round_trip_and_rebuilt_frames(self):
        with tempfile.TemporaryDirectory() as folder:
            data = sample()
            skin_anims.write(folder, *data)
            for a, b in zip(skin_anims.read(folder), data):
                self.assertTrue(np.array_equal(a, b))
            frames = skin_anims.float_frames(folder)
            np.testing.assert_allclose(frames[1, 1], [1, 1, 0, 0, 0, 1])
            lo, hi = skin_anims.bounds(*data)
            self.assertEqual((lo.tolist(), hi.tolist()), ([0, 0, 0], [2, 1, 0]))
            Path(folder, "meta.json").write_text(json.dumps({"quant": {"min": lo.tolist(), "max": hi.tolist()}}))
            q = np.frombuffer(skin_anims.quantized(folder), dtype=[("p", "<u2", 3), ("n", "i1", 3), ("pad", "i1")]).reshape(2, 2)
            self.assertEqual(q["p"][1, 0].tolist(), [65535, 0, 0])
            self.assertEqual(q["p"][1, 1].tolist(), [32768, 65535, 0])  # half of the move, ties to even
            self.assertEqual(q["n"][0, 0].tolist(), [0, 0, 127])

    def test_corrupt_file_is_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            skin_anims.write(folder, *sample())
            path = Path(folder, "skin.bin")
            path.write_bytes(b"NOTASKIN" + path.read_bytes()[8:])
            with self.assertRaises(ValueError):
                skin_anims.read(folder)

    def test_game_update_drops_only_what_exporters_would_keep(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            for name in ("models/a", "maps/b", "audio", "ui/prefabs"):
                (root / name).mkdir(parents=True)
                (root / name / "x").write_text("x")
            class Context:
                out = root
            import contextlib, io
            with contextlib.redirect_stdout(io.StringIO()):
                driver.drop_old_game_exports(Context)
            self.assertEqual(sorted(p.name for p in root.iterdir()), ["ui"])

    def test_migration_only_drops_vertex_frame_models(self):
        with tempfile.TemporaryDirectory() as folder:
            models = Path(folder, "models")
            old, new, static = models / "Tower/tiers/old", models / "Tower/tiers/new", models / "map_x"
            for d, files in ((old, ["meta.json", "anim_q.bin.wz", "verts.bin"]), (new, ["meta.json", "skin.bin"]), (static, ["meta.json", "idx.bin"])):
                d.mkdir(parents=True)
                for f in files:
                    (d / f).write_text("x")
            class Context:
                out = Path(folder)
            driver.drop_vertex_frame_models(Context)
            self.assertEqual(sorted(p.name for p in old.iterdir()), ["verts.bin"])
            self.assertEqual(sorted(p.name for p in new.iterdir()), ["meta.json", "skin.bin"])
            self.assertEqual(sorted(p.name for p in static.iterdir()), ["idx.bin", "meta.json"])


if __name__ == "__main__":
    unittest.main()
