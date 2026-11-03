"""Progress, batching and achievement-removal checks; no proprietary assets required."""
import importlib.util
import io
import json
import runpy
from pathlib import Path
import subprocess
import sys
import tempfile
import types
import threading
import time
import unittest
from contextlib import redirect_stdout, chdir
from unittest.mock import patch

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))
sys.path.insert(0, str(TOOLS / "assets"))
from extraction_progress import Progress, PREFIX, load_history
from model_batch import chunks, run as run_models, worker_count
from prune_achievements import prune
import export_animated_batch as batch


class ProgressTests(unittest.TestCase):
    def progress(self):
        self.now = 0.
        return Progress(["tower_models", "finish"], emit=lambda *a, **k: None, clock=lambda: self.now)

    def test_percentage_is_work_based_and_never_finishes_on_a_timer(self):
        p = self.progress();p.start("tower_models")
        p.update(2, 4, "Models 2/4")
        before = p.line().split('|')[0]
        self.now = 100000.
        self.assertEqual(p.line().split('|')[0], before)
        p.update(1, 4)
        self.assertEqual(p.line().split('|')[0], before)
        p.complete("tower_models");p.start("finish");p.update(1, 1)
        self.assertNotIn("100.0%", p.line())
        p.finish(False);self.assertNotIn("100.0%", p.line())
        p.finish(True);self.assertIn("100.0%", p.line())

    def test_eta_learns_completed_stage_times_and_saves_only_successful_stages(self):
        p = self.progress();p.start("tower_models");self.now = 120.
        p.complete("tower_models")
        self.assertEqual(p.history["tower_models"], 120.)
        p.start("finish");self.now = 130.;p.finish(False)
        self.assertNotIn("finish", p.history)
        self.assertIn("(estimate)", p.line())

    def test_child_progress_honors_stage_span_and_rejects_bad_records(self):
        p = self.progress();p.start("tower_models")
        self.assertTrue(p.child(PREFIX + json.dumps({"done": 1, "total": 2}), (.1, .8)))
        self.assertAlmostEqual(p.fraction, .5)
        self.assertFalse(p.child(PREFIX + "not json"))
        self.assertFalse(p.child("ordinary exporter output"))

    def test_invalid_history_does_not_break_first_install(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "timings.json";path.write_text("interrupted")
            self.assertEqual(load_history(path), {})
            p = Progress(["tower_models"], {"tower_models": -5}, emit=lambda *a, **k: None)
            self.assertGreater(p.weights["tower_models"], 0)


class BatchTests(unittest.TestCase):
    def test_grouping_is_bounded_deduplicated_by_caller_and_reuses_bundle_order(self):
        jobs = [[f"out/{i}", "bundle-b" if i % 2 else "bundle-a", "prefab"] for i in range(45)]
        with patch("model_batch.complete", return_value=False):
            groups = list(chunks(jobs, 16))
        self.assertEqual(list(map(len, groups)), [16, 16, 13])
        self.assertEqual([job[1] for group in groups for job in group], sorted(job[1] for job in jobs))

    def test_worker_budget_reserves_memory_and_cpu(self):
        self.assertEqual(worker_count(cpu=32, free=32 << 30), 4)
        self.assertEqual(worker_count(cpu=16, free=15 << 30), 3)
        self.assertEqual(worker_count(cpu=2, free=32 << 30), 1)
        self.assertEqual(worker_count(cpu=32, free=3 << 30), 1)
        self.assertEqual(worker_count(cpu=32, free=0), 1)
        self.assertEqual(worker_count(cpu=16,free=15 << 30,per_worker=2 << 30,max_workers=8),6)
        self.assertEqual(worker_count(cpu=32,free=32 << 30,per_worker=2 << 30,max_workers=8),8)
        self.assertEqual(worker_count(cpu=2,free=32 << 30,per_worker=2 << 30,max_workers=8),1)

    def test_parallel_batches_have_monotonic_aggregate_progress(self):
        active, peak = 0, 0
        guard = threading.Lock()
        class Child:
            def __init__(self, *args, **kwargs):
                self.stdout = self.lines()
            def __enter__(self):
                nonlocal active, peak
                with guard:
                    active += 1;peak = max(peak, active)
                return self
            def lines(self):
                for done in (1, 2):
                    time.sleep(.02)
                    yield PREFIX + json.dumps({"done": done, "total": 2})
            def wait(self): return 0
            def __exit__(self, *args):
                nonlocal active
                with guard:active -= 1
        output = io.StringIO()
        jobs = [[f"out/{i}", "bundle", "prefab"] for i in range(8)]
        with patch("model_batch.complete", return_value=False), patch("model_batch.worker_count", return_value=2), patch("model_batch.subprocess.Popen", Child), redirect_stdout(output):
            run_models("game", jobs, size=2)
        progress = [json.loads(line[len(PREFIX):])["done"] for line in output.getvalue().splitlines() if line.startswith(PREFIX)]
        self.assertEqual(peak, 2)
        self.assertEqual(progress, sorted(progress))
        self.assertEqual(progress[-1], 8)

    def test_mixed_bundles_run_in_two_phases_with_one_progress_total(self):
        class Child:
            def __init__(self,*a,**k):self.stdout=[]
            def __enter__(self):return self
            def __exit__(self,*a):pass
            def wait(self):return 0
        jobs=[[f"out/{i}","generated_assets_all" if i>=2 else "asset_references_assets_all","prefab"] for i in range(6)]
        output=io.StringIO()
        with patch("model_batch.complete",return_value=False), patch("model_batch.worker_count",return_value=2) as budget, patch("model_batch.subprocess.Popen",Child), redirect_stdout(output):
            run_models("game",jobs,size=1)
        self.assertEqual(budget.call_args_list,[unittest.mock.call(per_worker=2 << 30,max_workers=8)]*2)
        records=[json.loads(l[len(PREFIX):]) for l in output.getvalue().splitlines() if l.startswith(PREFIX)]
        self.assertEqual({r["total"] for r in records},{6})
        self.assertEqual([r["done"] for r in records],list(range(1,7)))

    def test_conflicting_outputs_are_rejected_before_starting_workers(self):
        with self.assertRaisesRegex(ValueError, "Conflicting model outputs"):
            run_models("game", [["same", "bundle", "a"], ["same", "bundle", "b"]])

    def test_cache_lock_serializes_processes_and_survives_worker_exit(self):
        with tempfile.TemporaryDirectory() as temp:
            code = "import sys,time;sys.path.insert(0,sys.argv[1]);from bundle_lock import cache_lock;from pathlib import Path;counter=Path(sys.argv[2]);lock=sys.argv[3];" + "\nfor _ in range(10):\n with cache_lock(lock):\n  n=int(counter.read_text()) if counter.exists() else 0\n  time.sleep(.002)\n  counter.write_text(str(n+1))"
            cmd = [sys.executable, "-I", "-c", code, str(TOOLS), str(Path(temp)/"count"), str(Path(temp)/"cache.lock")]
            children = [subprocess.Popen(cmd) for _ in range(3)]
            for child in children:self.assertEqual(child.wait(timeout=20), 0)
            self.assertEqual((Path(temp)/"count").read_text(), "30")
            code = "import sys,time;sys.path.insert(0,sys.argv[1]);from bundle_lock import cache_lock;\nwith cache_lock(sys.argv[2]):\n print('locked',flush=True)\n time.sleep(60)"
            child = subprocess.Popen([sys.executable, "-I", "-c", code, str(TOOLS), str(Path(temp)/"cache.lock")], stdout=subprocess.PIPE, text=True)
            try:
                self.assertEqual(child.stdout.readline().strip(), "locked")
            finally:
                child.terminate();child.wait(timeout=10);child.stdout.close()
            from bundle_lock import cache_lock
            with cache_lock(Path(temp)/"cache.lock"):pass

    def test_completed_outputs_do_not_start_a_process(self):
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp);(out / "meta.json").write_text('{"shadow": null}')
            (out / "outline.bin").write_bytes(b"complete")
            with patch("model_batch.subprocess.Popen") as child:
                run_models("game", [[str(out), "bundle", "prefab"]])
            child.assert_not_called()

    def test_float_mode_keeps_raw_animation_for_existing_finish_pass(self):
        with tempfile.TemporaryDirectory() as temp:
            out=Path(temp)
            exporter=types.SimpleNamespace(main=lambda:(out/"anim.bin").write_bytes(b"raw"),em=types.SimpleNamespace(OUT=None,tex_cache={}))
            convert=unittest.mock.Mock(side_effect=AssertionError("unexpected early quantization"))
            with patch.dict(sys.modules,{"export_animated":exporter,"compact_anims":types.SimpleNamespace(convert=convert)}), patch.object(sys,"argv",["fixture"]), redirect_stdout(io.StringIO()):
                batch.run("game",[[str(out),"bundle","prefab"]],compact=False)
            self.assertEqual((out/"anim.bin").read_bytes(),b"raw")
            convert.assert_not_called()

    def test_failed_bake_removes_completion_marker_and_propagates(self):
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp)
            def fail():
                (out / "meta.json").write_text('{}')
                raise RuntimeError("bake interrupted")
            exporter = types.SimpleNamespace(main=fail, em=types.SimpleNamespace(OUT=None, tex_cache={}))
            with patch.dict(sys.modules, {"export_animated": exporter, "compact_anims": types.SimpleNamespace(convert=lambda _: None)}), patch.object(sys, "argv", ["fixture"]):
                with self.assertRaisesRegex(RuntimeError, "interrupted"):
                    batch.run("game", [[str(out), "bundle", "prefab"]])
            self.assertFalse((out / "meta.json").exists())


class ExportCacheTests(unittest.TestCase):
    def test_prefab_lookup_decodes_once_and_preserves_first_matching_object(self):
        from export_cache import PrefabIndex
        reads = []
        bundle = types.SimpleNamespace(type=types.SimpleNamespace(name="AssetBundle"), path_id=1,
            read_typetree=lambda: reads.append(1) or {"m_Container": [("a", {"asset":{"m_PathID":2}}), ("b", {"asset":{"m_PathID":3}})]})
        a = types.SimpleNamespace(type=types.SimpleNamespace(name="GameObject"), path_id=2)
        b = types.SimpleNamespace(type=types.SimpleNamespace(name="GameObject"), path_id=3)
        duplicate = types.SimpleNamespace(type=a.type, path_id=2)
        env = types.SimpleNamespace(objects=[bundle, a, b, duplicate])
        cache = PrefabIndex()
        self.assertIs(cache.root(env, "a"), a)
        self.assertIs(cache.root(env, "b"), b)
        self.assertIs(cache.root(env, "a"), a)
        self.assertEqual(reads, [1])

    def test_clip_annotations_are_private_and_decode_is_reused(self):
        from export_cache import ClipCopies
        reads = []
        obj = types.SimpleNamespace(assets_file=types.SimpleNamespace(name="cab"), path_id=7,
            read_typetree=lambda: reads.append(1) or {})
        cache = ClipCopies(limit=1)
        decode = lambda _: types.SimpleNamespace(keys=(1,2,3))
        first = cache.get(obj,decode);first.role = "place";first.weight = 2
        second = cache.get(obj,decode)
        self.assertIsNot(first,second)
        self.assertFalse(hasattr(second,"role"))
        self.assertFalse(hasattr(second,"weight"))
        self.assertEqual(second.keys,first.keys)
        self.assertEqual(reads,[1])
        other = types.SimpleNamespace(assets_file=types.SimpleNamespace(name="another"), path_id=7, read_typetree=obj.read_typetree)
        cache.get(other,decode);cache.get(obj,decode)
        self.assertEqual(len(reads),3)
        self.assertEqual(len(cache.entries),1)

    def test_texture_copies_reuse_exact_bytes_and_keep_separate_output_files(self):
        from export_cache import TextureCopies
        reads = []
        def read():
            reads.append(1)
            return types.SimpleNamespace(m_Name="skin", image=types.SimpleNamespace(save=lambda stream, **kw: stream.write(b"png-bytes")))
        obj = types.SimpleNamespace(assets_file=types.SimpleNamespace(name="cab"), path_id=7, read=read)
        with tempfile.TemporaryDirectory() as temp:
            cache = TextureCopies(limit=10)
            a,b = Path(temp)/"a",Path(temp)/"b"
            self.assertEqual(cache.write(obj,a), "tex/skin_7.png")
            self.assertEqual(cache.write(obj,b), "tex/skin_7.png")
            self.assertEqual(reads,[1])
            self.assertEqual((a/"tex/skin_7.png").read_bytes(),(b/"tex/skin_7.png").read_bytes())
            (b/"tex/skin_7.png").write_bytes(b"edited")
            self.assertEqual((a/"tex/skin_7.png").read_bytes(),b"png-bytes")
            other = types.SimpleNamespace(assets_file=types.SimpleNamespace(name="different-cab"), path_id=7, read=read)
            cache.write(other,b)
            self.assertEqual(len(reads),2)
            self.assertLessEqual(cache.size,10)
            cache.write(obj,b)
            self.assertEqual(len(reads),3)
            uncached = TextureCopies(limit=1)
            uncached.write(obj,b);uncached.write(obj,b)
            self.assertEqual(uncached.size,0)
            self.assertEqual(len(reads),5)


class ArtworkBatchTests(unittest.TestCase):
    def test_batch_reuses_bundles_preserves_overwrites_and_numeric_names(self):
        loads = []
        class Image:
            size = (4,4)
            def __init__(self, value):self.value = value
            def save(self, path):Path(path).write_bytes(self.value)
        paths = ["tower/100UpgradeIcon.png", "first/shared.png", "second/shared.png", "third/unique.png", "tower/Pouākai.png", "tower/500-SkyWarden.png", "tower/Hero.png", "tower/HeroLv10.png"]
        index = {paths[0]:"a",paths[1]:"a",paths[2]:"b",paths[3]:"c",paths[4]:"b",paths[5]:"b",paths[6]:"b",paths[7]:"b"}
        def load(path):
            name = Path(path).name;loads.append(name)
            container = {p:types.SimpleNamespace(read=lambda p=p: types.SimpleNamespace(image=Image(p.encode()))) for p,b in index.items() if b==name}
            return types.SimpleNamespace(container=container)
        spec = importlib.util.spec_from_file_location("test_art_export",TOOLS/"assets/export_textures.py")
        module = importlib.util.module_from_spec(spec)
        with patch.dict(sys.modules,{"UnityPy":types.SimpleNamespace(load=load),"export_sprites":types.SimpleNamespace(full_rect=lambda obj:obj.image)}):
            spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as temp, redirect_stdout(io.StringIO()):
            jobs=[{"out":temp,"paths":[paths[0],paths[1]],"prefix_numeric":"Tower"},
                  {"out":temp,"paths":[paths[2]]},{"out":temp,"paths":[paths[1]]},
                  {"out":temp,"paths":[paths[3]]},{"out":temp,"paths":[paths[2],paths[4]],"ascii_names":True},
                  {"out":temp,"paths":[paths[5]],"portrait_tower":"Skywarden"},
                  {"out":str(Path(temp)/"hero"),"paths":[paths[6],paths[7]],"rename_hero":"Hero"},
                  {"out":str(Path(temp)/"alias"),"paths":[paths[2]],"name_prefix":"Tower__"}]
            module.export_jobs("game",jobs,index=index)
            self.assertEqual(loads,["a","b","c","b"])
            self.assertEqual((Path(temp)/"Tower100UpgradeIcon.png").read_bytes(),paths[0].encode())
            self.assertFalse((Path(temp)/"100UpgradeIcon.png").exists())
            self.assertEqual((Path(temp)/"shared.png").read_bytes(),paths[2].encode())
            self.assertEqual((Path(temp)/"Pouakai.png").read_bytes(),paths[4].encode())
            self.assertEqual((Path(temp)/"500-Skywarden.png").read_bytes(),paths[5].encode())
            self.assertNotIn("500-SkyWarden.png",[file.name for file in Path(temp).iterdir()])
            self.assertEqual((Path(temp)/"hero/HeroPortrait.png").read_bytes(),paths[6].encode())
            self.assertEqual((Path(temp)/"hero/HeroPortraitLvl10.png").read_bytes(),paths[7].encode())
            self.assertEqual((Path(temp)/"alias/Tower__shared.png").read_bytes(),paths[2].encode())


    def test_extended_art_batches_without_requiring_old_output_directories(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)/"local_extracted"
            (root/"display_plans").mkdir(parents=True)
            (root/"models/Desperado/tiers").mkdir(parents=True)
            (root/"game").mkdir()
            (root/"game/game.json").write_text('{"tower_tiers":{"Desperado":{}}}')
            (root/"display_plans/Desperado.json").write_text('{"towers":{}}')
            (root/"all_container_paths.json").write_text(json.dumps({
                "Assets/ResizedImages/UI/UpgradeIcons/Desperado/100UpgradeIcon.png":"bundle",
                "Assets/ResizedImages/MonkeyPortraits/Desperado/100-Desperado.png":"bundle"}))
            manifests=[]
            class Child:
                def __init__(self,cmd,**kwargs):
                    self.stdout=io.StringIO("ok\n")
                    if "--jobs" in cmd:manifests.append(json.loads(Path(cmd[-1]).read_text()))
                def __enter__(self):return self
                def __exit__(self,*args):self.stdout.close()
                def wait(self):return 0
            with chdir(temp), patch.object(sys,"argv",["export_tower_assets.py","game","Desperado"]), patch("subprocess.Popen",Child), redirect_stdout(io.StringIO()):
                runpy.run_path(str(TOOLS/"assets/export_tower_assets.py"),run_name="__main__")
            self.assertEqual(len(manifests),1)
            self.assertEqual(manifests[0][0]["prefix_numeric"],"Desperado")
            self.assertTrue(manifests[0][0]["ascii_names"])
            self.assertEqual(manifests[0][1]["portrait_tower"],"Desperado")
            self.assertFalse((root/".extended_art_jobs.json").exists())


class MapPairTests(unittest.TestCase):
    def test_map_and_geometry_share_the_scene_and_preserve_order(self):
        scene=object();calls=[]
        def data():
            calls.append(("data",list(sys.argv)))
            return scene
        def model(env):
            self.assertIs(env,scene);calls.append(("model",list(sys.argv)))
        with patch.dict(sys.modules,{"export_map":types.SimpleNamespace(main=data),"export_model":types.SimpleNamespace(main=model)}), patch.object(sys,"argv",["pair","game","logs","maps-out","model-out"]):
            runpy.run_path(str(TOOLS/"assets/export_map_pair.py"),run_name="__main__")
        self.assertEqual([c[0] for c in calls],["data","model"])
        self.assertEqual(calls[0][1][1:],["game","logs","maps-out"])
        self.assertEqual(calls[1][1][1:],["game","model-out","scene","map_logs"])


class ProjectileExportTests(unittest.TestCase):
    def test_null_texture_resolves_by_key_and_real_errors_are_not_hidden(self):
        from sprite_resolver import SpriteImages
        class Late:
            m_RenderDataKey=("right-guid",12)
            m_RD=types.SimpleNamespace(texture=types.SimpleNamespace(m_PathID=0))
            @property
            def image(self):raise ValueError("null local texture")
        wrong=types.SimpleNamespace(m_RenderDataKey=("wrong-guid",12),image="wrong")
        right=types.SimpleNamespace(m_RenderDataKey=("right-guid",12),image="exact atlas sprite")
        env=types.SimpleNamespace(objects=[types.SimpleNamespace(type=types.SimpleNamespace(name="Sprite"),read=lambda s=s:s) for s in [wrong,right]])
        resolver=SpriteImages("bundles")
        with patch.dict(sys.modules,{"UnityPy":types.SimpleNamespace(load=lambda _:env)}), patch("sprite_resolver.glob.glob",return_value=["atlas.bundle"]):
            self.assertEqual(resolver.image(Late()),"exact atlas sprite")
            self.assertEqual(resolver.image(Late()),"exact atlas sprite")
        broken=Late();broken.m_RD=types.SimpleNamespace(texture=types.SimpleNamespace(m_PathID=7))
        with self.assertRaises(ValueError):resolver.image(broken)
        missing=Late();missing.m_RenderDataKey=("missing",99)
        with self.assertRaises(KeyError):resolver.image(missing)
        self.assertEqual(resolver.image(types.SimpleNamespace(image="direct")),"direct")

    def test_partial_projectile_is_retried_and_commit_is_atomic(self):
        import export_checkpoint as checkpoint
        with tempfile.TemporaryDirectory() as temp:
            folder=Path(temp);(folder/"sprite.json").write_text("{}")
            self.assertFalse(checkpoint.complete(folder,"source1"))
            checkpoint.commit(folder,"source1")
            self.assertTrue(checkpoint.complete(folder,"source1"))
            self.assertFalse(checkpoint.complete(folder,"source2"))
            checkpoint.begin(folder)
            self.assertFalse(checkpoint.complete(folder,"source1"))
            with patch("export_checkpoint.os.replace",side_effect=OSError("interrupted")):
                with self.assertRaises(OSError):checkpoint.commit(folder,"source1")
            self.assertFalse(checkpoint.complete(folder,"source1"))
            self.assertEqual(sorted(p.name for p in folder.iterdir()),["sprite.json"])


class StaticMeshReuseTests(unittest.TestCase):
    def test_mesh_state_is_reset_and_arguments_restore_on_failure(self):
        import static_model_batch as batcher
        fake=types.SimpleNamespace(_envs={},tex_cache={},parts=[],verts=[],idxs=[],vnormals=[],vcolors=[])
        scene=object();seen=[]
        def main(env):
            self.assertIs(env,scene)
            self.assertEqual(fake.parts,[]);self.assertEqual(fake.verts,[])
            seen.append(fake.OUT);fake.parts.append("part");fake.verts.append("vertices")
            if fake.OUT=="fail":raise RuntimeError("bad mesh")
        fake.main=main
        original=sys.argv
        with patch.object(batcher,"_model",None), patch.dict(sys.modules,{"export_model":fake}):
            self.assertEqual(batcher.export("game","one","bundle_hash.bundle","prefab",scene),"mesh 1 parts")
            self.assertEqual(batcher.export("game","two","bundle_hash.bundle","prefab",scene),"mesh 1 parts")
            with self.assertRaisesRegex(RuntimeError,"bad mesh"):batcher.export("game","fail","bundle_hash.bundle","prefab",scene)
            self.assertIs(sys.argv,original)
            batcher.release();self.assertFalse(fake.parts);self.assertFalse(fake.verts)
        self.assertEqual(seen,["one","two","fail"])


class ExtendedModelQueueTests(unittest.TestCase):
    def test_indexes_publish_only_after_model_success_and_failed_manifest_is_removed(self):
        for fail in [False,True]:
            with tempfile.TemporaryDirectory() as temp:
                root=Path(temp)/"local_extracted"
                (root/"display_plans").mkdir(parents=True);(root/"models/Desperado/tiers").mkdir(parents=True);(root/"game").mkdir()
                (root/"game/game.json").write_text('{"tower_tiers":{"Desperado":{"paragon":{"display":"TestPG"}}}}')
                (root/"display_plans/Desperado.json").write_text('{"towers":{}}')
                (root/"all_container_paths.json").write_text('{"Assets/Monkeys/Desperado/Graphics/TestPG.prefab":"bundle_hash.bundle"}')
                index=root/"models/Desperado/tiers/index.json";index.write_text('{"000":"old"}')
                seen=[]
                class Child:
                    def __init__(self,cmd,**kwargs):
                        self.stdout=io.StringIO("ok\n");self.model="tools/assets/model_batch.py" in cmd
                        if self.model:
                            seen.append(cmd);self.assert_flags="--keep-floats" in cmd
                            assert json.loads(index.read_text())=={"000":"old"}
                    def __enter__(self):return self
                    def __exit__(self,*a):self.stdout.close()
                    def wait(self):return 7 if self.model and fail else 0
                with chdir(temp), patch.object(sys,"argv",["export_tower_assets.py","game","Desperado"]), patch("subprocess.Popen",Child), redirect_stdout(io.StringIO()):
                    if fail:
                        with self.assertRaises(subprocess.CalledProcessError):runpy.run_path(str(TOOLS/"assets/export_tower_assets.py"),run_name="__main__")
                    else:runpy.run_path(str(TOOLS/"assets/export_tower_assets.py"),run_name="__main__")
                self.assertEqual(len(seen),1);self.assertIn("--keep-floats",seen[0])
                self.assertFalse((root/".extended_model_jobs.json").exists())
                self.assertEqual(json.loads(index.read_text()),{"000":"old"} if fail else {"000":"old","paragon":"TestPG"})


class RemovalTests(unittest.TestCase):
    def test_achievement_cleanup_preserves_shared_icons_and_local_profile(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp);(root / "ui/runtime").mkdir(parents=True);(root / "ui/menu_icons").mkdir()
            (root / "ui/runtime/achievements.json").write_text(json.dumps([{"icon": "only.png", "hidden_icon": "shared.png"}]))
            (root / "ui/runtime/trophy_items.json").write_text('[{"icon":"shared.png"}]')
            (root / "ui/menu_icons/only.png").write_bytes(b"unused")
            (root / "ui/menu_icons/shared.png").write_bytes(b"needed")
            (root / "local-profile.json").write_text("local save")
            with redirect_stdout(io.StringIO()):prune(root)
            self.assertFalse((root / "ui/runtime/achievements.json").exists())
            self.assertFalse((root / "ui/menu_icons/only.png").exists())
            self.assertEqual((root / "ui/menu_icons/shared.png").read_bytes(), b"needed")
            self.assertEqual((root / "local-profile.json").read_text(), "local save")


if __name__ == "__main__":
    unittest.main()
