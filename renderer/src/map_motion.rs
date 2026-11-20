//! Map props' idle animation (tools/assets/export_map_motion.py's motion.json): each scene
//! Animator's / PlayableAnimatorSimple's clip from the start, moving the map model's own parts.
//! Rigid parts follow their node (world now x rest inverse); skinned parts (trees, bushes) are
//! re-skinned from their bones. Same evaluation as the home world (home_world.rs).
use crate::frontend::home_world::{local_pose, skin_position, SkinData, SkinVertex};
use crate::render3d::Model;
use macroquad::prelude::*;
use std::path::Path;

struct Skin { mesh: usize, vertices: Vec<SkinVertex>, bones: Vec<usize>, binds: Vec<Mat4>, matrices: Vec<Mat4> }
struct Clip {
    data: SkinData,
    parent: Mat4,
    worlds: Vec<Mat4>,
    inverse_rest: Vec<Mat4>,
    /// Per node: its rigid parts with their rest vertices.
    rigid: Vec<Vec<(usize, Vec<Vec3>)>>,
    skins: Vec<Skin>,
    speed: f32,
    phase: f32,
}
pub struct MapMotion { clips: Vec<Clip> }

impl MapMotion {
    pub fn load(dir: &Path, model: &Model) -> Option<Self> {
        let entries: Vec<SkinData> = serde_json::from_str(&std::fs::read_to_string(dir.join("motion.json")).ok()?).ok()?;
        let mut skinned = std::collections::HashSet::new();
        let mut clips = Vec::new();
        for data in entries {
            let mut slots = std::collections::HashMap::<String, usize>::new();
            let mut skins = Vec::new();
            for part in &data.parts {
                let candidates = model.parts_at_path(&part.path);
                let slot = slots.entry(part.path.clone()).or_default();
                let mesh = candidates.get(*slot).copied();
                *slot += 1;
                let Some(mesh) = mesh.filter(|&i| model.parts[i].vertices.len() == part.vertices.len()) else { continue };
                if part.bones.len() != part.binds.len() || part.bones.iter().any(|&b| b >= data.rig.nodes.len()) { continue; }
                if part.vertices.len() != part.weights.len() || part.vertices.len() != part.indices.len() { continue; }
                if part.weights.iter().zip(&part.indices).any(|(w, i)| w.len() > 4 || w.len() != i.len() || i.iter().any(|&i| i >= part.bones.len())) { continue; }
                let vertices = part.vertices.iter().zip(&part.weights).zip(&part.indices).map(|((p, w), i)| {
                    let mut weights = [0.0; 4];
                    let mut indices = [0; 4];
                    weights[..w.len()].copy_from_slice(w);
                    indices[..i.len()].copy_from_slice(i);
                    SkinVertex { position: Vec3::from_array(*p), weights, indices }
                }).collect();
                skinned.insert(mesh);
                skins.push(Skin { mesh, vertices, bones: part.bones.clone(), binds: part.binds.iter().map(Mat4::from_cols_array).collect(),
                    matrices: vec![Mat4::IDENTITY; part.bones.len()] });
            }
            let speed = data.speed_range.map_or(data.rig.speed, |[lo, hi]| macroquad::rand::gen_range(lo, hi));
            let phase = if data.random_start { macroquad::rand::gen_range(0.0, (data.rig.stop - data.rig.start).max(0.0001)) } else { 0.0 };
            clips.push(Clip {
                parent: Mat4::from_cols_array(&data.rig.parent),
                worlds: vec![Mat4::IDENTITY; data.rig.nodes.len()],
                inverse_rest: data.rig.nodes.iter().map(|n| Mat4::from_cols_array(&n.rest).inverse()).collect(),
                rigid: Vec::new(), skins, speed, phase, data,
            });
        }
        // Rigid parts: a node's own parts that no clip skins.
        for clip in &mut clips {
            clip.rigid = clip.data.rig.nodes.iter().map(|n| model.parts_at_path(&n.path).into_iter().filter(|i| !skinned.contains(i))
                .map(|i| (i, model.parts[i].vertices.iter().map(|v| v.position).collect())).collect()).collect();
        }
        if std::env::var_os("RRBTD6_TIMING").is_some() {
            eprintln!("map motion: {} clips, {} rigid parts, {} skinned parts", clips.len(),
                clips.iter().flat_map(|c| &c.rigid).map(Vec::len).sum::<usize>(), clips.iter().map(|c| c.skins.len()).sum::<usize>());
        }
        (!clips.is_empty()).then_some(Self { clips })
    }

    /// Pose every prop at `time` seconds since the map loaded.
    pub fn update(&mut self, model: &mut Model, time: f32) {
        let reflection = Mat4::from_scale(vec3(1.0, 1.0, -1.0));
        for clip in &mut self.clips {
            let rig = &clip.data.rig;
            let duration = (rig.stop - rig.start).max(0.0001);
            let elapsed = time * clip.speed + clip.phase;
            let t = rig.start + if rig.looped { elapsed.rem_euclid(duration) } else { elapsed.min(duration) };
            for (i, node) in rig.nodes.iter().enumerate() {
                clip.worlds[i] = node.parent.map_or(clip.parent, |p| clip.worlds[p]) * local_pose(node, t);
                if clip.rigid[i].is_empty() { continue; }
                let matrix = reflection * clip.worlds[i] * clip.inverse_rest[i] * reflection;
                for (part, original) in &clip.rigid[i] {
                    for (vertex, p) in model.parts[*part].vertices.iter_mut().zip(original) {
                        vertex.position = matrix.transform_point3(*p);
                    }
                }
            }
            for skin in &mut clip.skins {
                for (i, m) in skin.matrices.iter_mut().enumerate() { *m = reflection * clip.worlds[skin.bones[i]] * skin.binds[i]; }
                for (vertex, source) in model.parts[skin.mesh].vertices.iter_mut().zip(&skin.vertices) {
                    vertex.position = skin_position(source, &skin.matrices);
                }
            }
        }
    }
}
