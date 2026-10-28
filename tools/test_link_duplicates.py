"""Duplicate linking keeps every path and byte, and --split undoes it."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

TOOL = Path(__file__).parent / "assets/link_duplicates.py"


def run(*args):
    return subprocess.run([sys.executable, "-I", str(TOOL), *map(str, args)], check=True, capture_output=True, text=True).stdout


def inode(path):
    st = os.stat(path)
    return st.st_dev, st.st_ino


class LinkDuplicatesTests(unittest.TestCase):
    def fixture(self, root):
        same = os.urandom(10000)
        for tier in ("a", "b", "c"):
            d = Path(root, "models", "Tower", tier)
            d.mkdir(parents=True)
            (d / "anim_q.bin.dz").write_bytes(same)
            (d / "meta.json").write_bytes(b"{" + b" " * 5000 + b"}")
            (d / "own.png").write_bytes(os.urandom(5000))
        return same

    def test_links_identical_assets_and_keeps_bytes(self):
        with tempfile.TemporaryDirectory() as root:
            same = self.fixture(root)
            run(root)
            anims = sorted(Path(root).rglob("anim_q.bin.dz"))
            self.assertEqual(len({inode(p) for p in anims}), 1)
            self.assertTrue(all(p.read_bytes() == same for p in anims))
            metas = sorted(Path(root).rglob("meta.json"))
            self.assertEqual(len({inode(p) for p in metas}), 3)
            pngs = sorted(Path(root).rglob("own.png"))
            self.assertEqual(len({inode(p) for p in pngs}), 3)
            self.assertIn("linked 0", run(root))
            self.assertEqual(list(Path(root).rglob("*.pending")), [])

    def test_split_gives_each_path_its_own_copy(self):
        with tempfile.TemporaryDirectory() as root:
            same = self.fixture(root)
            run(root)
            run("--split", root)
            anims = sorted(Path(root).rglob("anim_q.bin.dz"))
            self.assertEqual(len({inode(p) for p in anims}), 3)
            anims[0].write_bytes(b"changed")
            self.assertTrue(all(p.read_bytes() == same for p in anims[1:]))
            self.assertEqual(list(Path(root).rglob("*.pending")), [])


if __name__ == "__main__":
    unittest.main()
