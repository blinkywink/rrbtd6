"""Writes states.json next to each exported tower model: the clip names of its
MonkeyAnimationController.animationStatesByIndex, in order. Weapon.SetAnimation(n) (VERIFIED-DECOMP
@0x180c51220) picks state n; AlternateAnimation.ProjectilesCreated (@0x180c68ff0) sets
emissionCount % states + originState + 1, Weapon.Process (@0x180c50670) weaponModel.animation.

  python -I tools/assets/export_anim_states.py <game dir> [model folder filter]
"""
import glob
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import UnityPy  # noqa: E402
from export_cache import PrefabIndex  # noqa: E402

GAME = sys.argv[1]
ONLY = sys.argv[2] if len(sys.argv) > 2 else None
_argv = sys.argv
sys.argv = [sys.argv[0], GAME, "local_extracted", "prefab", ""]  # export_model reads these on import
import export_model as em  # noqa: E402
sys.argv = _argv
L = Path("local_extracted")
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")


def resolve(af, ptr):
    """An object pointer, external files included (export_model.resolve, as the model export)."""
    if not ptr or not ptr.get("m_PathID"):
        return None
    try:
        return em.resolve(af, ptr)
    except MemoryError:
        return None


def overrides_of(af, go_list):
    """The prefab's AnimatorOverrideController: original clip path id -> override clip, as
    export_animated.py applies it to the exported clips."""
    for go in go_list:
        for c in go["m_Component"]:
            o = af.objects.get(c["component"]["m_PathID"])
            if o is None or o.type.name != "Animator":
                continue
            ctrl = resolve(af, o.read_typetree().get("m_Controller"))
            if ctrl is None:
                continue
            t = ctrl.read_typetree()
            if "m_Clips" not in t:
                return {}
            out = {}
            for pair in t["m_Clips"]:
                orig = resolve(ctrl.assets_file, pair["m_OriginalClip"])
                over = resolve(ctrl.assets_file, pair["m_OverrideClip"])
                if orig is not None and over is not None:
                    out[orig.path_id] = over
            return out
    return {}


def all_gos(root):
    af = root.assets_file
    out, stack = [], [root.read_typetree()]
    while stack:
        go = stack.pop()
        out.append(go)
        for c in go["m_Component"]:
            o = af.objects.get(c["component"]["m_PathID"])
            if o is not None and o.type.name == "Transform":
                for ch in o.read_typetree()["m_Children"]:
                    to = af.objects.get(ch["m_PathID"])
                    if to is not None:
                        stack.append(af.objects[to.read_typetree()["m_GameObject"]["m_PathID"]].read_typetree())
    return out


def states_of(root):
    af = root.assets_file
    gos = all_gos(root)
    over = overrides_of(af, gos)
    for go in gos:
        for c in go["m_Component"]:
            o = af.objects.get(c["component"]["m_PathID"])
            if o is None or o.type.name != "MonoBehaviour":
                continue
            t = o.read_typetree()
            if "animationStates" in t and "placementConfigs" in t:
                # MonkeyAnimationController.WriteHighQualityAnimationIndices (VERIFIED-DECOMP
                # @0x1815272b0): animationStatesByIndex is animationStates, then placementConfigs,
                # attackIdleConfigs and IdleVariants; SetState(n) plays entry n (@0x1815264c0).
                out = []
                for s in [x for k in ("animationStates", "placementConfigs", "attackIdleConfigs", "IdleVariants") for x in t.get(k, [])]:
                    co = resolve(af, s.get("animationClip"))
                    if co is not None:
                        co = over.get(co.path_id, co)
                    # (A clip that can't be resolved: None; the renderer then falls back to names.)
                    out.append(co.read_typetree()["m_Name"] if co is not None else None)
                return out
    return None


def main():
    paths = json.loads((L / "all_container_paths.json").read_text(encoding="utf-8"))
    by_name = {}
    for p in paths:
        if p.endswith(".prefab"):
            by_name.setdefault(p.rsplit("/", 1)[-1][:-7].strip().lower(), []).append(p)
    jobs = {}  # bundle -> [(folder, container path)]
    for meta in glob.glob(str(L / "models" / "**" / "meta.json"), recursive=True):
        folder = Path(meta).parent
        if ONLY and ONLY not in str(folder):
            continue
        cand = sorted(by_name.get(folder.name.strip().lower(), []))
        if not cand:
            continue
        bundle = paths[cand[0]].rsplit("_", 1)[0]
        jobs.setdefault(bundle, []).append((folder, cand[0]))
    pi = PrefabIndex()
    n = 0
    for bundle, items in sorted(jobs.items()):
        files = glob.glob(os.path.join(AA, bundle + "*.bundle"))
        if not files:
            continue
        # (asset_references_assets_all is ~900 MB packed: its decompressed files in
        # local_extracted/_bundle_cache when there, as export_projectiles.load_bundle.)
        cached = [d for d in sorted(glob.glob(str(L / "_bundle_cache" / (glob.escape(os.path.basename(files[0])) + "-*")))) if os.path.isdir(d)]
        try:
            env = UnityPy.load(*[os.path.join(cached[0], f) for f in sorted(os.listdir(cached[0]))]) if cached else UnityPy.load(files[0])
        except MemoryError:
            print("out of memory:", bundle)
            continue
        em._envs[os.path.basename(files[0])] = env
        for folder, cpath in items:
            try:
                st = states_of(pi.root(env, cpath))
            except Exception as e:  # a prefab that isn't in this bundle's container
                print("skip", folder, e)
                continue
            if st is not None:
                (folder / "states.json").write_text(json.dumps(st), encoding="utf-8")
                n += 1
        pi.bundles.clear()
        em._envs.clear()
    print(n, "states.json written")


if __name__ == "__main__":
    main()
