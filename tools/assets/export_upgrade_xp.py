"""UpgradeModel.xpCost (MonkeyButton.CanAcquireUpgrade) and UpgradeModel.icon sprite per upgrade name.
Local extracted data only.
Usage: python -I export_upgrade_xp.py EXTRACTED
"""
import json
import sys
from pathlib import Path

root=Path(sys.argv[1])
catalog={**json.loads((root/'catalog_sprites.json').read_text(encoding='utf8')),
         **json.loads((root/'catalog_sprite_paths.json').read_text(encoding='utf8'))}
costs={};icons={};paragon_portraits={}
for line in (root/'decoded/named/objects.jsonl').open(encoding='utf8'):
    if '"TowerModel"' in line and '"isParagon": true' in line:
        item=json.loads(line);f=item['f']
        # The Paragon TowerModel's portrait (file names can predate the Paragon's current name).
        path=catalog.get(f.get('portrait') or '')
        if item['type']=='TowerModel' and f.get('isParagon') and isinstance(path,str):paragon_portraits[f['baseId']]=Path(path).stem
        continue
    if '"UpgradeModel"' not in line:continue
    item=json.loads(line)
    if item['type']!='UpgradeModel':continue
    f=item['f'];costs[f['_name']]=f['xpCost']
    # The icon SpriteReference's GUID -> exported sprite file. Icons sharing a file name across
    # towers were exported with their folder as a prefix ("TackShooter__FasterShootingUpgradeIcon").
    path=catalog.get(f.get('icon') or '')
    if isinstance(path,str):
        stem=Path(path).stem;prefixed=f'{Path(path).parent.name}__{stem}'
        icons[f['_name']]=prefixed if (root/'ui/upgrades'/(prefixed+'.png')).exists() else stem
(root/'ui/runtime/upgrade_xp.json').write_text(json.dumps(costs,indent=1),encoding='utf8')
(root/'ui/runtime/upgrade_icons.json').write_text(json.dumps(icons,indent=1),encoding='utf8')
(root/'ui/runtime/paragon_portraits.json').write_text(json.dumps(paragon_portraits,indent=1),encoding='utf8')
print(len(costs),'upgrade XP costs;',len(icons),'upgrade icons;',len(paragon_portraits),'Paragon portraits')
