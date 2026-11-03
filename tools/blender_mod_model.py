"""Run in Blender: blender --background robot.blend --python tools/blender_mod_model.py --
--out mods/my-mod/assets/robot --idle 1 60 --attack 61 90 --release 70 --texture atlas.png

Put exported meshes in a collection named RustTD6 (or all visible meshes are used).
Feet: world origin. Blender Z is up, -Y is forward. Add an Empty named Muzzle
at the hand/barrel. Animate using the normal Blender timeline, including rigs.
The evaluated deformation is baked; no custom code executes in the game.
"""
import argparse
from pathlib import Path
import sys
import bpy

sys.path.insert(0,str(Path(__file__).resolve().parent))
from mod_model import bake


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out',required=True,type=Path)
    parser.add_argument('--idle',nargs=2,type=int,default=[1,60])
    parser.add_argument('--attack',nargs=2,type=int,required=True)
    parser.add_argument('--release',type=int,required=True)
    parser.add_argument('--texture',type=Path)
    args=parser.parse_args(sys.argv[sys.argv.index('--')+1:])
    scene=bpy.context.scene
    fps=scene.render.fps/scene.render.fps_base
    if not 1<=fps<=120:raise ValueError('Set scene FPS between 1 and 120')
    if not args.attack[0]<=args.release<=args.attack[1]:raise ValueError('Release must be inside the attack frame range')
    collection=bpy.data.collections.get('RustTD6')
    objects=sorted((o for o in (collection.all_objects if collection else scene.objects) if o.type=='MESH' and not o.hide_render),key=lambda o:o.name)
    if not objects:raise ValueError('No mesh objects to export')
    def xyz(p):return (float(p.x),float(p.z),float(-p.y))
    def evaluate(frame,rest=False):
        scene.frame_set(frame);graph=bpy.context.evaluated_depsgraph_get()
        positions=[];verts=[];indices=[]
        for obj in objects:
            evaluated=obj.evaluated_get(graph);mesh=evaluated.to_mesh()
            try:
                mesh.calc_loop_triangles()
                uv=mesh.uv_layers.active
                for triangle in mesh.loop_triangles:
                    for loop in triangle.loops:
                        vertex=mesh.loops[loop].vertex_index
                        point=xyz(evaluated.matrix_world@mesh.vertices[vertex].co)
                        positions.append(point)
                        if rest:
                            tex=tuple(uv.data[loop].uv) if uv else (0.,0.)
                            verts.append((*point,*tex));indices.append(len(indices))
            finally:evaluated.to_mesh_clear()
        return (verts,indices) if rest else positions
    vertices,indices=evaluate(args.idle[0],True)
    clips=[]
    for name,(start,end),loop in [('CustomIdle',args.idle,True),('CustomAttack',args.attack,False)]:
        if end<start or end-start>=600:raise ValueError('Each clip needs 1–600 frames')
        clips.append((name,[evaluate(frame) for frame in range(start,end+1)],fps,loop))
    scene.frame_set(args.release)
    muzzle=bpy.data.objects.get('Muzzle')
    if not muzzle:raise ValueError('Add an Empty named Muzzle at the release point')
    marker=muzzle.evaluated_get(bpy.context.evaluated_depsgraph_get()).matrix_world.translation
    bake(args.out,vertices,indices,clips,texture=args.texture,muzzle=xyz(marker),release=(args.release-args.attack[0])/fps)
    print('RustTD6 model exported:',args.out)


if __name__=='__main__':main()
