import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

def module(name):
    spec=importlib.util.spec_from_file_location(name,Path(__file__).resolve().parents[1].joinpath(name+'.py'))
    value=importlib.util.module_from_spec(spec);spec.loader.exec_module(value);return value

model=module('mod_model')
sdk=module('modsdk')

class ModelTests(unittest.TestCase):
    def test_starter_has_native_animated_geometry_and_socket_wired_manifest(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)/'throwbot';model.template(root)
            meta=json.loads((root/'assets/robot/meta.json').read_text())
            self.assertEqual(meta['anims'][1]['name'],'RobotAttack')
            self.assertEqual(meta['mod_socket']['muzzle'],[4,8,6])
            self.assertEqual((root/'assets/robot/anim.bin').stat().st_size,sum(a['frames'] for a in meta['anims'])*meta['vertex_count']*24)
            sdk.Mod.author_directory=root
            mod=sdk.Mod('throwbot','Test');mod.tower('robot').model('robot')
            edit=mod.manifest['content'][1]
            self.assertEqual(edit['op'],'tower_model');self.assertEqual(edit['asset'],'mod:throwbot/robot')
            self.assertEqual(edit['release'],.15)
            with self.assertRaises(ValueError):mod.model('../secret')

    def test_obj_import_handles_uv_seams_triangulation_and_negative_indices(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'mesh.obj'
            path.write_text('v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nf -4/1 -3/1 -2/1 -1/1\n')
            vertices,indices=model.obj(path)
            self.assertEqual(indices,[0,1,2,0,2,3]);self.assertEqual(len(vertices),4)
            path.write_text('v 0 0 0\nf 0 1 1\n')
            with self.assertRaises(ValueError):model.obj(path)

    def test_bad_pose_or_socket_fails_before_writing_output(self):
        with tempfile.TemporaryDirectory() as temp:
            out=Path(temp)/'output';vertices=[(0,0,0,0,0)]
            for clips,kwargs in [([('Attack',[],60,False)],{}),([('Attack',[[(0,0,0)]],60,False)],{'release':float('nan')})]:
                with self.assertRaises(ValueError):model.bake(out,vertices,[0,0,0],clips,**kwargs)
                self.assertFalse(out.exists())

if __name__=='__main__':unittest.main()
