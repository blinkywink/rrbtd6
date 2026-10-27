//! Continuous original home-world clips, staged off the render thread.
use crate::{anim::{AnimModel, Shaders, Staged}, render3d::Model};
use macroquad::prelude::*;
use std::{path::{Path, PathBuf}, thread::JoinHandle};

#[derive(serde::Deserialize)]
struct Entry { directory: String, path: String }
#[derive(serde::Deserialize)]
struct ButtonData {path:String,action:String,center:[f32;3],size:[f32;3],world:[f32;16]}
#[derive(serde::Deserialize)]
struct EffectData {path:String,name:String,delay:f32,position:[f32;3]}
#[derive(serde::Deserialize)]
struct TriggerData {directory:String,path:String,collider_path:String,center:[f32;3],size:[f32;3],world:[f32;16],duration:f32,no_interrupt:bool}
struct Trigger {data:TriggerData,dir:PathBuf,model:Option<AnimModel>,pending:Option<JoinHandle<Option<Staged>>>,began:Option<f64>}
fn ray_box(origin:Vec3,direction:Vec3,inverse:Mat4,center:Vec3,size:Vec3)->Option<f32> {
    let origin=inverse.transform_point3(origin)-center;
    let direction=inverse.transform_vector3(direction);
    let mut lo=0.0_f32;let mut hi=f32::INFINITY;
    for axis in 0..3 {
        let half=size[axis].abs()*0.5;
        if direction[axis].abs()<1e-8 {if origin[axis].abs()>half{return None;}continue;}
        let a=(-half-origin[axis])/direction[axis];let b=(half-origin[axis])/direction[axis];
        lo=lo.max(a.min(b));hi=hi.min(a.max(b));
        if lo>hi{return None;}
    }
    Some(lo)
}
fn ray_triangle(origin:Vec3,direction:Vec3,a:Vec3,b:Vec3,c:Vec3)->Option<f32> {
    let ab=b-a;let ac=c-a;let p=direction.cross(ac);let det=ab.dot(p);
    if det.abs()<1e-7{return None;}
    let inv=det.recip();let s=origin-a;let u=s.dot(p)*inv;
    if !(0.0..=1.0).contains(&u){return None;}
    let q=s.cross(ab);let v=direction.dot(q)*inv;
    if v<0.0 || u+v>1.0{return None;}
    let distance=ac.dot(q)*inv;(distance>=0.0).then_some(distance)
}
#[derive(serde::Deserialize)]
struct RotationData {
    path: String, speed: f32, axis: [f32;3], position: [f32;3], scale: [f32;3],
    parent: [f32;16], rest: [f32;16],
}
struct Rotation {
    data: RotationData, parent: Mat4, inverse_rest: Mat4,
    parts: Vec<(usize, Vec<Vec3>)>,
}
#[derive(serde::Deserialize)]
struct Channels { attribute: u32, curves: Vec<Vec<[f32;5]>> }
#[derive(serde::Deserialize)]
struct RigidNodeData {
    path:String, parent:Option<usize>, position:[f32;3], rotation:[f32;4], scale:[f32;3],
    rest:[f32;16], channels:Vec<Channels>,
}
#[derive(serde::Deserialize)]
struct RigidData {nodes:Vec<RigidNodeData>,parent:[f32;16],start:f32,stop:f32,looped:bool,speed:f32}
struct RigidClip {
    data:RigidData, parent:Mat4, worlds:Vec<Mat4>, inverse_rest:Vec<Mat4>,
    parts:Vec<Vec<(usize,Vec<Vec3>)>>,
}
#[derive(serde::Deserialize)]
struct SkinPartData {path:String,vertices:Vec<[f32;3]>,weights:Vec<Vec<f32>>,indices:Vec<Vec<usize>>,bones:Vec<usize>,binds:Vec<[f32;16]>}
#[derive(serde::Deserialize)]
struct SkinData {rig:RigidData,parts:Vec<SkinPartData>,random_start:bool,speed_range:Option<[f32;2]>}
struct SkinVertex {position:Vec3,weights:[f32;4],indices:[usize;4]}
struct SkinPart {mesh:usize,vertices:Vec<SkinVertex>,bones:Vec<usize>,binds:Vec<Mat4>,matrices:Vec<Mat4>}
struct SkinClip {rig:RigidData,parent:Mat4,worlds:Vec<Mat4>,parts:Vec<SkinPart>,speed:f32,phase:f32}
fn local_pose(node:&RigidNodeData,time:f32)->Mat4 {
    let mut position=node.position;let mut scale=node.scale;let mut rotation=node.rotation;
    for channels in &node.channels {
        match channels.attribute {
            1=>for (v,c) in position.iter_mut().zip(&channels.curves){if let Some(x)=sample(c,time){*v=x;}},
            2=>for (v,c) in rotation.iter_mut().zip(&channels.curves){if let Some(x)=sample(c,time){*v=x;}},
            3=>for (v,c) in scale.iter_mut().zip(&channels.curves){if let Some(x)=sample(c,time){*v=x;}},
            4=>{
                let mut e=[0.0;3];
                for (v,c) in e.iter_mut().zip(&channels.curves){*v=sample(c,time).unwrap_or(0.0).to_radians();}
                rotation=Quat::from_euler(EulerRot::ZXY,e[2],e[0],e[1]).to_array();
            }
            _=>{}
        }
    }
    Mat4::from_scale_rotation_translation(Vec3::from_array(scale),Quat::from_array(rotation).normalize(),Vec3::from_array(position))
}
fn skin_position(vertex:&SkinVertex,matrices:&[Mat4])->Vec3 {
    let mut position=Vec3::ZERO;
    for k in 0..4 {
        if vertex.weights[k]!=0.0 {position+=matrices[vertex.indices[k]].transform_point3(vertex.position)*vertex.weights[k];}
    }
    position
}
fn sample(keys:&[[f32;5]], time:f32) -> Option<f32> {
    let key=keys.iter().rev().find(|k|k[0]<=time).or_else(||keys.first())?;
    let dt=(time-key[0]).max(0.0);
    Some(((key[1]*dt+key[2])*dt+key[3])*dt+key[4])
}
#[derive(Default)]
pub(super) struct HomeMotion {
    started: bool,
    pending: Vec<(String, JoinHandle<Option<Staged>>)>,
    models: Vec<(String,AnimModel)>,
    triggers:Vec<Trigger>,
    buttons:Vec<ButtonData>,
    action:Option<String>,
    effects:Vec<EffectData>,
    effect_times:Vec<Option<f64>>,
    particles:Option<crate::particles::Particles>,
    debug_clicked:bool,
    shaders: Option<Shaders>,
    rotations: Vec<Rotation>,
    rigid: Vec<RigidClip>,
    vegetation_pending:Option<JoinHandle<Vec<SkinData>>>,
    vegetation:Vec<SkinClip>,
}
impl HomeMotion {
    pub(super) fn preparing(&self)->bool{
        !self.started || !self.pending.is_empty() || self.vegetation_pending.is_some()
            || self.triggers.iter().any(|t|t.pending.is_some())
    }
    pub(super) fn update(&mut self, root: &Path, world: &mut Model) {
        if !self.started {
            self.started = true;
            // This is a tutorial prompt, not part of Patch's normal appearance.
            // Only a bound tutorial state should enable it.
            world.hide_subtree("MainMenuWorld/Interactive/PatchQuestMonkeyGroup/TutorialArrow3D");
            let base = root.join("models/mainmenu_anim");
            self.buttons=std::fs::read_to_string(base.join("buttons.json")).ok()
                .and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default();
            self.effects=std::fs::read_to_string(base.join("effects.json")).ok()
                .and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default();
            self.effect_times=vec![None;self.effects.len()];
            self.particles=Some(crate::particles::Particles::new(root));
            let triggers:Vec<TriggerData>=std::fs::read_to_string(base.join("interactions.json")).ok()
                .and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default();
            self.triggers=triggers.into_iter().map(|data| {
                world.hide_subtree(&data.path);
                let dir=base.join(&data.directory);let stage_dir=dir.clone();
                Trigger {dir,data,model:None,pending:Some(std::thread::spawn(move||AnimModel::stage(&stage_dir))),began:None}
            }).collect();
            let vegetation_file=base.join("vegetation.json");
            self.vegetation_pending=Some(std::thread::spawn(move || std::fs::read_to_string(vegetation_file).ok()
                .and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default()));
            let entries: Vec<Entry> = std::fs::read_to_string(base.join("manifest.json")).ok()
                .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
            for entry in entries {
                world.hide_subtree(&entry.path);
                let dir: PathBuf = base.join(entry.directory);
                self.pending.push((entry.path, std::thread::spawn(move || AnimModel::stage(&dir))));
            }
            let rotations: Vec<RotationData> = std::fs::read_to_string(base.join("rotations.json")).ok()
                .and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default();
            for data in rotations {
                let parts = world.subtree_parts(&data.path).into_iter().map(|i|
                    (i,world.parts[i].vertices.iter().map(|v|v.position).collect())).collect();
                self.rotations.push(Rotation {parent:Mat4::from_cols_array(&data.parent),
                    inverse_rest:Mat4::from_cols_array(&data.rest).inverse(),data,parts});
            }
            let clips: Vec<RigidData> = std::fs::read_to_string(base.join("rigid_clips.json")).ok()
                .and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default();
            for data in clips {
                let inverse_rest=data.nodes.iter().map(|n|Mat4::from_cols_array(&n.rest).inverse()).collect();
                let parts=data.nodes.iter().map(|n|world.parts_at_path(&n.path).into_iter().map(|i|
                    (i,world.parts[i].vertices.iter().map(|v|v.position).collect())).collect()).collect();
                self.rigid.push(RigidClip {worlds:vec![Mat4::IDENTITY;data.nodes.len()],
                    parent:Mat4::from_cols_array(&data.parent),inverse_rest,parts,data});
            }
        }
        if self.vegetation_pending.as_ref().is_some_and(|h|h.is_finished()) {
            let entries=self.vegetation_pending.take().unwrap().join().unwrap_or_default();
            for data in entries {
                let mut slots=std::collections::HashMap::<String,usize>::new();
                let mut parts=Vec::new();
                for part in data.parts {
                    let candidates=world.parts_at_path(&part.path);
                    let slot=slots.entry(part.path.clone()).or_default();
                    let mesh=candidates.get(*slot).copied();*slot+=1;
                    let Some(mesh)=mesh.filter(|i|world.parts[*i].vertices.len()==part.vertices.len()) else {continue};
                    if part.bones.len()!=part.binds.len() || part.bones.iter().any(|&b|b>=data.rig.nodes.len()) {continue;}
                    if part.vertices.len()!=part.weights.len() || part.vertices.len()!=part.indices.len() {continue;}
                    if part.weights.iter().zip(&part.indices).any(|(w,i)|w.len()>4 || w.len()!=i.len() || i.iter().any(|&i|i>=part.bones.len())) {continue;}
                    let vertices=part.vertices.into_iter().zip(part.weights).zip(part.indices).map(|((position,w),i)| {
                        let mut weights=[0.0;4];let mut indices=[0;4];
                        weights[..w.len()].copy_from_slice(&w);indices[..i.len()].copy_from_slice(&i);
                        SkinVertex {position:Vec3::from_array(position),weights,indices}
                    }).collect();
                    let matrices=vec![Mat4::IDENTITY;part.bones.len()];
                    parts.push(SkinPart {mesh,vertices,bones:part.bones,binds:part.binds.iter().map(Mat4::from_cols_array).collect(),matrices});
                }
                let speed=data.speed_range.map_or(data.rig.speed,|[lo,hi]|macroquad::rand::gen_range(lo,hi));
                let phase=if data.random_start {macroquad::rand::gen_range(0.0,(data.rig.stop-data.rig.start).max(0.0001))} else {0.0};
                self.vegetation.push(SkinClip {parent:Mat4::from_cols_array(&data.rig.parent),worlds:vec![Mat4::IDENTITY;data.rig.nodes.len()],rig:data.rig,parts,speed,phase});
            }
            if std::env::var_os("RRBTD6_TIMING").is_some() {
                eprintln!("home: {} vegetation clips, {} mesh parts, {} skinned vertices",self.vegetation.len(),
                    self.vegetation.iter().map(|c|c.parts.len()).sum::<usize>(),
                    self.vegetation.iter().flat_map(|c|&c.parts).map(|p|p.vertices.len()).sum::<usize>());
            }
        }
        let reflection = Mat4::from_scale(vec3(1.0,1.0,-1.0));
        for clip in &mut self.rigid {
            let duration=(clip.data.stop-clip.data.start).max(0.0001);
            let elapsed=get_time() as f32*clip.data.speed;
            let time=clip.data.start+if clip.data.looped {elapsed.rem_euclid(duration)} else {elapsed.min(duration)};
            for (i,node) in clip.data.nodes.iter().enumerate() {
                let local=local_pose(node,time);
                clip.worlds[i]=node.parent.map_or(clip.parent,|p|clip.worlds[p])*local;
                let matrix=reflection*clip.worlds[i]*clip.inverse_rest[i]*reflection;
                for (part,original) in &clip.parts[i] {
                    for (vertex,position) in world.parts[*part].vertices.iter_mut().zip(original) {
                        vertex.position=matrix.transform_point3(*position);
                    }
                }
            }
        }
        for clip in &mut self.vegetation {
            let duration=(clip.rig.stop-clip.rig.start).max(0.0001);
            let elapsed=get_time() as f32*clip.speed+clip.phase;
            let time=clip.rig.start+if clip.rig.looped {elapsed.rem_euclid(duration)}else{elapsed.min(duration)};
            for (i,node) in clip.rig.nodes.iter().enumerate() {
                clip.worlds[i]=node.parent.map_or(clip.parent,|p|clip.worlds[p])*local_pose(node,time);
            }
            for part in &mut clip.parts {
                for (i,matrix) in part.matrices.iter_mut().enumerate(){*matrix=reflection*clip.worlds[part.bones[i]]*part.binds[i];}
                for (vertex,source) in world.parts[part.mesh].vertices.iter_mut().zip(&part.vertices){vertex.position=skin_position(source,&part.matrices);}
            }
        }
        for rotation in &self.rotations {
            let data = &rotation.data;
            let angle = (get_time() as f32 * data.speed).fract() * std::f32::consts::TAU;
            let local = Mat4::from_scale_rotation_translation(Vec3::from_array(data.scale),
                Quat::from_axis_angle(Vec3::from_array(data.axis).normalize(),angle),Vec3::from_array(data.position));
            let matrix = reflection * rotation.parent * local * rotation.inverse_rest * reflection;
            for (i,original) in &rotation.parts {
                for (vertex,position) in world.parts[*i].vertices.iter_mut().zip(original) {
                    vertex.position=matrix.transform_point3(*position);
                }
            }
        }
        let mut uploaded_trigger=false;
        if let Some(particles)=&mut self.particles {
            particles.update(get_frame_time().min(0.1));
            for (effect,last) in self.effects.iter().zip(&mut self.effect_times) {
                if let Some(began)=self.triggers.iter().find(|t|t.data.path==effect.path).and_then(|t|t.began) {
                    if *last!=Some(began) && get_time()-began>=effect.delay as f64 {
                        particles.spawn(&effect.name,effect.position,1.0);*last=Some(began);
                    }
                }
            }
        }
        for trigger in &mut self.triggers {
            if trigger.began.is_some_and(|began|get_time()-began>=trigger.data.duration as f64) {
                trigger.began=None;
                world.hide_subtree(&trigger.data.path);
            }
            if !uploaded_trigger && trigger.pending.as_ref().is_some_and(|h|h.is_finished()) {
                if let Some(staged)=trigger.pending.take().unwrap().join().ok().flatten() {
                    trigger.model=Some(AnimModel::from_staged(staged));
                    world.hide_subtree(&trigger.data.path);self.shaders.get_or_insert_with(Shaders::new);
                    uploaded_trigger=true;
                }
            }
            if trigger.began.is_some(){world.hide_subtree(&trigger.data.path);}
        }
        // Never block on unfinished I/O / PNG decode; upload at most one ready model per frame.
        if let Some(index) = self.pending.iter().position(|(_, h)| h.is_finished()).filter(|_|!uploaded_trigger) {
            let (path, handle) = self.pending.swap_remove(index);
            if let Some(staged) = handle.join().ok().flatten() {
                if world.hide_subtree(&path) {
                    self.models.push((path,AnimModel::from_staged(staged)));
                    self.shaders.get_or_insert_with(Shaders::new);
                }
            }
        }
    }
    pub(super) fn draw(&mut self, forward: Vec3) {
        let Some(shaders) = &mut self.shaders else { return };
        shaders.forward = forward;
        let time = get_time() as f32;
        for (path,model) in &mut self.models {
            if self.triggers.iter().any(|t|t.began.is_some() && &t.data.path==path){continue;}
            let clip = model.idle_clip();
            model.draw(shaders, clip, time, [0.0;3], 0.0, 0.7, false, None);
        }
        for trigger in &mut self.triggers {
            if trigger.began.is_none() && self.models.iter().any(|(p,_)|p==&trigger.data.path){continue;}
            if let Some(model)=&mut trigger.model {
                let time=trigger.began.map_or(0.0,|began|(get_time()-began) as f32);
                let clip=model.idle_clip();model.draw(shaders,clip,time,[0.0;3],0.0,0.7,false,None);
            }
        }
        if let Some(particles)=&mut self.particles {
            let right=forward.cross(Vec3::Y).normalize_or_zero();
            let up=right.cross(forward).normalize_or_zero();
            particles.draw(right,up,screen_height()/(2.0*(26.991467_f32.to_radians()*0.5).tan()*800.0),screen_height());
        }
    }
    fn collider_world(&self,trigger:&Trigger)->Mat4 {
        for clip in &self.vegetation {
            if let Some(index)=clip.rig.nodes.iter().position(|n|n.path==trigger.data.collider_path){return clip.worlds[index];}
        }
        Mat4::from_cols_array(&trigger.data.world)
    }
    pub(super) fn interact(&mut self,pointer:Vec2,vp:Mat4,world:&mut Model) {
        let inverse=vp.inverse();let ndc=vec2(pointer.x/screen_width()*2.0-1.0,1.0-pointer.y/screen_height()*2.0);
        let reflect=vec3(1.0,1.0,-1.0);
        let origin=inverse.project_point3(vec3(ndc.x,ndc.y,-1.0))*reflect;
        let far=inverse.project_point3(vec3(ndc.x,ndc.y,1.0))*reflect;
        let direction=(far-origin).normalize();
        let hit=self.triggers.iter().enumerate().filter_map(|(i,t)|ray_box(origin,direction,self.collider_world(t).inverse(),Vec3::from_array(t.data.center),Vec3::from_array(t.data.size)).map(|d|(i,d)))
            .min_by(|a,b|a.1.total_cmp(&b.1));
        // The alternate co-op display has no serialized collider of its own.
        // Pick its visible source triangles while both event previews are shown.
        let mut coop=None::<f32>;
        for (path,model) in &mut self.models {
            if !path.contains("/CoopEventMonkey/"){continue;}
            let (meshes,_)=model.meshes(model.idle_clip(),get_time() as f32,[0.0;3],0.0,0.7,false,None);
            for mesh in meshes {for triangle in mesh.indices.chunks_exact(3) {
                let point=|i:u16|mesh.vertices[i as usize].position*reflect;
                if let Some(d)=ray_triangle(origin,direction,point(triangle[0]),point(triangle[1]),point(triangle[2])) {
                    coop=Some(coop.map_or(d,|old|old.min(d)));
                }
            }}
        }
        let button=self.buttons.iter().filter_map(|b|ray_box(origin,direction,Mat4::from_cols_array(&b.world).inverse(),Vec3::from_array(b.center),Vec3::from_array(b.size)).map(|d|(b,d)))
            .min_by(|a,b|a.1.total_cmp(&b.1));
        if let Some(distance)=coop.filter(|d|hit.is_none_or(|(_,t)|*d<=t) && button.is_none_or(|(_,t)|*d<=t)) {
            let _=distance;self.action=Some("events".into());return;
        }
        if let Some((button,_))=button.filter(|(_,d)|hit.is_none_or(|(_,t)|*d<=t)) {
            self.action=Some(button.action.clone());return;
        }
        let hit=hit.map(|(i,_)|i);
        if let Some(index)=hit {
            let trigger=&mut self.triggers[index];
            if trigger.data.no_interrupt && trigger.began.is_some(){return;}
            if trigger.pending.is_some(){trigger.began=Some(get_time());return;}
            if trigger.model.is_some(){trigger.began=Some(get_time());world.hide_subtree(&trigger.data.path);}
            else{let dir=trigger.dir.clone();trigger.pending=Some(std::thread::spawn(move||AnimModel::stage(&dir)));}
            if std::env::var_os("RRBTD6_TIMING").is_some(){eprintln!("home: activate {}",trigger.data.collider_path);}
        }
    }
    pub(super) fn take_action(&mut self)->Option<String>{self.action.take()}
    pub(super) fn debug_interact(&mut self,vp:Mat4,world:&mut Model) {
        if self.debug_clicked{return;}
        let Ok(name)=std::env::var("RRBTD6_HOME_INTERACT") else{return};
        if let Some(button)=self.buttons.iter().find(|b|b.path.contains(&name)) {
            let point=Mat4::from_cols_array(&button.world).transform_point3(Vec3::from_array(button.center))*vec3(1.0,1.0,-1.0);
            let ndc=vp.project_point3(point);
            self.debug_clicked=true;
            self.interact(vec2((ndc.x+1.0)*0.5*screen_width(),(1.0-ndc.y)*0.5*screen_height()),vp,world);return;
        }
        let Some(trigger)=self.triggers.iter().find(|t|t.data.collider_path.contains(&name))else{return};
        let point=self.collider_world(trigger).transform_point3(Vec3::from_array(trigger.data.center))*vec3(1.0,1.0,-1.0);
        let ndc=vp.project_point3(point);let pointer=vec2((ndc.x+1.0)*0.5*screen_width(),(1.0-ndc.y)*0.5*screen_height());
        self.debug_clicked=true;self.interact(pointer,vp,world);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn visible_mesh_pick_is_two_sided_and_rejects_misses_and_behind() {
        let a=vec3(-1.0,-1.0,5.0);let b=vec3(1.0,-1.0,5.0);let c=vec3(0.0,1.0,5.0);
        assert_eq!(ray_triangle(Vec3::ZERO,Vec3::Z,a,b,c),Some(5.0));
        assert_eq!(ray_triangle(Vec3::ZERO,Vec3::Z,c,b,a),Some(5.0));
        assert!(ray_triangle(vec3(3.0,0.0,0.0),Vec3::Z,a,b,c).is_none());
        assert!(ray_triangle(Vec3::ZERO,-Vec3::Z,a,b,c).is_none());
    }
    #[test]
    fn native_box_hit_handles_rotated_scaled_colliders_and_parallel_misses() {
        let world=Mat4::from_scale_rotation_translation(vec3(2.0,1.0,3.0),Quat::from_rotation_y(0.6),vec3(10.0,0.0,20.0));
        let origin=world.transform_point3(vec3(0.0,0.0,-5.0));
        let direction=world.transform_vector3(Vec3::Z);
        assert!((ray_box(origin,direction,world.inverse(),Vec3::ZERO,Vec3::splat(2.0)).unwrap()-4.0).abs()<0.0001);
        assert!(ray_box(vec3(3.0,0.0,-5.0),Vec3::Z,Mat4::IDENTITY,Vec3::ZERO,Vec3::splat(2.0)).is_none());
        assert!(ray_box(vec3(0.0,0.0,5.0),Vec3::Z,Mat4::IDENTITY,Vec3::ZERO,Vec3::splat(2.0)).is_none());
    }
    #[test]
    fn bind_pose_and_weighted_bones_preserve_the_original_coordinate_system() {
        let rest=Mat4::from_scale_rotation_translation(vec3(2.0,3.0,0.5),Quat::from_rotation_y(0.7),vec3(12.0,4.0,8.0));
        let reflection=Mat4::from_scale(vec3(1.0,1.0,-1.0));
        let vertex=SkinVertex {position:vec3(1.0,2.0,3.0),weights:[0.25,0.75,0.0,0.0],indices:[0,1,0,0]};
        let rest_skin=reflection*rest*rest.inverse();
        let translated=reflection*Mat4::from_translation(vec3(8.0,0.0,0.0))*rest*rest.inverse();
        assert!(skin_position(&vertex,&[rest_skin,rest_skin]).abs_diff_eq(vec3(1.0,2.0,-3.0),0.0001));
        assert!(skin_position(&vertex,&[rest_skin,translated]).abs_diff_eq(vec3(7.0,2.0,-3.0),0.0001));
    }
}
