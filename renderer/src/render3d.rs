//! 3D view: BTD6's in-game camera is orthographic, pitched 60 degrees
//! (ui_scenes "Scene" camera: rotation x=0.5 w=0.866, orthographic size
//! 100.459). Models are exported by tools/assets/export_model.py in Unity
//! world space (left-handed, y up); we draw in GL space with z negated.

use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Comparison, Equation, PipelineParams};
use macroquad::models::Vertex;
use macroquad::prelude::*;
use std::path::Path;

/// Indices cap each batch at 5,000; an indexed mesh needs at most that many
/// vertices. Oversized exports are remapped before submission, not truncated.
pub fn vertex_capacity()->usize{
    static CAPACITY:std::sync::OnceLock<usize>=std::sync::OnceLock::new();
    *CAPACITY.get_or_init(||if std::env::var_os("RRBTD6_LEGACY_RENDER_STORAGE").is_some(){10000}else{5000})
}
fn opaque_batching()->bool{
    static BATCH:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
    *BATCH.get_or_init(||std::env::var_os("RRBTD6_NO_MAP_BATCHING").is_none()&&std::env::var_os("RRBTD6_LEGACY_RENDER_STORAGE").is_none())
}

/// Keep triangle order and exact vertex attributes; only renumber references
/// when an exported array is too large for a single streaming buffer.
pub(crate) fn split_indexed_mesh(idx:&[u16],vcount:usize)->Vec<(Vec<u16>,Vec<u16>)>{
    const MAX_INDICES:usize=4998;
    if idx.len()<=MAX_INDICES&&vcount<=vertex_capacity()-10{return vec![(Vec::new(),idx.to_vec())];}
    idx.chunks(MAX_INDICES).map(|tris|{
        let mut map=std::collections::HashMap::new();let mut verts=Vec::new();
        let local=tris.iter().map(|&v|*map.entry(v).or_insert_with(||{verts.push(v);(verts.len()-1) as u16})).collect();
        (verts,local)
    }).collect()
}

#[cfg(test)]mod streaming_tests{
    #[test]fn remapping_preserves_sparse_and_multichunk_triangles(){
        for (count,vertices) in [(6,7000),(4998,7000),(15003,12000)]{
            let indices:Vec<u16>=(0..count).map(|i|((i*13+6999)%vertices) as u16).collect();
            let chunks=super::split_indexed_mesh(&indices,vertices);let mut restored=Vec::new();
            for (selection,local) in chunks{
                assert!(local.len()<5000);assert_eq!(local.len()%3,0);
                let count=if selection.is_empty(){vertices}else{selection.len()};assert!(count<super::vertex_capacity());
                restored.extend(local.iter().map(|&i|if selection.is_empty(){i}else{selection[i as usize]}));
            }
            assert_eq!(restored,indices);
        }
    }
}

pub(crate) const SPRITE_VS: &str = r#"#version 100
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

pub(crate) const SPRITE_FS: &str = r#"#version 100
varying lowp vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
void main() {
    gl_FragColor = texture2D(Texture, uv) * color;
}"#;

thread_local! {
    /// Transparent sprites (bloons, pops, projectiles): depth-tested but not depth-written, so
    /// their transparent corners don't hide what is drawn after them.
    static SPRITE_MAT: Material = load_material(
        ShaderSource::Glsl { vertex: SPRITE_VS, fragment: SPRITE_FS },
        MaterialParams {
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: false,
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .expect("sprite shader");
    /// Additive sprites (glow beams: Ray of Doom's BeamLight quad is light on black).
    static ADD_MAT: Material = load_material(
        ShaderSource::Glsl { vertex: SPRITE_VS, fragment: SPRITE_FS },
        MaterialParams {
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: false,
                color_blend: Some(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::One)),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .expect("additive sprite shader");
}

/// `flat_sprite_tinted`, blended additively. With `beam` the texture's middle row runs the
/// whole length (a glow's cross-section as a continuous beam).
pub fn flat_sprite_additive(tex: &Texture2D, x: f32, y: f32, z: f32, w: f32, h: f32, pivot: [f32; 2], dir: [f32; 2], color: [u8; 4], beam: bool) {
    let fwd = vec2(dir[0], dir[1]).normalize_or_zero();
    let right = vec2(fwd.y, -fwd.x);
    let v = |u: f32, vv: f32| {
        let p = vec2(x, z) + right * ((u - pivot[0]) * w) + fwd * ((vv - pivot[1]) * h);
        Vertex { position: gl(p.x, y, p.y), uv: vec2(u, if beam { 0.5 } else { 1.0 - vv }), color, normal: Vec4::ZERO }
    };
    ADD_MAT.with(|m| {
        gl_use_material(m);
        draw_geometry(&[v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0), v(0.0, 1.0)], &[0, 1, 2, 0, 2, 3], Some(tex));
        gl_use_default_material();
    });
}

fn draw_sprite_mesh(mesh: &Mesh) {
    draw_sprite_geometry(&mesh.vertices,&mesh.indices,mesh.texture.as_ref());
}

fn draw_sprite_geometry(vertices:&[Vertex],indices:&[u16],texture:Option<&Texture2D>){
    SPRITE_MAT.with(|m| {
        gl_use_material(m);
        draw_geometry(vertices,indices,texture);
        gl_use_default_material();
    });
}

/// Same submission as macroquad::draw_mesh, borrowing immutable mesh data.
pub async fn verify_sprite_submission(){
    let texture=Texture2D::from_rgba8(2,2,&[255,20,10,255,10,255,20,128,20,10,255,200,255,255,255,0]);
    texture.set_filter(FilterMode::Linear);
    let rt=render_target_ex(128,128,RenderTargetParams{sample_count:1,depth:true});
    let camera=Camera3D{position:vec3(0.0,20.0,20.0),target:Vec3::ZERO,up:Vec3::Y,projection:Projection::Orthographics,fovy:25.0,aspect:Some(1.0),render_target:Some(rt.clone()),..Default::default()};
    for case in 0..6{
        let a=case as f32*0.31;let (s,c)=a.sin_cos();
        let vertices=[(-7.0,-7.0,0.0,1.0),(7.0,-7.0,1.0,1.0),(7.0,7.0,1.0,0.0),(-7.0,7.0,0.0,0.0)].map(|(x,z,u,v)|Vertex{position:vec3(x*c+z*s,case as f32*0.1,z*c-x*s),uv:vec2(u,v),color:[255,180,230,200],normal:Vec4::ZERO});
        let indices=[0,1,2,0,2,3];let tex=if case%2==0{Some(&texture)}else{None};
        let render=|borrowed|{
            set_camera(&camera);clear_background(BLACK);
            if borrowed{draw_sprite_geometry(&vertices,&indices,tex);}else{
                SPRITE_MAT.with(|m|{gl_use_material(m);draw_mesh(&Mesh{vertices:vertices.to_vec(),indices:indices.to_vec(),texture:tex.cloned()});gl_use_default_material();});
            }
            set_default_camera();rt.texture.get_texture_data().bytes
        };
        let reference=render(false);assert!(reference.chunks_exact(4).any(|p|p[0]>0||p[1]>0||p[2]>0));assert_eq!(reference,render(true),"sprite submission pixel mismatch {case}");
    }
    eprintln!("sprite-submission-check: 6 exact GPU comparisons passed");
}

/// Same submission as macroquad::draw_mesh, borrowing immutable mesh data.
pub fn draw_geometry(vertices:&[Vertex],indices:&[u16],texture:Option<&Texture2D>){
    // DrawMode is private in macroquad. Prime the identical public mesh state
    // without allocating arrays; its empty submission coalesces with the data.
    draw_mesh(&Mesh{vertices:Vec::new(),indices:Vec::new(),texture:texture.cloned()});
    let gl=unsafe{get_internal_gl()}.quad_gl;
    gl.geometry(vertices,indices);
}

/// `Camera.orthographicSize` of the in-game camera (half the view height).
pub const ORTHO_SIZE: f32 = 100.458_946;
pub const PITCH_DEG: f32 = 60.0;

pub fn gl(x: f32, y: f32, z: f32) -> Vec3 {
    vec3(x, y, -z)
}

pub struct Model {
    /// CPU meshes retained for streaming/transformed draws. Before directly
    /// editing map geometry/materials, call `invalidate_map_geometry`.
    pub parts: Vec<Mesh>,
    pub names: Vec<String>,
    paths: Vec<String>,
    hidden: Vec<bool>,
    /// Per part: the NinjaKiwi/BTD6/Gradient_3Color ramp, if that's its shader.
    pub grads: Vec<Option<Grad>>,
    /// Per part: its material drawn with the game's translated shader (water, waterfalls...).
    pub fx: Vec<Option<crate::fx::Fx>>,
    /// Stable render queue order, computed once when the model is loaded.
    transparent_order: Vec<usize>,
    gpu_map:Option<crate::static_map::Geometry>,
}

/// NinjaKiwi/BTD6/Gradient_3Color (DXBC): per pixel, t = mesh UV v; below `_Middle` it blends
/// _ColorBot -> _ColorMid over [0, middle], above it _ColorMid -> _ColorTop over [middle, 1].
#[derive(Clone, Copy)]
pub struct Grad {
    pub bot: Vec4,
    pub mid: Vec4,
    pub top: Vec4,
    pub middle: f32,
}

pub(crate) const GRAD_VS: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying highp float v;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    v = 1.0 - texcoord.y;
}"#;

pub(crate) const GRAD_FS: &str = r#"#version 100
precision mediump float;
varying highp float v;
uniform vec4 Bot;
uniform vec4 Mid;
uniform vec4 Top;
uniform float Middle;
void main() {
    vec4 hi = mix(Mid, Top, (v - Middle) / (1.0 - Middle));
    vec4 lo = mix(Bot, Mid, v / Middle);
    gl_FragColor = (v >= Middle ? hi : vec4(0.0)) + (Middle >= v ? lo : vec4(0.0));
}"#;

thread_local! {
    static GRAD_MAT: std::cell::OnceCell<Option<Material>> = const { std::cell::OnceCell::new() };
    /// Opaque map / prop meshes: depth-tested and depth-written, so the hollow logs, bridges
    /// and tunnels hide the bloon sprites under them (sprites are depth-tested, not written).
    static OPAQUE_MAT: Material = load_material(
        ShaderSource::Glsl { vertex: SPRITE_VS, fragment: SPRITE_FS },
        MaterialParams {
            pipeline_params: PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, ..Default::default() },
            ..Default::default()
        },
    )
    .expect("opaque shader");
}

fn draw_grad(mesh: &Mesh, g: &Grad) {
    GRAD_MAT.with(|m| {
        let m = m.get_or_init(|| {
            load_material(
                ShaderSource::Glsl { vertex: GRAD_VS, fragment: GRAD_FS },
                MaterialParams {
                    pipeline_params: PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, ..Default::default() },
                    uniforms: vec![
                        UniformDesc::new("Bot", UniformType::Float4),
                        UniformDesc::new("Mid", UniformType::Float4),
                        UniformDesc::new("Top", UniformType::Float4),
                        UniformDesc::new("Middle", UniformType::Float1),
                    ],
                    ..Default::default()
                },
            )
            .ok()
        });
        match m {
            Some(m) => {
                m.set_uniform("Bot", g.bot);
                m.set_uniform("Mid", g.mid);
                m.set_uniform("Top", g.top);
                m.set_uniform("Middle", g.middle);
                gl_use_material(m);
                draw_mesh(mesh);
                gl_use_default_material();
            }
            None => draw_mesh(mesh),
        }
    });
}

impl Model {
    pub async fn load(dir: &Path) -> Option<Model> {
        let worker_dir=dir.to_owned();
        let (meta,vb,ib,cb)=crate::load_work::spawn(move||{
            let meta:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(worker_dir.join("meta.json")).ok()?).ok()?;
            Some((meta,std::fs::read(worker_dir.join("verts.bin")).ok()?,std::fs::read(worker_dir.join("idx.bin")).ok()?,std::fs::read(worker_dir.join("colors.bin")).unwrap_or_default()))
        }).await?;
        let mut budget=crate::load_work::Budget::new();
        let f = |o: usize| f32::from_le_bytes(vb[o..o + 4].try_into().unwrap());
        let mut parts = Vec::new();
        let mut names = Vec::new();
        let mut scene_paths = Vec::new();
        let mut grads = Vec::new();
        let mut fxs = Vec::new();
        // Every part texture decoded up front, in parallel.
        let mut names_t: Vec<String> = meta["parts"].as_array()?.iter().filter_map(|p| p["texture"].as_str().map(str::to_string)).collect();
        // Textures of translated-shader materials (noise, distortion...).
        for p in meta["parts"].as_array()? {
            if let Some(t) = p["props"]["textures"].as_object() {
                names_t.extend(t.values().filter_map(|x| x["file"].as_str().map(str::to_string)));
            }
        }
        names_t.sort();
        names_t.dedup();
        let paths: Vec<std::path::PathBuf> = names_t.iter().map(|t| dir.join(t)).collect();
        // Tower / sub-tower models share textures through the cache; a map's are its own.
        let share = !dir.file_name().is_some_and(|n| n.to_string_lossy().starts_with("map_"));
        let cache: std::collections::HashMap<String, Option<Texture2D>> = if share {
            let keys: Vec<Option<TexKey>> = paths.iter().map(|p| tex_key(p, false)).collect();
            let todo: Vec<std::path::PathBuf> = paths
                .iter()
                .zip(&keys)
                .filter(|(_, k)| k.as_ref().is_some_and(|k| cached_tex(k).is_none()))
                .map(|(p, _)| p.clone())
                .collect();
            let worker_paths=todo.clone();let decoded=crate::load_work::spawn(move||decode_batch(&worker_paths)).await;
            let mut imgs:std::collections::HashMap<_,_>=todo.into_iter().zip(decoded).collect();
            let mut cache=std::collections::HashMap::new();
            for (name,(path,key)) in names_t.into_iter().zip(paths.iter().zip(keys)){
                let texture=if let Some(key)=key{
                    if let Some(texture)=cached_tex(&key){Some(texture)}else{
                        if let Some(img)=imgs.remove(path).flatten(){Some(cache_tex_cooperative(&key,&img).await)}else{None}
                    }
                }else{None};cache.insert(name,texture);
                budget.checkpoint().await;
            }cache
        } else {
            // Map textures (up to 4096^2) are seen at about the window's width: below 2048 px the
            // full-size level is never what the game's trilinear sampling shows, so it's
            // uploaded as its 2x box-filtered mip (a quarter of the memory).
            let half = screen_width() * screen_dpi_scale() <= 2048.0 && !FULL_TEXTURES.load(std::sync::atomic::Ordering::Relaxed);
            let worker_paths=paths.clone();let imgs=crate::load_work::spawn(move||decode_batch(&worker_paths)
                .into_iter()
                .map(|i| i.map(|i| if half && i.width > 2048 && i.height > 1024 { half_size(&i) } else { i }))
                .collect::<Vec<_>>()).await;
            let mut cache=std::collections::HashMap::new();
            for (name,img) in names_t.into_iter().zip(imgs){
                let texture=if let Some(img)=img{Some(upload_cooperative(&img,false).await)}else{None};
                cache.insert(name,texture);budget.checkpoint().await;
            }
            cache
        };
        for p in meta["parts"].as_array()? {
            let (voff, vcount) = (p["voff"].as_u64()? as usize, p["vcount"].as_u64()? as usize);
            let (ioff, icount) = (p["ioff"].as_u64()? as usize, p["icount"].as_u64()? as usize);
            // Material tint: one colour, or Gradient_3Color's bottom / mid / top ramp.
            let tint: Vec<[f32; 4]> = p["tint"]
                .as_array()
                .map(|a| a.iter().map(|c| {
                    let c = c.as_array().unwrap();
                    [0, 1, 2, 3].map(|i| c[i].as_f64().unwrap_or(1.0) as f32)
                }).collect())
                .unwrap_or_else(|| vec![[1.0; 4]]);
            // Exports before "middle" was recorded: Gradient_3Color's default.
            let grad = (tint.len() == 3 && p["shader"].as_str().is_none_or(|s| s == "NinjaKiwi/BTD6/Gradient_3Color")).then(|| Grad {
                bot: Vec4::from_array(tint[0]),
                mid: Vec4::from_array(tint[1]),
                top: Vec4::from_array(tint[2]),
                middle: p["middle"].as_f64().unwrap_or(0.5).clamp(1e-3, 1.0 - 1e-3) as f32,
            });
            let colour = if grad.is_some() || p["shader_file"].is_string() { [255; 4] } else { tint[0].map(|x| (x.clamp(0.0, 1.0) * 255.0) as u8) };
            let coff = p["coff"].as_u64().map(|c| c as usize).filter(|c| (c + vcount) * 4 <= cb.len());
            let verts: Vec<Vertex> = (0..vcount)
                .map(|k| {
                    let o = (voff + k) * 20;
                    Vertex {
                        position: gl(f(o), f(o + 4), f(o + 8)),
                        uv: vec2(f(o + 12), 1.0 - f(o + 16)),
                        color: coff.map_or(colour, |c| cb[(c + k) * 4..(c + k) * 4 + 4].try_into().unwrap()),
                        normal: Vec4::ZERO,
                    }
                })
                .collect();
            let idx: Vec<u16> = (0..icount)
                .map(|k| u32::from_le_bytes(ib[(ioff + k) * 4..(ioff + k) * 4 + 4].try_into().unwrap()) as u16)
                .collect();
            let tex = p["texture"].as_str().and_then(|t| cache.get(t).cloned().flatten());
            if idx.is_empty(){continue;}
            let chunks=if vertex_capacity()==10000&&verts.len()<=9990{
                // Retain the original redundant arrays for a faithful same-build A/B.
                idx.chunks(4998).map(|i|(Vec::new(),i.to_vec())).collect()
            }else{split_indexed_mesh(&idx,verts.len())};
            for (selection,ci) in chunks {
                let cv=if selection.is_empty(){verts.clone()}else{selection.iter().map(|&i|verts[i as usize]).collect()};
                parts.push(Mesh { vertices: cv, indices: ci, texture: tex.clone() });
                names.push(p["name"].as_str().unwrap_or("").to_string());
                scene_paths.push(p["path"].as_str().unwrap_or("").to_string());
                grads.push(grad);
                fxs.push(crate::fx::Fx::new(dir, p, |f| cache.get(f).cloned().flatten()));
                budget.checkpoint().await;
            }
        }
        let mut transparent_order: Vec<usize> = (0..parts.len()).filter(|&i| fxs[i].as_ref().is_some_and(|f| f.shader.transparent)).collect();
        transparent_order.sort_by_key(|&i| fxs[i].as_ref().map_or(0, |f| f.queue));
        let hidden = vec![false; parts.len()];
        let mut model=Model { parts, names, paths: scene_paths, hidden, grads, fx: fxs, transparent_order,gpu_map:None };
        if dir.file_name().is_some_and(|n|n.to_string_lossy().starts_with("map_")){
            model.gpu_map=crate::static_map::Geometry::build(&model,&mut budget).await;
            if std::env::var_os("RRBTD6_TIMING").is_some(){if let Some(g)=&model.gpu_map{let (runs,bytes)=g.resource_sizes();eprintln!("static-map: {runs} immutable draws, {bytes} buffer bytes");}}
        }
        Some(model)
    }

    /// Replace a precise scene subtree only once its animated model is ready.
    pub fn hide_subtree(&mut self, path: &str) -> bool {self.set_subtree_hidden(path,true)}
    pub fn set_subtree_hidden(&mut self, path: &str, hidden:bool) -> bool {
        let mut found = false;
        for (i, candidate) in self.paths.iter().enumerate() {
            if candidate == path || candidate.strip_prefix(path).is_some_and(|s| s.starts_with('/')) {
                self.hidden[i] = hidden;
                found = true;
            }
        }
        found
    }

    pub fn subtree_parts(&self, path: &str) -> Vec<usize> {
        self.paths.iter().enumerate().filter_map(|(i, candidate)|
            (candidate == path || candidate.strip_prefix(path).is_some_and(|s| s.starts_with('/'))).then_some(i)).collect()
    }

    pub fn parts_at_path(&self, path: &str) -> Vec<usize> {
        self.paths.iter().enumerate().filter_map(|(i,p)|(p==path).then_some(i)).collect()
    }

    /// Opaque parts first, then the transparent translated-shader ones (water ripples, edges,
    /// waterfalls) by render queue, as Unity orders them.
    pub fn draw(&self) {
        self.draw_inner(opaque_batching(),get_time() as f32);
    }

    /// Identity-transform, unclipped map pass. Menu worlds and mutable prop
    /// models deliberately retain the streaming path.
    pub fn draw_map(&self,viewport:Option<(i32,i32,i32,i32)>){self.draw_map_inner(viewport,get_time() as f32);}
    /// Drop immutable buffers before editing a map's vertices, indices or
    /// materials. Visibility changes already use the safe streaming fallback.
    pub fn invalidate_map_geometry(&mut self){self.gpu_map=None;}
    fn draw_map_inner(&self,viewport:Option<(i32,i32,i32,i32)>,time:f32)->Option<crate::static_map::DrawStats>{
        let Some(gpu)=self.gpu_map.as_ref().filter(|_|std::env::var_os("RRBTD6_PARTS").is_none())else{self.draw_inner(opaque_batching(),time);return None;};
        gl_use_default_material();
        let vp=unsafe{get_internal_gl()}.quad_gl.get_projection_matrix()*Mat4::from_scale(vec3(1.0,1.0,-1.0));
        let stats=gpu.draw(&self.hidden,viewport,|i|{
            match (&self.grads[i],&self.fx[i]){
                (_,Some(fx))=>fx.draw(&self.parts[i],vp,time),
                (Some(g),None)=>draw_grad(&self.parts[i],g),
                (None,None)=>OPAQUE_MAT.with(|mat|{gl_use_material(mat);draw_mesh(&self.parts[i]);gl_use_default_material();}),
            }
        });
        for &i in &self.transparent_order{if !self.hidden[i]{if let Some(fx)=&self.fx[i]{fx.draw(&self.parts[i],vp,time);}}}
        gl_use_default_material();Some(stats)
    }

    fn draw_inner(&self,batch:bool,time:f32) {
        let vp = unsafe { get_internal_gl() }.quad_gl.get_projection_matrix() * Mat4::from_scale(vec3(1.0, 1.0, -1.0));
        let range: Option<(usize, usize)> = std::env::var("RRBTD6_PARTS").ok().and_then(|v| {
            let (a, b) = v.split_once("..")?;
            Some((a.parse().ok()?, b.parse().ok()?))
        });
        OPAQUE_MAT.with(|mat| {
            let mut opaque=false;
            for (i, ((m, g), fx)) in self.parts.iter().zip(&self.grads).zip(&self.fx).enumerate() {
                if self.hidden[i] { continue; }
                if range.is_some_and(|(a, b)| i < a || i >= b) {
                    continue;
                }
                match (g, fx) {
                    (_, Some(fx)) if fx.shader.transparent => {}
                    (_, Some(fx)) => {if opaque{gl_use_default_material();opaque=false;}fx.draw(m, vp, time);},
                    (Some(g), None) => {if opaque{gl_use_default_material();opaque=false;}draw_grad(m, g);},
                    (None, None) => {
                        // Preserve primitive order, but keep compatible opaque parts
                        // on the same pipeline. Resetting it breaks macroquad batching.
                        if !opaque{gl_use_material(mat);opaque=true;}
                        draw_mesh(m);
                        if !batch{gl_use_default_material();opaque=false;}
                    }
                }
            }
            if opaque{gl_use_default_material();}
        });
        for &i in &self.transparent_order {
            if self.hidden[i] { continue; }
            if let Some(fx) = &self.fx[i] {
                fx.draw(&self.parts[i], vp, time);
            }
        }
    }

    /// Draw with a yaw (radians, about +y, Unity convention: +z forward) and offset (Unity world).
    pub fn draw_at(&self, pos: [f32; 3], yaw: f32, scratch: &mut Vec<Vertex>) {
        let (s, c) = yaw.sin_cos();
        for m in &self.parts {
            scratch.clear();
            scratch.extend(m.vertices.iter().map(|vx| {
                let (x, y, z) = (vx.position.x, vx.position.y, -vx.position.z);
                let (rx, rz) = (x * c + z * s, -x * s + z * c);
                Vertex { position: gl(rx + pos[0], y + pos[1], rz + pos[2]), ..*vx }
            }));
            draw_geometry(scratch,&m.indices,m.texture.as_ref());
        }
    }
}

/// Opt-in real-scene comparison, with a fixed shader clock so animated water
/// cannot mask a rendering difference between the two submission paths.
pub async fn verify_map_batching(root:&Path){
    let target=render_target_ex(512,288,RenderTargetParams{sample_count:1,depth:true});
    let mut cases=0;let mut fingerprint=blake3::Hasher::new();
    for name in ["map_tutorial","map_logs","map_tinkerton","mainmenu"]{
        let Some(mut model)=Model::load(&root.join("models").join(name)).await else{panic!("missing batch audit model {name}")};
        let mut lo=Vec3::splat(f32::INFINITY);let mut hi=Vec3::splat(f32::NEG_INFINITY);
        for v in model.parts.iter().flat_map(|m|m.indices.iter().map(|&i|&m.vertices[i as usize])){lo=lo.min(v.position);hi=hi.max(v.position);}
        let center=(lo+hi)*0.5;let mut camera=camera([0.0,0.0],512.0,288.0);
        camera.position+=center;camera.target+=center;camera.fovy=(hi-lo).length().max(1.0)*1.1;camera.render_target=Some(target.clone());
        for variant in 0..2{
            if variant==1{for (i,hidden) in model.hidden.iter_mut().enumerate(){*hidden|=i%5==0;}}
            let mut pixels=Vec::new();
            for batch in [false,true]{
                set_camera(&camera);clear_background(BLACK);model.draw_inner(batch,1.2345);set_default_camera();
                pixels.push(target.texture.get_texture_data().bytes);
            }
            assert_eq!(pixels[0],pixels[1],"map batch pixels changed: {name}, hidden variant {variant}");
            fingerprint.update(&pixels[0]);
            assert!(pixels[0].chunks_exact(4).any(|p|p[0]!=0||p[1]!=0||p[2]!=0),"audit must draw visible geometry: {name}");cases+=1;
        }
        next_frame().await;
    }
    eprintln!("map-batching-check: {cases} exact real-scene GPU comparisons passed; fingerprint {}",fingerprint.finalize());
}

/// Pixel and ownership audit of immutable maps against streaming submission.
/// Fixed clocks and geometry are used; no readback runs in ordinary play.
pub async fn verify_static_maps(root:&Path){
    assert!(crate::static_map::enabled());
    let marker=Texture2D::from_rgba8(2,2,&[255,50,10,180,255,50,10,180,50,10,255,180,50,10,255,180]);
    let mut cases=0;let mut fingerprint=blake3::Hasher::new();
    for name in ["tutorial","logs","tinkerton","adorastemple"]{
        let before=crate::static_map::live_buffers();
        let mut model=Model::load(&root.join("models").join(format!("map_{name}"))).await.expect("map audit assets");
        let (runs,bytes)=model.gpu_map.as_ref().expect("immutable map cache").resource_sizes();
        assert!(runs>0&&bytes>0);assert_eq!(crate::static_map::live_buffers(),before+runs*2);
        let mut lo=Vec3::splat(f32::INFINITY);let mut hi=Vec3::splat(f32::NEG_INFINITY);
        for v in model.parts.iter().flat_map(|m|m.indices.iter().map(|&i|&m.vertices[i as usize])){lo=lo.min(v.position);hi=hi.max(v.position);}
        let center=(lo+hi)*0.5;let span=(hi-lo).length().max(1.0);
        for (w,h,viewport,hide,samples) in [
            (512,288,None,false,1),(512,288,Some((17,11,471,253)),false,1),
            (256,144,None,false,1),(512,288,None,true,1),(512,288,None,false,4),
        ]{
            for (i,hidden) in model.hidden.iter_mut().enumerate(){*hidden=hide&&i%5==0;}
            let target=render_target_ex(w,h,RenderTargetParams{sample_count:samples,depth:true});
            let mut camera=camera([0.0,0.0],w as f32,h as f32);
            camera.position+=center;camera.target+=center;camera.fovy=span*1.1;camera.render_target=Some(target.clone());camera.viewport=viewport;
            let mut pixels=Vec::new();
            for cached in [false,true]{
                set_camera(&camera);clear_background(Color::from_rgba(17,29,43,255));
                if cached{
                    let stats=model.draw_map_inner(viewport,1.2345).expect("GPU map draw");
                    if !hide{assert_eq!(stats.cached_draws,runs);}
                }else{model.draw_inner(true,1.2345);}
                // Follow with ordinary streaming geometry, including a depth-tested
                // transparent sprite: this catches framebuffer/state/occlusion errors.
                draw_cube(center+vec3(0.0,span*0.05,0.0),Vec3::splat(span*0.07),None,PINK);
                flat_sprite(&marker,center.x,center.y+span*0.1,-center.z,span*0.16,span*0.16,[0.5,0.5],[1.0,0.0]);
                set_default_camera();pixels.push(target.texture.get_texture_data().bytes);
            }
            assert_eq!(pixels[0],pixels[1],"static GPU pixels changed: {name}, {w}x{h}, {viewport:?}, hidden {hide}, MSAA {samples}");
            assert!(pixels[0].chunks_exact(4).any(|p|p!=[17,29,43,255]));
            assert_eq!(crate::static_map::live_buffers(),before+runs*2,"draw must not allocate replacement map buffers");
            fingerprint.update(&pixels[0]);cases+=1;
        }
        eprintln!("static-map-check: {name}, {runs} immutable draws, {bytes} bytes uploaded once");
        drop(model);assert_eq!(crate::static_map::live_buffers(),before,"map buffers must release on unload");
        next_frame().await;
    }
    eprintln!("static-map-check: {cases} exact GPU/state comparisons and unload checks passed; fingerprint {}",fingerprint.finalize());
}

/// A single named part, recentred on its centroid, with its long axis (centroid -> farthest
/// vertex) returned in GL space. Used to fly the hand-held dart mesh as a projectile.
pub fn extract_part(m: &Model, name: &str) -> Option<(Model, Vec3)> {
    let i = m.names.iter().position(|n| n == name)?;
    let p = &m.parts[i];
    let v = &p.vertices;
    let c = v.iter().fold(Vec3::ZERO, |a, x| a + x.position) / v.len() as f32;
    let tip = v.iter().map(|x| x.position - c).max_by(|a, b| a.length().total_cmp(&b.length()))?;
    let verts = v.iter().map(|x| Vertex { position: x.position - c, ..*x }).collect();
    let part = Mesh { vertices: verts, indices: p.indices.clone(), texture: p.texture.clone() };
    Some((Model { parts: vec![part], names: vec![name.into()], paths: vec![String::new()], hidden: vec![false], grads: vec![None], fx: vec![None], transparent_order: Vec::new(),gpu_map:None }, tip.normalize_or_zero()))
}

/// Draws `m` (centred) at GL position `at`, rotating its `axis` onto `dir` (both GL space).
pub fn draw_oriented(m: &Model, axis: Vec3, at: Vec3, dir: Vec3, scale: f32) {
    let q = Quat::from_rotation_arc(axis, dir.normalize_or_zero());
    for p in &m.parts {
        let verts:Vec<_> = p.vertices.iter().map(|x| Vertex { position: at + q * (x.position * scale), ..*x }).collect();
        draw_geometry(&verts,&p.indices,p.texture.as_ref());
    }
}

/// Half-width of the playable map (`Map.IsPointOnMap`: x within ±150).
pub const PLAY_HALF_WIDTH: f32 = 150.0;

/// The in-game layout for a window (logical pixels), as the game builds it:
/// ScreenRatio.IsOrOver14x9 (VERIFIED-DISASM @0x1815f7130: aspect - 14/9 > -0.01) picks
/// RatioCanvasScaler.AdjustRefResolution's reference (3840 x 2160, else 3840 x 2880; CanvasScaler
/// Expand: the canvas covers it), and InGameUIRect.UpdateRect (@0x180876350) a centred UIRect of
/// a 3:2 map plus the shop (600 wide on the right, else 320 tall along the bottom) scaled to fit:
/// 3840 x 2160 or 3840 x 2880 canvas units. InGameMapRect.UpdateRect (@0x180876090) gives the
/// map camera the UIRect less the shop. Outside the UIRect the shop's tiled pattern pads.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    /// Screen px per canvas unit.
    pub scale: f32,
    /// UIRect, map rect (the camera's 3:2 view) and the shop's rect, screen px.
    pub ui: Rect,
    pub map: Rect,
    pub shop: Rect,
    /// Below 14:9: the shop is the bottom bar (ShopMenu4:3).
    pub bottom: bool,
}

pub fn layout(width: f32, height: f32) -> Layout {
    let (w, h) = (width.max(1.0), height.max(1.0));
    let bottom = !(w / h - 14.0 / 9.0 > -0.01);
    let (cw, ch) = if bottom { (3840.0, 2880.0) } else { (3840.0, 2160.0) };
    let scale = (w / cw).min(h / ch);
    let ui = Rect::new((w - cw * scale) * 0.5, (h - ch * scale) * 0.5, cw * scale, ch * scale);
    let (map, shop) = if bottom {
        let bar = 320.0 * scale;
        (Rect::new(ui.x, ui.y, ui.w, ui.h - bar), Rect::new(ui.x, ui.bottom() - bar, ui.w, bar))
    } else {
        let side = 600.0 * scale;
        (Rect::new(ui.x, ui.y, ui.w - side, ui.h), Rect::new(ui.right() - side, ui.y, side, ui.h))
    };
    Layout { scale, ui, map, shop, bottom }
}

/// The rect the map camera renders into (logical pixels): InGameMapRect.UpdateRect
/// (@0x180876090) sets Camera.pixelRect to the map rect's world corners, so the camera's view is
/// the 3:2 map rect itself, centred on the camera target (orthographicSize 100.46 at 3:2 is 301
/// units across: the playable x range, Map.IsPointOnMap's +-150).
pub fn map_frame(width:f32,height:f32)->Rect{
    layout(width, height).map
}

/// Full-window projection surrounding the unchanged native camera composition.
/// Padding is masked after rendering, including when render scale is enabled.
pub fn ortho_half_height(target: [f32; 2], aspect: f32) -> f32 {
    let _=target;
    ORTHO_SIZE * ((16.0/9.0)/aspect.max(0.001)).max(1.0)
}

pub fn camera(target: [f32; 2], viewport_w: f32, viewport_h: f32) -> Camera3D {
    let a = PITCH_DEG.to_radians();
    // Unity forward after pitching down: (0, -sin a, cos a).
    let fwd = gl(0.0, -a.sin(), a.cos());
    let t = gl(target[0], 0.0, target[1]);
    Camera3D {
        // Far back (orthographic: only depth changes) to leave room for MOABs pushed toward
        // the camera by their order (main.rs MOAB_STEP).
        position: t - fwd * 6000.0,
        target: t,
        up: gl(0.0, a.cos(), a.sin()),
        // RRBTD6_ZOOM=k (debug): k times closer.
        fovy: ORTHO_SIZE * 2.0
            / std::env::var("RRBTD6_ZOOM").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.0),
        aspect: Some(1.5),
        projection: Projection::Orthographics,
        z_near: 1.0,
        z_far: 12000.0,
        // The map rect's viewport (map_frame); GL viewports are physical pixels from
        // the bottom left, map_frame logical from the top left (high_dpi).
        viewport: {
            let f = map_frame(viewport_w, viewport_h);
            let k = screen_dpi_scale();
            Some(((f.x * k).round() as i32, ((viewport_h - f.bottom()) * k).round() as i32, (f.w * k).round() as i32, (f.h * k).round() as i32))
        },
        ..Default::default()
    }
}

/// Unity (x, y, z) -> logical screen position.
pub fn to_screen(cam: &Camera3D, p: [f32; 3], vw: f32, vh: f32) -> Vec2 {
    let n = cam.matrix().project_point3(gl(p[0], p[1], p[2]));
    let f = map_frame(vw, vh);
    vec2(f.x + (n.x + 1.0) / 2.0 * f.w, f.y + (1.0 - n.y) / 2.0 * f.h)
}

/// Mouse position -> Unity ground-plane (x, z), using the inverse camera matrix.
pub fn pick_ground(cam: &Camera3D, mouse: Vec2, vw: f32, vh: f32) -> Option<[f32; 2]> {
    let f = map_frame(vw, vh);
    if !f.contains(mouse){return None;}
    let ndc = vec2((mouse.x - f.x) / f.w * 2.0 - 1.0, 1.0 - (mouse.y - f.y) / f.h * 2.0);
    let inv = cam.matrix().inverse();
    let p0 = inv.project_point3(vec3(ndc.x, ndc.y, -1.0));
    let p1 = inv.project_point3(vec3(ndc.x, ndc.y, 1.0));
    let d = p1 - p0;
    if d.y.abs() < 1e-6 {
        return None;
    }
    let t = -p0.y / d.y;
    let h = p0 + d * t;
    Some([h.x, -h.z])
}

#[cfg(test)]mod framing_tests{
    use super::*;
    #[test]fn resized_map_keeps_native_composition_and_uniform_scale(){
        for (w,h) in [(1600.0,900.0),(800.0,900.0),(2560.0,1080.0),(900.0,1600.0),(1200.0,1100.0),(1440.0,900.0),(1.0,1.0)]{
            let l=layout(w,h);
            // InGameUIRect: 3840x2160 (14:9 and wider) or 3840x2880, uniformly scaled, centred,
            // inside the window; the map rect is 3:2 either way.
            assert_eq!(l.bottom,w/h<14.0/9.0-0.01);
            assert!((l.ui.w/l.ui.h-if l.bottom{4.0/3.0}else{16.0/9.0}).abs()<1e-4);
            assert!(l.ui.x>=-0.01&&l.ui.y>=-0.01&&l.ui.right()<=w+0.01&&l.ui.bottom()<=h+0.01);
            assert!((l.ui.center()-vec2(w/2.0,h/2.0)).length()<0.001);
            assert!((l.map.w/l.map.h-1.5).abs()<1e-4);
            // The camera renders the map rect itself (Camera.pixelRect).
            let r=map_frame(w,h);
            assert!((r.x-l.map.x).abs()<1e-4&&(r.y-l.map.y).abs()<1e-4&&(r.w-l.map.w).abs()<1e-4&&(r.h-l.map.h).abs()<1e-4);
        }
        assert_eq!(ortho_half_height([0.0,0.0],16.0/9.0),ORTHO_SIZE);
    }
}

/// A camera-facing textured quad standing on the ground at Unity (x, z).
pub fn billboard(tex: &Texture2D, x: f32, z: f32, lift: f32, w: f32) {
    let a = PITCH_DEG.to_radians();
    let h = w * tex.height() / tex.width();
    let right = vec3(1.0, 0.0, 0.0);
    let up = gl(0.0, a.cos(), a.sin());
    let base = gl(x, lift, z);
    let (bl, br) = (base - right * (w / 2.0), base + right * (w / 2.0));
    let (tl, tr) = (bl + up * h, br + up * h);
    let v = |p: Vec3, u: f32, vv: f32| Vertex { position: p, uv: vec2(u, vv), color: [255; 4], normal: Vec4::ZERO };
    draw_sprite_geometry(&[v(bl,0.0,1.0),v(br,1.0,1.0),v(tr,1.0,0.0),v(tl,0.0,0.0)],&[0,1,2,0,2,3],Some(tex));
}

/// Batches camera-facing sprites that share a texture into one draw call, keeping submission
/// order (sprites don't write depth, so order is the painter's order). Consecutive sprites with
/// the same texture merge; a texture change or a full buffer flushes.
pub struct SpriteBatch {
    tex: Option<Texture2D>,
    verts: Vec<Vertex>,
    idx: Vec<u16>,
    right: Vec3,
    up: Vec3,
    /// Draw calls issued (profiling).
    pub flushes: u32,
}

/// macroquad's default draw call holds 5000 indices: 800 quads = 4800.
const BATCH_QUADS: usize = 800;

impl SpriteBatch {
    pub fn new() -> Self {
        let a = PITCH_DEG.to_radians();
        Self {
            tex: None,
            verts: Vec::with_capacity(BATCH_QUADS * 4),
            idx: Vec::with_capacity(BATCH_QUADS * 6),
            right: vec3(1.0, 0.0, 0.0),
            up: gl(0.0, a.cos(), a.sin()),
            flushes: 0,
        }
    }

    /// Camera-facing quad of size (w, h) centred at Unity (x, height, z).
    #[allow(clippy::too_many_arguments)]
    pub fn billboard(&mut self, tex: &Texture2D, x: f32, z: f32, height: f32, w: f32, h: f32, tint: [u8; 4]) {
        self.billboard_uv(tex, [0.0, 0.0, 1.0, 1.0], x, z, height, w, h, tint);
    }

    /// `billboard` showing the [u0, v0, u1, v1] part of `tex` (an atlas entry).
    #[allow(clippy::too_many_arguments)]
    pub fn billboard_uv(&mut self, tex: &Texture2D, uv: [f32; 4], x: f32, z: f32, height: f32, w: f32, h: f32, tint: [u8; 4]) {
        if self.tex.as_ref() != Some(tex) || self.verts.len() >= BATCH_QUADS * 4 {
            self.flush();
            self.tex = Some(tex.clone());
        }
        let c = gl(x, height, z);
        let (r, u) = (self.right * (w / 2.0), self.up * (h / 2.0));
        let base = self.verts.len() as u16;
        let v = |p: Vec3, uu: f32, vv: f32| Vertex { position: p, uv: vec2(uu, vv), color: tint, normal: Vec4::ZERO };
        let [u0, v0, u1, v1] = uv;
        self.verts.extend_from_slice(&[v(c - r - u, u0, v1), v(c + r - u, u1, v1), v(c + r + u, u1, v0), v(c - r + u, u0, v0)]);
        self.idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    pub fn flush(&mut self) {
        if self.verts.is_empty() {
            return;
        }
        self.flushes += 1;
        let mesh = Mesh { vertices: std::mem::take(&mut self.verts), indices: std::mem::take(&mut self.idx), texture: self.tex.clone() };
        draw_sprite_mesh(&mesh);
        // Reuse the allocations next frame.
        self.verts = mesh.vertices;
        self.idx = mesh.indices;
        self.verts.clear();
        self.idx.clear();
    }
}

impl Default for SpriteBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// A sprite lying flat at height `y` (projectile SpriteRenderers are rotated 90 degrees about x),
/// its +y axis along the ground direction `dir`.
#[allow(clippy::too_many_arguments)]
pub fn flat_sprite(tex: &Texture2D, x: f32, y: f32, z: f32, w: f32, h: f32, pivot: [f32; 2], dir: [f32; 2]) {
    flat_sprite_tinted(tex, x, y, z, w, h, pivot, dir, [255; 4]);
}

/// `flat_sprite` with a vertex colour (SpriteRenderer.color).
#[allow(clippy::too_many_arguments)]
pub fn flat_sprite_tinted(tex: &Texture2D, x: f32, y: f32, z: f32, w: f32, h: f32, pivot: [f32; 2], dir: [f32; 2], color: [u8; 4]) {
    let fwd = vec2(dir[0], dir[1]).normalize_or_zero();
    let right = vec2(fwd.y, -fwd.x);
    let v = |u: f32, vv: f32| {
        let p = vec2(x, z) + right * ((u - pivot[0]) * w) + fwd * ((vv - pivot[1]) * h);
        Vertex { position: gl(p.x, y, p.y), uv: vec2(u, 1.0 - vv), color, normal: Vec4::ZERO }
    };
    draw_sprite_geometry(&[v(0.0,0.0),v(1.0,0.0),v(1.0,1.0),v(0.0,1.0)],&[0,1,2,0,2,3],Some(tex));
}

/// The in-game range display. Colours come from the game's `Assets/Monkeys/Materials/Range.png`
/// lookup (valid: blue fill a=51, rim a=77; invalid: red fill a=89, rim a=128). The rim width is
/// a guess.
pub fn range_disc(x: f32, z: f32, r: f32, valid: bool) {
    let (rgb, fill_a, rim_a) = if valid { ([0u8, 59, 99], 51u8, 77u8) } else { ([255, 9, 8], 89, 128) };
    let rim_w = 1.2f32.min(r * 0.5);
    let n = 72u16;
    let mut v = Vec::with_capacity(3 * n as usize + 1);
    let vert = |px: f32, pz: f32, a: u8| Vertex {
        position: gl(px, 0.6, pz),
        uv: Vec2::ZERO,
        color: [rgb[0], rgb[1], rgb[2], a],
        normal: Vec4::ZERO,
    };
    v.push(vert(x, z, fill_a));
    for k in 0..n {
        let a = k as f32 / n as f32 * std::f32::consts::TAU;
        let (s, c) = a.sin_cos();
        v.push(vert(x + (r - rim_w) * c, z + (r - rim_w) * s, fill_a));
        v.push(vert(x + (r - rim_w) * c, z + (r - rim_w) * s, rim_a));
        v.push(vert(x + r * c, z + r * s, rim_a));
    }
    let mut idx = Vec::with_capacity(9 * n as usize);
    for k in 0..n {
        let (i, j) = (1 + 3 * k, 1 + 3 * ((k + 1) % n));
        idx.extend_from_slice(&[0, i, j]);
        idx.extend_from_slice(&[i + 1, i + 2, j + 2, i + 1, j + 2, j + 1]);
    }
    draw_sprite_mesh(&Mesh { vertices: v, indices: idx, texture: None });
}

/// Flat strip `w` wide at height `y` from `a` (colour `ca`) to `b` (colour `cb`), sim coordinates.
pub fn flat_segment(a: [f32; 2], b: [f32; 2], y: f32, w: f32, ca: Color, cb: Color) {
    let d = vec2(b[0] - a[0], b[1] - a[1]).normalize_or_zero();
    let n = vec2(-d.y, d.x) * (w / 2.0);
    let vert = |p: [f32; 2], s: f32, c: Color| Vertex {
        position: gl(p[0] + n.x * s, y, p[1] + n.y * s),
        uv: Vec2::ZERO,
        color: c.into(),
        normal: Vec4::ZERO,
    };
    let v = [vert(a, -1.0, ca), vert(a, 1.0, ca), vert(b, 1.0, cb), vert(b, -1.0, cb)];
    draw_sprite_geometry(&v,&[0,1,2,0,2,3],None);
}

/// Flat band on the ground from radius `r - w` (colour `inner`) to `r` (colour `outer`).
pub fn ground_band(x: f32, z: f32, r: f32, w: f32, inner: Color, outer: Color) {
    let n = 72u16;
    let (ri, ro) = ((r - w).max(0.0), r);
    let vert = |px: f32, pz: f32, c: Color| Vertex {
        position: gl(px, 0.7, pz),
        uv: Vec2::ZERO,
        color: c.into(),
        normal: Vec4::ZERO,
    };
    let mut v = Vec::with_capacity(2 * n as usize);
    for k in 0..n {
        let (s, c) = (k as f32 / n as f32 * std::f32::consts::TAU).sin_cos();
        v.push(vert(x + ri * c, z + ri * s, inner));
        v.push(vert(x + ro * c, z + ro * s, outer));
    }
    let mut idx = Vec::with_capacity(6 * n as usize);
    for k in 0..n {
        let (i, j) = (2 * k, 2 * ((k + 1) % n));
        idx.extend_from_slice(&[i, i + 1, j + 1, i, j + 1, j]);
    }
    draw_sprite_mesh(&Mesh { vertices: v, indices: idx, texture: None });
}

/// PNGs decoded on worker threads (same pixels as loading them one by one).
pub fn decode_batch(paths: &[std::path::PathBuf]) -> Vec<Option<Image>> {
    let t0 = std::time::Instant::now();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(paths.len().max(1));
    let chunk = paths.len().div_ceil(threads.max(1)).max(1);
    let images: Vec<Option<Image>> = std::thread::scope(|sc| {
        let hs: Vec<_> = paths
            .chunks(chunk)
            .map(|ps| sc.spawn(move || ps.iter().map(|p| std::fs::read(p).ok().and_then(|b| Image::from_file_with_format(&b, None).ok())).collect::<Vec<_>>()))
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
    });
    if std::env::var("RRBTD6_TIMING").is_ok() {
        let big = images.iter().flatten().map(|i| i.width as usize * i.height as usize).max().unwrap_or(0);
        eprintln!("load: decoded {} images (largest {} px) in {:.0} ms", images.len(), big, t0.elapsed().as_secs_f64() * 1e3);
    }
    for (img, path) in images.iter().zip(paths) {
        if img.is_none() {
            eprintln!("missing texture {}", path.display());
        }
    }
    images
}

/// Settings "Texture Quality: Full": never halve big map textures.
pub static FULL_TEXTURES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

const UP_FS: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
uniform vec2 Texel;
void main() {
    // Contrast-adaptive sharpening (AMD CAS / FSR RCAS idea): a negative-lobe cross filter whose
    // strength backs off where the neighbourhood is already high-contrast.
    vec3 c = texture2D(Texture, uv).rgb;
    vec3 a = texture2D(Texture, uv + vec2(0.0, -Texel.y)).rgb;
    vec3 b = texture2D(Texture, uv + vec2(-Texel.x, 0.0)).rgb;
    vec3 d = texture2D(Texture, uv + vec2(Texel.x, 0.0)).rgb;
    vec3 e = texture2D(Texture, uv + vec2(0.0, Texel.y)).rgb;
    vec3 mn = min(c, min(min(a, b), min(d, e)));
    vec3 mx = max(c, max(max(a, b), max(d, e)));
    vec3 amp = sqrt(clamp(min(mn, 1.0 - mx) / max(mx, vec3(1e-4)), 0.0, 1.0));
    vec3 w = -amp / 6.5;
    vec3 o = (c + (a + b + d + e) * w) / (1.0 + 4.0 * w);
    gl_FragColor = vec4(clamp(o, 0.0, 1.0), 1.0);
}"#;

thread_local! {
    static UP_MAT: Material = load_material(
        ShaderSource::Glsl { vertex: SPRITE_VS, fragment: UP_FS },
        MaterialParams { uniforms: vec![UniformDesc::new("Texel", UniformType::Float2)], ..Default::default() },
    )
    .expect("upscale shader");
}

/// Draws an offscreen 3D view (render target texture) into the screen rect, optionally sharpened.
pub fn draw_scaled(tex: &Texture2D, w: f32, h: f32, sharpen: bool) {
    let params = DrawTextureParams { dest_size: Some(vec2(w, h)), flip_y: true, ..Default::default() };
    if sharpen {
        UP_MAT.with(|m| {
            m.set_uniform("Texel", vec2(1.0 / tex.width().max(1.0), 1.0 / tex.height().max(1.0)));
            gl_use_material(m);
            draw_texture_ex(tex, 0.0, 0.0, WHITE, params);
            gl_use_default_material();
        });
    } else {
        draw_texture_ex(tex, 0.0, 0.0, WHITE, params);
    }
}

/// The next mip level of an RGBA image: each output texel the mean of a 2x2 block.
pub fn half_size(img: &Image) -> Image {
    let (w, h) = (img.width as usize / 2, img.height as usize / 2);
    let src = &img.bytes;
    let sw = img.width as usize;
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            for c in 0..4 {
                let at = |dx: usize, dy: usize| src[((2 * y + dy) * sw + 2 * x + dx) * 4 + c] as u32;
                out[(y * w + x) * 4 + c] = ((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) / 4) as u8;
            }
        }
    }
    Image { bytes: out, width: w as u16, height: h as u16 }
}

/// Uploads a decoded image as a linearly filtered texture (what `load_texture` gives).
pub fn upload(img: &Image) -> Texture2D {
    let t = Texture2D::from_image(img);
    t.set_filter(FilterMode::Linear);
    t
}

/// Keep ownership managed by macroquad, but transfer large images in bounded row
/// spans. The texture is returned only when every original pixel is uploaded.
pub async fn upload_cooperative(img:&Image,mip:bool)->Texture2D{
    if img.bytes.len()<=1048576{
        let texture=upload(img);if mip{crate::anim::mipmapped(&texture);}return texture;
    }
    let mut budget=crate::load_work::Budget::new();
    let texture=Texture2D::from_rgba8(1,1,&[0,0,0,0]);
    let id=texture.raw_miniquad_id();
    unsafe{get_internal_gl()}.quad_context.texture_resize(id,img.width as u32,img.height as u32,None);
    budget.checkpoint().await;
    let stride=img.width as usize*4;let rows=(1048576/stride.max(1)).max(1);
    for y in (0..img.height as usize).step_by(rows){
        let count=rows.min(img.height as usize-y);
        unsafe{get_internal_gl()}.quad_context.texture_update_part(id,0,y as i32,img.width as i32,count as i32,&img.bytes[y*stride..(y+count)*stride]);
        budget.checkpoint().await;
    }
    texture.set_filter(FilterMode::Linear);
    if mip{budget.checkpoint().await;crate::anim::mipmapped(&texture);budget.checkpoint().await;}
    texture
}

pub async fn verify_cooperative_upload(){
    let (width,height)=(257u16,1025u16);
    let bytes:Vec<_>=(0..width as usize*height as usize*4).map(|i|(i%251) as u8).collect();
    let image=Image{width,height,bytes};let texture=upload_cooperative(&image,false).await;
    assert_eq!(texture.size(),vec2(width as f32,height as f32));
    assert_eq!(texture.get_texture_data().bytes,image.bytes,"chunked GPU upload must preserve every pixel");
    eprintln!("loading upload: multi-chunk GPU pixel check passed");
}

/// `tex` for many files at once: decoded in parallel, uploaded here (GL is single-threaded).
pub fn tex_batch(paths: &[std::path::PathBuf]) -> Vec<Option<Texture2D>> {
    decode_batch(paths).iter().map(|i| i.as_ref().map(upload)).collect()
}

/// Identity of a model texture file: exports copy the same texture into every tier / damage
/// state that uses it (3,377 files, 723 distinct images), named "<Name>_<path id>.png". Same
/// name and size = same image. `mip`: uploaded mipmapped (animated models) or not.
pub type TexKey = (String, u64, bool);

pub fn tex_key(path: &std::path::Path, mip: bool) -> Option<TexKey> {
    let len = std::fs::metadata(path).ok()?.len();
    Some((path.file_name()?.to_string_lossy().into_owned(), len, mip))
}

thread_local! {
    /// Uploaded model textures by `tex_key`: each image is on the GPU once however many models
    /// use it. Kept for the session (towers and bloons come back; maps aren't cached).
    static TEX_CACHE: std::cell::RefCell<std::collections::HashMap<TexKey, Texture2D>> = Default::default();
}

/// Keys some thread has already claimed for decoding (so duplicates aren't decoded twice).
// false: a staging owner may release this claim; true: uploaded/session-cached.
static CLAIMED: std::sync::Mutex<Option<std::collections::HashMap<TexKey,bool>>> = std::sync::Mutex::new(None);

/// True for the first caller per key: that one decodes it; the others reuse its upload.
pub fn claim_tex(key: &TexKey) -> bool {
    let mut g = CLAIMED.lock().unwrap_or_else(|e| e.into_inner());
    let claims=g.get_or_insert_with(Default::default);
    if claims.contains_key(key){false}else{claims.insert(key.clone(),false);true}
}

pub fn release_decode_claim(key:&TexKey){
    let mut g=CLAIMED.lock().unwrap_or_else(|e|e.into_inner());
    if let Some(claims)=g.as_mut(){if claims.get(key)==Some(&false){claims.remove(key);}}
}

fn mark_uploaded(key:&TexKey){
    CLAIMED.lock().unwrap_or_else(|e|e.into_inner()).get_or_insert_with(Default::default).insert(key.clone(),true);
}

pub fn cached_tex(key: &TexKey) -> Option<Texture2D> {
    TEX_CACHE.with(|c| c.borrow().get(key).cloned())
}

/// The cached texture for `key`, else uploads `img` (or decodes `path` when another caller
/// claimed the decode but hasn't uploaded yet) and caches it.
pub fn cache_tex(key: &TexKey, img: Option<&Image>, path: &std::path::Path) -> Option<Texture2D> {
    if let Some(t) = cached_tex(key) {
        return Some(t);
    }
    let decoded;
    let img = match img {
        Some(i) => i,
        None => {
            decoded = Image::from_file_with_format(&std::fs::read(path).ok()?, None).ok()?;
            &decoded
        }
    };
    let t = upload(img);
    if key.2 {
        crate::anim::mipmapped(&t);
    }
    TEX_CACHE.with(|c| c.borrow_mut().insert(key.clone(), t.clone()));
    mark_uploaded(key);
    Some(t)
}

pub async fn cache_tex_cooperative(key:&TexKey,img:&Image)->Texture2D{
    if let Some(texture)=cached_tex(key){return texture;}
    let texture=upload_cooperative(img,key.2).await;
    TEX_CACHE.with(|c|c.borrow_mut().insert(key.clone(),texture.clone()));mark_uploaded(key);texture
}

#[cfg(test)]mod decode_claim_tests{
    use super::*;
    #[test]fn completed_uploads_keep_their_decode_claim_when_a_stage_owner_drops(){
        let key=("__uploaded_prediction_claim".into(),1,true);
        assert!(claim_tex(&key));mark_uploaded(&key);release_decode_claim(&key);
        assert!(!claim_tex(&key),"an uploaded texture must not be decoded again");
    }
}

/// Several sprites packed into one texture so a painter-ordered run of different sprites is
/// still one draw call. Each sprite keeps a 2-texel border copied from its edge texels, so
/// linear filtering at its edges reads what clamp-to-edge on its own texture would: it looks
/// the same as drawing from separate textures.
pub struct Atlas {
    pub tex: Texture2D,
    /// Per input image: [u0, v0, u1, v1] (v0 at the image's top row), or None.
    pub uv: Vec<Option<[f32; 4]>>,
}

const ATLAS_PAD: usize = 2;

pub fn build_atlas(images: &[Option<&Image>]) -> Option<Atlas> {
    let (img, uv) = pack_atlas(images)?;
    Some(Atlas { tex: upload(&img), uv })
}

/// The atlas image and UV rects (see `Atlas`).
pub fn pack_atlas(images: &[Option<&Image>]) -> Option<(Image, Vec<Option<[f32; 4]>>)> {
    let pad = ATLAS_PAD;
    let widest = images.iter().flatten().map(|i| i.width as usize + 2 * pad).max()?;
    let width = widest.max(2048);
    // Shelf packing, tallest first.
    let mut order: Vec<usize> = (0..images.len()).filter(|&i| images[i].is_some()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(images[i].unwrap().height));
    let mut spots = vec![None; images.len()];
    let (mut x, mut y, mut shelf) = (0usize, 0usize, 0usize);
    for &i in &order {
        let im = images[i].unwrap();
        let (w, h) = (im.width as usize + 2 * pad, im.height as usize + 2 * pad);
        if x + w > width {
            x = 0;
            y += shelf;
            shelf = 0;
        }
        spots[i] = Some((x, y));
        x += w;
        shelf = shelf.max(h);
    }
    let height = y + shelf;
    let mut bytes = vec![0u8; width * height * 4];
    for (i, spot) in spots.iter().enumerate() {
        let (Some((ox, oy)), Some(im)) = (spot, images[i]) else { continue };
        let (w, h) = (im.width as usize, im.height as usize);
        for yy in 0..h + 2 * pad {
            let sy = yy.saturating_sub(pad).min(h - 1);
            for xx in 0..w + 2 * pad {
                let sx = xx.saturating_sub(pad).min(w - 1);
                let s = (sy * w + sx) * 4;
                let d = ((oy + yy) * width + ox + xx) * 4;
                bytes[d..d + 4].copy_from_slice(&im.bytes[s..s + 4]);
            }
        }
    }
    let uv = spots
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let ((ox, oy), im) = (s.as_ref()?, images[i]?);
            let (x0, y0) = (ox + pad, oy + pad);
            Some([
                x0 as f32 / width as f32,
                y0 as f32 / height as f32,
                (x0 + im.width as usize) as f32 / width as f32,
                (y0 + im.height as usize) as f32 / height as f32,
            ])
        })
        .collect();
    Some((Image { bytes, width: width as u16, height: height as u16 }, uv))
}

#[cfg(test)]
mod atlas_tests {
    use super::*;

    /// Bilinear sample with clamp-to-edge, as the GPU filters a Linear texture.
    fn sample(img: &Image, u: f32, v: f32) -> [f32; 4] {
        let (w, h) = (img.width as i32, img.height as i32);
        let (x, y) = (u * w as f32 - 0.5, v * h as f32 - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let px = |xi: i32, yi: i32| {
            let (xi, yi) = (xi.clamp(0, w - 1), yi.clamp(0, h - 1));
            let o = ((yi * w + xi) * 4) as usize;
            [0, 1, 2, 3].map(|c| img.bytes[o + c] as f32)
        };
        let (a, b, c, d) = (px(x0 as i32, y0 as i32), px(x0 as i32 + 1, y0 as i32), px(x0 as i32, y0 as i32 + 1), px(x0 as i32 + 1, y0 as i32 + 1));
        [0, 1, 2, 3].map(|k| (a[k] * (1.0 - fx) + b[k] * fx) * (1.0 - fy) + (c[k] * (1.0 - fx) + d[k] * fx) * fy)
    }

    /// Every sprite samples the same from the atlas as from its own texture, edges included.
    #[test]
    fn atlas_samples_match() {
        let mut seed = 12345u32;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        let imgs: Vec<Image> = (0..40)
            .map(|_| {
                let (w, h) = (3 + rnd() % 90, 3 + rnd() % 120);
                let bytes = (0..w * h * 4).map(|_| (rnd() & 255) as u8).collect();
                Image { bytes, width: w as u16, height: h as u16 }
            })
            .collect();
        let refs: Vec<Option<&Image>> = imgs.iter().map(Some).collect();
        let (atlas, uv) = pack_atlas(&refs).unwrap();
        for (img, uv) in imgs.iter().zip(&uv) {
            let [u0, v0, u1, v1] = uv.unwrap();
            for k in 0..=40 {
                for j in 0..=40 {
                    let (s, t) = (k as f32 / 40.0, j as f32 / 40.0);
                    let a = sample(img, s, t);
                    let b = sample(&atlas, u0 + (u1 - u0) * s, v0 + (v1 - v0) * t);
                    for c in 0..4 {
                        assert!((a[c] - b[c]).abs() < 0.6, "{a:?} vs {b:?} at {s},{t}");
                    }
                }
            }
        }
    }
}
