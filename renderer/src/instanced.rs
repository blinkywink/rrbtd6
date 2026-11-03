//! MOAB-class bloon models drawn as GPU instances. Every bloon of one model shares the frame's pose
//! (same idle clip, same time), so it's posed once at the origin and the GPU places each copy (its
//! position and yaw) instead of the CPU re-posing and re-sending every vertex per bloon: with 6,000
//! ZOMGs, bloon drawing went from 59 ms a frame to 0.9 ms (8 fps to the 144 fps cap).
//!
//! A run of consecutive MOAB-class bloons (in draw order) draws every outline, then every body, as
//! `anim::draw_meshes` does for one tower's models; regular bloons between runs keep their place in
//! the painter's order. MOABs apart look exactly as drawn one by one (RRBTD6_INSTANCE_CHECK: 0 of
//! 44,526 pixels differ); where they overlap, an outline no longer draws over the body of a MOAB drawn
//! before it (254 of 28,339 pixels). Which of the two the game does isn't verified.
//! Turned off with RRBTD6_NO_INSTANCED_BLOONS (or off the OpenGL backend).
use macroquad::{miniquad as mq, miniquad::CullFace, prelude::*};
use std::collections::HashMap;

use crate::anim::{AnimModel, PosedMesh, Shaders};

/// `TOON_VS` / `OUTLINE_VS` with the CPU path's placement (anim.rs `meshes_inner`: yaw about +y in
/// Unity space, then `render3d::gl`) applied to the origin-posed vertex.
const PLACE: &str = r#"
attribute vec4 inst;
vec3 turn(vec3 g) {
    float s = sin(inst.w);
    float c = cos(inst.w);
    return vec3(g.x * c - g.z * s, g.y, g.x * s + g.z * c);
}
vec3 place(vec3 g) { return turn(g) + vec3(inst.x, inst.y, -inst.z); }
"#;

const BODY_VS: &str = r#"
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
    gl_Position = Projection * Model * vec4(place(position), 1);
    color = color0 / 255.0;
    uv = texcoord;
    n = normal.xyz;
}"#;

const OUTLINE_VS: &str = r#"
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
uniform vec3 Forward;
void main() {
    vec3 n = normalize(turn(normal.xyz));
    vec3 side = n - dot(n, Forward) * Forward;
    gl_Position = Projection * Model * vec4(place(position) + side * normal.w, 1);
    color = color0 / 255.0;
}"#;

#[repr(C)]
struct BodyUniforms {
    model: [f32; 16],
    projection: [f32; 16],
}
#[repr(C)]
struct OutlineUniforms {
    model: [f32; 16],
    projection: [f32; 16],
    forward: [f32; 3],
}

struct Program {
    pipeline: mq::Pipeline,
    shader: mq::ShaderId,
}

impl Program {
    fn new(body: &str, fragment: &str, outline: bool, cull: CullFace) -> Option<Self> {
        let mut uniforms = vec![mq::UniformDesc::new("Model", mq::UniformType::Mat4), mq::UniformDesc::new("Projection", mq::UniformType::Mat4)];
        if outline {
            uniforms.push(mq::UniformDesc::new("Forward", mq::UniformType::Float3));
        }
        let vertex = format!("#version 100\n{PLACE}{body}");
        let ctx = unsafe { get_internal_gl() }.quad_context;
        let shader = ctx
            .new_shader(mq::ShaderSource::Glsl { vertex: &vertex, fragment }, mq::ShaderMeta { images: vec!["Texture".into()], uniforms: mq::UniformBlockLayout { uniforms } })
            .map_err(|e| eprintln!("instanced bloon shader: {e:?}"))
            .ok()?;
        // As anim::Shaders: the body writes depth, the outline doesn't (the body covers it after).
        let params = mq::PipelineParams { depth_test: mq::Comparison::LessOrEqual, depth_write: !outline, cull_face: cull, ..Default::default() };
        let pipeline = ctx.new_pipeline(
            &[mq::BufferLayout::default(), mq::BufferLayout { step_func: mq::VertexStep::PerInstance, ..Default::default() }],
            &[
                mq::VertexAttribute::new("position", mq::VertexFormat::Float3),
                mq::VertexAttribute::new("texcoord", mq::VertexFormat::Float2),
                mq::VertexAttribute::new("color0", mq::VertexFormat::Byte4),
                mq::VertexAttribute::new("normal", mq::VertexFormat::Float4),
                mq::VertexAttribute::with_buffer("inst", mq::VertexFormat::Float4, 1),
            ],
            shader,
            params,
        );
        Some(Self { pipeline, shader })
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        let ctx = unsafe { get_internal_gl() }.quad_context;
        ctx.delete_pipeline(self.pipeline);
        ctx.delete_shader(self.shader);
    }
}

/// A stream buffer, grown when a frame needs more.
struct Stream {
    id: mq::BufferId,
    cap: usize,
}

fn stream<T>(pool: &mut Vec<Stream>, slot: usize, kind: mq::BufferType, data: &[T]) -> mq::BufferId {
    let ctx = unsafe { get_internal_gl() }.quad_context;
    if pool.len() <= slot {
        pool.push(Stream { id: ctx.new_buffer(kind, mq::BufferUsage::Stream, mq::BufferSource::empty::<T>(data.len().max(1))), cap: data.len().max(1) });
    } else if pool[slot].cap < data.len() {
        ctx.delete_buffer(pool[slot].id);
        let cap = data.len().next_power_of_two();
        pool[slot] = Stream { id: ctx.new_buffer(kind, mq::BufferUsage::Stream, mq::BufferSource::empty::<T>(cap)), cap };
    }
    ctx.buffer_update(pool[slot].id, mq::BufferSource::slice(data));
    pool[slot].id
}

pub struct Instancer {
    /// Per cull (Nothing, Front, Back): body and outline.
    programs: Vec<(Program, Program)>,
    vertices: Vec<Stream>,
    instances: Vec<Stream>,
    /// Index buffers by the posed meshes' shared index storage (it lives as long as the model).
    indices: HashMap<usize, (mq::BufferId, std::sync::Arc<[u16]>)>,
    /// This run: model name -> (Unity position, yaw) per bloon, in order of first appearance.
    run: Vec<(String, Vec<[f32; 4]>)>,
    /// Instances already queued this run (model index, bits): an exact copy draws the same pixels
    /// again (same triangles, colours and depth), so it's skipped. A sandbox send with no spacing
    /// stacks thousands of identical MOABs on one spot.
    seen: std::collections::HashSet<(usize, [u32; 4])>,
}

const CULLS: [CullFace; 3] = [CullFace::Nothing, CullFace::Front, CullFace::Back];

impl Instancer {
    pub fn new() -> Option<Self> {
        if std::env::var_os("RRBTD6_NO_INSTANCED_BLOONS").is_some() || unsafe { get_internal_gl() }.quad_context.info().backend != mq::Backend::OpenGl {
            return None;
        }
        let mut programs = Vec::new();
        for cull in CULLS {
            programs.push((Program::new(BODY_VS, crate::anim::TOON_FS, false, cull)?, Program::new(OUTLINE_VS, crate::anim::OUTLINE_FS, true, cull)?));
        }
        Some(Self { programs, vertices: Vec::new(), instances: Vec::new(), indices: HashMap::new(), run: Vec::new(), seen: Default::default() })
    }

    /// Queues a bloon drawn with `model` at `pos` (Unity world) turned by `yaw`.
    pub fn push(&mut self, model: &str, pos: [f32; 3], yaw: f32) {
        let inst = [pos[0], pos[1], pos[2], yaw];
        let k = match self.run.iter().position(|(m, _)| m == model) {
            Some(k) => k,
            None => {
                self.run.push((model.to_string(), Vec::new()));
                self.run.len() - 1
            }
        };
        if self.seen.insert((k, inst.map(f32::to_bits))) {
            self.run[k].1.push(inst);
        }
    }

    pub fn pending(&self) -> bool {
        !self.run.is_empty()
    }

    /// Draws the queued run: posed once per model, every outline then every body.
    pub fn flush(&mut self, models: &mut HashMap<String, Option<AnimModel>>, sh: &Shaders, time: f32, outline_scale: f32, viewport: Option<(i32, i32, i32, i32)>) {
        if self.run.is_empty() {
            return;
        }
        let run = std::mem::take(&mut self.run);
        self.seen.clear();
        // (run entry, body meshes, outline meshes) per model.
        let mut posed: Vec<(usize, Vec<PosedMesh>, Vec<PosedMesh>)> = Vec::new();
        for (i, (name, _)) in run.iter().enumerate() {
            let Some(m) = models.get_mut(name).and_then(|m| m.as_mut()) else { continue };
            let idle = m.idle_clip();
            let (body, outlines) = m.meshes(idle, time, [0.0; 3], 0.0, outline_scale, false, None);
            posed.push((i, body, outlines));
        }
        let mut gl = unsafe { get_internal_gl() };
        gl.flush();
        let projection = gl.quad_gl.get_projection_matrix().to_cols_array();
        let pass = gl.quad_gl.get_active_render_pass();
        let ctx = gl.quad_context;
        let (w, h) = pass.map(|p| ctx.texture_size(ctx.render_pass_texture(p))).unwrap_or_else(|| {
            let (w, h) = mq::window::screen_size();
            (w as u32, h as u32)
        });
        let (x, y, vw, vh) = viewport.unwrap_or((0, 0, w as i32, h as i32));
        ctx.begin_pass(pass, mq::PassAction::Nothing);
        ctx.apply_viewport(x, y, vw, vh);
        ctx.apply_scissor_rect(0, 0, w as i32, h as i32);
        let empty = Texture2D::empty();
        let model = Mat4::IDENTITY.to_cols_array();
        let mut vslot = 0;
        let inst_ids: Vec<mq::BufferId> = run.iter().enumerate().map(|(i, (_, inst))| stream(&mut self.instances, i, mq::BufferType::VertexBuffer, inst)).collect();
        for outline in [true, false] {
            for (ci, cull) in CULLS.into_iter().enumerate() {
                let program = if outline { &self.programs[ci].1 } else { &self.programs[ci].0 };
                let mut applied = false;
                for (k, body, outlines) in &posed {
                    let (k, count) = (*k, run[*k].1.len());
                    for m in (if outline { outlines } else { body }).iter().filter(|m| m.cull == cull) {
                        if !applied {
                            ctx.apply_pipeline(&program.pipeline);
                            applied = true;
                        }
                        let vb = stream(&mut self.vertices, vslot, mq::BufferType::VertexBuffer, &m.vertices);
                        vslot += 1;
                        let key = std::sync::Arc::as_ptr(&m.indices) as *const u16 as usize;
                        let ib = self
                            .indices
                            .entry(key)
                            .or_insert_with(|| (ctx.new_buffer(mq::BufferType::IndexBuffer, mq::BufferUsage::Immutable, mq::BufferSource::slice(&m.indices)), m.indices.clone()))
                            .0;
                        let tex = m.texture.as_ref().unwrap_or(&empty).raw_miniquad_id();
                        ctx.apply_bindings(&mq::Bindings { vertex_buffers: vec![vb, inst_ids[k]], index_buffer: ib, images: vec![tex] });
                        if outline {
                            ctx.apply_uniforms(mq::UniformsSource::table(&OutlineUniforms { model, projection, forward: sh.forward.to_array() }));
                        } else {
                            ctx.apply_uniforms(mq::UniformsSource::table(&BodyUniforms { model, projection }));
                        }
                        ctx.draw(0, m.indices.len() as i32, count as i32);
                    }
                }
            }
        }
        ctx.end_render_pass();
    }
}

impl Drop for Instancer {
    fn drop(&mut self) {
        let ctx = unsafe { get_internal_gl() }.quad_context;
        for s in self.vertices.iter().chain(&self.instances) {
            ctx.delete_buffer(s.id);
        }
        for (id, _) in self.indices.values() {
            ctx.delete_buffer(*id);
        }
    }
}

/// RRBTD6_INSTANCE_CHECK[=<dir> for the images]: real MOAB-class models drawn one by one
/// (`AnimModel::draw`) and as instances, compared on the GPU. Apart, they must match; overlapping,
/// only outlines over another model's body differ.
pub async fn verify(root: &std::path::Path) {
    let Some(mut inst) = Instancer::new() else {
        eprintln!("instance check: instancing unavailable");
        return;
    };
    let sh = Shaders::new();
    let mut models: HashMap<String, Option<AnimModel>> = HashMap::new();
    let names = ["Moab3D", "Bfb3D", "Zomg3D", "Ddt3D"];
    for n in names {
        models.insert(n.to_string(), AnimModel::load(&root.join("models/bloons").join(n)).await);
    }
    let rt = render_target_ex(512, 512, RenderTargetParams { sample_count: 1, depth: true });
    let mut camera = crate::render3d::camera([0.0, 0.0], 512.0, 512.0);
    camera.render_target = Some(rt.clone());
    let time = 0.137;
    for (case, spacing) in [("apart", 75.0f32), ("overlapping", 22.0)] {
        // A 4 x 4 grid at the MOAB height, every model, assorted headings.
        let bloons: Vec<(&str, [f32; 3], f32)> = (0..16)
            .map(|k| {
                let (gx, gz) = ((k % 4) as f32 - 1.5, (k / 4) as f32 - 1.5);
                (names[k % names.len()], [gx * spacing, 120.0, gz * spacing], k as f32 * 0.9)
            })
            .collect();
        set_camera(&camera);
        clear_background(BLACK);
        for &(n, pos, yaw) in &bloons {
            if let Some(m) = models.get_mut(n).and_then(|m| m.as_mut()) {
                let idle = m.idle_clip();
                m.draw(&sh, idle, time, pos, yaw, 0.7, false, None);
            }
        }
        set_default_camera();
        let expected = rt.texture.get_texture_data();
        set_camera(&camera);
        clear_background(BLACK);
        for &(n, pos, yaw) in &bloons {
            inst.push(n, pos, yaw);
        }
        inst.flush(&mut models, &sh, time, 0.7, camera.viewport);
        set_default_camera();
        let got = rt.texture.get_texture_data();
        let drawn = expected.bytes.chunks_exact(4).filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0).count();
        let differ = expected.bytes.chunks_exact(4).zip(got.bytes.chunks_exact(4)).filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 8)).count();
        eprintln!("instance check {case}: {drawn} pixels drawn, {differ} differ");
        // RRBTD6_INSTANCE_CHECK=<dir>: both images, for a look.
        let out = std::path::PathBuf::from(std::env::var_os("RRBTD6_INSTANCE_CHECK").unwrap_or_default());
        if out.is_dir() {
            expected.export_png(&out.join(format!("{case}-each.png")).to_string_lossy());
            got.export_png(&out.join(format!("{case}-instanced.png")).to_string_lossy());
        }
        assert!(drawn > 1000, "the check must draw visible models");
        if case == "apart" {
            assert!(differ * 1000 <= drawn, "instanced MOABs apart must match the per-bloon draw");
        }
        next_frame().await;
    }
}
