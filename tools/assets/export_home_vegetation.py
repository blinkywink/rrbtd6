"""Export source home vegetation curves and skinning, without per-frame mesh bakes."""
import glob,json,sys,zlib
from pathlib import Path
import numpy as np
import UnityPy
from UnityPy.helpers.MeshHelper import MeshHandler

game,root=sys.argv[1:3];root=Path(root)
sys.argv=[sys.argv[0],game,str(root/'models/mainmenu'),'scene','ui_scenes_all_']
import export_model as em
bundle=glob.glob(str(Path(em.AA)/'ui_scenes_all_*.bundle'))[0]
env=UnityPy.load(bundle);em._envs[Path(bundle).name]=env
af=next(o.assets_file for o in env.objects if o.assets_file.name=='CAB-613509f63417784aa3960cd4dc66a279')
sys.argv=[sys.argv[0],game,str(root/'ui/runtime')]
from export_ui_runtime import curves

def info(tid):
    names=[];active=True
    while tid:
        t=af.objects[tid].read_typetree();g=af.objects[t['m_GameObject']['m_PathID']].read_typetree()
        names.append(g['m_Name']);active &= bool(g['m_IsActive']);tid=t['m_Father']['m_PathID']
    return '/'.join(reversed(names)),active

clips=[]
for obj in af.objects.values():
    if obj.type.name!='MonoBehaviour':continue
    settings=obj.read_typetree()
    if 'dontPlayOnInit' not in settings or settings['dontPlayOnInit'] or not settings.get('m_Enabled',1):continue
    go=af.objects[settings['m_GameObject']['m_PathID']].read_typetree()
    tr=next((af.objects[c['component']['m_PathID']] for c in go['m_Component'] if af.objects[c['component']['m_PathID']].type.name=='Transform'),None)
    if not tr:continue
    path,active=info(tr.path_id)
    if not active or not any(s in path for s in ('/Grass/','/PalmTree')):continue
    nodes=[];ids={};hashes={};stack=[(tr.path_id,None)]
    while stack:
        tid,parent=stack.pop();t=af.objects[tid].read_typetree();p,a=info(tid)
        index=len(nodes);ids[tid]=index;hashes[zlib.crc32(p[len(path):].lstrip('/').encode())]=index
        nodes.append({'path':p,'parent':parent,'position':[t['m_LocalPosition'][a] for a in 'xyz'],
            'rotation':[t['m_LocalRotation'][a] for a in 'xyzw'],'scale':[t['m_LocalScale'][a] for a in 'xyz'],
            'rest':em.world_matrix(af,tid).T.flatten().tolist(),'channels':[]})
        stack.extend((c['m_PathID'],index) for c in reversed(t['m_Children']))
    source=em.resolve(af,settings['animationClip']);clip=source.read_typetree();channels=curves(clip);offset=0
    for binding in clip['m_ClipBindingConstant']['genericBindings']:
        count={1:3,2:4,3:3,4:3}.get(binding['attribute'],1) if binding['typeID']==4 else 1
        if binding['typeID']==4 and binding['path'] in hashes:
            nodes[hashes[binding['path']]]['channels'].append({'attribute':binding['attribute'],'curves':channels[offset:offset+count]})
        offset+=count
    parts=[]
    for tid,index in ids.items():
        t=af.objects[tid].read_typetree();g=af.objects[t['m_GameObject']['m_PathID']].read_typetree()
        if not info(tid)[1]:continue
        for c in g['m_Component']:
            renderer=af.objects[c['component']['m_PathID']]
            if renderer.type.name!='SkinnedMeshRenderer':continue
            r=renderer.read_typetree()
            if not r.get('m_Enabled',1):continue
            # The palm's nested triggered monkey has its own controller, not this clip.
            if '/BoomerangPopUp/' in nodes[index]['path']:continue
            mesh=em.resolve(af,r['m_Mesh']).read();h=MeshHandler(mesh);h.process()
            vertices=np.asarray(h.m_Vertices,dtype=float).reshape(-1,3)
            if not h.m_BoneWeights:raise ValueError('Missing source vegetation weights: '+path)
            weights=np.asarray(h.m_BoneWeights,dtype=float).reshape(len(vertices),-1)
            indices=np.asarray(h.m_BoneIndices,dtype=int).reshape(len(vertices),-1)
            bones=[ids[b['m_PathID']] for b in r['m_Bones']]
            binds=[em.mat4(b).T.flatten().tolist() for b in mesh.m_BindPose]
            for sub,mref in zip(h.get_triangles(),r['m_Materials']):
                material=em.resolve(af,mref)
                if material is None or material.type.name!='Material':continue
                ind=np.asarray(sub,dtype=np.uint32).reshape(-1)
                if len(ind)==0:continue
                used=np.unique(ind)
                if len(used)>9990:raise ValueError('Vegetation requires chunk remapping: '+path)
                part={'path':nodes[index]['path'],'vertices':vertices[used].tolist(),'weights':weights[used].tolist(),
                      'indices':indices[used].tolist(),'bones':bones,'binds':binds}
                # Model::load duplicates a submesh's vertex array per index chunk.
                parts.extend([part]*((len(ind)+4997)//4998))
    muscle=clip['m_MuscleClip'];t=tr.read_typetree()
    clips.append({'rig':{'nodes':nodes,'parent':em.world_matrix(af,t['m_Father']['m_PathID']).T.flatten().tolist(),
        'start':muscle['m_StartTime'],'stop':muscle['m_StopTime'],'looped':bool(muscle['m_LoopTime']),'speed':settings['playbackSpeed']},
        'parts':parts,'random_start':bool(settings['randomStartTime']),
        'speed_range':[settings['randomSpeedRange'][k] for k in ('min','max')] if settings['randomizeSpeed'] else None})
# Verify remapped source skin data against the existing world mesh before publishing it.
meta=json.loads((root/'models/mainmenu/meta.json').read_text())
static=np.fromfile(root/'models/mainmenu/verts.bin',dtype='<f4').reshape(-1,5)
error=0.0
for entry in clips:
    slots={}
    for part in entry['parts']:
        candidates=[p for p in meta['parts'] if p.get('path')==part['path']]
        slot=slots.get(part['path'],0);slots[part['path']]=slot+1
        native=candidates[slot]
        expected=static[native['voff']:native['voff']+native['vcount'],:3]
        assert len(expected)==len(part['vertices'])
        pos=np.c_[part['vertices'],np.ones(len(part['vertices']))]
        actual=np.zeros((len(pos),4))
        for k in range(len(part['weights'][0])):
            for j in range(len(pos)):
                bone=part['indices'][j][k]
                rest=np.asarray(entry['rig']['nodes'][part['bones'][bone]]['rest']).reshape(4,4).T
                bind=np.asarray(part['binds'][bone]).reshape(4,4).T
                actual[j]+=part['weights'][j][k]*(rest@bind@pos[j])
        error=max(error,float(np.max(np.abs(expected-actual[:,:3]))))
if error>0.001:raise ValueError(f'Vegetation rest-pose mismatch: {error}')
print(f'Verified source rest-pose maximum error: {error:.8f}')
output=root/'models/mainmenu_anim/vegetation.json';output.write_text(json.dumps(clips,allow_nan=False,separators=(',',':')))
print(f'{len(clips)} vegetation clips; {sum(len(p["vertices"]) for c in clips for p in c["parts"])} skinned vertices; {output.stat().st_size} bytes')
