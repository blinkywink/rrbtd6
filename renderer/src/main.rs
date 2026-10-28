//! Playable local prototype. Loads data and art extracted from the user's own
//! BTD6 install (local_extracted/, git-ignored) at runtime.
//!
//! Controls: click the Dart Monkey, Bomb Shooter or Tack Shooter in the shop (or press
//! 1 / 2 / 3), click the map to place.
//! Space / Go button = start next round, F = toggle 1x/3x speed, A = auto-start,
//! gear button / Esc = pause screen, Esc = cancel placement, V = toggle 3D / 2D view.

mod anim;
mod animation_storage;
mod runtime_anim_export;
mod audio;
mod data;
mod game;
mod load_work;
mod frame_bench;
mod frame_pacer;
mod presentation_check;
mod round_bench;
mod coop_game;
#[cfg(test)]
mod coop_replay_tests;
mod hud;
mod particles;
mod frontend;
mod fx;
mod advanced;
mod keybinds;
mod menus;
mod render3d;
mod startup;
mod account_ui;
mod local_profile;
mod static_map;
mod sandbox;
mod temple_popup;
mod support_popup;
mod geraldo_shop;
mod ui;
mod uitree;

use game::{FPS, Game};
use hud::Hud;
use macroquad::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// Seconds an explosion or Ring of Fire burst stays visible (stand-in effect).
const BURST_LIFE: f32 = 0.3;
/// Seconds a Sniper shot's tracer stays visible (stand-in effect).
const TRACER_LIFE: f32 = 0.12;

/// The tier-index keys of tower `t`'s model and, for a Paragon, of its top: the display level's
/// own versions when exported ("paragon3", "paragon_top3"), else the first level's.
fn model_keys(game: &Game, t: &game::Tower, idx: &HashMap<String, String>) -> (String, Option<String>) {
    let Some(lvl) = game.paragon_level(t) else { return (t.tier_key(), None) };
    let pick = |base: &str| {
        let k = format!("{base}{lvl}");
        if idx.contains_key(&k) { k } else { base.to_string() }
    };
    (pick("paragon"), t.air.is_none().then(|| pick("paragon_top")))
}

/// Circle shown while placing: TowerModel.range (map-wide towers show their small nominal range).
/// STAND-IN size of the MortarTarget marker (TargetSelectedPointModel.scale 1; the sprite's
/// world size isn't known): about the shell's spread circle.
const MARKER_SIZE: f32 = 36.0;

/// Where the set-point marker goes: the cursor while picking a point, else the selected
/// tower's set point when it is aiming there.
/// A Heli faces its target for a second after attacking, else its heading.
fn heli_yaw(t: &game::Tower, a: &game::AirUnit) -> f32 {
    std::f32::consts::FRAC_PI_2 - if t.attack_t < 1.0 { t.angle } else { a.angle }
}

fn marker_at(game: &Game, picking: Option<usize>, selected: Option<usize>, in_map: bool, mouse: [f32; 2]) -> Option<[f32; 2]> {
    if picking.is_some() {
        return in_map.then_some(mouse);
    }
    let t = selected?;
    game.uses_point(t).then_some(game.towers[t].point).flatten()
}

/// This process's private memory in MB (Windows; 0 elsewhere), for RRBTD6_TIMING logs.
pub fn private_mb() -> u64 {
    #[cfg(windows)]
    {
        #[repr(C)]
        #[derive(Default)]
        struct Counters {
            cb: u32,
            page_faults: u32,
            peak_ws: usize,
            ws: usize,
            peak_paged_pool: usize,
            paged_pool: usize,
            peak_nonpaged_pool: usize,
            nonpaged_pool: usize,
            pagefile: usize,
            peak_pagefile: usize,
            private: usize,
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetCurrentProcess() -> isize;
            fn K32GetProcessMemoryInfo(p: isize, c: *mut Counters, cb: u32) -> i32;
        }
        let mut c = Counters { cb: std::mem::size_of::<Counters>() as u32, ..Default::default() };
        // SAFETY: c is a valid PROCESS_MEMORY_COUNTERS_EX of the size passed.
        if unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) } != 0 {
            return (c.private >> 20) as u64;
        }
    }
    0
}

fn placement_radius(def: &data::TowerDef) -> f32 {
    def.range
}

/// "DartMonkey" -> "Dart Monkey".
pub fn spaced(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// Where the extracted game data lives: a `local_extracted` folder next to the exe (portable
/// copies), else in the working directory, else the source tree's (development builds).
fn root() -> PathBuf {
    let has_data = |p: &PathBuf| p.join("game").join("game.json").exists();
    let beside_exe = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("local_extracted")));
    let cwd = std::env::current_dir().ok().map(|d| d.join("local_extracted"));
    beside_exe
        .into_iter()
        .chain(cwd)
        .find(has_data)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("local_extracted"))
}

/// Opt-in benchmark file overrides never affect ordinary play.
fn benchmark_input(name:&str)->Option<PathBuf>{
    (std::env::var_os("RRBTD6_AUTOTEST").is_some()&&std::env::var_os("RRBTD6_FRAME_BENCH").is_some())
        .then(||std::env::var_os(name)).flatten().map(PathBuf::from)
}
fn settings_path(root:&std::path::Path)->PathBuf{
    benchmark_input("RRBTD6_BENCH_SETTINGS").unwrap_or_else(||root.join("settings.json"))
}
fn simulation_path(root:&std::path::Path,relative:impl AsRef<std::path::Path>)->PathBuf{
    benchmark_input("RRBTD6_BENCH_SIM_DIR").unwrap_or_else(||root.to_path_buf()).join(relative)
}
/// Volume sliders and preferences normally live in local_extracted/settings.json.
fn read_settings(root: &std::path::Path) -> serde_json::Value {
    std::fs::read_to_string(settings_path(root)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(serde_json::json!({}))
}

fn load_settings(root: &std::path::Path) -> Option<[f32; 3]> {
    let v = read_settings(root);
    let a = v["volumes"].as_array()?;
    Some([0, 1, 2].map(|i| a.get(i).and_then(|x| x.as_f64()).unwrap_or(1.0).clamp(0.0, 1.0) as f32))
}

/// Updates one key of local_extracted/settings.json, keeping the others.
fn save_setting(root: &std::path::Path, key: &str, value: serde_json::Value) {
    let mut v = read_settings(root);
    v[key] = value;
    let _ = std::fs::write(settings_path(root), v.to_string());
}

fn save_settings(root: &std::path::Path, v: [f32; 3]) {
    save_setting(root, "volumes", serde_json::json!(v));
}

/// Maps exported to local_extracted/maps/ (tools/assets/export_map.py), Monkey Meadow first.
fn map_keys(root: &std::path::Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(root.join("maps"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join("map.json").exists())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    v.sort_by_key(|k| (k != "tutorial", k.clone()));
    v
}

/// The game data with `hero` in the shop's hero slot (the default hero if unknown) on map `map`
/// (game.json's own map if it isn't exported).
fn game_data(root: &std::path::Path, json: &str, hero: Option<&str>, map: &str) -> data::GameData {
    let mut d = with_hero(json, hero);
    // The difficulty from settings.json ("mode"), Medium by default.
    d.mode = read_settings(root)["mode"].as_str().unwrap_or("Medium").to_string();
    match std::fs::read_to_string(simulation_path(root,PathBuf::from("maps").join(map).join("map.json"))).map(|s| serde_json::from_str(&s)) {
        Ok(Ok(m)) => d.map = m,
        Ok(Err(e)) => eprintln!("map {map}: map.json doesn't load ({e}); using game.json's map"),
        Err(e) => eprintln!("map {map}: no map.json ({e}); using game.json's map"),
    }
    d
}

/// The game data with `hero` in the shop's hero slot (the default hero if unknown).
fn with_hero(json: &str, hero: Option<&str>) -> data::GameData {
    let mut d: data::GameData = serde_json::from_str(json).expect("bad game.json");
    if let Some(h) = hero.and_then(|h| d.heroes.iter().find(|x| x.id == h)).cloned() {
        if let Some(slot) = d.shop.iter_mut().find(|s| s.hero) {
            *slot = h;
        }
    }
    d
}

fn win_size() -> (i32, i32) {
    std::env::var("RRBTD6_WINDOW")
        .ok()
        .and_then(|v| {
            let (w, h) = v.split_once('x')?;
            Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
        })
        .unwrap_or((1600, 900))
}

/// The window and taskbar icon: brand/icon{16,32,64}.rgba from the player's own game.
fn game_icon(root: &std::path::Path) -> Option<miniquad::conf::Icon> {
    let read = |n: u32| std::fs::read(root.join(format!("brand/icon{n}.rgba"))).ok();
    Some(miniquad::conf::Icon {
        small: read(16)?.try_into().ok()?,
        medium: read(32)?.try_into().ok()?,
        big: read(64)?.try_into().ok()?,
    })
}

fn conf() -> macroquad::conf::Conf {
    if std::env::args().nth(1).as_deref()==Some("--canonicalize-anims") {
        let Some(path)=std::env::args_os().nth(2) else {eprintln!("missing models directory");std::process::exit(2)};
        match runtime_anim_export::run(std::path::Path::new(&path)) {
            Ok(())=>std::process::exit(0),
            Err(error)=>{eprintln!("animation migration failed: {error}");std::process::exit(1)}
        }
    }
    let adv = advanced::Advanced::load(&read_settings(&root()));
    let miniquad_conf=Conf {
        window_title: WINDOW_TITLE.into(),
        // RRBTD6_WINDOW=WxH overrides the start size (any size works; the map always fits).
        window_width: win_size().0,
        window_height: win_size().1,
        high_dpi: true,
        sample_count: std::env::var("RRBTD6_MSAA").ok().and_then(|v| v.parse().ok()).unwrap_or(adv.msaa.max(1)),
        platform: miniquad::conf::Platform { window_aspect_ratio:Some((16,9)), swap_interval: Some(if adv.vsync { 1 } else { 0 }), ..Default::default() },
        // The game's own icon, taken from the player's BloonsTD6.exe at setup (brand/; nothing
        // of it is shipped). None until the assets are prepared.
        icon: game_icon(&root()),
        ..Default::default()
    };
    let mut conf:macroquad::conf::Conf=miniquad_conf.into();
    conf.draw_call_vertex_capacity=render3d::vertex_capacity();
    conf
}

const WINDOW_TITLE: &str = "RustTD6";

/// Ends a frame for the Advanced settings: "Show FPS" puts the frame rate in the window title
/// (twice a second; see DEVIATIONS.md), and the "FPS Limit" (0: none) sleeps out the rest of the
/// presentation interval, including submission and swap after this call.
fn pace(frame_t0: std::time::Instant, adv: &advanced::Advanced, pacer: &mut frame_pacer::FramePacer, legacy: bool) {
    static SHOWN: std::sync::Mutex<(bool, f64)> = std::sync::Mutex::new((false, 0.0));
    if let Ok(mut shown) = SHOWN.lock() {
        let now = get_time();
        if adv.show_fps && now - shown.1 >= 0.5 {
            miniquad::window::set_window_title(&format!("{WINDOW_TITLE} - {} FPS", get_fps()));
            *shown = (true, now);
        } else if !adv.show_fps && shown.0 {
            miniquad::window::set_window_title(WINDOW_TITLE);
            shown.0 = false;
        }
    }
    let cap = adv.fps_cap;
    if !legacy {
        pacer.wait(cap, frame_t0);
        return;
    }
    if cap == 0 {
        return;
    }
    let target = std::time::Duration::from_secs_f64(1.0 / cap as f64);
    let left = target.saturating_sub(frame_t0.elapsed());
    if left > std::time::Duration::from_millis(2) {
        std::thread::sleep(left - std::time::Duration::from_millis(1));
    }
    while frame_t0.elapsed() < target {
        std::hint::spin_loop();
    }
}

async fn tex(path: PathBuf) -> Option<Texture2D> {
    match load_texture(path.to_str().unwrap()).await {
        Ok(t) => {
            t.set_filter(FilterMode::Linear);
            Some(t)
        }
        Err(e) => {
            eprintln!("missing texture {}: {e}", path.display());
            None
        }
    }
}

/// The game's hand cursor (pressed sprite while a button is held).
/// Cursor size in screen pixels (f32 bits): the game's Normal hardware cursor is 64.
static CURSOR_SCALE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x4280_0000);

fn draw_cursor(up: &Option<Texture2D>, down: &Option<Texture2D>) {
    let (mx, my) = mouse_position();
    let held = is_mouse_button_down(MouseButton::Left) || is_mouse_button_down(MouseButton::Right);
    if let (Some(up), Some(down)) = (up, down) {
        let t = if held { down } else { up };
        // Assets.Cursor hardware: HandCursor 64x64, hotspot (21, 5); a hardware cursor is
        // physical pixels, so it's divided by the DPI scale.
        let px = f32::from_bits(CURSOR_SCALE.load(std::sync::atomic::Ordering::Relaxed)) / screen_dpi_scale();
        let k = px / t.width().max(1.0);
        draw_texture_ex(t, mx - 21.0 * k, my - 5.0 * k, WHITE, DrawTextureParams {
            dest_size: Some(vec2(t.width() * k, t.height() * k)),
            ..Default::default()
        });
    }
}

enum ProjDisplay {
    Sprite { tex: Texture2D, w: f32, h: f32, pivot: [f32; 2] },
    Mesh(render3d::Model),
    /// Every SpriteRenderer of the prefab (export_projectiles.py layers.json).
    Layers(Vec<ProjLayer>),
}

/// One sprite of a layered projectile prefab, in its sprite plane (x across, y along the flight).
struct ProjLayer {
    tex: Texture2D,
    w: f32,
    h: f32,
    pivot: [f32; 2],
    offset: [f32; 2],
    /// Degrees, plus `spin` degrees per second (CustomRotationSimple).
    angle: f32,
    spin: f32,
    color: [u8; 4],
    /// CustomScaleAnimator: (duration, [(time, scale)]) looping.
    scale_anim: Option<(f32, Vec<(f32, f32)>)>,
}

/// Draws a layered projectile at Unity (x, y, z), flying along `heading` (radians), `age` seconds
/// old. Bigger layers (glows) go first, under the smaller ones.
fn draw_layers(layers: &[ProjLayer], x: f32, y: f32, z: f32, heading: f32, age: f32) {
    for (i, l) in layers.iter().enumerate() {
        let a = heading + (l.angle + l.spin * age).to_radians();
        let k = l.scale_anim.as_ref().map_or(1.0, |(dur, keys)| {
            let t = if *dur > 0.0 { (age / dur).fract() } else { 0.0 };
            match keys.iter().position(|k| k.0 > t) {
                Some(0) => keys[0].1,
                Some(j) => {
                    let (a, b) = (keys[j - 1], keys[j]);
                    a.1 + (b.1 - a.1) * ((t - a.0) / (b.0 - a.0).max(1e-6))
                }
                None => keys.last().map_or(1.0, |k| k.1),
            }
        });
        // Sprite-plane offset: x across the flight, y along it.
        let (fx, fz) = (heading.cos(), heading.sin());
        let (rx, rz) = (fz, -fx);
        let (px, pz) = (x + rx * l.offset[0] + fx * l.offset[1], z + rz * l.offset[0] + fz * l.offset[1]);
        render3d::flat_sprite_tinted(&l.tex, px, y + i as f32 * 0.01, pz, l.w * k, l.h * k, l.pivot, [a.cos(), a.sin()], l.color);
    }
}

struct View {
    s: f32,
    ox: f32,
    oy: f32,
    zmax: f32,
    xmin: f32,
}

impl View {
    fn new(g: &Game) -> Self {
        let (lo, hi) = g.bounds;
        let frame=render3d::map_frame(screen_width(),screen_height());
        let (w, h) = (frame.w - Hud::panel_px(), frame.h);
        let s = (w / (hi[0] - lo[0])).min(h / (hi[1] - lo[1]));
        Self { s, ox: frame.x+(w - (hi[0] - lo[0]) * s) / 2.0, oy: frame.y+(h - (hi[1] - lo[1]) * s) / 2.0, zmax: hi[1], xmin: lo[0] }
    }
    fn to_screen(&self, p: [f32; 2]) -> Vec2 {
        vec2(self.ox + (p[0] - self.xmin) * self.s, self.oy + (self.zmax - p[1]) * self.s)
    }
    fn to_world(&self, v: Vec2) -> [f32; 2] {
        [(v.x - self.ox) / self.s + self.xmin, self.zmax - (v.y - self.oy) / self.s]
    }
}

async fn loading_task<F:std::future::Future>(future:F,front:&mut frontend::FrontEnd,hud:&mut Option<Hud>,audio:&mut audio::Audio,cap:u32)->F::Output{
    let mut future=Box::pin(future);
    let target=std::time::Duration::from_secs_f64(1.0/if cap==0{120}else{cap.min(120)} as f64);
    loop{
        let began=std::time::Instant::now();
        let result=future.as_mut().poll(&mut std::task::Context::from_waker(std::task::Waker::noop()));
        if began.elapsed().as_millis()>16&&std::env::var_os("RRBTD6_TIMING").is_some(){eprintln!("loading work slice: {:.1} ms {}",began.elapsed().as_secs_f64()*1000.0,std::any::type_name::<F>());}
        if let std::task::Poll::Ready(value)=result{return value;}
        if let Some(h)=hud.as_mut(){front.draw_loading(h);}
        audio.music(get_time());
        let remaining=target.saturating_sub(began.elapsed());if !remaining.is_zero(){std::thread::sleep(remaining);}
        next_frame().await;
    }
}

fn main() {
    macroquad::Window::from_config(conf(),presentation_check::wrap(game_main()));
}

async fn game_main() {
    let load_t0 = std::time::Instant::now();
    let mark = |what: &str| if std::env::var("RRBTD6_TIMING").is_ok() { eprintln!("load: {what} at {:.0} ms, {} MB", load_t0.elapsed().as_secs_f64() * 1e3, private_mb()) };
    let root = root();
    mark("json");
    let json = std::sync::Arc::new(std::fs::read_to_string(simulation_path(&root,"game/game.json"))
        .expect("local_extracted/game/game.json missing: run tools/assets/build_game_data.py"));
    let mut hero: Option<String> = read_settings(&root)["hero"].as_str().map(str::to_string);
    let maps = map_keys(&root);
    let mut map_key: String = read_settings(&root)["map"].as_str().filter(|m| maps.iter().any(|k| k == m)).unwrap_or("tutorial").to_string();
    let mut game = Game::new(game_data(&root, &json, hero.as_deref(), &map_key));
    if benchmark_input("RRBTD6_BENCH_SIM_DIR").is_some()&&benchmark_input("RRBTD6_BENCH_SETTINGS").is_some(){
        eprintln!("frame-bench-inputs: isolated mode={}",game.mode_name);
    }
    mark("audio");
    let mut audio = audio::Audio::new(&root);
    let saved_volumes = load_settings(&root);
    if let Some(v) = saved_volumes {
        audio.sfx_volume = audio::DEFAULT_SFX * v[1];
        audio.set_music_volume(audio::DEFAULT_MUSIC * v[0]);
    }

    let mut adv = advanced::Advanced::load(&read_settings(&root));
    render3d::FULL_TEXTURES.store(adv.full_textures, std::sync::atomic::Ordering::Relaxed);
    mark("hud");
    let mut hud = Hud::load(&root, &game).await;
    let startup_test = std::env::var_os("RRBTD6_STARTUP").is_some();
    let mut startup = ((std::env::var_os("RRBTD6_FRONT").is_none() && std::env::var_os("RRBTD6_AUTOTEST").is_none() && !adv.skip_startup) || startup_test).then(|| {
        let version = std::fs::read_to_string(root.join("game_version.json")).ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v["display"].as_str().map(str::to_owned)).unwrap_or_default();
        startup::Startup::new(&root, version, None)
    });
    macro_rules! boot {
        ($step:expr, $label:expr, $work:expr) => {{
            if let (Some(screen), Some(h)) = (startup.as_mut(), hud.as_mut()) {
                screen.task(h, $step, $label, adv.fps_cap, $work).await
            } else { $work.await }
        }};
    }

    mark("terrain");
    // Bloon sprites (shared ones once) and shop icons, decoded in parallel.
    let sprites: Vec<Option<String>> = game.defs.iter().map(|d| d.display.first().map(|l| l.png.trim_end_matches(".png").to_string()).or(d.sprite.clone())).collect();
    let mut uniq: Vec<String> = sprites.iter().flatten().cloned().collect();
    uniq.sort();
    uniq.dedup();
    let sprite_path = |s: &str| if s.starts_with("bloon_displays/") { root.join(format!("{s}.png")) } else { root.join("sprites").join(format!("{s}.png")) };
    let mut paths: Vec<PathBuf> = uniq.iter().map(|s| sprite_path(s)).collect();
    paths.extend(game.tower_defs.iter().map(|(_, d)| root.join("sprites").join("MonkeyIcons").join(format!("{}.png", d.icon))));
    let decode_paths = paths.clone();
    let images = boot!(1, "Inflating Bloons...", load_work::spawn(move || render3d::decode_batch(&decode_paths)));
    // Every bloon sprite in one atlas: a crowd of mixed bloons stays one batch in painter's
    // order (separate textures broke it into thousands of draw calls).
    let atlas = render3d::build_atlas(&images[..uniq.len()].iter().map(Option::as_ref).collect::<Vec<_>>());
    // Controlled reference retains the old upload/lifetime behavior in the same build.
    let legacy_sprite_storage=std::env::var_os("RRBTD6_KEEP_SPRITE_DUPLICATES").is_some();
    let decoded_sprite_bytes:usize=images.iter().flatten().map(|i|i.bytes.len()).sum();
    let loaded: Vec<Option<Texture2D>> = images[..uniq.len()].iter().map(|i| i.as_ref().map(render3d::upload)).collect();
    // The terrain image (11.8 Mpx on Monkey Meadow) is only drawn by the 2D view: loaded the
    // first time that view shows.
    let mut terrain: Option<Texture2D> = None;
    let mut terrain_loaded = false;
    let tower_tex:Vec<Option<std::rc::Rc<Texture2D>>>=images[uniq.len()..].iter().zip(&paths[uniq.len()..]).map(|(img,path)|img.as_ref().and_then(|img|{
        if legacy_sprite_storage{Some(std::rc::Rc::new(render3d::upload(img)))}else{ui::shared_decoded_ui_texture(path,img)}
    })).collect();
    if std::env::var_os("RRBTD6_SPRITE_STORAGE_CHECK").is_some(){
        let mut checked=0;
        for ((img,path),texture) in images[uniq.len()..].iter().zip(&paths[uniq.len()..]).zip(&tower_tex){
            if let (Some(img),Some(texture))=(img,texture){
                assert_eq!(texture.get_texture_data().bytes,img.bytes,"decoded icon GPU pixels differ: {}",path.display());
                if !legacy_sprite_storage{assert!(std::rc::Rc::ptr_eq(texture,&ui::shared_decoded_ui_texture(path,img).unwrap()));}
                checked+=1;
            }
        }
        eprintln!("sprite-storage-check: {checked} icons match GPU pixels; {} decoded bytes released in optimized mode",decoded_sprite_bytes);
    }
    let cache: HashMap<&str, Option<Texture2D>> = uniq.iter().map(String::as_str).zip(loaded).collect();
    let bloon_tex: Vec<Option<Texture2D>> = sprites.iter().map(|s| s.as_deref().and_then(|s| cache[s].clone())).collect();
    let uv_of: HashMap<&str, Option<[f32; 4]>> = uniq.iter().map(String::as_str).zip(atlas.as_ref().map_or(vec![None; uniq.len()], |a| a.uv.clone())).collect();
    let bloon_uv: Vec<Option<[f32; 4]>> = sprites.iter().map(|s| s.as_deref().and_then(|s| uv_of[s])).collect();
    let legacy_sprite_images=if legacy_sprite_storage{Some(images)}else{drop(images);None};
    // Keep the reference branch alive until exit; optimized play retains no CPU pixels.
    let _legacy_sprite_images=legacy_sprite_images;
    if std::env::var_os("RRBTD6_TIMING").is_some(){eprintln!("sprite-storage: decoded {} MiB, retained {} MiB",decoded_sprite_bytes/1048576,if legacy_sprite_storage{decoded_sprite_bytes/1048576}else{0});}

    mark("map model");
    let mut map_model = boot!(2, "Trimming the Gardens...", render3d::Model::load(&root.join("models").join(format!("map_{map_key}"))));
    if map_model.is_none() {
        eprintln!("map model for {map_key} failed to load");
    }
    // Tower base models load when first needed (placed, in hand) or one per frame between
    // rounds in the background, not all at startup (about 0.8 s for every tower).
    let ntowers = game.tower_defs.len();
    let mut tower_models: Vec<Option<render3d::Model>> = (0..ntowers).map(|_| None).collect();
    let mut models_loaded = vec![false; ntowers];
    let mut shaders = anim::Shaders::new();
    let mut tower_anims: Vec<Option<anim::AnimModel>> = Vec::new();
    // Per-tier display prefabs (tools/assets/export_dart_tiers.py): tier key ("030") -> model dir,
    // loaded on first use since each one is 8-18 MB of baked animation.
    let mut tier_index: Vec<HashMap<String, String>> = Vec::new();
    // Attack displays (export_tower_tiers.py attack.json): per tier key, the models an attack
    // adds on the tower (Dartling guns, Super Monkey arms...): (model dir, offset, scale).
    let mut attack_index: Vec<HashMap<String, Vec<(String, [f32; 3], usize)>>> = Vec::new();
    for (name, _) in &game.tower_defs {
        let a: serde_json::Value = std::fs::read_to_string(root.join("models").join(name).join("tiers").join("attack.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        attack_index.push(
            a.as_object()
                .map(|o| {
                    o.iter()
                        .map(|(k, v)| {
                            let list = v.as_array().map(|l| {
                                l.iter()
                                    .filter_map(|e| {
                                        let off = e["offset"].as_array().map(|o| [0, 1, 2].map(|i| o.get(i).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32)).unwrap_or([0.0; 3]);
                                        Some((e["dir"].as_str()?.to_string(), off, e["attack"].as_u64().unwrap_or(0) as usize))
                                    })
                                    .collect()
                            });
                            (k.clone(), list.unwrap_or_default())
                        })
                        .collect()
                })
                .unwrap_or_default(),
        );
        tower_anims.push(None);
        let idx = std::fs::read_to_string(root.join("models").join(name).join("tiers").join("index.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        tier_index.push(idx);
    }
    let mut tier_anims: HashMap<(usize, String), Option<anim::AnimModel>> = HashMap::new();
    // Each tower body's current idle (clip, start): idle variants play out whole (anim.rs idle_pose).
    let mut idle_picks: HashMap<u64, (usize, f32)> = HashMap::new();
    let mut prefetch = anim::Prefetch::default();
    let retire_predictions=std::env::var_os("RRBTD6_NO_PREDICTIVE_RETIRE").is_none();
    let mut prediction_key=None;
    let mut predicted_models=Vec::<((usize,String),std::path::PathBuf)>::new();
    let mut predicted_effects=Vec::<&'static str>::new();
    // Scope preparation to the round's reachable displays; decoded jobs and
    // cooperative GPU uploads share a four-model staging limit.
    let mut warm_round=None;
    let mut warm_bloons=Vec::<String>::new();
    let bloon_model_root=root.join("models").join("bloons");
    let background_assets=std::env::var_os("RRBTD6_NO_BACKGROUND_ASSETS").is_none();
    if !background_assets{
        // Preserve the previous four-job startup prefetch in A/B references.
        let mut names:Vec<_>=game.defs.iter().flat_map(|d|d.model.iter().chain(d.damage_models.iter().map(|m|&m.model))).collect();
        names.sort();names.dedup();
        for name in names{prefetch.request(bloon_model_root.join(name));}
    }
    // MOAB-class 3D displays and damage states, loaded when first seen.
    let mut bloon_models: HashMap<String, Option<anim::AnimModel>> = HashMap::new();
    // Sub-tower (sentry) stands and turrets, models/subtowers/<prefab>, loaded when first seen.
    let mut sub_models: HashMap<String, Option<anim::AnimModel>> = HashMap::new();
    // Heli Pilot air units by prefab name (models/air/<name>).
    let mut air_models: HashMap<String, Option<anim::AnimModel>> = HashMap::new();
    // Monkey Fan Club fans by prefab name (models/fans/<name>).
    let mut fan_models: HashMap<String, Option<anim::AnimModel>> = HashMap::new();
    let outline_scale: f32 = std::env::var("RRBTD6_OUTLINE").ok().and_then(|v| v.parse().ok()).unwrap_or(0.7);
    mark("tower index");
    // Loads tower def `d`'s base models if not yet loaded.
    macro_rules! ensure_tower {
        ($d:expr) => {{
            let d: usize = $d;
            if !models_loaded[d] {
                models_loaded[d] = true;
                let dir = root.join("models").join(&game.tower_defs[d].0);
                tower_models[d] = load_work::complete_asset(render3d::Model::load(&dir));
                tower_anims[d] = load_work::complete_asset(anim::AnimModel::load(&dir));
            }
        }};
    }
    if let Some(d) = game.tower_defs.iter().position(|(n, _)| n == "DartMonkey") {
        models_loaded[d] = true;
        let dir = root.join("models").join(&game.tower_defs[d].0);
        tower_models[d] = boot!(3, "Training Monkeys...", render3d::Model::load(&dir));
        tower_anims[d] = boot!(4, "Preparing Darts...", anim::AnimModel::load(&dir));
    }
    let dart = game
        .tower_defs
        .iter()
        .position(|(n, _)| n == "DartMonkey")
        .and_then(|i| tower_models[i].as_ref())
        .and_then(|m| render3d::extract_part(m, "DartMonkeyDart"));
    // Projectile displays (tools/assets/export_projectiles.py): DisplayModel GUID -> sprite or mesh.
    let proj_root = root.join("models").join("projectiles");
    mark("projectiles");
    let proj_index: HashMap<String, String> = std::fs::read_to_string(proj_root.join("index.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    // Loaded when a projectile (or orbit) first shows them, not all at startup.
    let mut proj_displays: HashMap<String, ProjDisplay> = HashMap::new();
    let mut proj_tried: std::collections::HashSet<String> = std::collections::HashSet::new();
    async fn load_proj(dir: std::path::PathBuf) -> Option<ProjDisplay> {
        let _trace=load_work::Trace::new("projectile",&dir);
        if let Some(j) = std::fs::read_to_string(dir.join("layers.json")).ok().and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()) {
            let mut layers = Vec::new();
            for l in j["layers"].as_array().into_iter().flatten() {
                let Some(t) = tex(dir.join(l["png"].as_str().unwrap_or(""))).await else { continue };
                let f = |v: &serde_json::Value| v.as_f64().unwrap_or(0.0) as f32;
                let c = |i: usize| (f(&l["color"][i]).clamp(0.0, 1.0) * 255.0) as u8;
                layers.push((f(&l["order"]), f(&l["w"]) * f(&l["h"]), ProjLayer {
                    tex: t,
                    w: f(&l["w"]),
                    h: f(&l["h"]),
                    pivot: [f(&l["pivot"][0]), f(&l["pivot"][1])],
                    offset: [f(&l["offset"][0]), f(&l["offset"][1])],
                    angle: f(&l["angle"]),
                    spin: f(&l["spin"]),
                    color: [c(0), c(1), c(2), c(3)],
                    scale_anim: l["scale_anim"].as_object().map(|a| {
                        (a["duration"].as_f64().unwrap_or(1.0) as f32,
                         a["keys"].as_array().into_iter().flatten().map(|k| (f(&k[0]), f(&k[1]))).collect())
                    }),
                }));
            }
            if !layers.is_empty() {
                layers.sort_by(|a, b| a.0.total_cmp(&b.0).then(b.1.total_cmp(&a.1)));
                return Some(ProjDisplay::Layers(layers.into_iter().map(|l| l.2).collect()));
            }
        }
        let meta: Option<serde_json::Value> = std::fs::read_to_string(dir.join("sprite.json")).ok().and_then(|s| serde_json::from_str(&s).ok());
        let d = match meta {
            Some(m) => tex(dir.join("sprite.png")).await.map(|t| ProjDisplay::Sprite {
                tex: t,
                w: m["w"].as_f64().unwrap_or(3.0) as f32,
                h: m["h"].as_f64().unwrap_or(9.0) as f32,
                pivot: [m["pivot"][0].as_f64().unwrap_or(0.5) as f32, m["pivot"][1].as_f64().unwrap_or(0.5) as f32],
            }),
            None => render3d::Model::load(&dir).await.map(ProjDisplay::Mesh),
        };
        d
    }
    fn projectile_art(tier:&data::Tier)->Vec<&str>{
        fn visit<'a>(p:&'a data::Proj,out:&mut Vec<&'a str>){
            out.extend(p.display.iter().map(String::as_str));
            out.extend(p.pierce_sprites.iter().map(String::as_str));
            for child in &p.on_contact{visit(&child.projectile,out);}
            for child in &p.on_exhaust{visit(&child.projectile,out);}
        }
        let mut out=Vec::new();
        for attack in &tier.attacks{for weapon in &attack.weapons{
            visit(&weapon.projectile,&mut out);
            if let Some(alt)=&weapon.alternate{visit(&alt.projectile,&mut out);}
            out.extend(weapon.crit.iter().flat_map(|c|c.display.iter().map(String::as_str)));
        }}
        if let Some(p)=&tier.preempt{visit(p,&mut out);}
        out.extend(tier.orbit.iter().flat_map(|o|o.display.iter().map(String::as_str)));
        out
    }
    let mut warm_towers=Vec::<(u64,usize,[u8;3],Option<u32>)>::new();
    let keep_fallback_models=std::env::var_os("RRBTD6_KEEP_FALLBACK_MODELS").is_some();
    let mut warm_projectiles=Vec::<String>::new();
    let mut projectile_upload:Option<(String,std::pin::Pin<Box<dyn std::future::Future<Output=Option<ProjDisplay>>>>)>=None;
    let mut warm_effects=Vec::<String>::new();
    let mut effect_upload:Option<(String,std::pin::Pin<Box<dyn std::future::Future<Output=Option<particles::EffectDef>>>>)>=None;
    // Pop flipbook (Assets/Bloons/Graphics/Effects/Pop.prefab).
    let pop_dir = root.join("effects").join("pop");
    mark("pops");
    let pop_meta: Option<serde_json::Value> = std::fs::read_to_string(pop_dir.join("effect.json")).ok().and_then(|s| serde_json::from_str(&s).ok());
    let pop_frames = boot!(5, "Collecting Bananas...", async {
    let mut pop_frames: Vec<(Texture2D, f32, f32)> = Vec::new();
    if let Some(pm) = &pop_meta {
        for f in pm["frames"].as_array().cloned().unwrap_or_default() {
            if let Some(t) = tex(pop_dir.join(f["png"].as_str().unwrap_or(""))).await {
                pop_frames.push((t, f["w"].as_f64().unwrap_or(10.0) as f32, f["h"].as_f64().unwrap_or(10.0) as f32));
            }
        }
    }
    pop_frames
    });
    let pop_fps = pop_meta.as_ref().and_then(|m| m["fps"].as_f64()).unwrap_or(24.0) as f32;
    let pop_life = pop_meta.as_ref().and_then(|m| m["lifespan"].as_f64()).unwrap_or(0.17) as f32;
    // TUNED (player report: pops looked too big next to the game's): the prefab's own size
    // (frame px / PPU x root scale 0.9) is ~1.8x a bloon; drawn at 0.7 of it (~1.25x).
    let pop_scale = 0.7 * pop_meta.as_ref().and_then(|m| m["scale"].as_f64()).unwrap_or(0.9) as f32;
    let mut pops: Vec<([f32; 2], f32)> = Vec::new();
    let mut bursts: Vec<(game::Burst, f32)> = Vec::new();
    let mut tracers: Vec<(([f32; 2], [f32; 2]), f32)> = Vec::new();
    let mut cash_pops: Vec<(([f32; 2], f64), f32)> = Vec::new();
    // Crossbow Master's "Crit!" (CritText.prefab): (where, when).
    let mut crit_texts: Vec<([f32; 2], f32)> = Vec::new();
    // Effect prefabs (Unity particle systems): upgrade effects so far.
    let mut particles = particles::Particles::new(&root);
    particles.spawn_ambient(&map_key);
    // Particle systems inside tower prefabs (export_particles.py tiers:<Tower>): model dir ->
    // effect, per tower type; and which models' effects each placed tower is playing.
    let tier_fx: Vec<HashMap<String, String>> = game
        .tower_defs
        .iter()
        .map(|(name, _)| {
            std::fs::read_to_string(root.join("effects").join(format!("tier_{name}.json")))
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        })
        .collect();
    let mut tower_fx: HashMap<u64, Vec<String>> = HashMap::new();
    // Particle systems inside projectile prefabs (export_particles.py projectiles): projectile
    // display name -> effect; the projectiles whose effect is playing (tagged by uid).
    let proj_fx: HashMap<String, String> = std::fs::read_to_string(root.join("effects").join("proj_index.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut proj_fx_live: std::collections::HashSet<u64> = std::collections::HashSet::new();
    /// Tags of projectile effects, apart from tower ids.
    const PROJ_FX_TAG: u64 = 1 << 62;
    let cursor_up = boot!(6, "Waking the Sun God...", tex(root.join("ui").join("HandCursor.png")));
    let cursor_down = boot!(7, "Summoning Heroes...", tex(root.join("ui").join("HandCursorDown.png")));
    if cursor_up.is_some() && cursor_down.is_some() {
        show_mouse(false);
    }

    if let (Some(h), Some(v)) = (hud.as_mut(), saved_volumes) {
        h.menus.volumes = v;
    }
    if hud.is_none() {
        eprintln!("missing local_extracted/ui/prefabs: run tools/assets/dump_ui_prefab.py (see tools/build_local_assets.sh)");
    }
    let mut view3d = map_model.is_some() && std::env::var("RRBTD6_2D").is_err();
    let mut scratch = Vec::new();

    let mut placing: Option<usize> = None;
    // Placing by dragging from the shop: the tower drops where the mouse is released.
    let mut dragging = false;
    let mut selected: Option<usize> = None;
    // "Set Target": the tower whose aim point the next map click sets.
    let mut picking: Option<usize> = None;
    // A tower-targeted ability (Overclock) waiting for the player to click a tower.
    let mut ability_pick: Option<String> = None;
    // A Geraldo shop item in hand: the next map click uses it there.
    let mut item_pick: Option<usize> = None;
    mark("rest");
    if std::env::var_os("RRBTD6_SHARED_ASSET_CHECK").is_some(){ui::verify_shared_ui_textures(&root);render3d::verify_cooperative_upload().await;}
    if std::env::var_os("RRBTD6_RENDER_CHECK").is_some(){anim::verify_draw_submission(&root).await;render3d::verify_sprite_submission().await;}
    if std::env::var_os("RRBTD6_PREFETCH_CHECK").is_some(){anim::verify_prefetch(&root).await;}
    if std::env::var_os("RRBTD6_TEXT_CACHE_CHECK").is_some(){uitree::verify_text_layout_cache(&root).await;}
    if std::env::var_os("RRBTD6_PARTICLE_TEXTURE_CHECK").is_some(){particles::verify_texture_sharing(&root).await;}
    if std::env::var_os("RRBTD6_UPGRADE_PREP_CHECK").is_some(){particles::verify_upgrade_preparation(&root).await;}
    if std::env::var_os("RRBTD6_SHADOW_TEXTURE_CHECK").is_some(){anim::verify_shadow_sharing(&root).await;}
    if std::env::var_os("RRBTD6_MAP_BATCH_CHECK").is_some(){render3d::verify_map_batching(&root).await;}
    if std::env::var_os("RRBTD6_STATIC_MAP_CHECK").is_some(){render3d::verify_static_maps(&root).await;}
    let mortar_target = tex(root.join("sprites").join("IngameUi").join("MortarTarget.png")).await;
    let mut speed = 1u32;
    let mut coop_config:Option<rrbtd6_coop::MatchStart>=None;
    let mut coop_runtime:Option<coop_game::Runtime>=None;
    let mut coop_error:Option<String>=None;
    let mut coop_mouse_at=0.0f64;
    let mut coop_test_stage=0u8;
    let mut coop_test_position=None;
    let mut sprite_batch = render3d::SpriteBatch::new();
    let mut bloon_order: Vec<([f32; 2], usize)> = Vec::new();
    let mut moab_rank:HashMap<usize,f32>=HashMap::new();
    let mut moab_order:Vec<usize>=Vec::new();
    let mut sim_stats: (f64, u64, f64) = (0.0, 0, 0.0);
    let mut acc = 0.0f32;
    let mut auto_start = read_settings(&root)["auto_start"].as_bool().unwrap_or(false);
    let mut show_placement = std::env::var("RRBTD6_PLACEMAP").is_ok();
    // An open full-screen menu (the game is paused behind it) and one requested this frame.
    let mut menu: Option<(menus::Menu, menus::Backdrop)> = None;
    let mut open_menu: Option<menus::Menu> = None;
    // RRBTD6_MENU=pause|upgrades: autotest opens that menu shortly before its screenshot.
    let autotest_menu = match std::env::var("RRBTD6_MENU").as_deref() {
        Ok("pause") => Some(menus::Menu::Pause),
        Ok("upgrades") => Some(menus::Menu::Upgrades),
        Ok("victory") => Some(menus::Menu::Victory),
        Ok("defeat") => Some(menus::Menu::Defeat),
        _ => None,
    };
    // Victory / defeat screens: shown once when the game ends; Review Map hides them until the
    // next click.
    let mut end_shown = false;
    let mut reviewing = false;
    // The front end (main menu -> map -> difficulty -> mode). Autotests start in the game unless
    // RRBTD6_FRONT=main|maps|difficulty|modes asks for a front-end screenshot.
    let mut pause_options_test=if std::env::var_os("RRBTD6_AUTOTEST").is_some(){std::env::var("RRBTD6_PAUSE_OPTIONS").ok()}else{None};
    let mut options_back_test=if std::env::var_os("RRBTD6_AUTOTEST").is_some(){std::env::var("RRBTD6_OPTIONS_BACK_FRAME").ok().and_then(|s|s.parse::<u32>().ok())}else{None};
    let mut front = frontend::FrontEnd::new(&root);
    if startup.is_some() {
        let mut names=boot!(8,"Trimming the Gardens...",front.prepare_home_entry());
        if std::env::var_os("RRBTD6_NO_TITLE_PRELOAD").is_none(){names.extend(startup::title_sprites(&root));}
        let preparation=hud.as_ref().map(|h|h.sprites.prepare(&names));
        let (plane, prepared) = boot!(8, "Trimming the Gardens...", async {
            let plane = anim::AnimModel::load(&root.join("models/startup_plane")).await;
            let prepared = match preparation { Some(work) => work.await, None => Vec::new() };
            (plane, prepared)
        });
        if let Some(h) = hud.as_mut() { h.sprites.install_prepared(prepared); }
        if let Some(screen) = startup.as_mut() { screen.set_plane(plane); }
    }
    let front_shot = std::env::var("RRBTD6_FRONT").ok();
    front.screen = match front_shot.as_deref() {
        Some("maps") => frontend::Screen::Maps,
        Some("heroes") => frontend::Screen::Heroes,
        Some("loading") => frontend::Screen::Loading,
        Some("monkeys") => frontend::Screen::Monkeys,
        Some("difficulty") => frontend::Screen::Difficulty,
        Some("modes") => frontend::Screen::Modes,
        Some("settings") => frontend::Screen::Settings,
        Some("advanced") => frontend::Screen::Advanced,
        Some("hotkeys") => frontend::Screen::Hotkeys,
        Some(name) => frontend::MenuPage::from_name(name).map(frontend::Screen::Menu).unwrap_or(frontend::Screen::Main),
        _ => frontend::Screen::Main,
    };
    let mut in_front = std::env::var("RRBTD6_AUTOTEST").is_err() || front_shot.is_some() || startup_test;
    if let Some(v) = saved_volumes {
        front.sliders[..3].copy_from_slice(&v);
    }
    // Advanced settings (FPS limit / counter, render scale) and the 3D view's offscreen target.

    front.adv = adv.clone();
    let mut binds = keybinds::Keybinds::load(&read_settings(&root));
    front.binds = binds.clone();
    let mut view_rt: Option<(RenderTarget, u32, u32)> = None;
    let mut frame_t0 = std::time::Instant::now();
    let mut frame_pacer = frame_pacer::FramePacer::default();
    let legacy_pacing = std::env::var_os("RRBTD6_LEGACY_FRAME_PACING").is_some();
    {
        let (lo, hi, def) = frontend::CURSOR_PX;
        let px = read_settings(&root)["cursor_px"].as_f64().map_or(def, |v| v as f32).clamp(lo, hi);
        CURSOR_SCALE.store(px.to_bits(), std::sync::atomic::Ordering::Relaxed);
        front.sliders[3] = (px - lo) / (hi - lo);
    }
    // A map to load at the top of the next frame (front end Play, N key).
    let mut pending_map: Option<String> = None;
    let mut race_config:Option<serde_json::Value>=None;

    // Self-test: place towers at the first valid spots near the track, play, screenshot, exit.
    let autotest = std::env::var("RRBTD6_AUTOTEST").ok();
    let presentation_check=autotest.is_some()&&std::env::var_os("RRBTD6_PRESENTATION_CHECK").is_some();
    let late_tower=if autotest.is_some(){std::env::var("RRBTD6_LATE_TOWER").ok().and_then(|s|{
        let (name,frame)=s.split_once(':')?;
        Some((game.tower_defs.iter().position(|(id,_)|id==name)?,frame.parse::<u32>().ok()?))
    })}else{None};
    let late_preview=late_tower.is_some()&&std::env::var_os("RRBTD6_LATE_PREVIEW").is_some();
    let upgrade_case=if autotest.is_some(){std::env::var("RRBTD6_UPGRADE_CASE").ok().and_then(|s|{
        let v:Vec<usize>=s.split(':').map(str::parse).collect::<Result<_,_>>().ok()?;
        (v.len()==4&&v[2]<3).then(||(v[0],v[1] as u32,v[2],v[3] as u32))
    })}else{None};
    let mut frame_capture=frame_bench::Capture::from_env();
    let bench_steps=if autotest.is_some()&&frame_capture.is_some(){Some(std::env::var("RRBTD6_BENCH_STEPS").ok().and_then(|v|v.parse::<u32>().ok()).unwrap_or(2).clamp(1,60))}else{None};
    if bench_steps.is_some(){game.lives=1.0e12;}
    let bench_round=if bench_steps.is_some(){std::env::var("RRBTD6_BENCH_ROUND").ok().and_then(|r|r.parse::<usize>().ok())}else{None};
    let bench_warmup=std::env::var("RRBTD6_BENCH_WARMUP").ok().and_then(|v|v.parse::<u32>().ok()).unwrap_or(300);
    let gallery = std::env::var("RRBTD6_GALLERY").is_ok();
    mark("loop");
    let mut frames = 0u32;
    let frontend_timing=std::env::var("RRBTD6_TIMING").is_ok();
    let (mut frontend_samples,mut frontend_cpu,mut frontend_max)=(0u32,0.0f64,0.0f64);
    let mut hud_time = 0.0f64;
    // CPU time spent submitting bloons (autotest log).
    let mut bloon_time = 0.0f64;
    // Our CPU time per frame and the time spent in next_frame (GPU submit, swap).
    let (mut cpu_time, mut flip_time) = (0.0f64, 0.0f64);
    // RRBTD6_TOWERS: tower indices (tower_defs order) the autotest cycles through, default "0";
    // letters count on from 10 (a = 10). Or comma-separated tower names ("MonkeySub,Mermonkey").
    let by_name: Vec<usize> = std::env::var("RRBTD6_TOWERS")
        .map(|s| s.split(',').filter_map(|n| game.tower_defs.iter().position(|(x, _)| x == n.trim())).collect())
        .unwrap_or_default();
    let autotest_towers: Vec<usize> = std::env::var("RRBTD6_TOWERS")
        .ok()
        .filter(|_| by_name.is_empty())
        .ok_or(())
        .map(|s| {
            s.bytes()
                .filter_map(|b| match b {
                    b'0'..=b'9' => Some(usize::from(b - b'0')),
                    b'a'..=b'z' => Some(usize::from(b - b'a') + 10),
                    _ => None,
                })
                .filter(|&d| d < game.tower_defs.len())
                .collect()
        })
        .ok()
        .filter(|v: &Vec<usize>| !v.is_empty())
        .or((!by_name.is_empty()).then(|| by_name.clone()))
        .unwrap_or_else(|| vec![0]);
    if autotest.is_some() {
        if let Some(c) = std::env::var("RRBTD6_CASH").ok().and_then(|v| v.parse().ok()) {
            game.cash = c;
        }
        let mut placed = 0;
        // RRBTD6_PLACE: comma-separated track distances to place towers next to.
        let spots: Vec<f32> = std::env::var("RRBTD6_PLACE")
            .ok()
            .map(|s| s.split(',').filter_map(|v| v.trim().parse().ok()).collect())
            .filter(|v: &Vec<f32>| !v.is_empty())
            .unwrap_or_else(|| vec![300.0, 520.0, 700.0]);
        'outer: for &d in &spots {
            let c = game.track.pos(d);
            // Further out when the near ring has no room (water towers need the pond).
            'ring: for r in [24.0f32, 40.0, 60.0, 90.0, 130.0] {
                for k in 0..48 {
                    let a = k as f32 / 48.0 * std::f32::consts::TAU;
                    let p = [c[0] + r * a.cos(), c[1] + r * a.sin()];
                    if game.place(autotest_towers[placed % autotest_towers.len()], p) {
                        placed += 1;
                        if placed == 3 { break 'outer; }
                        break 'ring;
                    }
                }
            }
        }
        // RRBTD6_PATHS also upgrades the first towers straight away.
        if let Ok(s) = std::env::var("RRBTD6_PATHS") {
            for t in 0..game.towers.len() {
                for path in s.bytes().filter_map(|b| b.checked_sub(b'0').map(usize::from)).filter(|&p| p < 3) {
                    while game.upgrade(t, path) {}
                }
            }
        }
        // RRBTD6_GALLERY=<tower>: that tower in a grid of tiers (RRBTD6_GALLERY_TIERS, comma
        // list, default every path's 1-5 plus 000), no bloons, for checking models.
        if let Ok(name) = std::env::var("RRBTD6_GALLERY") {
            game.towers.clear();
            game.cash = 1e12;
            if let Some(def) = game.tower_defs.iter().position(|(n, _)| *n == name) {
                let tiers: Vec<String> = std::env::var("RRBTD6_GALLERY_TIERS")
                    .map(|s| s.split(',').map(str::to_string).collect())
                    .unwrap_or_else(|_| ["000", "100", "200", "300", "400", "500", "010", "020", "030", "040", "050", "001", "002", "003", "004", "005"].map(String::from).to_vec());
                // 6 x 3 grid over the middle of the map, each spot nudged until placement works.
                let mut k = 0;
                'grid: for gy in 0..3 {
                    for gx in 0..6 {
                        if k >= tiers.len() {
                            break 'grid;
                        }
                        let c = [-150.0 + gx as f32 * 48.0, -70.0 + gy as f32 * 60.0];
                        let spot = (0..40).map(|i| {
                            let (r, a) = (i as f32 * 1.5, i as f32 * 2.4);
                            [c[0] + r * a.cos(), c[1] + r * a.sin()]
                        }).find(|&p| game.place(def, p));
                        if spot.is_some() {
                            let t = game.towers.len() - 1;
                            let b = tiers[k].as_bytes();
                            game.towers[t].tiers = [b[0] - b'0', b[1] - b'0', b[2] - b'0'];
                            game.towers[t].angle = -std::f32::consts::FRAC_PI_2;
                            k += 1;
                        }
                    }
                }
                eprintln!("autotest: gallery {name}: {k} of {}", tiers.len());
            }
        }
        // RRBTD6_PARAGON: the three placed towers become 5-0-0, 0-5-0 and 0-0-5, then the
        // first buys its Paragon.
        if std::env::var("RRBTD6_PARAGON").is_ok() && game.towers.len() >= 3 {
            for (i, k) in [[5, 0, 0], [0, 5, 0], [0, 0, 5]].into_iter().enumerate() {
                game.towers[i].tiers = k;
            }
            if std::env::var("RRBTD6_PARAGON").as_deref() == Ok("popup") {
                open_menu = Some(menus::Menu::Paragon);
            } else if std::env::var("RRBTD6_PARAGON").as_deref() == Ok("ready") {
                // Tiers set only: the panel offers the Paragon.
            } else {
                eprintln!("autotest: paragon {}", game.upgrade(0, 0));
            }
            for tw in &game.towers {
                eprintln!("autotest: tower {} tiers {:?} sub {} paragon {:?}", game.tower_defs[tw.def].0, tw.tiers, tw.sub.is_some(), tw.paragon.as_ref().map(|p| p.0));
            }
        }
        // RRBTD6_MODE=Point|Pursuit|...: targeting mode for every placed tower.
        if let Some(m) = std::env::var("RRBTD6_MODE").ok().and_then(|v| game::TargetMode::parse(&v)) {
            for tw in &mut game.towers {
                tw.target = m;
            }
        }
        eprintln!("autotest: placed {placed} towers, cash {}", game.cash);
        for (i, tw) in game.towers.iter().enumerate() {
            eprintln!("autotest: tower {i} at {:?}", tw.pos);
        }
        // RRBTD6_SELECT=n: which placed tower starts selected (default the first).
        let pick = std::env::var("RRBTD6_SELECT").ok().and_then(|v| v.parse().ok()).unwrap_or(0).min(placed.max(1) - 1);
        selected = (placed > 0 && std::env::var("RRBTD6_NOSELECT").is_err()).then_some(pick);
        // RRBTD6_ROUND=n: start at round n (40: first MOAB).
        if let Some(r) = std::env::var("RRBTD6_ROUND").ok().and_then(|v| v.parse::<usize>().ok()) {
            game.round = r.clamp(1, game.total_rounds()) - 1;
        }
        if let Some(round)=bench_round{
            let degree=std::env::var("RRBTD6_BENCH_DEGREE").ok().and_then(|v|v.parse::<u32>().ok()).unwrap_or(20).clamp(1,100);
            round_bench::setup(&mut game,round,degree);selected=None;auto_start=false;
        }
        game.start_round();
        // RRBTD6_SANDBOX=<bloon>: the sandbox menu open, 10 of that bloon sent.
        if let Ok(b) = std::env::var("RRBTD6_SANDBOX") {
            if let Some(h) = hud.as_mut() {
                h.sandbox.open = true;
            }
            game.sandbox_spawn(&b, false, false, false, 10, 0.3);
        }
        speed = std::env::var("RRBTD6_SPEED").ok().and_then(|v| v.parse().ok()).unwrap_or(3);
    }

    loop {
        // A transition is captured on the frame that requested it, before any clear.
        // Load on the next frame so the old menu/map remains intact during the fade.
        if let Some(k) = pending_map.take() {
            prediction_key=None;predicted_models.clear();predicted_effects.clear();
            warm_round=None;warm_towers.clear();warm_projectiles.clear();projectile_upload=None;warm_effects.clear();effect_upload=None;
            while !front.loading_open_done() {
                if let Some(h) = hud.as_mut() { front.draw_loading(h); }
                audio.music(get_time());
                next_frame().await;
            }
            map_key = k;
            save_setting(&root, "map", serde_json::json!(map_key));
            let worker_root=root.clone();let worker_json=json.clone();let worker_map=map_key.clone();
            let worker_hero=if coop_config.is_some(){None}else{hero.clone()};
            let worker_mode=coop_config.as_ref().map(|c|["Easy","Medium","Hard"][c.room.settings.difficulty as usize].to_owned());
            game=loading_task(load_work::spawn(move||{
                let mut loaded=game_data(&worker_root,&worker_json,worker_hero.as_deref(),&worker_map);
                if let Some(mode)=worker_mode{loaded.mode=mode;}
                Game::new(loaded)
            }),&mut front,&mut hud,&mut audio,adv.fps_cap).await;
            if let Some(config)=&race_config{if let Err(error)=game.configure_front_event(config){eprintln!("Race configuration: {error}");}}
            terrain = None;
            terrain_loaded = false;
            if let Some(h) = hud.as_mut() { front.draw_loading(h); }
            next_frame().await;
            map_model = loading_task(render3d::Model::load(&root.join("models").join(format!("map_{map_key}"))),&mut front,&mut hud,&mut audio,adv.fps_cap).await;
            particles.spawn_ambient(&map_key);
            if let Some(h) = hud.as_mut() { front.draw_loading(h); }
            next_frame().await;
            hud = loading_task(Hud::load(&root, &game),&mut front,&mut hud,&mut audio,adv.fps_cap).await;
            if let Some(config)=&coop_config{
                game.freeplay_seed=config.seed;
                if std::env::var_os("RRBTD6_COOP_PLAYTEST").is_some()&&autotest.is_some(){
                    game.cash=10000.0;coop_test_stage=0;
                    let def=game.tower_defs.iter().position(|(id,_)|id=="DartMonkey").unwrap();
                    let c=game.track.pos(180.0);let mut spots:Vec<[f32;2]>=Vec::new();
                    for r in [24.0f32,36.0,48.0,64.0]{for k in 0..32{
                        let angle=k as f32/32.0*std::f32::consts::TAU;let p=[c[0]+r*angle.cos(),c[1]+r*angle.sin()];
                        if game.can_place(def,p)&&spots.iter().all(|q|(p[0]-q[0]).hypot(p[1]-q[1])>=24.0){spots.push(p);}
                    }}
                    coop_test_position=spots.get(front.coop_slot() as usize).copied();
                }
                let runtime=coop_game::Runtime::new(config.room.players.iter().map(|p|p.slot).collect(),&mut game).expect("validated room");
                front.coop_send(rrbtd6_coop::Command::Loaded(runtime.replica.digest(&game)));
                coop_runtime=Some(runtime);coop_error=None;speed=1;auto_start=false;acc=0.0;
            }
            // Behind the loading screen, load what the first seconds of play would otherwise load
            // on the fly (a hitch as the map appears): every tower's base model, each shop tower's
            // first-tier model (the one in hand when placing) and the MOAB-class models. The
            // background threads stage them; the loading screen keeps drawing in between.
            // Only selectable towers need warm models. Other heroes and towers
            // excluded by this mode stay lazy until they can actually be used.
            let shop: Vec<usize> = game.shop.iter().filter_map(|item|game.tower_defs.iter().position(|(id,_)|*id==item.id)).collect();
            for d in &shop {
                if let Some(dir) = tier_index[*d].get("000").or(tier_index[*d].get("100")) {
                    if !tier_anims.contains_key(&(*d, dir.clone())) {
                        prefetch.request(root.join("models").join(&game.tower_defs[*d].0).join("tiers").join(dir));
                    }
                }
            }
            let moabs: Vec<String> = {
                let mut v: Vec<String> = game.defs.iter().flat_map(|d| d.model.iter().cloned().chain(d.damage_models.iter().map(|m| m.model.clone()))).collect();
                v.sort();
                v.dedup();
                v.retain(|m| !bloon_models.contains_key(m));
                v
            };
            for (k, d) in shop.iter().enumerate() {
                for upcoming in shop.iter().skip(k).take(4){
                    if let Some(dir)=tier_index[*upcoming].get("000").or(tier_index[*upcoming].get("100")){
                        if !tier_anims.contains_key(&(*upcoming,dir.clone())){prefetch.request(root.join("models").join(&game.tower_defs[*upcoming].0).join("tiers").join(dir));}
                    }
                }
                if let Some(dir) = tier_index[*d].get("000").or(tier_index[*d].get("100")).cloned() {
                    if !tier_anims.contains_key(&(*d, dir.clone())) {
                        let path = root.join("models").join(&game.tower_defs[*d].0).join("tiers").join(&dir);
                        tier_anims.insert((*d, dir),loading_task(prefetch.load_cooperative(&path),&mut front,&mut hud,&mut audio,adv.fps_cap).await);
                    }
                }
                mark(&format!("warm tower {}",game.tower_defs[*d].0));
                if let Some(h) = hud.as_mut() { front.draw_loading(h); }
                next_frame().await;
            }
            for (index,m) in moabs.iter().enumerate() {
                for upcoming in moabs.iter().skip(index).take(4){prefetch.request(root.join("models").join("bloons").join(upcoming));}
                let path = root.join("models").join("bloons").join(&m);
                bloon_models.insert(m.clone(),loading_task(prefetch.load_cooperative(&path),&mut front,&mut hud,&mut audio,adv.fps_cap).await);
                mark(&format!("warm bloon {m}"));
                if let Some(h) = hud.as_mut() { front.draw_loading(h); }
                next_frame().await;
            }
            if let Some(h) = hud.as_mut() { front.draw_loading(h); }
            next_frame().await;
            front.finish_loading();
            placing = None;
            selected = None;
        }
        if let Some(runtime)=coop_runtime.as_mut().filter(|_|coop_error.is_none()){
            let selected_id=selected.and_then(|t|game.towers.get(t)).map(|t|t.id);
            let picking_id=picking.and_then(|t|game.towers.get(t)).map(|t|t.id);
            if let Err(error)=runtime.update(&mut game,&mut front,get_frame_time()){
                eprintln!("Co-op stopped: {error}");coop_error=Some(error);front.coop_stop();
            }
            let created=runtime.replica.placed.drain(..).filter(|(slot,_)|*slot==front.coop_slot()).map(|(_,id)|id).last();
            selected=created.or(selected_id).and_then(|id|game.towers.iter().position(|t|t.id==id));
            picking=picking_id.and_then(|id|game.towers.iter().position(|t|t.id==id));
            if created.is_some(){if let Some(t)=selected{if game.target_modes(t)==[game::TargetMode::Point]{picking=Some(t);}}}
            game.removed.clear();speed=runtime.replica.speed as u32;auto_start=runtime.replica.auto_start;
            if std::env::var_os("RRBTD6_COOP_PLAYTEST").is_some()&&autotest.is_some()&&front.coop_running(){
                let slot=front.coop_slot();let own=game.towers.iter().find(|t|runtime.replica.owns(slot,t.id)&&t.sub.is_none()).map(|t|t.id);
                if coop_test_stage==0&&runtime.replica.frame>=5{if let Some(at)=coop_test_position{front.coop_input(rrbtd6_coop::Action::Place{kind:"DartMonkey".into(),at});coop_test_stage=1;}}
                else if coop_test_stage==1&&runtime.replica.frame>=20{if let Some(tower)=own{front.coop_input(rrbtd6_coop::Action::Upgrade{tower,path:0});coop_test_stage=2;}}
                else if coop_test_stage==2&&runtime.replica.frame>=30{if slot==0{front.coop_input(rrbtd6_coop::Action::StartRound);}coop_test_stage=3;}
                else if coop_test_stage==3&&runtime.replica.frame>=100{if slot==1{if let Some(tower)=own{front.coop_input(rrbtd6_coop::Action::Sell{tower});}}coop_test_stage=4;}
                else if coop_test_stage==4&&runtime.replica.frame>=120{if slot==0{front.coop_input(rrbtd6_coop::Action::Donate{to:1,amount:100.0});}else if slot==1{front.coop_input(rrbtd6_coop::Action::RequestCash);}coop_test_stage=5;}
            }
        }
        // Front end: the menus before a game.
        if in_front {
            if let (Some(st), Some(h)) = (startup.as_mut(), hud.as_mut()) {
                // Title music only on the title screen, fading out after Start (TitleScreen
                // .TransitionToMainMenu); the main menu's playlist carries on from there.
                let full = audio::DEFAULT_MUSIC * front.sliders[0];
                match st.title_music() {
                    Some(f) => { audio.set_music_volume(full * f); audio.music(get_time()); }
                    None => audio.set_music_volume(0.0),
                }
                if st.transitioning_home() { front.draw_home_underlay().await; }
                if st.draw(h) {
                    startup = None;
                    audio.set_music_volume(full);
                    front.finish_startup();
                } else {
                    draw_cursor(&cursor_up, &cursor_down);
                    if let (true, Some(path)) = (startup_test, &autotest) {
                        frames += 1;
                        if frames >= std::env::var("RRBTD6_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(420) {
                            get_screen_data().export_png(path);
                            eprintln!("autotest: startup screenshot");
                            break;
                        }
                    }
                    pace(frame_t0, &adv, &mut frame_pacer, legacy_pacing);
                    next_frame().await;
                    frame_t0 = std::time::Instant::now();
                    continue;
                }
            }
            let front_t0=frontend_timing.then(std::time::Instant::now);
            let mut act = None;
            if options_back_test.is_some_and(|n|frames>=n) {options_back_test=None;front.request_back();}
            if let Some(h) = hud.as_mut() {
                act = h.draw_front(&mut front).await;
            }
            if let Some(t0)=front_t0 {
                let elapsed=t0.elapsed().as_secs_f64()*1000.0;
                frontend_samples+=1;frontend_cpu+=elapsed;frontend_max=frontend_max.max(elapsed);
                if frontend_samples==100 {
                    eprintln!("frontend: draw CPU avg {:.3} ms max {:.3} ms (100 frames)",frontend_cpu/100.0,frontend_max);
                    (frontend_samples,frontend_cpu,frontend_max)=(0,0.0,0.0);
                }
            }
            // RRBTD6_FRONT_PLAY=<map>:<mode>: autotest picks that game from the front end.
            if let Some((m, md)) = std::env::var("RRBTD6_FRONT_PLAY").ok().and_then(|v| v.split_once(':').map(|(a, b)| (a.to_string(), b.to_string()))) {
                if frames >= 5 {
                    act = Some(frontend::FrontAction::Play { map: m, mode: md });
                }
            }
            if std::env::var_os("RRBTD6_FRONT_RACE").is_some()&&frames>=5{
                if let Some(data)=frontend::race_entry(&root){act=Some(frontend::FrontAction::Race(data));}
            }
            if let Ok(key)=std::env::var("RRBTD6_FRONT_DAILY"){if frames>=5{
                if let Some(data)=frontend::daily_entry(&root,&key){act=Some(frontend::FrontAction::Race(data));}
            }}
            match act {
                Some(frontend::FrontAction::Coop(config))=>{
                    (placing,selected,picking,ability_pick)=(None,None,None,None);
                    race_config=None;front.begin_loading();pending_map=Some(config.room.settings.map.clone());
                    coop_config=Some(config);coop_runtime=None;coop_error=None;
                    (end_shown,reviewing,menu)=(false,false,None);speed=1;auto_start=false;acc=0.0;in_front=false;
                }
                Some(frontend::FrontAction::Race(data))=>{
                    if let Some(map)=data["challenge"]["map"].as_str(){
                        let map=frontend::map_key(map);
                        let difficulty=data["challenge"]["difficulty"].as_str().unwrap_or("Medium");
                        let rules=data["challenge"]["mode"].as_str().unwrap_or("Standard");
                        let mode=match rules{"Standard"=>difficulty.to_owned(),"Impoppable"=>"Impoppable".into(),"Clicks"=>"CHIMPS".into(),other=>format!("{difficulty} {other}")};
                        save_setting(&root,"mode",serde_json::json!(mode));
                        race_config=Some(data);front.begin_loading();pending_map=Some(map);
                        (end_shown,reviewing,menu)=(false,false,None);speed=1;auto_start=false;acc=0.0;in_front=false;
                    }
                }
                Some(frontend::FrontAction::Play { map, mode }) => {
                    front.coop_stop();coop_config=None;coop_runtime=None;coop_error=None;
                    race_config=None;
                    front.begin_loading();
                    save_setting(&root, "mode", serde_json::json!(mode));
                    pending_map = Some(map);
                    (end_shown, reviewing, menu) = (false, false, None);
                    in_front = false;
                }
                Some(frontend::FrontAction::Quit) => break,
                Some(frontend::FrontAction::ReturnToPause) => {
                    in_front=false;
                    if autotest.is_some(){eprintln!("autotest: options returned to retained pause menu: {}",menu.is_some());}
                }
                Some(frontend::FrontAction::SelectSkin { hero: id, skin }) => {
                    let mut skins = read_settings(&root)["hero_skins"].as_object().cloned().unwrap_or_default();
                    skins.insert(id, serde_json::json!(skin));
                    save_setting(&root, "hero_skins", serde_json::Value::Object(skins));
                }
                Some(frontend::FrontAction::SelectHero(id)) => {
                    save_setting(&root, "hero", serde_json::json!(id));
                    hero = Some(id);
                }
                Some(frontend::FrontAction::Volume(v)) => {
                    audio.sfx_volume = audio::DEFAULT_SFX * v[1];
                    audio.set_music_volume(audio::DEFAULT_MUSIC * v[0]);
                    if let Some(h) = hud.as_mut() {
                        h.menus.volumes = v;
                    }
                    save_settings(&root, v);
                }
                Some(frontend::FrontAction::Jukebox(change))=>{
                    save_setting(&root,"jukebox",serde_json::json!(change.tracks.iter().map(|(id,_)|id).collect::<Vec<_>>()));
                    audio.set_jukebox(&change);
                }
                Some(frontend::FrontAction::Advanced(a)) => {
                    adv = a;
                    render3d::FULL_TEXTURES.store(adv.full_textures, std::sync::atomic::Ordering::Relaxed);
                    save_setting(&root, "advanced", serde_json::to_value(&adv).unwrap_or_default());
                }
                Some(frontend::FrontAction::Hotkeys(b)) => {
                    save_setting(&root, "hotkeys", b.to_json());
                    binds = b;
                }
                Some(frontend::FrontAction::CursorScale(k)) => {
                    CURSOR_SCALE.store(k.to_bits(), std::sync::atomic::Ordering::Relaxed);
                    save_setting(&root, "cursor_px", serde_json::json!(k));
                }
                None => {}
            }
            draw_cursor(&cursor_up, &cursor_down);
            audio.music(get_time());
            if let Some(path) = &autotest {
                frames += 1;
                if frames >= std::env::var("RRBTD6_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(420) {
                    get_screen_data().export_png(path);
                    eprintln!("autotest: front-end screenshot");
                    break;
                }
            }
            pace(frame_t0, &adv, &mut frame_pacer, legacy_pacing);
            next_frame().await;
            frame_t0 = std::time::Instant::now();
            continue;
        }
        let cpu_t0 = get_time();
        if background_assets&&view3d {
            let round_key=(game.round,game.round_active);
            if warm_round!=Some(round_key){warm_round=Some(round_key);warm_bloons=game.upcoming_bloon_models();}
            warm_bloons.retain(|name|!bloon_models.contains_key(name));
            for name in warm_bloons.iter().take(4){prefetch.request(bloon_model_root.join(name));}
            if let Some((path,model))=prefetch.advance(&bloon_model_root){
                if let Some(name)=path.file_name().and_then(|n|n.to_str()){bloon_models.insert(name.to_owned(),model);}
            }
            let changed=warm_towers.len()!=game.towers.len()||game.towers.iter().zip(&warm_towers)
                .any(|(t,key)|*key!=(t.id,t.def,t.tiers,t.paragon.as_ref().map(|p|p.0)));
            if changed{
                warm_towers.clear();warm_towers.extend(game.towers.iter().map(|t|(t.id,t.def,t.tiers,t.paragon.as_ref().map(|p|p.0))));
                warm_projectiles=game.towers.iter().filter_map(|t|game.tier_of(t)).flat_map(projectile_art)
                    .filter_map(|id|proj_index.get(id)).cloned().collect();
                warm_projectiles.sort();warm_projectiles.dedup();
                warm_effects=warm_projectiles.iter().filter_map(|name|proj_fx.get(name)).cloned().collect();
                warm_effects.sort();warm_effects.dedup();
            }
            // One in-flight display, polled once per frame. The same loader and
            // texture data serve both preparation and immediate first-use draws.
            if projectile_upload.is_none(){
                if let Some(name)=warm_projectiles.iter().find(|name|!proj_tried.contains(*name)){
                    projectile_upload=Some((name.clone(),Box::pin(load_proj(proj_root.join(name)))));
                }
            }
            if let Some((name,task))=projectile_upload.as_mut(){
                let mut cx=std::task::Context::from_waker(std::task::Waker::noop());
                if let std::task::Poll::Ready(display)=task.as_mut().poll(&mut cx){
                    proj_tried.insert(name.clone());
                    if let Some(display)=display{proj_displays.insert(name.clone(),display);}
                    projectile_upload=None;
                }
            }
            if effect_upload.is_none(){
                if let Some(name)=warm_effects.iter().map(String::as_str).chain(predicted_effects.iter().copied()).find(|name|!particles.is_prepared(name)){
                    effect_upload=Some((name.to_owned(),Box::pin(particles::EffectDef::load_cooperative(root.join("effects").join(name)))));
                }
            }
            if let Some((name,task))=effect_upload.as_mut(){
                let mut cx=std::task::Context::from_waker(std::task::Waker::noop());
                if let std::task::Poll::Ready(def)=task.as_mut().poll(&mut cx){
                    particles.retain_prepared(name.clone(),def);effect_upload=None;
                }
            }
        }
        if let Some((kind, backdrop)) = &menu {
            let kind = *kind;
            clear_background(BLACK);
            backdrop.draw();
            let mut close = is_key_pressed(KeyCode::Escape);
            if let Some(h) = hud.as_mut() {
                // MusicItem.locKey names the track; else a name made from the clip id.
                h.menus.track_name = audio.track_loc().and_then(|k| h.menus.loc.get(k).cloned())
                    .unwrap_or_else(|| h.menus.track_display(audio.track()));
                h.menus.loop_on = audio.loop_track;
                h.menus.shuffle_on = audio.shuffle;
                h.menus.private_coop(coop_runtime.is_some());
                let mut actions=h.draw_menu(kind, &game, auto_start);
                if kind==menus::Menu::Pause {
                    if let Some(name)=pause_options_test.take() {
                        actions.push(if name=="hotkeys" {menus::MenuAction::Hotkeys}else{menus::MenuAction::Advanced});
                    }
                }
                for a in actions {
                    match a {
                        menus::MenuAction::Close => close = true,
                        menus::MenuAction::Hotkeys | menus::MenuAction::Advanced => {
                            let screen=if matches!(a,menus::MenuAction::Hotkeys){frontend::Screen::Hotkeys}else{frontend::Screen::Advanced};
                            front.open_game_options(screen,backdrop.clone());in_front=true;
                        }
                        menus::MenuAction::ToggleAuto => {
                            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::AutoStart{value:!auto_start});}
                            else{auto_start = !auto_start;save_setting(&root, "auto_start", serde_json::json!(auto_start));}
                        }
                        menus::MenuAction::Track(d) => audio.skip_track(d),
                        menus::MenuAction::ToggleLoop => audio.loop_track = !audio.loop_track,
                        menus::MenuAction::ToggleShuffle => audio.shuffle = !audio.shuffle,
                        menus::MenuAction::Volume(v) => {
                            audio.sfx_volume = audio::DEFAULT_SFX * v[1];
                            audio.set_music_volume(audio::DEFAULT_MUSIC * v[0]);
                            save_settings(&root, v);
                        }
                        menus::MenuAction::Home => {
                            in_front = true;
                            front.return_home();
                            coop_config=None;coop_runtime=None;coop_error=None;
                            (placing, selected, picking, ability_pick) = (None, None, None, None);
                            (dragging, acc) = (false, 0.0);
                            (end_shown, reviewing) = (false, false);
                            close = true;
                        }
                        menus::MenuAction::Restart => {
                            if coop_runtime.is_none(){
                            game = Game::new(game_data(&root, &json, hero.as_deref(), &map_key));
            if let Some(config)=&race_config{if let Err(error)=game.configure_front_event(config){eprintln!("Race configuration: {error}");}}
                            pops.clear();
                            (placing, selected, speed, acc) = (None, None, 1, 0.0);
                            (end_shown, reviewing) = (false, false);
                            }
                            close = true;
                        }
                        menus::MenuAction::BuyParagon(extra) => {
                            if coop_runtime.is_none(){game.buy_paragon(h.menus.paragon_tower, extra);}
                            close = true;
                        }
                        menus::MenuAction::Freeplay => {
                            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Freeplay);}else{game.start_freeplay();}
                            (end_shown, reviewing) = (false, false);
                            close = true;
                        }
                        menus::MenuAction::ReviewMap => {
                            reviewing = true;
                            close = true;
                        }
                    }
                }
            }
            if close {
                menu = None;
            }
            audio.music(get_time());
            draw_cursor(&cursor_up, &cursor_down);
            if let Some(path) = &autotest {
            if std::env::var_os("RRBTD6_COOP_PLAYTEST").is_some()&&std::env::var_os("RRBTD6_COOP_TEST_DONE").is_some_and(|p|std::path::Path::new(&p).exists()){
                get_screen_data().export_png(path);break;
            }
                frames += 1;
                if frames >= std::env::var("RRBTD6_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(420) {
                    get_screen_data().export_png(path);
                    eprintln!("autotest: menu screenshot");
                    break;
                }
            }
            pace(frame_t0, &adv, &mut frame_pacer, legacy_pacing);
            next_frame().await;
            frame_t0 = std::time::Instant::now();
            continue;
        }
        clear_background(Color::from_rgba(30, 40, 30, 255));
        let view = View::new(&game);
        // The 3D view spans the whole window, as in BTD6: the shop panel is drawn over the
        // map's right edge (the camera's orthographicSize fits the height; at 16:9 the terrain
        // reaches past the panel).
        let (vw, vh) = (screen_width(), screen_height());
        let (lo3, hi3) = game.bounds;
        // The in-game camera looks at the world origin on every map (terrain meshes aren't
        // always centred on it: Town Centre's reaches further left).
        // RRBTD6_LOOK=x,z (debug, with RRBTD6_ZOOM): centre the camera there.
        let look = std::env::var("RRBTD6_LOOK").ok().and_then(|v| {
            let (a, b) = v.split_once(',')?;
            Some([a.parse().ok()?, b.parse().ok()?])
        });
        let cam = render3d::camera(look.unwrap_or([0.0, 0.0]), vw, vh);
        if frames%30==10&&std::env::var_os("RRBTD6_VIEWPORT_CHECK").is_some(){
            for p in [[0.0,0.0],[-100.0,-60.0],[100.0,60.0],[-100.0,60.0],[100.0,-60.0]]{
                let s=render3d::to_screen(&cam,[p[0],0.0,p[1]],vw,vh);
                let back=render3d::pick_ground(&cam,s,vw,vh).expect("map point outside fitted viewport");
                assert!((vec2(back[0],back[1])-vec2(p[0],p[1])).length()<0.02,"resized map input mismatch");
            }
            let a=render3d::to_screen(&cam,[-100.0,0.0,0.0],vw,vh);let b=render3d::to_screen(&cam,[100.0,0.0,0.0],vw,vh);
            let r=render3d::map_frame(vw,vh);
            assert!(((b.x-a.x)/r.w-100.0/(render3d::ORTHO_SIZE*(16.0/9.0))).abs()<0.0001);
            eprintln!("viewport-check: {vw}x{vh}, uniform native framing and five input round trips passed");
        }
        shaders.forward = (cam.target - cam.position).normalize();
        if is_key_pressed(KeyCode::F1) && map_model.is_some() {
            view3d = !view3d;
        }

        // Input.
        // Hotkeys (keybinds.rs, rebindable in Settings > Hotkeys); none while a sandbox text
        // field is being typed in.
        binds.muted = hud.as_ref().is_some_and(|h| h.sandbox.typing());
        if game.sandbox {
            let mut acts = Vec::new();
            for (a, _, _) in keybinds::SANDBOX.iter() {
                if let (true, Some(h)) = (binds.pressed(a), hud.as_ref()) {
                    acts.push(h.sandbox.spawn(&a["send:".len()..]));
                }
            }
            for (a, s) in [
                ("clear_bloons", sandbox::SandboxAction::ClearBloons),
                ("clear_towers", sandbox::SandboxAction::ClearTowers),
                ("reset_damage", sandbox::SandboxAction::ResetDamage),
                ("reset_cooldowns", sandbox::SandboxAction::ResetCooldowns),
            ] {
                if binds.pressed(a) {
                    acts.push(s);
                }
            }
            for a in acts {
                if matches!(a, sandbox::SandboxAction::ClearTowers) {
                    selected = None;
                }
                a.apply(&mut game);
            }
        }
        // Play / fast forward: the Go button (start the round, or toggle fast forward).
        if binds.pressed("play") {
            if coop_runtime.is_some(){front.coop_input(if game.round_active{rrbtd6_coop::Action::Speed{value:if speed==1{3}else{1}}}else{rrbtd6_coop::Action::StartRound});}
            else if game.race.is_some(){game.request_race_round();}
            else if game.round_active || game.sandbox {
                speed = if speed == 1 { 3 } else { 1 };
            } else {
                game.start_round();
            }
        }
        if binds.pressed("send_next_round")&&game.race.is_some(){game.request_race_round();}
        // Tower special: Skywarden Focus / Spread stance; Monkey Bank collect.
        if let (true, Some(t)) = (binds.pressed("special"), selected) {
            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Special{tower:game.towers[t].id});}
            else{game.toggle_stance(t);game.collect_bank(t);}
        }
        // Dev keys (not BTD6's): F1 2D/3D view, F2 placement map, F3 next map, F4 next hero,
        // F5 next difficulty.
        if is_key_pressed(KeyCode::F2) {
            show_placement = !show_placement;
        }
        // Tower hotkeys: pick up that tower if it's in the shop.
        for (id, _) in keybinds::TOWER_DEFAULTS.iter() {
            if binds.pressed(&format!("tower:{id}")) && game.shop.iter().any(|s| s.id == *id) {
                if let Some(d) = game.tower_defs.iter().position(|(n, _)| n == id) {
                    placing = Some(d);
                    selected = None;
                }
            }
        }
        // Activated abilities 1-12 in the ability bar's order.
        let groups = game.ability_groups();
        for (k, g) in groups.iter().enumerate().take(12) {
            if binds.pressed(&format!("ability{}", k + 1)) && g.ready > 0 {
                if let Some(runtime)=&coop_runtime{if let Some(action)=runtime.replica.ability(&game,front.coop_slot(),&g.name){
                    if game.ability_targets_tower(&g.name)||game.ability_targets_point(&g.name){ability_pick=Some(g.name.clone());}else{front.coop_input(action);}
                }}
                else if game.ability_targets_tower(&g.name) || game.ability_targets_point(&g.name) {
                    ability_pick = Some(g.name.clone());
                } else {
                    game.activate_ability(&g.name);
                }
            }
        }
        // Hero: pick up the hero in the shop's hero slot.
        if binds.pressed("hero") && !game.towers.iter().any(|t| game.hero_def(t.def).is_some()) {
            let slot = game.shop.iter().find(|s| s.hero).map(|s| s.id.clone());
            if let Some(d) = (0..game.tower_defs.len()).find(|&d| Some(&game.tower_defs[d].0) == slot.as_ref()) {
                placing = Some(d);
            }
        }
        // D: next difficulty (Shift+D: previous), only before anything is done. Saved to settings.json.
        if coop_runtime.is_none()&&is_key_pressed(KeyCode::F5) && game.untouched() {
            let order = ["Easy", "Medium", "Hard", "Impoppable", "CHIMPS"];
            let cur = order.iter().position(|m| *m == game.mode_name).unwrap_or(1);
            let back = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
            let next = order[(cur + if back { order.len() - 1 } else { 1 }) % order.len()];
            save_setting(&root, "mode", serde_json::json!(next));
            game = Game::new(game_data(&root, &json, hero.as_deref(), &map_key));
            if let Some(config)=&race_config{if let Err(error)=game.configure_front_event(config){eprintln!("Race configuration: {error}");}}
            eprintln!("difficulty: {next}");
        }
        // N: next map (Shift+N: previous), only before anything is placed. Saved to settings.json.
        if coop_runtime.is_none()&&is_key_pressed(KeyCode::F3) && maps.len() > 1 && game.untouched() {
            let cur = maps.iter().position(|k| *k == map_key).unwrap_or(0);
            let back = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
            pending_map = Some(maps[(cur + if back { maps.len() - 1 } else { 1 }) % maps.len()].clone());
        }
        // H: next hero (Shift+H: previous), like choosing one before a game; only before
        // anything is placed. Saved to settings.json.
        if coop_runtime.is_none()&&is_key_pressed(KeyCode::F4) && game.untouched() {
            let ids: Vec<String> = game.heroes.iter().map(|h| h.id.clone()).collect();
            if !ids.is_empty() {
                let cur = game.shop.iter().find(|s| s.hero).and_then(|s| ids.iter().position(|h| *h == s.id)).unwrap_or(0);
                let back = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
                let next = (cur + if back { ids.len() - 1 } else { 1 }) % ids.len();
                hero = Some(ids[next].clone());
                save_setting(&root, "hero", serde_json::json!(ids[next]));
                game = Game::new(game_data(&root, &json, hero.as_deref(), &map_key));
            if let Some(config)=&race_config{if let Err(error)=game.configure_front_event(config){eprintln!("Race configuration: {error}");}}
                hud = load_work::complete_asset(Hud::load(&root, &game));
                placing = None;
                selected = None;
            }
        }
        if is_key_pressed(KeyCode::Escape) && placing.is_none() && selected.is_none()&&ability_pick.is_none()&&picking.is_none()&&item_pick.is_none() {
            open_menu = Some(menus::Menu::Pause);
        }
        if is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) {
            placing = None;
        }
        let preview=late_tower.filter(|(_,frame)|late_preview&&frames>=frame.saturating_sub(8)&&frames<*frame);
        if let Some((def,_))=preview{placing=Some(def);}
        let (mx, my) = mouse_position();
        let mouse_world = if preview.is_some(){[0.0,0.0]}else if view3d {
            render3d::pick_ground(&cam, vec2(mx, my), vw, vh).unwrap_or([1e9, 1e9])
        } else {
            game.pops.clear();
            game.bursts.clear();
            game.tracers.clear();
            view.to_world(vec2(mx, my))
        };
        let in_map = preview.is_some()||(render3d::map_frame(vw,vh).contains(vec2(mx,my)) && mx < screen_width() - Hud::panel_px() && !hud.as_ref().is_some_and(|h| h.blocks(vec2(mx, my))));
        if in_map && mouse_world[0] < 1e8 {
            if coop_runtime.is_some(){
                if get_time()-coop_mouse_at>=0.1{
                    front.coop_input(rrbtd6_coop::Action::Pointer{at:mouse_world});
                    if placing.is_none(){front.coop_input(rrbtd6_coop::Action::Collect{at:mouse_world});}
                    coop_mouse_at=get_time();
                }
            }else{game.pointer = Some(mouse_world);}
        }
        picking = picking.filter(|&t| t < game.towers.len() && placing.is_none());
        if picking.is_some() && (is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right)) {
            picking = None;
        }
        dragging &= placing.is_some();
        if ability_pick.is_some() && (is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) || placing.is_some()) {
            ability_pick = None;
        }
        if item_pick.is_some() && (is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) || placing.is_some()) {
            item_pick = None;
        }
        if let Some(i) = item_pick.filter(|_| in_map && is_mouse_button_pressed(MouseButton::Left)) {
            // A spot it can't be used keeps it in hand.
            if game.use_item(i, mouse_world) {
                item_pick = None;
            }
        } else if let Some(name) = ability_pick.clone().filter(|_| in_map && is_mouse_button_pressed(MouseButton::Left)) {
            if let Some(runtime)=&coop_runtime{
                if let Some(mut action)=runtime.replica.ability(&game,front.coop_slot(),&name){
                    if let rrbtd6_coop::Action::Ability{at,target,..}=&mut action{
                        if game.ability_targets_point(&name){*at=Some(mouse_world);}
                        else{*target=game.tower_at(mouse_world).map(|t|game.towers[t].id);}
                        if at.is_some()||target.is_some(){front.coop_input(action);}
                    }
                }
            }else if game.ability_targets_point(&name) {
                game.activate_ability_at(&name, mouse_world);
            } else if let Some(t) = game.tower_at(mouse_world) {
                game.activate_ability_on(&name, t);
                selected = Some(t);
            }
            ability_pick = None;
        } else if let Some(t) = picking.filter(|_| in_map && is_mouse_button_pressed(MouseButton::Left)) {
            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Point{tower:game.towers[t].id,at:mouse_world});}else{game.towers[t].point = Some(mouse_world);}
            picking = None;
        } else if let Some(def) = placing {
            let drop = if dragging { is_mouse_button_released(MouseButton::Left) } else { is_mouse_button_pressed(MouseButton::Left) };
            if drop&&in_map&&coop_runtime.is_some(){
                front.coop_input(rrbtd6_coop::Action::Place{kind:game.tower_defs[def].0.clone(),at:mouse_world});placing=None;
            } else if drop && in_map && game.place(def, mouse_world) {
                placing = None;
                let t = game.towers.len() - 1;
                selected = Some(t);
                // Like the game, a tower that only fires at a set point asks for it straight away.
                if game.target_modes(t) == [game::TargetMode::Point] {
                    picking = Some(t);
                }
            } else if drop && dragging {
                // Dropped on the HUD or a spot it can't go: cancelled, nothing stays in hand.
                placing = None;
            }
            dragging &= !drop;
        } else if in_map && is_mouse_button_pressed(MouseButton::Left) {
            let was = selected;
            selected = game.tower_at(mouse_world);
            // CreateSoundOnSelected: a hero says something when clicked.
            if let Some(t) = selected.filter(|&t| was != Some(t)) {
                game.select_voice(t);
            }
        }
        if placing.is_some() {
            selected = None;
        }
        if selected.is_some_and(|t| t >= game.towers.len()) {
            selected = None;
        }
        // Upgrade path hotkeys, change targeting (Shift: backwards), sell.
        if let Some(t) = selected {
            for (a, path) in [("upgrade1", 0), ("upgrade2", 1), ("upgrade3", 2)] {
                if binds.pressed(a) {
                    if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Upgrade{tower:game.towers[t].id,path:path as u8});}
                    else if game.towers[t].tiers[path] == 5 && game.paragon_quote(t).is_some() {
                        if let Some(h) = hud.as_mut() {
                            h.menus.paragon_tower = t;
                        }
                        open_menu = Some(menus::Menu::Paragon);
                    } else if !game.temple_prompt(t, path).is_some_and(|two| hud.as_mut().is_some_and(|h| h.open_temple(t, path, two))) {
                        game.upgrade(t, path);
                    }
                }
            }
            if binds.pressed("targeting") {
                let back = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
                let mode=game.cycle_target(t, back);
                if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Target{tower:game.towers[t].id,mode:coop_game::target_code(mode)});}else{game.towers[t].target=mode;}
            }
            if binds.pressed("sell") {
                if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Sell{tower:game.towers[t].id});}else{game.sell(t);}
                selected = None;
            }
        }
        // Bananas are collected by moving the cursor over them.
        if coop_runtime.is_none()&&in_map && placing.is_none() {
            game.collect_cash(mouse_world);
        }
        // RRBTD6_COLLECT: autotests pick up every landed banana.
        if autotest.is_some() && std::env::var("RRBTD6_COLLECT").is_ok() {
            let at: Vec<[f32; 2]> = game.projectiles.iter().filter(|p| p.spec.cash.is_some()).map(|p| p.pos).collect();
            for p in at {
                game.collect_cash(p);
            }
        }

        if autotest.is_some()&&game.race.is_some(){
            if let Some(every)=std::env::var("RRBTD6_RACE_SEND_EVERY").ok().and_then(|v|v.parse::<u32>().ok()).filter(|n|*n>0){
                if frames%every==0{game.request_race_round();}
            }
        }
        // Fixed-step simulation (P-TIME).
        if coop_runtime.is_none(){
            if let Some(steps)=bench_steps{acc=if bench_round.is_some()&&frames<bench_warmup{0.0}else{(steps as f32+0.01)/FPS};}else{acc += get_frame_time().min(0.25) * speed as f32;}
        }
        while acc >= 1.0 / FPS {
            let t_step = std::time::Instant::now();
            let completed_before=game.round;
            game.step();
            if game.round!=completed_before && !game.sandbox && game.race.is_none() && std::env::var_os("RRBTD6_AUTOTEST").is_none() && frontend::account_preview::is_local() {
                let profile_root=root.clone();let completed_round=game.round;
                std::thread::spawn(move||{if local_profile::update(&profile_root,|p|p.highest_round=p.highest_round.max(completed_round)).is_ok(){frontend::account_preview::refresh_local(&profile_root);}});
            }
            let dt_step = t_step.elapsed().as_secs_f64();
            sim_stats.0 += dt_step;
            sim_stats.1 += 1;
            sim_stats.2 = sim_stats.2.max(dt_step);
            // Like the BTD6 setting: the first round is started by hand.
            if auto_start && game.race.is_none() && !game.sandbox && game.round > 0 {
                game.start_round();
            }
            acc -= 1.0 / FPS;
        }
        // Expired sentries shift the indices after them.
        for r in game.removed.drain(..) {
            let fix = |s: Option<usize>| match s {
                Some(s) if s == r => None,
                Some(s) if s > r => Some(s - 1),
                s => s,
            };
            selected = fix(selected);
            picking = fix(picking);
        }
        // M mutes / unmutes the music.
        if is_key_pressed(KeyCode::F6) {
            audio.set_music_volume(if audio.music_volume > 0.0 { 0.0 } else { 0.4 });
        }
        audio.play(&mut game.sfx, get_time(), speed > 1);
        audio.music(get_time());

        // Background loads: the models the selected tower's next upgrades show, and the
        // tower in hand's first tier, so upgrading / placing doesn't wait on the disk.
        if retire_predictions{
            let picked=selected.filter(|&t|t<game.towers.len());
            let key=(picked.map(|t|{let t=&game.towers[t];(t.id,t.def,t.tiers,t.paragon.as_ref().map(|p|p.0))}),placing);
            if prediction_key!=Some(key){
                prediction_key=Some(key);predicted_models.clear();predicted_effects.clear();
                if let Some(t)=picked{
                    let tw=&game.towers[t];
                    let hero=game.hero_def(tw.def).is_some();
                    for path in 0..3{
                        if game.next_upgrade(t,path).is_none(){continue;}
                        let effect=match (hero,tw.tiers[path]){
                            (true,19..)=>"HeroUpgradeEffectLv20",(true,_)=>"HeroUpgradeEffect",
                            (false,4..)=>"UpgradeEffectTier5",_=>"UpgradeEffect",
                        };
                        if !predicted_effects.contains(&effect){predicted_effects.push(effect);}
                        if !hero&&tw.tiers[path]>=5{continue;}
                        let mut tiers=tw.tiers;tiers[path]+=1;
                        if let Some(dir)=tier_index[tw.def].get(&format!("{}{}{}",tiers[0],tiers[1],tiers[2])){
                            predicted_models.push(((tw.def,dir.clone()),root.join("models").join(&game.tower_defs[tw.def].0).join("tiers").join(dir)));
                        }
                    }
                }
                if let Some(d)=placing{
                    if let Some(dir)=tier_index[d].get("000").or(tier_index[d].get("100")){
                        predicted_models.push(((d,dir.clone()),root.join("models").join(&game.tower_defs[d].0).join("tiers").join(dir)));
                    }
                }
            }
            predicted_models.retain(|(key,_)|!tier_anims.contains_key(key));
            prefetch.retire(|path|path.parent()==Some(bloon_model_root.as_path())||predicted_models.iter().any(|(_,wanted)|wanted==path));
            for (_,path) in &predicted_models{prefetch.request_path(path);}
        }else if let Some(t) = selected.filter(|&t| t < game.towers.len()) {
            let tw = &game.towers[t];
            for path in 0..3 {
                let mut k = tw.tiers;
                k[path] += 1;
                if let Some(dir) = tier_index[tw.def].get(&format!("{}{}{}", k[0], k[1], k[2])) {
                    if !tier_anims.contains_key(&(tw.def, dir.clone())) {
                        prefetch.request(root.join("models").join(&game.tower_defs[tw.def].0).join("tiers").join(dir));
                    }
                }
            }
        }
        // The model in hand: tier 000 (heroes: level 1, "100"), loaded now so it shows at once.
        if let Some(d) = placing {
            if let Some(dir) = tier_index[d].get("000").or(tier_index[d].get("100")).cloned() {
                if !tier_anims.contains_key(&(d, dir.clone())) {
                    let path = root.join("models").join(&game.tower_defs[d].0).join("tiers").join(&dir);
                    tier_anims.insert((d, dir), prefetch.load(&path));
                }
            }
        }
        if view3d {
            for t in &game.towers {
                for m in t.fan.as_ref().and_then(|(f, _)| f.display.as_ref()).into_iter().chain(t.look.as_ref().map(|l| &l.0)).chain(t.morph.as_ref().and_then(|m| m.1.as_ref())) {
                    if !fan_models.contains_key(m) {
                        fan_models.insert(m.clone(), prefetch.load(&root.join("models").join("fans").join(m)));
                    }
                }
                if let Some(m) = game.tier_of(t).and_then(|tr| tr.air.as_ref()).and_then(|a| a.display.as_ref()) {
                    if !air_models.contains_key(m) {
                        air_models.insert(m.clone(), prefetch.load(&root.join("models").join("air").join(m)));
                    }
                }
                if let Some(s) = &t.sub {
                    for m in [&s.spec.base, &s.spec.model].into_iter().flatten() {
                        if !sub_models.contains_key(m) {
                            sub_models.insert(m.clone(), prefetch.load(&root.join("models").join("subtowers").join(m)));
                        }
                    }
                    continue;
                }
                // A Paragon also has its attack display on top ("paragon_top").
                let (base, top) = model_keys(&game, t, &tier_index[t.def]);
                for (dir, _, _) in attack_index[t.def].get(&base).into_iter().flatten() {
                    let key = (t.def, dir.clone());
                    if !tier_anims.contains_key(&key) {
                        let path = root.join("models").join(&game.tower_defs[t.def].0).join("tiers").join(dir);
                        tier_anims.insert(key, prefetch.load(&path));
                    }
                }
                for k in std::iter::once(base).chain(top) {
                    if let Some(dir) = tier_index[t.def].get(&k) {
                        let key = (t.def, dir.clone());
                        if !tier_anims.contains_key(&key) {
                            let path = root.join("models").join(&game.tower_defs[t.def].0).join("tiers").join(dir);
                            tier_anims.insert(key, prefetch.load(&path));
                        }
                    }
                }
            }
            // Tier/skin art above is the real display. Load base fallback art only
            // when that display is unavailable, preserving the original fallback.
            for i in 0..game.towers.len(){
                let t=&game.towers[i];if t.sub.is_some(){continue;}
                let ready=tier_index[t.def].get(&model_keys(&game,t,&tier_index[t.def]).0)
                    .and_then(|dir|tier_anims.get(&(t.def,dir.clone()))).is_some_and(|m|m.is_some());
                if keep_fallback_models||!ready{ensure_tower!(game.towers[i].def);}
            }
            if let Some(d)=placing{
                let ready=tier_index[d].get("000").or(tier_index[d].get("100"))
                    .and_then(|dir|tier_anims.get(&(d,dir.clone()))).is_some_and(|m|m.is_some());
                if keep_fallback_models||!ready{ensure_tower!(d);}
            }
            for b in &game.bloons {
                let d = &game.defs[b.kind];
                if let Some(m) = d.model_at(b.hp as f32 / d.max_health.max(1) as f32) {
                    if !bloon_models.contains_key(m) {
                        let path = root.join("models").join("bloons").join(m);
                        bloon_models.insert(m.to_string(), prefetch.load(&path));
                    }
                }
            }
            // Render scale: the 3D view goes to an offscreen target of the scaled size (with
            // depth), drawn to the screen after (sharpened if on).
            let scaled = adv.render_scale != 1.0;
            if scaled {
                let (w, h) = (((vw * screen_dpi_scale() * adv.render_scale) as u32).max(1), ((vh * screen_dpi_scale() * adv.render_scale) as u32).max(1));
                if view_rt.as_ref().is_none_or(|r| r.1 != w || r.2 != h) {
                    let rt = render_target_ex(w, h, RenderTargetParams { sample_count: 1, depth: true });
                    rt.texture.set_filter(FilterMode::Linear);
                    view_rt = Some((rt, w, h));
                }
                set_camera(&Camera3D {
                    position: cam.position,
                    target: cam.target,
                    up: cam.up,
                    fovy: cam.fovy,
                    aspect: cam.aspect,
                    projection: cam.projection,
                    z_near: cam.z_near,
                    z_far: cam.z_far,
                    render_target: view_rt.as_ref().map(|r| r.0.clone()),
                    viewport: None,
                    ..Default::default()
                });
                clear_background(Color::from_rgba(30, 40, 30, 255));
            } else {
                set_camera(&cam);
            }
            if let Some(m) = &map_model {
                m.draw_map(if scaled{None}else{cam.viewport});
            }
            // Tower ground shadows, under the range circles.
            for t in &game.towers {
                let (key,_)=model_keys(&game,t,&tier_index[t.def]);
                if let Some(a) = &t.air {
                    let m = game.tier_of(t).and_then(|tr| tr.air.as_ref()).and_then(|s| s.display.as_ref());
                    if let Some(m) = m.and_then(|m| air_models.get(m)).and_then(|m| m.as_ref()) {
                        m.draw_shadow([a.pos[0], 0.0, a.pos[1]], heli_yaw(t, a));
                    }
                }
                if let Some(s) = &t.sub {
                    if let Some(m) = s.spec.base.as_ref().and_then(|b| sub_models.get(b)).and_then(|m| m.as_ref()) {
                        m.draw_shadow([t.pos[0], 0.0, t.pos[1]], std::f32::consts::PI);
                    }
                    continue;
                }
                let model = tier_index[t.def].get(&key).and_then(|dir| tier_anims.get(&(t.def, dir.clone()))).and_then(|m| m.as_ref());
                // A base under turning attack models (Sun Temple, True Sun God, Dartling pads)
                // whose attacks never turn the tower keeps the placement facing, whatever an
                // earlier tier (Sun Avatar) left it at.
                let fixed_base = attack_index[t.def].contains_key(&key)
                    && game.tier_of(t).is_some_and(|tr| !tr.attacks.is_empty() && tr.attacks.iter().all(|a| !a.turn_body));
                let yaw = if t.air.is_some() || fixed_base { std::f32::consts::PI } else { std::f32::consts::FRAC_PI_2 - t.angle };
                if let Some(m) = model.or(tower_anims[t.def].as_ref()) {
                    m.draw_shadow([t.pos[0], 0.0, t.pos[1]], yaw);
                }
            }
            if let (Some(def), true) = (placing, in_map) {
                let in_hand=tier_index[def].get("000").or(tier_index[def].get("100"));
                let model=in_hand.and_then(|dir|tier_anims.get(&(def,dir.clone()))).and_then(|m|m.as_ref());
                if let Some(m) = model.or(tower_anims[def].as_ref()) {
                    m.draw_shadow([mouse_world[0], 0.0, mouse_world[1]], std::f32::consts::PI);
                }
            }
            // Range circles lie on the ground, under towers and bloons.
            if let (Some(def), true) = (placing, in_map) {
                let ok = game.can_place(def, mouse_world) && game.cash >= game.place_cost(def, mouse_world);
                render3d::range_disc(mouse_world[0], mouse_world[1], placement_radius(&game.tower_defs[def].1), ok);
            }
            if let Some(t) = selected {
                let p = game.attack_pos(t);
                render3d::range_disc(p[0], p[1], game.range(t), true);
            }
            if let (Some(q), Some(tx)) = (marker_at(&game, picking, selected, in_map, mouse_world), &mortar_target) {
                render3d::flat_sprite(tx, q[0], 0.4, q[1], MARKER_SIZE, MARKER_SIZE, [0.5, 0.5], [0.0, 1.0]);
            }
            for (ti, t) in game.towers.iter().enumerate() {
                let (key,top_key)=model_keys(&game,t,&tier_index[t.def]);
                // A Heli's pad keeps its placement facing; the Heli itself turns.
                // A base under turning attack models (Sun Temple, True Sun God, Dartling pads)
                // whose attacks never turn the tower keeps the placement facing, whatever an
                // earlier tier (Sun Avatar) left it at.
                let fixed_base = attack_index[t.def].contains_key(&key)
                    && game.tier_of(t).is_some_and(|tr| !tr.attacks.is_empty() && tr.attacks.iter().all(|a| !a.turn_body));
                let yaw = if t.air.is_some() || fixed_base { std::f32::consts::PI } else { std::f32::consts::FRAC_PI_2 - t.angle };
                // Blink ~every 4 s for 0.12 s, phase-shifted per tower.
                let blink = (get_time() as f32 + ti as f32 * 1.37) % 4.0 < 0.12;
                // A submerged Sub sinks by SubmergeModel.submergeDepth.
                let sink = match game.tier_of(t).and_then(|tr| tr.submerge.as_ref()) {
                    Some(sm) if t.submerged => sm.depth,
                    _ => 0.0,
                };
                // Towers on a ship's deck stand at the deck's height.
                let pos = [t.pos[0], t.deck.map_or(0.0, |d| d.1) - sink, t.pos[1]];
                let sel_outline = (selected == Some(ti)).then_some([255u8; 4]);
                if let Some(a) = &t.air {
                    // STAND-IN P-HELIHEIGHT: the Heli hovers at height 40, moved back along the
                    // camera tilt like the MOABs so it shows over its sim position.
                    let h = a.height;
                    let ap = [a.pos[0], h, a.pos[1] - h / 60f32.to_radians().tan()];
                    let m = game.tier_of(t).and_then(|tr| tr.air.as_ref()).and_then(|s| s.display.as_ref());
                    match m.and_then(|m| air_models.get_mut(m)).and_then(|m| m.as_mut()) {
                        Some(m) => {
                            let clip = match m.attack_clip() {
                                Some(c) if t.attack_t < m.clip_duration(c) => (c, t.attack_t),
                                _ => (m.idle_clip(), get_time() as f32),
                            };
                            // PathMovement's bank and take-off pitch / scale (planes; a Heli has none).
                            m.tilt = Some((a.pitch.to_radians(), a.bank.to_radians(), a.scale));
                            m.draw(&shaders, clip.0, clip.1, ap, heli_yaw(t, a), outline_scale, false, sel_outline);
                            m.tilt = None;
                        }
                        None => draw_cube(render3d::gl(ap[0], ap[1], ap[2]), vec3(8.0, 4.0, 8.0), None, GRAY),
                    }
                }
                if let Some(s) = &t.sub {
                    // The stand keeps its placement facing; the turret turns to aim.
                    let aim_yaw = std::f32::consts::FRAC_PI_2 - t.aim;
                    for (m, yaw) in [(&s.spec.base, std::f32::consts::PI), (&s.spec.model, aim_yaw)] {
                        if let Some(m) = m.as_ref().and_then(|m| sub_models.get_mut(m)).and_then(|m| m.as_mut()) {
                            let clip = match m.attack_clip() {
                                Some(a) if t.attack_t < m.clip_duration(a) => (a, t.attack_t),
                                _ => (m.idle_clip(), t.idle_t),
                            };
                            m.draw(&shaders, clip.0, clip.1, pos, yaw, outline_scale, false, None);
                        }
                    }
                    if t.air.is_none() && s.spec.base.as_ref().and_then(|b| sub_models.get(b)).map_or(true, |m| m.is_none()) {
                        draw_cube(render3d::gl(t.pos[0], 3.0, t.pos[1]), vec3(6.0, 6.0, 6.0), None, GRAY);
                    }
                    continue;
                }
                // Total Transformation's monster, else a display swap, else a fan.
                let look = t.morph.as_ref().and_then(|m| m.1.as_ref()).or(t.look.as_ref().map(|l| &l.0)).or(t.fan.as_ref().and_then(|(f, _)| f.display.as_ref()));
                let fan_model = look.and_then(|m| fan_models.get_mut(m)).and_then(|m| m.as_mut());
                let tier_model = tier_index[t.def]
                    .get(&key)
                    .and_then(|dir| tier_anims.get_mut(&(t.def, dir.clone())))
                    .and_then(|m| m.as_mut());
                // Attack plays once after each throw, otherwise the idle loop.
                // A one-off the game triggered (the temple opening) plays over everything else.
                let shown = |m: &anim::AnimModel| {
                    let (name, at) = t.show.as_ref()?;
                    m.clip_named(name).filter(|&c| *at < m.clip_duration(c)).map(|c| (c, *at))
                };
                // The body keeps its idle pick (variants) between frames; other parts just idle.
                let mut pick = idle_picks.get(&t.id).copied().unwrap_or((usize::MAX, 0.0));
                let attacked = t.attack_t < f32::MAX;
                let mut pose = |m: &mut anim::AnimModel, body: bool| match m.attack_clip() {
                    _ if shown(m).is_some() => {
                        let (c, at) = shown(m).unwrap();
                        m.meshes(c, at, pos, yaw, outline_scale, blink, sel_outline)
                    }
                    Some(a) if t.attack_t < m.clip_duration(a) => m.meshes(a, t.attack_t, pos, yaw, outline_scale, blink, sel_outline),
                    _ => {
                        let mut other = (usize::MAX, 0.0);
                        let (c, at) = m.idle_pose(attacked, t.idle_t, if body { &mut pick } else { &mut other });
                        m.meshes(c, at, pos, yaw, outline_scale, blink, sel_outline)
                    }
                };
                if let Some(m) = fan_model.or(tier_model).or(tower_anims[t.def].as_mut()) {
                    let (body, outline) = pose(m, true);
                    // The Paragon's attack display, on top with the same facing; both outlines
                    // go under both bodies.
                    let top = top_key.as_ref().and_then(|k| tier_index[t.def].get(k)).cloned();
                    let (tb, to) = match top.and_then(|dir| tier_anims.get_mut(&(t.def, dir))).and_then(|m| m.as_mut()) {
                        Some(m) => pose(m, false),
                        None => (Vec::new(), Vec::new()),
                    };
                    // Attack displays (Dartling gun, Super Monkey arm...): on the tower, turned to
                    // the attack's aim, animating with its throws.
                    let (mut ab, mut ao) = (Vec::new(), Vec::new());
                    for (dir, off, ai) in attack_index[t.def].get(&key).into_iter().flatten() {
                        // Each attack display turns with its own attack (Robo Monkey's arms).
                        let aim_yaw = std::f32::consts::FRAC_PI_2 - t.aims.get(*ai).map_or(t.aim, |a| a.0);
                        if let Some(m) = tier_anims.get_mut(&(t.def, dir.clone())).and_then(|m| m.as_mut()) {
                            let (s, c) = aim_yaw.sin_cos();
                            let p = [pos[0] + off[0] * c + off[2] * s, pos[1] + off[1], pos[2] - off[0] * s + off[2] * c];
                            let (b, o) = match m.attack_clip() {
                                _ if shown(m).is_some() => {
                                    let (c, at) = shown(m).unwrap();
                                    m.meshes(c, at, p, aim_yaw, outline_scale, blink, sel_outline)
                                }
                                Some(a) if t.attack_t < m.clip_duration(a) => m.meshes(a, t.attack_t, p, aim_yaw, outline_scale, blink, sel_outline),
                                _ => {
                                    let (c, at) = m.idle_pose(attacked, t.idle_t, &mut (usize::MAX, 0.0));
                                    m.meshes(c, at, p, aim_yaw, outline_scale, blink, sel_outline)
                                }
                            };
                            ab.extend(b);
                            ao.extend(o);
                        }
                    }
                    anim::draw_meshes(&shaders, &[&outline, &to, &ao], &[&body, &tb, &ab]);
                    idle_picks.insert(t.id, pick);
                    continue;
                }
                match &tower_models[t.def] {
                    Some(m) => m.draw_at(pos, yaw, &mut scratch),
                    None => draw_sphere(render3d::gl(t.pos[0], 6.0, t.pos[1]), 6.0, None, BROWN),
                }
            }
            let bloon_t0 = get_time();
            // Bloons: far (larger z) first.
            // Positions computed once (not inside the sort comparator), then far-to-near.
            bloon_order.clear();
            bloon_order.extend(game.bloons.iter().enumerate().map(|(i, b)| (game.bloon_pos(b), i)));
            bloon_order.sort_unstable_by(|a, b| b.0[1].total_cmp(&a.0[1]).then_with(||game.bloons[a.1].id.cmp(&game.bloons[b.1].id)));
            // MOAB-class models: the one furthest along the track draws over every one behind it
            // (bunched-up MOABs show the leader whole). Each is pushed toward the camera by its
            // rank; with the orthographic camera that only changes which is in front.
            const MOAB_STEP: f32 = 45.0;
            moab_rank.clear();
            {
                moab_order.clear();moab_order.extend((0..game.bloons.len()).filter(|&i|game.defs[game.bloons[i].kind].model.is_some()));
                let ms=&mut moab_order;
                ms.sort_unstable_by(|&a, &b| game.bloons[a].progress().total_cmp(&game.bloons[b].progress()).then_with(||game.bloons[a].id.cmp(&game.bloons[b].id)));
                let n = ms.len();
                for (k, &i) in ms.iter().enumerate() {
                    moab_rank.insert(i, (k + 100).saturating_sub(n) as f32);
                }
            }
            for &(p, i) in &bloon_order {
                let b = &game.bloons[i];
                let d = &game.defs[b.kind];
                let model = d
                    .model_at(b.hp as f32 / d.max_health.max(1) as f32)
                    .and_then(|m| bloon_models.get_mut(m))
                    .and_then(|m| m.as_mut());
                if let Some(m) = model {
                    sprite_batch.flush(); // keep painter's order around 3D models
                    // Path.DistanceToPointInfoBloon: MOAB-class displays sit at height 120, moved
                    // back along the camera tilt like the 2D bloons; BloonModel.rotateToFollowPath
                    // turns them along the track.
                    let h = 120.0;
                    // b.pos already has the bloon's 4 off the path line (Track::bloon_pos).
                    let ahead = game.tracks[b.path as usize].bloon_pos(b.dist + 1.0);
                    let heading = (ahead[1] - p[1]).atan2(ahead[0] - p[0]);
                    let lift = moab_rank.get(&i).copied().unwrap_or(0.0) * MOAB_STEP;
                    let a = render3d::PITCH_DEG.to_radians();
                    let pos = [p[0], h + lift * a.sin(), p[1] - h / 60f32.to_radians().tan() - lift * a.cos()];
                    let idle = m.idle_clip();
                    m.draw(&shaders, idle, get_time() as f32, pos, std::f32::consts::FRAC_PI_2 - heading, outline_scale, false, None);
                    continue;
                }
                match &bloon_tex[b.kind] {
                    // Display height: Path.DistanceToPointInfoBloon gives y -= 4, height +20.
                    Some(t) => match d.display.first() {
                        // Sim position: y -= 4, height 20 (Path.DistanceToPointInfoBloon); Unity from
                        // sim via inverse ToSimulationSpace(tilt 60): u.z = -s.y - s.z / tan(60).
                        // STAND-IN tints for the game's frozen / glued bloon overlays.
                        Some(l) => {
                            let tint = if b.frozen() {
                                [150, 210, 255, 255]
                            } else if b.glue() > 0 {
                                [255, 240, 120, 255]
                            } else {
                                [255; 4]
                            };
                            let y = p[1] - 20.0 / 60f32.to_radians().tan();
                            match (&atlas, bloon_uv[b.kind]) {
                                (Some(a), Some(uv)) => sprite_batch.billboard_uv(&a.tex, uv, p[0], y, 20.0, l.w, l.h, tint),
                                _ => sprite_batch.billboard(t, p[0], y, 20.0, l.w, l.h, tint),
                            }
                        }
                        None => {
                            sprite_batch.flush();
                            render3d::billboard(t, p[0], p[1], 2.0, d.radius * 1.9)
                        }
                    },
                    None => draw_sphere(render3d::gl(p[0], d.radius, p[1]), d.radius, None, RED),
                }
            }
            sprite_batch.flush();
            bloon_time += get_time() - bloon_t0;
            let tilt = 1.0 / 60f32.to_radians().tan();
            // Projectile displays seen for the first time this frame (projectiles and orbits).
            let wanted: Vec<String> = game
                .projectiles
                .iter()
                .flat_map(|p| p.spec.pierce_sprites.iter().chain(p.spec.display.iter()))
                .chain((0..game.towers.len()).filter_map(|t| game.tier(t).and_then(|tr| tr.orbit.as_ref()).and_then(|o| o.display.as_ref())))
                .filter_map(|g| proj_index.get(g))
                .filter(|n| !proj_tried.contains(*n))
                .cloned()
                .collect();
            for n in wanted {
                if proj_tried.insert(n.clone()) {
                    let display=if projectile_upload.as_ref().is_some_and(|(name,_)|name==&n){load_work::complete_asset(projectile_upload.take().unwrap().1)}
                        else{load_work::complete_asset(load_proj(proj_root.join(&n)))};
                    if let Some(d) = display {
                        proj_displays.insert(n, d);
                    }
                }
            }
            for p in &game.projectiles {
                // Display height 10 (HYPOTHESIS), tilt-corrected like bloons.
                let (x, z) = (p.pos[0], p.pos[1] - 10.0 * tilt);
                // SetSpriteFromPierceModel (spike piles; entry i shows i + 1 spikes). HYPOTHESIS
                // P-PIERCESPRITE: entry ceil(pierce left) - 1, clamped to the list (loopMode Clamp).
                let n = p.spec.pierce_sprites.len();
                // Avatar of Wrath's bolts look stronger as Wrath grows (weak / medium / epic).
                let wrath_display = p.spec.wrath.as_ref().and_then(|_| {
                    let o = game.towers.iter().position(|o| o.id == p.owner)?;
                    let (lvl, v) = game.wrath_level(o)?;
                    v.displays.get(lvl).cloned().flatten()
                });
                let display = if n > 0 {
                    let k = p.pierce.ceil() as usize;
                    p.spec.pierce_sprites.get(k.clamp(1, n) - 1)
                } else {
                    wrath_display.as_ref().or(p.spec.display.as_ref())
                };
                // Ring and explosion hits have no projectile graphic (drawn as bursts).
                if p.spec.link_radius || display.map(String::as_str) == Some("") {
                    continue;
                }
                // Landed spikes and bananas lie on the ground.
                let (y, z) = if (n > 0 || p.spec.cash.is_some()) && p.vel == [0.0, 0.0] { (1.0, p.pos[1] - tilt) } else { (10.0, z) };
                // RotateModel spins the display (boomerangs) instead of facing the flight direction.
                let heading = if p.spec.spin != 0.0 { (p.spec.spin * p.age / FPS).to_radians() } else { p.vel[1].atan2(p.vel[0]) };
                match display.and_then(|g| proj_index.get(g)).and_then(|n| proj_displays.get(n)) {
                    // A plain sprite is a stand-in for a particle-only prefab: skipped when its
                    // real particle effect plays (real sprite prefabs load as Layers).
                    Some(ProjDisplay::Sprite { .. }) if display.and_then(|g| proj_index.get(g)).is_some_and(|n| proj_fx.contains_key(n)) => continue,
                    Some(ProjDisplay::Sprite { tex, w, h, pivot }) => {
                        render3d::flat_sprite(tex, x, y, z, *w, *h, *pivot, [heading.cos(), heading.sin()]);
                        continue;
                    }
                    Some(ProjDisplay::Layers(ls)) => {
                        draw_layers(ls, x, y, z, heading, p.age / FPS);
                        continue;
                    }
                    Some(ProjDisplay::Mesh(m)) => {
                        m.draw_at([x, y, z], std::f32::consts::FRAC_PI_2 - heading, &mut scratch);
                        continue;
                    }
                    // STAND-IN for particle-only area projectiles (Wall of Fire): a fiery disc.
                    None if p.vel == [0.0, 0.0] && p.frames_left > 6.0 => {
                        let r = p.spec.radius;
                        render3d::ground_band(p.pos[0], p.pos[1], r, r, Color::new(1.0, 0.75, 0.2, 0.35), Color::new(1.0, 0.3, 0.0, 0.55));
                        continue;
                    }
                    None => {}
                }
                let at = render3d::gl(x, 10.0, z);
                let dir = render3d::gl(p.vel[0], 0.0, p.vel[1]);
                match &dart {
                    Some((m, axis)) => render3d::draw_oriented(m, *axis, at, dir, 1.0),
                    None => {
                        let v = vec2(p.vel[0], p.vel[1]).normalize_or_zero() * 5.0;
                        draw_line_3d(render3d::gl(p.pos[0] - v.x, 10.0, p.pos[1] - v.y), render3d::gl(p.pos[0] + v.x, 10.0, p.pos[1] + v.y), BLACK);
                    }
                }
            }
            // Glaive Lord's orbiting glaives. STAND-IN: OrbitModel has no speed field; the turn
            // rate (1 turn/s) and the glaive spin are guesses.
            for t in 0..game.towers.len() {
                let Some(orbit) = game.tier(t).and_then(|tier| tier.orbit.clone()) else { continue };
                let Some(disp) = orbit.display.as_ref().and_then(|g| proj_index.get(g)).and_then(|n| proj_displays.get(n)) else {
                    continue;
                };
                let c = game.towers[t].pos;
                let turn = get_time() as f32 * std::f32::consts::TAU;
                for k in 0..orbit.count {
                    let a = turn + k as f32 * std::f32::consts::TAU / orbit.count as f32;
                    let (x, z) = (c[0] + orbit.range * a.cos(), c[1] + orbit.range * a.sin() - 10.0 * tilt);
                    let spin = -turn * 4.0;
                    match disp {
                        ProjDisplay::Sprite { tex, w, h, pivot } => render3d::flat_sprite(tex, x, 10.0, z, *w, *h, *pivot, [spin.cos(), spin.sin()]),
                        ProjDisplay::Mesh(m) => m.draw_at([x, 10.0, z], std::f32::consts::FRAC_PI_2 - spin, &mut scratch),
                        ProjDisplay::Layers(ls) => draw_layers(ls, x, 10.0, z, spin, get_time() as f32),
                    }
                }
            }
            // Pops: flipbook at the bloon's display position.
            let now = get_time() as f32;
            pops.extend(game.pops.drain(..).map(|p| (p, now)));
            pops.retain(|(_, t0)| now - t0 < pop_life.max(0.05) * (speed as f32).recip() + 0.2);
            // Batched per flipbook frame (one draw call per frame texture, not per pop).
            for k in 0..pop_frames.len() {
                let (t, w, h) = &pop_frames[k];
                for (p, t0) in &pops {
                    let age = (now - t0) * speed as f32;
                    if age > pop_life || ((age * pop_fps) as usize).min(pop_frames.len() - 1) != k {
                        continue;
                    }
                    sprite_batch.billboard(t, p[0], p[1] + 4.0 - 20.0 * tilt, 20.5, w * pop_scale, h * pop_scale, [255; 4]);
                }
            }
            sprite_batch.flush();
            // Bomb explosions and Ring of Fire bursts. STAND-IN: the game's effects (bomb explosion,
            // RingOfFireBurst) are particle systems, not exported; drawn as expanding, fading discs.
            bursts.extend(game.bursts.drain(..).map(|b| (b, now)));
            bursts.retain(|(_, t0)| (now - t0) * (speed as f32) < BURST_LIFE);
            for (b, t0) in &bursts {
                let k = (now - t0) * speed as f32 / BURST_LIFE;
                let a = 1.0 - k;
                if b.ice {
                    // STAND-IN for the Ice Monkey's frost blast particles.
                    let (x, z, rr) = (b.pos[0], b.pos[1], b.radius * (0.6 + 0.4 * k));
                    render3d::ground_band(x, z, rr, rr, Color::new(0.85, 0.95, 1.0, 0.35 * a), Color::new(0.55, 0.85, 1.0, 0.6 * a));
                    render3d::ground_band(x, z, rr + 2.0, 2.0, Color::new(0.55, 0.85, 1.0, 0.6 * a), Color::new(1.0, 1.0, 1.0, 0.0));
                } else if b.fire {
                    let (x, z, rr) = (b.pos[0], b.pos[1], b.radius * (0.55 + 0.45 * k));
                    render3d::ground_band(x, z, rr, rr * 0.35, Color::new(1.0, 0.85, 0.2, 0.0), Color::new(1.0, 0.45, 0.05, 0.85 * a));
                    render3d::ground_band(x, z, rr + 2.0, 2.0, Color::new(1.0, 0.45, 0.05, 0.85 * a), Color::new(0.9, 0.15, 0.0, 0.0));
                } else {
                    // At projectile height, tilt-corrected like the bombs that cause them.
                    let (x, z, rr) = (b.pos[0], b.pos[1] - 10.0 * tilt, b.radius * (0.5 + 0.5 * k.sqrt()));
                    render3d::ground_band(x, z, rr, rr, Color::new(1.0, 0.95, 0.6, 0.9 * a), Color::new(1.0, 0.5, 0.1, 0.6 * a));
                    render3d::ground_band(x, z, rr + 2.5, 2.5, Color::new(1.0, 0.5, 0.1, 0.6 * a), Color::new(0.4, 0.2, 0.1, 0.0));
                }
            }
            // Sniper shots. STAND-IN: the game shows a muzzle flash (EjectEffectModel) and, for
            // bounces, a CreateEffectFromCollisionToCollision streak; drawn as a fading tracer.
            tracers.extend(game.tracers.drain(..).map(|s| (s, now)));
            tracers.retain(|(_, t0)| (now - t0) * (speed as f32) < TRACER_LIFE);
            for ((a, b), t0) in &tracers {
                let k = 1.0 - (now - t0) * speed as f32 / TRACER_LIFE;
                let (a, b) = ([a[0], a[1] - 10.0 * tilt], [b[0], b[1] - 10.0 * tilt]);
                render3d::flat_segment(a, b, 10.0, 1.2, Color::new(1.0, 0.9, 0.5, 0.15 * k), Color::new(1.0, 1.0, 0.85, 0.9 * k));
            }
            if let (Some(def), true) = (placing, in_map) {
                let in_hand = tier_index[def].get("000").or(tier_index[def].get("100")).cloned();
                if let Some(m) = in_hand.and_then(|dir| tier_anims.get_mut(&(def, dir))).and_then(|m| m.as_mut()).or(tower_anims[def].as_mut()) {
                    let idle = m.idle_clip();
                    m.draw(&shaders, idle, get_time() as f32, [mouse_world[0], 0.0, mouse_world[1]], std::f32::consts::PI, outline_scale, false, None);
                } else if let Some(m) = &tower_models[def] {
                    m.draw_at([mouse_world[0], 0.0, mouse_world[1]], std::f32::consts::PI, &mut scratch);
                }
            }
            // Particle effects: upgrades. Played at game speed (Unity's timeScale), paused with
            // the game behind a menu.
            for (p, fx) in game.upgrade_fx.drain(..) {
                particles.spawn(fx, [p[0], 0.0, p[1]], 1.0);
            }
            for (p, fx, k) in game.place_fx.drain(..) {
                particles.spawn(&fx, [p[0], 0.0, p[1]], k);
            }
            // Each tower's own particle systems (Ring of Fire's flames...), restarted when its
            // models change (upgrades) and stopped when it's gone.
            {
                let mut alive = std::collections::HashSet::new();
                for t in &game.towers {
                    alive.insert(t.id);
                    let key = model_keys(&game, t, &tier_index[t.def]).0;
                    let mut want: Vec<String> = Vec::new();
                    if t.sub.is_none() && t.paragon.is_none() {
                        let dirs = tier_index[t.def].get(&key).into_iter().chain(attack_index[t.def].get(&key).into_iter().flatten().map(|(d, _, _)| d));
                        want.extend(dirs.filter_map(|d| tier_fx[t.def].get(d).cloned()));
                    }
                    // Avatar of Wrath's glow for how strong Wrath is now.
                    if let Some((lvl, v)) = game.towers.iter().position(|o| o.id == t.id).and_then(|i| game.wrath_level(i)) {
                        if let Some(Some(fx)) = lvl.checked_sub(1).and_then(|k| v.fx.get(k)) {
                            want.push(fx.clone());
                        }
                    }
                    // Alchemist brews on it: their buff effect plays while they last.
                    for kind in ["berserk", "acid"] {
                        if t.brews.iter().any(|b| b.brew.kind == kind) {
                            want.push(if kind == "berserk" { "BeserkerBrewBuff" } else { "AcidicMixtureBubbleBuff" }.to_string());
                        }
                    }
                    if tower_fx.get(&t.id) != Some(&want) {
                        particles.kill(t.id);
                        for fx in &want {
                            particles.spawn_tagged(fx, [t.pos[0], 0.0, t.pos[1]], t.id);
                        }
                        tower_fx.insert(t.id, want);
                    }
                }
                tower_fx.retain(|id, _| {
                    let keep = alive.contains(id);
                    if !keep {
                        particles.kill(*id);
                    }
                    keep
                });
            }
            // Each projectile's own particle systems (bolt glows, trails) follow it; when it's
            // gone they stop emitting and what they left fades out.
            {
                let tilt = 1.0 / 60f32.to_radians().tan();
                let mut seen = std::collections::HashSet::new();
                for p in &game.projectiles {
                    let Some(fx) = p.spec.display.as_ref().and_then(|g| proj_index.get(g)).and_then(|n| proj_fx.get(n)) else { continue };
                    let tag = PROJ_FX_TAG | p.uid;
                    let at = [p.pos[0], 10.0, p.pos[1] - 10.0 * tilt];
                    seen.insert(p.uid);
                    if proj_fx_live.insert(p.uid) {
                        particles.spawn_tagged(fx, at, tag);
                    } else {
                        particles.set_pos(tag, at);
                    }
                }
                proj_fx_live.retain(|uid| {
                    let keep = seen.contains(uid);
                    if !keep {
                        particles.release(PROJ_FX_TAG | uid);
                    }
                    keep
                });
            }
            if menu.is_none() {
                particles.update(get_frame_time().min(0.1) * speed as f32);
            }
            {
                let a = render3d::PITCH_DEG.to_radians();
                let px_per_unit = vh / cam.fovy.max(1.0);
                particles.draw(vec3(1.0, 0.0, 0.0), render3d::gl(0.0, a.cos(), a.sin()), px_per_unit, vh);
            }
            set_default_camera();
            if let (true, Some((rt, _, _))) = (scaled, view_rt.as_ref()) {
                render3d::draw_scaled(&rt.texture, vw, vh, adv.sharpen && adv.render_scale < 1.0);
            }
            // Window shapes the game doesn't offer (much wider or narrower than 16:9) show past
            // the terrain's edge. Mask that off so only the map is visible, and draw the scene's
            // map border there (Map MonoBehaviour: drawMapBorder, mapBorderThickness 8,
            // mapBorderColor black a=0.5, mapBorderOutlineColor black). HYPOTHESIS P-BORDER: the
            // thickness is in world units and the band sits just inside the terrain edge.
            {
                let a = render3d::to_screen(&cam, [lo3[0], 0.0, hi3[1]], vw, vh);
                let b = render3d::to_screen(&cam, [hi3[0], 0.0, lo3[1]], vw, vh);
                let (x0, x1) = (a.x.min(b.x).max(0.0), a.x.max(b.x).min(vw));
                let (y0, y1) = (a.y.min(b.y).max(0.0), a.y.max(b.y).min(vh));
                let bg = Color::from_rgba(12, 14, 12, 255);
                let masked = game.has_terrain && (x0 > 0.5 || y0 > 0.5 || x1 < vw - 0.5 || y1 < vh - 0.5);
                if masked {
                    draw_rectangle(0.0, 0.0, vw, y0, bg);
                    draw_rectangle(0.0, y1, vw, vh - y1, bg);
                    draw_rectangle(0.0, y0, x0, y1 - y0, bg);
                    draw_rectangle(x1, y0, vw - x1, y1 - y0, bg);
                    let px_per_unit = (b.x - a.x).abs() / (hi3[0] - lo3[0]);
                    let t = 8.0 * px_per_unit;
                    let band = Color::new(0.0, 0.0, 0.0, 0.5);
                    if x0 > 0.5 { draw_rectangle(x0, y0, t, y1 - y0, band); }
                    if x1 < vw - 0.5 { draw_rectangle(x1 - t, y0, t, y1 - y0, band); }
                    if y0 > 0.5 { draw_rectangle(x0, y0, x1 - x0, t, band); }
                    if y1 < vh - 0.5 { draw_rectangle(x0, y1 - t, x1 - x0, t, band); }
                    draw_rectangle_lines(x0, y0, x1 - x0, y1 - y0, 2.0, BLACK);
                }
            }
            // Preserve native map framing on resize, without another render target.
            let frame=render3d::map_frame(vw,vh);
            let padding=Color::from_rgba(12,14,12,255);
            if frame.y>0.0{draw_rectangle(0.0,0.0,vw,frame.y,padding);draw_rectangle(0.0,frame.bottom(),vw,vh-frame.bottom(),padding);}
            if frame.x>0.0{draw_rectangle(0.0,0.0,frame.x,vh,padding);draw_rectangle(frame.right(),0.0,vw-frame.right(),vh,padding);}
            // Gallery autotest: each tower's tiers under it.
            if gallery {
                for t in &game.towers {
                    let q = render3d::to_screen(&cam, [t.pos[0], 0.0, t.pos[1]], vw, vh);
                    let label = format!("{}{}{}", t.tiers[0], t.tiers[1], t.tiers[2]);
                    draw_rectangle(q.x - 22.0, q.y + 14.0, 44.0, 20.0, Color::new(0.0, 0.0, 0.0, 0.7));
                    draw_text(&label, q.x - 18.0, q.y + 30.0, 22.0, WHITE);
                }
            }
            // Buff icons over the selected tower (BuffIndicatorUi).
            if let (Some(t), Some(h)) = (selected.filter(|&t| t < game.towers.len()), hud.as_mut()) {
                let p = game.towers[t].pos;
                let at = render3d::to_screen(&cam, [p[0], 0.0, p[1]], vw, vh);
                h.draw_buffs(at, &game.buff_icons(t));
            }
            // Collected cash: CashUp.prefab, "+${0}" in the title font, yellow (1, 0.85, 0) with
            // a black outline, popping in then rising and fading over its 1 s lifespan.
            cash_pops.extend(game.cash_pops.drain(..).map(|c| (c, now)));
            cash_pops.retain(|(_, t0)| now - t0 < 1.0);
            if let Some(h) = hud.as_ref() {
                for ((p, v), t0) in &cash_pops {
                    let k = (now - t0) as f32;
                    let s = render3d::to_screen(&cam, [p[0], 12.0, p[1]], vw, vh);
                    let pop = if k < 0.12 { 0.6 + 0.4 * k / 0.12 } else { 1.0 };
                    let a = if k > 0.7 { 1.0 - (k - 0.7) / 0.3 } else { 1.0 };
                    let y = s.y - 40.0 * Hud::canvas_scale() * k;
                    h.float_text(&format!("+${}", v.floor()), vec2(s.x, y), 130.0 * pop, Color::new(1.0, 0.848, 0.0, a));
                }
            }
            // CritText.prefab: white "Crit!" over the bloon for ShowTextOnHitModel.lifespan (1 s).
            // STAND-IN P-CRITANIM: its Animation clip isn't read; the cash text's pop and rise.
            crit_texts.extend(game.hit_texts.drain(..).map(|p| (p, now)));
            crit_texts.retain(|(_, t0)| now - t0 < 1.0);
            if let Some(h) = hud.as_ref() {
                for (p, t0) in &crit_texts {
                    let k = (now - t0) as f32;
                    let s = render3d::to_screen(&cam, [p[0], 12.0, p[1]], vw, vh);
                    let pop = if k < 0.12 { 0.6 + 0.4 * k / 0.12 } else { 1.0 };
                    let a = if k > 0.7 { 1.0 - (k - 0.7) / 0.3 } else { 1.0 };
                    h.float_text("Crit!", vec2(s.x, s.y - 40.0 * Hud::canvas_scale() * k), 110.0 * pop, Color::new(1.0, 1.0, 1.0, a));
                }
            }
        } else {
            game.cash_pops.clear();
            game.hit_texts.clear();
            game.upgrade_fx.clear();
            game.place_fx.clear();
            // Map.
            let (lo, hi) = game.bounds;
            if !terrain_loaded {
                terrain_loaded = true;
                terrain = load_work::complete_asset(tex(root.join("maps").join(&map_key).join("terrain.png")));
            }
            if let Some(t) = &terrain {
                let a = view.to_screen([lo[0], hi[1]]);
                draw_texture_ex(t, a.x, a.y, WHITE, DrawTextureParams {
                    dest_size: Some(vec2((hi[0] - lo[0]) * view.s, (hi[1] - lo[1]) * view.s)),
                    ..Default::default()
                });
            }

            if show_placement {
                let (lo, hi) = game.bounds;
                let mut x = lo[0];
                while x < hi[0] {
                    let mut z = lo[1];
                    while z < hi[1] {
                        let c = view.to_screen([x, z]);
                        let col = if game.can_place(0, [x, z]) { Color::new(0.2, 1.0, 0.2, 0.6) } else { Color::new(1.0, 0.1, 0.1, 0.45) };
                        draw_rectangle(c.x - 1.5, c.y - 1.5, 3.0, 3.0, col);
                        z += 3.0;
                    }
                    x += 3.0;
                }
            }

            // Towers.
            for t in &game.towers {
                let c = view.to_screen(t.pos);
                // STAND-IN: sentries as small grey copies of their Engineer.
                let (size, tint) = if t.sub.is_some() { (10.0 * view.s, GRAY) } else { (20.0 * view.s, WHITE) };
                if let Some(tx) = &tower_tex[t.def] {
                    draw_texture_ex(tx, c.x - size / 2.0, c.y - size / 2.0, tint, DrawTextureParams {
                        dest_size: Some(vec2(size, size)),
                        rotation: -t.angle - std::f32::consts::FRAC_PI_2,
                        ..Default::default()
                    });
                } else {
                    draw_circle(c.x, c.y, 8.0 * view.s, BROWN);
                }
            }

            // Bloons (furthest along drawn last, i.e. on top).
            let mut order: Vec<usize> = (0..game.bloons.len()).collect();
            order.sort_by(|&a, &b| game.bloons[a].dist.total_cmp(&game.bloons[b].dist));
            for i in order {
                let b = &game.bloons[i];
                let d = &game.defs[b.kind];
                let c = view.to_screen(game.bloon_pos(b));
                let size = d.radius * 1.9 * view.s;
                match &bloon_tex[b.kind] {
                    Some(t) => {
                        let aspect = t.height() / t.width();
                        draw_texture_ex(t, c.x - size / 2.0, c.y - size * aspect / 2.0, WHITE, DrawTextureParams {
                            dest_size: Some(vec2(size, size * aspect)),
                            ..Default::default()
                        });
                    }
                    None => draw_circle(c.x, c.y, size / 2.0, RED),
                }
            }

            // Darts.
            for p in &game.projectiles {
                let c = view.to_screen(p.pos);
                let v = vec2(p.vel[0], -p.vel[1]).normalize_or_zero() * 6.0 * view.s;
                draw_line(c.x - v.x, c.y - v.y, c.x + v.x, c.y + v.y, 3.0, Color::from_rgba(60, 40, 20, 255));
            }

            if let (Some(q), Some(tx)) = (marker_at(&game, picking, selected, in_map, mouse_world), &mortar_target) {
                let (c, size) = (view.to_screen(q), MARKER_SIZE * view.s);
                draw_texture_ex(tx, c.x - size / 2.0, c.y - size / 2.0, WHITE, DrawTextureParams {
                    dest_size: Some(vec2(size, size)),
                    ..Default::default()
                });
            }
            // Placement preview.
            if let Some(def) = placing {
                if in_map {
                    let ok = game.can_place(def, mouse_world) && game.cash >= game.place_cost(def, mouse_world);
                    let c = vec2(mx, my);
                    // Range.png lookup colours (see render3d::range_disc).
                    let col = if ok { Color::from_rgba(0, 59, 99, 51) } else { Color::from_rgba(255, 9, 8, 89) };
                    draw_circle(c.x, c.y, placement_radius(&game.tower_defs[def].1) * view.s, col);
                    if let Some(tx) = &tower_tex[def] {
                        let size = 20.0 * view.s;
                        draw_texture_ex(tx, c.x - size / 2.0, c.y - size / 2.0, WHITE, DrawTextureParams {
                            dest_size: Some(vec2(size, size)),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        // HUD: the game's own in-game UI prefabs (see hud.rs).
        if let Some(h) = hud.as_mut() {
            let t0 = get_time();
            h.geraldo.armed = item_pick;
            let coop_blocked=h.coop_blocks(mouse_position().into());
            if coop_blocked{uitree::block_input(true);}
            let mut actions = h.draw(&game, selected, placing, speed, auto_start);
            if coop_blocked{uitree::block_input(false);actions.clear();}
            h.draw_coop(&game,coop_config.as_ref(),front.coop_slot(),coop_runtime.as_ref().map(|r|&r.replica.requests),coop_runtime.as_ref().map_or(0,|r|r.replica.frame),&mut actions);
            hud_time += get_time() - t0;
            for a in actions {
                if !matches!(a, hud::Action::Upgrade(_) | hud::Action::Sell | hud::Action::Drag(_)) {
                    game.sfx.push(game::Sfx { name: "UIClick01".into(), group: None });
                }
                match a {
                    hud::Action::Coop(action)=>{if coop_runtime.is_some(){front.coop_input(action);}},
                    // A tap only selects the shop tower (its name shows); towers are dragged out
                    // of the shop to place them (or picked up with their hotkey).
                    hud::Action::Buy(_) => {
                        placing = None;
                    }
                    hud::Action::Drag(i) => {
                        placing = Some(i);
                        dragging = true;
                        selected = None;
                    }
                    hud::Action::Go => {
                        if coop_runtime.is_some(){front.coop_input(if game.round_active{rrbtd6_coop::Action::Speed{value:if speed==1{3}else{1}}}else{rrbtd6_coop::Action::StartRound});}
                        else if game.race.is_some(){game.request_race_round();}
                        else if game.round_active || game.sandbox {
                            // Sandbox: fast forward only; rounds are sent from its menu.
                            speed = if speed == 1 { 3 } else { 1 };
                        } else {
                            game.start_round();
                        }
                    }
                    hud::Action::Pause => open_menu = Some(menus::Menu::Pause),
                    hud::Action::Upgrades(i) => {
                        h.menus.open_upgrades(i);
                        open_menu = Some(menus::Menu::Upgrades);
                    }
                    hud::Action::Upgrade(path) => {
                        if let Some(t) = selected {
                            // A tier-5 path's next upgrade is the Paragon: confirm it first.
                            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Upgrade{tower:game.towers[t].id,path:path as u8});}
                            else if game.towers[t].tiers[path] == 5 && game.paragon_quote(t).is_some() {
                                h.menus.paragon_tower = t;
                                open_menu = Some(menus::Menu::Paragon);
                            } else if !game.temple_prompt(t, path).is_some_and(|two| h.open_temple(t, path, two)) {
                                game.upgrade(t, path);
                            }
                        }
                    }
                    // The temple popup's OK (the tower can't have moved: the popup is modal).
                    hud::Action::TempleUpgrade(t, path) => {
                        if coop_runtime.is_none()&&t < game.towers.len() {
                            game.upgrade(t, path);
                        }
                    }
                    hud::Action::Sell => {
                        if let Some(t) = selected.take() {
                            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Sell{tower:game.towers[t].id});}else{game.sell(t);}
                        }
                    }
                    hud::Action::Target(m) => {
                        if let Some(t) = selected {
                            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Target{tower:game.towers[t].id,mode:coop_game::target_code(m)});}else{game.towers[t].target = m;}
                        }
                    }
                    hud::Action::SetPoint => picking = selected,
                    // Not synchronised in co-op yet.
                    hud::Action::BankCollect(all) => {
                        if coop_runtime.is_none() {
                            let banks: Vec<usize> = if all { (0..game.towers.len()).filter(|&o| game.towers[o].bank > 0.0).collect() } else { selected.into_iter().collect() };
                            for o in banks {
                                game.collect_bank(o);
                            }
                        }
                    }
                    hud::Action::LeftTarget(m) => {
                        if let Some(t) = selected.filter(|_| coop_runtime.is_none()) {
                            game.towers[t].left_target = m;
                        }
                    }
                    hud::Action::Reverse => {
                        if let Some(t) = selected.filter(|_| coop_runtime.is_none()) {
                            game.reverse_air(t);
                        }
                    }
                    hud::Action::Submerge => {
                        if let Some(t) = selected.filter(|&t| coop_runtime.is_none() && game.can_submerge(t)) {
                            game.towers[t].submerged = !game.towers[t].submerged;
                        }
                    }
                    hud::Action::ShowPops(on) => save_setting(&root, "show_pops", serde_json::json!(on)),
                    hud::Action::Hand(left) => {
                        if let Some(t) = selected.filter(|_| coop_runtime.is_none()) {
                            game.towers[t].flip = left;
                        }
                    }
                    hud::Action::CamoPrio => {
                        if let Some(t) = selected.filter(|_| coop_runtime.is_none()) {
                            game.towers[t].camo_prio = !game.towers[t].camo_prio;
                        }
                    }
                    // Not synchronised in co-op yet.
                    hud::Action::GeraldoItem(i) => {
                        if coop_runtime.is_none() {
                            if game.item_needs_target(i) {
                                item_pick = Some(i);
                            } else {
                                game.use_item(i, [0.0, 0.0]);
                            }
                        }
                    }
                    hud::Action::Close => selected = None,
                    hud::Action::Ability(name) => {
                        if let Some(runtime)=&coop_runtime{if let Some(action)=runtime.replica.ability(&game,front.coop_slot(),&name){
                            if game.ability_targets_tower(&name)||game.ability_targets_point(&name){ability_pick=Some(name);}else{front.coop_input(action);}
                        }continue;}
                        if game.ability_targets_tower(&name) || game.ability_targets_point(&name) {
                            ability_pick = Some(name);
                        } else {
                            game.activate_ability(&name);
                        }
                    }
                    hud::Action::Sandbox(a) => {
                        if matches!(a, sandbox::SandboxAction::ClearTowers) {
                            selected = None;
                        }
                        a.apply(&mut game);
                    }
                }
            }
            // The end screens (VictoryScreen / DefeatScreen) open once the game ends; after
            // Review Map a click brings them back.
            let ended = game.won || game.game_over;
            if ended && (!end_shown || (reviewing && is_mouse_button_pressed(MouseButton::Left))) {
                if game.won&&!end_shown{
                    if let Some(race)=&game.race{
                        let path=root.join("race-practice-results.json");
                        let mut results:serde_json::Value=std::fs::read(&path).ok().and_then(|b|serde_json::from_slice(&b).ok()).filter(|v:&serde_json::Value|v.is_object()).unwrap_or_else(||serde_json::json!({}));
                        let row=&mut results[&race.event_id];
                        let best=row["bestFrames"].as_u64().map_or(race.elapsed,|n|n.min(race.elapsed));
                        *row=serde_json::json!({"bestFrames":best,"lastFrames":race.elapsed,"practiceOnly":true});
                        if let Ok(bytes)=serde_json::to_vec(&results){let _=std::fs::write(path,bytes);}
                    }
                }
                if !end_shown && !game.sandbox && game.race.is_none() && std::env::var_os("RRBTD6_AUTOTEST").is_none() && frontend::account_preview::is_local() {
                    let profile_root=root.clone();let won=game.won;let completed_round=game.round;
                    std::thread::spawn(move||{if local_profile::update(&profile_root,|p|{if won{p.wins+=1;}else{p.losses+=1;}p.highest_round=p.highest_round.max(completed_round);}).is_ok(){frontend::account_preview::refresh_local(&profile_root);}});
                }
                end_shown = true;
                reviewing = false;
                open_menu = Some(if game.won { menus::Menu::Victory } else { menus::Menu::Defeat });
            }
            // Snapshot the finished game frame (before the cursor) as the menu backdrop.
            if let Some(m) = open_menu.take().filter(|&m| h.menus.available(m)) {
                selected = None;
                placing = None;
                let back = match m {
                    menus::Menu::Victory | menus::Menu::Defeat | menus::Menu::Paragon => menus::Backdrop::capture_sharp(),
                    _ => menus::Backdrop::capture(),
                };
                menu = Some((m, back));
            }
        }
        open_menu = None;
        if pending_map.is_some() {
            front.begin_loading();
        }
        if front.loading_closing() {
            if let Some(h) = hud.as_mut() { front.draw_loading(h); }
        }
        if coop_runtime.is_some(){
            let message=coop_error.as_deref().map(|e|format!("Co-op stopped: {e}. Esc opens Home.")).unwrap_or_else(||format!("Private co-op prototype | Player {} wallet | heroes and sacrifices unfinished",front.coop_slot()+1));
            draw_rectangle(0.0,screen_height()-30.0,screen_width()-Hud::panel_px(),30.0,Color::from_rgba(0,0,0,190));
            draw_text(&message,12.0,screen_height()-9.0,19.0,if coop_error.is_some(){RED}else{WHITE});
        }
        if autotest.is_some()&&std::env::var_os("RRBTD6_RESIZE_CHECK").is_some(){
            match frames{30=>request_new_screen_size(800.0,900.0),60=>request_new_screen_size(1920.0,720.0),90=>request_new_screen_size(1600.0,900.0),_=>{}}
            if matches!(frames,25|55|85|115){if let Some(path)=&autotest{get_screen_data().export_png(&format!("{path}.resize-{frames}.png"));}}
        }
        draw_cursor(&cursor_up, &cursor_down);

        if let Some(path) = &autotest {
            if presentation_check{presentation_check::arm();}
            if std::env::var_os("RRBTD6_COOP_PLAYTEST").is_some()&&std::env::var_os("RRBTD6_COOP_TEST_DONE").is_some_and(|p|std::path::Path::new(&p).exists()){
                get_screen_data().export_png(path);break;
            }
            frames += 1;
            if let Some((tower,select_at,path,buy_at))=upgrade_case{
                if frames==select_at{assert!(tower<game.towers.len());selected=Some(tower);eprintln!("upgrade-case: selected {tower} frame {frames}");}
                if frames==buy_at{assert!(game.upgrade(tower,path),"upgrade fixture must succeed");eprintln!("upgrade-case: upgraded {tower} path {path} frame {frames}");}
            }
            if late_preview&&late_tower.is_some_and(|(_,frame)|frames+4==frame){get_screen_data().export_png(&format!("{path}.preview.png"));}
            if let Some((def,_))=late_tower.filter(|(_,frame)|*frame==frames){
                placing=None;
                let centre=game.track.pos(450.0);let mut placed=false;
                'place: for radius in [24.0,40.0,60.0,90.0]{for k in 0..48{
                    let angle=k as f32/48.0*std::f32::consts::TAU;
                    if game.place(def,[centre[0]+radius*angle.cos(),centre[1]+radius*angle.sin()]){placed=true;break 'place;}
                }}
                assert!(placed,"late tower must be placed");
                eprintln!("autotest: late tower {} at frame {frames}",game.tower_defs[def].0);
            }
            // RRBTD6_TEMPLE: the first tower (a Super Monkey) goes 3xx, shows the temple popup,
            // becomes a Sun Temple, then the same for True Sun God.
            if std::env::var("RRBTD6_TEMPLE").is_ok() && !game.towers.is_empty() {
                match frames {
                    20 => {
                        for _ in 0..3 {
                            game.upgrade(0, 0);
                        }
                    }
                    30 | 150 => {
                        let two = game.temple_prompt(0, 0);
                        eprintln!("autotest: temple prompt {two:?}");
                        if let (Some(h), Some(two)) = (hud.as_mut(), two) {
                            h.open_temple(0, 0, two);
                        }
                    }
                    90 | 210 => {
                        if let Some(h) = hud.as_mut() {
                            h.temple.open = None;
                        }
                        eprintln!("autotest: temple upgrade {}", game.upgrade(0, 0));
                    }
                    _ => {}
                }
            }
            let last = std::env::var("RRBTD6_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(420u32);
            if let Some(m) = autotest_menu.filter(|_| if pause_options_test.is_some(){frames==5}else{frames + 3 == last}) {
                if m == menus::Menu::Upgrades {
                    if let Some(h) = hud.as_mut() {
                        let id = &game.tower_defs[autotest_towers[0]].0;
                        h.menus.open_upgrades(game.shop.iter().position(|s| &s.id == id).unwrap_or(1));
                        if std::env::var("RRBTD6_PICK").is_ok() {
                            h.menus.pick_first();
                        }
                    }
                }
                open_menu = Some(m);
            }
            if bench_round.is_none()&&coop_runtime.is_none()&&!game.round_active {
                // Spend spare cash on more monkeys near the track, then start the next round.
                for (i, d) in [200.0f32, 350.0, 450.0, 600.0, 800.0, 950.0].into_iter().enumerate() {
                    let def = autotest_towers[i % autotest_towers.len()];
                    if game.cash < game.tower_defs[def].1.cost { break; }
                    let c = game.track.pos(d);
                    for k in 0..24 {
                        let a = k as f32 / 24.0 * std::f32::consts::TAU;
                        if game.place(def, [c[0] + 20.0 * a.cos(), c[1] + 20.0 * a.sin()]) { break; }
                    }
                }
                // Upgrade: camo/range path first, then pierce, then speed (crosspath rules apply).
                let order: Vec<usize> = std::env::var("RRBTD6_PATHS")
                    .map(|s| s.bytes().filter_map(|b| b.checked_sub(b'0').map(usize::from)).filter(|&p| p < 3).collect())
                    .unwrap_or_else(|_| vec![2, 0, 1]);
                for t in 0..game.towers.len() {
                    for &path in &order {
                        while game.upgrade(t, path) {}
                    }
                }
                game.start_round();
            }
            // RRBTD6_BURST=n: also save the n frames before the final one (<path>.<frame>.png).
            let last = std::env::var("RRBTD6_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(420u32);
            let burst: u32 = std::env::var("RRBTD6_BURST").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
            if frames < last && frames + burst >= last {
                get_screen_data().export_png(&format!("{path}.{frames}.png"));
                eprintln!("burst {frames}: projectiles {:?}", game.projectiles.iter().map(|p| (p.owner, p.pos.map(|v| v.round()))).collect::<Vec<_>>());
            }
            // RRBTD6_ABILITY=n: uses every ready ability at frame n.
            if std::env::var("RRBTD6_ABILITY").ok().and_then(|v| v.parse::<u32>().ok()) == Some(frames) {
                for g in game.ability_groups() {
                    // Off cooldown first: an autotest has no rounds to wait through.
                    for tw in &mut game.towers {
                        tw.ability_cd.values_mut().for_each(|c| *c = 0.0);
                    }
                    eprintln!("autotest: ability {} used {}", g.name, game.activate_ability(&g.name));                }
            }
            // RRBTD6_FLOOD=Bloon:n spawns n bloons of that type every frame (stress test).
            if let Some((name, n)) = std::env::var("RRBTD6_FLOOD").ok().and_then(|v| {
                let (a, b) = v.split_once(':')?;
                Some((a.to_string(), b.parse::<usize>().ok()?))
            }) {
                game.debug_spawn(&name, n, 0.0);
            }
            if frames % 100 == 0 {
                eprintln!("autotest: frame {frames} fps {} frame {:.1} ms hud {:.1} ms bloons-draw {:.1} ms cpu {:.1} ms flip {:.1} ms batches {} | sim step avg {:.3} ms max {:.3} ms ({} steps) bloons {} projectiles {} towers {}",
                    get_fps(), get_frame_time() * 1000.0, hud_time * 10.0, bloon_time * 10.0, cpu_time * 10.0, flip_time * 10.0, sprite_batch.flushes / 100,
                    sim_stats.0 * 1000.0 / sim_stats.1.max(1) as f64, sim_stats.2 * 1000.0, sim_stats.1,
                    game.bloons.len(), game.projectiles.len(), game.towers.len());
                sim_stats = (0.0, 0, 0.0);
                hud_time = 0.0;
                bloon_time = 0.0;
                cpu_time = 0.0;
                sprite_batch.flushes = 0;
                flip_time = 0.0;
            }
            if frames % 600 == 0 {
                eprintln!("autotest: frame {frames} round {} lives {} cash {:.0} towers {:?}", game.round + 1, game.lives, game.cash,
                    game.towers.iter().map(|t| t.tier_key()).collect::<Vec<_>>());
            }
            let round_complete=bench_round.is_some()&&frames>bench_warmup&&!game.round_active&&game.bloons.is_empty();
            if frames == std::env::var("RRBTD6_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(420) || round_complete {
                if round_complete{eprintln!("round-bench: completed round {} at frame {frames}, popped {}",bench_round.unwrap(),game.popped);}
                get_screen_data().export_png(path);
                for tw in &game.towers {
                    eprintln!("autotest: end tower {} {:?} at {:?} sub {:?} fan {} look {:?}", game.tower_defs[tw.def].0, tw.tiers, tw.pos,
                        tw.sub.as_ref().map(|s| s.spec.id.clone()), tw.fan.is_some(), tw.look.as_ref().map(|l| l.0.clone()));
                    let dir = tier_index[tw.def].get(&model_keys(&game, tw, &tier_index[tw.def]).0).cloned();
                    let loaded = dir.as_ref().and_then(|d| tier_anims.get(&(tw.def, d.clone()))).map(|m| m.as_ref().map(|m| m.vertex_count()));
                    eprintln!("autotest:   model {dir:?} loaded {loaded:?}");
                }
                eprintln!("autotest: round {} active {} bloons {} popped {} lives {} cash {}",
                    game.round + 1, game.round_active, game.bloons.len(), game.popped, game.lives, game.cash);
                eprintln!("autotest: audio (loaded, missing, music) {:?}", audio.stats());
                for t in game.towers.iter().filter(|t| t.air.is_some()) {
                    eprintln!("autotest: air unit at {:?} mode {:?} pointer {:?}", t.air.as_ref().map(|a| a.pos), t.target, game.pointer);
                }
                for b in game.bloons.iter().filter(|b| game.defs[b.kind].model.is_some()) {
                    eprintln!("autotest: 3D bloon at {:?} dist {}", game.bloon_pos(b), b.dist);
                }
                eprintln!("autotest: bloon models {:?}", bloon_models.iter().map(|(k, v)| (k, v.is_some())).collect::<Vec<_>>());
                break;
            }
        }
        let frame_cpu=get_time()-cpu_t0;
        cpu_time += frame_cpu;
        pace(frame_t0, &adv, &mut frame_pacer, legacy_pacing);
        let nf0 = get_time();
        next_frame().await;
        if let Some(c)=frame_capture.as_mut(){c.record(frames,frame_t0.elapsed().as_secs_f64(),frame_cpu,game.bloons.len(),game.projectiles.len(),game.towers.len(),game.round+1,game.round_active,game.towers.iter().filter(|t|t.paragon.is_some()).count());}
        frame_t0 = std::time::Instant::now();
        flip_time += get_time() - nf0;
    }
}
