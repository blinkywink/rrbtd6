"""Bake the original title-intro Monkey Ace in its actor's local space. Local assets only."""
import gc
import subprocess
import sys
import os
import json
from pathlib import Path
import UnityPy

game, root = map(Path, sys.argv[1:3])
aa = game / "BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full"
bundle = next(aa.glob("ui_scenes_all_*.bundle"))
env = UnityPy.load(str(bundle))
matches = [cab for obj in env.objects if obj.type.name == "AssetBundle"
           for path, cab in obj.read_typetree().get("m_SceneHashes", [])
           if path.lower().endswith("/titlescreenui.unity")]
if len(matches) != 1:
    raise ValueError(f"Expected one native title scene, got {len(matches)}")
cab = matches[0]
del env
gc.collect()
subprocess.run([sys.executable, "-I", str(Path(__file__).with_name("export_animated.py")),
                str(game), str(root / "models/startup_plane"), "ui_scenes_all_",
                f"scene:{cab}|TitleScreenBg/MonkeyAce/MonkeyAce", "60"],
               env={**os.environ, "RRBTD6_ANIM_LOCAL": "1"}, check=True)
metadata = json.loads((root / "models/startup_plane/meta.json").read_text())
if not metadata.get("parts"):
    raise RuntimeError("The native title plane exported no geometry")
