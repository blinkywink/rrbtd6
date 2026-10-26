"""Export the installed avatar/banner catalogs, preserving native order and asset references."""
import glob
import json
from pathlib import Path
import sys
import UnityPy

game,root=map(Path,sys.argv[1:3])
base=game/'BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full'
env=UnityPy.load(glob.glob(str(base/'game_data_assets_all_*.bundle'))[0],glob.glob(str(base/'*_monoscripts_*.bundle'))[0])
obj=env.container['Assets/Data/GameData.asset'];obj=obj.deref() if hasattr(obj,'deref') else obj
gd=obj.read_typetree()
paths=json.loads((root/'catalog_sprite_paths.json').read_text(encoding='utf-8'))
def read(pointer):
    if pointer['m_FileID']:raise ValueError('Unexpected external profile catalog reference.')
    return obj.assets_file.objects[pointer['m_PathID']].read_typetree()
result={}
for field,category in [('profileAvatars','avatars'),('profileBanners','banners')]:
    entries=[]
    for pointer in read(gd[field])[field]:
        item=read(pointer)
        source=paths.get(item['icon']['guidRef'])
        small=paths.get(item.get('iconSmall',{}).get('guidRef')) or source
        entries.append({'id':item['id'],'icon':Path(source).stem if source else None,
            'iconSmall':Path(small).stem if small else None,'source':source,'sourceSmall':small,
            'trophyStoreId':item.get('trophyStoreId',''),'iap':item.get('iapStoreItemKey',''),
            'type':item.get('profileItemType',0),'unlockText':item.get('customUnlockInfoLocKey','')})
    result[category]=entries
path=root/'ui/runtime/profile_cosmetics.json'
path.write_text(json.dumps(result,indent=1),encoding='utf-8')
print('Profile cosmetics:',{key:len(value) for key,value in result.items()})
