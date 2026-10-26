"""Index original Sprite GUIDs to exact container paths, preserving same-name variants.
Unity GUID text reverses the hexadecimal nibbles of each serialized GUID word.
"""
import glob,json,os,sys
from pathlib import Path
os.environ['OPENBLAS_NUM_THREADS']='1'
import UnityPy
from UnityPy.helpers import TypeTreeHelper
TypeTreeHelper.read_typetree_boost=False
game,out=map(Path,sys.argv[1:3]);base=game/'BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full'
e=UnityPy.load(glob.glob(str(base/'asset_references_assets_all_*.bundle'))[0]);paths={}
for path,ref in e.container.items():
    obj=ref.deref() if hasattr(ref,'deref') else ref
    if obj.type.name!='Sprite':continue
    t=obj.read_typetree();key=t.get('m_RenderDataKey')
    if not key:continue
    guid=''.join(f'{key[0]["data["+str(i)+"]"]:08x}'[::-1] for i in range(4))
    if guid in paths and paths[guid]!=path:raise ValueError(f'Conflicting GUID {guid}: {paths[guid]} and {path}')
    paths[guid]=path
out.parent.mkdir(parents=True,exist_ok=True);out.write_text(json.dumps(paths,indent=1),encoding='utf8')
print(len(paths),'exact Sprite GUID paths')
