"""Extract original animated Power portrait UI prefabs and their animation curves locally."""
import json
from pathlib import Path
import subprocess
import sys

game, root = sys.argv[1:3]
root=Path(root)
catalog=json.loads((root/'all_container_paths.json').read_text())
powers=json.loads((root/'ui/runtime/powers.json').read_text())
names={p['animated_icon'] for p in powers if p.get('animated_icon')}
paths=sorted(p for p in catalog if p.startswith('Assets/UI - New/Prefabs/Powers/AnimatedPortraits/') and Path(p).name in names)
assert len(paths)==len(names), 'An animated Power portrait source is missing'
tools=Path(__file__).parent
subprocess.run([sys.executable,str(tools/'dump_ui_prefab.py'),game,str(root/'ui/prefabs'),*paths],check=True)
subprocess.run([sys.executable,str(tools/'export_ui_runtime.py'),game,str(root/'ui/runtime'),
                *('prefab:'+path for path in paths)],check=True)
