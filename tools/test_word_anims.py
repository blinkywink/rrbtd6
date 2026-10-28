"""Lossless word prediction tests use synthetic animation samples only."""
import importlib.util
import json
import os
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest.mock import patch
import zlib

import numpy as np
import zstandard as zstd

spec=importlib.util.spec_from_file_location("word_anims",Path(__file__).parent/"assets/word_anims.py")
storage=importlib.util.module_from_spec(spec)
before_path=sys.path[:]
try:spec.loader.exec_module(storage)
finally:sys.path[:]=before_path


def fixture(root,kind=".dz",variant=0,count=400):
    root=Path(root);root.mkdir(parents=True,exist_ok=True)
    data=np.zeros((count,20,5),dtype="<u2")
    for frame in range(count):
        for vertex in range(20):
            data[frame,vertex]=[(frame*frame*(k+1)+vertex*179+variant*977+k*33471)%65536 for k in range(5)]
    raw=data.tobytes();stride=200
    if kind==".dz":
        residual=np.frombuffer(raw,dtype=np.uint8).reshape(-1,stride).copy()
        residual[1:]-=residual[:-1].copy()
        encoded=b"RRDIF001"+struct.pack("<Q",stride)+zlib.compress(residual.tobytes(),6)
    elif kind==".z":encoded=zlib.compress(raw,6)
    else:encoded=raw
    source=root/("anim_q.bin"+kind);source.write_bytes(encoded)
    meta={"vertex_count":20,"quant":{"min":[-1,-2,-3],"max":[1,2,3]},"anims":[{"offset":0,"frames":count}]}
    (root/"meta.json").write_text(json.dumps(meta))
    return source,raw


class WordTests(unittest.TestCase):
    def setUp(self):self.before_path=sys.path[:]
    def tearDown(self):sys.path[:]=self.before_path
    def test_empty_placeholder_models_remain_unchanged(self):
        with tempfile.TemporaryDirectory() as d:
            path=Path(d)/"anim_q.bin";path.write_bytes(b"")
            path.with_name("meta.json").write_text(json.dumps({"vertex_count":0,"quant":{"min":[0]*3,"max":[0]*3},"anims":[{"offset":0,"frames":1}]}))
            self.assertEqual(storage.pack(d),(0,0));self.assertTrue(path.exists())

    def test_interrupted_candidate_pass_resumes_without_encoding(self):
        with tempfile.TemporaryDirectory() as d:
            source,_=fixture(d);storage.pack(d,keep_original=True)
            target=source.with_name("anim_q.bin.wz");original=target.read_bytes()
            with patch.object(storage.zstd,"ZstdCompressor",side_effect=AssertionError("should reuse validated candidate")):
                storage.pack(d,keep_original=True)
            self.assertEqual(target.read_bytes(),original);self.assertTrue(source.exists())

    def test_native_decisions_apply_only_to_unchanged_inputs(self):
        for accepted,change in [(True,False),(False,False),(True,True)]:
            with self.subTest(accepted=accepted,change=change),tempfile.TemporaryDirectory() as d:
                source,raw=fixture(d);original=source.read_bytes()
                storage.pack(d,keep_original=True);target=source.with_name("anim_q.bin.wz")
                def audit(args,**kwargs):
                    Path(args[-1]).write_text(json.dumps([{"directory":"","accepted":accepted,"old_ms":10,"new_ms":5}]))
                    if change:
                        meta=source.with_name("meta.json");meta.write_bytes(meta.read_bytes()+b" ")
                with patch.object(storage.subprocess,"run",side_effect=audit):
                    if change:
                        with self.assertRaises(RuntimeError):storage.finalize(d,source)
                        self.assertEqual(source.read_bytes(),original);self.assertTrue(target.exists())
                    else:
                        result=storage.finalize(d,source)
                        self.assertEqual(result["accepted"],int(accepted))
                        if accepted:
                            self.assertFalse(source.exists());self.assertEqual(storage.read_bytes(target),raw)
                        else:
                            self.assertEqual(source.read_bytes(),original);self.assertFalse(target.exists())

    def test_larger_decoder_window(self):
        with tempfile.TemporaryDirectory() as d:
            source,raw=fixture(d,count=4000)
            storage.pack(d)
            self.assertEqual(storage.read_bytes(source.with_name("anim_q.bin.wz")),raw)

    def test_source_layouts_exact_and_repeat_safe(self):
        for kind in [".dz",".z",""]:
            with self.subTest(kind=kind),tempfile.TemporaryDirectory() as d:
                source,raw=fixture(d,kind);meta=source.with_name("meta.json").read_bytes()
                before,after=storage.pack(d)
                self.assertLess(after,before);self.assertFalse(source.exists())
                output=source.with_name("anim_q.bin.wz")
                self.assertEqual(storage.read_bytes(output),raw)
                self.assertEqual(source.with_name("meta.json").read_bytes(),meta)
                self.assertEqual(storage.pack(d),(0,0))
                spec=importlib.util.spec_from_file_location("pack_read_fixture",Path(__file__).parent/"assets/pack_anims.py")
                helper=importlib.util.module_from_spec(spec);spec.loader.exec_module(helper)
                self.assertEqual(helper.read_bytes(source.with_name("anim_q.bin")),raw)

    def test_failed_validation_keeps_source_and_prior_output(self):
        with tempfile.TemporaryDirectory() as d:
            source,_=fixture(d);original=source.read_bytes();target=source.with_name("anim_q.bin.wz");target.write_bytes(b"prior")
            with patch.object(storage,"restored_frames",return_value=iter([bytes(80000)])):
                with self.assertRaises(ValueError):storage.pack(d)
            self.assertEqual(source.read_bytes(),original);self.assertEqual(target.read_bytes(),b"prior")
            self.assertEqual(list(Path(d).glob("*.pending")),[])

    def test_fresh_export_replaces_stale_packed_variants(self):
        with tempfile.TemporaryDirectory() as d:
            source,old=fixture(d);storage.pack(d)
            fixture(d,".dz",variant=0)
            fresh,new=fixture(d,".z",variant=1)
            self.assertNotEqual(old,new);storage.pack(d)
            self.assertEqual(storage.read_bytes(Path(d)/"anim_q.bin.wz"),new)
            self.assertFalse(fresh.exists());self.assertFalse(source.exists())

    def test_shared_original_is_never_overwritten_in_place(self):
        with tempfile.TemporaryDirectory() as d:
            source,raw=fixture(Path(d)/"a")
            other=Path(d)/"other.dz";os.link(source,other);original=other.read_bytes()
            storage.pack(source.parent)
            self.assertEqual(other.read_bytes(),original)
            self.assertEqual(storage.read_bytes(source.with_name("anim_q.bin.wz")),raw)

    def test_corrupt_header_checksum_or_body_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            source,_=fixture(d);storage.pack(d);path=source.with_name("anim_q.bin.wz");original=path.read_bytes()
            for offset in [0,8,16,24,28,36,len(original)-1]:
                with self.subTest(offset=offset):
                    bad=bytearray(original);bad[offset]^=1;path.write_bytes(bad)
                    with self.assertRaises((ValueError,zstd.ZstdError)):storage.read_bytes(path)
            for bad in [original[:-1],original+b"trailing"]:
                path.write_bytes(bad)
                with self.assertRaises((ValueError,zstd.ZstdError)):storage.read_bytes(path)

    def test_changed_metadata_keeps_source(self):
        with tempfile.TemporaryDirectory() as d:
            source,_=fixture(d);original=source.read_bytes();real=storage.restored_frames
            def frames(path):
                yield from real(path)
                meta=source.with_name("meta.json");meta.write_bytes(meta.read_bytes()+b" ")
            with patch.object(storage,"restored_frames",side_effect=frames):
                with self.assertRaises(RuntimeError):storage.pack(d)
            self.assertEqual(source.read_bytes(),original)
            self.assertEqual(list(Path(d).glob("*.pending")),[])


if __name__=="__main__":unittest.main()
