"""Recompression preserves packed animation semantics and failure safety."""
import importlib.util
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest.mock import patch
import zlib

spec=importlib.util.spec_from_file_location("repack_anims",Path(__file__).parent/"assets/repack_anims.py")
storage=importlib.util.module_from_spec(spec)
before_path=sys.path[:]
try:spec.loader.exec_module(storage)
finally:sys.path[:]=before_path


def fixture(root,name="anim_q.bin.dz"):
    path=Path(root)/name
    stride=100 if name.startswith("anim_q") else 240
    data=bytes(range(256))*(stride*2000//256)
    data=data[:len(data)//stride*stride]
    header=storage.MAGIC+struct.pack("<Q",stride) if name.endswith(".dz") else b""
    path.write_bytes(header+zlib.compress(data,1))
    meta={"vertex_count":10,"anims":[{"offset":0,"frames":len(data)//stride}]}
    path.with_name("meta.json").write_text(json.dumps(meta))
    return path,data,header


class RepackTests(unittest.TestCase):
    def test_all_formats_exact_and_idempotent(self):
        for name in storage.NAMES:
            with self.subTest(name=name),tempfile.TemporaryDirectory() as d:
                path,data,header=fixture(d,name)
                meta=path.with_name("meta.json").read_bytes()
                before,after=storage.repack(path)
                self.assertLess(after,before)
                encoded=path.read_bytes()
                self.assertEqual(encoded[:len(header)],header)
                self.assertEqual(zlib.decompress(encoded[len(header):]),data)
                self.assertEqual(path.with_name("meta.json").read_bytes(),meta)
                self.assertEqual(storage.repack(path),(after,after))
                self.assertEqual(list(Path(d).glob("*.pending")),[])

    def test_corrupt_stream_and_bad_dimensions_preserve_source(self):
        for mode in ["truncated","trailing","wrong-stride","wrong-size"]:
            with self.subTest(mode=mode),tempfile.TemporaryDirectory() as d:
                path,_,_=fixture(d)
                original=path.read_bytes()
                if mode=="truncated":path.write_bytes(original[:-1])
                elif mode=="trailing":path.write_bytes(original+b"trailing")
                elif mode=="wrong-stride":path.write_bytes(original[:8]+struct.pack("<Q",101)+original[16:])
                else:path.with_name("meta.json").write_text(json.dumps({"vertex_count":10,"anims":[{"offset":0,"frames":1}]}))
                original=path.read_bytes()
                with self.assertRaises((ValueError,zlib.error)):storage.repack(path)
                self.assertEqual(path.read_bytes(),original)
                self.assertEqual(list(Path(d).glob("*.pending")),[])

    def test_concurrent_metadata_change_keeps_original(self):
        with tempfile.TemporaryDirectory() as d:
            path,_,_=fixture(d);original=path.read_bytes();real=storage.chunks
            def chunks(source,skip):
                yield from real(source,skip)
                if source!=path:
                    meta=path.with_name("meta.json");meta.write_bytes(meta.read_bytes()+b" ")
            with patch.object(storage,"chunks",side_effect=chunks):
                with self.assertRaises(RuntimeError):storage.repack(path)
            self.assertEqual(path.read_bytes(),original)
            self.assertEqual(list(Path(d).glob("*.pending")),[])


if __name__=="__main__":unittest.main()
