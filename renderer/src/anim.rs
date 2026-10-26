//! Animated tower models.
//!
//! Data comes from tools/assets/export_animated.py: every Animator clip is
//! baked to per-frame vertex positions + normals (Unity prefab space), so the
//! runtime only interpolates frames.
//!
//! Shading follows the game's shaders (DXBC disassembled with d3dcompiler's
//! D3DDisassemble): the monkey materials (`Shader Graphs/GrayscaleShader`,
//! `NinjaKiwi/BTD6/LitMonkey`) are unlit, colour = mipmapped texture sample (saturation 1).
//! Outlines are the URP "Tower Outlines" RenderObjects pass with
//! `NinjaKiwi/SimpleUnlitOutline`'s Outline pass: each vertex moves in the view plane by
//! normalize(viewNormal).xy * 0.7 * `_Outline` and is filled with `_OutlineColor`, stencil-tested
//! so it never covers the model itself (only the silhouette shows).

use crate::render3d::gl;
use macroquad::models::Vertex;
use macroquad::prelude::*;
use macroquad::miniquad::{Comparison, CullFace, PipelineParams};
use std::path::Path;

const TOON_VS: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;
varying lowp vec2 uv;
varying lowp vec4 color;
varying mediump vec3 n;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
    n = normal.xyz;
}"#;

const TOON_FS: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;
varying lowp vec4 color;
varying mediump vec3 n;
uniform sampler2D Texture;
void main() {
    gl_FragColor = vec4(texture2D(Texture, uv).rgb * color.rgb, 1.0);
}"#;

const OUTLINE_VS: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
uniform vec3 Forward;
void main() {
    // Clip-space xy offset P_diag * n_view.xy (no w) == a view-plane offset of n_view.xy.
    // normal.w carries 0.7 * _Outline (world units).
    vec3 n = normalize(normal.xyz);
    vec3 side = n - dot(n, Forward) * Forward;
    gl_Position = Projection * Model * vec4(position + side * normal.w, 1);
    color = color0 / 255.0;
}"#;

const OUTLINE_FS: &str = r#"#version 100
precision mediump float;
varying lowp vec4 color;
void main() { gl_FragColor = vec4(color.rgb, 1.0); }"#;

pub struct Shaders {
    pub unlit: Material,
    pub outline: Material,
    /// Camera forward (world, normalized); set each frame before drawing.
    pub forward: Vec3,
}

impl Shaders {
    pub fn new() -> Self {
        let unlit = load_material(
            ShaderSource::Glsl { vertex: TOON_VS, fragment: TOON_FS },
            MaterialParams {
                pipeline_params: PipelineParams {
                    depth_test: Comparison::LessOrEqual,
                    depth_write: true,
                    cull_face: CullFace::Nothing,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("unlit shader");
        // Drawn before the model without writing depth: the model then covers it, which leaves
        // only the silhouette band (the game's stencil test).
        let outline = load_material(
            ShaderSource::Glsl { vertex: OUTLINE_VS, fragment: OUTLINE_FS },
            MaterialParams {
                pipeline_params: PipelineParams {
                    depth_test: Comparison::LessOrEqual,
                    depth_write: false,
                    cull_face: CullFace::Nothing,
                    ..Default::default()
                },
                uniforms: vec![UniformDesc::new("Forward", UniformType::Float3)],
                ..Default::default()
            },
        )
        .expect("outline shader");
        Self { unlit, outline, forward: vec3(0.0, -1.0, 0.0) }
    }
}

/// Generates mipmaps and sets trilinear filtering (Unity's default for model textures).
pub fn mipmapped(tex: &Texture2D) {
    use macroquad::miniquad::{FilterMode as MqFilter, MipmapFilterMode};
    let ctx = unsafe { get_internal_gl() }.quad_context;
    let id = tex.raw_miniquad_id();
    ctx.texture_generate_mipmaps(id);
    ctx.texture_set_min_filter(id, MqFilter::Linear, MipmapFilterMode::Linear);
    ctx.texture_set_mag_filter(id, MqFilter::Linear);
}

/// Tower.show for "play the placement clip" (game.rs sets it when a tower is placed).
pub const PLACE_CLIP: &str = "@place";
/// Tower.show for "play the upgrade clip" (set on an upgrade or hero level).
pub const UPGRADE_CLIP: &str = "@upgrade";

pub struct Clip {
    pub name: String,
    pub frames: usize,
    pub fps: f32,
    pub looping: bool,
    pub offset: usize,
    pub duration: f32,
    /// MonkeyAnimationController.placementConfigs' clip (meta role "place": Fall, the drop in).
    pub place: bool,
    /// Its other state lists: "attack_idle" (attackIdleConfigs), "idle_variant" (IdleVariants,
    /// picked by `weight`), "upgrade".
    pub role: String,
    pub weight: f32,
}

/// MonkeyAnimationController.attackReadyIdleSustain (.ctor: 2.0): seconds AttackIdle plays after
/// an attack.
const ATTACK_IDLE_SUSTAIN: f32 = 2.0;
/// HYPOTHESIS P-IDLEWEIGHT: the idle loop's weight in idleSelector (SetUpHighQualityAnimator adds
/// it with a constant we couldn't read; the variants add their variantWeight, 0.08 for the Dart
/// Monkey's Look / Scratch).
const IDLE_WEIGHT: f32 = 1.0;

struct Part<T = Texture2D> {
    voff: usize,
    vcount: usize,
    uv: Vec<Vec2>,
    /// The part's triangles in draw calls macroquad takes whole: one draw is clamped to its
    /// batch capacities (5,000 indices, configured vertex capacity); larger meshes were once silently
    /// dropped (staffs, arms, whole bodies of the larger tiers). Per chunk: the part vertices
    /// it uses (empty = all of them, in order) and its indices into that list.
    chunks: Vec<(Vec<u16>, std::sync::Arc<[u16]>)>,
    tex: Option<T>,
    outline: f32,
    outline_color: [u8; 4],
    /// Per-vertex outline (outline.bin): colour and width (material `_Outline` units). Baked
    /// "Flat_" meshes keep it in their vertex colours instead of the material.
    vout: Vec<([u8; 4], f32)>,
    /// Eyelid mesh: hidden except while blinking (the prefab's DartBlink behaviour
    /// toggles this renderer; it is not meant to be visible all the time).
    blink_only: bool,
}

/// The prefab's ground shadow (`RadialShadow` SpriteRenderer, MonkeyRadialShadow sprite).
struct Shadow<T = Texture2D> {
    tex: T,
    size: f32,
    pos: [f32; 2],
}

pub struct AnimModel {
    shadow: Option<Shadow<std::rc::Rc<Texture2D>>>,
    parts: Vec<Part>,
    clips: Vec<Clip>,
    nverts: usize,
    frames: std::sync::Arc<Frames>,
    legacy_scratch:Option<(Vec<Vertex>,Vec<Vertex>)>,
    /// Set before a draw: (pitch, roll, scale) applied in the model's own space before its yaw
    /// (nose up about x, roll about its forward z, left wing down for positive roll).
    pub tilt: Option<(f32, f32, f32)>,
}

pub struct PosedMesh{
    pub vertices:Vec<Vertex>,
    pub indices:std::sync::Arc<[u16]>,
    pub texture:Option<Texture2D>,
}

const POSE_POOL_BYTES:usize=4*1024*1024;
fn pose_reuse_enabled()->bool{
    static ENABLED:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
    *ENABLED.get_or_init(||std::env::var_os("RRBTD6_NO_POSE_REUSE").is_none())
}
#[derive(Default)]struct PosePool{buffers:Vec<Vec<Vec<Vertex>>>,bytes:usize,entries:usize}
thread_local!{static POSE_POOL:std::cell::RefCell<PosePool>=Default::default();}
fn take_pose_vertices(count:usize)->Vec<Vertex>{
    let capacity=count.div_ceil(32)*32;
    if capacity*std::mem::size_of::<Vertex>()>POSE_POOL_BYTES{return Vec::with_capacity(count);}
    POSE_POOL.with(|pool|{let mut pool=pool.borrow_mut();
        if let Some(v)=pool.buffers.get_mut(capacity/32).and_then(Vec::pop){
            pool.bytes-=v.capacity()*std::mem::size_of::<Vertex>();pool.entries-=1;v
        }else{Vec::with_capacity(capacity)}
    })
}
fn recycle_pose_vertices(mut vertices:Vec<Vertex>){
    if !pose_reuse_enabled(){return;}
    vertices.clear();let bytes=vertices.capacity()*std::mem::size_of::<Vertex>();
    if bytes==0||bytes>POSE_POOL_BYTES||vertices.capacity()%32!=0{return;}
    let _=POSE_POOL.try_with(|pool|{let mut pool=pool.borrow_mut();
        if pool.entries<256&&pool.bytes+bytes<=POSE_POOL_BYTES{
            let bucket=vertices.capacity()/32;if pool.buffers.len()<=bucket{pool.buffers.resize_with(bucket+1,Vec::new);}
            pool.bytes+=bytes;pool.entries+=1;pool.buffers[bucket].push(vertices);
        }
    });
}
impl Drop for PosedMesh{fn drop(&mut self){recycle_pose_vertices(std::mem::take(&mut self.vertices));}}

// Tower exports repeat a few shadow images across many tiers. Key decoded
// pixels (including dimensions), and never keep an upload alive without a model.
thread_local!{static SHADOW_TEXTURES:std::cell::RefCell<std::collections::HashMap<blake3::Hash,std::rc::Weak<Texture2D>>>=Default::default();}
fn share_shadows()->bool{
    static SHARED:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
    *SHARED.get_or_init(||std::env::var_os("RRBTD6_NO_SHARED_SHADOWS").is_none())
}
fn shadow_key(img:&Image)->blake3::Hash{
    let mut h=blake3::Hasher::new();h.update(&img.width.to_le_bytes());h.update(&img.height.to_le_bytes());h.update(&img.bytes);h.finalize()
}
fn cached_shadow(key:&blake3::Hash)->Option<std::rc::Rc<Texture2D>>{
    SHADOW_TEXTURES.with(|c|c.borrow().get(key).and_then(std::rc::Weak::upgrade))
}
fn retain_shadow(key:blake3::Hash,texture:Texture2D)->std::rc::Rc<Texture2D>{
    let texture=std::rc::Rc::new(texture);
    SHADOW_TEXTURES.with(|c|{let mut c=c.borrow_mut();c.retain(|_,v|v.strong_count()>0);c.insert(key,std::rc::Rc::downgrade(&texture));});texture
}
fn upload_shadow(img:&Image)->std::rc::Rc<Texture2D>{
    let key=share_shadows().then(||shadow_key(img));
    if let Some(t)=key.as_ref().and_then(cached_shadow){return t;}
    let t=Texture2D::from_image(img);mipmapped(&t);
    match key{Some(key)=>retain_shadow(key,t),None=>std::rc::Rc::new(t)}
}
async fn upload_shadow_cooperative(img:&Image)->std::rc::Rc<Texture2D>{
    let key=share_shadows().then(||shadow_key(img));
    if let Some(t)=key.as_ref().and_then(cached_shadow){return t;}
    let t=crate::render3d::upload_cooperative(img,true).await;
    match key{Some(key)=>retain_shadow(key,t),None=>std::rc::Rc::new(t)}
}

/// Real-asset GPU/lifetime audit. Only runs on explicit request; ordinary loading
/// performs no readbacks. The resource tally represents base + final-path tiers,
/// not every exported model being resident in normal play.
pub async fn verify_shadow_sharing(root:&Path){
    use std::rc::Rc;
    assert!(share_shadows(),"run the audit with sharing enabled");
    let image=Image::from_file_with_format(&std::fs::read(root.join("models/DartMonkey/shadow.png")).unwrap(),None).unwrap();
    let shared=upload_shadow(&image);
    let cooperative=upload_shadow_cooperative(&image).await;
    assert!(Rc::ptr_eq(&shared,&cooperative));
    let original=Texture2D::from_image(&image);mipmapped(&original);
    assert_eq!(original.get_texture_data().bytes,shared.get_texture_data().bytes);
    let target=render_target(256,256);
    let mut camera=Camera2D::from_display_rect(Rect::new(0.0,0.0,256.0,256.0));camera.render_target=Some(target.clone());
    for size in [19.0,67.0,220.0]{
        let mut pixels=Vec::new();
        for tex in [&original,shared.as_ref()]{
            set_camera(&camera);clear_background(BLUE);
            draw_texture_ex(tex,13.0,17.0,WHITE,DrawTextureParams{dest_size:Some(vec2(size,size)),..Default::default()});
            set_default_camera();pixels.push(target.texture.get_texture_data().bytes);
        }
        assert_eq!(pixels[0],pixels[1],"shadow mip sampling changed at {size}");
        assert!(pixels[0].chunks_exact(4).any(|p|p!=[0,0,255,255]),"shadow must draw visible pixels");
    }
    let unique=Image::gen_image_color(7,9,Color::from_rgba(17,53,91,173));
    let key=shadow_key(&unique);let owned=upload_shadow(&unique);
    assert!(!Rc::ptr_eq(&owned,&shared));let weak=Rc::downgrade(&owned);
    drop(owned);assert!(weak.upgrade().is_none());assert!(cached_shadow(&key).is_none());
    let mut held=Vec::new();let mut unique=std::collections::HashMap::new();let mut before=0usize;
    for entry in std::fs::read_dir(root.join("models")).unwrap().flatten(){
        let dir=entry.path();let index=dir.join("tiers/index.json");
        let Ok(bytes)=std::fs::read(index)else{continue};
        let index:serde_json::Value=serde_json::from_slice(&bytes).unwrap();
        for tier in ["000","500","050","005"]{
            let Some(name)=index[tier].as_str()else{continue};
            let path=dir.join("tiers").join(name).join("shadow.png");
            let Ok(bytes)=std::fs::read(path)else{continue};
            let image=Image::from_file_with_format(&bytes,None).unwrap();
            let mut w=image.width as usize;let mut h=image.height as usize;let mut size=0;
            loop{size+=w*h*4;if w==1&&h==1{break;}w=(w/2).max(1);h=(h/2).max(1);}
            before+=size;let texture=upload_shadow_cooperative(&image).await;
            let key=shadow_key(&image);
            if let Some((old,_))=unique.get(&key){assert!(Rc::ptr_eq(old,&texture));}
            else{unique.insert(key,(texture.clone(),size));}
            held.push(texture);
        }
    }
    let after:usize=unique.values().map(|(_,size)|size).sum();
    eprintln!("shadow sharing: pixel/mip sampling and weak lifetime checks passed; base/final-tier shadow workload: {} uploads -> {}, {} -> {} mipmapped RGBA bytes (resource sizes, not measured process RAM)",held.len(),unique.len(),before,after);
}

/// An `AnimModel` read and decoded but not yet on the GPU: built on any thread
/// (`AnimModel::stage`), uploaded on the render thread (`AnimModel::from_staged`).
pub struct Staged {
    shadow: Option<Shadow<Image>>,
    parts: Vec<Part<StagedTex>>,
    clips: Vec<Clip>,
    nverts: usize,
    frames: std::sync::Arc<Frames>,
}

/// A part texture read off-thread: its cache key, path, and pixels if this model was the first
/// to claim it (otherwise another model's upload is reused).
struct StagedTex {
    key: crate::render3d::TexKey,
    path: std::path::PathBuf,
    img: Option<Image>,
    claimed:bool,
}
impl Drop for StagedTex{fn drop(&mut self){if self.claimed{crate::render3d::release_decode_claim(&self.key);}}}

/// Baked vertex animation, kept quantized like anim_q.bin (10 bytes per vertex per frame
/// instead of 24 as f32): positions as u16 in the model's animated bounding box, normals as
/// i8 / 127. Frame-major: [frame][vertex].
struct Frames {
    q: Vec<QVert>,
    lo: [f32; 3],
    /// (max - min) / 65535 per axis.
    step: [f32; 3],
}

// Hashing/decoding happens while staging, never while sampling animation frames.
// Weak entries let the last owning model release its animation normally.
static FRAME_CACHE:std::sync::Mutex<Option<std::collections::HashMap<blake3::Hash,std::sync::Weak<Frames>>>>=std::sync::Mutex::new(None);
fn shared_frames(bytes:&[u8],quant:Option<([f32;3],[f32;3])>)->std::sync::Arc<Frames>{
    let mut hash=blake3::Hasher::new();hash.update(&[quant.is_some() as u8]);
    if let Some((lo,hi))=quant{for v in lo.into_iter().chain(hi){hash.update(&v.to_bits().to_le_bytes());}}
    hash.update(bytes);let key=hash.finalize();
    {
        let cache=FRAME_CACHE.lock().unwrap_or_else(|e|e.into_inner());
        if let Some(frames)=cache.as_ref().and_then(|c|c.get(&key)).and_then(std::sync::Weak::upgrade){return frames;}
    }
    let decoded=std::sync::Arc::new(match quant{Some((lo,hi))=>Frames::from_q(bytes,lo,hi),None=>Frames::from_f32(bytes)});
    let mut cache=FRAME_CACHE.lock().unwrap_or_else(|e|e.into_inner());let cache=cache.get_or_insert_with(Default::default);
    // Another staging worker may have finished the same data while we decoded.
    if let Some(frames)=cache.get(&key).and_then(std::sync::Weak::upgrade){return frames;}
    if cache.len()>4096{cache.retain(|_,v|v.strong_count()>0);}
    cache.insert(key,std::sync::Arc::downgrade(&decoded));decoded
}

#[derive(Clone, Copy)]
struct QVert {
    p: [u16; 3],
    n: [i8; 3],
}

impl Frames {
    fn from_q(ab: &[u8], lo: [f32; 3], hi: [f32; 3]) -> Self {
        let q = ab
            .chunks_exact(10)
            .map(|c| QVert { p: [0, 1, 2].map(|k| u16::from_le_bytes([c[2 * k], c[2 * k + 1]])), n: [0, 1, 2].map(|k| c[6 + k] as i8) })
            .collect();
        Self { q, lo, step: [0, 1, 2].map(|k| (hi[k] - lo[k]) / 65535.0) }
    }

    /// The f32 anim.bin ([px py pz nx ny nz] per vertex), quantized the same way as
    /// compact_anims.py (1/65535 of the animated bounds).
    fn from_f32(ab: &[u8]) -> Self {
        let v: Vec<f32> = ab.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
        let (mut lo, mut hi) = ([0.0f32; 3], [0.0f32; 3]);
        if v.len() >= 6 {
            (lo, hi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for c in v.chunks_exact(6) {
                for k in 0..3 {
                    lo[k] = lo[k].min(c[k]);
                    hi[k] = hi[k].max(c[k]);
                }
            }
        }
        let span = [0, 1, 2].map(|k| (hi[k] - lo[k]).max(1e-9));
        let q = v
            .chunks_exact(6)
            .map(|c| QVert {
                p: [0, 1, 2].map(|k| (((c[k] - lo[k]) / span[k]) * 65535.0).round().clamp(0.0, 65535.0) as u16),
                n: [3, 4, 5].map(|k| (c[k].clamp(-1.0, 1.0) * 127.0).round() as i8),
            })
            .collect();
        Self { q, lo, step: [0, 1, 2].map(|k| (hi[k] - lo[k]) / 65535.0) }
    }

    /// Position and normal blended between two frame vertices.
    #[inline]
    fn lerp(&self, i0: usize, i1: usize, u: f32) -> ([f32; 3], [f32; 3]) {
        let (a, b) = (self.q[i0], self.q[i1]);
        let p = [0, 1, 2].map(|k| self.lo[k] + (a.p[k] as f32 * (1.0 - u) + b.p[k] as f32 * u) * self.step[k]);
        let n = [0, 1, 2].map(|k| (a.n[k] as f32 * (1.0 - u) + b.n[k] as f32 * u) * (1.0 / 127.0));
        (p, n)
    }
}

fn decode(path: &Path) -> Option<Image> {
    Image::from_file_with_format(&std::fs::read(path).ok()?, None).ok()
}

impl AnimModel {
    pub async fn load(dir: &Path) -> Option<Self> {
        Some(Self::from_staged(Self::stage(dir)?))
    }

    /// Uploads a staged model's textures (mipmapped, as `load` always did).
    pub fn from_staged(st: Staged) -> Self {
        let parts = st
            .parts
            .into_iter()
            .map(|p| Part {
                tex: p.tex.and_then(|t| crate::render3d::cache_tex(&t.key, t.img.as_ref(), &t.path)),
                voff: p.voff,
                vcount: p.vcount,
                uv: p.uv,
                chunks: p.chunks,
                outline: p.outline,
                outline_color: p.outline_color,
                vout: p.vout,
                blink_only: p.blink_only,
            })
            .collect();
        let shadow = st.shadow.map(|s| Shadow { tex: upload_shadow(&s.tex), size: s.size, pos: s.pos });
        Self { shadow, parts, clips: st.clips, nverts: st.nverts, frames: st.frames, legacy_scratch:(!pose_reuse_enabled()).then(Default::default),tilt: None }
    }

    async fn from_staged_cooperative(st:Staged)->Self{
        let mut budget=crate::load_work::Budget::new();let mut parts=Vec::with_capacity(st.parts.len());
        for p in st.parts{
            let tex=if let Some(mut t)=p.tex{
                if let Some(texture)=crate::render3d::cached_tex(&t.key){Some(texture)}else{
                    let img=match t.img.take(){Some(img)=>Some(img),None=>{let path=t.path.clone();crate::load_work::spawn(move||decode(&path)).await}};
                    if let Some(img)=img{Some(crate::render3d::cache_tex_cooperative(&t.key,&img).await)}else{None}
                }
            }else{None};
            parts.push(Part{
                tex,
                voff:p.voff,vcount:p.vcount,uv:p.uv,chunks:p.chunks,outline:p.outline,
                outline_color:p.outline_color,vout:p.vout,blink_only:p.blink_only,
            });budget.checkpoint().await;
        }
        let shadow=if let Some(s)=st.shadow{let tex=upload_shadow_cooperative(&s.tex).await;Some(Shadow{tex,size:s.size,pos:s.pos})}else{None};
        Self{shadow,parts,clips:st.clips,nverts:st.nverts,frames:st.frames,legacy_scratch:(!pose_reuse_enabled()).then(Default::default),tilt:None}
    }

    /// Reads and decodes a model directory without touching the GPU (safe off the main thread).
    pub fn stage(dir: &Path) -> Option<Staged> {
        let meta: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("meta.json")).ok()?).ok()?;
        let anims = meta["anims"].as_array()?;
        let vb = std::fs::read(dir.join("verts.bin")).ok()?;
        let ib = std::fs::read(dir.join("idx.bin")).ok()?;
        // anim_q.bin (tools/assets/compact_anims.py): per vertex per frame 3 x u16 position in the
        // meta "quant" box, 3 x i8 normal, 1 pad; else the original f32 anim.bin.
        let quant = meta.get("quant").and_then(|q| {
            let v = |k: &str| -> Option<[f32; 3]> {
                let a = q[k].as_array()?;
                Some([0, 1, 2].map(|i| a.get(i).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32))
            };
            Some((v("min")?, v("max")?))
        });
        let ab = match quant {
            Some(_) => std::fs::read(dir.join("anim_q.bin")).ok()?,
            None => std::fs::read(dir.join("anim.bin")).ok()?,
        };
        let ob = std::fs::read(dir.join("outline.bin")).unwrap_or_default();
        let f = |b: &[u8], o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let nverts = meta["vertex_count"].as_u64()? as usize;
        let mut parts = Vec::new();
        for p in meta["parts"].as_array()? {
            let (voff, vcount) = (p["voff"].as_u64()? as usize, p["vcount"].as_u64()? as usize);
            let (ioff, icount) = (p["ioff"].as_u64()? as usize, p["icount"].as_u64()? as usize);
            let uv = (0..vcount).map(|k| vec2(f(&vb, (voff + k) * 20 + 12), 1.0 - f(&vb, (voff + k) * 20 + 16))).collect();
            let idx: Vec<u16> = (0..icount)
                .map(|k| u32::from_le_bytes(ib[(ioff + k) * 4..(ioff + k) * 4 + 4].try_into().unwrap()) as u16)
                .collect();
            let tex = p["texture"].as_str().and_then(|t| {
                let path = dir.join(t);
                let key = crate::render3d::tex_key(&path, true)?;
                let claimed=crate::render3d::claim_tex(&key);
                let img = if claimed { decode(&path) } else { None };
                Some(StagedTex { key, path, img, claimed })
            });
            // STAND-IN: untextured quads are effect planes (glows) whose particle / additive
            // materials aren't exported; drawn plain they'd be white squares, so they're left out.
            if tex.is_none() && vcount <= 4 {
                continue;
            }
            let oc = p["outline_color"].as_array();
            let outline_color = match oc {
                Some(c) => [0, 1, 2, 3].map(|i| (c[i].as_f64().unwrap_or(0.0).clamp(0.0, 1.0) * 255.0) as u8),
                None => [30, 20, 15, 255],
            };
            let name = p["name"].as_str().unwrap_or("");
            let outline = p["outline"].as_f64().unwrap_or(0.0) as f32;
            // Baked "Flat_" parts (no material _Outline): NinjaKiwi/SimpleUnlitOutline's
            // BAKED_MESHES outline pass (DXBC) pushes out by vertex alpha x _OutlineColor.a and
            // colours with the vertex rgb. export_animated.py stored alpha x 0.97 (an earlier
            // guess at the width), so that factor is swapped for the material's alpha here.
            let baked_k = if outline == 0.0 {
                oc.and_then(|c| c.get(3)).and_then(|a| a.as_f64()).unwrap_or(1.0) as f32 / 0.97
            } else {
                1.0
            };
            let vout: Vec<([u8; 4], f32)> = if ob.len() >= (voff + vcount) * 16 {
                (0..vcount)
                    .map(|k| {
                        let o = (voff + k) * 16;
                        let c = |i: usize| (f(&ob, o + i * 4).clamp(0.0, 1.0) * 255.0) as u8;
                        ([c(0), c(1), c(2), 255], f(&ob, o + 12) * baked_k)
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let chunks = split_chunks(&idx, vcount).into_iter().map(|(v,i)|(v,i.into())).collect();
            parts.push(Part {
                // Eyelid meshes ("MonkeyEyelidsGeo", "Blink", "EngineerBlink", "NinjaEyeBlinkGeo",
                // "MermonkeyEyelid"...): shown only while blinking.
                // Wizard and Super Monkey name theirs "LOD_1": the blink material gives them away.
                blink_only: {
                    let n = name.to_lowercase();
                    let m = p["material"].as_str().unwrap_or("").to_lowercase();
                    n.contains("eyelid") || n.contains("blink") || m.contains("blink")
                },
                voff,
                vcount,
                uv,
                chunks,
                tex,
                outline: vout.iter().map(|v| v.1).fold(outline, f32::max),
                outline_color,
                vout,
            });
        }
        let clips = anims
            .iter()
            .map(|a| Clip {
                name: a["name"].as_str().unwrap_or("").to_string(),
                frames: a["frames"].as_u64().unwrap_or(1) as usize,
                fps: a["fps"].as_f64().unwrap_or(60.0) as f32,
                looping: a["loop"].as_bool().unwrap_or(false),
                offset: a["offset"].as_u64().unwrap_or(0) as usize,
                duration: a["duration"].as_f64().unwrap_or(1.0) as f32,
                place: a["role"] == "place",
                role: a["role"].as_str().unwrap_or("").to_string(),
                weight: a["weight"].as_f64().unwrap_or(0.0) as f32,
            })
            .collect();
        let frames = shared_frames(&ab,quant);
        let shadow = match meta["shadow"].as_object() {
            Some(s) => decode(&dir.join("shadow.png")).map(|tex| {
                let pos = s["pos"].as_array().map_or([0.0, 0.0], |p| [p[0].as_f64().unwrap_or(0.0) as f32, p[1].as_f64().unwrap_or(0.0) as f32]);
                Shadow { tex, size: s["size"].as_f64().unwrap_or(0.0) as f32, pos }
            }),
            None => None,
        };
        Some(Staged { shadow, parts, clips, nverts, frames })
    }

    pub fn vertex_count(&self) -> usize {
        self.nverts
    }

    /// Draws the ground shadow flat under the tower at Unity `pos`.
    pub fn draw_shadow(&self, pos: [f32; 3], yaw: f32) {
        let Some(s) = &self.shadow else { return };
        let (sn, co) = yaw.sin_cos();
        let (x, z) = (s.pos[0] * co + s.pos[1] * sn, -s.pos[0] * sn + s.pos[1] * co);
        crate::render3d::flat_sprite(&s.tex, pos[0] + x, pos[1] + 0.3, pos[2] + z, s.size, s.size, [0.5, 0.5], [1.0, 0.0]);
    }

    /// The throw clip: "Attack", "CrossbowAttack", "JuggernautAttack", ...
    pub fn attack_clip(&self) -> Option<usize> {
        self.clips.iter().position(|c| c.name.ends_with("Attack"))
    }

    /// The idle loop: "DartIdle", "CrossbowIdle", "Spike-o-pultIdle", ... ("AttackIdle" only as a fallback).
    pub fn idle_clip(&self) -> usize {
        self.clips
            .iter()
            .position(|c| c.name.ends_with("Idle") && !c.name.ends_with("AttackIdle"))
            .or_else(|| self.clips.iter().position(|c| c.name.ends_with("Idle")))
            .unwrap_or(0)
    }

    /// The clip called `name` (a one-off the game triggers: "TrueSunGodUpgrade"...).
    /// `PLACE_CLIP` is the model's placement clip, whatever it's called.
    pub fn clip_named(&self, name: &str) -> Option<usize> {
        if name == PLACE_CLIP {
            return self.clips.iter().position(|c| c.place);
        }
        if name == UPGRADE_CLIP {
            // Psi has one per path stage (Upgrade_P1..P3); which plays when isn't known yet.
            let ups: Vec<usize> = (0..self.clips.len()).filter(|&c| self.clips[c].role == "upgrade").collect();
            return (ups.len() == 1).then(|| ups[0]);
        }
        self.clips.iter().position(|c| c.name == name)
    }

    pub fn clip_duration(&self, c: usize) -> f32 {
        self.clips[c].duration
    }

    /// The idle to show and its time (MonkeyAnimationController.SelectIdleHighQuality /
    /// OnPlayableLoop): AttackIdle for ATTACK_IDLE_SUSTAIN after an attack (`since`: seconds
    /// since it); otherwise the idle loop, and each time a loop ends the next is picked by weight
    /// from it and the idle variants (looking around, scratching...). `pick` (clip, start) keeps
    /// the choice between frames; `since` is also the clock.
    pub fn idle_pose(&self, attacked: bool, since: f32, pick: &mut (usize, f32)) -> (usize, f32) {
        if attacked && since < ATTACK_IDLE_SUSTAIN {
            if let Some(a) = self.clips.iter().position(|c| c.role == "attack_idle") {
                *pick = (usize::MAX, 0.0);
                return (a, since);
            }
        }
        let idle = self.idle_clip();
        let valid = |c: usize| c == idle || self.clips.get(c).is_some_and(|x| x.role == "idle_variant");
        if !valid(pick.0) || since < pick.1 {
            *pick = (idle, since);
        }
        while since - pick.1 >= self.clips[pick.0].duration.max(1.0 / 60.0) {
            let start = pick.1 + self.clips[pick.0].duration.max(1.0 / 60.0);
            let variants: Vec<usize> = (0..self.clips.len()).filter(|&c| self.clips[c].role == "idle_variant").collect();
            let total = IDLE_WEIGHT + variants.iter().map(|&c| self.clips[c].weight).sum::<f32>();
            let mut r = macroquad::rand::gen_range(0.0, total);
            let mut next = idle;
            for &c in &variants {
                if r < self.clips[c].weight {
                    next = c;
                    break;
                }
                r -= self.clips[c].weight;
            }
            *pick = (next, start);
        }
        (pick.0, since - pick.1)
    }

    /// Draws clip `c` at time `t` (seconds), yawed (Unity convention) and placed at Unity `pos`.
    /// `outline_scale` converts the material `_Outline` value to world units (the outline
    /// vertex shader's constant 0.7). `outline_color` replaces every outline colour (the
    /// selected tower's white outline).
    #[allow(clippy::too_many_arguments)]
    pub fn draw(&mut self, sh: &Shaders, c: usize, t: f32, pos: [f32; 3], yaw: f32, outline_scale: f32, blink: bool, outline_color: Option<[u8; 4]>) {
        let (meshes, outlines) = self.meshes(c, t, pos, yaw, outline_scale, blink, outline_color);
        draw_meshes(sh, &[&outlines], &[&meshes]);
    }

    /// The posed body meshes and outline meshes `draw` would draw.
    #[allow(clippy::too_many_arguments)]
    pub fn meshes(&mut self, c: usize, t: f32, pos: [f32; 3], yaw: f32, outline_scale: f32, blink: bool, outline_color: Option<[u8; 4]>) -> (Vec<PosedMesh>, Vec<PosedMesh>) {
        self.meshes_inner(c,t,pos,yaw,outline_scale,blink,outline_color,pose_reuse_enabled())
    }

    #[allow(clippy::too_many_arguments)]
    fn meshes_inner(&mut self,c:usize,t:f32,pos:[f32;3],yaw:f32,outline_scale:f32,blink:bool,outline_color:Option<[u8;4]>,reuse:bool)->(Vec<PosedMesh>,Vec<PosedMesh>){
        let clip = &self.clips[c];
        let ft = t * clip.fps;
        let (f0, f1, u) = if clip.looping {
            let n = clip.frames as f32;
            let x = ft.rem_euclid(n);
            let a = x.floor() as usize % clip.frames;
            (a, (a + 1) % clip.frames, x.fract())
        } else {
            let x = ft.clamp(0.0, (clip.frames - 1) as f32);
            let a = x.floor() as usize;
            (a, (a + 1).min(clip.frames - 1), x.fract())
        };
        let base0 = (clip.offset + f0) * self.nverts;
        let base1 = (clip.offset + f1) * self.nverts;
        let (s, co) = yaw.sin_cos();
        let rot = |x: f32, z: f32| (x * co + z * s, -x * s + z * co);
        let tilt = self.tilt.map(|(p, r, k)| (p.sin_cos(), r.sin_cos(), k));
        let tilted = |v: [f32; 3], k: f32| -> [f32; 3] {
            let Some(((sp, cp), (sr, cr), _)) = tilt else { return v };
            let [x, y, z] = [v[0] * k, v[1] * k, v[2] * k];
            let (x, y) = (x * cr - y * sr, y * cr + x * sr);
            let (y, z) = (y * cp + z * sp, z * cp - y * sp);
            [x, y, z]
        };
        let scale = tilt.map_or(1.0, |t| t.2);
        let mut meshes = Vec::with_capacity(self.parts.len());
        let mut outlines = Vec::new();
        for part in self.parts.iter().filter(|p|blink||!p.blink_only){
            let (mut vertices,mut outline_vertices)=if reuse{
                (take_pose_vertices(part.vcount),if part.outline>0.0{take_pose_vertices(part.vcount)}else{Vec::new()})
            }else{let (mut v,mut o)=self.legacy_scratch.take().unwrap_or_default();v.clear();o.clear();(v,o)};
            for k in 0..part.vcount {
                let (vp, vn) = self.frames.lerp(base0 + part.voff + k, base1 + part.voff + k, u);
                let (vp, vn) = (tilted(vp, scale), tilted(vn, 1.0));
                let (px, pz) = rot(vp[0], vp[2]);
                let (nx, nz) = rot(vn[0], vn[2]);
                let p = gl(px + pos[0], vp[1] + pos[1], pz + pos[2]);
                let n = gl(nx, vn[1], nz).normalize_or_zero();
                vertices.push(Vertex { position: p, uv: part.uv[k], color: [255; 4], normal: n.extend(0.0) });
                if part.outline > 0.0 {
                    let (col, w) = part.vout.get(k).copied().unwrap_or((part.outline_color, part.outline));
                    outline_vertices.push(Vertex {
                        position: p,
                        uv: Vec2::ZERO,
                        color: outline_color.unwrap_or(col),
                        normal: n.extend(w * outline_scale),
                    });
                }
            }
            for (vs, idx) in &part.chunks {
                let pick = |src:&mut Vec<Vertex>| if !reuse{if vs.is_empty(){src.clone()}else{vs.iter().map(|&v|src[v as usize]).collect()}}else if vs.is_empty(){std::mem::take(src)}else{let mut out=take_pose_vertices(vs.len());out.extend(vs.iter().map(|&v|src[v as usize]));out};
                let has_outline=!outline_vertices.is_empty();
                meshes.push(PosedMesh { vertices: pick(&mut vertices), indices: idx.clone(), texture: part.tex.clone() });
                if has_outline {
                    outlines.push(PosedMesh { vertices: pick(&mut outline_vertices), indices: idx.clone(), texture: None });
                }
            }
            if reuse{recycle_pose_vertices(vertices);recycle_pose_vertices(outline_vertices);}else{self.legacy_scratch=Some((vertices,outline_vertices));}
        }
        (meshes, outlines)
    }
}

/// Splits a part's triangles into draws within macroquad's batch size (see `Part::chunks`).
fn split_chunks(idx: &[u16], vcount: usize) -> Vec<(Vec<u16>, Vec<u16>)> {
    crate::render3d::split_indexed_mesh(idx,vcount)
}

/// Every outline first, then every body, so no outline is drawn over any part of a tower (also
/// across models that make up one tower, like a Paragon's legs and top).
pub fn draw_meshes(sh: &Shaders, outlines: &[&[PosedMesh]], meshes: &[&[PosedMesh]]) {
    gl_use_material(&sh.outline);
    sh.outline.set_uniform("Forward", sh.forward);
    for m in outlines.iter().flat_map(|x| x.iter()) {
        crate::render3d::draw_geometry(&m.vertices,&m.indices,m.texture.as_ref());
    }
    gl_use_material(&sh.unlit);
    for m in meshes.iter().flat_map(|x| x.iter()) {
        crate::render3d::draw_geometry(&m.vertices,&m.indices,m.texture.as_ref());
    }
    gl_use_default_material();
}

/// Opt-in A/B check against the original macroquad mesh submission. No readback
/// or profiling occurs during ordinary play.
pub async fn verify_draw_submission(root:&Path){
    fn reference(sh:&Shaders,outlines:&[PosedMesh],meshes:&[PosedMesh]){
        gl_use_material(&sh.outline);sh.outline.set_uniform("Forward",sh.forward);
        for m in outlines{draw_mesh(&Mesh{vertices:m.vertices.clone(),indices:m.indices.to_vec(),texture:m.texture.clone()});}
        gl_use_material(&sh.unlit);
        for m in meshes{draw_mesh(&Mesh{vertices:m.vertices.clone(),indices:m.indices.to_vec(),texture:m.texture.clone()});}
        gl_use_default_material();
    }
    let shaders=Shaders::new();let rt=render_target_ex(256,256,RenderTargetParams{sample_count:1,depth:true});
    let mut camera=crate::render3d::camera([0.0,0.0],256.0,256.0);camera.fovy=70.0;camera.render_target=Some(rt.clone());
    let dir=root.join("models/DartMonkey/tiers");let index:serde_json::Value=serde_json::from_slice(&std::fs::read(dir.join("index.json")).unwrap()).unwrap();
    let mut cases=0;
    for tier in ["000","500"]{
        let mut model=AnimModel::load(&dir.join(index[tier].as_str().unwrap())).await.expect("Dart tier model");
        for variant in 0..4{
            model.tilt=if variant==3{Some((0.2,-0.1,1.2))}else{None};
            let clip=if variant%2==0{model.idle_clip()}else{model.attack_clip().unwrap_or_else(||model.idle_clip())};
            let (body,outlines)=model.meshes(clip,0.137,[0.0,0.0,0.0],variant as f32*0.7,0.7,variant%2==0,if variant==2{Some([255;4])}else{None});
            let (old_body,old_outlines)=model.meshes_inner(clip,0.137,[0.0,0.0,0.0],variant as f32*0.7,0.7,variant%2==0,if variant==2{Some([255;4])}else{None},false);
            for (old,new) in old_body.iter().chain(&old_outlines).zip(body.iter().chain(&outlines)){
                assert_eq!(old.vertices.len(),new.vertices.len());
                for (a,b) in old.vertices.iter().zip(&new.vertices){
                    assert_eq!(a.position.to_array().map(f32::to_bits),b.position.to_array().map(f32::to_bits));
                    assert_eq!(a.normal.to_array().map(f32::to_bits),b.normal.to_array().map(f32::to_bits));
                    assert_eq!(a.uv.to_array().map(f32::to_bits),b.uv.to_array().map(f32::to_bits));assert_eq!(a.color,b.color);
                }
            }
            for outline in &outlines{assert!(body.iter().any(|m|std::sync::Arc::ptr_eq(&m.indices,&outline.indices)),"body and outline must share index storage");}
            set_camera(&camera);clear_background(BLACK);reference(&shaders,&old_outlines,&old_body);set_default_camera();
            let expected=rt.texture.get_texture_data();
            set_camera(&camera);clear_background(BLACK);draw_meshes(&shaders,&[&outlines],&[&body]);set_default_camera();
            assert_eq!(expected.bytes,rt.texture.get_texture_data().bytes,"GPU pixels changed at tier {tier}, case {variant}");
            assert!(expected.bytes.chunks_exact(4).any(|p|p[0]!=0||p[1]!=0||p[2]!=0),"comparison must draw visible geometry");cases+=1;
        }
        let (mut body,_)=model.meshes(model.idle_clip(),0.1,[0.0;3],0.0,0.7,false,None);
        let mut times=[0.0f64;2];
        for sample in 0..24{
            set_camera(&camera);clear_background(BLACK);gl_use_material(&shaders.unlit);
            let legacy=sample%2==0;let began=std::time::Instant::now();
            for _ in 0..80{for m in &mut body{
                if legacy{
                    // Move/recover the vertex buffer to isolate the old per-draw
                    // index allocation/copy, without adding a vertex clone.
                    let old=Mesh{vertices:std::mem::take(&mut m.vertices),indices:m.indices.to_vec(),texture:m.texture.clone()};
                    draw_mesh(&old);m.vertices=old.vertices;
                }else{crate::render3d::draw_geometry(&m.vertices,&m.indices,m.texture.as_ref());}
            }}
            if sample>=4{times[(!legacy) as usize]+=began.elapsed().as_secs_f64();}gl_use_default_material();set_default_camera();next_frame().await;
        }
        eprintln!("render submission tier {tier}: legacy {:.3} ms, shared {:.3} ms per 80 model submissions",times[0]*1000.0/10.0,times[1]*1000.0/10.0);
    }
    eprintln!("render equivalence: {cases} real-model GPU pixel comparisons passed");
}

/// Background model loading: `request` stages a model directory on its own thread; `load`
/// uploads it when it's needed, waiting only for whatever reading is still left (or loading
/// it on the spot if it was never requested).
#[derive(Default)]
pub struct Prefetch {
    pending: std::collections::HashMap<std::path::PathBuf, std::thread::JoinHandle<Option<Staged>>>,
    warming: Option<(std::path::PathBuf, std::pin::Pin<Box<dyn std::future::Future<Output=AnimModel>>>)>,
}

impl Prefetch {
    fn can_request(&self,dir:&Path)->bool{
        self.pending.len()+usize::from(self.warming.is_some())<4&&!self.pending.contains_key(dir)
            &&self.warming.as_ref().is_none_or(|(path,_)|path!=dir)
    }

    pub fn request(&mut self, dir: std::path::PathBuf) {
        // Completed workers retain their decoded images until consumed, so the
        // pending count bounds both parallel work and retained staging memory.
        if self.can_request(&dir) {
            let d = dir.clone();
            self.pending.insert(dir, std::thread::spawn(move || AnimModel::stage(&d)));
        }
    }

    /// Borrow the persistent prediction path; allocate only when admitting a job.
    pub fn request_path(&mut self,dir:&Path){if self.can_request(dir){self.request(dir.to_owned());}}

    /// Completed obsolete predictions can be dropped. Running jobs still count
    /// toward the limit until they finish; never detach workers while retargeting.
    pub fn retire(&mut self,mut relevant:impl FnMut(&Path)->bool)->usize{
        let before=self.pending.len();
        self.pending.retain(|path,worker|relevant(path)||!worker.is_finished());
        before-self.pending.len()
    }

    pub fn load(&mut self, dir: &Path) -> Option<AnimModel> {
        let _trace=crate::load_work::Trace::new("animation",dir);
        if self.warming.as_ref().is_some_and(|(path,_)|path==dir){
            let (_,mut task)=self.warming.take().unwrap();
            let mut cx=std::task::Context::from_waker(std::task::Waker::noop());
            loop{if let std::task::Poll::Ready(model)=task.as_mut().poll(&mut cx){return Some(model);}std::thread::yield_now();}
        }
        let staged = match self.pending.remove(dir) {
            Some(h) => h.join().ok().flatten(),
            None => AnimModel::stage(dir),
        };
        staged.map(AnimModel::from_staged)
    }

    pub async fn load_cooperative(&mut self,dir:&Path)->Option<AnimModel>{
        if self.warming.as_ref().is_some_and(|(path,_)|path==dir){return Some(self.warming.take().unwrap().1.await);}
        let handle=self.pending.remove(dir).unwrap_or_else(||{let path=dir.to_owned();std::thread::spawn(move||AnimModel::stage(&path))});
        while !handle.is_finished(){crate::load_work::yield_once().await;}
        let staged=handle.join().ok().flatten()?;
        Some(AnimModel::from_staged_cooperative(staged).await)
    }

    /// Advance at most one cooperative upload per frame. Disk/decode jobs are
    /// joined only when finished; the normal load path can take over any partial
    /// upload when a display is required immediately, without omitting a frame.
    pub fn advance(&mut self, directory:&Path)->Option<(std::path::PathBuf,Option<AnimModel>)>{
        if self.warming.is_none(){
            let path=self.pending.iter().filter(|(path,job)|path.parent()==Some(directory)&&job.is_finished())
                .map(|(path,_)|path).min().cloned()?;
            let staged=self.pending.remove(&path).unwrap().join().ok().flatten();
            let Some(staged)=staged else{return Some((path,None));};
            self.warming=Some((path,Box::pin(AnimModel::from_staged_cooperative(staged))));
        }
        let (path,task)=self.warming.as_mut().unwrap();
        let mut cx=std::task::Context::from_waker(std::task::Waker::noop());
        if let std::task::Poll::Ready(model)=task.as_mut().poll(&mut cx){
            let path=path.clone();self.warming=None;Some((path,Some(model)))
        }else{None}
    }
}

pub async fn verify_prefetch(root:&Path){
    let directory=root.join("models/bloons");
    let shaders=Shaders::new();
    let target=render_target_ex(256,256,RenderTargetParams{sample_count:1,depth:true});
    let mut camera=crate::render3d::camera([0.0,0.0],256.0,256.0);
    camera.fovy=160.0;camera.render_target=Some(target.clone());
    for name in ["MoabFortified3D","BadFortified3D"]{
        let path=directory.join(name);
        for takeover in [false,true]{
            let mut loader=Prefetch::default();loader.request(path.clone());
            while !loader.pending.get(&path).unwrap().is_finished(){next_frame().await;}
            let mut prepared=if takeover{
                // Start a cooperative upload, then demand this exact model.
                match loader.advance(&directory){Some((_,model))=>model.unwrap(),None=>loader.load(&path).unwrap()}
            }else{loop{if let Some((_,model))=loader.advance(&directory){break model.unwrap();}next_frame().await;}};
            let mut reference=AnimModel::load(&path).await.unwrap();
            set_camera(&camera);clear_background(BLACK);
            reference.draw(&shaders,reference.idle_clip(),0.137,[0.0;3],0.7,0.7,false,None);
            set_default_camera();let expected=target.texture.get_texture_data();
            set_camera(&camera);clear_background(BLACK);
            prepared.draw(&shaders,prepared.idle_clip(),0.137,[0.0;3],0.7,0.7,false,None);
            set_default_camera();assert_eq!(expected.bytes,target.texture.get_texture_data().bytes,"prefetch pixels: {name} takeover={takeover}");
            assert!(expected.bytes.chunks_exact(4).any(|p|p[0]!=0||p[1]!=0||p[2]!=0));
            assert!(loader.pending.is_empty());assert!(loader.warming.is_none());
        }
    }
    eprintln!("prefetch equivalence: 4 exact GPU comparisons passed (background and immediate takeover)");
}

#[cfg(test)]mod prefetch_tests{
    use super::*;
    #[test]fn cancelled_prediction_returns_its_claim_even_after_pixels_move_into_an_upload(){
        let key=("__cancelled_prediction_claim".into(),4,true);
        assert!(crate::render3d::claim_tex(&key));
        let mut tex=StagedTex{key:key.clone(),path:Default::default(),img:Some(Image::gen_image_color(1,1,WHITE)),claimed:true};
        let pixels=tex.img.take();drop(tex);
        assert!(crate::render3d::claim_tex(&key),"a later background job must be able to decode the cancelled prediction");
        crate::render3d::release_decode_claim(&key);drop(pixels);
    }
    #[test]fn retiring_predictions_keeps_running_workers_until_they_finish(){
        let path=std::path::PathBuf::from("__obsolete_prediction");
        let barrier=std::sync::Arc::new(std::sync::Barrier::new(2));let other=barrier.clone();
        let mut p=Prefetch::default();p.pending.insert(path.clone(),std::thread::spawn(move||{other.wait();None}));
        assert_eq!(p.retire(|_|false),0);assert_eq!(p.pending.len(),1);
        barrier.wait();while !p.pending[&path].is_finished(){std::thread::yield_now();}
        assert_eq!(p.retire(|_|false),1);assert!(p.pending.is_empty());
    }
    #[test]fn obsolete_finished_predictions_free_slots_without_exceeding_the_limit(){
        let mut p=Prefetch::default();let paths:Vec<_>=(0..4).map(|i|std::path::PathBuf::from(format!("__prediction_{i}"))).collect();
        for path in &paths{p.request_path(path);}
        while p.pending.values().any(|h|!h.is_finished()){std::thread::yield_now();}
        assert_eq!(p.retire(|path|path==paths[0].as_path()),3);assert!(p.pending.contains_key(&paths[0]));
        for i in 0..8{p.request(std::path::PathBuf::from(format!("__replacement_{i}")));}
        assert_eq!(p.pending.len(),4);
    }
    #[test]fn background_advance_never_joins_a_running_worker(){
        let path=std::path::PathBuf::from("prefetch-test/model");
        let barrier=std::sync::Arc::new(std::sync::Barrier::new(2));let worker_barrier=barrier.clone();
        let mut loader=Prefetch::default();
        loader.pending.insert(path.clone(),std::thread::spawn(move||{worker_barrier.wait();None}));
        assert!(loader.advance(Path::new("prefetch-test")).is_none());
        barrier.wait();assert!(loader.load(&path).is_none());
    }
    #[test]fn posed_buffers_reuse_storage_only_after_mesh_drop(){
        POSE_POOL.with(|p|*p.borrow_mut()=PosePool::default());
        let vertices=take_pose_vertices(128);let pointer=vertices.as_ptr();
        let mesh=PosedMesh{vertices,indices:std::sync::Arc::from([0,1,2]),texture:None};
        let other=take_pose_vertices(128);assert_ne!(pointer,other.as_ptr());
        drop(mesh);let reused=take_pose_vertices(128);assert_eq!(pointer,reused.as_ptr());assert!(reused.is_empty());
        recycle_pose_vertices(reused);recycle_pose_vertices(other);
    }
    #[test]fn posed_buffer_cache_has_a_hard_byte_and_entry_limit(){
        POSE_POOL.with(|p|*p.borrow_mut()=PosePool::default());
        for _ in 0..300{recycle_pose_vertices(Vec::with_capacity(4096));}
        POSE_POOL.with(|p|{let p=p.borrow();assert!(p.bytes<=POSE_POOL_BYTES);assert!(p.entries<=256);assert_eq!(p.bytes,p.buffers.iter().flatten().map(|v|v.capacity()*std::mem::size_of::<Vertex>()).sum::<usize>());});
    }
    #[test]fn identical_animation_payloads_share_without_changing_sampling(){
        let bytes=[1,2,3,4,5,6,127,0,129,0,11,12,13,14,15,16,64,63,192,0];let bounds=Some(([-2.0,3.0,4.0],[8.0,9.0,10.0]));
        let a=shared_frames(&bytes,bounds);let b=shared_frames(&bytes,bounds);
        assert!(std::sync::Arc::ptr_eq(&a,&b));
        let decoded=Frames::from_q(&bytes,bounds.unwrap().0,bounds.unwrap().1);
        for blend in [0.0,0.25,0.37,0.75,1.0]{assert_eq!(a.lerp(0,1,blend),decoded.lerp(0,1,blend));}
        let different=shared_frames(&bytes,Some(([0.0;3],[1.0;3])));
        assert!(!std::sync::Arc::ptr_eq(&a,&different));assert_ne!(a.lerp(0,0,0.0),different.lerp(0,0,0.0));
        let mut changed=bytes;changed[0]=9;assert!(!std::sync::Arc::ptr_eq(&a,&shared_frames(&changed,bounds)));
    }
    #[test]fn shared_animation_cache_does_not_keep_unused_frames_alive(){
        let a=shared_frames(&[99,87,65,43,21,9,12,23,34,0],Some(([12.0;3],[34.0;3])));
        let weak=std::sync::Arc::downgrade(&a);drop(a);assert!(weak.upgrade().is_none());
    }
    #[test]fn concurrent_staging_reuses_one_animation_allocation(){
        let handles:Vec<_>=(0..8).map(|_|std::thread::spawn(||shared_frames(&[78,56,34,12,1,2,3,4,5,0],Some(([45.0;3],[67.0;3]))))).collect();
        let frames:Vec<_>=handles.into_iter().map(|h|h.join().unwrap()).collect();
        assert!(frames.iter().all(|f|std::sync::Arc::ptr_eq(&frames[0],f)));
    }
    #[test]
    #[ignore="requires local exported Desperado tier assets"]
    fn shared_exported_animations(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_extracted/models/Desperado/tiers");
        let frames:Vec<_>=["500","501","502","510","520"].into_iter().map(|tier|{
            let path=root.join(format!("{tier}-Desperado"));
            let meta:serde_json::Value=serde_json::from_slice(&std::fs::read(path.join("meta.json")).unwrap()).unwrap();
            let bounds=|key:&str|[0,1,2].map(|i|meta["quant"][key][i].as_f64().unwrap() as f32);
            shared_frames(&std::fs::read(path.join("anim_q.bin")).unwrap(),Some((bounds("min"),bounds("max"))))
        }).collect();
        assert!(frames.iter().all(|f|std::sync::Arc::ptr_eq(&frames[0],f)));
        let bytes=frames[0].q.len()*std::mem::size_of::<QVert>();
        eprintln!("Identical exported animations: 5 models share {:.1} MiB, avoiding {:.1} MiB of copies",bytes as f64/1048576.0,4.0*bytes as f64/1048576.0);
    }
    #[test]fn finished_workers_still_count_toward_the_staging_limit(){
        let mut p=Prefetch::default();
        let paths:Vec<_>=(0..16).map(|i|std::path::PathBuf::from(format!("__missing_prefetch_test_{i}"))).collect();
        for path in &paths{p.request(path.clone());}
        assert_eq!(p.pending.len(),4);
        for path in paths{assert!(p.load(&path).is_none());}
        assert!(p.pending.is_empty());
    }
}
