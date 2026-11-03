"""Extract Trophy Store display catalogs and icons; never writes game state.
Usage: python export_menu_catalogs.py GAME EXTRACTED [--images]
All outputs are proprietary, local-only extracted data.
"""
import glob,json,os,sys,gc,xml.etree.ElementTree as ET
from pathlib import Path
os.environ['OPENBLAS_NUM_THREADS']='1'
import UnityPy
from export_sprites import full_rect

game,root=map(Path,sys.argv[1:3]);base=game/'BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full'
e=UnityPy.load(glob.glob(str(base/'game_data_assets_all_*.bundle'))[0])
gd=e.container['Assets/Data/GameData.asset'];gd=gd.deref() if hasattr(gd,'deref') else gd
fields=gd.read_typetree();catalog=json.loads((root/'catalog_sprites.json').read_text());index=json.loads((root/'all_container_paths.json').read_text())
loc={t.attrib['id']:''.join(t.itertext()) for t in ET.parse(root/'text/English.xml').iter('T') if 'id' in t.attrib}
def data(ref):
    if not ref.get('m_PathID'):return {}
    if ref.get('m_FileID',0):raise ValueError('Unexpected external catalog data reference')
    return gd.assets_file.objects[ref['m_PathID']].read_typetree()
path_catalog=json.loads((root/'catalog_sprite_paths.json').read_text()) if (root/'catalog_sprite_paths.json').exists() else {}
source_aliases={}
def icon(ref):
    key=ref.get('guidRef','')
    if key in path_catalog:
        path=path_catalog[key];alias='MenuIcon__'+key[:12]+'__'+Path(path).name
        source_aliases[alias]=path
        return alias
    return key.split('[',1)[1].rstrip(']')+'.png' if '[' in key else catalog.get(key)
type_icons={t['tab']:icon(t['icon']) for t in data(fields['trophyStoreItems'])['trophyStoreTypeToIcon']}
items=[];seen=set()
for path,p in e.container.items():
    if not path.startswith('Assets/Data/TrophyStore/') or not path.endswith('.asset') or path in seen:continue
    seen.add(path);t=p.read_typetree()
    if not t.get('id') or 'icon' not in t:continue
    key=t['id'];items.append({'id':key,'title':loc.get(key+'Name',key),'short_name':loc.get(key+'ShortName',loc.get(key+'Name',key)),
        'description':loc.get(key+'Description',''),'icon':icon(t['icon']),'category':t['storeFilter'],
        'subfilter':t['subFilter'],'priority':t['cosmeticPriority'],'type_icon':type_icons.get(t['storeFilter']),'raw':t})
items.sort(key=lambda i:(i['category'],i['subfilter'],i['priority'],i['id']))
out=root/'ui/runtime';out.mkdir(parents=True,exist_ok=True)
(out/'trophy_items.json').write_text(json.dumps(items,indent=1),encoding='utf8')
print(len(items),'trophy items',flush=True)
if '--images' in sys.argv:
    from collections import defaultdict
    del e,gd;gc.collect()
    names={i['icon'] for i in items if i['icon']}
    names|={i['type_icon'] for i in items if i['type_icon']}
    by_name=defaultdict(list)
    for path in index:by_name[Path(path).name].append(path)
    bundles=defaultdict(list);dest=root/'ui/menu_icons';dest.mkdir(exist_ok=True)
    missing=[]
    for name in sorted(names):
        if (dest/name).exists():continue
        candidates=[source_aliases[name]] if name in source_aliases else by_name.get(name,[])
        if not candidates:missing.append(name);continue
        # Explicit trophy icon folders outrank any unrelated same-name art.
        path=sorted(candidates,key=lambda p:('/TrophyStoreIcons/' not in p,p))[0]
        bundles[index[path]].append((path,name))
    for bundle,paths in bundles.items():
        images=UnityPy.load(str(base/bundle))
        for path,name in paths:
            source=images.container[path].read();im=full_rect(source) if hasattr(source,'m_Rect') else source.image
            im.save(dest/name)
            for f in images.files.values():
                for sf in getattr(f,'files',{}).values():
                    if hasattr(sf,'_cache'):sf._cache.clear()
            del im
        del images;gc.collect()
    print(len(names)-len(missing),'catalog icons available; unresolved:',missing,flush=True)
