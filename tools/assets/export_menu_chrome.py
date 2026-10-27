"""Export scene-embedded menu sprites absent from the shared atlases, with slicing metadata.

Usage: python export_menu_chrome.py GAME EXTRACTED
Outputs only local, proprietary assets under EXTRACTED/sprites/MenuChrome.
"""
import gc
import glob
import json
import sys
from pathlib import Path

import UnityPy
sys.path.insert(0,str(Path(__file__).resolve().parent))
from export_sprites import full_rect


def main():
    game, root = map(Path, sys.argv[1:3])
    wanted = set()
    def walk(node):
        for component in node.get('comps', []):
            if component.get('type') == 'Image' and component.get('sprite'):
                wanted.add(component['sprite'])
        for child in node.get('children', []):
            walk(child)
    for source in (root/'ui/prefabs').glob('*.json'):
        walk(json.loads(source.read_text(encoding='utf8')))
    available = {p.stem for p in (root/'sprites').glob('*/*.png')}
    wanted -= available
    dest = root/'sprites/MenuChrome'
    dest.mkdir(parents=True, exist_ok=True)
    metadata = {}
    base = game/'BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full'
    for pattern in ('ui_scenes_all_*.bundle', 'asset_references_assets_all_*.bundle'):
        env = UnityPy.load(glob.glob(str(base/pattern))[0])
        for obj in env.objects:
            if obj.type.name != 'Sprite':
                continue
            sprite = obj.read()
            if sprite.m_Name not in wanted:
                continue
            full_rect(sprite).save(dest/(sprite.m_Name+'.png'))
            border = sprite.m_Border
            metadata[sprite.m_Name] = [border.x,border.y,border.z,border.w,sprite.m_PixelsToUnits]
            wanted.remove(sprite.m_Name)
            for bundle in env.files.values():
                for file in getattr(bundle,'files',{}).values():
                    if hasattr(file,'_cache'):
                        file._cache.clear()
        del env
        gc.collect()
    path = dest/'_borders.json'
    previous = json.loads(path.read_text()) if path.exists() else {}
    previous.update(metadata)
    path.write_text(json.dumps(previous,indent=1))
    print(len(metadata),'missing menu sprites exported; unresolved:',sorted(wanted))


if __name__ == '__main__':
    main()
