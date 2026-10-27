"""Extract native Home menu hit volumes; proprietary output remains local."""
import glob,json,sys,zlib
from pathlib import Path
import UnityPy
game,root=sys.argv[1:3];root=Path(root)
sys.argv=[sys.argv[0],game,str(root/'models/mainmenu'),'scene','ui_scenes_all_']
import export_model as em
bundle=glob.glob(str(Path(em.AA)/'ui_scenes_all_*.bundle'))[0]
env=UnityPy.load(bundle);em._envs[Path(bundle).name]=env
af=next(o.assets_file for o in env.objects if o.assets_file.name=='CAB-613509f63417784aa3960cd4dc66a279')
sys.argv=[sys.argv[0],game,str(root/'effects')]
import export_particles as ep
sys.argv=[sys.argv[0],game,str(root/'models/mainmenu'),'ui_scenes_all_','unused']
import export_animated as aa
exported=set()
routes={'PatchQuestMonkey':'quests','GiftBox':'gift','DailyChest':'daily_reward','DigitalContent':'tv','EventMonkey':'events','CoopEventMonkey':'events','QuestMonkey':'events'}
buttons=[]
effects=[]
for obj in af.objects.values():
    if obj.type.name!='MonoBehaviour':continue
    s=obj.read_typetree()
    if not all(k in s for k in ('effect','effectLocationMarker','delay','clip')) or not s.get('m_Enabled',1):continue
    go=af.objects[s['m_GameObject']['m_PathID']].read_typetree()
    tr=next(af.objects[x['component']['m_PathID']] for x in go['m_Component'] if af.objects[x['component']['m_PathID']].type.name=='Transform')
    tid=tr.path_id;names=[];active=True
    while tid:
        t=af.objects[tid].read_typetree();g=af.objects[t['m_GameObject']['m_PathID']].read_typetree();names.append(g['m_Name']);active &=bool(g['m_IsActive']);tid=t['m_Father']['m_PathID']
    path='/'.join(reversed(names))
    if not active or not path.startswith('MainMenuWorld/Interactive/'):continue
    effect=em.resolve(af,s['effect']);name=effect.read_typetree()['m_Name']
    marker=s['effectLocationMarker']['m_PathID'] or tr.path_id
    if af.objects[marker].type.name=='GameObject':
        mg=af.objects[marker].read_typetree()
        marker=next(x['component']['m_PathID'] for x in mg['m_Component'] if af.objects[x['component']['m_PathID']].type.name=='Transform')
    # Evaluate the animated marker at its source event time, not its bind pose.
    pose=aa.Clip(em.resolve(af,s['clip']).read_typetree()).pose(s['delay'])
    chain=[];tid=marker
    while tid:
        chain.append(tid)
        if tid==tr.path_id:break
        tid=af.objects[tid].read_typetree()['m_Father']['m_PathID']
    matrix=em.world_matrix(af,af.objects[tr.path_id].read_typetree()['m_Father']['m_PathID'])
    relative=[]
    for tid in reversed(chain):
        t=af.objects[tid].read_typetree()
        if tid!=tr.path_id:relative.append(af.objects[t['m_GameObject']['m_PathID']].read_typetree()['m_Name'])
        p=pose.get(zlib.crc32('/'.join(relative).encode()),{})
        position=p.get(1,[t['m_LocalPosition'][a] for a in 'xyz'])
        rotation=p.get(2,[t['m_LocalRotation'][a] for a in 'xyzw'])
        if 4 in p:rotation=aa.euler_quat(p[4])
        scale=p.get(3,[t['m_LocalScale'][a] for a in 'xyz'])
        matrix=matrix@aa.trs_mat(position,rotation,scale)
    pos=(matrix if tr.path_id in chain else em.world_matrix(af,marker))[:3,3].tolist()
    if name not in exported:
        ep.export_root(env,effect.read(),name);exported.add(name)
    effects.append(dict(path=path,name=name,delay=s['delay'],position=pos))
for obj in af.objects.values():
    if obj.type.name!='BoxCollider':continue
    c=obj.read_typetree()
    if not c['m_Enabled']:continue
    go=af.objects[c['m_GameObject']['m_PathID']].read_typetree()
    tr=next(af.objects[x['component']['m_PathID']] for x in go['m_Component'] if af.objects[x['component']['m_PathID']].type.name=='Transform')
    tid=tr.path_id;names=[];active=True
    while tid:
        t=af.objects[tid].read_typetree();g=af.objects[t['m_GameObject']['m_PathID']].read_typetree()
        names.append(g['m_Name']);active &=bool(g['m_IsActive']);tid=t['m_Father']['m_PathID']
    path='/'.join(reversed(names))
    if not active or not path.startswith('MainMenuWorld/Interactive/'):continue
    action=next((routes[n.strip()] for n in names if n.strip() in routes),None)
    if not action:continue
    buttons.append(dict(path=path,action=action,world=em.world_matrix(af,tr.path_id).T.flatten().tolist(),
                        center=[c['m_Center'][a] for a in 'xyz'],size=[c['m_Size'][a] for a in 'xyz']))
out=root/'models/mainmenu_anim/buttons.json';out.write_text(json.dumps(buttons,indent=1))
(root/'models/mainmenu_anim/effects.json').write_text(json.dumps(effects,indent=1))
print('Home effects:',effects)
print('Home button volumes:',[(b['path'],b['action']) for b in buttons])
