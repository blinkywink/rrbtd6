"""Bake author-owned OBJ meshes into RustTD6's native model/animation format.

python tools/mod_model.py template mods/throwbot
python tools/mod_model.py import robot.obj --out mods/my-mod/assets/robot --texture robot.png
OBJ uses game coordinates: Y up, +Z forward, feet at the origin. Apply transforms
when exporting from Blender. Optional --attack frame*.obj bakes a throw sequence.
No Python, OBJ parsing or rig evaluation runs during gameplay.
"""
import argparse
import json
import math
from pathlib import Path
import shutil
import struct
import zlib


def obj(path):
    positions, uv, vertices, triangles = [], [], [], []
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        words = line.split()
        if not words or words[0].startswith('#'):
            continue
        if words[0] == 'v':
            positions.append(tuple(map(float, words[1:4])))
        elif words[0] == 'vt':
            uv.append(tuple(map(float, words[1:3])))
        elif words[0] == 'f':
            face = []
            for word in words[1:]:
                fields = word.split('/')
                def index(value, size):
                    i = int(value)
                    if i == 0 or not -size <= i <= size:
                        raise ValueError('OBJ index is out of bounds')
                    return i - 1 if i > 0 else size + i
                xyz = positions[index(fields[0], len(positions))]
                tex = uv[index(fields[1], len(uv))] if len(fields) > 1 and fields[1] else (0., 0.)
                face.append(len(vertices)); vertices.append((*xyz, *tex))
            for i in range(1, len(face) - 1):
                triangles.extend((face[0], face[i], face[i+1]))
    if not vertices or len(vertices) > 60000 or len(triangles) > 180000:
        raise ValueError('Use a nonempty triangulated model below 60,000 split vertices / 60,000 triangles')
    if not all(math.isfinite(v) for row in vertices for v in row):
        raise ValueError('Model contains non-finite coordinates')
    return vertices, triangles


def solid_png(path, rgb=(255,255,255)):
    def chunk(tag, body):
        return struct.pack('>I',len(body))+tag+body+struct.pack('>I',zlib.crc32(tag+body)&0xffffffff)
    Path(path).write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>2I5B',1,1,8,2,0,0,0))+
                          chunk(b'IDAT',zlib.compress(b'\0'+bytes(rgb)))+chunk(b'IEND',b''))


def palette_png(path, colors=None):
    colors=colors or [(27,83,135),(65,180,230),(98,213,237),(98,115,134),(15,30,50),(255,202,55),(221,241,255),(240,95,65)]
    def chunk(tag,body):return struct.pack('>I',len(body))+tag+body+struct.pack('>I',zlib.crc32(tag+body)&0xffffffff)
    Path(path).write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>2I5B',len(colors),1,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(b'\0'+bytes(c for rgb in colors for c in rgb)))+chunk(b'IEND',b''))


def bake(out, vertices, indices, clips, *, texture=None, muzzle=(0,8,5), release=.15):
    out = Path(out)
    if not math.isfinite(release) or release < 0 or release > 5 or len(muzzle) != 3 or not all(math.isfinite(x) for x in muzzle):
        raise ValueError('Use a finite muzzle and release time between zero and five seconds')
    if not vertices or len(vertices)>60000 or not indices or any(i<0 or i>=len(vertices) for i in indices):
        raise ValueError('Invalid model topology')
    frames, anims = [], []
    for name, poses, fps, loop in clips:
        if not poses or len(poses)>600 or not 1 <= fps <= 120:
            raise ValueError('Clips need 1–600 frames at 1–120 FPS')
        if any(len(p)!=len(vertices) for p in poses):
            raise ValueError('Every animation frame must have identical topology')
        duration=(len(poses) if loop else max(1,len(poses)-1))/fps
        anims.append(dict(name=name,frames=len(poses),fps=fps,loop=loop,offset=len(frames),duration=duration))
        frames.extend(poses)
    if len(frames)*len(vertices)*24>128*1024*1024:
        raise ValueError('Baked animation exceeds 128 MiB; reduce vertices, frames or FPS')
    if not all(math.isfinite(v) for frame in frames for xyz in frame for v in xyz):
        raise ValueError('Animation contains non-finite coordinates')
    out.mkdir(parents=True,exist_ok=True)
    if texture:
        if Path(texture).suffix.lower() != '.png':raise ValueError('Use a PNG texture')
        if Path(texture).resolve()!=(out/'texture.png').resolve():shutil.copyfile(texture,out/'texture.png')
    else:solid_png(out/'texture.png')
    (out/'verts.bin').write_bytes(b''.join(struct.pack('<5f',*v) for v in vertices))
    (out/'idx.bin').write_bytes(struct.pack('<'+'I'*len(indices),*indices))
    with (out/'anim.bin').open('wb') as stream:
        for frame in frames:
            # Compute actual face normals for native silhouettes.
            normals=[[0.,0.,0.] for _ in vertices]
            for a,b,c in zip(indices[::3],indices[1::3],indices[2::3]):
                u=[frame[b][i]-frame[a][i] for i in range(3)];v=[frame[c][i]-frame[a][i] for i in range(3)]
                n=(u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0])
                for k in (a,b,c):
                    for i in range(3):normals[k][i]+=n[i]
            for xyz,n in zip(frame,normals):
                length=math.sqrt(sum(v*v for v in n)) or 1
                stream.write(struct.pack('<6f',*xyz,*(v/length for v in n)))
    part=dict(name='CustomModel',voff=0,vcount=len(vertices),ioff=0,icount=len(indices),texture='texture.png',tint=[[1,1,1,1]],outline=.25)
    meta=dict(vertex_count=len(vertices),parts=[part],anims=anims,mod_socket=dict(muzzle=list(muzzle),release=release))
    (out/'meta.json').write_text(json.dumps(meta,indent=2),encoding='utf-8')


def template(out, *, overwrite=False):
    """An original animated training robot; no game assets are bundled."""
    out=Path(out)
    if (out/"mod.py").exists() and not overwrite:raise ValueError("Template already exists; use --force only to replace it")
    vertices=[];indices=[];arm=[]
    def box(center,size,moving=False,color=0):
        start=len(vertices)
        for x,y,z in [(-1,-1,-1),(1,-1,-1),(1,1,-1),(-1,1,-1),(-1,-1,1),(1,-1,1),(1,1,1),(-1,1,1)]:
            vertices.append((center[0]+x*size[0]/2,center[1]+y*size[1]/2,center[2]+z*size[2]/2,(color+.5)/8,.5))
        for face in [(0,3,2,1),(4,5,6,7),(0,1,5,4),(3,7,6,2),(0,4,7,3),(1,2,6,5)]:
            for i in (face[0],face[1],face[2],face[0],face[2],face[3]):indices.append(start+i)
        if moving:arm.extend(range(start,len(vertices)))
    box((0,5,0),(6,8,5),color=0);box((0,11,0),(7,5,6),color=1);box((4,8,3),(2,2,7),True,color=2)
    box((-2,1,0),(2,2,4),color=3);box((2,1,0),(2,2,4),color=3)
    box((0,11,3.1),(5,3,.3),color=4)
    box((-1.3,11.2,3.35),(.9,.9,.3),color=5);box((1.3,11.2,3.35),(.9,.9,.3),color=5)
    box((0,5,2.6),(2,2,.3),color=6)
    rest=[v[:3] for v in vertices];throw=[]
    for frame in range(31):
        pose=list(rest);t=frame/60
        angle=math.sin(math.pi*min(t/.15,1))*.9 if t<=.15 else -.45*math.sin(math.pi*(t-.15)/.35)
        for i in arm:
            x,y,z=rest[i];y-=8;z-=.5
            pose[i]=(x,8+y*math.cos(angle)-z*math.sin(angle),.5+y*math.sin(angle)+z*math.cos(angle))
        throw.append(pose)
    bake(out/'assets/robot',vertices,indices,[('RobotIdle',[rest],60,True),('RobotAttack',throw,60,False)],muzzle=(4,8,6),release=.15)
    palette_png(out/'assets/robot/texture.png')
    shutil.copytree(out/'assets/robot',out/'assets/robot-upgraded',dirs_exist_ok=True)
    palette_png(out/'assets/robot-upgraded/texture.png',[(160,55,30),(240,135,60),(255,185,85),(98,115,134),(15,30,50),(255,230,90),(221,241,255),(240,95,65)])
    bolt=[(-.5,-.5,-2,0,0),(.5,-.5,-2,1,0),(.5,.5,-2,1,1),(-.5,.5,-2,0,1),(0,0,2,0,0)]
    triangles=[0,1,4,1,2,4,2,3,4,3,0,4,0,3,2,0,2,1]
    bake(out/'assets/bolt',bolt,triangles,[('BoltIdle',[[v[:3] for v in bolt]],60,True)])
    solid_png(out/'assets/bolt/texture.png',(255,195,55))
    script='''# Build: python tools/modsdk.py build mods/throwbot/mod.py
mod = Mod("throwbot", "Throwbot", description="An original animated model with a timed throw.")
tower = mod.tower("robot", base="DartMonkey", cost=250)
tower.model("robot")  # Reads the exported muzzle and release timing automatically.
tower.projectile(display=mod.model("bolt"), damage=2, pierce=3)
tower.upgrade(0, 1, cost=150, stats={"range":40}, weapon={"rate":0.5},
              projectile=mod.projectile("upgraded-bolt", display=mod.model("bolt"), damage=5, pierce=5),
              model="robot-upgraded")
'''
    identifier=out.name
    import re
    if not re.fullmatch(r'[a-z0-9][a-z0-9_-]{0,63}',identifier):raise ValueError('Use a lowercase mod folder name with letters, digits, underscores or hyphens')
    script=script.replace('mods/throwbot/',f'mods/{identifier}/').replace('Mod("throwbot",',f'Mod("{identifier}",')
    (out/'mod.py').write_text(script,encoding='utf-8')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    sub=parser.add_subparsers(dest='command',required=True)
    t=sub.add_parser('template');t.add_argument('out',type=Path);t.add_argument('--force',action='store_true')
    i=sub.add_parser('import');i.add_argument('obj',type=Path);i.add_argument('--out',required=True,type=Path)
    i.add_argument('--texture',type=Path);i.add_argument('--attack',nargs='+',type=Path)
    i.add_argument('--fps',type=float,default=60);i.add_argument('--muzzle',nargs=3,type=float,default=[0,8,5]);i.add_argument('--release',type=float,default=.15)
    args=parser.parse_args()
    if args.command=='template':template(args.out,overwrite=args.force);return
    vertices,indices=obj(args.obj);clips=[('CustomIdle',[[v[:3] for v in vertices]],args.fps,True)]
    if args.attack:
        poses=[]
        for path in args.attack:
            frame,triangles=obj(path)
            if triangles!=indices or len(frame)!=len(vertices):raise ValueError('Attack OBJ files must use identical topology and face order')
            poses.append([v[:3] for v in frame])
        clips.append(('CustomAttack',poses,args.fps,False))
    bake(args.out,vertices,indices,clips,texture=args.texture,muzzle=args.muzzle,release=args.release)


if __name__=='__main__':main()
