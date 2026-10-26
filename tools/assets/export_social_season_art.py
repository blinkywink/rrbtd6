"""Export native Social Season theme artwork for the local menu renderer."""
import glob,json,sys
from collections import defaultdict
from pathlib import Path
import UnityPy
from export_sprites import full_rect

game,root=map(Path,sys.argv[1:3])
base=game/'BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full'
index=json.loads((root/'all_container_paths.json').read_text(encoding='utf8'))
groups=defaultdict(list)
for path,bundle in index.items():
    if path.endswith('.png') and ('/Events/SocialSeasons/Goals/' in path or '/ThemeBanners/' in path or '/SocialSeasonMedals/' in path or path.endswith('/TeamIslandBtn.png')):
        groups[bundle].append(path)
dest=root/'sprites/SocialSeasons';dest.mkdir(parents=True,exist_ok=True)
count=0
for bundle,paths in groups.items():
    env=UnityPy.load(str(base/bundle))
    for path in paths:
        obj=env.container[path];obj=obj.deref() if hasattr(obj,'deref') else obj
        data=obj.read()
        image=full_rect(data) if obj.type.name=='Sprite' else data.image
        image.save(dest/Path(path).name);count+=1
print(count,'Social Season images exported')
