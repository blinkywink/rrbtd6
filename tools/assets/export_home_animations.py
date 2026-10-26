"""Bake original continuous home-world idle clips in scene coordinates; local assets only."""
import json
import os
from pathlib import Path
import subprocess
import sys

game, root = sys.argv[1:3]
root = Path(root)
cab = 'CAB-613509f63417784aa3960cd4dc66a279'
entries = [
    ('dart', 'MainMenuWorld/Interactive/FountainAnimation/DartMonkeyPopUp/LOD_1', 'DartMonkeyPopupIdle'),
    ('coop', 'MainMenuWorld/Interactive/QuestMonkey/CoopEventMonkey/LOD_1', 'CoopMonkey@EventIdle'),
    ('patch', 'MainMenuWorld/Interactive/PatchQuestMonkeyGroup/PatchQuestMonkey/Container/PatchMonkey', 'Idle'),
    ('event_idle', 'MainMenuWorld/Interactive/QuestMonkey/EventMonkey/LOD_1', 'EventOffIdle'),
    ('ninja_idle', 'MainMenuWorld/Interactive/NinjaMonkeyPopUp/LOD_1', 'NinjaMonkeyPopUpIdle'),
    ('bomb_idle', 'MainMenuWorld/Interactive/BombShooterPopUp/LOD_1', 'Idle'),
    ('boomerang_idle', 'MainMenuWorld/Town/PalmTree (2)/LOD_1/PalmTree_Base_JNT/PalmTree_Trunk_JNT/PalmTree_TrunkMid_JNT/PalmTree_TrunkTop_JNT/BoomerangPopUp/LOD_1', 'Idle'),
]
manifest = []
for key, path, clip in entries:
    dest = root/'models/mainmenu_anim'/key
    subprocess.run([sys.executable,str(Path(__file__).with_name('export_animated.py')),game,str(dest),
                    'ui_scenes_all_',f'scene:{cab}|{path}','60'],
                   env={**os.environ,'RRBTD6_ANIM_CLIP':clip},check=True)
    data = json.loads((dest/'meta.json').read_text())
    # Preserve the source loop flag. Short non-looping idle clips hold their
    # final pose; they must never fall back to the exported skin bind pose.
    manifest.append({'directory':key,'path':path,'clip':clip})
(root/'models/mainmenu_anim/manifest.json').write_text(json.dumps(manifest,indent=1))

# The windmill is driven by a script, not an Animator clip. Preserve its source
# axis, speed and parent/rest transforms; CustomRotationSimple.Update uses
# AngleAxis(angle + speed * deltaTime * 360, rotationAxis).
import glob
import UnityPy
sys.argv = [sys.argv[0],game,str(root/'models/mainmenu'),'scene','ui_scenes_all_']
import export_model as em
bundle = glob.glob(str(Path(em.AA)/'ui_scenes_all_*.bundle'))[0]
env = UnityPy.load(bundle)
em._envs[Path(bundle).name] = env
af = next(o.assets_file for o in env.objects if o.assets_file.name == cab)
def scene_path(tid):
    names=[]
    while tid:
        t=af.objects[tid].read_typetree()
        names.append(af.objects[t['m_GameObject']['m_PathID']].read_typetree()['m_Name'])
        tid=t['m_Father']['m_PathID']
    return '/'.join(reversed(names))
path='MainMenuWorld/Town/Windmill/WindmillSails'
transform=next(o for o in af.objects.values() if o.type.name=='Transform' and scene_path(o.path_id)==path)
t=transform.read_typetree()
go=af.objects[t['m_GameObject']['m_PathID']].read_typetree()
settings=next(af.objects[c['component']['m_PathID']].read_typetree() for c in go['m_Component']
              if af.objects[c['component']['m_PathID']].type.name=='MonoBehaviour'
              and 'rotationAxis' in af.objects[c['component']['m_PathID']].read_typetree())
assert settings['useLocalCoordinates'] and settings['minStartingOffset']==settings['maxStartingOffset']==0
rotation={'path':path,'speed':settings['speed'],'axis':[settings['rotationAxis'][a] for a in 'xyz'],
          'position':[t['m_LocalPosition'][a] for a in 'xyz'],'scale':[t['m_LocalScale'][a] for a in 'xyz'],
          'parent':em.world_matrix(af,t['m_Father']['m_PathID']).T.flatten().tolist(),
          'rest':em.world_matrix(af,transform.path_id).T.flatten().tolist()}
(root/'models/mainmenu_anim/rotations.json').write_text(json.dumps([rotation],indent=1))

# Clouds are rigid meshes with PlayableAnimatorSimple curves. Keep the original
# cubic channels instead of baking long loops into thousands of vertex frames.
sys.argv=[sys.argv[0],game,str(root/'ui/runtime')]
from export_ui_runtime import curves
import zlib
rigid=[]
for cloud in ('CloudsForeground','CloudsMidground'):
    path='MainMenuWorld/Sky/'+cloud
    tr=next(o for o in af.objects.values() if o.type.name=='Transform' and scene_path(o.path_id)==path)
    nodes=[];stack=[(tr.path_id,None)];hashes={}
    while stack:
        tid,parent=stack.pop();t=af.objects[tid].read_typetree();p=scene_path(tid)
        index=len(nodes);hashes[zlib.crc32(p[len(path):].lstrip('/').encode())]=index
        nodes.append({'path':p,'parent':parent,'position':[t['m_LocalPosition'][a] for a in 'xyz'],
                      'rotation':[t['m_LocalRotation'][a] for a in 'xyzw'],'scale':[t['m_LocalScale'][a] for a in 'xyz'],
                      'rest':em.world_matrix(af,tid).T.flatten().tolist(),'channels':[]})
        stack.extend((child['m_PathID'],index) for child in reversed(t['m_Children']))
    t=tr.read_typetree();go=af.objects[t['m_GameObject']['m_PathID']].read_typetree()
    settings=next(af.objects[c['component']['m_PathID']].read_typetree() for c in go['m_Component']
                  if af.objects[c['component']['m_PathID']].type.name=='MonoBehaviour'
                  and 'dontPlayOnInit' in af.objects[c['component']['m_PathID']].read_typetree())
    assert not settings['dontPlayOnInit'] and not settings['randomizeSpeed'] and not settings['randomStartTime']
    clip=em.resolve(af,settings['animationClip']).read_typetree();channels=curves(clip);index=0
    for binding in clip['m_ClipBindingConstant']['genericBindings']:
        count={1:3,2:4,3:3,4:3}.get(binding['attribute'],1) if binding['typeID']==4 else 1
        if binding['typeID']==4 and binding['path'] in hashes:
            nodes[hashes[binding['path']]]['channels'].append({'attribute':binding['attribute'],'curves':channels[index:index+count]})
        index+=count
    muscle=clip['m_MuscleClip']
    rigid.append({'nodes':nodes,'parent':em.world_matrix(af,t['m_Father']['m_PathID']).T.flatten().tolist(),
                  'start':muscle['m_StartTime'],'stop':muscle['m_StopTime'],'looped':bool(muscle['m_LoopTime']),
                  'speed':settings['playbackSpeed']})
(root/'models/mainmenu_anim/rigid_clips.json').write_text(json.dumps(rigid,allow_nan=False))
