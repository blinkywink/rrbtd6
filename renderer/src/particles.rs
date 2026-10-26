//! Unity particle-system (Shuriken) player for the game's effect prefabs, exported by
//! tools/assets/export_particles.py to effects/<Prefab>/effect.json.
//!
//! Simulated: the main module (duration, looping, start delay, lifetime / speed / size /
//! rotation / colour, gravity, max particles), emission (rate over time, bursts with cycles),
//! shape (sphere, hemisphere, cone, box, circle, edge, rectangle), size / colour / rotation /
//! velocity / force / limit-velocity over lifetime and texture-sheet animation; billboard,
//! horizontal, stretched and quad-mesh renderers with the material's blend and tint.
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

/// MinMaxGradient: 0 colour, 1 gradient, 2 random between gradients, 3 random between colours,
/// 4 random colour from the gradient.
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
            2 => mix4(self.g0.at(t), self.g.at(t), r),
            3 => mix4(self.c0, self.c, r),
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
    #[serde(default = "yes")]
    active: bool,
    pos: [f32; 3],
    rot: [f32; 4],
    scale: [f32; 3],
    parent: Option<usize>,
    system: Option<System>,
    renderer: Option<Renderer>,
}

#[derive(Deserialize)]
struct EffectJson {
    nodes: Vec<NodeJson>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Blend {
    Alpha,
    Add,
    Premul,
}

struct SysDef {
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
}

pub struct EffectDef {
    systems: Vec<SysDef>,
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
        for n in &j.nodes {
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
            systems.push(SysDef { sys: sys.clone(), ren: ren.clone(), rot: r, pos: p, scale, size_scale, tex, blend, tint, add_color });
        }
        Some(EffectDef { systems })
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
    age: f32,
    life: f32,
    size: Vec2,
    rot: f32,
    color: [f32; 4],
    /// Per-particle randoms for curves / gradients (Unity draws one per property).
    r: [f32; 6],
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
        Self { def, at, scale, states, age: 0.0, ambient: false, tag: None, stopped: false }
    }

    /// Advances `dt` seconds; false once every system has finished.
    fn update(&mut self, dt: f32, rng: &mut Rng) -> bool {
        self.age += dt;
        let mut alive = false;
        for (sd, st) in self.def.systems.iter().zip(&mut self.states) {
            let sys = &sd.sys;
            let dt = dt * sys.speed;
            let local_t = self.age * sys.speed - st.delay;
            let dur = sys.duration.max(1e-3);
            let emitting = !self.stopped && local_t >= 0.0 && (sys.looping || local_t < dur);
            // Emission.
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
                    let r = [rng.next(), rng.next(), rng.next(), rng.next(), rng.next(), rng.next()];
                    let (p, d) = emit_shape(sys.shape.as_ref(), rng);
                    let speed = sys.init.speed.at(nt, r[0]);
                    let size = sys.init.size.at(nt, r[1]);
                    let size_y = if sys.init.size3d { sys.init.size_y.at(nt, r[1]) } else { size };
                    st.parts.push(Particle {
                        origin: if sys.space == 1 { self.at } else { Vec3::ZERO },
                        pos: sd.pos + sd.rot * (p * sd.scale),
                        vel: sd.rot * d * speed,
                        age: 0.0,
                        life: sys.init.lifetime.at(nt, r[2]).max(1e-3),
                        size: vec2(size, size_y) * sd.size_scale,
                        rot: sys.init.rotation.at(nt, r[3]),
                        color: sys.init.color.at(nt, r[4]),
                        r,
                    });
                }
            }
            // Particles.
            let g = vec3(0.0, -9.81, 0.0);
            st.parts.retain_mut(|p| {
                p.age += dt;
                if p.age >= p.life {
                    return false;
                }
                let u = p.age / p.life;
                p.vel += g * sys.init.gravity.at(u, p.r[5]) * dt;
                if let Some(f) = &sys.force {
                    let v = vec3(f.x.at(u, p.r[0]), f.y.at(u, p.r[1]), f.z.at(u, p.r[2]));
                    p.vel += if f.world { v } else { sd.rot * v } * dt;
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
                    if v.world { w } else { sd.rot * w }
                });
                p.pos += (p.vel + extra) * dt;
                if let Some(rm) = &sys.rotation {
                    p.rot += rm.z.at(u, p.r[4]) * dt;
                }
                true
            });
            if !st.parts.is_empty() || (local_t < dur || sys.looping) {
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
}

impl Particles {
    pub fn new(root: &Path) -> Self {
        Self { root: root.join("effects"), defs: HashMap::new(), live: Vec::new(), mats: HashMap::new(), rng: Rng(0x9E37_79B9), verts: Vec::new(), idx: Vec::new() }
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

    pub fn kill(&mut self, tag: u64) {
        self.live.retain(|i| i.tag != Some(tag));
    }

    pub fn update(&mut self, dt: f32) {
        let rng = &mut self.rng;
        self.live.retain_mut(|i| i.update(dt, rng));
    }

    /// Draws every live particle; `right` / `up`: the camera's GL-space axes (billboards),
    /// `px_per_unit`: screen pixels per world unit (maxParticleSize), `view_h`: viewport px.
    pub fn draw(&mut self, right: Vec3, up: Vec3, px_per_unit: f32, view_h: f32) {
        if self.live.is_empty() {
            return;
        }
        for inst in &self.live {
            for (sd, st) in inst.def.systems.iter().zip(&inst.states) {
                if st.parts.is_empty() {
                    continue;
                }
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
                    let centre = if sd.sys.space == 1 { p.origin } else { inst.at } + p.pos * inst.scale;
                    let cg = gl(centre.x, centre.y, centre.z);
                    let (s, co) = (-p.rot).sin_cos();
                    // Quad axes (GL space) by render mode.
                    let (ax, ay) = match (sd.ren.mode, sd.ren.align) {
                        // Horizontal billboard: flat on the ground.
                        (2, _) => (vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, -1.0)),
                        // Stretched along the velocity.
                        (1, _) => {
                            let dir = gl(p.vel.x, p.vel.y, p.vel.z).normalize_or_zero();
                            let side = dir.cross(right.cross(up)).normalize_or_zero();
                            let len = sd.ren.length + p.vel.length() * sd.ren.vel_scale / size.y.max(1e-3);
                            (side, dir * len)
                        }
                        // Quad meshes aligned Local / World: the system's XY plane.
                        (4, 1) => (vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0)),
                        (4, 2) => {
                            let x = sd.rot * Vec3::X;
                            let y = sd.rot * Vec3::Y;
                            (gl(x.x, x.y, x.z), gl(y.x, y.y, y.z))
                        }
                        _ => (right, up),
                    };
                    let rx = (ax * co + ay * s) * (size.x * 0.5);
                    let ry = (ay * co - ax * s) * (size.y * 0.5);
                    let base = self.verts.len() as u16;
                    let v = |q: Vec3, a: f32, b: f32| Vertex { position: q, uv: vec2(a, b), color: c, normal: Vec4::ZERO };
                    self.verts.extend_from_slice(&[v(cg - rx - ry, u0, v1), v(cg + rx - ry, u1, v1), v(cg + rx + ry, u1, v0), v(cg - rx + ry, u0, v0)]);
                    self.idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                    if self.idx.len() >= 4800 {
                        Self::flush(&mut self.mats, sd, &mut self.verts, &mut self.idx);
                    }
                }
                Self::flush(&mut self.mats, sd, &mut self.verts, &mut self.idx);
            }
        }
    }

    fn flush(mats: &mut HashMap<(Blend, bool), Material>, sd: &SysDef, verts: &mut Vec<Vertex>, idx: &mut Vec<u16>) {
        if idx.is_empty() {
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
