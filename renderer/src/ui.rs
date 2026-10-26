//! Game UI primitives: TextMeshPro SDF fonts (tools/assets/export_font.py) rendered with the
//! TMP_SDF shader maths and the game's material settings, and a sprite library over the
//! exported atlases (tools/assets/export_sprites.py) with 9-slice borders.

use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams};
use macroquad::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const SDF_VS: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec4 color;
varying highp vec2 uv;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    color = color0 / 255.0;
    uv = texcoord;
}
"#;

// TMP_SDF.shader fragment: GetColor() face/outline blend plus the underlay pass. Output is
// premultiplied alpha. P0 = (scale, bias, outline, softness); U = (uv offset, bScale, bBias).
const SDF_FS: &str = r#"#version 100
precision highp float;
varying lowp vec4 color;
varying highp vec2 uv;
uniform sampler2D Texture;
uniform vec4 P0;
uniform vec4 FaceColor;
uniform vec4 OutlineColor;
uniform vec4 UnderlayColor;
uniform vec4 U;
void main() {
    float c = texture2D(Texture, uv).a;
    float sd = (P0.y - c) * P0.x;
    float outline = P0.z;
    float softness = P0.w;
    vec4 face = FaceColor * color;
    vec4 oc = OutlineColor;
    oc.a *= color.a;
    float faceAlpha = 1.0 - clamp((sd - outline * 0.5 + softness * 0.5) / (1.0 + softness), 0.0, 1.0);
    float outlineAlpha = clamp(sd + outline * 0.5, 0.0, 1.0) * sqrt(min(1.0, outline));
    face.rgb *= face.a;
    oc.rgb *= oc.a;
    face = mix(face, oc, outlineAlpha);
    face *= faceAlpha;
    if (UnderlayColor.a > 0.0) {
        float d = texture2D(Texture, uv + U.xy).a * U.z;
        vec4 ul = UnderlayColor;
        ul.rgb *= ul.a;
        ul *= color.a;
        face += ul * clamp(d - U.w, 0.0, 1.0) * (1.0 - face.a);
    }
    gl_FragColor = face;
}
"#;

/// TMP material properties (dumped per text by dump_ui_prefab.py).
#[derive(Clone, Debug)]
pub struct TmpStyle {
    pub face: Color,
    pub outline: Color,
    pub outline_width: f32,
    pub outline_softness: f32,
    pub face_dilate: f32,
    pub underlay: Option<(Color, Vec2, f32, f32)>,
    pub gradient_scale: f32,
    pub ratio_a: f32,
    pub ratio_c: f32,
    pub weight_normal: f32,
    pub weight_bold: f32,
}

impl Default for TmpStyle {
    fn default() -> Self {
        Self {
            face: WHITE,
            outline: BLACK,
            outline_width: 0.0,
            outline_softness: 0.0,
            face_dilate: 0.0,
            underlay: None,
            gradient_scale: 6.0,
            ratio_a: 1.0,
            ratio_c: 1.0,
            weight_normal: 0.0,
            weight_bold: 0.75,
        }
    }
}

pub fn color(v: &serde_json::Value) -> Option<Color> {
    let a = v.as_array()?;
    let f = |i: usize| a.get(i).and_then(|x| x.as_f64()).unwrap_or(1.0) as f32;
    Some(Color::new(f(0), f(1), f(2), f(3)))
}

impl TmpStyle {
    pub fn from_json(m: &serde_json::Value) -> Self {
        let mut s = Self::default();
        let f = |k: &str, d: f32| m[k].as_f64().map(|x| x as f32).unwrap_or(d);
        let keywords: Vec<&str> = m["keywords"].as_array().map(|a| a.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default();
        s.face = color(&m["_FaceColor"]).unwrap_or(WHITE);
        s.outline = color(&m["_OutlineColor"]).unwrap_or(BLACK);
        s.outline_width = f("_OutlineWidth", 0.0);
        s.outline_softness = f("_OutlineSoftness", 0.0);
        s.face_dilate = f("_FaceDilate", 0.0);
        s.gradient_scale = f("_GradientScale", 6.0);
        s.ratio_a = f("_ScaleRatioA", 1.0);
        s.ratio_c = f("_ScaleRatioC", 1.0);
        s.weight_normal = f("_WeightNormal", 0.0);
        s.weight_bold = f("_WeightBold", 0.75);
        if keywords.contains(&"UNDERLAY_ON") {
            s.underlay = Some((
                color(&m["_UnderlayColor"]).unwrap_or(BLACK),
                vec2(f("_UnderlayOffsetX", 0.0), f("_UnderlayOffsetY", 0.0)),
                f("_UnderlayDilate", 0.0),
                f("_UnderlaySoftness", 0.0),
            ));
        }
        s
    }
}

struct Glyph {
    src: Rect,
    bearing: Vec2,
    advance: f32,
}

pub struct SdfFont {
    identity:u64,
    tex: Texture2D,
    glyphs: HashMap<char, Glyph>,
    pub point: f32,
    pub ascent: f32,
    pub descent: f32,
    pub line_height: f32,
    padding: f32,
    atlas: Vec2,
}

impl SdfFont {
    pub async fn load(dir: &Path) -> Option<Self> {
        let dir=dir.to_owned();let (meta,image)=crate::load_work::spawn(move||{
            let meta:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(dir.join("font.json")).ok()?).ok()?;
            let image=Image::from_file_with_format(&std::fs::read(dir.join("atlas.png")).ok()?,Some(ImageFormat::Png)).ok()?;
            Some((meta,image))
        }).await?;
        let tex = crate::render3d::upload_cooperative(&image,false).await;
        tex.set_filter(FilterMode::Linear);
        let mut glyphs = HashMap::new();
        for (k, g) in meta["glyphs"].as_object()? {
            let Some(c) = k.parse::<u32>().ok().and_then(char::from_u32) else { continue };
            let f = |i: usize| g[i].as_f64().unwrap_or(0.0) as f32;
            glyphs.insert(c, Glyph { src: Rect::new(f(0), f(1), f(2), f(3)), bearing: vec2(f(4), f(5)), advance: f(6) });
        }
        let f = |k: &str| meta[k].as_f64().unwrap_or(0.0) as f32;
        Some(Self {
            identity:{static NEXT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(1);NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed)},
            tex,
            glyphs,
            point: f("point_size"),
            ascent: f("ascent"),
            descent: f("descent"),
            line_height: f("line_height"),
            padding: f("padding"),
            atlas: vec2(meta["atlas"][0].as_f64()? as f32, meta["atlas"][1].as_f64()? as f32),
        })
    }

    /// Width of one line in font units (multiply by size / point).
    pub fn line_width(&self, text: &str, spacing: f32) -> f32 {
        text.chars().filter_map(|c| self.glyphs.get(&c)).map(|g| g.advance + spacing).sum()
    }
}

pub struct Fonts {
    pub fonts: HashMap<String, SdfFont>,
    mat: Material,
}

/// Text layout request in screen pixels (y down).
pub struct TextBox<'a> {
    pub text: &'a str,
    pub rect: Rect,
    pub size: f32,
    pub color: Color,
    pub h_align: i64,
    pub v_align: i64,
    pub style: &'a TmpStyle,
    pub bold: bool,
    pub spacing: f32,
    /// TMP vertex gradient corners (TL, TR, BL, BR), each multiplied by `color` per glyph quad.
    pub gradient: Option<[Color; 4]>,
}

impl Fonts {
    pub async fn load(root: &Path, names: &[(&str, &str)]) -> Self {
        let mut fonts = HashMap::new();
        for (name, dir) in names {
            if let Some(f) = SdfFont::load(&root.join(dir)).await {
                fonts.insert(name.to_string(), f);
            }
        }
        let mat = load_material(
            ShaderSource::Glsl { vertex: SDF_VS, fragment: SDF_FS },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::OneMinusValue(BlendValue::SourceAlpha))),
                    ..Default::default()
                },
                uniforms: vec![
                    UniformDesc::new("P0", UniformType::Float4),
                    UniformDesc::new("FaceColor", UniformType::Float4),
                    UniformDesc::new("OutlineColor", UniformType::Float4),
                    UniformDesc::new("UnderlayColor", UniformType::Float4),
                    UniformDesc::new("U", UniformType::Float4),
                ],
                ..Default::default()
            },
        )
        .expect("tmp sdf shader");
        Self { fonts, mat }
    }

    pub fn get(&self, name: &str) -> Option<&SdfFont> {
        // Unresolved external references must not choose a random face each launch.
        self.fonts.get(name)
            .or_else(||self.fonts.get("Btd6FontTitleSDF"))
            .or_else(||self.fonts.get("Btd6FontBodySDF"))
            .or_else(||self.fonts.iter().min_by_key(|(name,_)|*name).map(|(_,font)|font))
    }

    pub(crate) fn layout_identity(&self,name:&str)->(u64,[u32;4]){
        self.get(name).map_or((0,[0;4]),|f|(f.identity,[f.point.to_bits(),f.ascent.to_bits(),f.descent.to_bits(),f.line_height.to_bits()]))
    }

    /// Lines laid out like TMP (no word wrap): returns the size actually used.
    pub fn line_size(&self, font: &str, text: &str, size: f32, spacing: f32) -> Vec2 {
        let Some(f) = self.get(font) else { return Vec2::ZERO };
        let k = size / f.point;
        let lines: Vec<&str> = text.trim_end_matches('\n').split('\n').collect();
        let w = lines.iter().map(|l| f.line_width(l, spacing)).fold(0.0, f32::max) * k;
        vec2(w, (f.ascent - f.descent + (lines.len() as f32 - 1.0) * f.line_height) * k)
    }

    /// Greedy word wrap of each line to `width` (same units as `size`).
    pub fn wrap(&self, font: &str, text: &str, size: f32, spacing: f32, width: f32) -> String {
        let mut out = String::new();
        for (i, para) in text.trim_end_matches('\n').split('\n').enumerate() {
            if i > 0 {
                out.push('\n');
            }
            let mut line = String::new();
            for word in para.split(' ') {
                let cand = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
                if !line.is_empty() && self.line_size(font, &cand, size, spacing).x > width {
                    out.push_str(&line);
                    out.push('\n');
                    line = word.to_string();
                } else {
                    line = cand;
                }
            }
            out.push_str(&line);
        }
        out
    }

    pub fn draw(&self, font: &str, b: &TextBox) {
        let Some(f) = self.get(font) else { return };
        let k = b.size / f.point;
        let dpi = screen_dpi_scale();
        let s = b.style;
        // TMP_SDF vertex: scale = screen px per SDF unit; bias/weight from dilate and bold.
        let scale = k * dpi * s.gradient_scale;
        let weight = ((if b.bold { s.weight_bold } else { s.weight_normal }) / 4.0 + s.face_dilate) * s.ratio_a * 0.5;
        let bias = (0.5 - weight) + 0.5 / scale;
        let outline = s.outline_width * s.ratio_a * scale;
        let softness = s.outline_softness * s.ratio_a * scale;
        let (ucol, uoff) = match s.underlay {
            Some((c, off, dilate, soft)) => {
                let bscale = scale / (1.0 + soft * s.ratio_c * scale);
                let bbias = (0.5 - weight) * bscale - 0.5 - dilate * s.ratio_c * 0.5 * bscale;
                // Unity UVs are v-up; the exported atlas is v-down.
                let du = -(off.x * s.ratio_c) * s.gradient_scale / f.atlas.x;
                let dv = (off.y * s.ratio_c) * s.gradient_scale / f.atlas.y;
                (c, vec4(du, dv, bscale, bbias))
            }
            None => (Color::new(0.0, 0.0, 0.0, 0.0), vec4(0.0, 0.0, 1.0, 0.0)),
        };
        self.mat.set_uniform("P0", vec4(scale, bias, outline, softness));
        self.mat.set_uniform("FaceColor", vec4(s.face.r, s.face.g, s.face.b, s.face.a));
        self.mat.set_uniform("OutlineColor", vec4(s.outline.r, s.outline.g, s.outline.b, s.outline.a));
        self.mat.set_uniform("UnderlayColor", vec4(ucol.r, ucol.g, ucol.b, ucol.a));
        self.mat.set_uniform("U", uoff);
        gl_use_material(&self.mat);
        let lines: Vec<&str> = b.text.trim_end_matches('\n').split('\n').collect();
        let block_h = (f.ascent - f.descent + (lines.len() as f32 - 1.0) * f.line_height) * k;
        // TMP vertical alignment: Top 256, Middle 512, Bottom 1024 (others treated as Middle).
        let top = match b.v_align {
            256 => b.rect.y,
            1024 => b.rect.bottom() - block_h,
            _ => b.rect.y + (b.rect.h - block_h) / 2.0,
        };
        let p = f.padding;
        for (i, line) in lines.iter().enumerate() {
            let w = f.line_width(line, b.spacing) * k;
            let mut pen = match b.h_align {
                1 | 8 | 16 => b.rect.x,
                4 => b.rect.right() - w,
                _ => b.rect.x + (b.rect.w - w) / 2.0,
            };
            let baseline = top + (f.ascent + i as f32 * f.line_height) * k;
            for c in line.chars() {
                let Some(g) = f.glyphs.get(&c) else { continue };
                if g.src.w > 1.0 {
                    let src = Rect::new(g.src.x - p, g.src.y - p, g.src.w + 2.0 * p, g.src.h + 2.0 * p);
                    let (x, y) = (pen + (g.bearing.x - p) * k, baseline - (g.bearing.y + p) * k);
                    if let Some(corners) = b.gradient {
                        let (tw, th) = (f.tex.width(), f.tex.height());
                        let mul = |c: Color| Color::new(c.r * b.color.r, c.g * b.color.g, c.b * b.color.b, c.a * b.color.a);
                        let (w, h) = (src.w * k, src.h * k);
                        let (u0, v0, u1, v1) = (src.x / tw, src.y / th, (src.x + src.w) / tw, (src.y + src.h) / th);
                        draw_mesh(&Mesh {
                            vertices: vec![
                                Vertex::new(x, y, 0.0, u0, v0, mul(corners[0])),
                                Vertex::new(x + w, y, 0.0, u1, v0, mul(corners[1])),
                                Vertex::new(x, y + h, 0.0, u0, v1, mul(corners[2])),
                                Vertex::new(x + w, y + h, 0.0, u1, v1, mul(corners[3])),
                            ],
                            indices: vec![0, 1, 2, 1, 3, 2],
                            texture: Some(f.tex.clone()),
                        });
                    } else {
                        draw_texture_ex(&f.tex, x, y, b.color, DrawTextureParams {
                            dest_size: Some(vec2(src.w * k, src.h * k)),
                            source: Some(src),
                            ..Default::default()
                        });
                    }
                }
                pen += (g.advance + b.spacing) * k;
            }
        }
        gl_use_default_material();
    }
}

pub struct Sprite {
    pub tex: std::rc::Rc<Texture2D>,
    /// left, bottom, right, top (pixels).
    pub border: [f32; 4],
    pub ppu: f32,
}

/// Sprites by name, loaded on first use from the exported atlases and extra texture folders.
pub struct Sprites {
    index: HashMap<String, (PathBuf, [f32; 4], f32)>,
    loaded: HashMap<String, Option<Sprite>>,
}

// UI PNGs always use the same linear, non-mipmapped sampling. Reuse uploads
// across screen-local sprite catalogs and aliases. Weak entries preserve the
// screens' existing lifetimes rather than retaining every visited store item.
#[derive(Default)]
struct UiTextures{
    paths:HashMap<PathBuf,blake3::Hash>,
    textures:HashMap<blake3::Hash,std::rc::Weak<Texture2D>>,
}
thread_local!{static UI_TEXTURES:std::cell::RefCell<UiTextures>=Default::default();}
fn shared_ui_texture(path:&Path)->Option<std::rc::Rc<Texture2D>>{
    shared_ui_texture_with(path,None)
}

pub(crate) fn shared_decoded_ui_texture(path:&Path,img:&Image)->Option<std::rc::Rc<Texture2D>>{
    shared_ui_texture_with(path,Some(img))
}

fn shared_ui_texture_with(path:&Path,decoded:Option<&Image>)->Option<std::rc::Rc<Texture2D>>{
    UI_TEXTURES.with(|cache|{
        let mut cache=cache.borrow_mut();
        if let Some(texture)=cache.paths.get(path).and_then(|key|cache.textures.get(key)).and_then(std::rc::Weak::upgrade){return Some(texture);}
        let bytes=std::fs::read(path).ok()?;let key=blake3::hash(&bytes);
        let texture=if let Some(texture)=cache.textures.get(&key).and_then(std::rc::Weak::upgrade){texture}else{
            let texture=std::rc::Rc::new(match decoded{Some(img)=>Texture2D::from_image(img),None=>Texture2D::from_file_with_format(&bytes,Some(ImageFormat::Png))});
            texture.set_filter(FilterMode::Linear);cache.textures.insert(key,std::rc::Rc::downgrade(&texture));texture
        };
        cache.paths.insert(path.to_owned(),key);Some(texture)
    })
}

/// Opt-in renderer smoke check: exercises actual GPU uploads after context creation.
pub fn verify_shared_ui_textures(root:&Path){
    let mut a=Sprites::new(root,&[]);let mut b=Sprites::new(root,&[]);
    let first=a.get("CoinIcon").expect("CoinIcon exported").tex.clone();
    let second=b.get("CoinIcon").expect("CoinIcon exported").tex.clone();
    let path=a.index.get("CoinIcon").unwrap().0.clone();
    let decoded=Image::from_file_with_format(&std::fs::read(&path).unwrap(),Some(ImageFormat::Png)).unwrap();
    assert!(std::rc::Rc::ptr_eq(&first,&shared_decoded_ui_texture(&path,&decoded).unwrap()),"decoded and lazy UI paths must share the upload");
    assert!(std::rc::Rc::ptr_eq(&first,&second),"screen catalogs must share one upload");
    let alias=b.index.iter().find(|(name,(path,_,_))|name.as_str()!="CoinIcon"&&a.index.get("CoinIcon").is_some_and(|(p,_,_)|p==path)).map(|(name,_)|name.clone()).expect("namespaced sprite alias");
    assert!(std::rc::Rc::ptr_eq(&first,&b.get(&alias).unwrap().tex),"aliases must share one upload");
    eprintln!("shared assets: UI catalog and alias upload checks passed");
}

impl Sprites {
    /// Indexes `<root>/sprites/<Atlas>/*.png` (with `_borders.json`) and each extra dir's PNGs.
    fn build_index(root: &Path, extra: &[PathBuf]) -> HashMap<String,(PathBuf,[f32;4],f32)> {
        let mut index = HashMap::new();
        for atlas in std::fs::read_dir(root.join("sprites")).into_iter().flatten().flatten() {
            let dir = atlas.path();
            let meta: HashMap<String, Vec<f32>> = std::fs::read_to_string(dir.join("_borders.json"))
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            for f in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let name = f.file_name().to_string_lossy().to_string();
                if let Some(stem) = name.strip_suffix(".png") {
                    let m = meta.get(stem);
                    let b = m.map(|v| [v[0], v[1], v[2], v[3]]).unwrap_or([0.0; 4]);
                    let ppu = m.and_then(|v| v.get(4).copied()).unwrap_or(10.8);
                    index.entry(stem.to_string()).or_insert((f.path(), b, ppu));
                    // Also "Atlas/Name": atlases share sprite names (Bloons/Zebra, UiEmoteIcons/Zebra).
                    index.insert(format!("{}/{stem}", atlas.file_name().to_string_lossy()), (f.path(), b, ppu));
                }
            }
        }
        for dir in extra {
            for f in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let name = f.file_name().to_string_lossy().to_string();
                if let Some(stem) = name.strip_suffix(".png") {
                    index.insert(stem.to_string(), (f.path(), [0.0; 4], 10.8));
                }
            }
        }
        index
    }

    pub fn new(root:&Path,extra:&[PathBuf])->Self{Self{index:Self::build_index(root,extra),loaded:HashMap::new()}}
    pub async fn load(root:&Path,extra:&[PathBuf])->Self{
        let root=root.to_owned();let extra=extra.to_vec();
        let index=crate::load_work::spawn(move||Self::build_index(&root,&extra)).await;
        Self{index,loaded:HashMap::new()}
    }

    /// A sprite of this name exists.
    pub fn has(&self, name: &str) -> bool {
        self.index.contains_key(name)
    }

    pub fn get(&mut self, name: &str) -> Option<&Sprite> {
        if !self.loaded.contains_key(name) {
            let s = self.index.get(name).and_then(|(path, border, ppu)| {
                let tex = shared_ui_texture(path)?;
                Some(Sprite { tex, border: *border, ppu: *ppu })
            });
            self.loaded.insert(name.to_string(), s);
        }
        self.loaded[name].as_ref()
    }
}

/// Draws `t` stretched into `r` (screen px), optionally sliced with borders given in screen px,
/// mirrored per axis.
pub fn draw_sliced(t: &Texture2D, r: Rect, border_px: [f32; 4], border_dst: [f32; 4], fill_center: bool, tint: Color, flip: [bool; 2]) {
    let (tw, th) = (t.width(), t.height());
    let [l, b, rr, top] = border_px;
    let [dl, db, dr, dt] = border_dst;
    let fit = |a: f32, c: f32, total: f32| {
        let k = if a + c > total && a + c > 0.0 { total / (a + c) } else { 1.0 };
        (a * k, c * k)
    };
    let (cl, cr) = fit(dl, dr, r.w);
    let (ct, cb) = fit(dt, db, r.h);
    let cols = [(0.0, l, cl), (l, tw - l - rr, r.w - cl - cr), (tw - rr, rr, cr)];
    let rows = [(0.0, top, ct), (top, th - top - b, r.h - ct - cb), (th - b, b, cb)];
    let mut y = r.y;
    for (j, &(sy, sh, dh)) in rows.iter().enumerate() {
        let mut x = r.x;
        for (i, &(sx, sw, dw)) in cols.iter().enumerate() {
            if sw > 0.0 && sh > 0.0 && dw > 0.0 && dh > 0.0 && (fill_center || i != 1 || j != 1) {
                let dx = if flip[0] { 2.0 * r.x + r.w - x - dw } else { x };
                let dy = if flip[1] { 2.0 * r.y + r.h - y - dh } else { y };
                draw_texture_ex(t, dx, dy, tint, DrawTextureParams {
                    dest_size: Some(vec2(dw, dh)),
                    source: Some(Rect::new(sx, sy, sw, sh)),
                    flip_x: flip[0],
                    flip_y: flip[1],
                    ..Default::default()
                });
            }
            x += dw;
        }
        y += dh;
    }
}

/// Upgrade icon sprite name for an upgrade ("Crossbow Master" -> "CrossBowMasterUpgradeIcon"),
/// matched case-insensitively against the exported icons.
pub fn icon_key(s: &str) -> String {
    s.trim_end_matches("UpgradeIcon").chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase()
}

/// The icon for upgrade `name` (tier `tier`, 0-based, on `path`) of tower `tower`. Some upgrades
/// carry the tower's name ("Alchemist Faster Throwing") while their icon doesn't
/// (FasterThrowingUpgradeIcon); the Desperado's icons are named by tier code, exported as
/// Desperado100UpgradeIcon.
/// Upgrades whose icon file has another name (INFERRED from each tower's UpgradeIcons folder: the
/// one icon left over; UpgradeModel.icon is a sprite GUID the catalogs don't resolve).
const ICON_ALIASES: &[(&str, &str)] = &[
    ("Large Calibre", "LongCalibre"),
    ("Supply Drop", "CashDrop"),
    ("Spectre", "Specter"),
    ("Centered Path", "FlightOrders"),
    ("Hydra Rocket Pods", "HydraRockets"),
    ("Monkeyopolis", "Metropolis"),
];

/// UpgradeModel.icon sprite by upgrade name (ui/runtime/upgrade_icons.json), set when the HUD loads.
pub static UPGRADE_ICONS: std::sync::OnceLock<std::collections::HashMap<String, String>> = std::sync::OnceLock::new();

pub fn find_icon(icons: &std::collections::HashMap<String, String>, name: &str, tower: &str, path: usize, tier: usize) -> Option<String> {
    // The upgrade's own icon reference first; the name matching below only covers what wasn't exported.
    if let Some(icon) = UPGRADE_ICONS.get().and_then(|m| m.get(name)) {
        return Some(icon.clone());
    }
    // The tower's own copy of a name several towers share ("TackShooter__FasterShooting...").
    let folder = match tower {
        "DartlingGunner" => "DartlingMonkey",
        "Druid" => "DruidMonkey",
        "Skywarden" => "SkyWarden",
        t => t,
    };
    // ...also by the name without its tower prefix ("Buccaneer-Faster Shooting",
    // "Mortar Faster Reload").
    let bare = name.split_once('-').map(|x| x.1).or_else(|| {
        name.split_once(' ').filter(|(w, _)| w.len() > 3 && tower.to_lowercase().starts_with(&w.to_lowercase())).map(|x| x.1)
    });
    for n in std::iter::once(name).chain(bare) {
        if let Some(i) = icons.get(&icon_key(&format!("{folder}{n}"))) {
            return Some(i.clone());
        }
    }
    if let Some((_, a)) = ICON_ALIASES.iter().find(|(n, _)| *n == name) {
        if let Some(i) = icons.get(&icon_key(a)) {
            return Some(i.clone());
        }
    }
    let mut code = [b'0'; 3];
    if path < 3 {
        code[path] = b'1' + tier.min(8) as u8;
    }
    icons
        .get(&icon_key(name))
        .or_else(|| name.split_once(' ').and_then(|(_, rest)| icons.get(&icon_key(rest))))
        // "Buccaneer-Faster Shooting": FasterShootingUpgradeIcon.
        .or_else(|| name.split_once('-').and_then(|(_, rest)| icons.get(&icon_key(rest))))
        .or_else(|| icons.get(&icon_key(&format!("{tower}{}", String::from_utf8_lossy(&code)))))
        // "DartMonkey Paragon": DartMonkeyParagonIcon.
        .or_else(|| icons.get(&format!("{}icon", icon_key(name))))
        // The game's own misspellings ("GuildedMagic" for Guided Magic): one letter off.
        .or_else(|| {
            let k = icon_key(name);
            (k.len() > 6).then(|| icons.iter().find(|(n, _)| one_edit(n, &k)).map(|(_, v)| v)).flatten()
        })
        // Shortened icon names ("XXXL Trap": XXXLUpgradeIcon): the longest icon name the
        // upgrade name starts with.
        .or_else(|| {
            let k = icon_key(name);
            icons.iter().filter(|(n, _)| n.len() >= 4 && k.starts_with(n.as_str())).max_by_key(|(n, _)| n.len()).map(|(_, v)| v)
        })
        .cloned()
}

/// `a` and `b` differ by at most one inserted, deleted or changed character.
fn one_edit(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (s, l) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    if l.len() - s.len() > 1 {
        return false;
    }
    let p = s.iter().zip(l).take_while(|(x, y)| x == y).count();
    if s.len() == l.len() {
        s[p..].iter().skip(1).eq(l[p..].iter().skip(1))
    } else {
        s[p..] == l[p + 1..]
    }
}
