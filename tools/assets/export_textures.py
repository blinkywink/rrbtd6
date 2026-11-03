"""Export individual textures/sprites by Addressables container path.

Usage:
    tools/.venv/Scripts/python.exe tools/assets/export_textures.py <game install> <out_dir> <container path>...

Bundles are looked up in local_extracted/all_container_paths.json. Each asset is
written to <out_dir>/<file stem>.png. Proprietary art: local use only.
"""
import json
import os
import sys
from collections import defaultdict, OrderedDict

import UnityPy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from export_sprites import full_rect  # noqa: E402

def export_jobs(game, jobs, index=None):
    """Keep output order, sharing at most two loaded bundle/container pairs."""
    aa = os.path.join(game, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
    if index is None:
        with open("local_extracted/all_container_paths.json", encoding="utf-8") as stream:
            index = json.load(stream)
    loaded = OrderedDict()
    for i, job in enumerate(jobs):
        out, paths = job["out"], job["paths"]
        by_bundle = defaultdict(list)
        for path in paths:
            if path not in index:
                sys.exit(f"not in container index: {path}")
            by_bundle[index[path]].append(path)
        os.makedirs(out, exist_ok=True)
        for bundle, paths in by_bundle.items():
            if bundle in loaded:
                env, containers = loaded.pop(bundle)
            else:
                if len(loaded) >= 2:
                    loaded.popitem(last=False)
                    import gc
                    gc.collect()
                env = UnityPy.load(os.path.join(aa, bundle))
                containers = dict(env.container.items())
            loaded[bundle] = env, containers
            for path in paths:
                obj = containers[path].read()
                name = os.path.splitext(os.path.basename(path))[0]
                prefix = job.get("prefix_numeric")
                if prefix and len(name) == 14 and name[:3].isascii() and name[:3].isdigit() and name[3:] == "UpgradeIcon":
                    name = prefix + name
                if job.get("ascii_names"):
                    import unicodedata
                    name = unicodedata.normalize("NFKD", name).encode("ascii", "ignore").decode()
                if job.get("portrait_tower"):
                    tier, dash, _ = name.partition("-")
                    if dash and tier.isdigit():
                        name = tier + "-" + job["portrait_tower"]
                name = job.get("name_prefix", "") + name
                # Preserve the sprite's original transparent margins, just as before.
                image = full_rect(obj) if hasattr(obj, "m_Rect") else obj.image
                image.save(os.path.join(out, name + ".png"))
                print(path, "->", name + ".png", image.size)
        if job.get("rename_hero"):
            import re
            hero = job["rename_hero"]
            # Match the original post-export rename pass, including directory order.
            for filename in os.listdir(out):
                match = re.search(r"Lvl?(\d+)\.png$", filename)
                renamed = f"{hero}PortraitLvl{match.group(1)}.png" if match else f"{hero}Portrait.png"
                if filename != renamed:
                    os.replace(os.path.join(out, filename), os.path.join(out, renamed))
        if len(jobs) > 1:
            print("@@RRBTD6_PROGRESS " + json.dumps({"done": i + 1, "total": len(jobs),
                  "label": f"Tower artwork: {i + 1}/{len(jobs)}"}), flush=True)


def main():
    # Asset names may contain non-ASCII letters on a Windows console.
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    game = sys.argv[1]
    if len(sys.argv) == 4 and sys.argv[2] == "--jobs":
        with open(sys.argv[3], encoding="utf-8") as stream:
            jobs = json.load(stream)
    else:
        jobs = [{"out": sys.argv[2], "paths": sys.argv[3:]}]
    export_jobs(game, jobs)


if __name__ == "__main__":
    main()
