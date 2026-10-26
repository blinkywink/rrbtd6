"""Resolve missing scene sprite references from a separate bundle, keeping peak RAM low.

Usage: UnityPy-python resolve_ui_sprites.py GAME ROOT BUNDLE_GLOB PREFAB_NAMES...
Outputs proprietary art under ROOT/sprites/MenuChrome only.
"""
import glob
import json
from pathlib import Path
import sys
import UnityPy
from export_sprites import full_rect

game,root,pattern,*names=sys.argv[1:]
root=Path(root)
sources={root/'ui/prefabs'/f'{name}.json':None for name in names}
wanted={}
def walk(node):
    for component in node.get('comps',[]):
        value=component.get('sprite')
        if component.get('type')=='Image' and isinstance(value,str) and value.startswith('?'):
            wanted.setdefault(int(value[1:]),[]).append(component)
    for child in node.get('children',[]):walk(child)
for path in sources:
    sources[path]=json.loads(path.read_text(encoding='utf-8'));walk(sources[path])
destination=root/'sprites/MenuChrome';destination.mkdir(parents=True,exist_ok=True)
meta=destination/'_borders.json'
borders=json.loads(meta.read_text()) if meta.exists() else {}
for bundle in glob.glob(str(Path(game)/'BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full'/pattern)):
    env=UnityPy.load(bundle)
    for obj in env.objects:
        if obj.path_id not in wanted or obj.type.name!='Sprite':continue
        sprite=obj.read()
        full_rect(sprite).save(destination/(sprite.m_Name+'.png'))
        b=sprite.m_Border
        borders[sprite.m_Name]=[b.x,b.y,b.z,b.w,sprite.m_PixelsToUnits]
        for component in wanted.pop(obj.path_id):component['sprite']=sprite.m_Name
    del env
for path,data in sources.items():path.write_text(json.dumps(data,indent=1),encoding='utf-8')
meta.write_text(json.dumps(borders,indent=1),encoding='utf-8')
print('Unresolved sprite references:',len(wanted))
