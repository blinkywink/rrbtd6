"""Prepare original co-op layouts for the shared map renderer; no network operations."""
import copy,json,sys
from pathlib import Path
root=Path(sys.argv[1]);path=root/'ui/prefabs'
d=json.loads((path/'CoopMapSelectUI.json').read_text(encoding='utf8'))
def walk(n):
    if n['name']=='CoopMapSelectCanvas':n['name']='MapSelectCanvas'
    if n['name']=='CoopMapSelectScreen':n['name']='MapSelectScreen'
    if n['name']=='Maps':
        result=[]
        for child in n['children']:
            if child['name'].startswith('Map_'):
                wrapper={'name':child['name'],'active':child.get('active',True),'rect':copy.deepcopy(child['rect']),'comps':[],'children':[child]}
                child['rect'].update(anchor_min=[0,0],anchor_max=[1,1],pos=[0,0],size=[0,0])
                result.append(wrapper)
            else:result.append(child)
        n['children']=result
        return
    for child in n.get('children',[]):walk(child)
walk(d);(path/'CoopMapSelectPrepared.json').write_text(json.dumps(d),encoding='utf8')
print('Prepared native co-op map layout')
