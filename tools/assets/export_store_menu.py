"""Extract the original local Shop catalog and product art; no account or transaction writes."""
import gc,glob,json,sys,xml.etree.ElementTree as ET
from collections import defaultdict
from pathlib import Path
import UnityPy
from export_sprites import full_rect

game,root=map(Path,sys.argv[1:3]);base=game/'BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full'
env=UnityPy.load(glob.glob(str(base/'game_data_assets_all_*.bundle'))[0],glob.glob(str(base/'*_monoscripts_*.bundle'))[0])
gd=env.container['Assets/Data/GameData.asset'];gd=gd.deref() if hasattr(gd,'deref') else gd
store=gd.assets_file.objects[gd.read_typetree()['storeItems']['m_PathID']].read_typetree()
loc={t.attrib['id']:''.join(t.itertext()).replace('\\n','\n') for t in ET.parse(root/'text/English.xml').iter('T') if 'id' in t.attrib}
paths=json.loads((root/'catalog_sprite_paths.json').read_text());bundles=json.loads((root/'all_container_paths.json').read_text())
images={};items=[]
for ref in store['storeItems']:
    item=gd.assets_file.objects[ref['m_PathID']].read_typetree();key=item['productID'];guid=item['icon'].get('guidRef','');path=paths.get(guid)
    icon='MenuIcon__'+guid[:12]+'__'+Path(path).name if path else None
    if icon:images[icon]=path
    items.append({'id':key,'title':loc.get(item['bundleTitleLoc'],loc.get(key,key)),
                  'description':loc.get(key+' Description',''),'icon':icon,'visible':bool(item['viewableInStore']),
                  'product_type':item['productType'],'raw':item})
display = [i for i in items if i['visible'] and i['product_type']==0]
missing = [i['id'] for i in display if not i['icon'] or i['title']==i['id']]
if missing:
    raise ValueError('Missing regular Shop presentation data: '+', '.join(missing))
scene=json.loads((root/'ui/prefabs/StoreUI.json').read_text());nodes=[]
def walk(n,path=''):
    path=path+'/'+n['name'] if path else n['name'];nodes.append((path.removeprefix('StoreUI/'),n))
    for child in n['children']:walk(child,path)
walk(scene)
refs={str(c['id']):p for p,n in nodes for c in n['comps'] if 'id' in c};refs.update({str(n['id']):p for p,n in nodes})
script=next(c for _,n in nodes for c in n['comps'] if c['type']=='StoreScreen')
labels=[]
for field,key in [('popularTxt','MostPopular'),('monkeyMoneyTxt','MonkeyMoney'),('heroesMonkeysTxt','HeroesAndMonkeys'),('modesTxt','ModesAndMore'),('legendsTxt','Legends')]:
    labels.append({'path':refs[script[field].removeprefix('->')],'text':loc['StoreCategory_'+key]})
(root/'ui/runtime/store_menu.json').write_text(json.dumps({'items':items,'labels':labels},ensure_ascii=False,indent=1),encoding='utf8')
del gd,env;gc.collect()
dest=root/'ui/menu_icons';dest.mkdir(exist_ok=True);groups=defaultdict(list)
for name,path in images.items():
    if not (dest/name).exists():groups[bundles[path]].append((name,path))
for bundle,entries in groups.items():
    env=UnityPy.load(str(base/bundle))
    for name,path in entries:
        sprite=env.container[path].read();full_rect(sprite).save(dest/name)
        for bf in env.files.values():
            for sf in getattr(bf,'files',{}).values():
                if hasattr(sf,'_cache'):sf._cache.clear()
    del env;gc.collect()
print(f'Shop: {len(items)} records, {len(images)} exact icon references; '+str(sum(i['visible'] and i['product_type']==0 for i in items))+' regular display records')
