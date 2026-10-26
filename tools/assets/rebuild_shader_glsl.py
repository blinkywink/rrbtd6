"""Regenerate local GLSL from its retained original shader disassembly; no game-state writes."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from dxbc_glsl import build

count = 0
for path in (Path(sys.argv[1])/'shaders').glob('*.json'):
    data = json.loads(path.read_text(encoding='utf8'))
    if not data.get('asm'):
        continue
    vs, fs, _, _, _, attrs = build(data['asm']['vs'], data['asm']['fs'])
    data.update(vs=vs, fs=fs, attributes=attrs)
    path.write_text(json.dumps(data,indent=1),encoding='utf8')
    count += 1
print(f'Rebuilt {count} local shaders from original DXBC disassembly')
