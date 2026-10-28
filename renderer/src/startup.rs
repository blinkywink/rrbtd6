//! The original startup sequence: the Ninja Kiwi splash (NinjaKiwiSplashUI), the initial loading
//! screen (InitialLoadingUI) and the title screen (TitleScreenUI): a perspective world of 2D
//! sprites behind a canvas with Start, the BTD6 logo included (from the player's own game files).
//! Advanced > Skip Startup Screen goes straight to the main menu.
use crate::frontend::{apply_animation, curve_value};
use crate::hud::Hud;
use crate::uitree::{draw_tree, Ctx, Node, R, Rt};
use macroquad::prelude::*;
use serde_json::Value;
use std::path::Path;

const SPLASH: &str = "NinjaKiwiSplashOpen";
const TITLE_UI: &str = "Canvas/ScreenBoxer/TitleScreen";
/// TitleScreen.SkipTitleAnimClicked: Animator.Play(state, 0, normalizedTime).
const SKIP_UI: f32 = 0.83;
const SKIP_BG: f32 = 0.63;
/// TitleScreen.TransitionToMainMenu: AudioFactory.FadeMusic(2).
pub const MUSIC_FADE: f64 = 2.0;

/// Only assets used by the title world and its own controls, including those
/// revealed partway through the opening. Other menu pages remain lazy.
pub(crate) fn title_sprites(root: &Path) -> Vec<String> {
    fn visit(node: &Value, path: &str, names: &mut Vec<String>) {
        let path = if path.is_empty() { node["name"].as_str().unwrap_or("").to_owned() }
            else { format!("{path}/{}", node["name"].as_str().unwrap_or("")) };
        for component in node["comps"].as_array().into_iter().flatten() {
            let world = path.starts_with("TitleScreenBg") && component["type"] == "SpriteRenderer";
            let ui = path.starts_with(TITLE_UI) && component["type"] == "Image";
            if world || ui {
                if let Some(name) = component["sprite"].as_str() { names.push(name.to_owned()); }
            }
        }
        for child in node["children"].as_array().into_iter().flatten() { visit(child, &path, names); }
    }
    let data = std::fs::read_to_string(root.join("ui/prefabs/TitleScreenUI.json")).ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok()).unwrap_or_default();
    let mut names = Vec::new();
    for child in data["children"].as_array().into_iter().flatten() { visit(child, "", &mut names); }
    names.sort(); names.dedup(); names
}

struct Scene {
    node: Node,
    runtime: Value,
}

impl Scene {
    fn load(root: &Path, name: &str, keep: Option<&str>) -> Option<Self> {
        let json: Value = serde_json::from_str(&std::fs::read_to_string(root.join(format!("ui/prefabs/{name}.json"))).ok()?).ok()?;
        let mut node = Node::from_json(&json);
        // Only the canvas is a UI tree; world roots (camera, sprites) are drawn separately.
        if let Some(keep) = keep {
            node.children.retain(|c| c.name == keep);
        }
        for c in &mut node.children {
            c.rt = Rt::default();
        }
        node.hide_named(&["Jewel", "JewelIcon"]);
        let runtime = serde_json::from_str(&std::fs::read_to_string(root.join(format!("ui/runtime/{name}.json"))).ok()?).ok()?;
        Some(Self { node, runtime })
    }
    fn clip_length(&self, controller: &str, clip: &str) -> f32 {
        self.runtime["animators"].as_array().into_iter().flatten().filter(|a| a["controller"] == controller)
            .flat_map(|a| a["clips"].as_array().into_iter().flatten()).find(|c| c["name"] == clip)
            .map(|c| (c["stop"].as_f64().unwrap_or(0.0) - c["start"].as_f64().unwrap_or(0.0)) as f32).unwrap_or(0.0)
    }
    /// Plays `clip` at `t`; looping clips wrap.
    fn play(&mut self, controller: &str, clip: &str, t: f32, looped: bool) -> bool {
        let len = self.clip_length(controller, clip).max(0.001);
        let t = if looped { t.rem_euclid(len) } else { t };
        apply_animation(&mut self.node, &self.runtime, controller, clip, Some(t))
    }
    fn draw(&mut self, hud: &mut Hud) {
        let scale = Hud::canvas_scale();
        let canvas = vec2(screen_width() / scale, screen_height() / scale);
        let mut ctx = Ctx { sprites: &mut hud.sprites, fonts: &hud.fonts, scale, canvas_h: canvas.y, clip: None };
        draw_tree(&mut self.node, R { min: Vec2::ZERO, size: canvas }, &mut ctx);
    }
}

// ---- The title screen's sprite world -----------------------------------------------------------

struct Sprite {
    name: String,
    color: [f32; 4],
    layer: i64,
    order: i64,
    pivot: Vec2,
    /// World units (sprite pixels / pixels per unit).
    size: Vec2,
}

/// CustomMoveAnimator: localPosition = home + lerp(startOffset, endOffset, curve(t)).
struct Mover {
    keys: Vec<[f32; 4]>,
    duration: f32,
    speed: f32,
    looped: bool,
    start: Vec3,
    end: Vec3,
    home: Option<Vec3>,
    elapsed: f32,
    playing: bool,
}

struct World {
    name: String,
    active: bool,
    pos: Vec3,
    rot: Quat,
    scale: Vec3,
    sprite: Option<Sprite>,
    mover: Option<Mover>,
    children: Vec<World>,
}

fn v3(v: &Value, d: f32) -> Vec3 {
    let f = |i: usize| v[i].as_f64().map(|x| x as f32).unwrap_or(d);
    vec3(f(0), f(1), f(2))
}

impl World {
    fn from_json(v: &Value) -> Self {
        let t = &v["transform"];
        let rot = t["rot"].as_array().filter(|a| a.len() == 4)
            .map(|a| Quat::from_xyzw(a[0].as_f64().unwrap_or(0.0) as f32, a[1].as_f64().unwrap_or(0.0) as f32,
                a[2].as_f64().unwrap_or(0.0) as f32, a[3].as_f64().unwrap_or(1.0) as f32)).unwrap_or(Quat::IDENTITY);
        let comps = v["comps"].as_array().cloned().unwrap_or_default();
        let sprite = comps.iter().find(|c| c["type"] == "SpriteRenderer" && c["enabled"] != false).and_then(|c| {
            let ppu = c["ppu"].as_f64()? as f32;
            Some(Sprite {
                name: c["sprite"].as_str()?.to_owned(),
                color: [0, 1, 2, 3].map(|i| c["color"][i].as_f64().unwrap_or(1.0) as f32),
                layer: c["layer"].as_i64().unwrap_or(0),
                order: c["order"].as_i64().unwrap_or(0),
                pivot: vec2(c["pivot"][0].as_f64().unwrap_or(0.5) as f32, c["pivot"][1].as_f64().unwrap_or(0.5) as f32),
                size: vec2(c["rect"][0].as_f64()? as f32, c["rect"][1].as_f64()? as f32) / ppu,
            })
        });
        let mover = comps.iter().find(|c| c["type"] == "CustomMoveAnimator" && c["enabled"] != false).map(|c| Mover {
            keys: c["curve"]["keys"].as_array().into_iter().flatten()
                .map(|k| [0, 1, 2, 3].map(|i| k[i].as_f64().unwrap_or(0.0) as f32)).collect(),
            duration: c["duration"].as_f64().unwrap_or(1.0).max(1e-4) as f32,
            speed: c["speedMultiplier"].as_f64().unwrap_or(1.0) as f32,
            looped: c["loop"] != 0,
            start: v3(&c["startOffset"], 0.0),
            end: v3(&c["endOffset"], 0.0),
            home: None,
            elapsed: 0.0,
            playing: c["isPlaying"] != 0,
        });
        Self {
            name: v["name"].as_str().unwrap_or("").to_owned(),
            active: v["active"] != false,
            pos: v3(&t["pos"], 0.0),
            rot,
            scale: v3(&t["scale"], 1.0),
            sprite,
            mover,
            children: v["children"].as_array().into_iter().flatten().map(World::from_json).collect(),
        }
    }

    fn find_mut(&mut self, path: &str) -> Option<&mut World> {
        let mut n = self;
        for part in path.split('/').filter(|p| !p.is_empty()) {
            n = n.children.iter_mut().find(|c| c.name == part)?;
        }
        Some(n)
    }

    fn actor_matrix(&self, path: &str) -> Option<Mat4> {
        if !self.active { return None; }
        let mut node = self;
        let mut matrix = Mat4::from_scale_rotation_translation(node.scale, node.rot, node.pos);
        for name in path.split('/') {
            node = node.children.iter().find(|c| c.name == name && c.active)?;
            matrix *= Mat4::from_scale_rotation_translation(node.scale, node.rot, node.pos);
        }
        Some(matrix)
    }

    /// CustomAnimationComponent.Update: elapsed += dt / duration * speed; looping wraps (Repeat).
    fn tick(&mut self, dt: f32, parent_active: bool) {
        let active = parent_active && self.active;
        if let Some(m) = &mut self.mover {
            if !active {
                // resetOnEnable: starts over when shown again.
                m.elapsed = 0.0;
            } else if m.playing {
                let home = *m.home.get_or_insert(self.pos);
                m.elapsed += dt / m.duration * m.speed;
                if m.looped || m.elapsed <= 1.0 {
                    let t = if m.looped { m.elapsed.rem_euclid(1.0) } else { m.elapsed };
                    let k = hermite(&m.keys, t).clamp(0.0, 1.0);
                    self.pos = home + m.start + (m.end - m.start) * k;
                } else {
                    self.pos = home + m.end;
                }
            }
        }
        for c in &mut self.children {
            c.tick(dt, active);
        }
    }

    fn collect<'a>(&'a self, parent: Mat4, out: &mut Vec<(&'a Sprite, Mat4)>) {
        if !self.active {
            return;
        }
        let m = parent * Mat4::from_scale_rotation_translation(self.scale, self.rot, self.pos);
        if let Some(s) = &self.sprite {
            out.push((s, m));
        }
        for c in &self.children {
            c.collect(m, out);
        }
    }
}

/// Unity's AnimationCurve between keys (cubic Hermite); clamped outside.
fn hermite(keys: &[[f32; 4]], t: f32) -> f32 {
    let (Some(first), Some(last)) = (keys.first(), keys.last()) else { return 0.0 };
    if t <= first[0] { return first[1]; }
    if t >= last[0] { return last[1]; }
    let i = keys.windows(2).position(|w| t >= w[0][0] && t <= w[1][0]).unwrap_or(0);
    let (a, b) = (keys[i], keys[i + 1]);
    let dt = (b[0] - a[0]).max(1e-6);
    let s = (t - a[0]) / dt;
    let (s2, s3) = (s * s, s * s * s);
    (2.0 * s3 - 3.0 * s2 + 1.0) * a[1] + (s3 - 2.0 * s2 + s) * a[3] * dt + (-2.0 * s3 + 3.0 * s2) * b[1] + (s3 - s2) * b[2] * dt
}

/// Unity Transform.localEulerAngles (degrees): Z, then X, then Y.
fn euler(v: Vec3) -> Quat {
    let r = v * (std::f32::consts::PI / 180.0);
    Quat::from_rotation_y(r.y) * Quat::from_rotation_x(r.x) * Quat::from_rotation_z(r.z)
}

/// Applies one clip of the background Animator to the world at `t` (Transform position, euler
/// rotation and scale, GameObject active, SpriteRenderer alpha).
fn animate_world(world: &mut World, data: &Value, clip: &str, t: f32) -> bool {
    let Some(state) = data["animators"].as_array().into_iter().flatten().filter(|a| a["controller"] == "TitleScreenBg")
        .flat_map(|a| a["clips"].as_array().into_iter().flatten()).find(|c| c["name"] == clip) else { return true };
    let stop = state["stop"].as_f64().unwrap_or(0.0) as f32;
    let t = t.min(stop);
    for b in state["bindings"].as_array().into_iter().flatten() {
        let Some(path) = b["path"].as_str() else { continue };
        let Some(n) = world.find_mut(path.strip_prefix("TitleScreenBg").unwrap_or(path)) else { continue };
        let channel = |i: usize| curve_value(&b["curves"][i], t)
            .or_else(|| b["end"][i].as_f64().map(|v| v as f32));
        match (b["type"].as_i64(), b["attribute"].as_i64(), b["property"].as_str()) {
            (Some(4), Some(1), _) => n.pos = vec3(channel(0).unwrap_or(n.pos.x), channel(1).unwrap_or(n.pos.y), channel(2).unwrap_or(n.pos.z)),
            (Some(4), Some(3), _) => n.scale = vec3(channel(0).unwrap_or(n.scale.x), channel(1).unwrap_or(n.scale.y), channel(2).unwrap_or(n.scale.z)),
            (Some(4), Some(4), _) => n.rot = euler(vec3(channel(0).unwrap_or(0.0), channel(1).unwrap_or(0.0), channel(2).unwrap_or(0.0))),
            (Some(1), _, Some("m_IsActive")) => if let Some(v) = channel(0) { n.active = v > 0.5 },
            (_, _, Some("m_Color.a")) => if let (Some(v), Some(s)) = (channel(0), n.sprite.as_mut()) { s.color[3] = v },
            _ => {}
        }
    }
    t >= stop
}

pub(crate) struct Title {
    ui: Scene,
    world: World,
    /// The camera's vertical field of view (degrees).
    fov: f32,
    began: f64,
    /// Seconds into the open animations (skippable).
    skip: Option<(f64, f32, f32)>,
    closing: Option<f64>,
    last: f64,
    plane: Option<(crate::anim::AnimModel, crate::anim::Shaders)>,
    trace_frames: bool,
    root: std::path::PathBuf,

}

impl Title {
    fn load(root: &Path, version: &str, player: Option<&str>, plane: Option<crate::anim::AnimModel>) -> Option<Self> {
        let mut ui = Scene::load(root, "TitleScreenUI", Some("Canvas"))?;
        let json: Value = serde_json::from_str(&std::fs::read_to_string(root.join("ui/prefabs/TitleScreenUI.json")).ok()?).ok()?;
        let roots = json["children"].as_array()?;
        let world = World::from_json(roots.iter().find(|r| r["name"] == "TitleScreenBg")?);
        let fov = roots.iter().find(|r| r["name"] == "Camera")
            .and_then(|c| c["comps"].as_array()?.iter().find(|x| x["type"] == "Camera")?["fov"].as_f64()).unwrap_or(60.0) as f32;
        // TitleScreen.Start / SetPlayerInfo.
        ui.node.set_text(&format!("{TITLE_UI}/BuildInfo"), version);
        ui.node.set_text(&format!("{TITLE_UI}/PlayerId"), "");
        ui.node.set_active(&format!("{TITLE_UI}/iCloudInfo"), false);
        ui.node.set_active(&format!("{TITLE_UI}/PlayerInfo"), true);
        ui.node.set_active(&format!("{TITLE_UI}/PlayerInfo/LoginButton"), player.is_none());
        ui.node.set_active(&format!("{TITLE_UI}/PlayerInfo/PlayerNameTxt"), player.is_some());
        ui.node.set_text(&format!("{TITLE_UI}/PlayerInfo/PlayerNameTxt"), player.unwrap_or(""));
        ui.node.set_active(&format!("{TITLE_UI}/LoadingSpinner"), false);
        Some(Self { root: root.to_owned(), ui, world, fov, began: get_time(), skip: None, closing: None, last: get_time(), plane: plane.map(|p| (p, crate::anim::Shaders::new())), trace_frames: std::env::var_os("RRBTD6_STARTUP_FRAME_TRACE").is_some() })
    }

    fn draw_plane(actor: &mut Option<(crate::anim::AnimModel, crate::anim::Shaders)>, began: f64, fov: f32, matrix: Mat4) {
        let Some((plane, shaders)) = actor else { return };
        let clip = plane.idle_clip();
        let (mut meshes, mut outlines) = plane.meshes(clip, (get_time() - began) as f32, [0.0; 3], 0.0, 0.7, false, None);
        let normal_matrix = matrix.inverse().transpose();
        let flip = |p: Vec3| vec3(p.x, p.y, -p.z);
        for mesh in meshes.iter_mut().chain(outlines.iter_mut()) {
            for vertex in &mut mesh.vertices {
                vertex.position = flip(matrix.transform_point3(flip(vertex.position)));
                let normal = flip(normal_matrix.transform_vector3(flip(vertex.normal.truncate()))).normalize_or_zero();
                vertex.normal = normal.extend(vertex.normal.w * matrix.x_axis.truncate().length());
            }
        }
        set_camera(&Camera3D { position: Vec3::ZERO, target: vec3(0.0, 0.0, -1.0), up: Vec3::Y,
            fovy: fov.to_radians(), aspect: Some(screen_width() / screen_height()),
            z_near: 0.01, z_far: 2000.0, ..Default::default() });
        shaders.forward = vec3(0.0, 0.0, -1.0);
        crate::anim::draw_meshes(shaders, &[&outlines], &[&meshes]);
        set_default_camera();
    }

    fn draw_world(&mut self, hud: &mut Hud) {
        let mut sprites = Vec::new();
        self.world.collect(Mat4::IDENTITY, &mut sprites);
        // Sorting layer, then order in layer, then back to front.
        sprites.sort_by(|a, b| (a.0.layer != 0).cmp(&(b.0.layer != 0)).then(a.0.order.cmp(&b.0.order))
            .then(b.1.w_axis.z.total_cmp(&a.1.w_axis.z)));
        let (w, h) = (screen_width(), screen_height());
        let f = h * 0.5 / (self.fov.to_radians() * 0.5).tan();
        let project = |p: Vec3| vec2(w * 0.5 + p.x / p.z * f, h * 0.5 - p.y / p.z * f);
        let actor = self.world.actor_matrix("MonkeyAce");
        let mut actor_drawn = false;
        for (s, m) in sprites {
            if !actor_drawn && actor.is_some_and(|a| s.layer != 0 || s.order > 0 || (s.order == 0 && m.w_axis.z <= a.w_axis.z)) {
                if let Some(a) = actor { Self::draw_plane(&mut self.plane, self.began, self.fov, a); }
                actor_drawn = true;
            }
            let Some(t) = hud.sprites.get(&s.name) else { continue };
            let (tex, size) = ((*t.tex).clone(), s.size);
            let lo = -s.pivot * size;
            let corners = [vec2(lo.x, lo.y + size.y), vec2(lo.x + size.x, lo.y + size.y), vec2(lo.x, lo.y), vec2(lo.x + size.x, lo.y)]
                .map(|c| m.transform_point3(vec3(c.x, c.y, 0.0)));
            if corners.iter().any(|c| c.z <= 0.01) {
                continue;
            }
            let color = Color::new(s.color[0], s.color[1], s.color[2], s.color[3]);
            let p = corners.map(project);
            draw_mesh(&Mesh {
                vertices: vec![
                    Vertex::new(p[0].x, p[0].y, 0.0, 0.0, 0.0, color),
                    Vertex::new(p[1].x, p[1].y, 0.0, 1.0, 0.0, color),
                    Vertex::new(p[2].x, p[2].y, 0.0, 0.0, 1.0, color),
                    Vertex::new(p[3].x, p[3].y, 0.0, 1.0, 1.0, color),
                ],
                indices: vec![0, 1, 2, 1, 3, 2],
                texture: Some(tex),
            });
        }
        if !actor_drawn {
            if let Some(a) = actor { Self::draw_plane(&mut self.plane, self.began, self.fov, a); }
        }
    }

    /// Draws the title; returns true once Start was clicked and the close animations finished.
    fn draw(&mut self, hud: &mut Hud) -> bool {
        let now = get_time();
        let dt = (now - self.last) as f32;
        self.last = now;
        let (ui_open, bg_open) = (self.ui.clip_length("TitleScreenUi", "TitleScreenUiOpen"), self.ui.clip_length("TitleScreenBg", "TitleScreenBgOpen"));
        let since = (now - self.began) as f32;
        if self.trace_frames { eprintln!("title-frame: {since:.6} {dt:.6}"); }
        let (ui_t, bg_t) = match self.skip {
            Some((at, u, b)) => (u + (now - at) as f32, b + (now - at) as f32),
            None => (since, since),
        };
        // Background: open, then idle.
        if animate_world(&mut self.world, &self.ui.runtime, "TitleScreenBgOpen", bg_t) {
            animate_world(&mut self.world, &self.ui.runtime, "TitleScreenBgIdle", 0.0);
        }
        if let Some(c) = self.closing {
            animate_world(&mut self.world, &self.ui.runtime, "TitleScreenBgClose", (now - c) as f32);
        }
        self.world.tick(dt, true);
        set_default_camera();
        if self.closing.is_none() { clear_background(BLACK); }
        self.draw_world(hud);
        match self.closing {
            Some(c) => { self.ui.play("TitleScreenUi", "TitleScreenUiClose", (now - c) as f32, false); }
            None => { self.ui.play("TitleScreenUi", "TitleScreenUiOpen", ui_t, false); }
        }
        hud.account_ui.poll();
        crate::frontend::account_preview::ensure(&self.root);
        let player=crate::frontend::account_preview::player_name();
        let info=format!("{TITLE_UI}/PlayerInfo");
        self.ui.node.set_active(&format!("{info}/LoginButton"),player.is_none());
        self.ui.node.set_active(&format!("{info}/PlayerNameTxt"),player.is_some());
        self.ui.node.set_text(&format!("{info}/PlayerNameTxt"),player.as_deref().unwrap_or(""));
        crate::uitree::block_input(hud.account_ui.is_open());
        self.ui.draw(hud);
        crate::uitree::block_input(false);
        if hud.account_ui.is_open() {
            hud.draw_account_ui();
            return false;
        }
        if let Some(c) = self.closing {
            if let Ok(folder) = std::env::var("RRBTD6_STARTUP_TRANSITION_OUT") {
                for (name, elapsed) in [("begin", 0.0), ("middle", 0.25), ("end", 0.49)] {
                    let path = std::path::PathBuf::from(&folder).join(format!("{name}.png"));
                    if now - c >= elapsed && !path.exists() { get_screen_data().export_png(path.to_str().unwrap()); }
                }
            }
            return now - c >= self.ui.clip_length("TitleScreenBg", "TitleScreenBgClose").max(0.25) as f64;
        }
        let opening = ui_t < ui_open || bg_t < bg_open;
        let test_start = std::env::var_os("RRBTD6_AUTOTEST").is_some() && std::env::var_os("RRBTD6_STARTUP_PRESS_START").is_some();
        let test_account=std::env::var_os("RRBTD6_AUTOTEST").is_some() && std::env::var_os("RRBTD6_STARTUP_ACCOUNT").is_some();
        if !opening && (self.ui.node.clicked(&format!("{info}/LoginButton/Button")) || self.ui.node.clicked(&format!("{info}/PlayerNameTxt")) || test_account) {
            hud.account_ui.open(&self.root);
        } else if (self.ui.node.clicked(&format!("{TITLE_UI}/Start")) || test_start) && !opening {
            // TitleScreen.TransitionToMainMenu (click sound, music fades over 2 s).
            self.closing = Some(now);
            if std::env::var_os("RRBTD6_PRESENTATION_CHECK").is_some() { crate::presentation_check::arm(); }
        } else if opening && self.ui.node.clicked(&format!("{TITLE_UI}/FullScreenButton")) && self.skip.is_none() {
            self.skip = Some((now, (SKIP_UI * ui_open).max(ui_t), (SKIP_BG * bg_open).max(bg_t)));
        }
        false
    }
}

// ---- The whole sequence -------------------------------------------------------------------------

enum Stage {
    Splash(Scene, f64),
    Initial(Scene, f64, Option<f64>),
    Title(Box<Title>),
}

pub(crate) struct Startup {
    stage: Option<Stage>,
    root: std::path::PathBuf,
    version: String,
    player: Option<String>,
    progress: Option<(usize, &'static str)>,
    loaded: bool,
    plane: Option<crate::anim::AnimModel>,
}

impl Startup {
    pub(crate) fn transitioning_home(&self) -> bool {
        matches!(&self.stage, Some(Stage::Title(title)) if title.closing.is_some())
    }
    pub(crate) fn new(root: &Path, version: String, player: Option<String>) -> Self {
        let stage = Scene::load(root, "NinjaKiwiSplashUI", Some("Canvas")).map(|s| Stage::Splash(s, get_time()));
        let mut s = Self { stage, root: root.to_path_buf(), version, player, progress: None, loaded: false, plane: None };
        if s.stage.is_none() { s.next_initial(); }
        s
    }

    fn next_initial(&mut self) {
        self.stage = Scene::load(&self.root, "InitialLoadingUI", Some("Canvas")).map(|mut s| {
            // InitialLoadingScreen.Start: no main/sub text yet, version and player id, no progress bar.
            let base = "Canvas/ScreenBoxer/InitialLoadingScreen";
            s.node.set_text(&format!("{base}/MainText"), "");
            s.node.set_text(&format!("{base}/SubText"), "");
            s.node.set_text(&format!("{base}/BuildInfo"), &self.version);
            s.node.set_text(&format!("{base}/PlayerId"), "");
            s.node.set_active(&format!("{base}/ProgressBar"), false);
            Stage::Initial(s, get_time(), None)
        });
        if self.stage.is_none() { self.next_title(); }
    }

    fn next_title(&mut self) {
        self.stage = Title::load(&self.root, &self.version, self.player.as_deref(), self.plane.take()).map(|t| Stage::Title(Box::new(t)));
    }

    pub(crate) fn set_plane(&mut self, plane: Option<crate::anim::AnimModel>) { self.plane = plane; }

    /// Poll real loading work between presentations so the original plane keeps animating.
    pub(crate) async fn task<F: std::future::Future>(&mut self, hud: &mut Hud, step: usize, label: &'static str, cap: u32, work: F) -> F::Output {
        let started = std::time::Instant::now();
        let interval = std::time::Duration::from_secs_f64(1.0 / if cap == 0 { 120 } else { cap.min(120) } as f64);
        self.progress = Some((step, label));
        // Play the splash before starting the jobs represented by the loading screen.
        while matches!(self.stage, Some(Stage::Splash(..))) {
            let began = std::time::Instant::now();
            self.draw(hud);
            let remaining = interval.saturating_sub(began.elapsed());
            if !remaining.is_zero() { std::thread::sleep(remaining); }
            next_frame().await;
        }
        let mut work = std::pin::pin!(work);
        loop {
            let began = std::time::Instant::now();
            self.draw(hud);
            if let Ok(folder) = std::env::var("RRBTD6_STARTUP_PROGRESS_OUT") {
                let path = std::path::PathBuf::from(folder).join(format!("step-{step}.png"));
                if !path.exists() { get_screen_data().export_png(path.to_str().unwrap()); }
            }
            next_frame().await;
            if let std::task::Poll::Ready(value) = work.as_mut().poll(&mut std::task::Context::from_waker(std::task::Waker::noop())) {
                if std::env::var_os("RRBTD6_TIMING").is_some() {
                    eprintln!("startup: step {step}/8 completed ({label}) in {:.1} ms", started.elapsed().as_secs_f64() * 1000.0);
                }
                if step == 8 { self.loaded = true; }
                return value;
            }
            let remaining = interval.saturating_sub(began.elapsed());
            if !remaining.is_zero() { std::thread::sleep(remaining); }
        }
    }

    /// The title music should play (at this volume factor), or None.
    pub(crate) fn title_music(&self) -> Option<f32> {
        match &self.stage {
            Some(Stage::Title(t)) => Some(t.closing.map(|c| (1.0 - (get_time() - c) / MUSIC_FADE).clamp(0.0, 1.0) as f32).unwrap_or(1.0)),
            _ => None,
        }
    }

    /// Draws the current screen; true once the player pressed Start and the title closed.
    pub(crate) fn draw(&mut self, hud: &mut Hud) -> bool {
        let now = get_time();
        set_default_camera();
        match &mut self.stage {
            None => return true,
            Some(Stage::Splash(s, began)) => {
                clear_background(BLACK);
                let t = (now - *began) as f32;
                let done = s.play("NinjaKiwiSplashAnim", SPLASH, t, false);
                s.draw(hud);
                if done { self.next_initial(); }
            }
            Some(Stage::Initial(s, began, closing)) => {
                clear_background(BLACK);
                let t = (now - *began) as f32;
                let open = s.clip_length("InitialLoadingScene", "InitialLoadingOpen");
                match *closing {
                    Some(c) => { s.play("InitialLoadingScene", "InitialLoadingClose", (now - c) as f32, false); }
                    None => if s.play("InitialLoadingScene", "InitialLoadingOpen", t, false) {
                        s.play("InitialLoadingScene", "InitialLoadingIdle", t - open, true);
                    },
                }
                s.play("LoadingCloudPuff", "LoadingCloudPuff", t, true);
                s.play("LoadingPlaneSprites", "LoadingPlaneSprites", t, true);
                s.play("LoadingPlaneSprites", "LoadingPlaneBob", t, true);
                if let Some((step, label)) = self.progress {
                    let base = "Canvas/ScreenBoxer/InitialLoadingScreen";
                    s.node.set_text(&format!("{base}/MainText"), label);
                    s.node.set_text(&format!("{base}/SubText"), &format!("Step {step} of 8..."));
                }
                s.draw(hud);
                if closing.is_none() && self.loaded && t >= open { *closing = Some(now); }
                let close = s.clip_length("InitialLoadingScene", "InitialLoadingClose");
                if closing.is_some_and(|c| now - c >= close as f64) { self.next_title(); }
            }
            Some(Stage::Title(t)) => if t.draw(hud) { self.stage = None; return true; },
        }
        false
    }
}
