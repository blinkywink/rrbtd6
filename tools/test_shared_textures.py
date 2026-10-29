"""Model texture sharing: synthetic bytes and metadata only."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec=importlib.util.spec_from_file_location("share_model_textures",Path(__file__).parent/"assets/share_model_textures.py")
storage=importlib.util.module_from_spec(spec);spec.loader.exec_module(storage)


def model(root,name,content=b"identical png bytes",shader=False):
    directory=Path(root)/name; (directory/"tex").mkdir(parents=True)
    texture=directory/"tex"/(name+".png");texture.write_bytes(content)
    part={"name":name+".png","texture":"tex/"+texture.name,"voff":13}
    if shader:
        part["props"]={"textures":{"_Noise":{"file":"tex/"+texture.name,"scale":[2,3]}}}
    meta={"parts":[part],"quant":{"min":[0,1,2]},"anims":[{"frames":8}]}
    path=directory/"meta.json";path.write_text(json.dumps(meta))
    (directory/"shadow.png").write_bytes(b"implicit shadow")
    return path,texture,meta


class TextureTests(unittest.TestCase):
    def test_sharing_preserves_every_texture_byte_and_other_fields(self):
        with tempfile.TemporaryDirectory() as d:
            a,ta,ma=model(d,"a",shader=True);b,tb,mb=model(d,"b")
            result=storage.compact(d)
            self.assertEqual(result["metadata_updated"],2)
            self.assertEqual(result["net_bytes_saved"],len(b"identical png bytes"))
            self.assertFalse(ta.exists());self.assertFalse(tb.exists())
            for path,original in [(a,ma),(b,mb)]:
                actual=json.loads(path.read_bytes())
                for obj,key in storage.slots(actual):
                    self.assertEqual((path.parent/obj[key]).read_bytes(),b"identical png bytes")
                self.assertEqual(actual["quant"],original["quant"])
                self.assertEqual(actual["anims"],original["anims"])
                self.assertEqual(actual["parts"][0]["name"],original["parts"][0]["name"])
                self.assertEqual((path.parent/"shadow.png").read_bytes(),b"implicit shadow")
            self.assertEqual(storage.compact(d)["net_bytes_saved"],0)

    def test_equal_size_different_bytes_not_shared(self):
        with tempfile.TemporaryDirectory() as d:
            _,ta,_=model(d,"a",b"abc");_,tb,_=model(d,"b",b"def")
            self.assertEqual(storage.compact(d)["net_bytes_saved"],0)
            self.assertTrue(ta.exists() and tb.exists())

    def test_failed_metadata_write_keeps_originals_and_references_readable(self):
        with tempfile.TemporaryDirectory() as d:
            a,ta,_=model(d,"a");b,tb,_=model(d,"b")
            real=storage.atomic_write
            def write(path,data):
                if path.resolve()==b.resolve():raise OSError("simulated disk failure")
                return real(path,data)
            with patch.object(storage,"atomic_write",side_effect=write):
                with self.assertRaises(OSError):storage.compact(d)
            self.assertTrue(ta.exists() and tb.exists())
            for path in [a,b]:
                for obj,key in storage.slots(json.loads(path.read_bytes())):
                    self.assertTrue((path.parent/obj[key]).is_file())
            storage.compact(d)
            self.assertFalse(tb.exists())

    def test_fresh_reexport_reuses_existing_shared_file(self):
        with tempfile.TemporaryDirectory() as d:
            a,_,_=model(d,"a");model(d,"b");storage.compact(d)
            old=json.loads(a.read_bytes())["parts"][0]["texture"]
            tex=a.parent/"tex/a.png";tex.write_bytes(b"identical png bytes")
            meta=json.loads(a.read_bytes());meta["parts"][0]["texture"]="tex/a.png";a.write_text(json.dumps(meta))
            result=storage.compact(d)
            self.assertEqual(result["shared_bytes_created"],0)
            self.assertFalse(tex.exists())
            self.assertEqual(json.loads(a.read_bytes())["parts"][0]["texture"],old)

    def test_path_escape_and_corrupt_shared_file_keep_sources(self):
        with tempfile.TemporaryDirectory() as d:
            a,ta,ma=model(d,"a");_,tb,_=model(d,"b")
            ma["parts"][0]["texture"]="../../outside.png";a.write_text(json.dumps(ma))
            with self.assertRaises(ValueError):storage.compact(d)
            self.assertTrue(ta.exists() and tb.exists())
            ma["parts"][0]["texture"]="tex/a.png";a.write_text(json.dumps(ma))
            shared=Path(d)/storage.SHARED;shared.mkdir()
            (shared/(storage.digest(ta)+".png")).write_bytes(b"corrupt")
            with self.assertRaises(ValueError):storage.compact(d)
            self.assertTrue(ta.exists() and tb.exists())


if __name__=="__main__":unittest.main()
