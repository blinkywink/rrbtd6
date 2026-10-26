"""Export original home interaction clips and their native BoxColliders. Local assets only."""
import glob,json,os,subprocess,sys
from pathlib import Path
import UnityPy

game,root=sys.argv[1:3];root=Path(root);cab='CAB-613509f63417784aa3960cd4dc66a279'
sys.argv=[sys.argv[0],game,str(root/'models/mainmenu'),'scene','ui_scenes_all_']
import export_model as em
bundle=glob.glob(str(Path(em.AA)/'ui_scenes_all_*.bundle'))[0]
env=UnityPy.load(bundle);em._envs[Path(bundle).name]=env
af=next(o.assets_file for o in env.objects if o.assets_file.name==cab)
def transform(go):return next(af.objects[c['component']['m_PathID']] for c in go['m_Component'] if af.objects[c['component']['m_PathID']].type.name=='Transform')
def info(tid):
    names=[];active=True
    while tid:
        t=af.objects[tid].read_typetree();g=af.objects[t['m_GameObject']['m_PathID']].read_typetree();names.append(g['m_Name']);active &=bool(g['m_IsActive']);tid=t['m_Father']['m_PathID']
    return '/'.join(reversed(names)),active
entries=[]
for obj in af.objects.values():
    if obj.type.name!='MonoBehaviour':continue
    settings=obj.read_typetree()
    if 'triggerAnimationLength' not in settings or not settings.get('m_Enabled',1):continue
    go=af.objects[settings['m_GameObject']['m_PathID']].read_typetree();tr=transform(go);path,active=info(tr.path_id)
    if not active:continue
    collider=next((af.objects[c['component']['m_PathID']].read_typetree() for c in go['m_Component'] if af.objects[c['component']['m_PathID']].type.name=='BoxCollider'),None)
    if not collider or not collider['m_Enabled']:continue
    target=tr
    if settings['animator']['m_PathID']:
        a=af.objects[settings['animator']['m_PathID']].read_typetree();target=transform(af.objects[a['m_GameObject']['m_PathID']].read_typetree())
    target_path,_=info(target.path_id)
    clip=em.resolve(af,settings['animationClip']).read_typetree();m=clip['m_MuscleClip']
    entries.append({'path':target_path,'collider_path':path,'center':[collider['m_Center'][a] for a in 'xyz'],
        'size':[abs(collider['m_Size'][a]) for a in 'xyz'],'world':em.world_matrix(af,tr.path_id).T.flatten().tolist(),
        'duration':m['m_StopTime']-m['m_StartTime'],'no_interrupt':bool(settings['noInterrupt']),'clip':clip['m_Name']})
entries.sort(key=lambda x:x['collider_path'])
for i,entry in enumerate(entries):
    entry['directory']=f'interact_{i}';dest=root/'models/mainmenu_anim'/entry['directory']
    subprocess.run([sys.executable,str(Path(__file__).with_name('export_animated.py')),game,str(dest),'ui_scenes_all_',f'scene:{cab}|'+entry['path'],'60'],
        env={**os.environ,'RRBTD6_ANIM_INTERACT':'1','RRBTD6_ANIM_CLIP':entry['clip']},check=True)
(root/'models/mainmenu_anim/interactions.json').write_text(json.dumps(entries,indent=1))
print('Exported',len(entries),'original home interactions')
