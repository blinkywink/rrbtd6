//! Map materials drawn with the game's own shaders (animated water, waterfalls, god rays...).
//! tools/assets/dxbc_glsl.py translates each shader's D3D11 bytecode to GLSL 330 at export time
//! (local_extracted/shaders/<name>.json, with its constant-buffer layout and render state); here
//! the constant buffers are filled the way Unity fills them: material properties by name, plus
//! the globals these shaders read (time, the camera's view-projection, identity object matrices).

use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Comparison, Equation, PipelineParams, TextureWrap};
use macroquad::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

/// One constant buffer of one stage: `uniform vec4 <uniform>[regs]`.
struct Cb {
    uniform: String,
    regs: usize,
    /// (property, byte offset, components)
    vectors: Vec<(String, usize, usize)>,
    /// (property, byte offset): 4 registers, column-major.
    matrices: Vec<(String, usize)>,
}

pub struct FxShader {
    mat: Material,
    vs: Vec<Cb>,
    fs: Vec<Cb>,
    /// (material texture property, sampler slot)
    textures: Vec<(String, usize)>,
    /// Drawn after the opaque map (Unity's Transparent queue or no depth write).
    pub transparent: bool,
    pub queue: i32,
}

/// A part's material: its shader and its constant-buffer contents from the material.
pub struct Fx {
    pub shader: Rc<FxShader>,
    /// Register contents per constant buffer (vertex then fragment); the time / camera
    /// globals are rewritten each draw.
    regs: std::cell::RefCell<Vec<Vec<Vec4>>>,
    textures: Vec<(String, Texture2D)>,
    pub queue: i32,
}

fn cbs(v: &Value) -> Vec<Cb> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| {
            let slot = c["slot"].as_u64()?;
            Some(Cb {
                uniform: String::new() + &slot.to_string(),
                regs: c["regs"].as_u64()? as usize,
                vectors: c["vectors"]
                    .as_array()?
                    .iter()
                    .filter_map(|x| Some((x[0].as_str()?.to_string(), x[1].as_u64()? as usize, x[2].as_u64()? as usize)))
                    .collect(),
                matrices: c["matrices"].as_array()?.iter().filter_map(|x| Some((x[0].as_str()?.to_string(), x[1].as_u64()? as usize))).collect(),
            })
        })
        .collect()
}

/// UnityEngine.Rendering.BlendMode -> miniquad.
fn factor(m: u64) -> BlendFactor {
    match m {
        0 => BlendFactor::Zero,
        1 => BlendFactor::One,
        2 => BlendFactor::Value(BlendValue::DestinationColor),
        3 => BlendFactor::Value(BlendValue::SourceColor),
        4 => BlendFactor::OneMinusValue(BlendValue::DestinationColor),
        5 => BlendFactor::Value(BlendValue::SourceAlpha),
        6 => BlendFactor::OneMinusValue(BlendValue::SourceColor),
        7 => BlendFactor::Value(BlendValue::DestinationAlpha),
        8 => BlendFactor::OneMinusValue(BlendValue::DestinationAlpha),
        9 => BlendFactor::SourceAlphaSaturate,
        _ => BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
    }
}

impl FxShader {
    fn load(path: &Path) -> Option<FxShader> {
        let j: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
        let (mut vs, mut fs) = (cbs(&j["vs_cbuffers"]), cbs(&j["fs_cbuffers"]));
        for c in &mut vs {
            c.uniform = format!("vs_cb{}", c.uniform);
        }
        for c in &mut fs {
            c.uniform = format!("fs_cb{}", c.uniform);
        }
        let textures: Vec<(String, usize)> =
            j["textures"].as_array()?.iter().filter_map(|t| Some((t[0].as_str()?.to_string(), t[1].as_u64()? as usize))).collect();
        let st = &j["state"];
        let (src, dst) = (st["src"].as_f64().unwrap_or(1.0) as u64, st["dst"].as_f64().unwrap_or(0.0) as u64);
        let blend = (!(src == 1 && dst == 0)).then(|| BlendState::new(Equation::Add, factor(src), factor(dst)));
        let zwrite = st["zwrite"].as_f64().unwrap_or(1.0) != 0.0;
        let ztest = match st["ztest"].as_f64().unwrap_or(4.0) as u64 {
            0 | 8 => Comparison::Always,
            1 => Comparison::Never,
            2 => Comparison::Less,
            3 => Comparison::Equal,
            5 => Comparison::Greater,
            6 => Comparison::NotEqual,
            7 => Comparison::GreaterOrEqual,
            _ => Comparison::LessOrEqual,
        };
        let queue = match st["queue"].as_str().unwrap_or("Geometry") {
            q if q.starts_with("Transparent") => 3000,
            q if q.starts_with("AlphaTest") => 2450,
            q if q.starts_with("Background") => 1000,
            q if q.starts_with("Overlay") => 4000,
            _ => 2000,
        };
        let mut uniforms = Vec::new();
        for c in vs.iter().chain(&fs) {
            uniforms.push(UniformDesc::new(&c.uniform, UniformType::Float4).array(c.regs));
        }
        let mat = load_material(
            ShaderSource::Glsl { vertex: j["vs"].as_str()?, fragment: j["fs"].as_str()? },
            MaterialParams {
                pipeline_params: PipelineParams {
                    depth_test: ztest,
                    depth_write: zwrite,
                    color_blend: blend,
                    alpha_blend: blend,
                    ..Default::default()
                },
                uniforms,
                textures: textures.iter().map(|(_, s)| format!("t{s}")).collect(),
            },
        );
        let mat = match mat {
            Ok(m) => m,
            Err(e) => {
                eprintln!("fx: {} failed to compile: {e:?}", path.display());
                return None;
            }
        };
        Some(FxShader { mat, vs, fs, textures, transparent: !zwrite || queue >= 2500, queue })
    }
}

thread_local! {
    static SHADERS: std::cell::RefCell<HashMap<std::path::PathBuf, Option<Rc<FxShader>>>> = Default::default();
}

pub fn shader(path: &Path) -> Option<Rc<FxShader>> {
    SHADERS.with(|s| s.borrow_mut().entry(path.to_path_buf()).or_insert_with(|| FxShader::load(path).map(Rc::new)).clone())
}

/// Writes `v` (n components) at byte offset `off` of a register array.
fn put(regs: &mut [Vec4], off: usize, v: &[f32]) {
    for (i, x) in v.iter().enumerate() {
        let o = off / 4 + i;
        if let Some(r) = regs.get_mut(o / 4) {
            r[o % 4] = *x;
        }
    }
}

fn fill(c: &Cb, props: &Value, tex_size: &HashMap<String, (f32, f32)>) -> Vec<Vec4> {
    let mut regs = vec![Vec4::ZERO; c.regs];
    for (name, off, dim) in &c.vectors {
        let v: Option<Vec<f32>> = if let Some(f) = props["floats"][name].as_f64() {
            Some(vec![f as f32])
        } else if let Some(a) = props["colors"][name].as_array() {
            Some(a.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect())
        } else if let Some(t) = name.strip_suffix("_ST").map(|t| &props["textures"][t]).filter(|t| t.is_object()) {
            let s = |k: &str, i: usize| t[k][i].as_f64().unwrap_or(0.0) as f32;
            Some(vec![s("scale", 0), s("scale", 1), s("offset", 0), s("offset", 1)])
        } else if let Some(&(w, h)) = name.strip_suffix("_TexelSize").and_then(|t| tex_size.get(t)) {
            Some(vec![1.0 / w, 1.0 / h, w, h])
        } else {
            None
        };
        if let Some(v) = v {
            put(&mut regs, *off, &v[..v.len().min(*dim)]);
        }
    }
    for (name, off) in &c.matrices {
        if matches!(name.as_str(), "unity_ObjectToWorld" | "unity_WorldToObject" | "unity_MatrixM" | "unity_MatrixInvM") {
            put(&mut regs, *off, &Mat4::IDENTITY.to_cols_array());
        }
    }
    regs
}

impl Fx {
    /// The part's material, if its shader was translated. `tex` loads a texture file of the model.
    pub fn new(models_dir: &Path, part: &Value, mut tex: impl FnMut(&str) -> Option<Texture2D>) -> Option<Fx> {
        let file = part["shader_file"].as_str()?;
        // RRBTD6_NOFX=<text>: shaders whose file name contains it draw as plain textures (debug).
        if std::env::var("RRBTD6_NOFX").is_ok_and(|x| !x.is_empty() && file.contains(x.as_str())) {
            return None;
        }
        let shader = shader(&models_dir.join("..").join("..").join("shaders").join(file))?;
        let props = &part["props"];
        let mut textures = Vec::new();
        let mut sizes = HashMap::new();
        for (prop, slot) in &shader.textures {
            let t = &props["textures"][prop];
            let Some(tx) = t["file"].as_str().and_then(&mut tex) else { continue };
            let wrap = |i: usize| if t["wrap"][i].as_u64().unwrap_or(0) == 0 { TextureWrap::Repeat } else { TextureWrap::Clamp };
            let ctx = unsafe { get_internal_gl() }.quad_context;
            ctx.texture_set_wrap(tx.raw_miniquad_id(), wrap(0), wrap(1));
            sizes.insert(prop.clone(), (tx.width(), tx.height()));
            textures.push((format!("t{slot}"), tx));
        }
        let regs = shader.vs.iter().chain(&shader.fs).map(|c| fill(c, props, &sizes)).collect();
        let custom = part["queue"].as_i64().unwrap_or(-1);
        let queue = if custom > 0 { custom as i32 } else { shader.queue };
        Some(Fx { shader, regs: std::cell::RefCell::new(regs), textures, queue })
    }

    /// Draws `mesh` with this material. `vp`: the camera's view-projection in Unity world space.
    pub fn draw(&self, mesh: &Mesh, vp: Mat4, time: f32) {
        let t = [time, time.sin(), time.cos(), 0.0];
        let unity_time = [time / 20.0, time, time * 2.0, time * 3.0];
        let sh = &self.shader;
        let mut all = self.regs.borrow_mut();
        for (c, regs) in sh.vs.iter().chain(&sh.fs).zip(all.iter_mut()) {
            for (name, off, _) in &c.vectors {
                match name.as_str() {
                    "_TimeParameters" => put(regs, *off, &t),
                    "_Time" => put(regs, *off, &unity_time),
                    "_SinTime" => put(regs, *off, &[(time / 8.0).sin(), (time / 4.0).sin(), (time / 2.0).sin(), time.sin()]),
                    "_CosTime" => put(regs, *off, &[(time / 8.0).cos(), (time / 4.0).cos(), (time / 2.0).cos(), time.cos()]),
                    "_ProjectionParams" => put(regs, *off, &[1.0, 0.1, 2000.0, 1.0 / 2000.0]),
                    _ => {}
                }
            }
            for (name, off) in &c.matrices {
                if matches!(name.as_str(), "unity_MatrixVP" | "unity_MatrixVPGL" | "glstate_matrix_projection") {
                    put(regs, *off, &vp.to_cols_array());
                }
            }
            sh.mat.set_uniform_array(&c.uniform, &regs[..]);
        }
        for (name, tx) in &self.textures {
            sh.mat.set_texture(name, tx.clone());
        }
        gl_use_material(&sh.mat);
        draw_mesh(mesh);
        gl_use_default_material();
    }
}
