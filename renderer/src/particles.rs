//! Unity particle-system (Shuriken) player for the game's effect prefabs, exported by
//! tools/assets/export_particles.py to effects/<Prefab>/effect.json.
//!
//! Simulated: the main module (duration, looping, start delay, lifetime / speed / size /
//! rotation / colour, gravity, max particles), emission (rate over time, bursts with cycles),
//! shape (sphere, hemisphere, cone, box, circle, edge, rectangle, donut), 3D start rotation,
//! size / colour / rotation / velocity (linear, orbital, radial, speed modifier) / force /
//! limit-velocity over lifetime,
//! texture-sheet animation and sub-emitters (birth, death); billboard, horizontal, stretched and
//! mesh renderers (Unity's Quad or the prefab's own mesh) with the material's blend and tint.
//! Also the prefab's sprites (SpriteRenderer) and meshes (MeshRenderer), moved by NinjaKiwi's
//! Custom*Animator components and the Animator's clip, all drawn in sorting order.
//! STAND-IN: per-particle randoms come from a small xorshift (Unity's own sequence isn't
//! reproduced; effects are display only and never touch the simulation); a material without
//! an exported texture (Unity's built-in Default-Particle) draws a soft round dot.

use crate::render3d::gl;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Comparison, Equation, PipelineParams};
use macroquad::models::Vertex;
use macroquad::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

// ---------------------------------------------------------------- data (effect.json)

/// MinMaxCurve: 0 constant, 1 curve, 2 random between two curves, 3 random between constants.
#[derive(Deserialize, Clone, Default)]
pub struct Curve {
    m: u8,
    s: f32,
    #[serde(default)]
    lo: f32,
    #[serde(default)]
    c: Vec<[f32; 4]>,
    #[serde(default)]
    c0: Vec<[f32; 4]>,
}

/// AnimationCurve keys [time, value, inSlope, outSlope]: cubic Hermite between keys, clamped
/// outside them.
fn eval_keys(k: &[[f32; 4]], t: f32) -> f32 {
    match k {
        [] => 1.0,
        [one] => one[1],
        _ => {
            if t <= k[0][0] {
                return k[0][1];
            }
            let last = k[k.len() - 1];
            if t >= last[0] {
                return last[1];
            }
            let i = k.iter().position(|x| x[0] > t).unwrap_or(k.len() - 1).max(1);
            let (a, b) = (k[i - 1], k[i]);
            let dt = (b[0] - a[0]).max(1e-6);
            let u = (t - a[0]) / dt;
            let (u2, u3) = (u * u, u * u * u);
            let h00 = 2.0 * u3 - 3.0 * u2 + 1.0;
            let h10 = u3 - 2.0 * u2 + u;
            let h01 = -2.0 * u3 + 3.0 * u2;
            let h11 = u3 - u2;
            h00 * a[1] + h10 * dt * a[3] + h01 * b[1] + h11 * dt * b[2]
        }
    }
}

impl Curve {
    /// Value at normalized time `t` for a particle whose random for this property is `r`.
    fn at(&self, t: f32, r: f32) -> f32 {
        match self.m {
            0 => self.s,
            1 => self.s * eval_keys(&self.c, t),
            2 => self.s * (eval_keys(&self.c0, t) + (eval_keys(&self.c, t) - eval_keys(&self.c0, t)) * r),
            _ => self.lo + (self.s - self.lo) * r,
        }
    }
}

#[derive(Deserialize, Clone, Default)]
pub struct Grad {
    c: Vec<[f32; 4]>,
    a: Vec<[f32; 2]>,
    #[serde(default)]
    fixed: bool,
}

impl Grad {
    fn at(&self, t: f32) -> [f32; 4] {
        let rgb = |t: f32| -> [f32; 3] {
            let k = &self.c;
            if k.is_empty() {
                return [1.0; 3];
            }
            if t <= k[0][0] {
                return [k[0][1], k[0][2], k[0][3]];
            }
            for w in k.windows(2) {
                if t <= w[1][0] {
                    if self.fixed {
                        return [w[1][1], w[1][2], w[1][3]];
                    }
                    let u = (t - w[0][0]) / (w[1][0] - w[0][0]).max(1e-6);
                    return [1, 2, 3].map(|i| w[0][i] + (w[1][i] - w[0][i]) * u);
                }
            }
            let l = k[k.len() - 1];
            [l[1], l[2], l[3]]
        };
        let alpha = |t: f32| -> f32 {
            let k = &self.a;
            if k.is_empty() {
                return 1.0;
            }
            if t <= k[0][0] {
                return k[0][1];
            }
            for w in k.windows(2) {
                if t <= w[1][0] {
                    if self.fixed {
                        return w[1][1];
                    }
                    let u = (t - w[0][0]) / (w[1][0] - w[0][0]).max(1e-6);
                    return w[0][1] + (w[1][1] - w[0][1]) * u;
                }
            }
            k[k.len() - 1][1]
        };
        let [r, g, b] = rgb(t);
        [r, g, b, alpha(t)]
    }
}

/// MinMaxGradient (Unity's serialized minMaxState): 0 colour, 1 gradient, 2 random between two
/// colours, 3 random between two gradients, 4 random colour from the gradient.
#[derive(Deserialize, Clone, Default)]
pub struct ColorSpec {
    m: u8,
    c: [f32; 4],
    c0: [f32; 4],
    g: Grad,
    g0: Grad,
}

fn mix4(a: [f32; 4], b: [f32; 4], u: f32) -> [f32; 4] {
    [0, 1, 2, 3].map(|i| a[i] + (b[i] - a[i]) * u)
}

impl ColorSpec {
    fn at(&self, t: f32, r: f32) -> [f32; 4] {
        match self.m {
            0 => self.c,
            1 => self.g.at(t),
            2 => mix4(self.c0, self.c, r),
            3 => mix4(self.g0.at(t), self.g.at(t), r),
            _ => self.g.at(r),
        }
    }
}

#[derive(Deserialize, Clone)]
struct Init {
    lifetime: Curve,
    speed: Curve,
    color: ColorSpec,
    size: Curve,
    #[serde(default)]
    size3d: bool,
    size_y: Curve,
    rotation: Curve,
    /// 3D start rotation (startRotationX / Y; `rotation` is Z), radians.
    #[serde(default)]
    rotation3d: bool,
    #[serde(default)]
    rotation_x: Curve,
    #[serde(default)]
    rotation_y: Curve,
    gravity: Curve,
    #[serde(default = "thousand")]
    max: u32,
}

fn thousand() -> u32 {
    1000
}

#[derive(Deserialize, Clone)]
struct Burst {
    time: f32,
    count: Curve,
    cycles: u32,
    interval: f32,
    probability: f32,
}

#[derive(Deserialize, Clone)]
struct Emission {
    rate: Curve,
    #[serde(default)]
    bursts: Vec<Burst>,
}

#[derive(Deserialize, Clone)]
struct Shape {
    #[serde(rename = "type")]
    kind: u8,
    angle: f32,
    radius: f32,
    thickness: f32,
    arc: f32,
    #[serde(rename = "box")]
    box_: [f32; 3],
    pos: [f32; 3],
    rot: [f32; 3],
    random_dir: f32,
    spherize: f32,
    /// Donut: the tube's radius (the ring's is `radius`).
    #[serde(default)]
    donut: f32,
}

#[derive(Deserialize, Clone)]
struct Axes {
    x: Curve,
    y: Option<Curve>,
    z: Curve,
    #[serde(default)]
    separate: bool,
}

#[derive(Deserialize, Clone)]
struct Vel {
    x: Curve,
    y: Curve,
    z: Curve,
    #[serde(default)]
    world: bool,
    /// VelocityOverLifetime orbital (radians a second about the system's X / Y / Z, around its
    /// centre + offset), radial (units a second away from that centre) and speedModifier (x the
    /// particle's velocity). ForceOverLifetime has none of these.
    #[serde(default)]
    orbital: Option<Vec<Curve>>,
    #[serde(default)]
    radial: Option<Curve>,
    #[serde(default)]
    speed_mod: Option<Curve>,
    #[serde(default)]
    offset: Option<Vec<Curve>>,
}

#[derive(Deserialize, Clone)]
struct Limit {
    speed: Curve,
    dampen: f32,
}

#[derive(Deserialize, Clone)]
struct Sheet {
    x: u32,
    y: u32,
    frame: Curve,
    start: Curve,
    cycles: f32,
}

/// A sub-emitter (SubModule): the node of the ParticleSystem it drives, Birth (0) / Collision (1) /
/// Death (2) / Trigger (3) / Manual (4), inherited properties (1 colour, 2 size, 4 rotation,
/// 8 lifetime, 16 duration) and emit probability.
#[derive(Deserialize, Clone)]
struct SubJson {
    emitter: usize,
    #[serde(default, rename = "type")]
    kind: u8,
    #[serde(default)]
    props: u32,
    #[serde(default = "one_f")]
    prob: f32,
}

fn one_f() -> f32 {
    1.0
}

#[derive(Deserialize, Clone)]
struct System {
    duration: f32,
    looping: bool,
    #[serde(default)]
    prewarm: bool,
    /// ParticleSystemScalingMode: 0 Hierarchy, 1 Local (own transform only), 2 Shape (positions
    /// only, not sizes).
    #[serde(default = "one_u8")]
    scaling: u8,
    /// simulationSpace: 0 local (particles move with the effect), 1 world (they stay where
    /// they were emitted: a projectile's trail).
    #[serde(default)]
    space: u8,
    delay: Curve,
    #[serde(default = "one")]
    speed: f32,
    init: Init,
    emission: Option<Emission>,
    shape: Option<Shape>,
    size: Option<Axes>,
    color: Option<ColorSpec>,
    rotation: Option<Axes>,
    velocity: Option<Vel>,
    force: Option<Vel>,
    limit: Option<Limit>,
    sheet: Option<Sheet>,
    /// CustomDataModule: Custom1 / Custom2 as (mode: 1 vector, [x, y, z, w] over the
    /// particle's life), for the shader's custom vertex streams.
    #[serde(default)]
    custom: Vec<(u8, Vec<Curve>)>,
    /// SubModule: sub-emitters.
    #[serde(default)]
    sub: Vec<SubJson>,
}

fn one() -> f32 {
    1.0
}

fn one_u8() -> u8 {
    1
}

#[derive(Deserialize, Clone, Default)]
struct Renderer {
    mode: u8,
    #[serde(default = "two")]
    length: f32,
    #[serde(default)]
    vel_scale: f32,
    #[serde(default = "half")]
    max_size: f32,
    #[serde(default)]
    align: u8,
    #[serde(default = "yes")]
    enabled: bool,
    #[serde(default)]
    blend: String,
    tint: Option<[f32; 4]>,
    #[serde(default)]
    tint_x2: bool,
    texture: Option<String>,
    /// The renderer's material (absent when it couldn't be resolved at export).
    material: Option<String>,
    /// URP Particles/Unlit _ColorMode: 0 multiply (texture x tint x particle colour),
    /// 1 additive (texture x tint + particle colour).
    #[serde(default)]
    color_mode: u8,
    /// A material drawn with the game's translated shader (export_particles.py TRANSLATED: the
    /// Wizard's Molotov fire), its properties and render queue; fx.rs draws it.
    #[serde(default)]
    shader_file: Option<String>,
    /// Its variant reading TEXCOORD0.zw from the vertex normal's xy, and what goes there (the
    /// custom vertex streams after UV: "c1x".."c2w", "age").
    #[serde(default)]
    shader_file_uvzw: Option<String>,
    #[serde(default)]
    uv_extra: Vec<String>,
    #[serde(default)]
    props: Option<serde_json::Value>,
    #[serde(default)]
    queue: Option<i64>,
    /// Renderer.sortingOrder: transparent renderers (particles and sprites) draw in this order.
    #[serde(default)]
    order: i32,
    /// Mesh particles (mode 4): pmesh_<name>.json, the mesh each particle draws (absent for
    /// Unity's built-in Quad).
    #[serde(default)]
    mesh_file: Option<String>,
}

/// A SpriteRenderer (export_particles.py sprite_node): its sprite image, world size, pivot,
/// colour and sorting order; Sprites-Default draws texture x colour, alpha blended.
#[derive(Deserialize, Clone)]
struct SpriteJson {
    texture: String,
    size: [f32; 2],
    pivot: [f32; 2],
    color: [f32; 4],
    #[serde(default)]
    flip: [bool; 2],
    #[serde(default)]
    order: i32,
    #[serde(default = "yes")]
    enabled: bool,
}

/// A MeshFilter + MeshRenderer (mesh_node): mesh_<name>.json (x y z u v, indices) and its
/// material's texture and tint. The monkey materials (NinjaKiwi/BTD6/LitMonkey) are unlit:
/// colour = texture (see anim.rs).
#[derive(Deserialize, Clone)]
struct MeshPartJson {
    file: String,
    #[serde(default)]
    order: i32,
    #[serde(default = "yes")]
    enabled: bool,
    texture: Option<String>,
    tint: Option<[f32; 4]>,
}

/// A NinjaKiwi CustomAnimationComponent (anim_node): color / scale / rotation of node `target`.
#[derive(Deserialize, Clone)]
struct AnimJson {
    kind: String,
    target: usize,
    #[serde(default = "one")]
    duration: f32,
    #[serde(default)]
    curve: Vec<[f32; 4]>,
    #[serde(default = "yes")]
    playing: bool,
    #[serde(default, rename = "loop")]
    looping: bool,
    #[serde(default)]
    stop_on_end: bool,
    #[serde(default)]
    disable_on_end: bool,
    #[serde(default = "one")]
    speed: f32,
    #[serde(default = "yes")]
    enabled: bool,
    c1: Option<[f32; 4]>,
    c2: Option<[f32; 4]>,
    #[serde(default)]
    rotations: f32,
    #[serde(default)]
    axis: u8,
    #[serde(default = "yes")]
    local: bool,
    #[serde(default)]
    random_angle: bool,
    #[serde(default)]
    random_speed: bool,
    speed_range: Option<[f32; 2]>,
}

/// An Animator's default state (animator_clip): its clip sampled every 1/60 s per node.
#[derive(Deserialize, Clone)]
struct ClipJson {
    len: f32,
    #[serde(default, rename = "loop")]
    looping: bool,
    #[serde(default = "one")]
    speed: f32,
    tracks: Vec<TrackJson>,
}

/// attr 1 localPosition, 2 localRotation (x y z w), 3 localScale, 4 localEulerAngles.
#[derive(Deserialize, Clone)]
struct TrackJson {
    node: usize,
    attr: u8,
    samples: Vec<Vec<f32>>,
}

fn two() -> f32 {
    2.0
}
fn half() -> f32 {
    0.5
}
fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct NodeJson {
    #[serde(default)]
    name: String,
    #[serde(default = "yes")]
    active: bool,
    pos: [f32; 3],
    rot: [f32; 4],
    scale: [f32; 3],
    parent: Option<usize>,
    system: Option<System>,
    renderer: Option<Renderer>,
    #[serde(default)]
    sprite: Option<SpriteJson>,
    #[serde(default)]
    mesh_part: Option<MeshPartJson>,
    #[serde(default)]
    anims: Vec<AnimJson>,
    #[serde(default)]
    clip: Option<ClipJson>,
    #[serde(default)]
    text: Option<TextJson>,
}

/// A world-space TextMeshPro node (export_particles.py text_node): its placeholder text, font
/// size (fontSize; world units fontSize / 10), face colour, four-corner gradient, TMP alignment
/// flags and rect. Drawn by the caller (`Particles::texts`), with the title font.
#[derive(Deserialize, Clone)]
pub struct TextJson {
    pub text: String,
    pub size: f32,
    pub color: [f32; 4],
    #[serde(default)]
    pub gradient: Option<[[f32; 4]; 4]>,
    #[serde(default)]
    pub h_align: u32,
    #[serde(default)]
    pub v_align: u32,
    #[serde(default)]
    pub rect: [f32; 2],
    #[serde(default)]
    pub pivot: [f32; 2],
}

/// A live effect's text where its node is now: position (effect world, before `gl`), the
/// node's world scale (x, y), the text (the instance's own, else the prefab's) and its style.
pub struct WorldText {
    pub at: Vec3,
    /// The node's world rotation (its text plane: local X right, Y up).
    pub rot: Quat,
    pub scale: Vec2,
    pub text: String,
    pub style: TextJson,
}

#[derive(Deserialize)]
struct EffectJson {
    /// The root has an enabled SpriteRenderer: shown as a sprite display (`spawn_display`).
    #[serde(default)]
    sprite_display: bool,
    nodes: Vec<NodeJson>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Blend {
    Alpha,
    Add,
    Premul,
}

struct SysDef {
    /// Its node's index (sorting ties: hierarchy order).
    node: usize,
    sys: System,
    ren: Renderer,
    /// Node transform relative to the prefab root (Unity space).
    rot: Quat,
    pos: Vec3,
    /// Scale applied to emission positions, and to particle sizes (per scalingMode).
    scale: Vec3,
    size_scale: f32,
    tex: Texture2D,
    blend: Blend,
    tint: [f32; 4],
    add_color: bool,
    /// Drawn with this translated shader instead of the particle material (`Renderer::shader_file`).
    fx: Option<Rc<crate::fx::Fx>>,
    /// The particle mesh (local x y z, uv; triangle indices) when it isn't a quad.
    pmesh: Option<Rc<(Vec<(Vec3, Vec2)>, Vec<u16>)>>,
}

pub struct EffectDef {
    systems: Vec<SysDef>,
    /// Every node's parent and local transform (prefab space) and active flag: sprites and meshes
    /// are placed through them each frame, since animators change them.
    nodes: Vec<NodeDef>,
    sprites: Vec<SpriteDef>,
    meshes: Vec<MeshDef>,
    anims: Vec<AnimJson>,
    /// The node each animator component is on.
    anim_owner: Vec<usize>,
    clips: Vec<(usize, ClipJson)>,
    /// Each node's exported world (prefab-root) matrix, and per system whether an animator moves
    /// it or an ancestor (its particles then follow the node's transform as it is each frame).
    baked: Vec<Mat4>,
    sys_animated: Vec<bool>,
    /// Per system: its sub-emitters (system index, kind, inherited properties, probability).
    subs: Vec<Vec<(usize, u8, u32, f32)>>,
    /// Systems that are some system's sub-emitter: they emit only for their parent's particles.
    is_sub: Vec<bool>,
    /// Text nodes (node index, style).
    texts: Vec<(usize, TextJson)>,
    /// UnityDisplayNode.isSprite for this prefab as a display (its root's SpriteRenderer).
    sprite_display: bool,
}

struct NodeDef {
    /// Names from the prefab root ("Root/Child/..."): what a model's attach paths name.
    path: String,
    parent: Option<usize>,
    pos: Vec3,
    rot: Quat,
    scale: Vec3,
    active: bool,
}

struct SpriteDef {
    node: usize,
    tex: Texture2D,
    size: Vec2,
    pivot: Vec2,
    color: [f32; 4],
    flip: [bool; 2],
    order: i32,
}

struct MeshDef {
    node: usize,
    order: i32,
    tex: Texture2D,
    tint: [f32; 4],
    /// Local vertices (prefab-node space) and triangle indices.
    verts: Vec<(Vec3, Vec2)>,
    idx: Vec<u16>,
}

// ---------------------------------------------------------------- loading

fn soft_dot() -> Texture2D {
    thread_local! {
        static DOT: Texture2D = {
            let n = 64u16;
            let mut img = Image::gen_image_color(n, n, Color::new(1.0, 1.0, 1.0, 0.0));
            for y in 0..n {
                for x in 0..n {
                    let d = ((x as f32 + 0.5 - 32.0).powi(2) + (y as f32 + 0.5 - 32.0).powi(2)).sqrt() / 32.0;
                    let a = (1.0 - d).clamp(0.0, 1.0);
                    img.set_pixel(x as u32, y as u32, Color::new(1.0, 1.0, 1.0, a * a));
                }
            }
            let t = Texture2D::from_image(&img);
            t.set_filter(FilterMode::Linear);
            t
        };
    }
    DOT.with(|d| d.clone())
}

impl EffectDef {
    pub fn load(dir: &Path) -> Option<EffectDef> {
        let _trace=crate::load_work::Trace::new("particles",dir);
        Self::load_inner(dir,true)
    }

    fn load_inner(dir:&Path,share_textures:bool)->Option<EffectDef>{
        let j: EffectJson = serde_json::from_str(&std::fs::read_to_string(dir.join("effect.json")).ok()?).ok()?;
        Self::from_json(dir,j,share_textures,HashMap::new())
    }

    pub async fn load_cooperative(dir:PathBuf)->Option<EffectDef>{
        let worker_dir=dir.clone();
        let (j,names,images)=crate::load_work::spawn(move||{
            let j:EffectJson=serde_json::from_slice(&std::fs::read(worker_dir.join("effect.json")).ok()?).ok()?;
            let mut names:Vec<_>=j.nodes.iter().filter_map(|n|n.renderer.as_ref()).filter_map(|r|r.texture.clone()).collect();
            names.sort();names.dedup();
            let paths:Vec<_>=names.iter().map(|name|worker_dir.join(name)).collect();
            let images=crate::render3d::decode_batch(&paths);
            Some((j,names,images))
        }).await?;
        let mut textures=HashMap::new();let mut budget=crate::load_work::Budget::new();
        for (name,image) in names.into_iter().zip(images){
            if let Some(image)=image{textures.insert(name,crate::render3d::upload_cooperative(&image,false).await);}
            budget.checkpoint().await;
        }
        Self::from_json(&dir,j,true,textures)
    }

    fn from_json(dir:&Path,j:EffectJson,share_textures:bool,mut textures:HashMap<String,Texture2D>)->Option<EffectDef>{
        // Several systems in a prefab use the same texture. Keep their systems/materials
        // independent, but decode/upload each identical path once for this definition.
        // World (prefab-root) transforms: parents come before children in the export.
        let mut world: Vec<(Vec3, Quat, Vec3, bool)> = Vec::with_capacity(j.nodes.len());
        let mut systems = Vec::new();
        let (mut nodes, mut sprites, mut meshes, mut anims, mut anim_owner, mut clips) = (Vec::<NodeDef>::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut texts = Vec::new();
        let mut load_tex = |name: &str, textures: &mut HashMap<String, Texture2D>| -> Option<Texture2D> {
            if let Some(t) = textures.get(name) {
                return Some(t.clone());
            }
            let b = std::fs::read(dir.join(name)).ok()?;
            let t = Texture2D::from_file_with_format(&b, None);
            t.set_filter(FilterMode::Linear);
            textures.insert(name.to_string(), t.clone());
            Some(t)
        };
        for (ni, n) in j.nodes.iter().enumerate() {
            let path = match n.parent.and_then(|pi| nodes.get(pi)) {
                Some(pn) => format!("{}/{}", pn.path, n.name),
                None => n.name.clone(),
            };
            nodes.push(NodeDef {
                path,
                parent: n.parent,
                pos: Vec3::from_array(n.pos),
                rot: Quat::from_array(n.rot).normalize(),
                scale: Vec3::from_array(n.scale),
                active: n.active,
            });
            if let Some(sp) = n.sprite.as_ref().filter(|s| s.enabled) {
                if let Some(tex) = load_tex(&sp.texture, &mut textures) {
                    sprites.push(SpriteDef { node: ni, tex, size: Vec2::from_array(sp.size), pivot: Vec2::from_array(sp.pivot), color: sp.color, flip: sp.flip, order: sp.order });
                }
            }
            if let Some(mp) = n.mesh_part.as_ref().filter(|m| m.enabled) {
                let tex = mp.texture.as_deref().and_then(|t| load_tex(t, &mut textures));
                let mesh: Option<serde_json::Value> = std::fs::read(dir.join(&mp.file)).ok().and_then(|b| serde_json::from_slice(&b).ok());
                if let (Some(tex), Some(mesh)) = (tex, mesh) {
                    let verts: Vec<(Vec3, Vec2)> = mesh["v"].as_array().map(|a| a.iter().filter_map(|v| {
                        let f: Vec<f32> = v.as_array()?.iter().filter_map(|x| x.as_f64().map(|x| x as f32)).collect();
                        (f.len() >= 5).then(|| (vec3(f[0], f[1], f[2]), vec2(f[3], f[4])))
                    }).collect()).unwrap_or_default();
                    let idx: Vec<u16> = mesh["i"].as_array().map(|a| a.iter().filter_map(|x| x.as_u64().map(|x| x as u16)).collect()).unwrap_or_default();
                    if verts.len() <= u16::MAX as usize && !idx.is_empty() {
                        meshes.push(MeshDef { node: ni, order: mp.order, tex, tint: mp.tint.unwrap_or([1.0; 4]), verts, idx });
                    }
                }
            }
            for a in n.anims.iter().filter(|a| a.enabled && a.target < j.nodes.len()) {
                anims.push(a.clone());
                anim_owner.push(ni);
            }
            if let Some(c) = &n.clip {
                clips.push((ni, c.clone()));
            }
            if let Some(t) = &n.text {
                texts.push((ni, t.clone()));
            }
        }
        for (node, n) in j.nodes.iter().enumerate() {
            let (lp, lr, ls) = (Vec3::from_array(n.pos), Quat::from_array(n.rot).normalize(), Vec3::from_array(n.scale));
            let (p, r, s, active) = match n.parent.and_then(|i| world.get(i)) {
                Some(&(pp, pr, ps, pa)) => (pp + pr * (lp * ps), pr * lr, ps * ls, pa && n.active),
                None => (Vec3::ZERO, lr, ls, n.active),
            };
            world.push((p, r, s, active));
            let (Some(sys), Some(ren)) = (&n.system, &n.renderer) else { continue };
            if !active || !ren.enabled || ren.mode == 5 || (sys.emission.is_none()) {
                continue;
            }
            // No material (it lives in a file the export couldn't follow): nothing faithful to
            // draw, e.g. the GodRay-shaded light shards on Tinkerton.
            if ren.material.is_none() {
                continue;
            }
            let tex = ren
                .texture
                .as_ref()
                .and_then(|name| {
                    if share_textures{if let Some(t)=textures.get(name){return Some(t.clone());}}
                    let b=std::fs::read(dir.join(name)).ok()?;
                    let t = Texture2D::from_file_with_format(&b, None);
                    t.set_filter(FilterMode::Linear);
                    if share_textures{textures.insert(name.clone(),t.clone());}
                    Some(t)
                })
                .unwrap_or_else(soft_dot);
            let blend = match ren.blend.as_str() {
                "add" => Blend::Add,
                "premul" => Blend::Premul,
                _ => Blend::Alpha,
            };
            let mut tint = ren.tint.unwrap_or([1.0; 4]);
            if ren.tint_x2 {
                tint = tint.map(|v| v * 2.0);
            }
            let (scale, size_scale) = match sys.scaling {
                0 => (s, s.x.abs()),
                2 => (s, 1.0),
                _ => (ls, ls.x.abs()),
            };
            let add_color = ren.color_mode == 1;
            // fx.rs finds shaders two dirs above the models dir it's given: <effects>/../shaders.
            let file = if ren.uv_extra.is_empty() { ren.shader_file.as_ref() } else { ren.shader_file_uvzw.as_ref().or(ren.shader_file.as_ref()) };
            let fx = file.and_then(|file| {
                let part = serde_json::json!({"shader_file": file, "props": ren.props.clone().unwrap_or_default(), "queue": ren.queue.unwrap_or(-1)});
                let models = dir.parent()?.parent()?.join("models").join("_");
                crate::fx::Fx::new(&models, &part, |f| {
                    let b = std::fs::read(dir.join(f)).ok()?;
                    let t = Texture2D::from_file_with_format(&b, None);
                    t.set_filter(FilterMode::Linear);
                    Some(t)
                })
                .map(Rc::new)
            });
            let pmesh = if ren.mode == 4 {
                ren.mesh_file.as_ref().and_then(|f| {
                    let m: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join(f)).ok()?).ok()?;
                    let verts: Vec<(Vec3, Vec2)> = m["v"].as_array()?.iter().filter_map(|v| {
                        let f: Vec<f32> = v.as_array()?.iter().filter_map(|x| x.as_f64().map(|x| x as f32)).collect();
                        (f.len() >= 5).then(|| (vec3(f[0], f[1], f[2]), vec2(f[3], f[4])))
                    }).collect();
                    let idx: Vec<u16> = m["i"].as_array()?.iter().filter_map(|x| x.as_u64().map(|x| x as u16)).collect();
                    (!idx.is_empty() && verts.len() < 4000).then(|| Rc::new((verts, idx)))
                })
            } else {
                None
            };
            systems.push(SysDef { node, pmesh, sys: sys.clone(), ren: ren.clone(), rot: r, pos: p, scale, size_scale, tex, blend, tint, add_color, fx });
        }
                let baked: Vec<Mat4> = world.iter().map(|&(p, r, sc, _)| Mat4::from_scale_rotation_translation(sc, r, p)).collect();
        // Nodes an animator or Animator track moves, and everything below them.
        let mut moved = vec![false; nodes.len()];
        for a in &anims {
            moved[a.target] = true;
        }
        for (_, c) in &clips {
            for t in &c.tracks {
                if t.node < moved.len() {
                    moved[t.node] = true;
                }
            }
        }
        for i in 0..nodes.len() {
            if let Some(pi) = nodes[i].parent {
                if pi < i && moved[pi] {
                    moved[i] = true;
                }
            }
        }
        let sys_animated = systems.iter().map(|sd| moved.get(sd.node).copied().unwrap_or(false)).collect();
        let by_node: HashMap<usize, usize> = systems.iter().enumerate().map(|(k, sd)| (sd.node, k)).collect();
        let subs: Vec<Vec<(usize, u8, u32, f32)>> = systems
            .iter()
            .map(|sd| sd.sys.sub.iter().filter_map(|e| by_node.get(&e.emitter).map(|&k| (k, e.kind, e.props, e.prob))).collect())
            .collect();
        let mut is_sub = vec![false; systems.len()];
        for list in &subs {
            for &(k, ..) in list {
                is_sub[k] = true;
            }
        }
        Some(EffectDef { systems, nodes, sprites, meshes, anims, anim_owner, clips, baked, sys_animated, subs, is_sub, texts, sprite_display: j.sprite_display })
    }
}

/// Compare the original per-system upload with shared textures on the real GPU.
pub async fn verify_texture_sharing(root:&Path){
    let name="proj_AscendedShadowBombTagsActive";let dir=root.join("effects").join(name);
    let reference=Rc::new(EffectDef::load_inner(&dir,false).expect("particle audit prefab missing"));
    let shared=Rc::new(EffectDef::load_inner(&dir,true).unwrap());
    let prepared=Rc::new(EffectDef::load_cooperative(dir.clone()).await.unwrap());
    assert_eq!(reference.systems.len(),shared.systems.len());
    for (a,b) in reference.systems.iter().zip(&shared.systems){assert_eq!(a.tex.get_texture_data().bytes,b.tex.get_texture_data().bytes);}
    assert_eq!(reference.systems.len(),prepared.systems.len());
    for (a,b) in reference.systems.iter().zip(&prepared.systems){assert_eq!(a.tex.get_texture_data().bytes,b.tex.get_texture_data().bytes);}
    let unique=|d:&EffectDef|{let mut ids=Vec::new();let mut bytes=0usize;for s in &d.systems{let id=s.tex.raw_miniquad_id();if !ids.contains(&id){ids.push(id);bytes+=s.tex.width() as usize*s.tex.height() as usize*4;}}(ids.len(),bytes)};
    let (old,new)=(unique(&reference),unique(&shared));assert!(new.0<old.0);
    let rt=render_target(256,256);let mut camera=Camera2D::from_display_rect(Rect::new(0.0,0.0,256.0,256.0));camera.render_target=Some(rt.clone());
    for size in [28.0,47.5,63.0]{
        let render=|def:&EffectDef|{
            set_camera(&camera);clear_background(BLACK);
            for (i,system) in def.systems.iter().enumerate(){draw_texture_ex(&system.tex,(i%3) as f32*80.0,(i/3) as f32*80.0,WHITE,DrawTextureParams{dest_size:Some(vec2(size,size)),..Default::default()});}
            set_default_camera();rt.texture.get_texture_data().bytes
        };
        assert_eq!(render(&reference),render(&shared),"particle texture sampling changed pixels");
        assert_eq!(render(&reference),render(&prepared),"prepared particle sampling changed pixels");
    }
    eprintln!("particle-texture-check: all texture readbacks and 3 sampling renders match; uploads {} -> {}, texture bytes {} -> {}",old.0,new.0,old.1,new.1);
}

/// Opt-in proof that preparing upgrade definitions leaves particle RNG and
/// the subsequently spawned animation unchanged, including all four FX families.
pub async fn verify_upgrade_preparation(root:&Path){
    let target=render_target_ex(256,256,RenderTargetParams{sample_count:1,depth:true});
    let mut camera=crate::render3d::camera([0.0;2],256.0,256.0);camera.fovy=160.0;camera.render_target=Some(target.clone());
    let angle=crate::render3d::PITCH_DEG.to_radians();let up=gl(0.0,angle.cos(),angle.sin());
    let mut comparisons=0;
    for name in ["UpgradeEffect","UpgradeEffectTier5","HeroUpgradeEffect","HeroUpgradeEffectLv20"]{
        let mut reference=Particles::new(root);let mut prepared=Particles::new(root);
        let definition=EffectDef::load_cooperative(root.join("effects").join(name)).await.expect("upgrade effect fixture");
        prepared.retain_prepared(name.to_owned(),Some(definition));
        assert_eq!(reference.rng.0,prepared.rng.0);assert!(prepared.live.is_empty());
        assert!(reference.spawn(name,[0.0;3],1.0));assert!(prepared.spawn(name,[0.0;3],1.0));
        let mut visible=false;
        for _ in 0..20{
            reference.update(1.0/60.0);prepared.update(1.0/60.0);
            assert_eq!(reference.rng.0,prepared.rng.0,"particle random sequence changed for {name}");
            let render=|particles:&mut Particles|{
                set_camera(&camera);clear_background(BLACK);
                particles.draw(vec3(1.0,0.0,0.0),up,256.0/camera.fovy,256.0);
                set_default_camera();target.texture.get_texture_data().bytes
            };
            let expected=render(&mut reference);let actual=render(&mut prepared);
            visible|=expected.chunks_exact(4).any(|p|p[0]!=0||p[1]!=0||p[2]!=0);
            assert_eq!(expected,actual,"prepared upgrade animation pixels changed for {name}");comparisons+=1;
        }
        assert!(visible,"upgrade audit must render visible particles for {name}");
    }
    eprintln!("upgrade-preparation: {comparisons} exact particle-animation GPU comparisons and RNG checks passed");
}

// ---------------------------------------------------------------- simulation

struct Particle {
    /// Where the effect was when this world-space particle was emitted (zero for local space).
    origin: Vec3,
    pos: Vec3,
    vel: Vec3,
    /// Last step's total velocity (its own + velocity over lifetime, orbital, radial, speed
    /// modifier): Unity's totalVelocity, which stretched billboards follow.
    tvel: Vec3,
    age: f32,
    life: f32,
    size: Vec2,
    rot: f32,
    /// 3D rotation about X / Y (start rotation + rotation over lifetime's separate axes); `rot`
    /// is Z.
    rot3: Vec2,
    color: [f32; 4],
    /// Per-particle randoms for curves / gradients (Unity draws one per property).
    r: [f32; 8],
    /// Birth sub-emitters (up to two): their emission accumulators and bursts fired.
    sub_acc: [f32; 2],
    sub_fired: [u16; 2],
}

struct SysState {
    delay: f32,
    emit_acc: f32,
    bursts_fired: Vec<u32>,
    parts: Vec<Particle>,
}

struct Instance {
    def: Rc<EffectDef>,
    at: Vec3,
    scale: f32,
    states: Vec<SysState>,
    age: f32,
    /// A map's ambient effect: plays until the map changes.
    ambient: bool,
    /// Set for effects owned by something (a tower's own flames): kept until `kill`.
    tag: Option<u64>,
    /// No more emission (its projectile is gone): ends once its particles have.
    stopped: bool,
    /// Per animator: elapsed (in durations), playing, speedMultiplier, angleOffset.
    anim_state: Vec<(f32, bool, f32, f32)>,
    /// Nodes deactivated by an animator's disableOnEnd.
    disabled: Vec<usize>,
    /// Per Animator: its state's time (seconds).
    clip_t: Vec<f32>,
    /// Replaces the text nodes' placeholder (Simulation.CreateTextEffect's text).
    text: Option<String>,
    /// The display's own rotation (Unity space) over the whole prefab: identity, or a sprite
    /// display's (UnityDisplayNode.IsSprite: RotationCache, Euler(60, 0, -rotation)).
    rot: Quat,
    /// Per node: its world matrix (prefab-root space) where a model's animation puts it (the
    /// joint a tower's particle system hangs from), else none.
    overrides: Vec<Option<Mat4>>,
}

struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }

    fn unit(&mut self) -> Vec3 {
        let z = self.next() * 2.0 - 1.0;
        let a = self.next() * std::f32::consts::TAU;
        let r = (1.0 - z * z).max(0.0).sqrt();
        vec3(r * a.cos(), r * a.sin(), z)
    }
}

/// Shape emission point and direction in the system's local space.
fn emit_shape(sh: Option<&Shape>, rng: &mut Rng) -> (Vec3, Vec3) {
    let Some(sh) = sh else { return (Vec3::ZERO, Vec3::Z) };
    let rad = |rng: &mut Rng| sh.radius * (1.0 - sh.thickness * rng.next()).max(0.0).sqrt().max(1.0 - sh.thickness);
    let arc = |rng: &mut Rng| rng.next() * sh.arc.to_radians();
    let (p, d) = match sh.kind {
        0 | 1 => {
            let u = rng.unit();
            let r = sh.radius * (1.0 - sh.thickness * rng.next()).cbrt();
            (u * r, u)
        }
        2 | 3 => {
            let mut u = rng.unit();
            u.z = u.z.abs();
            let r = sh.radius * (1.0 - sh.thickness * rng.next()).cbrt();
            (u * r, u)
        }
        4 | 7 | 8 | 9 => {
            let a = arc(rng);
            let f = (1.0 - sh.thickness * rng.next()).sqrt().max(1.0 - sh.thickness);
            let v = vec2(a.cos(), a.sin()) * f;
            let s = sh.angle.to_radians().sin();
            let p = vec3(v.x, v.y, 0.0) * sh.radius;
            (p, vec3(v.x * s, v.y * s, sh.angle.to_radians().cos()).normalize_or_zero())
        }
        5 | 15 | 16 => (
            vec3((rng.next() - 0.5) * sh.box_[0], (rng.next() - 0.5) * sh.box_[1], (rng.next() - 0.5) * sh.box_[2]),
            Vec3::Z,
        ),
        10 | 11 => {
            let a = arc(rng);
            let v = vec3(a.cos(), a.sin(), 0.0);
            (v * rad(rng), v)
        }
        12 => (vec3((rng.next() * 2.0 - 1.0) * sh.radius, 0.0, 0.0), Vec3::Y),
        // Donut: a ring of `radius` about Z with a tube of `donut`; a point at a random angle
        // round the tube, its distance from the tube's centre linear in the random (as the real
        // ParagonFXDuring streams fall: quartiles 0.27 / 0.46 / 0.75 of the tube radius).
        // VERIFY: the direction (out from the tube's centre) isn't checked against a capture yet.
        17 => {
            let a = arc(rng);
            let ring = vec3(a.cos(), a.sin(), 0.0);
            let phi = rng.next() * std::f32::consts::TAU;
            let out = ring * phi.cos() + Vec3::Z * phi.sin();
            (ring * sh.radius + out * sh.donut * (1.0 - sh.thickness * rng.next()), out)
        }
        18 => (vec3(rng.next() - 0.5, rng.next() - 0.5, 0.0), Vec3::Z),
        _ => (Vec3::ZERO, Vec3::Z),
    };
    let q = Quat::from_euler(EulerRot::ZXY, sh.rot[2].to_radians(), sh.rot[0].to_radians(), sh.rot[1].to_radians());
    let mut d = q * d;
    let p = q * p + Vec3::from_array(sh.pos);
    if sh.spherize > 0.0 {
        d = d.lerp(p.normalize_or_zero(), sh.spherize).normalize_or_zero();
    }
    if sh.random_dir > 0.0 {
        d = d.lerp(rng.unit(), sh.random_dir).normalize_or_zero();
    }
    (p, d)
}

impl Instance {
    fn new(def: Rc<EffectDef>, at: Vec3, scale: f32, rng: &mut Rng) -> Self {
        let states = def
            .systems
            .iter()
            .map(|s| SysState {
                delay: s.sys.delay.at(0.0, rng.next()),
                emit_acc: 0.0,
                bursts_fired: vec![0; s.sys.emission.as_ref().map_or(0, |e| e.bursts.len())],
                parts: Vec::new(),
            })
            .collect();
        // CustomAnimationComponent.OnEnable -> Reset (resetOnEnable): elapsed 0, playing;
        // CustomRotationAnimator.Awake: randomizeAngle Random.Range(0, 360) (int), randomizeSpeed
        // Random.Range(min, max) for speedMultiplier. (Display only: the particles' own RNG.)
        let anim_state = def
            .anims
            .iter()
            .map(|a| {
                let speed = if a.random_speed { a.speed_range.map_or(a.speed, |[lo, hi]| lo + (hi - lo) * rng.next()) } else { a.speed };
                let off = if a.random_angle { (rng.next() * 360.0).floor().min(359.0) } else { 0.0 };
                (0.0, a.playing, speed, off)
            })
            .collect();
        let clip_t = vec![0.0; def.clips.len()];
        Self { def, at, scale, states, age: 0.0, ambient: false, tag: None, stopped: false, anim_state, disabled: Vec::new(), clip_t, text: None, rot: Quat::IDENTITY, overrides: Vec::new() }
    }

    /// The nodes' local transforms and sprite colours now (prefab space): the export's, with the
    /// animators applied (CustomAnimationComponent.Update @0x521830: the curve at fmodf(elapsed,
    /// 1) while looping or elapsed <= 1, else at 1; then the Animator, which Unity applies after
    /// scripts' Update).
    fn pose(&self) -> (Vec<(Vec3, Quat, Vec3, bool)>, HashMap<usize, [f32; 4]>) {
        let def = &self.def;
        let mut local: Vec<(Vec3, Quat, Vec3, bool)> = def.nodes.iter().enumerate().map(|(i, n)| (n.pos, n.rot, n.scale, n.active && !self.disabled.contains(&i))).collect();
        let mut colors: HashMap<usize, [f32; 4]> = HashMap::new();
        for (a, &(elapsed, _, _, off)) in def.anims.iter().zip(&self.anim_state) {
            if elapsed <= 0.0 {
                // Not updated yet (updateAnimOnReset is off): the serialized values stand.
                continue;
            }
            let t = if a.looping || elapsed <= 1.0 { elapsed % 1.0 } else { 1.0 };
            let v = eval_keys(&a.curve, t);
            match a.kind.as_str() {
                // CustomColorAnimator.UpdateAnimation (@0x521a60): lerp(color1, color2, clamp01).
                "color" => {
                    if let (Some(c1), Some(c2)) = (a.c1, a.c2) {
                        let k = v.clamp(0.0, 1.0);
                        colors.insert(a.target, [0, 1, 2, 3].map(|i| (c2[i] - c1[i]) * k + c1[i]));
                    }
                }
                // CustomScaleAnimator.UpdateAnimation (@0x522a60): RootScale (one) x curve.
                "scale" => local[a.target].2 = Vec3::splat(v),
                // CustomRotationAnimator.UpdateAnimation (@0x522600): AngleAxis(curve x
                // rotationsPerDuration x 360 + angleOffset) about right / up / forward.
                "rotation" => {
                    let axis = match a.axis {
                        1 => Vec3::Y,
                        2 => Vec3::Z,
                        _ => Vec3::X,
                    };
                    let q = Quat::from_axis_angle(axis, (v * a.rotations * 360.0 + off).to_radians());
                    if a.local {
                        local[a.target].1 = q;
                    } else {
                        // World rotation: undo the parent's.
                        let pr = def.nodes[a.target].parent.map_or(Quat::IDENTITY, |p| self.world_rot(&local, p));
                        local[a.target].1 = pr.inverse() * q;
                    }
                }
                _ => {}
            }
        }
        for ((node, c), &t) in def.clips.iter().zip(&self.clip_t) {
            let _ = node;
            let f = t * 60.0;
            for tr in &c.tracks {
                let n = tr.samples.len();
                if n == 0 || tr.node >= local.len() {
                    continue;
                }
                let i0 = (f.floor() as usize).min(n - 1);
                let i1 = (i0 + 1).min(n - 1);
                let u = (f - i0 as f32).clamp(0.0, 1.0);
                let a = &tr.samples[i0];
                let b = &tr.samples[i1];
                let lerp = |k: usize| a.get(k).copied().unwrap_or(0.0) * (1.0 - u) + b.get(k).copied().unwrap_or(0.0) * u;
                match tr.attr {
                    1 => local[tr.node].0 = vec3(lerp(0), lerp(1), lerp(2)),
                    2 => local[tr.node].1 = Quat::from_xyzw(lerp(0), lerp(1), lerp(2), lerp(3)).normalize(),
                    3 => local[tr.node].2 = vec3(lerp(0), lerp(1), lerp(2)),
                    // Unity's Euler order: Z, then X, then Y.
                    4 => {
                        let (x, y, z) = (lerp(0).to_radians(), lerp(1).to_radians(), lerp(2).to_radians());
                        local[tr.node].1 = Quat::from_rotation_y(y) * Quat::from_rotation_x(x) * Quat::from_rotation_z(z);
                    }
                    _ => {}
                }
            }
        }
        (local, colors)
    }

    /// Per system whose node an animator moves: current world matrix x the exported one's
    /// inverse, and the scale factor its particle sizes take (scalingMode Hierarchy: the world
    /// scale; Local: the node's own; Shape: none).
    fn corrections(&self) -> Vec<Option<(Mat4, f32)>> {
        let def = &self.def;
        let overridden = self.overrides.iter().any(Option::is_some);
        if !overridden && !def.sys_animated.iter().any(|&a| a) {
            return Vec::new();
        }
        let (local, _) = self.pose();
        let mut world: Vec<Mat4> = Vec::with_capacity(local.len());
        for (i, &(lp, lr, ls, _)) in local.iter().enumerate() {
            let m = Mat4::from_scale_rotation_translation(ls, lr, lp);
            let w = match (self.overrides.get(i).copied().flatten(), def.nodes[i].parent.and_then(|pi| world.get(pi))) {
                (Some(o), _) => o,
                (None, Some(pm)) => *pm * m,
                (None, None) => m,
            };
            world.push(w);
        }
        def.systems
            .iter()
            .zip(&def.sys_animated)
            .map(|(sd, &a)| {
                (a || overridden).then(|| {
                    let now = world[sd.node];
                    let c = now * def.baked[sd.node].inverse();
                    let k = match sd.sys.scaling {
                        0 => now.to_scale_rotation_translation().0.x.abs() / def.baked[sd.node].to_scale_rotation_translation().0.x.abs().max(1e-6),
                        2 => 1.0,
                        _ => local[sd.node].2.x.abs() / def.nodes[sd.node].scale.x.abs().max(1e-6),
                    };
                    (c, k)
                })
            })
            .collect()
    }

    fn world_rot(&self, local: &[(Vec3, Quat, Vec3, bool)], i: usize) -> Quat {
        let mut q = local[i].1;
        let mut p = self.def.nodes[i].parent;
        while let Some(j) = p {
            q = local[j].1 * q;
            p = self.def.nodes[j].parent;
        }
        q
    }

    /// Advances the animators and Animator clips by `dt` seconds.
    fn update_parts(&mut self, dt: f32) {
        let def = self.def.clone();
        for (i, (a, st)) in def.anims.iter().zip(self.anim_state.iter_mut()).enumerate() {
            let _ = i;
            if !st.1 {
                continue;
            }
            st.0 += dt / a.duration.max(1e-6) * st.2;
            if !a.looping && st.0 > 1.0 {
                // disableOnEnd: the animator's own GameObject goes inactive (its node and children).
                if a.disable_on_end && !self.disabled.contains(&def.anim_owner[i]) {
                    self.disabled.push(def.anim_owner[i]);
                }
                if a.stop_on_end {
                    st.1 = false;
                }
            }
        }
        for ((_, c), t) in def.clips.iter().zip(self.clip_t.iter_mut()) {
            *t += dt * c.speed;
            if c.looping && c.len > 0.0 {
                *t %= c.len;
            } else {
                *t = t.min(c.len);
            }
        }
    }

    /// Advances `dt` seconds; false once every system has finished.
    fn update(&mut self, dt: f32, rng: &mut Rng) -> bool {
        self.age += dt;
        if !self.def.anims.is_empty() || !self.def.clips.is_empty() {
            self.update_parts(dt);
        }
        let mut alive = false;
        let corr = self.corrections();
        let rr = self.rot;
        // Sub-emitter spawns this frame: (system, origin, position, count, inherited properties,
        // parent colour, size, rotation, remaining life).
        let mut sub_spawns: Vec<(usize, Vec3, Vec3, u32, u32, [f32; 4], Vec2, f32, f32)> = Vec::new();
        for (si, (sd, st)) in self.def.systems.iter().zip(&mut self.states).enumerate() {
            let cw = corr.get(si).copied().flatten();
            let sys = &sd.sys;
            let dt = dt * sys.speed;
            let local_t = self.age * sys.speed - st.delay;
            let dur = sys.duration.max(1e-3);
            let emitting = !self.stopped && local_t >= 0.0 && (sys.looping || local_t < dur) && !self.def.is_sub[si];
            // Particles.
            let g = vec3(0.0, -9.81, 0.0);
            let subs = &self.def.subs[si];
            let defs = &self.def.systems;
            st.parts.retain_mut(|p| {
                p.age += dt;
                let u = (p.age / p.life).min(1.0);
                let cur_size = sys.size.as_ref().map_or(p.size, |sm| {
                    let sx = sm.x.at(u, p.r[1]);
                    p.size * if sm.separate { vec2(sx, sm.y.as_ref().map_or(sx, |c| c.at(u, p.r[1]))) } else { Vec2::splat(sx) }
                });
                let cur_color = sys.color.as_ref().map_or(p.color, |cm| {
                    let k = cm.at(u, p.r[4]);
                    [0, 1, 2, 3].map(|i| p.color[i] * k[i])
                });
                let mut birth = 0usize;
                for &(k, kind, props, prob) in subs {
                    let child = &defs[k].sys;
                    let n = match kind {
                        // Birth: the child's own emission over the parent particle's life (rate
                        // over time, bursts at their times).
                        0 if birth < 2 => {
                            let b = birth;
                            birth += 1;
                            let Some(em) = &child.emission else { continue };
                            let cdur = child.duration.max(1e-3);
                            let t = if child.looping { p.age % cdur } else { p.age };
                            let nt = (t / cdur).clamp(0.0, 1.0);
                            let mut n = 0u32;
                            if child.looping || p.age < cdur {
                                p.sub_acc[b] += em.rate.at(nt, p.r[0]) * dt;
                                n = p.sub_acc[b].floor() as u32;
                                p.sub_acc[b] -= n as f32;
                            }
                            // Each burst once, in time order, when the parent reaches its time.
                            while let Some(bt) = em.bursts.get(p.sub_fired[b] as usize).filter(|bt| p.age >= bt.time) {
                                p.sub_fired[b] += 1;
                                n += bt.count.at(nt, p.r[1]).round().max(0.0) as u32;
                            }
                            n
                        }
                        // Death: the child's bursts (rate over time doesn't apply) where it died.
                        2 if p.age >= p.life => child.emission.as_ref().map_or(0, |em| em.bursts.iter().map(|bt| bt.count.at(0.0, p.r[1]).round().max(0.0) as u32).sum()),
                        _ => 0,
                    };
                    if n > 0 && (prob >= 1.0 || p.r[5] <= prob) {
                        sub_spawns.push((k, p.origin, p.pos, n, props, cur_color, cur_size, p.rot, p.life - p.age));
                    }
                }
                if p.age >= p.life {
                    return false;
                }
                let u = p.age / p.life;
                p.vel += g * sys.init.gravity.at(u, p.r[5]) * dt;
                if let Some(f) = &sys.force {
                    let v = vec3(f.x.at(u, p.r[0]), f.y.at(u, p.r[1]), f.z.at(u, p.r[2]));
                    p.vel += if f.world { v } else { rr * sd.rot * v } * dt;
                }
                if let Some(l) = &sys.limit {
                    let max = l.speed.at(u, p.r[3]);
                    let s = p.vel.length();
                    if s > max {
                        p.vel *= (s - (s - max) * l.dampen) / s;
                    }
                }
                let extra = sys.velocity.as_ref().map_or(Vec3::ZERO, |v| {
                    let w = vec3(v.x.at(u, p.r[0]), v.y.at(u, p.r[1]), v.z.at(u, p.r[2]));
                    if v.world { w } else { rr * sd.rot * w }
                });
                let mut disp = (p.vel + extra) * dt;
                if let Some(v) = &sys.velocity {
                    if let Some(sm) = &v.speed_mod {
                        disp *= sm.at(u, p.r[0]);
                    }
                    let has_orbit = v.orbital.as_ref().is_some_and(|o| o.len() == 3);
                    if has_orbit || v.radial.is_some() {
                        // Centre: the system's origin + offset, in its frame (local simulation:
                        // the prefab's; world: where the effect was then, the particle's origin).
                        let off = v.offset.as_ref().filter(|o| o.len() == 3).map_or(Vec3::ZERO, |o| vec3(o[0].at(u, p.r[0]), o[1].at(u, p.r[1]), o[2].at(u, p.r[2])));
                        let centre = rr * (sd.pos + sd.rot * off);
                        let rel = p.pos - centre;
                        if let (true, Some(o)) = (has_orbit, &v.orbital) {
                            let w = vec3(o[0].at(u, p.r[0]), o[1].at(u, p.r[1]), o[2].at(u, p.r[2])) * dt;
                            let local = (rr * sd.rot).inverse() * rel;
                            let turned = Quat::from_rotation_x(w.x) * Quat::from_rotation_y(w.y) * Quat::from_rotation_z(w.z) * local;
                            disp += rr * sd.rot * turned - rel;
                        }
                        if let Some(r) = &v.radial {
                            disp += rel.normalize_or_zero() * r.at(u, p.r[3]) * dt;
                        }
                    }
                }
                p.pos += disp;
                if dt > 0.0 {
                    p.tvel = disp / dt;
                }
                if let Some(rm) = &sys.rotation {
                    p.rot += rm.z.at(u, p.r[4]) * dt;
                    if rm.separate {
                        p.rot3 += vec2(rm.x.at(u, p.r[6]), rm.y.as_ref().map_or(0.0, |c| c.at(u, p.r[7]))) * dt;
                    }
                }
                true
            });
            // Emission, after the live particles are updated and the dead ones gone (Unity's
            // order: a looping max-1 system's replacement isn't refused by the particle it
            // replaces; the Village fire blinked out for a cycle).
            if let (Some(em), true) = (&sys.emission, emitting) {
                let t = if sys.looping { local_t % dur } else { local_t };
                let nt = (t / dur).clamp(0.0, 1.0);
                st.emit_acc += em.rate.at(nt, rng.next()) * dt;
                let mut n = st.emit_acc.floor() as u32;
                st.emit_acc -= n as f32;
                for (bi, b) in em.bursts.iter().enumerate() {
                    let cycles = if b.cycles == 0 { u32::MAX } else { b.cycles };
                    while st.bursts_fired[bi] < cycles && local_t >= b.time + st.bursts_fired[bi] as f32 * b.interval.max(1e-3) {
                        st.bursts_fired[bi] += 1;
                        if rng.next() <= b.probability {
                            n += b.count.at(nt, rng.next()).round().max(0.0) as u32;
                        }
                    }
                }
                for _ in 0..n {
                    if st.parts.len() as u32 >= sys.init.max {
                        break;
                    }
                    let r = [rng.next(), rng.next(), rng.next(), rng.next(), rng.next(), rng.next(), rng.next(), rng.next()];
                    let (p, d) = emit_shape(sys.shape.as_ref(), rng);
                    let speed = sys.init.speed.at(nt, r[0]);
                    let size = sys.init.size.at(nt, r[1]);
                    let size_y = if sys.init.size3d { sys.init.size_y.at(nt, r[1]) } else { size };
                    // World-space particles start from the node's transform now (an animator
                    // may have moved it); local-space ones follow it when drawn.
                    let (mut pos, mut vel, mut k) = (sd.pos + sd.rot * (p * sd.scale), sd.rot * d * speed, 1.0);
                    if let (1, Some((c, f))) = (sys.space, cw) {
                        pos = c.transform_point3(pos);
                        vel = c.transform_vector3(vel).normalize_or_zero() * vel.length();
                        k = f;
                    }
                    // Particles live in the display's frame (its rotation applied); gravity and
                    // world-space modules stay in Unity's world axes.
                    let (pos, vel) = (rr * pos, rr * vel);
                    st.parts.push(Particle {
                        origin: if sys.space == 1 { self.at } else { Vec3::ZERO },
                        pos,
                        vel,
                        age: 0.0,
                        life: sys.init.lifetime.at(nt, r[2]).max(1e-3),
                        size: vec2(size, size_y) * sd.size_scale * k,
                        rot: sys.init.rotation.at(nt, r[3]),
                        color: sys.init.color.at(nt, r[4]),
                        r,
                        sub_acc: [0.0; 2],
                        sub_fired: [0; 2],
                        tvel: Vec3::ZERO,
                        rot3: if sys.init.rotation3d { vec2(sys.init.rotation_x.at(nt, r[6]), sys.init.rotation_y.at(nt, r[7])) } else { Vec2::ZERO },
                    });
                }
            }
            if !st.parts.is_empty() || (local_t < dur || sys.looping) {
                alive = true;
            }
        }
        // Sub-emitter particles: from the parent particle's position, through the child's shape
        // and orientation, inheriting colour (1), size (2), rotation (4) and lifetime (8: the
        // parent's remaining life) as its properties say.
        for (k, origin, at, n, props, col, size, rot, rem) in sub_spawns {
            let def = self.def.clone();
            let sd = &def.systems[k];
            let sys = &sd.sys;
            let st = &mut self.states[k];
            for _ in 0..n {
                if st.parts.len() as u32 >= sys.init.max {
                    break;
                }
                let r = [rng.next(), rng.next(), rng.next(), rng.next(), rng.next(), rng.next(), rng.next(), rng.next()];
                let (p, d) = emit_shape(sys.shape.as_ref(), rng);
                let speed = sys.init.speed.at(0.0, r[0]);
                let sz = sys.init.size.at(0.0, r[1]);
                let sz_y = if sys.init.size3d { sys.init.size_y.at(0.0, r[1]) } else { sz };
                let mut c = sys.init.color.at(0.0, r[4]);
                if props & 1 != 0 {
                    c = [0, 1, 2, 3].map(|i| c[i] * col[i]);
                }
                let mut s2 = vec2(sz, sz_y) * sd.size_scale;
                if props & 2 != 0 {
                    s2 *= size;
                }
                let mut life = sys.init.lifetime.at(0.0, r[2]).max(1e-3);
                if props & 8 != 0 {
                    life *= rem.max(1e-3);
                }
                st.parts.push(Particle {
                    origin,
                    pos: at + rr * sd.rot * (p * sd.scale),
                    vel: rr * sd.rot * d * speed,
                    age: 0.0,
                    life,
                    size: s2,
                    rot: sys.init.rotation.at(0.0, r[3]) + if props & 4 != 0 { rot } else { 0.0 },
                    color: c,
                    r,
                    sub_acc: [0.0; 2],
                    sub_fired: [0; 2],
                    tvel: Vec3::ZERO,
                    rot3: if sys.init.rotation3d { vec2(sys.init.rotation_x.at(0.0, r[6]), sys.init.rotation_y.at(0.0, r[7])) } else { Vec2::ZERO },
                });
                alive = true;
            }
        }
        // Looping systems on one-shot effects stop after a while.
        if self.stopped {
            return self.states.iter().any(|s| !s.parts.is_empty());
        }
        self.ambient || self.tag.is_some() || (alive && self.age < 12.0)
    }
}

// ---------------------------------------------------------------- drawing

const PART_VS: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}"#;

const PART_FS: &str = r#"#version 100
varying lowp vec4 color;
varying lowp vec2 uv;
uniform sampler2D Texture;
void main() {
    gl_FragColor = texture2D(Texture, uv) * color;
}"#;

/// URP Particles/Unlit additive colour mode: albedo (texture x tint) plus the particle colour.
const PART_ADD_FS: &str = r#"#version 100
varying lowp vec4 color;
varying lowp vec2 uv;
uniform sampler2D Texture;
uniform lowp vec4 Tint;
void main() {
    lowp vec4 t = texture2D(Texture, uv) * Tint;
    gl_FragColor = vec4(min(t.rgb + color.rgb, vec3(1.0)), t.a * color.a);
}"#;

fn material(blend: Blend, add_color: bool) -> Material {
    let (src, dst) = match blend {
        Blend::Alpha => (BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::OneMinusValue(BlendValue::SourceAlpha)),
        Blend::Add => (BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::One),
        Blend::Premul => (BlendFactor::One, BlendFactor::OneMinusValue(BlendValue::SourceAlpha)),
    };
    load_material(
        ShaderSource::Glsl { vertex: PART_VS, fragment: if add_color { PART_ADD_FS } else { PART_FS } },
        MaterialParams {
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: false,
                color_blend: Some(BlendState::new(Equation::Add, src, dst)),
                ..Default::default()
            },
            uniforms: vec![UniformDesc::new("Tint", UniformType::Float4)],
            ..Default::default()
        },
    )
    .expect("particle shader")
}

/// Live effects and their loaded prefabs.
pub struct Particles {
    root: PathBuf,
    defs: HashMap<String, Option<Rc<EffectDef>>>,
    live: Vec<Instance>,
    mats: HashMap<(Blend, bool), Material>,
    rng: Rng,
    verts: Vec<Vertex>,
    idx: Vec<u16>,
    /// Mesh parts (a LitMonkey silhouette): textured, alpha blended, depth tested and written.
    mesh_mat: Option<Material>,
    /// RRBTD6_FXDUMP=<file>: every drawn frame's particles in SimRecorder's .fx format (U, S, q
    /// quads in Unity axes; m bounds for mesh-mode systems), to compare with a real-game capture.
    dump: Option<std::io::BufWriter<std::fs::File>>,
    /// The effects' own clock (the dt they've been updated by), the dump's frame time.
    clock: f64,
}

impl Particles {
    pub fn new(root: &Path) -> Self {
        Self { root: root.join("effects"), defs: HashMap::new(), live: Vec::new(), mats: HashMap::new(), rng: Rng(0x9E37_79B9), verts: Vec::new(), idx: Vec::new(), mesh_mat: None,
            dump: std::env::var("RRBTD6_FXDUMP").ok().and_then(|p| std::fs::File::create(p).ok()).map(std::io::BufWriter::new), clock: 0.0 }
    }

    pub fn is_prepared(&self,name:&str)->bool{self.defs.contains_key(name)}
    /// Definitions/materials only: never construct an instance or touch particle RNG.
    pub fn retain_prepared(&mut self,name:String,def:Option<EffectDef>){
        if let Some(def)=&def{for system in &def.systems{
            self.mats.entry((system.blend,system.add_color)).or_insert_with(||material(system.blend,system.add_color));
        }}
        self.defs.entry(name).or_insert_with(||def.map(Rc::new));
    }

    /// Plays effects/<name> at Unity position `at`, scaled by `scale`. False if it isn't exported.
    pub fn spawn(&mut self, name: &str, at: [f32; 3], scale: f32) -> bool {
        let root = &self.root;
        let def = self.defs.entry(name.to_string()).or_insert_with(|| EffectDef::load(&root.join(name)).map(Rc::new)).clone();
        let Some(def) = def else { return false };
        let inst = Instance::new(def, Vec3::from_array(at), scale, &mut self.rng);
        self.live.push(inst);
        true
    }

    /// The map's ambient effects (effects/map_<key>.json from export_particles.py scene:<key>),
    /// replacing the previous map's. Prewarmed systems start a full cycle in, as Unity does.
    pub fn spawn_ambient(&mut self, map_key: &str) {
        self.live.retain(|i| !i.ambient);
        let Ok(s) = std::fs::read_to_string(self.root.join(format!("map_{map_key}.json"))) else { return };
        let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&s) else { return };
        for e in list {
            let (Some(name), Some(p)) = (e["effect"].as_str(), e["pos"].as_array()) else { continue };
            let at = [0, 1, 2].map(|i| p.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32);
            if !self.spawn(name, at, 1.0) {
                continue;
            }
            let rng = &mut self.rng;
            if let Some(inst) = self.live.last_mut() {
                inst.ambient = true;
                let warm = inst.def.systems.iter().filter(|s| s.sys.prewarm).map(|s| s.sys.duration).fold(0.0f32, f32::max);
                let mut t = 0.0;
                while t < warm.min(20.0) {
                    inst.update(1.0 / 30.0, rng);
                    t += 1.0 / 30.0;
                }
            }
        }
    }

    /// Plays `name` until `kill(tag)`, e.g. the particle systems inside a tower's own prefab
    /// (Ring of Fire's flames). False if it isn't exported.
    pub fn spawn_tagged(&mut self, name: &str, at: [f32; 3], tag: u64) -> bool {
        if !self.spawn(name, at, 1.0) {
            return false;
        }
        if let Some(i) = self.live.last_mut() {
            i.tag = Some(tag);
        }
        true
    }

    /// Moves the effect tagged `tag` (a projectile's) to `at`.
    pub fn set_pos(&mut self, tag: u64, at: [f32; 3]) {
        for i in self.live.iter_mut().filter(|i| i.tag == Some(tag)) {
            i.at = Vec3::from_array(at);
        }
    }

    /// Stops the effect tagged `tag` emitting; what it already emitted plays out (trails fade).
    pub fn release(&mut self, tag: u64) {
        for i in self.live.iter_mut().filter(|i| i.tag == Some(tag)) {
            i.tag = None;
            i.stopped = true;
        }
    }

    /// Rescales the effect tagged `tag` (a Paragon's investment orb shrinking into it).
    pub fn set_scale(&mut self, tag: u64, scale: f32) {
        for i in self.live.iter_mut().filter(|i| i.tag == Some(tag)) {
            i.scale = scale;
        }
    }

    /// Plays `name` as the game shows an effect's display (UnityDisplayNode): the display sets its
    /// root's rotation, replacing the prefab root's own. A sprite display (its root has an
    /// enabled SpriteRenderer, or `sprite` forces it: Scene.PresentText) faces the camera,
    /// RotationCache's Euler(60, 0, -angle); a 3D one turns AngleAxis(angle, up).
    /// (Recording dart_paragon_fx: Paragon effects spawned at rotation 0 show at angle 180, the
    /// degree text at 0.)
    pub fn spawn_display(&mut self, name: &str, at: [f32; 3], scale: f32, angle: f32, sprite: bool, tag: Option<u64>) -> bool {
        if !self.spawn(name, at, scale) {
            return false;
        }
        if let Some(i) = self.live.last_mut() {
            let a = angle.to_radians();
            let r = if sprite || i.def.sprite_display { Quat::from_rotation_x(60f32.to_radians()) * Quat::from_rotation_z(-a) } else { Quat::from_rotation_y(a) };
            let root = i.def.nodes.iter().position(|n| n.parent.is_none()).map_or(Quat::IDENTITY, |k| i.def.nodes[k].rot);
            i.rot = r * root.inverse();
            i.tag = tag;
        }
        true
    }

    /// Puts the effect tagged `tag` on a tower model: its yaw (Unity space, about +Y, as the
    /// model is drawn) and the world matrices (prefab-root space) of the nodes the model's
    /// animation moves, by path (anim::AnimModel::attach_mats).
    pub fn set_attach(&mut self, tag: u64, at: [f32; 3], yaw: f32, mats: &[(&str, Mat4)]) {
        for i in self.live.iter_mut().filter(|i| i.tag == Some(tag)) {
            i.at = Vec3::from_array(at);
            i.rot = Quat::from_rotation_y(yaw);
            let nodes = &i.def.nodes;
            i.overrides = nodes.iter().map(|n| mats.iter().find(|(p, _)| *p == n.path).map(|(_, m)| *m)).collect();
        }
    }

    /// Sets the text the effect tagged `tag` shows in its text nodes (Simulation.CreateTextEffect).
    pub fn set_text(&mut self, tag: u64, text: &str) {
        for i in self.live.iter_mut().filter(|i| i.tag == Some(tag)) {
            i.text = Some(text.to_string());
        }
    }

    /// Every live effect's text nodes where they are now (animators and clips applied); the
    /// caller draws them (the title font isn't the particle renderer's).
    pub fn texts(&self) -> Vec<WorldText> {
        let mut out = Vec::new();
        for inst in self.live.iter().filter(|i| !i.def.texts.is_empty()) {
            let def = &inst.def;
            let (local, _) = inst.pose();
            let mut world: Vec<(Vec3, Quat, Vec3, bool)> = Vec::with_capacity(local.len());
            for (i, &(lp, lr, ls, act)) in local.iter().enumerate() {
                let w = match def.nodes[i].parent.and_then(|pi| world.get(pi)) {
                    Some(&(pp, pr, ps, pa)) => (pp + pr * (lp * ps), pr * lr, ps * ls, pa && act),
                    None => (inst.rot * lp, inst.rot * lr, ls, act),
                };
                world.push(w);
            }
            for (node, style) in &def.texts {
                let Some(&(wp, wr, ws, active)) = world.get(*node) else { continue };
                if !active {
                    continue;
                }
                out.push(WorldText {
                    at: inst.at + wp * inst.scale,
                    rot: wr,
                    scale: vec2(ws.x, ws.y) * inst.scale,
                    text: inst.text.clone().unwrap_or_else(|| style.text.clone()),
                    style: style.clone(),
                });
            }
        }
        out
    }

    pub fn kill(&mut self, tag: u64) {
        self.live.retain(|i| i.tag != Some(tag));
    }

    pub fn update(&mut self, dt: f32) {
        self.clock += dt as f64;
        let rng = &mut self.rng;
        self.live.retain_mut(|i| i.update(dt, rng));
    }

    /// Draws every live particle; `right` / `up`: the camera's GL-space axes (billboards),
    /// `px_per_unit`: screen pixels per world unit (maxParticleSize), `view_h`: viewport px.
    pub fn draw(&mut self, right: Vec3, up: Vec3, px_per_unit: f32, view_h: f32) {
        if let Some(d) = &mut self.dump {
            use std::io::Write;
            let _ = writeln!(d, "U 0 0 {} 0 0", self.clock);
        }
        if self.live.is_empty() {
            return;
        }
        for inst in &self.live {
            // Renderers in sorting order (Renderer.sortingOrder), hierarchy order within one:
            // 0 particle system, 1 sprite, 2 mesh.
            let def = &inst.def;
            let mut items: Vec<(i32, usize, u8, usize)> = def.systems.iter().enumerate().map(|(k, sd)| (sd.ren.order, sd.node, 0u8, k)).collect();
            items.extend(def.sprites.iter().enumerate().map(|(k, sp)| (sp.order, sp.node, 1u8, k)));
            items.extend(def.meshes.iter().enumerate().map(|(k, m)| (m.order, m.node, 2u8, k)));
            items.sort_by_key(|x| (x.0, x.1));
            let corr = inst.corrections();
            let posed = (!def.sprites.is_empty() || !def.meshes.is_empty()).then(|| {
                let (local, colors) = inst.pose();
                // World (prefab-root) transforms, parents first.
                let mut world: Vec<(Vec3, Quat, Vec3, bool)> = Vec::with_capacity(local.len());
                for (i, &(lp, lr, ls, act)) in local.iter().enumerate() {
                    let w = match def.nodes[i].parent.and_then(|pi| world.get(pi)) {
                        Some(&(pp, pr, ps, pa)) => (pp + pr * (lp * ps), pr * lr, ps * ls, pa && act),
                        None => (inst.rot * lp, inst.rot * lr, ls, act),
                    };
                    world.push(w);
                }
                (world, colors)
            });
            for &(_, _, kind, k) in &items {
                if kind != 0 {
                    let Some((world, colors)) = &posed else { continue };
                    let place = |node: usize, q: Vec3| -> Vec3 {
                        let (wp, wr, ws, _) = world[node];
                        let u = inst.at + (wp + wr * (ws * q)) * inst.scale;
                        gl(u.x, u.y, u.z)
                    };
                    if kind == 1 {
                        // SpriteRenderer (Sprites-Default): a quad in its node's XY plane, pivot at
                        // the origin, texture x colour.
                        let sp = &def.sprites[k];
                        if !world[sp.node].3 {
                            continue;
                        }
                        let c = colors.get(&sp.node).copied().unwrap_or(sp.color);
                        if c[3] <= 0.0 {
                            continue;
                        }
                        let col = c.map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8);
                        let (x0, y0) = (-sp.pivot.x * sp.size.x, -sp.pivot.y * sp.size.y);
                        let (x1, y1) = (x0 + sp.size.x, y0 + sp.size.y);
                        let (u0, u1) = if sp.flip[0] { (1.0, 0.0) } else { (0.0, 1.0) };
                        let (vb, vt) = if sp.flip[1] { (0.0, 1.0) } else { (1.0, 0.0) };
                        let v = |q: Vec3, a: f32, b: f32| Vertex { position: q, uv: vec2(a, b), color: col, normal: Vec4::ZERO };
                        self.verts.clear();
                        self.idx.clear();
                        self.verts.extend_from_slice(&[
                            v(place(sp.node, vec3(x0, y0, 0.0)), u0, vb),
                            v(place(sp.node, vec3(x1, y0, 0.0)), u1, vb),
                            v(place(sp.node, vec3(x1, y1, 0.0)), u1, vt),
                            v(place(sp.node, vec3(x0, y1, 0.0)), u0, vt),
                        ]);
                        self.idx.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
                        let m = self.mats.entry((Blend::Alpha, false)).or_insert_with(|| material(Blend::Alpha, false));
                        gl_use_material(m);
                        let mesh = Mesh { vertices: std::mem::take(&mut self.verts), indices: std::mem::take(&mut self.idx), texture: Some(sp.tex.clone()) };
                        draw_mesh(&mesh);
                        gl_use_default_material();
                        self.verts = mesh.vertices;
                        self.idx = mesh.indices;
                    } else {
                        // MeshRenderer (LitMonkey: unlit, colour = texture x tint), depth tested.
                        let mp = &def.meshes[k];
                        if !world[mp.node].3 {
                            continue;
                        }
                        let col = mp.tint.map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8);
                        let pts: Vec<Vec3> = mp.verts.iter().map(|(q, _)| place(mp.node, *q)).collect();
                        let mat = self.mesh_mat.get_or_insert_with(|| {
                            load_material(
                                ShaderSource::Glsl { vertex: PART_VS, fragment: PART_FS },
                                MaterialParams {
                                    pipeline_params: PipelineParams {
                                        depth_test: Comparison::LessOrEqual,
                                        depth_write: true,
                                        color_blend: Some(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::OneMinusValue(BlendValue::SourceAlpha))),
                                        ..Default::default()
                                    },
                                    uniforms: vec![UniformDesc::new("Tint", UniformType::Float4)],
                                    ..Default::default()
                                },
                            )
                            .expect("mesh part shader")
                        });
                        gl_use_material(mat);
                        // In chunks under the batcher's cap (it drops what is past it).
                        for tris in mp.idx.chunks(4800 - 4800 % 3) {
                            let mut remap: HashMap<u16, u16> = HashMap::new();
                            let mut vs: Vec<Vertex> = Vec::new();
                            let mut is: Vec<u16> = Vec::with_capacity(tris.len());
                            for &i in tris {
                                let j = *remap.entry(i).or_insert_with(|| {
                                    let (_, uv) = mp.verts[i as usize];
                                    vs.push(Vertex { position: pts[i as usize], uv, color: col, normal: Vec4::ZERO });
                                    (vs.len() - 1) as u16
                                });
                                is.push(j);
                            }
                            draw_mesh(&Mesh { vertices: vs, indices: is, texture: Some(mp.tex.clone()) });
                        }
                        gl_use_default_material();
                    }
                    continue;
                }
                let (sd, st) = (&def.systems[k], &inst.states[k]);
                if st.parts.is_empty() {
                    continue;
                }
                let unity = |g: Vec3| vec3(g.x, g.y, -g.z);
                let mut mesh_bounds: Option<(Vec3, Vec3, usize)> = None;
                // Small mesh-mode systems also each Quad corner ("mv x y z u v"), as SimRecorder.
                let mut mesh_verts: Vec<(Vec3, f32, f32)> = Vec::new();
                if let Some(d) = &mut self.dump {
                    use std::io::Write;
                    let q = inst.rot * sd.rot;
                    let _ = writeln!(d, "S {} 0 0 0 {} {} {} {} 1 1 1 {} True 0", def.nodes[sd.node].path.replace(' ', "_"), q.x, q.y, q.z, q.w, st.parts.len());
                }
                let cl = if sd.sys.space == 1 { None } else { corr.get(k).copied().flatten() }
                    .map(|(c, f)| (Mat4::from_quat(inst.rot) * c * Mat4::from_quat(inst.rot.inverse()), f));
                self.verts.clear();
                self.idx.clear();
                let sys = &sd.sys;
                for p in &st.parts {
                    let u = p.age / p.life;
                    let mut size = p.size * inst.scale;
                    if let Some(sm) = &sys.size {
                        let sx = sm.x.at(u, p.r[1]);
                        let sy = if sm.separate { sm.y.as_ref().map_or(sx, |c| c.at(u, p.r[1])) } else { sx };
                        size *= vec2(sx, sy);
                    }
                    // maxParticleSize: billboards no larger than this fraction of the view height.
                    if matches!(sd.ren.mode, 0 | 2 | 3) {
                        let max = sd.ren.max_size * view_h / px_per_unit.max(1e-3);
                        size = size.min(Vec2::splat(max));
                    }
                    let mut c = p.color;
                    if let Some(cm) = &sys.color {
                        let k = cm.at(u, p.r[4]);
                        c = [0, 1, 2, 3].map(|i| c[i] * k[i]);
                    }
                    // Additive colour mode applies the tint in the shader instead.
                    let k = if sd.add_color { [1.0; 4] } else { sd.tint };
                    let c = [0, 1, 2, 3].map(|i| ((c[i] * k[i]).clamp(0.0, 1.0) * 255.0) as u8);
                    if c[3] == 0 {
                        continue;
                    }
                    // Texture sheet frame.
                    let (mut u0, mut v0, mut u1, mut v1) = (0.0, 0.0, 1.0, 1.0);
                    if let Some(sh) = &sys.sheet {
                        let n = (sh.x * sh.y).max(1);
                        let f = ((sh.start.at(0.0, p.r[5]) + sh.frame.at((u * sh.cycles) % 1.0, p.r[5]) * n as f32) as u32).min(n - 1);
                        let (fx, fy) = (f % sh.x.max(1), f / sh.x.max(1));
                        let (w, h) = (1.0 / sh.x.max(1) as f32, 1.0 / sh.y.max(1) as f32);
                        (u0, v0, u1, v1) = (fx as f32 * w, fy as f32 * h, (fx + 1) as f32 * w, (fy + 1) as f32 * h);
                    }
                    if let Some((_, f)) = cl {
                        size *= f;
                    }
                    let local_pos = cl.map_or(p.pos, |(c, _)| c.transform_point3(p.pos));
                    let centre = if sd.sys.space == 1 { p.origin } else { inst.at } + local_pos * inst.scale;
                    let cg = gl(centre.x, centre.y, centre.z);
                    let (s, co) = (-p.rot).sin_cos();
                    // Quad axes (GL space) by render mode.
                    let (ax, ay) = match (sd.ren.mode, sd.ren.align) {
                        // Horizontal billboard: flat on the ground.
                        (2, _) => (vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, -1.0)),
                        // Stretched along the velocity.
                        (1, _) => {
                            let v = if p.tvel == Vec3::ZERO { p.vel } else { p.tvel };
                            let dir = gl(v.x, v.y, v.z).normalize_or_zero();
                            let side = dir.cross(right.cross(up)).normalize_or_zero();
                            let len = sd.ren.length + v.length() * sd.ren.vel_scale / size.y.max(1e-3);
                            (side, dir * len)
                        }
                        // Quad meshes aligned Local / World: the system's XY plane.
                        (4, 1) => (vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0)),
                        (4, 2) => {
                            let x = inst.rot * sd.rot * Vec3::X;
                            let y = inst.rot * sd.rot * Vec3::Y;
                            (gl(x.x, x.y, x.z), gl(y.x, y.y, y.z))
                        }
                        _ => (right, up),
                    };
                    // Mesh particles aligned World / Local: the particle's rotation in the system's
                    // (or world) axes, Unity's Euler order Ry * Rx * Rz with its angles as they are
                    // (the real AirWavesInner / Rays / Circles corners: X column (cos b cos g, sin g,
                    // -sin b cos g), g rising at the rotation over lifetime's rate).
                    let az = match sd.ren.align {
                        2 => {
                            let z = inst.rot * sd.rot * Vec3::Z;
                            gl(z.x, z.y, z.z)
                        }
                        // World: Unity's +Z (gl flips z).
                        1 => gl(0.0, 0.0, 1.0),
                        _ => ax.cross(ay).normalize_or_zero(),
                    };
                    let turn3 = (sd.ren.mode == 4 && matches!(sd.ren.align, 1 | 2))
                        .then(|| Quat::from_rotation_y(p.rot3.y) * Quat::from_rotation_x(p.rot3.x) * Quat::from_rotation_z(p.rot));
                    let place = |l: Vec3| -> Vec3 {
                        match turn3 {
                            Some(q) => {
                                let l = q * l;
                                ax * l.x + ay * l.y + az * l.z
                            }
                            None => ax * (l.x * co - l.y * s) + ay * (l.x * s + l.y * co) + az * l.z,
                        }
                    };
                    let rx = place(vec3(size.x * 0.5, 0.0, 0.0));
                    let ry = place(vec3(0.0, size.y * 0.5, 0.0));
                    // A stretched particle leads its quad: the quad runs back from the particle
                    // along its velocity (the real ParagonFXDuring streams: every newborn's
                    // leading edge, not its centre, lies in the emitting donut).
                    let cg = if sd.ren.mode == 1 { cg - ay * (size.y * 0.5) } else { cg };
                    let base = self.verts.len() as u16;
                    // Custom vertex streams after UV (TEXCOORD0.zw), carried in the normal's xy.
                    let extra = |k: usize| -> f32 {
                        let Some(tok) = sd.ren.uv_extra.get(k) else { return 0.0 };
                        if tok == "age" {
                            return u;
                        }
                        let b = tok.as_bytes();
                        let (set, comp) = (b.get(1).map_or(0, |c| (c - b'1') as usize), b.get(2).map_or(0, |c| match c { b'x' => 0, b'y' => 1, b'z' => 2, _ => 3 }));
                        sys.custom.get(set).filter(|c| c.0 == 1).and_then(|c| c.1.get(comp)).map_or(0.0, |cv| cv.at(u, p.r[3]))
                    };
                    let nrm = if sd.ren.uv_extra.is_empty() { Vec4::ZERO } else { vec4(extra(0), extra(1), 0.0, 0.0) };
                    // A mesh particle (mode 4 with its own mesh): each vertex scaled by the
                    // particle's size (x, y; z as x), turned by its rotation in the quad's plane
                    // and placed on the quad's axes (the alignment's), as Unity draws a mesh where
                    // the Quad would be.
                    if let (4, Some(pm)) = (sd.ren.mode, &sd.pmesh) {
                        let base = self.verts.len() as u16;
                        for (q, uvq) in pm.0.iter() {
                            let (x, y, z) = (q.x * size.x, q.y * size.y, q.z * size.x);
                            let w = place(vec3(x, y, z));
                            if self.dump.is_some() {
                                let u = unity(cg + w);
                                let b = mesh_bounds.get_or_insert((u, u, 0));
                                *b = (b.0.min(u), b.1.max(u), b.2 + 1);
                            }
                            self.verts.push(Vertex { position: cg + w, uv: vec2(u0 + (u1 - u0) * uvq.x, v1 - (v1 - v0) * uvq.y), color: c, normal: nrm });
                        }
                        self.idx.extend(pm.1.iter().map(|&i| base + i));
                        if self.idx.len() >= 4800 - pm.1.len() {
                            Self::flush(&mut self.mats, sd, &mut self.verts, &mut self.idx);
                        }
                        continue;
                    }
                    if self.dump.is_some() {
                        let cs = [cg - rx - ry, cg + rx - ry, cg + rx + ry, cg - rx + ry].map(unity);
                        if sd.ren.mode == 4 {
                            for (u, (a, b)) in cs.into_iter().zip([(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]) {
                                mesh_verts.push((u, a, b));
                                let b = mesh_bounds.get_or_insert((u, u, 0));
                                *b = (b.0.min(u), b.1.max(u), b.2 + 1);
                            }
                        } else if let Some(d) = &mut self.dump {
                            use std::io::Write;
                            let f = |v: Vec3| format!("{} {} {}", v.x, v.y, v.z);
                            let _ = writeln!(d, "q {} {} {} {} {} {} {} {} {} {} {} {}", f(cs[0]), f(cs[1]), f(cs[2]), f(cs[3]), u0, v0, u1, v1, c[0], c[1], c[2], c[3]);
                        }
                    }
                    let v = |q: Vec3, a: f32, b: f32| Vertex { position: q, uv: vec2(a, b), color: c, normal: nrm };
                    self.verts.extend_from_slice(&[v(cg - rx - ry, u0, v1), v(cg + rx - ry, u1, v1), v(cg + rx + ry, u1, v0), v(cg - rx + ry, u0, v0)]);
                    self.idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                    if self.idx.len() >= 4800 {
                        Self::flush(&mut self.mats, sd, &mut self.verts, &mut self.idx);
                    }
                }
                Self::flush(&mut self.mats, sd, &mut self.verts, &mut self.idx);
                if let (Some(d), Some((lo, hi, n))) = (&mut self.dump, mesh_bounds) {
                    use std::io::Write;
                    let _ = writeln!(d, "m {} {} {} {} {} {} {} {}", n, n / 2, lo.x, lo.y, lo.z, hi.x, hi.y, hi.z);
                    if n <= 64 {
                        for (v, a, b) in &mesh_verts {
                            let _ = writeln!(d, "mv {} {} {} {} {}", v.x, v.y, v.z, a, b);
                        }
                    }
                }
            }
        }
    }

    fn flush(mats: &mut HashMap<(Blend, bool), Material>, sd: &SysDef, verts: &mut Vec<Vertex>, idx: &mut Vec<u16>) {
        if idx.is_empty() {
            return;
        }
        if let Some(fx) = &sd.fx {
            let vp = unsafe { get_internal_gl() }.quad_gl.get_projection_matrix() * Mat4::from_scale(vec3(1.0, 1.0, -1.0));
            let mesh = Mesh { vertices: std::mem::take(verts), indices: std::mem::take(idx), texture: None };
            fx.draw(&mesh, vp, get_time() as f32);
            *verts = mesh.vertices;
            *idx = mesh.indices;
            verts.clear();
            idx.clear();
            return;
        }
        let m = mats.entry((sd.blend, sd.add_color)).or_insert_with(|| material(sd.blend, sd.add_color));
        if sd.add_color {
            m.set_uniform("Tint", Vec4::from_array(sd.tint));
        }
        gl_use_material(m);
        let mesh = Mesh { vertices: std::mem::take(verts), indices: std::mem::take(idx), texture: Some(sd.tex.clone()) };
        draw_mesh(&mesh);
        gl_use_default_material();
        *verts = mesh.vertices;
        *idx = mesh.indices;
        verts.clear();
        idx.clear();
    }
}

