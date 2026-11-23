//! Playable local prototype. Loads data and art extracted from the user's own
//! BTD6 install (local_extracted/, git-ignored) at runtime.
//!
//! Controls: click the Dart Monkey, Bomb Shooter or Tack Shooter in the shop (or press
//! 1 / 2 / 3), click the map to place.
//! Space / Go button = start next round, F = toggle 1x/3x speed, A = auto-start,
//! gear button / Esc = pause screen, Esc = cancel placement, V = toggle 3D / 2D view.

#[cfg(windows)]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod boss_logic;
mod mem_guard;
mod map_routes;
mod anim;
mod instanced;
mod animation_storage;
mod runtime_anim_export;
mod audio;
mod data;
mod game;
mod modding;
mod mod_assets;
mod mod_runtime;
mod mod_entities;
mod mod_ui;
mod mod_maps;
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
mod map_motion;
mod sandbox;
mod temple_popup;
mod support_popup;
mod welcome_popup;
mod coop_mods_popup;
mod geraldo_shop;
mod powers_menu;
mod helper_message;
mod ui;
mod uitree;
mod callouts;
mod tower_xp;

use game::{FPS, Game};
use hud::Hud;
use macroquad::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// Seconds an explosion or Ring of Fire burst stays visible (stand-in effect).
const BURST_LIFE: f32 = 0.3;
/// Seconds a Sniper shot's tracer stays visible (stand-in effect).
const TRACER_LIFE: f32 = 0.12;
/// Seconds a LineEffectModel beam stays drawn after its shot.
const BEAM_HOLD: f32 = 0.25;

type AttackIndex=HashMap<String,Vec<(String,[f32;3],usize)>>;
fn tower_metadata(root:&std::path::Path,names:&[String])->(Vec<HashMap<String,String>>,Vec<AttackIndex>){
    let mut tier_index=Vec::new();let mut attack_index=Vec::new();
    for name in names {
        let a: serde_json::Value = std::fs::read_to_string(mod_assets::model(root,name).join("tiers").join("attack.json"))
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
                                        // ("attack": -1: an ability's attack display, usize::MAX: drawn only while it runs.)
                                        Some((e["dir"].as_str()?.to_string(), off, e["attack"].as_i64().map_or(0, |a| if a < 0 { usize::MAX } else { a as usize })))
                                    })
                                    .collect()
                            });
                            (k.clone(), list.unwrap_or_default())
                        })
                        .collect()
                })
                .unwrap_or_default(),
        );
        let idx = std::fs::read_to_string(mod_assets::model(root,name).join("tiers").join("index.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        tier_index.push(idx);
    }
    (tier_index,attack_index)
}

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
    if game.is_necro(t) {
        // (Its own TargetGreen effect marks the chosen segment.)
        return None;
    }
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

fn overlay_mod_visuals(game:&Game,tiers:&mut [HashMap<String,String>],attacks:&mut [AttackIndex]){
    for (i,definitions) in game.tower_tiers.iter().enumerate(){if let Some(definitions)=definitions{
        for (key,model) in &definitions.visuals{
            tiers[i].insert(key.clone(),model.clone());tiers[i].remove(&format!("{key}_top"));
            attacks[i].remove(key);
        }
    }}
}

/// Where the extracted game data lives: a `local_extracted` folder next to the exe (portable
/// copies), next to a macOS app bundle (RustTD6.app/Contents/MacOS/<exe>), else in the working
/// directory, else the source tree's (development builds).
/// An app bundle is read-only, and the game saves beside its data. So on iOS the root is a folder in
/// Documents linking to each bundled item; an item it has itself (settings, the profile) stays its own. The
/// bundle's path changes on every install, so the links are made again each launch.
#[cfg(target_os = "ios")]
fn ios_root() -> Option<PathBuf> {
    let bundled = std::env::current_exe().ok()?.parent()?.join("local_extracted");
    let root = PathBuf::from(std::env::var_os("HOME")?).join("Documents").join("local_extracted");
    std::fs::create_dir_all(&root).ok()?;
    for entry in std::fs::read_dir(&bundled).ok()?.flatten() {
        let link = root.join(entry.file_name());
        // A link is replaced; anything else is the player's own file.
        if link.symlink_metadata().is_ok_and(|m| m.file_type().is_symlink()) {
            let _ = std::fs::remove_file(&link);
        }
        if !link.exists() {
            let _ = std::os::unix::fs::symlink(entry.path(), &link);
        }
    }
    Some(root)
}

fn root() -> PathBuf {
    #[cfg(target_os = "ios")]
    if let Some(root) = ios_root() {
        return root;
    }
    let has_data = |p: &PathBuf| p.join("game").join("game.json").exists();
    let exe = std::env::current_exe().ok();
    let beside_exe = exe.as_ref().and_then(|e| e.parent().map(|d| d.join("local_extracted")));
    let beside_app = exe.as_ref().and_then(|e| e.ancestors().nth(4).map(|d| d.join("local_extracted")))
        .filter(|_| exe.as_ref().is_some_and(|e| e.ancestors().nth(3).is_some_and(|a| a.extension().is_some_and(|x| x == "app"))));
    let cwd = std::env::current_dir().ok().map(|d| d.join("local_extracted"));
    beside_exe
        .into_iter()
        .chain(beside_app)
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
    v.extend(mod_maps::catalog(root).into_keys());
    v.sort_by_key(|k| (k != "tutorial", k.clone()));
    v.dedup();
    v
}

/// The game data with `hero` in the shop's hero slot (the default hero if unknown) on map `map`
/// (game.json's own map if it isn't exported).
fn game_data(root: &std::path::Path, json: &str, hero: Option<&str>, map: &str,local_mods:bool) -> data::GameData {
    let compiled=match modding::game_json(root,json,local_mods){Ok(v)=>v,Err(error)=>{eprintln!("Content mods not applied: {error}");None}};
    let mut d = with_hero(compiled.as_deref().unwrap_or(json), hero);
    // The difficulty from settings.json ("mode"), Medium by default.
    d.mode = read_settings(root)["mode"].as_str().unwrap_or("Medium").to_string();
    d.map_difficulty = std::fs::read(root.join("ui/mapset.json")).ok()
        .and_then(|b| serde_json::from_slice::<Vec<serde_json::Value>>(&b).ok())
        .and_then(|rows| rows.into_iter().find(|r| r["id"].as_str().is_some_and(|id| frontend::map_key(id) == map)))
        .and_then(|r| r["difficulty"].as_u64()).map(|x| x as usize);
    if let Some(custom)=d.mod_maps.get(map){
        d.map_difficulty = None;
        custom.validate().expect("validated custom map");
        d.map=serde_json::from_value(custom.data.clone()).expect("validated map geometry");
        return d;
    }
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

/// The window and taskbar icon: our own "R" (assets/icon, tools/assets/make_icon.py; see
/// DEVIATIONS.md), not the game's.
fn game_icon(_root: &std::path::Path) -> Option<miniquad::conf::Icon> {
    Some(miniquad::conf::Icon {
        small: *include_bytes!("../assets/icon/icon16.rgba"),
        medium: *include_bytes!("../assets/icon/icon32.rgba"),
        big: *include_bytes!("../assets/icon/icon64.rgba"),
    })
}

fn conf() -> macroquad::conf::Conf {
    // Installed builds enter through the updater even when their EXE is opened directly.
    if std::env::var_os("RRBTD6_AUTOTEST").is_none()&&!std::env::args().any(|a|a=="--from-launcher") {
        if let Some(dir)=std::env::current_exe().ok().and_then(|p|p.parent().map(std::path::Path::to_path_buf)) {
            let launcher=dir.join("rrbtd6-launcher.exe");
            if dir.join("version.json").is_file()&&launcher.is_file(){
                if std::process::Command::new(launcher).current_dir(dir).spawn().is_ok(){std::process::exit(0);}
            }
        }
    }
    if std::env::args().nth(1).as_deref()==Some("--audit-animation-codec"){
        let args:Vec<_>=std::env::args().collect();
        let result=(||->std::io::Result<()>{
            let root=args.get(2).ok_or_else(||std::io::Error::other("missing models directory"))?;
            let report=args.get(3).ok_or_else(||std::io::Error::other("missing report path"))?;
            let results=animation_storage::audit_candidates(std::path::Path::new(root))?;
            std::fs::write(report,serde_json::to_vec(&results)?)?;Ok(())
        })();
        if let Err(error)=result{eprintln!("codec audit failed: {error}");std::process::exit(1);}
        std::process::exit(0);
    }
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
        // Any window shape: render3d::layout frames the map and HUD as the game does.
        platform: miniquad::conf::Platform { window_aspect_ratio:None, swap_interval: Some(if adv.vsync { 1 } else { 0 }), ..Default::default() },
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

fn apply_display_mode(a: &advanced::Advanced) {
    miniquad::window::set_fullscreen(a.fullscreen);
}

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

/// RRBTD6_MENU_BACKGROUND=1: footage mode for the website header. It opens straight on the
/// home island, windowed, with no menu UI, popups or cursor and a fixed window title.
pub(crate) fn menu_background_mode() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("RRBTD6_MENU_BACKGROUND").is_some())
}

fn draw_cursor(up: &Option<Texture2D>, down: &Option<Texture2D>) {
    if menu_background_mode() { return; }
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
    /// Mirrored in x / y (SpriteRenderer flipX / flipY, or a mirroring parent).
    flip: [bool; 2],
    spin: f32,
    color: [u8; 4],
    /// A scale animator on it or a parent: its curve over `duration`, looping or held at its end.
    scale_anim: Option<LayerAnim>,
    /// A colour animator (UnpoppedArmyBloon's glow fades out in 0.3 s): Color.Lerp(from, to,
    /// curve) replaces the renderer's colour.
    color_anim: Option<(LayerAnim, [f32; 4], [f32; 4])>,
}

struct LayerAnim {
    duration: f32,
    keys: Vec<(f32, f32)>,
    looping: bool,
}

impl LayerAnim {
    /// The curve at `age` seconds (keys joined linearly).
    fn at(&self, age: f32) -> f32 {
        let t = if self.duration <= 0.0 { 1.0 } else if self.looping { (age / self.duration).fract() } else { (age / self.duration).min(1.0) };
        let keys = &self.keys;
        match keys.iter().position(|k| k.0 > t) {
            Some(0) => keys[0].1,
            Some(j) => {
                let (a, b) = (keys[j - 1], keys[j]);
                a.1 + (b.1 - a.1) * ((t - a.0) / (b.0 - a.0).max(1e-6))
            }
            None => keys.last().map_or(1.0, |k| k.1),
        }
    }
}

/// The overlay prefabs a bloon shows: GameData.bloonOverlayData's prefab for the bloon's
/// BloonModel.overlayClass under each mutator overlayType it has (AlchemistGold on a Red:
/// RedRubberToGold; without that table: "<class><overlayType>", e.g. WhiteRegrowGlueBasic,
/// MiniLych's LychMini...), one per distinct overlay, lowest displayLayer first.
/// HYPOTHESIS: every distinct overlay it has shows at once.
fn bloon_overlays(table: &HashMap<String, data::OverlayDef>, d: &data::BloonDef, b: &game::Bloon) -> Vec<String> {
    let mut out: Vec<(i32, String)> = Vec::new();
    if d.overlay_class.is_empty() {
        return Vec::new();
    }
    let class = if d.overlay_class == "MiniLych" { "LychMini" } else { d.overlay_class.as_str() };
    for e in &b.effects {
        if !e.overlay.is_empty() {
            let (layer, name) = match table.get(&*e.overlay) {
                Some(o) => match o.assets.get(d.overlay_class.as_str()) {
                    Some(n) => (o.layer, n.clone()),
                    None => continue,
                },
                None => (0, format!("{class}{}", e.overlay)),
            };
            if !out.iter().any(|(_, n)| *n == name) {
                out.push((layer, name));
            }
        }
    }
    out.sort_by_key(|(l, _)| *l);
    out.into_iter().map(|(_, n)| n).collect()
}

/// Draws a layered projectile at Unity (x, y, z), flying along `heading` (radians), `age` seconds
/// old. Bigger layers (glows) go first, under the smaller ones.
fn draw_layers(layers: &[ProjLayer], x: f32, y: f32, z: f32, heading: f32, age: f32) {
    for (i, l) in layers.iter().enumerate() {
        let a = heading + (l.angle + l.spin * age).to_radians();
        let k = l.scale_anim.as_ref().map_or(1.0, |an| an.at(age));
        let color = l.color_anim.as_ref().map_or(l.color, |(an, from, to)| {
            let t = an.at(age).clamp(0.0, 1.0);
            std::array::from_fn(|c| ((from[c] + (to[c] - from[c]) * t).clamp(0.0, 1.0) * 255.0) as u8)
        });
        // Sprite-plane offset: x across the flight, y along it.
        let (fx, fz) = (heading.cos(), heading.sin());
        let (rx, rz) = (fz, -fx);
        let (px, pz) = (x + rx * l.offset[0] + fx * l.offset[1], z + rz * l.offset[0] + fz * l.offset[1]);
        // SpriteRenderer flipX / flipY (and a mirroring parent): mirrored about the pivot.
        let (sx, sy) = (if l.flip[0] { -1.0 } else { 1.0 }, if l.flip[1] { -1.0 } else { 1.0 });
        render3d::flat_sprite_tinted(&l.tex, px, y + i as f32 * 0.01, pz, l.w * k * sx, l.h * k * sy, l.pivot, [a.cos(), a.sin()], color);
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
    mem_guard::limit_process();
    // A crash leaves its reason in ~/RustTD6-crash.log (a Mac app has no console to read).
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            use std::io::Write;
            let path = PathBuf::from(home).join("RustTD6-crash.log");
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(f, "{info}
data folder: {}
{}
", root().display(), std::backtrace::Backtrace::force_capture());
            }
        }
        default_hook(info);
    }));
    macroquad::Window::from_config(conf(),presentation_check::wrap(game_main()));
}

async fn game_main() {
    if std::env::var_os("RRBTD6_MENU_SNAPSHOT_CHECK").is_some(){menus::verify_menu_snapshot();return;}
    let load_t0 = std::time::Instant::now();
    let mark = |what: &str| if std::env::var("RRBTD6_TIMING").is_ok() { eprintln!("load: {what} at {:.0} ms, {} MB", load_t0.elapsed().as_secs_f64() * 1e3, private_mb()) };
    let root = root();
    uitree::set_loc(menus::load_loc(&root.join("text").join("English.xml")));
    render3d::init_compressed_textures();
    mark("json");
    let json = std::sync::Arc::new(std::fs::read_to_string(simulation_path(&root,"game/game.json"))
        .expect("local_extracted/game/game.json missing: run tools/assets/build_game_data.py"));
    let mut hero: Option<String> = read_settings(&root)["hero"].as_str().map(str::to_string);
    let mut maps = map_keys(&root);
    // (RRBTD6_MAP=<map>: an autotest's map, over the saved one.)
    let env_map = std::env::var("RRBTD6_MAP").ok().filter(|m| maps.iter().any(|k| k == m));
    let mut map_key: String = env_map.or_else(|| read_settings(&root)["map"].as_str().filter(|m| maps.iter().any(|k| k == m)).map(String::from)).unwrap_or_else(|| "tutorial".to_string());
    let mut game = Game::new(game_data(&root, &json, hero.as_deref(), &map_key,true));
    modding::apply_new_sandbox(&root,&mut game,true);
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
    if std::env::var_os("RRBTD6_AUTOTEST").is_some() {
        adv.fullscreen = std::env::var("RRBTD6_FULLSCREEN").is_ok_and(|v| v == "1");
    }
    if menu_background_mode() {
        adv.fullscreen = false;
        adv.show_fps = false;
        adv.skip_startup = true;
    }
    apply_display_mode(&adv);
    let mut display_pending = true;
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
    let mut map_model = boot!(2, "Trimming the Gardens...", render3d::Model::load(&mod_maps::model_dir(&root,&map_key)));
    if map_model.is_none() {
        eprintln!("map model for {map_key} failed to load");
    }
    // Map props' idle animation (map_motion.rs). Its clock is game time since the map loaded, as
    // the effects': Unity's Animators run on scaled time (fast forward; paused behind a menu).
    let mut map_motion = map_model.as_ref().and_then(|m| map_motion::MapMotion::load(&mod_maps::model_dir(&root,&map_key), m));
    let mut map_motion_clock = 0.0f32;
    // Tower base models load when first needed (placed, in hand) or one per frame between
    // rounds in the background, not all at startup (about 0.8 s for every tower).
    let ntowers = game.tower_defs.len();
    let mut tower_models: Vec<Option<render3d::Model>> = (0..ntowers).map(|_| None).collect();
    let mut models_loaded = vec![false; ntowers];
    let mut instancer = instanced::Instancer::new();
    let mut shaders = anim::Shaders::new();
    // Per-tier display prefabs (tools/assets/export_dart_tiers.py): tier key ("030") -> model dir,
    // loaded on first use since each one is 8-18 MB of baked animation.
    // Attack displays (export_tower_tiers.py attack.json): per tier key, the models an attack
    // adds on the tower (Dartling guns, Super Monkey arms...): (model dir, offset, scale).
    let mut tower_visual_ids:Vec<_>=game.tower_defs.iter().enumerate().map(|(i,(id,_))|(id.clone(),game.tower_model(i).to_owned())).collect();
    let names:Vec<_>=tower_visual_ids.iter().map(|(_,model)|model.clone()).collect();
    let (mut tier_index,mut attack_index)=tower_metadata(&root,&names);
    overlay_mod_visuals(&game,&mut tier_index,&mut attack_index);
    let mut tower_anims:Vec<Option<anim::AnimModel>>=(0..ntowers).map(|_|None).collect();
    let mut tier_anims: HashMap<(usize, String), Option<anim::AnimModel>> = HashMap::new();
    // Each tower body's current idle (clip, start): idle variants play out whole (anim.rs idle_pose).
    let mut idle_picks: HashMap<u64, (usize, f32)> = HashMap::new();
    // Each tower model drawn this frame: (tower, model dir) -> (clip, seconds, position, yaw), so
    // its particle systems can follow the joints they hang from (anim::AnimModel::attach_mats).
    let mut model_poses: HashMap<(u64, String), (usize, f32, [f32; 3], f32)> = HashMap::new();
    // Per tower: (seconds into its attack clip, its attack_t when last drawn). The Attack state
    // isn't interruptible (MonkeyAnimationController animationStates: Attack interruptibleBehaviour
    // 0, Glaive Dominus and Apex Plasma Master): a throw while the clip plays doesn't restart it,
    // so a tower that throws faster than its clip (Glaive Dominus: every 0.04 s, a 0.15 s clip)
    // plays it through instead of freezing on its first frames.
    // The bool: a throw came while it played, so it plays again from its end.
    let mut attack_phase: HashMap<u64, (f32, f32, bool)> = HashMap::new();
    // Per tower: (throws seen, attack_t last frame), to pick among several attack clips.
    let mut throw_count: HashMap<u64, (u32, f32)> = HashMap::new();
    let mut prefetch = anim::Prefetch::default();
    let retire_predictions=std::env::var_os("RRBTD6_NO_PREDICTIVE_RETIRE").is_none();
    let mut prediction_key=None;
    let mut predicted_models=Vec::<((usize,String),std::path::PathBuf)>::new();
    let mut predicted_effects=Vec::<&'static str>::new();
    // The projectile displays the selected tower's next upgrades (and its Paragon) or the tower in
    // hand would fire, warmed like the placed towers' so a new tier's first shot doesn't load them.
    let mut predicted_projectiles=Vec::<String>::new();
    // Scope preparation to the round's reachable displays; decoded jobs and
    // cooperative GPU uploads share a four-model staging limit.
    let mut warm_round=None;
    let mut warm_bloons=Vec::<String>::new();
    // Bloon types the warm list already covers (with what they pop into): a sandbox send mid-round
    // brings new ones (40 BADs: their damage states, ZOMGs, DDTs... each loaded at first sight).
    let mut warm_kinds=Vec::<bool>::new();
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
    // A tier's extra displays with a mesh (models/extra/<name>: Mermonkey's bases, Arctic Wind's
    // snow, Superstorm's cloud), loaded when first seen; the meshless ones play as effects.
    let mut extra_models: HashMap<String, Option<anim::AnimModel>> = HashMap::new();
    // Live effect extras: tag -> (tower id, prefab name).
    let mut extra_fx_live: HashMap<u64, (u64, String)> = HashMap::new();
    // Per extra display name: (has a mesh in models/extra, has an effect in effects/), looked up once.
    let mut extra_kind: HashMap<String, (bool, bool)> = HashMap::new();
    // Per sub-tower with a parent-attack effect: its attack_t last frame (a new throw restarts it).
    let mut parent_fx_seen: HashMap<u64, f32> = HashMap::new();
    // CreateEffectProjectileAfterTime: per (projectile uid, effect index) the age its effect is next due (-1 done).
    // Each tower's hand last frame (FlipFollowPath): a switch plays its effectOnFlip.
    let mut flip_seen: HashMap<u64, bool> = HashMap::new();
    // Live Jungle Vines (JungleVineEffect): (effect tag, where, stand-in list index, camo,
    // regrow, effectFrames left, degradeStep, stepCount, End frames left once released).
    let mut vines: Vec<(u64, [f32; 2], usize, bool, bool, i32, i32, i32, Option<i32>)> = Vec::new();
    let mut vine_tag: u64 = 0;
    let mut vine_clock: u64 = 0;
    // Towers showing a frozen platform (FreezeNearbyWater): tower id -> its effect.
    let mut ice_seen: HashMap<u64, String> = HashMap::new();
    // Towers showing their Vigilante buff display (tower id).
    let mut vig_seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
    // Ability zone displays on the towers they buff: (tower id, display) -> its effect tag.
    let mut zone_seen: HashMap<(u64, String), u64> = HashMap::new();
    // Legend of the Night's immunity effects at the track ends are up.
    let mut immune_fx_on = false;
    // Each Great White effect's (pet, createdAt) -> the sim frame it exited (its Fall clip's start).
    let mut gw_exit: HashMap<(u64, i64), u64> = HashMap::new();
    // Spirit of the Forest's vines shown so far, per tower id.
    let mut spirit_shown: HashMap<u64, usize> = HashMap::new();
    // Towers whose ammo crate had arrived last frame (SniperAmmoSwap).
    let mut ammo_seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut after_fx_seen: HashMap<(u64, usize), f32> = HashMap::new();
    let outline_scale: f32 = std::env::var("RRBTD6_OUTLINE").ok().and_then(|v| v.parse().ok()).unwrap_or(0.7);
    mark("tower index");
    // Loads tower def `d`'s base models if not yet loaded.
    macro_rules! ensure_tower {
        ($d:expr) => {{
            let d: usize = $d;
            if !models_loaded[d] {
                models_loaded[d] = true;
                let dir = mod_assets::model(&root,game.tower_model(d));
                tower_models[d] = load_work::complete_asset(render3d::Model::load(&dir));
                tower_anims[d] = load_work::complete_asset(anim::AnimModel::load(&dir));
            }
        }};
    }
    if let Some(d) = game.tower_defs.iter().position(|(n, _)| n == "DartMonkey") {
        models_loaded[d] = true;
        let dir = mod_assets::model(&root,game.tower_model(d));
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
    let mut proj_index: HashMap<String, String> = std::fs::read_to_string(proj_root.join("index.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    mod_assets::register(&root,&mut proj_index);
    // Loaded when a projectile (or orbit) first shows them, not all at startup.
    let mut proj_displays: HashMap<String, ProjDisplay> = HashMap::new();
    let mut proj_tried: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Each projectile's display swapped in when it was made (TurboMutator: Turbo Charge / Perma
    // Charge's projectileDisplay), by uid.
    let mut proj_look: HashMap<u64, Option<String>> = HashMap::new();
    // MOAB Takedown ropes' chain links and hooks (models/extra/<name>), loaded when first drawn.
    let mut rope_models: HashMap<String, Option<render3d::Model>> = HashMap::new();
    async fn load_proj(dir: std::path::PathBuf) -> Option<ProjDisplay> {
        let _trace=load_work::Trace::new("projectile",&dir);
        if let Some(j) = std::fs::read_to_string(dir.join("layers.json")).ok().and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()) {
            let mut layers = Vec::new();
            for l in j["layers"].as_array().into_iter().flatten() {
                let Some(t) = tex(dir.join(l["png"].as_str().unwrap_or(""))).await else { continue };
                let f = |v: &serde_json::Value| v.as_f64().unwrap_or(0.0) as f32;
                let c = |i: usize| (f(&l["color"][i]).clamp(0.0, 1.0) * 255.0) as u8;
                let anim = |a: &serde_json::Map<String, serde_json::Value>| LayerAnim {
                    duration: a["duration"].as_f64().unwrap_or(1.0) as f32,
                    keys: a["keys"].as_array().into_iter().flatten().map(|k| (f(&k[0]), f(&k[1]))).collect(),
                    looping: a.get("loop").and_then(|v| v.as_bool()).unwrap_or(true),
                };
                layers.push((f(&l["order"]), f(&l["w"]) * f(&l["h"]), ProjLayer {
                    tex: t,
                    w: f(&l["w"]),
                    h: f(&l["h"]),
                    pivot: [f(&l["pivot"][0]), f(&l["pivot"][1])],
                    offset: [f(&l["offset"][0]), f(&l["offset"][1])],
                    angle: f(&l["angle"]),
                    flip: [l["flip"][0].as_bool().unwrap_or(false), l["flip"][1].as_bool().unwrap_or(false)],
                    spin: f(&l["spin"]),
                    color: [c(0), c(1), c(2), c(3)],
                    scale_anim: l["scale_anim"].as_object().map(|a| anim(a)),
                    color_anim: l["color_anim"].as_object().map(|a| {
                        let col = |i: usize| std::array::from_fn(|c| f(&a["colors"][i][c]));
                        (anim(a), col(0), col(1))
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
    /// The particle effects a tier's projectiles make (contact, exhaust, expiry and emission:
    /// the paragons' ExplosionXL, ~16 ms to load at first use).
    fn projectile_effects(tier:&data::Tier)->Vec<&str>{
        fn visit<'a>(p:&'a data::Proj,out:&mut Vec<&'a str>){
            for list in [&p.contact_fx,&p.exhaust_fx,&p.expire_fx,&p.emit_fx]{out.extend(list.iter().map(|(n,_)|n.as_str()));}
            out.extend(p.fraction_fx.iter().map(|f|f.0.as_str()));
            for child in &p.on_contact{visit(&child.projectile,out);}
            for child in &p.on_exhaust{visit(&child.projectile,out);}
        }
        let mut out=Vec::new();
        for attack in &tier.attacks{for weapon in &attack.weapons{
            visit(&weapon.projectile,&mut out);
            if let Some(alt)=&weapon.alternate{visit(&alt.projectile,&mut out);}
        }}
        if let Some(p)=&tier.preempt{visit(p,&mut out);}
        out
    }
    let mut warm_towers=Vec::<(u64,usize,[u8;3],Option<u32>)>::new();
    let keep_fallback_models=std::env::var_os("RRBTD6_KEEP_FALLBACK_MODELS").is_some();
    let mut warm_projectiles=Vec::<String>::new();
    let mut projectile_upload:Option<(String,std::pin::Pin<Box<dyn std::future::Future<Output=Option<ProjDisplay>>>>)>=None;
    let mut warm_effects=Vec::<String>::new();
    // Projectiles' fraction effects already played: (uid, index in fraction_fx).
    let mut fraction_fired=std::collections::HashSet::<(u64,u8)>::new();
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
    // The bounce tracer's sprite (effects/BouncingBulletTracer), loaded on first use.
    let mut bounce_tracer_tex: Option<Option<Texture2D>> = None;
    let mut beams: Vec<(([f32; 2], [f32; 2], data::LineFx), f32)> = Vec::new();
    let mut bolts: Vec<(([f32; 2], [f32; 2], data::Bolts, u64), f32)> = Vec::new();
    // Beam quads (LineEffectModel displays exported as a one-sided mesh): their texture, tint
    // and width from meta.json, drawn as a flat strip.
    let mut beam_strips: HashMap<String, Option<(Texture2D, [u8; 4], f32)>> = HashMap::new();
    let mut cash_pops: Vec<(([f32; 2], f64), f32)> = Vec::new();
    // Crossbow Master's "Crit!" (CritText.prefab): (where, when).
    let mut crit_texts: Vec<([f32; 2], f32)> = Vec::new();
    // Effect prefabs (Unity particle systems): upgrade effects so far.
    let mut particles = particles::Particles::new(&root);
    // Paragon purchases in progress (game.sacrifices): the effect data, the Paragon's "during"
    // effect tag, each orb (tag, start, direction, distance, scale), finished.
    let mut sacrifices: Vec<(game::Sacrifice, data::SacrificeFx, u64, Vec<(u64, [f32; 2], [f32; 2], f32, f32)>, bool)> = Vec::new();
    // ParagonTower.CreateDegreeText effects: (tag, the sim frame their lifespan ends).
    let mut degree_texts: Vec<(u64, u64)> = Vec::new();
    let mut fx_tag: u64 = 0xFA00_0000_0000_0000;
    // Tier effects that follow their model's joints: (tower, model dir) -> (tag, effect).
    let mut attached_fx: HashMap<(u64, String), (u64, String)> = HashMap::new();
    particles.spawn_ambient(&map_key);
    // Particle systems inside tower prefabs (export_particles.py tiers:<Tower>): model dir ->
    // effect, per tower type; and which models' effects each placed tower is playing.
    let mut tier_fx: Vec<HashMap<String, String>> = game
        .tower_defs
        .iter()
        .map(|(name, def)| {
            let name=def.model.as_deref().unwrap_or(name);
            std::fs::read_to_string(root.join("effects").join(format!("tier_{name}.json")))
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        })
        .collect();
    macro_rules! refresh_mod_models {($front:ident) => {{
        let ids:Vec<_>=game.tower_defs.iter().enumerate().map(|(i,(id,_))|(id.clone(),game.tower_model(i).to_owned())).collect();
        mod_assets::register(&root,&mut proj_index);
        if ids!=tower_visual_ids {
            let names:Vec<_>=ids.iter().map(|(_,model)|model.clone()).collect();let worker_root=root.clone();
            let (tiers,attacks,effects)=loading_task(load_work::spawn(move||{
                let (tiers,attacks)=tower_metadata(&worker_root,&names);
                let effects=names.iter().map(|name|std::fs::read_to_string(worker_root.join("effects").join(format!("tier_{name}.json")))
                    .ok().and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default()).collect::<Vec<HashMap<String,String>>>();
                (tiers,attacks,effects)
            }),&mut $front,&mut hud,&mut audio,adv.fps_cap).await;
            tier_index=tiers;attack_index=attacks;tier_fx=effects;
            overlay_mod_visuals(&game,&mut tier_index,&mut attack_index);
            models_loaded=vec![false;ids.len()];tower_models=(0..ids.len()).map(|_|None).collect();tower_anims=(0..ids.len()).map(|_|None).collect();
            tier_anims.clear();idle_picks.clear();tower_visual_ids=ids;
        }
    }};}
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
    const WHILE_FX_TAG: u64 = 1 << 61;
    let mut while_fx_live: HashMap<u64, [f32; 2]> = HashMap::new();
    /// With PROJ_FX_TAG: an orbiting glaive's effect (tower id << 8 | glaive), apart from projectile uids.
    const ORBIT_FX_TAG: u64 = 1 << 61;
    /// With PROJ_FX_TAG: a projectile's ScaleProjectileOverTime effect (projectile uid).
    const SCALE_FX_TAG: u64 = 1 << 59;
/// Jungle Vines' displays (main.rs `vines`).
const VINE_FX_TAG: u64 = 1 << 58;
/// A vine's back display (drawn under the bloons).
const VINE_BACK: u64 = 1 << 40;
/// Frozen platforms under towers on frozen water (tower id).
const ICE_FX_TAG: u64 = 1 << 56;
/// Vigilante buff displays (tower id).
const VIG_FX_TAG: u64 = 1 << 55;
/// Ability zone displays on towers.
const ZONE_FX_TAG: u64 = 1 << 54;
/// Legend of the Night's track-end immunity effects.
const IMMUNE_FX_TAG: u64 = (1 << 53) | 1;
/// Spirit of the Forest's vines (tower id).
const SPIRIT_FX_TAG: u64 = 1 << 52;
/// A projectile's CreateEffectOnExhaustFractionModel effect that ends with it (| its uid).
const FRACTION_FX_TAG: u64 = 1 << 51;
/// The vines' End clips (JungleVineWrapEnd 0.25 s, JungleVineGrowEnd 0.367 s): frames they
/// stay after the grab ends.
const VINE_END_FRAMES: i32 = 23;
    // NecromancerTargetTrackWithinRange.StartDraw: its display (TargetGreen) on the chosen spawn
    // segment while the tower is selected.
    const NECRO_FX_TAG: u64 = (1 << 62) - 1;
    let mut necro_fx: Option<[f32; 2]> = None;
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
    // InputManager's placement state: entityPositionWorld, cursorMode (0 follows the cursor,
    // 1 nudged by a third of the cursor's movement, 2 locked) and cursorPositionWorld (the next
    // frame's cursorPositionWorldLast). EnterPlacementMode starts at cursorMode 0.
    let (mut place_at, mut place_mode, mut place_cursor, mut place_def) = ([0.0f32; 2], 0u8, [0.0f32; 2], None);
    let mut selected: Option<usize> = None;
    // The removeable clicked on the map, and the model parts hidden for those bought so far.
    let mut selected_removeable: Option<usize> = None;
    let mut memory_guard = mem_guard::MemGuard::default();
    let mut hidden_parts: Vec<usize> = Vec::new();
    // "Set Target": the tower whose aim point the next map click sets.
    let mut picking: Option<usize> = None;
    // A Beast Handler pet move in progress: (handler id, leash slot) and whether its helper
    // message is up.
    let mut pet_pick: Option<(u64, usize)> = None;
    let mut pet_pick_was = false;
    // A tower-targeted ability (Overclock) waiting for the player to click a tower.
    let mut ability_pick: Option<String> = None;
    // OverclockInput: the helper message shows InvalidTarget after a click on a banned tower.
    let mut ability_invalid = false;
    // InputManager.EnterCustomMode / ExitCustomMode: the mode last frame, and the tower that was
    // selected when it began (customInputSelectedTower, selected again at the exit).
    let mut pick_was: Option<String> = None;
    let mut pick_saved: Option<u64> = None;
    // OverclockInput.targetImages: the selectionObjectPath marker's tag on each tower it may pick.
    let mut pick_markers: HashMap<u64, u64> = HashMap::new();
    // BeastHandlerMergeInput: (source handler id, leash slot, the MergeCIData's tower ids, fixed
    // as the mode begins), the mode last frame, the tower selected when it began, and
    // CannotMergeToTower pending.
    let mut merge_pick: Option<(u64, usize, Vec<u64>)> = None;
    let mut merge_was = false;
    let mut merge_saved: Option<u64> = None;
    let mut merge_invalid = false;
    // A Geraldo shop item in hand: the next map click uses it there.
    let mut item_pick: Option<usize> = None;
    // A Power picked in the Powers menu, used on the next map click.
    let mut power_pick: Option<String> = None;
    mark("rest");
    if std::env::var_os("RRBTD6_SHARED_ASSET_CHECK").is_some(){ui::verify_shared_ui_textures(&root);render3d::verify_cooperative_upload().await;}
    if std::env::var_os("RRBTD6_RENDER_CHECK").is_some(){anim::verify_draw_submission(&root).await;render3d::verify_sprite_submission().await;}
    if std::env::var_os("RRBTD6_PREFETCH_CHECK").is_some(){anim::verify_prefetch(&root).await;}
    if std::env::var_os("RRBTD6_INSTANCE_CHECK").is_some(){instanced::verify(&root).await;}
    if std::env::var_os("RRBTD6_TEXT_CACHE_CHECK").is_some(){uitree::verify_text_layout_cache(&root).await;}
    if std::env::var_os("RRBTD6_PARTICLE_TEXTURE_CHECK").is_some(){particles::verify_texture_sharing(&root).await;}
    if std::env::var_os("RRBTD6_UPGRADE_PREP_CHECK").is_some(){particles::verify_upgrade_preparation(&root).await;}
    if std::env::var_os("RRBTD6_SHADOW_TEXTURE_CHECK").is_some(){anim::verify_shadow_sharing(&root).await;}
    if std::env::var_os("RRBTD6_MAP_BATCH_CHECK").is_some(){render3d::verify_map_batching(&root).await;}
    if std::env::var_os("RRBTD6_STATIC_MAP_CHECK").is_some(){render3d::verify_static_maps(&root).await;}
    if std::env::var_os("RRBTD6_MESH_INSTANCE_CHECK").is_some(){render3d::verify_mesh_instancing(&root).await;}
    if std::env::var_os("RRBTD6_BCT_CHECK").is_some(){render3d::verify_bct(&root).await;}
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
    boot!(8,"Trimming the Gardens...",async{menus::warm_menu_snapshot();});
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
    let bench_spawn:Option<(String,usize)>=std::env::var("RRBTD6_BENCH_SPAWN").ok().and_then(|v|{let (a,b)=v.split_once(':')?;Some((a.to_string(),b.parse().ok()?))});
    let bench_warmup=std::env::var("RRBTD6_BENCH_WARMUP").ok().and_then(|v|v.parse::<u32>().ok()).unwrap_or(300);
    let gallery = std::env::var("RRBTD6_GALLERY").is_ok();
    mark("loop");
    let mut front_open_after=autotest.as_ref().and_then(|_|std::env::var("RRBTD6_FRONT_OPEN_AFTER").ok()).and_then(|v|{let (frame,page)=v.split_once(':')?;Some((frame.parse::<u32>().ok()?,frontend::MenuPage::from_name(page)?))});
    let mut frames = 0u32;
    let mut refresh_models_after_menu=false;
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
        // RRBTD6_PLACE_AT: "x,y;x,y;..." exact positions (the clone's: the game's y negated), as a
        // SimRecorder scenario places them, instead of next to track distances.
        let at: Vec<[f32; 2]> = std::env::var("RRBTD6_PLACE_AT").ok().map(|s| s.split(';').filter_map(|p| {
            let v: Vec<f32> = p.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            (v.len() == 2).then(|| [v[0], v[1]])
        }).collect()).unwrap_or_default();
        for p in &at {
            if game.place(autotest_towers[placed % autotest_towers.len()], *p) {
                placed += 1;
            }
        }
        'outer: for &d in if at.is_empty() { &spots[..] } else { &[] } {
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
        // RRBTD6_NECRO_PICK: each Necromancer's spawn segment set at the track nearest it.
        if std::env::var("RRBTD6_NECRO_PICK").is_ok() {
            for t in 0..game.towers.len() {
                if game.is_necro(t) {
                    let p = game.towers[t].pos;
                    game.necro_select(t, p);
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
                            // (The placed tower itself, not a pet it made: Corvus's Spirit.)
                            let t = (0..game.towers.len()).rev().find(|&i| game.towers[i].def == def && game.towers[i].sub.is_none()).unwrap_or(game.towers.len() - 1);
                            let b = tiers[k].as_bytes();
                            // (A hero's level key, "1000" = level 10: tiers [level, 0, 0].)
                            game.towers[t].tiers = if b.len() > 3 {
                                [(tiers[k].parse::<u32>().unwrap_or(100) / 100).min(255) as u8, 0, 0]
                            } else {
                                [b[0] - b'0', b[1] - b'0', b[2] - b'0']
                            };
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
                // RRBTD6_PARAGON_DEGREE=d: the bought Paragon at degree d (its display level).
                if let Some(d) = std::env::var("RRBTD6_PARAGON_DEGREE").ok().and_then(|v| v.parse::<u32>().ok()) {
                    let def = game.towers[0].def;
                    if let Some(base) = game.tower_tiers[def].as_ref().and_then(|x| x.paragon.as_ref()).map(|p| p.tier.clone()) {
                        game.towers[0].paragon = Some((d.clamp(1, 100), std::sync::Arc::new(game.paragon_tier(&base, d.clamp(1, 100)))));
                    }
                }
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
            // RRBTD6_BENCH_SPAWN=<bloon>:<n>: only those, sent at spacing 0 when the warm-up ends
            // (the round itself is round 1's few Reds).
            if bench_spawn.is_some(){game.round=0;}
        }
        game.start_round();
        // RRBTD6_SANDBOX=<bloon>: the sandbox menu open, RRBTD6_SANDBOX_COUNT (10) of that bloon sent
        // RRBTD6_SANDBOX_SPACING (0.3) apart.
        if let Ok(b) = std::env::var("RRBTD6_SANDBOX") {
            if let Some(h) = hud.as_mut() {
                h.sandbox.open = true;
            }
            let count = std::env::var("RRBTD6_SANDBOX_COUNT").ok().and_then(|v| v.parse().ok()).unwrap_or(10);
            let spacing = std::env::var("RRBTD6_SANDBOX_SPACING").ok().and_then(|v| v.parse().ok()).unwrap_or(0.3);
            game.sandbox_spawn(&b, false, false, false, count, spacing);
        }
        speed = std::env::var("RRBTD6_SPEED").ok().and_then(|v| v.parse().ok()).unwrap_or(3);
    }
    // RRBTD6_BENCH_SPEED: a frame bench's game speed (with real-time stepping).
    if let Some(s)=std::env::var("RRBTD6_BENCH_SPEED").ok().and_then(|v|v.parse().ok()){speed=s;}

    loop {
        // A transition is captured on the frame that requested it, before any clear.
        #[cfg(target_os = "windows")]
        if display_pending {
            display_pending = false;
        } else {
            let fullscreen = miniquad::window::is_fullscreen();
            if adv.fullscreen != fullscreen {
                adv.fullscreen = fullscreen;
                front.adv = adv.clone();
                if autotest.is_none() {
                    save_setting(&root, "advanced", serde_json::to_value(&adv).unwrap_or_default());
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        if is_key_pressed(KeyCode::F11) {
            adv.fullscreen = !adv.fullscreen;
            apply_display_mode(&adv);
            front.adv = adv.clone();
            if autotest.is_none() { save_setting(&root, "advanced", serde_json::to_value(&adv).unwrap_or_default()); }
        }
        // Load on the next frame so the old menu/map remains intact during the fade.
        if let Some(k) = pending_map.take() {
            prediction_key=None;predicted_models.clear();predicted_effects.clear();
            warm_round=None;warm_kinds.clear();warm_towers.clear();warm_projectiles.clear();projectile_upload=None;warm_effects.clear();effect_upload=None;
            while !front.loading_open_done() {
                if let Some(h) = hud.as_mut() { front.draw_loading(h); }
                audio.music(get_time());
                next_frame().await;
            }
            map_key = k;
            save_setting(&root, "map", serde_json::json!(map_key));
            let worker_root=root.clone();let worker_json=json.clone();let worker_map=map_key.clone();
            // Co-op: this player's own lobby hero in their shop.
            let worker_hero=match &coop_config{
                Some(c)=>c.room.players.iter().find(|p|p.slot==front.coop_slot()).map(|p|p.profile.hero.clone()).filter(|h|!h.is_empty()),
                None=>hero.clone(),
            };
            let worker_local=coop_config.is_none();
            let worker_mods=coop_config.as_ref().map(|c|c.room.mods.clone()).unwrap_or_default();
            let worker_mode=coop_config.as_ref().map(|c|c.room.settings.custom_mode.clone().or_else(||frontend::coop_menu::mode_name(c.room.settings.difficulty,c.room.settings.mode)).unwrap_or_else(||["Easy","Medium","Hard"][c.room.settings.difficulty.min(2) as usize].to_owned()));
            game=loading_task(load_work::spawn(move||{
                let shared=match modding::room_json(&worker_json,&worker_mods){Ok(v)=>v,Err(error)=>{eprintln!("Room mods invalid: {error}");None}};
                let mut loaded=game_data(&worker_root,shared.as_deref().unwrap_or(&worker_json),worker_hero.as_deref(),&worker_map,worker_local);
                if let Some(mode)=worker_mode{loaded.mode=mode;}
                let mut game=Game::new(loaded);
                modding::apply_new_sandbox(&worker_root,&mut game,worker_local);
                game
            }),&mut front,&mut hud,&mut audio,adv.fps_cap).await;
            refresh_mod_models!(front);
            terrain = None;
            terrain_loaded = false;
            if let Some(h) = hud.as_mut() { front.draw_loading(h); }
            next_frame().await;
            map_model = loading_task(render3d::Model::load(&mod_maps::model_dir(&root,&map_key)),&mut front,&mut hud,&mut audio,adv.fps_cap).await;
            map_motion = map_model.as_ref().and_then(|m| map_motion::MapMotion::load(&mod_maps::model_dir(&root,&map_key), m));
            map_motion_clock = 0.0;
            particles.spawn_ambient(&map_key);
            if let Some(h) = hud.as_mut() { front.draw_loading(h); }
            next_frame().await;
            hud = loading_task(Hud::load(&root, &game),&mut front,&mut hud,&mut audio,adv.fps_cap).await;
            // A new Hud's menus start at full volume: the settings' sliders are one set.
            if let Some(h) = hud.as_mut() { h.menus.volumes = [front.sliders[0], front.sliders[1], front.sliders[2]]; }
            if let Some(config)=&coop_config{
                game.freeplay_seed=config.seed;
                if std::env::var_os("RRBTD6_COOP_PLAYTEST").is_some()&&autotest.is_some(){
                    game.cash=10000.0;coop_test_stage=0;
                    let kind=std::env::var("RRBTD6_COOP_TEST_TOWER").unwrap_or_else(|_|"DartMonkey".into());
                    let def=game.tower_defs.iter().position(|(id,_)|id==&kind).unwrap();
                    let c=game.track.pos(180.0);let mut spots:Vec<[f32;2]>=Vec::new();
                    for r in [24.0f32,36.0,48.0,64.0]{for k in 0..32{
                        let angle=k as f32/32.0*std::f32::consts::TAU;let p=[c[0]+r*angle.cos(),c[1]+r*angle.sin()];
                        if game.can_place(def,p)&&spots.iter().all(|q|(p[0]-q[0]).hypot(p[1]-q[1])>=24.0){spots.push(p);}
                    }}
                    coop_test_position=spots.get(front.coop_slot() as usize).copied();
                }
                let mut runtime=coop_game::Runtime::new(config.room.players.iter().map(|p|p.slot).collect(),&mut game).expect("validated room");
                // Each player's hero (a known one only) and the Powers' behaviours.
                for p in &config.room.players{
                    if game.heroes.iter().any(|h|h.id==p.profile.hero){runtime.replica.heroes[p.slot as usize]=Some(p.profile.hero.clone());}
                }
                if let Some(h)=hud.as_ref(){runtime.replica.powers=h.powers.all_behaviors();}
                // The division's placement areas (game/coop_areas.json; Free For All has none).
                let slots:Vec<u8>=config.room.players.iter().map(|p|p.slot).collect();
                let division=frontend::coop_menu::division_name(config.room.settings.division).unwrap_or("FreeForAll");
                let areas=std::fs::read(root.join("game").join("coop_areas.json")).map_err(|e|e.to_string())
                    .and_then(|b|serde_json::from_slice::<serde_json::Value>(&b).map_err(|e|e.to_string()))
                    .and_then(|j|game::coop_areas::CoopAreas::load(&j,division,&slots));
                let mut area_error=None;
                match areas{
                    Ok(a)=>game.coop_areas=Some(a),
                    Err(e)=>{eprintln!("Co-op areas: {e}");if division!="FreeForAll"{area_error=Some(format!("Co-op areas: {e}"));}}
                }
                front.coop_send(rrbtd6_coop::Command::Loaded(runtime.replica.digest(&game)));
                coop_runtime=Some(runtime);coop_error=area_error;speed=1;auto_start=false;acc=0.0;
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
                        prefetch.request(mod_assets::tier(&root,game.tower_model(*d),&dir));
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
                        if !tier_anims.contains_key(&(*upcoming,dir.clone())){prefetch.request(mod_assets::tier(&root,game.tower_model(*upcoming),&dir));}
                    }
                }
                if let Some(dir) = tier_index[*d].get("000").or(tier_index[*d].get("100")).cloned() {
                    if !tier_anims.contains_key(&(*d, dir.clone())) {
                        let path = mod_assets::tier(&root,game.tower_model(*d),&dir);
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
            for (slot,id) in std::mem::take(&mut runtime.replica.powers_used){
                if slot==front.coop_slot(){let _=local_profile::update(&root,|p|{if let Some(n)=p.powers.get_mut(&id){*n=n.saturating_sub(1);}});}
            }
            let created=runtime.replica.placed.drain(..).filter(|(slot,_)|*slot==front.coop_slot()).map(|(_,id)|id).last();
            selected=created.or(selected_id).and_then(|id|game.towers.iter().position(|t|t.id==id));
            picking=picking_id.and_then(|id|game.towers.iter().position(|t|t.id==id));
            if created.is_some(){if let Some(t)=selected{if game.target_modes(t)==[game::TargetMode::Point]{picking=Some(t);}}}
            game.removed.clear();speed=runtime.replica.speed as u32;auto_start=runtime.replica.auto_start;
            if std::env::var_os("RRBTD6_COOP_PLAYTEST").is_some()&&autotest.is_some()&&front.coop_running(){
                let slot=front.coop_slot();let own=game.towers.iter().find(|t|runtime.replica.owns(slot,t.id)&&t.sub.is_none()).map(|t|t.id);
                if coop_test_stage==0&&runtime.replica.frame>=5{if let Some(at)=coop_test_position{front.coop_input(rrbtd6_coop::Action::Place{kind:std::env::var("RRBTD6_COOP_TEST_TOWER").unwrap_or_else(|_|"DartMonkey".into()),at});coop_test_stage=1;}}
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
            if front_open_after.is_some_and(|(frame,_)|frames>=frame){
                if let Some((_,page))=front_open_after.take(){front.screen=frontend::Screen::Menu(page);}
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
            match act {
                Some(frontend::FrontAction::Coop(config))=>{
                    (placing,selected,picking,ability_pick)=(None,None,None,None);
                    front.begin_loading();pending_map=Some(config.room.settings.map.clone());
                    coop_config=Some(config);coop_runtime=None;coop_error=None;
                    (end_shown,reviewing,menu)=(false,false,None);speed=1;auto_start=false;acc=0.0;in_front=false;
                }
                Some(frontend::FrontAction::Play { map, mode }) => {
                    front.coop_stop();coop_config=None;coop_runtime=None;coop_error=None;
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
                Some(frontend::FrontAction::Mods(ids))=>{save_setting(&root,"mods",serde_json::json!(ids));front.refresh_maps(&root);maps=map_keys(&root);if let Some(h)=hud.as_mut(){h.sprites.clear_mod_files();}tower_visual_ids.clear();proj_displays.clear();proj_tried.clear();projectile_upload=None;prefetch=anim::Prefetch::default();warm_projectiles.clear();warm_towers.clear();}
                Some(frontend::FrontAction::ModSettings(settings))=>{save_setting(&root,"mod_settings",serde_json::json!(settings));}
                Some(frontend::FrontAction::Jukebox(change))=>{
                    save_setting(&root,"jukebox",serde_json::json!(change.tracks.iter().map(|(id,_)|id).collect::<Vec<_>>()));
                    audio.set_jukebox(&change);
                }
                Some(frontend::FrontAction::Advanced(a)) => {
                    if a.fullscreen != adv.fullscreen {
                        apply_display_mode(&a);
                        display_pending = true;
                    }
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
        macroquad::set_draw_tag("other");
        if background_assets&&view3d {
            let round_key=(game.round,game.round_active);
            if warm_kinds.len()!=game.defs.len(){warm_kinds=vec![false;game.defs.len()];}
            let new_kind=game.present_kinds().any(|k|!warm_kinds[k]);
            if warm_round!=Some(round_key)||new_kind{
                warm_round=Some(round_key);warm_bloons=game.upcoming_bloon_models();
                for k in game.present_kinds(){warm_kinds[k]=true;}
            }
            warm_bloons.retain(|name|!bloon_models.contains_key(name));
            for name in warm_bloons.iter().take(4){prefetch.request(bloon_model_root.join(name));}
            if let Some((path,model))=prefetch.advance(&bloon_model_root){
                if let Some(name)=path.file_name().and_then(|n|n.to_str()){bloon_models.insert(name.to_owned(),model);}
            }
            // The selected tower's next tiers (and the tower in hand), uploaded ahead too: an
            // upgrade or placement then finds its model ready instead of uploading it that frame.
            if let Some((path,model))=prefetch.advance_where(|p|predicted_models.iter().any(|(_,w)|w==p)){
                if let Some((key,_))=predicted_models.iter().find(|(_,w)|*w==path){tier_anims.insert(key.clone(),model);}
            }
            let changed=warm_towers.len()!=game.towers.len()||game.towers.iter().zip(&warm_towers)
                .any(|(t,key)|*key!=(t.id,t.def,t.tiers,t.paragon.as_ref().map(|p|p.0)));
            if changed{
                warm_towers.clear();warm_towers.extend(game.towers.iter().map(|t|(t.id,t.def,t.tiers,t.paragon.as_ref().map(|p|p.0))));
                warm_projectiles=game.towers.iter().filter_map(|t|game.tier_of(t)).flat_map(projectile_art)
                    .filter_map(|id|proj_index.get(id)).cloned().collect();
                warm_projectiles.sort();warm_projectiles.dedup();
                warm_effects=warm_projectiles.iter().filter_map(|name|proj_fx.get(name)).cloned()
                    .chain(game.towers.iter().filter_map(|t|game.tier_of(t)).flat_map(projectile_effects).map(str::to_owned)).collect();
                warm_effects.sort();warm_effects.dedup();
            }
            // One in-flight display, polled once per frame. The same loader and
            // texture data serve both preparation and immediate first-use draws.
            if projectile_upload.is_none(){
                if let Some(name)=warm_projectiles.iter().chain(predicted_projectiles.iter()).find(|name|!proj_tried.contains(*name)){
                    projectile_upload=Some((name.clone(),Box::pin(load_proj(mod_assets::projectile(&root,name)))));
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
                if let Some(name)=warm_effects.iter().map(String::as_str).chain(predicted_effects.iter().copied())
                    .chain(predicted_projectiles.iter().filter_map(|n|proj_fx.get(n)).map(String::as_str)).find(|name|!particles.is_prepared(name)){
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
            // The end screens show the HUD as it is, sharp over the blurred map and under the
            // screen's Lightbox (the real VictoryUI / DefeatUI frames: the HUD unchanged there).
            if matches!(kind, menus::Menu::Victory | menus::Menu::Defeat) {
                if let Some(h) = hud.as_mut() {
                    uitree::block_input(true);
                    let _ = h.draw(&game, None, None, speed, auto_start);
                    uitree::block_input(false);
                }
            }
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
                        menus::MenuAction::DragAndDrop(v) => save_setting(&root, "drop_and_lock", serde_json::json!(!v)),
                        menus::MenuAction::ToggleLoop => audio.loop_track = !audio.loop_track,
                        menus::MenuAction::ToggleShuffle => audio.shuffle = !audio.shuffle,
                        menus::MenuAction::Volume(v) => {
                            audio.sfx_volume = audio::DEFAULT_SFX * v[1];
                            audio.set_music_volume(audio::DEFAULT_MUSIC * v[0]);
                            front.sliders[..3].copy_from_slice(&v);
                            save_settings(&root, v);
                        }
                        menus::MenuAction::Home => {
                            in_front = true;
                            front.return_home();
                            coop_config=None;coop_runtime=None;coop_error=None;
                            (placing, selected, picking, ability_pick) = (None, None, None, None);
                            acc = 0.0;
                            (end_shown, reviewing) = (false, false);
                            close = true;
                        }
                        menus::MenuAction::Restart => {
                            if coop_runtime.is_none(){
                            game = Game::new(game_data(&root, &json, hero.as_deref(), &map_key,coop_config.is_none()));
            refresh_models_after_menu=true;
                            modding::apply_new_sandbox(&root,&mut game,coop_config.is_none());
                            pops.clear();
                            (placing, selected, speed, acc) = (None, None, 1, 0.0);
                            (end_shown, reviewing) = (false, false);
                            }
                            close = true;
                        }
                        menus::MenuAction::BuyParagon(extra) => {
                            if coop_runtime.is_some(){
                                if let Some(tw)=game.towers.get(h.menus.paragon_tower){front.coop_input(rrbtd6_coop::Action::Paragon{tower:tw.id,extra});}
                            }else{game.buy_paragon(h.menus.paragon_tower, extra);}
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
            if std::mem::take(&mut refresh_models_after_menu){refresh_mod_models!(front);}
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
        fx::set_camera(&cam);
        if frames%30==10&&std::env::var_os("RRBTD6_VIEWPORT_CHECK").is_some(){
            for p in [[0.0,0.0],[-100.0,-60.0],[100.0,60.0],[-100.0,60.0],[100.0,-60.0]]{
                let s=render3d::to_screen(&cam,[p[0],0.0,p[1]],vw,vh);
                let back=render3d::pick_ground(&cam,s,vw,vh).expect("map point outside fitted viewport");
                assert!((vec2(back[0],back[1])-vec2(p[0],p[1])).length()<0.02,"resized map input mismatch");
            }
            let a=render3d::to_screen(&cam,[-100.0,0.0,0.0],vw,vh);let b=render3d::to_screen(&cam,[100.0,0.0,0.0],vw,vh);
            let r=render3d::map_frame(vw,vh);
            assert!(((b.x-a.x)/r.w-100.0/(render3d::ORTHO_SIZE*1.5)).abs()<0.0001);
            eprintln!("viewport-check: {vw}x{vh}, uniform native framing and five input round trips passed");
        }
        shaders.forward = (cam.target - cam.position).normalize();
        if is_key_pressed(KeyCode::F1) && map_model.is_some() {
            view3d = !view3d;
        }

        // Input.
        // Hotkeys (keybinds.rs, rebindable in Settings > Hotkeys); none while a sandbox text
        // field is being typed in.
        if let Some(h)=hud.as_mut(){h.mod_ui.sync(&game);}
        let mod_modal=hud.as_ref().is_some_and(|h|h.mod_ui.modal());
        binds.muted = mod_modal||hud.as_ref().is_some_and(|h| h.sandbox.typing());
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
            for (key,kind) in [("set_cash",sandbox::Resource::Cash),("set_health",sandbox::Resource::Lives)]{
                if binds.pressed(key){if let Some(h)=hud.as_mut(){h.sandbox.edit_resource(kind,&game);}}
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
            else if game.round_active || game.sandbox {
                speed = if speed == 1 { 3 } else { 1 };
            } else {
                game.start_round();
            }
        }
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
                } else if game.activate_ability(&g.name) {
                    // The player's activations, for the end screens' Abilities Used.
                    game.stats.abilities += 1;
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
            game = Game::new(game_data(&root, &json, hero.as_deref(), &map_key,coop_config.is_none()));
            refresh_mod_models!(front);
            modding::apply_new_sandbox(&root,&mut game,coop_config.is_none());
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
                game = Game::new(game_data(&root, &json, hero.as_deref(), &map_key,coop_config.is_none()));
            refresh_mod_models!(front);
                modding::apply_new_sandbox(&root,&mut game,coop_config.is_none());
                hud = load_work::complete_asset(Hud::load(&root, &game));
                if let Some(h) = hud.as_mut() { h.menus.volumes = [front.sliders[0], front.sliders[1], front.sliders[2]]; }
                placing = None;
                selected = None;
            }
        }
        if !mod_modal && !hud.as_ref().is_some_and(|h|h.sandbox.typing()) && is_key_pressed(KeyCode::Escape) && placing.is_none() && selected.is_none()&&ability_pick.is_none()&&picking.is_none()&&item_pick.is_none()&&power_pick.is_none() {
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
        let in_map = preview.is_some()||(render3d::map_frame(vw,vh).contains(vec2(mx,my)) && !Hud::layout().shop.contains(vec2(mx,my)) && Hud::layout().map.contains(vec2(mx,my)) && !hud.as_ref().is_some_and(|h| h.blocks(vec2(mx, my))));
        // (InputManager.cursorActive: the cursor's over the map; FollowTouchSetting reads it.)
        game.cursor = None;
        if in_map && mouse_world[0] < 1e8 {
            if coop_runtime.is_some(){
                if get_time()-coop_mouse_at>=0.1{
                    front.coop_input(rrbtd6_coop::Action::Pointer{at:mouse_world});
                    if placing.is_none(){front.coop_input(rrbtd6_coop::Action::Collect{at:mouse_world});}
                    coop_mouse_at=get_time();
                }
            }else{game.pointer = Some(mouse_world);game.cursor = Some(mouse_world);}
        }
        picking = picking.filter(|&t| t < game.towers.len() && placing.is_none());
        if picking.is_some() && (is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right)) {
            picking = None;
        }
        if ability_pick.is_some() && (is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) || placing.is_some()) {
            ability_pick = None;
        }
        pet_pick = pet_pick.filter(|&(id, slot)| placing.is_none() && game.towers.iter().position(|t| t.id == id).is_some_and(|t| game.pet_move_state(t, slot).is_some()));
        if pet_pick.is_some() && (is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right)) {
            pet_pick = None;
        }
        merge_pick = merge_pick.filter(|(id, _, _)| placing.is_none() && game.towers.iter().any(|t| t.id == *id));
        if merge_pick.is_some() && (is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right)) {
            merge_pick = None;
        }
        if item_pick.is_some() && (is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) || placing.is_some()) {
            item_pick = None;
        }
        if power_pick.is_some() && (is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) || placing.is_some()) {
            power_pick = None;
        }
        if let Some(id) = power_pick.clone().filter(|_| in_map && is_mouse_button_pressed(MouseButton::Left)) {
            // PowersMenu.PowerUseSuccess (@0x180ae14c0): Btd6Player.ConsumePower(id, 1). HYPOTHESIS
            // P-SANDBOXNOCONSUME: sandbox's menu has dontUsePlayerInventory, so nothing is used up.
            let behaviors = hud.as_ref().and_then(|h| h.powers.behaviors(&id)).cloned();
            if coop_runtime.is_some() {
                // Sent; the profile pays once the match has used it (replica.powers_used).
                front.coop_input(rrbtd6_coop::Action::Power { id: id.clone(), at: mouse_world });
                power_pick = None;
            } else if behaviors.is_some_and(|b| game.use_power(&id, &b, mouse_world)) {
                if !game.sandbox {
                    let _ = local_profile::update(&root, |p| {
                        if let Some(n) = p.powers.get_mut(&id) {
                            *n = n.saturating_sub(1);
                        }
                    });
                }
                power_pick = None;
            }
        } else if let Some(i) = item_pick.filter(|_| in_map && is_mouse_button_pressed(MouseButton::Left)) {
            // A spot it can't be used keeps it in hand.
            if coop_runtime.is_some() {
                front.coop_input(rrbtd6_coop::Action::Item { index: i as u16, at: Some(mouse_world) });
                item_pick = None;
            } else if game.use_item(i, mouse_world) {
                item_pick = None;
            }
        } else if let Some(name) = ability_pick.clone().filter(|_| in_map && is_mouse_button_pressed(MouseButton::Left)) {
            // OverclockInput.CursorUp (VERIFIED-DECOMP @0x180aad200): a tower in the valid list
            // gets it and the mode ends; another tower only sets the helper message to
            // InvalidTarget and empty ground does nothing: the mode stays.
            let mut keep = false;
            if let Some(runtime)=&coop_runtime{
                if let Some(mut action)=runtime.replica.ability(&game,front.coop_slot(),&name){
                    if let rrbtd6_coop::Action::Ability{at,target,..}=&mut action{
                        if game.ability_targets_point(&name){*at=Some(mouse_world);}
                        else{*target=game.tower_at(mouse_world).filter(|&t|game.tap_target_valid(t)).map(|t|game.towers[t].id);keep=target.is_none();ability_invalid=keep&&game.tower_at(mouse_world).is_some();}
                        if at.is_some()||target.is_some(){front.coop_input(action);}
                    }
                }
            }else if game.ability_targets_point(&name) {
                if game.activate_ability_at(&name, mouse_world) {
                    game.stats.abilities += 1;
                }
            } else {
                match game.tower_at(mouse_world) {
                    Some(t) if game.tap_target_valid(t) => {
                        if game.activate_ability_on(&name, t) {
                            game.stats.abilities += 1;
                        }
                    }
                    Some(_) => (keep, ability_invalid) = (true, true),
                    None => keep = true,
                }
            }
            if !keep {
                ability_pick = None;
            }
        } else if let Some((id, slot, list)) = merge_pick.clone().filter(|_| in_map && is_mouse_button_pressed(MouseButton::Left)) {
            // BeastHandlerMergeInput.CursorUp (VERIFIED-DECOMP @0x180a3e520): of the towers within
            // 20 of the click (DistanceSquared <= 400), the nearest in the list (a later one at an
            // equal distance wins) gets the merge (TowerToSimulation.ApplyCustomInputData ->
            // BeastHandlerLeash.ApplyCustomInputData's Contribute) and the mode ends; with only
            // unlisted towers there, SetHelperMessage(CannotMergeToTower, -1); the mode stays.
            let mut best: Option<(usize, f32)> = None;
            let mut other = false;
            for (i, t) in game.towers.iter().enumerate() {
                let d = (t.pos[0] - mouse_world[0]).powi(2) + (t.pos[1] - mouse_world[1]).powi(2);
                if d > 400.0 {
                    continue;
                }
                if !list.contains(&t.id) {
                    other = true;
                } else if best.is_none_or(|(_, b)| !(b < d)) {
                    best = Some((i, d));
                }
            }
            match (best, game.towers.iter().position(|t| t.id == id)) {
                (Some((dst, _)), Some(src)) => {
                    game.merge_beast(src, slot, dst);
                    merge_pick = None;
                }
                _ => merge_invalid = other,
            }
        } else if let Some((id, slot)) = pet_pick.filter(|_| in_map && is_mouse_button_pressed(MouseButton::Left)) {
            // DeployInput (VERIFIED-DECOMP IsPositionValid @0x180a59dd0): a valid point applies the
            // custom input (BeastHandlerLeash.ApplyCustomInputData) and the mode ends; an invalid
            // one keeps it.
            if let Some(t) = game.towers.iter().position(|t| t.id == id) {
                if game.move_pet(t, slot, mouse_world) {
                    pet_pick = None;
                }
            }
        } else if let Some(t) = picking.filter(|_| in_map && is_mouse_button_pressed(MouseButton::Left)) {
            // (A point outside a range-locked attack's range isn't taken: the picker stays open.)
            if !game.is_necro(t) && !game.point_allowed(t, mouse_world) {
            } else {
                if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Point{tower:game.towers[t].id,at:mouse_world});}else if game.is_necro(t){game.necro_select(t, mouse_world);}else{game.set_point(t, mouse_world);}
                picking = None;
            }
        } else if let Some(def) = placing {
            // InputManager.Update (mouse, nudge mode on: Game.DisableNudgeMode is only offered
            // on touch screens): CursorDown / CursorUp on the button's edges, then
            // entityPositionWorld follows cursorMode. Lock & Drop is Game.DropAndLock.
            let lock = hud.as_ref().is_some_and(|h| !h.menus.drag_and_drop);
            if place_def != Some(def) {
                (place_mode, place_cursor) = (0, mouse_world);
            }
            let mut last = place_cursor;
            if mouse_world[0] < 1e8 {
                place_cursor = mouse_world;
            }
            let cursor = place_cursor;
            // TryPlace at entityPositionWorld (no TowerSnappingData snap to lastValidPosition).
            macro_rules! try_place {
                () => {
                    if coop_runtime.is_some() {
                        front.coop_input(rrbtd6_coop::Action::Place { kind: game.tower_defs[def].0.clone(), at: place_at });
                        placing = None;
                    } else if game.place(def, place_at) {
                        placing = None;
                        let t = game.towers.len() - 1;
                        selected = Some(t);
                        // Like the game, a tower that only fires at a set point asks for it straight away.
                        if game.target_modes(t) == [game::TargetMode::Point] {
                            picking = Some(t);
                        }
                    }
                };
            }
            // The RightMenu buttons' onClick runs before InputManager.Update's CursorUp.
            match hud.as_mut().and_then(|h| h.placement_click()) {
                Some(hud::PlaceClick::Cancel) => {
                    placing = None;
                    // (InputManager.CancelClicked in a custom mode: ExitCustomMode.)
                    ability_pick = None;
                    merge_pick = None;
                }
                Some(hud::PlaceClick::Confirm) => try_place!(),
                None => {}
            }
            if placing.is_some() && is_mouse_button_pressed(MouseButton::Left) {
                // CursorDown: unless on the confirm / cancel button, a press within 10 of the
                // tower picks it up (cursorMode 0), anywhere else nudges it (cursorMode 1).
                if !hud.as_ref().is_some_and(|h| h.over_placement_button(vec2(mx, my))) {
                    let d = [cursor[0] - place_at[0], cursor[1] - place_at[1]];
                    place_mode = if d[0] * d[0] + d[1] * d[1] < 100.0 { 0 } else { 1 };
                }
                last = cursor;
            }
            if placing.is_some() && is_mouse_button_released(MouseButton::Left) {
                // CursorUp: released off the world cancels (CancelAllPlacementActions). Drag &
                // Drop tries to place; whatever is still in hand locks where it is (cursorMode 2).
                if !in_map {
                    placing = None;
                } else {
                    if !lock {
                        try_place!();
                    }
                    place_mode = 2;
                }
            }
            match place_mode {
                0 => place_at = cursor,
                1 => {
                    // Constants.worldXMin/Max, worldZMin/Max.
                    place_at[0] = (place_at[0] + (cursor[0] - last[0]) * 0.33).clamp(-150.0, 150.0);
                    place_at[1] = (place_at[1] + (cursor[1] - last[1]) * 0.33).clamp(-116.0, 116.0);
                }
                _ => {}
            }
        } else if in_map && is_mouse_button_pressed(MouseButton::Left) {
            // With the sandbox's bloon menu open, a press on the map only closes it: towers can't be
            // selected from that screen.
            let menu_open = game.sandbox && hud.as_ref().is_some_and(|h| h.sandbox.owns_map_press(get_time()));
            if game.sandbox {
                if let Some(h) = hud.as_mut() { h.sandbox.scene_hit(); }
            }
            let was = selected;
            if !menu_open { selected = game.tower_at(mouse_world); }
            selected_removeable = if selected.is_none() && !menu_open { game.removeable_at(mouse_world) } else { None };
            // CreateSoundOnSelected: a hero says something when clicked.
            if let Some(t) = selected.filter(|&t| was != Some(t)) {
                game.select_voice(t);
            }
        }
        place_def = placing;
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
                    if game.towers[t].tiers[path] == 5 && game.paragon_quote(t).is_some() {
                        if let Some(h) = hud.as_mut() {
                            h.menus.paragon_tower = t;
                        }
                        open_menu = Some(menus::Menu::Paragon);
                    } else if !game.temple_prompt(t, path).is_some_and(|two| hud.as_mut().is_some_and(|h| h.open_temple(t, path, two))) {
                        if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Upgrade{tower:game.towers[t].id,path:path as u8});}
                        else{game.upgrade(t, path);}
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

        // Fixed-step simulation (P-TIME).
        if coop_runtime.is_none(){
            // (RRBTD6_BENCH_REALTIME: a bench stepped like play, from the frame's real time.)
            let realtime=bench_steps.is_some()&&std::env::var_os("RRBTD6_BENCH_REALTIME").is_some();
            if let Some(steps)=bench_steps.filter(|_|!realtime){acc=if bench_round.is_some()&&frames<bench_warmup{0.0}else{(steps as f32+0.01)/FPS};}
            else if realtime&&bench_round.is_some()&&frames<bench_warmup{acc=0.0;}
            else{acc += get_frame_time().min(0.25) * speed as f32;}
        }
        // Under load (a huge pop: one step can take 40 ms) a frame runs steps for at most
        // STEP_BUDGET, at least one, and lets the time it couldn't get to go: the game slows for a
        // moment instead of the next frame owing more steps (each as heavy), the frame after more
        // still. The steps and their order are the same; only how many fit in a frame. Co-op keeps
        // its lockstep timing. RRBTD6_NO_STEP_BUDGET: every owed step, as before.
        const STEP_BUDGET: f64 = 0.010;
        let budget = coop_runtime.is_none() && (bench_steps.is_none() || std::env::var_os("RRBTD6_BENCH_REALTIME").is_some()) && std::env::var_os("RRBTD6_NO_STEP_BUDGET").is_none();
        let frame_sim_t0 = std::time::Instant::now();
        while acc >= 1.0 / FPS {
            if budget && frame_sim_t0.elapsed().as_secs_f64() >= STEP_BUDGET {
                acc = acc.min(1.0 / FPS);
                break;
            }
            let t_step = std::time::Instant::now();
            let completed_before=game.round;
            game.step();
            // InGame.RoundEnd -> DrainXpAndCheckLevelUp: the round's Monkey XP into the profile.
            if game.round!=completed_before && std::env::var_os("RRBTD6_AUTOTEST").is_none() {
                tower_xp::drain(&root, &mut game);
            }
            if game.round!=completed_before && !game.sandbox && std::env::var_os("RRBTD6_AUTOTEST").is_none() && frontend::account_preview::is_local() {
                let profile_root=root.clone();let completed_round=game.round;let xp=((game.round.saturating_sub(completed_before)) as u64).saturating_mul(100);
                std::thread::spawn(move||{if local_profile::update(&profile_root,|p|{p.highest_round=p.highest_round.max(completed_round);local_profile::add_experience(p,xp);}).is_ok(){frontend::account_preview::refresh_local(&profile_root);}});
            }
            let dt_step = t_step.elapsed().as_secs_f64();
            sim_stats.0 += dt_step;
            sim_stats.1 += 1;
            sim_stats.2 = sim_stats.2.max(dt_step);
            // Like the BTD6 setting: the first round is started by hand.
            if auto_start && !game.sandbox && game.round > 0 {
                game.start_round();
            }
            acc -= 1.0 / FPS;
        }
        // Expired sentries: the last tower takes the slot (or the later ones shift down).
        for (r, moved) in game.removed.drain(..) {
            let fix = |s: Option<usize>| match s {
                Some(s) if s == r => None,
                Some(s) if moved == usize::MAX && s > r => Some(s - 1),
                Some(s) if moved != usize::MAX && s == moved => Some(r),
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
                prediction_key=Some(key);predicted_models.clear();predicted_effects.clear();predicted_projectiles.clear();
                if let Some(t)=picked{
                    let tw=&game.towers[t];
                    if let Some(tt)=game.tower_tiers[tw.def].as_ref(){
                        for path in 0..3{
                            let mut tiers=tw.tiers;tiers[path]+=1;
                            if let Some(tier)=tt.by_tiers(tiers){
                                predicted_projectiles.extend(projectile_art(tier).into_iter().filter_map(|id|proj_index.get(id)).cloned());
                            }
                        }
                        if tw.paragon.is_none()&&tw.tiers.iter().any(|&x|x>=5){
                            if let Some(pd)=tt.paragon.as_ref(){
                                predicted_projectiles.extend(projectile_art(&pd.tier).into_iter().filter_map(|id|proj_index.get(id)).cloned());
                            }
                        }
                    }
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
                            predicted_models.push(((tw.def,dir.clone()),mod_assets::tier(&root,game.tower_model(tw.def),&dir)));
                        }
                    }
                }
                if let Some(d)=placing{
                    if let Some(tier)=game.tower_tiers[d].as_ref().and_then(|tt|tt.by_tiers([0,0,0]).or(tt.by_tiers([1,0,0]))){
                        predicted_projectiles.extend(projectile_art(tier).into_iter().filter_map(|id|proj_index.get(id)).cloned());
                    }
                    if let Some(dir)=tier_index[d].get("000").or(tier_index[d].get("100")){
                        predicted_models.push(((d,dir.clone()),mod_assets::tier(&root,game.tower_model(d),&dir)));
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
                        prefetch.request(mod_assets::tier(&root,game.tower_model(tw.def),&dir));
                    }
                }
            }
        }
        // The model in hand: tier 000 (heroes: level 1, "100"), loaded now so it shows at once.
        if let Some(d) = placing {
            if let Some(dir) = tier_index[d].get("000").or(tier_index[d].get("100")).cloned() {
                if !tier_anims.contains_key(&(d, dir.clone())) {
                    let path = mod_assets::tier(&root,game.tower_model(d),&dir);
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
                for ex in game.tier_of(t).map(|tr| tr.extras.as_slice()).unwrap_or(&[]) {
                    // (A Paragon's at its display level: ParagonAssetSwapModel.)
                    let name = game.paragon_display(t, &ex.name).unwrap_or_else(|| ex.name.clone());
                    let kind = *extra_kind.entry(name.clone()).or_insert_with(|| {
                        (root.join("models").join("extra").join(&name).join("meta.json").is_file(), root.join("effects").join(&name).join("effect.json").is_file())
                    });
                    if !extra_models.contains_key(&name) && kind.0 {
                        extra_models.insert(name.clone(), prefetch.load(&root.join("models").join("extra").join(&name)));
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
                        let path = mod_assets::tier(&root,game.tower_model(t.def),&dir);
                        tier_anims.insert(key, prefetch.load(&path));
                    }
                }
                for k in std::iter::once(base).chain(top) {
                    if let Some(dir) = tier_index[t.def].get(&k) {
                        let key = (t.def, dir.clone());
                        if !tier_anims.contains_key(&key) {
                            let path = mod_assets::tier(&root,game.tower_model(t.def),&dir);
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
            // Parts of what has been bought; a new game gives them back.
            if hidden_parts != game.logic.hide_parts {
                if let Some(m) = &mut map_model {
                    hidden_parts.iter().for_each(|&p| m.set_part_hidden(p, false));
                    game.logic.hide_parts.iter().for_each(|&p| m.set_part_hidden(p, true));
                }
                hidden_parts = game.logic.hide_parts.clone();
            }
            if let (Some(m), Some(motion)) = (&mut map_model, &mut map_motion) {
                motion.update(m, map_motion_clock);
            }
            macroquad::set_draw_tag("map");
            if let Some(m) = &map_model {
                m.draw_map(if scaled{None}else{cam.viewport});
            }
            macroquad::set_draw_tag("towers");
            // ParagonTower.SetTowerActive (Steam build @0xd53930): a Paragon not yet active (its
            // purchase playing) has its display scaled to zero, shadow and attack displays with it
            // (recording dart_paragon_fx: scale 0 from the purchase until Finish).
            // LeapingSword.HideTower (@0x180e38a00): the tower's DisplayBehaviors scaled to zero
            // from the jump until Finish (Sauda's Leaping Sword).
            let hidden = |t: &game::Tower| (t.paragon.is_some() && game.sim_elapsed < t.paragon_active_at) || t.leap_paused;
            // Tower ground shadows, under the range circles.
            for t in &game.towers {
                if hidden(t) {
                    continue;
                }
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
                // So does a tower none of whose attacks turns it any more (Prince of Darkness after
                // a 0-0-4 that aimed: owner, the real one faces front once bought).
                let fixed_base = (attack_index[t.def].contains_key(&key)
                    && game.tier_of(t).is_some_and(|tr| !tr.attacks.is_empty() && tr.attacks.iter().all(|a| !a.turn_body)))
                    || game.tier_of(t).is_some_and(|tr| !tr.attacks.is_empty() && !tr.attacks.iter().any(|a| (a.rotate && a.turn_body) || a.face_middle || a.rotate_always));
                let yaw = if t.air.is_some() || fixed_base { std::f32::consts::PI } else { std::f32::consts::FRAC_PI_2 - t.angle };
                if let Some(m) = model.or(tower_anims[t.def].as_ref()) {
                    m.draw_shadow([t.pos[0], 0.0, t.pos[1]], yaw);
                }
            }
            if let (Some(def), true) = (placing, in_map || place_mode != 0) {
                let in_hand=tier_index[def].get("000").or(tier_index[def].get("100"));
                let model=in_hand.and_then(|dir|tier_anims.get(&(def,dir.clone()))).and_then(|m|m.as_ref());
                if let Some(m) = model.or(tower_anims[def].as_ref()) {
                    m.draw_shadow([place_at[0], 0.0, place_at[1]], std::f32::consts::PI);
                }
            }
            // Range circles lie on the ground, under towers and bloons.
            if let (Some(def), true) = (placing, in_map || place_mode != 0) {
                let ok = game.can_place_here(def, place_at) && game.cash >= game.place_cost(def, place_at);
                render3d::range_disc(place_at[0], place_at[1], placement_radius(&game.tower_defs[def].1), ok);
            }
            if let Some(t) = selected {
                let p = game.attack_pos(t);
                render3d::range_disc(p[0], p[1], game.range(t), true);
            }
            if let (Some(q), Some(tx)) = (marker_at(&game, picking, selected, in_map, mouse_world), &mortar_target) {
                render3d::flat_sprite(tx, q[0], 0.4, q[1], MARKER_SIZE, MARKER_SIZE, [0.5, 0.5], [0.0, 1.0]);
            }
            model_poses.clear();
            for (ti, t) in game.towers.iter().enumerate() {
                if hidden(t) {
                    continue;
                }
                let (key,top_key)=model_keys(&game,t,&tier_index[t.def]);
                // A Heli's pad keeps its placement facing; the Heli itself turns.
                // A base under turning attack models (Sun Temple, True Sun God, Dartling pads)
                // whose attacks never turn the tower keeps the placement facing, whatever an
                // earlier tier (Sun Avatar) left it at.
                // So does a tower none of whose attacks turns it any more (Prince of Darkness after
                // a 0-0-4 that aimed: owner, the real one faces front once bought).
                let fixed_base = (attack_index[t.def].contains_key(&key)
                    && game.tier_of(t).is_some_and(|tr| !tr.attacks.is_empty() && tr.attacks.iter().all(|a| !a.turn_body)))
                    || game.tier_of(t).is_some_and(|tr| !tr.attacks.is_empty() && !tr.attacks.iter().any(|a| (a.rotate && a.turn_body) || a.face_middle || a.rotate_always));
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
                            let clip = match m.attack_clip().or_else(|| m.pet_attack_clip()) {
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
                    // ContributionMutator.Mutate (VERIFIED-DECOMP @0x181208b50): a merged pet's
                    // look, the display step with the highest percent at or under
                    // (power - basePower) / (maxPower - basePower) once that is above 0.
                    let step_look = s.pet.then(|| {
                        let parent = game.towers.iter().position(|o| o.id == s.parent)?;
                        let pp = game.tier(parent)?.leash_power.get(&s.spec.id)?;
                        if pp.max == pp.base {
                            return None;
                        }
                        let frac = (s.power - pp.base) as f32 / (pp.max - pp.base) as f32;
                        if frac <= 0.0 {
                            return None;
                        }
                        pp.steps.iter().filter(|(d, p, _)| d.is_some() && *p <= frac).max_by(|a, b| a.1.total_cmp(&b.1)).and_then(|(d, _, _)| d.clone())
                    }).flatten();
                    if let Some(name) = &step_look {
                        if !extra_models.contains_key(name) {
                            extra_models.insert(name.clone(), prefetch.load(&root.join("models").join("extra").join(name)));
                        }
                    }
                    let step_model = step_look.as_ref().and_then(|n| extra_models.get_mut(n)).and_then(|m| m.as_mut());
                    if let Some(m) = step_model {
                        let clip = match m.attack_clip().or_else(|| m.pet_attack_clip()) {
                            Some(a) if t.attack_t < m.clip_duration(a) => (a, t.attack_t),
                            _ => (m.idle_clip(), t.idle_t),
                        };
                        m.draw(&shaders, clip.0, clip.1, pos, aim_yaw, outline_scale, false, None);
                        continue;
                    }
                    for (m, yaw) in [(&s.spec.base, std::f32::consts::PI), (&s.spec.model, aim_yaw)] {
                        if let Some(m) = m.as_ref().and_then(|m| sub_models.get_mut(m)).and_then(|m| m.as_mut()) {
                            let clip = match m.attack_clip().or_else(|| m.pet_attack_clip()) {
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
                let prev_phase = attack_phase.get(&t.id).copied();
                // Throws so far (a new one restarts attack_t): which attack clip plays when there
                // are several (Artillery Battery's three, MAD's left and right).
                let throws = {
                    let e = throw_count.entry(t.id).or_insert((0u32, f32::MAX));
                    if attacked && t.attack_t < e.1 {
                        e.0 = e.0.wrapping_add(1);
                    }
                    e.1 = if attacked { t.attack_t } else { f32::MAX };
                    e.0
                };
                let mut body_phase: Option<(f32, f32, bool)> = None;
                // (seconds into the attack clip of length `dur`, throw pending): it runs on from
                // the last frame; a throw restarts it only once it has finished, one that came
                // while it played starts it again from its end.
                // `restart`: the state may interrupt itself (its interruptibleBehaviour lets an
                // equal priority in, PlayableAnimator.Play @0x1815296e0): a throw then starts the
                // clip again at once (Druid's attack) instead of after its end.
                let phase = |dur: f32, restart: bool| -> (f32, bool) {
                    if !attacked {
                        return (f32::MAX, false);
                    }
                    let (p, pending) = match prev_phase {
                        Some((ph, last, pending)) if t.attack_t >= last => (ph + (t.attack_t - last), pending),
                        Some(_) if restart => (t.attack_t, false),
                        Some((ph, _, _)) if ph < dur => (ph + t.attack_t, true),
                        _ => (t.attack_t, false),
                    };
                    if p >= dur && pending { (p - dur, false) } else { (p, pending) }
                };
                let last_pose = std::cell::Cell::new((0usize, 0.0f32));
                // The animation state the last throw set (Weapon.SetAnimation: the model's
                // animationStates[n], Artillery Battery's Attack3, Attack1, Attack2), else by name.
                let mut pose = |m: &mut anim::AnimModel, body: bool, yaw: f32| match if t.anim_state >= 1 && !m.state_in_range(t.anim_state) { None } else { (t.anim_state >= 1).then(|| m.state_clip(t.anim_state)).flatten().or_else(|| m.attack_clip_n(throws)) } {
                    _ if shown(m).is_some() => {
                        let (c, at) = shown(m).unwrap();
                        last_pose.set((c, at));
                        m.meshes(c, at, pos, yaw, outline_scale, blink, sel_outline)
                    }
                    Some(a) if phase(m.clip_duration(a), t.anim_state >= 1 && m.state_interrupts(t.anim_state, t.anim_state)).0 < m.clip_duration(a) => {
                        let (ph, pending) = phase(m.clip_duration(a), t.anim_state >= 1 && m.state_interrupts(t.anim_state, t.anim_state));
                        if body {
                            body_phase = Some((ph, t.attack_t, pending));
                        }
                        last_pose.set((a, ph));
                        m.meshes(a, ph, pos, yaw, outline_scale, blink, sel_outline)
                    }
                    _ => {
                        let mut other = (usize::MAX, 0.0);
                        let (c, at) = m.idle_pose(attacked, t.idle_t, if body { &mut pick } else { &mut other });
                        last_pose.set((c, at));
                        m.meshes(c, at, pos, yaw, outline_scale, blink, sel_outline)
                    }
                };
                // The tier's extra displays with a mesh (TowerModel DisplayModels besides its body):
                // on the tower, idling, scaled to range / displayRadius where linked.
                if let Some(tr) = game.tier_of(t) {
                    for ex in &tr.extras {
                        let name = game.paragon_display(t, &ex.name).unwrap_or_else(|| ex.name.clone());
                        let Some(m) = extra_models.get_mut(&name).and_then(|m| m.as_mut()) else { continue };
                        let link = tr.range_links.iter().find(|(n, _)| *n == ex.name).map(|(_, r)| *r).filter(|r| *r > 0.0);
                        let k = ex.scale * link.map_or(1.0, |r| game.range(ti) / r);
                        m.tilt = (k != 1.0).then_some((0.0, 0.0, k));
                        let c = m.idle_clip();
                        m.draw(&shaders, c, t.idle_t, pos, yaw, outline_scale, false, None);
                        m.tilt = None;
                    }
                }
                let body_dir = (fan_model.is_none() && tier_model.is_some()).then(|| tier_index[t.def].get(&key).cloned()).flatten();
                if let Some(m) = fan_model.or(tier_model).or(tower_anims[t.def].as_mut()) {
                    // FlipFollowPath.Flip (VERIFIED-DECOMP @0x180c81de0): thrown with the other hand,
                    // flipTowerDisplayX negates the tower display's x scale (Boomerang).
                    m.mirror_x = t.flip && game.tier_of(t).is_some_and(|tr| tr.attacks.iter().flat_map(|a| a.weapons.iter()).any(|w| w.flip_eject.is_some_and(|f| f[0])));
                    let (body, outline) = pose(m, true, yaw);
                    m.mirror_x = false;
                    if let Some(dir) = body_dir.clone() {
                        let (c, at) = last_pose.get();
                        model_poses.insert((t.id, dir), (c, at, pos, yaw));
                    }
                    // The Paragon's attack display, on top; both outlines go under both bodies.
                    // Its attacks' RotateToTargetModel with rotateTower false (Apex Plasma
                    // Master) turns only this display to the target: the legs keep their facing.
                    let top = top_key.as_ref().and_then(|k| tier_index[t.def].get(k)).cloned();
                    let top_yaw = if game.tier_of(t).is_some_and(|tr| !tr.attacks.is_empty() && tr.attacks.iter().all(|a| !a.turn_body)) {
                        std::f32::consts::FRAC_PI_2 - t.aim
                    } else {
                        yaw
                    };
                    let (tb, to) = match top.clone().and_then(|dir| tier_anims.get_mut(&(t.def, dir))).and_then(|m| m.as_mut()) {
                        Some(m) => pose(m, false, top_yaw),
                        None => (Vec::new(), Vec::new()),
                    };
                    if let (Some(dir), false) = (top, tb.is_empty() && to.is_empty()) {
                        let (c, at) = last_pose.get();
                        model_poses.insert((t.id, dir), (c, at, pos, top_yaw));
                    }
                    // Attack displays (Dartling gun, Super Monkey arm...): on the tower, turned to
                    // the attack's aim, animating with its throws.
                    let (mut ab, mut ao) = (Vec::new(), Vec::new());
                    for (dir, off, ai) in attack_index[t.def].get(&key).into_iter().flatten() {
                        // An ability's attack display (Glue Storm's hose): only while one of the
                        // tower's ability attacks (ActivateAttack) runs.
                        if *ai == usize::MAX && t.temp.is_empty() {
                            continue;
                        }
                        // Each attack display turns with its own attack (Robo Monkey's arms).
                        // A RotateToPointer attack (Dartling) turns smoothly at its rate: its gun
                        // follows the tower's turning angle, not the aim of its last shot.
                        let turns = game.tier_of(t).and_then(|tr| tr.attacks.get(*ai)).is_some_and(|a| a.turn_rate.is_some_and(|r| r < 1e5));
                        // RotateToTargetAttackOffset.RotateToStartingPoint (VERIFIED-DECOMP @0x180ddb3f0):
                        // before its attack aims, a display rests along up turned by startRotation
                        // (Mermonkey 3xx+'s eight tentacles fanned around it, 22.5 + 45 x n);
                        // game y up, the clone's down: (-sin r, -cos r).
                        let rest = game.tier_of(t).and_then(|tr| tr.attacks.get(*ai)).and_then(|a| a.offset_rot).map(|[_, _, _, r]| {
                            let r = r.to_radians();
                            (-r.cos()).atan2(-r.sin())
                        });
                        let aimed = t.aims.get(*ai).filter(|a| a.1.is_some()).map(|a| a.0);
                        let aim_yaw = std::f32::consts::FRAC_PI_2 - if turns { t.angle } else { aimed.or(rest).unwrap_or_else(|| t.aims.get(*ai).map_or(t.aim, |a| a.0)) };
                        if let Some(m) = tier_anims.get_mut(&(t.def, dir.clone())).and_then(|m| m.as_mut()) {
                            let (s, c) = aim_yaw.sin_cos();
                            let p = [pos[0] + off[0] * c + off[2] * s, pos[1] + off[1], pos[2] - off[0] * s + off[2] * c];
                            // The weapon's animation state picks the clip here as on the body
                            // (Weapon.SetAnimation sets it on the attack's display: Rocket Storm's
                            // gun cycles RocketSwivelAttack01-04; by name it only ever played 01).
                            let clip = if t.anim_state >= 1 && !m.state_in_range(t.anim_state) {
                                None
                            } else {
                                (t.anim_state >= 1).then(|| m.state_clip(t.anim_state)).flatten().or_else(|| m.attack_clip_n(throws))
                            };
                            let (b, o) = match clip {
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
                    match body_phase {
                        Some(p) => { attack_phase.insert(t.id, p); }
                        None => { attack_phase.remove(&t.id); }
                    }
                    continue;
                }
                match &tower_models[t.def] {
                    Some(m) => m.draw_at(pos, yaw, &mut scratch),
                    None => draw_sphere(render3d::gl(t.pos[0], 6.0, t.pos[1]), 6.0, None, BROWN),
                }
            }
            // Tower parts with the game's translated effect shaders (Apex Plasma Master's lightning).
            anim::draw_fx(get_time() as f32);
            // Back-pass effects (a held bloon's vines behind it), under the bloons.
            {
                let a = render3d::PITCH_DEG.to_radians();
                let px_per_unit = vh / cam.fovy.max(1.0);
                particles.draw_back(vec3(1.0, 0.0, 0.0), render3d::gl(0.0, a.cos(), a.sin()), px_per_unit, vh);
            }
            macroquad::set_draw_tag("bloons");
            let bloon_t0 = get_time();
            // One pose time for every MOAB-class model this frame: a run's flush (one per regular
            // bloon between MOABs) reading the clock itself re-posed and re-uploaded every model
            // each time (500 BADs at their peak: ~400 runs a frame, 8 ms of driver stalls).
            let bloon_pose_t = get_time() as f32;
            // Bloons: far (larger z) first.
            // Positions computed once (not inside the sort comparator), then far-to-near.
            bloon_order.clear();
            bloon_order.extend(game.bloons.iter().enumerate().map(|(i, b)| (game.bloon_pos(b), i)));
            bloon_order.sort_unstable_by(|a, b| b.0[1].total_cmp(&a.0[1]).then_with(||game.bloons[a.1].id.cmp(&game.bloons[b.1].id)));
            // Bloons all but level (a pop's children moving together) differ by rounding noise
            // that changes every frame: each run closer than 0.05 apart goes by id, or their
            // overlapping overlays (glue) would flicker as the order swaps.
            let mut s = 0;
            while s < bloon_order.len() {
                let mut e = s + 1;
                while e < bloon_order.len() && bloon_order[e - 1].0[1] - bloon_order[e].0[1] < 0.05 {
                    e += 1;
                }
                if e - s > 1 {
                    bloon_order[s..e].sort_unstable_by_key(|x| game.bloons[x.1].id);
                }
                s = e;
            }
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
            let bloon_viewport = if scaled { None } else { cam.viewport };
            // Overlay prefabs (glue, ice, bleed...) seen for the first time (models/projectiles/<name>).
            for b in &game.bloons {
                if b.effects.iter().any(|e| !e.overlay.is_empty()) {
                    for n in bloon_overlays(&game.overlays, &game.defs[b.kind], b) {
                        if proj_tried.insert(n.clone()) {
                            if let Some(d) = load_work::complete_asset(load_proj(mod_assets::projectile(&root, &n))) {
                                proj_displays.insert(n, d);
                            }
                        }
                    }
                }
            }
            for &(p, i) in &bloon_order {
                let b = &game.bloons[i];
                let d = &game.defs[b.kind];
                let overlays = if b.effects.iter().any(|e| !e.overlay.is_empty()) { bloon_overlays(&game.overlays, d, b) } else { Vec::new() };
                let overlays: Vec<&ProjDisplay> = overlays.iter().filter_map(|n| proj_displays.get(n)).collect();
                let model_name = d.model_at(b.hp as f32 / d.max_health.max(1) as f32);
                let model = model_name.and_then(|m| bloon_models.get_mut(m)).and_then(|m| m.as_mut());
                // Consecutive MOAB-class bloons draw as GPU instances (instanced.rs); anything
                // else drawn here ends the run first, keeping the painter's order. One with an
                // overlay draws on its own, the overlay right over it.
                let instance = model.as_ref().is_some_and(|m| !m.has_fx()) && instancer.is_some() && overlays.is_empty();
                if !instance {
                    if let Some(inst) = instancer.as_mut().filter(|x| x.pending()) {
                        inst.flush(&mut bloon_models, &shaders, bloon_pose_t, outline_scale, bloon_viewport);
                    }
                }
                let model = model_name.and_then(|m| bloon_models.get_mut(m)).and_then(|m| m.as_mut());
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
                    if instance {
                        if let Some(inst) = instancer.as_mut() {
                            inst.push(model_name.unwrap_or_default(), pos, std::f32::consts::FRAC_PI_2 - heading);
                        }
                        continue;
                    }
                    let idle = m.idle_clip();
                    m.draw(&shaders, idle, bloon_pose_t, pos, std::f32::consts::FRAC_PI_2 - heading, outline_scale, false, None);
                    // The overlay mesh (MoabGlueBasic...): a child of the display, same space.
                    for o in &overlays {
                        if let ProjDisplay::Mesh(om) = o {
                            om.draw_at(pos, std::f32::consts::FRAC_PI_2 - heading, &mut scratch);
                        }
                    }
                    continue;
                }
                match &bloon_tex[b.kind] {
                    // Display height: Path.DistanceToPointInfoBloon gives y -= 4, height +20.
                    Some(t) => match d.display.first() {
                        // Sim position: y -= 4, height 20 (Path.DistanceToPointInfoBloon); Unity from
                        // sim via inverse ToSimulationSpace(tilt 60): u.z = -s.y - s.z / tan(60).
                        // STAND-IN tints for frozen / glued bloons without an overlay prefab.
                        Some(l) => {
                            let tint = if !overlays.is_empty() {
                                [255; 4]
                            } else if b.frozen() {
                                [150, 210, 255, 255]
                            } else if b.glue() > 0 {
                                [255, 240, 120, 255]
                            } else {
                                [255; 4]
                            };
                            let y = p[1] - 20.0 / 60f32.to_radians().tan();
                            // A sprite MOAB-class bloon (Test MOAB): lies at the MOAB height and
                            // turns along the track like the 3D ones (rotateToFollowPath).
                            if d.rotate_to_follow_path {
                                sprite_batch.flush();
                                let h = 120.0;
                                let ahead = game.tracks[b.path as usize].bloon_pos(b.dist + 1.0);
                                let heading = (ahead[1] - p[1]).atan2(ahead[0] - p[0]);
                                let lift = moab_rank.get(&i).copied().unwrap_or(0.0) * MOAB_STEP;
                                let a = render3d::PITCH_DEG.to_radians();
                                let (fx, fz) = (heading.cos(), heading.sin());
                                let x = p[0] + fz * l.trim[0] + fx * l.trim[1];
                                let z = p[1] - h / 60f32.to_radians().tan() - lift * a.cos() - fx * l.trim[0] + fz * l.trim[1];
                                render3d::flat_sprite_tinted(t, x, h + lift * a.sin(), z, l.w, l.h, [0.5, 0.5], [fx, fz], tint);
                                continue;
                            }
                            sprite_batch.shift_next(l.trim);
                            match (&atlas, bloon_uv[b.kind]) {
                                (Some(a), Some(uv)) => sprite_batch.billboard_uv(&a.tex, uv, p[0], y, 20.0, l.w, l.h, tint),
                                _ => sprite_batch.billboard(t, p[0], y, 20.0, l.w, l.h, tint),
                            }
                            // Its overlay sprites (BlueGlueBasic...), untrimmed, centred on the
                            // bloon like the display they're parented to.
                            for o in &overlays {
                                match o {
                                    ProjDisplay::Layers(ls) => {
                                        for ol in ls {
                                            sprite_batch.shift_next([ol.offset[0], ol.offset[1]]);
                                            sprite_batch.billboard(&ol.tex, p[0], y, 20.0, ol.w, ol.h, ol.color);
                                        }
                                    }
                                    ProjDisplay::Sprite { tex, w, h, .. } => sprite_batch.billboard(tex, p[0], y, 20.0, *w, *h, [255; 4]),
                                    ProjDisplay::Mesh(_) => {}
                                }
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
            // The Jungle Vines' stand-in bloons (bloonEffectDisplay: GameModel.GetBloon(
            // BloonType.Construct(list entry, camo, regrow))'s display), drawn as a bloon is until
            // the grab ends. STAND-IN: between the bloons and the effects, not between the vines'
            // back and front displays.
            // The Great White's held bloon (its display stays with the effect until it pops).
            for gw in game.gw_effects.iter().filter(|g| !g.exited) {
                let k = gw.kind;
                let (Some(t), Some(l)) = (bloon_tex.get(k).and_then(|t| t.as_ref()), game.defs.get(k).and_then(|d| d.display.first())) else { continue };
                let p = gw.pos;
                let y = p[1] - 20.0 / 60f32.to_radians().tan();
                sprite_batch.shift_next(l.trim);
                match (&atlas, bloon_uv.get(k).copied().flatten()) {
                    (Some(a), Some(uv)) => sprite_batch.billboard_uv(&a.tex, uv, p[0], y, 20.0, l.w, l.h, [255; 4]),
                    _ => sprite_batch.billboard(t, p[0], y, 20.0, l.w, l.h, [255; 4]),
                }
            }
            for v in vines.iter().filter(|v| v.8.is_none()) {
                const LIST: [&str; 9] = ["Red", "Blue", "Green", "Yellow", "Pink", "Black", "Zebra", "Rainbow", "Ceramic"];
                let name = format!("{}{}{}", LIST[v.2], if v.4 { "Regrow" } else { "" }, if v.3 { "Camo" } else { "" });
                let Some(k) = game.bloon_kind(&name).or_else(|| game.bloon_kind(LIST[v.2])) else { continue };
                let (Some(t), Some(l)) = (&bloon_tex[k], game.defs[k].display.first()) else { continue };
                let p = v.1;
                let y = p[1] - 20.0 / 60f32.to_radians().tan();
                sprite_batch.shift_next(l.trim);
                match (&atlas, bloon_uv[k]) {
                    (Some(a), Some(uv)) => sprite_batch.billboard_uv(&a.tex, uv, p[0], y, 20.0, l.w, l.h, [255; 4]),
                    _ => sprite_batch.billboard(t, p[0], y, 20.0, l.w, l.h, [255; 4]),
                }
            }
            sprite_batch.flush();
            if let Some(inst) = instancer.as_mut() {
                inst.flush(&mut bloon_models, &shaders, bloon_pose_t, outline_scale, bloon_viewport);
            }
            bloon_time += get_time() - bloon_t0;
            let tilt = 1.0 / 60f32.to_radians().tan();
            // TurboMutator.Mutate (VERIFIED-DECOMP @0x180e7ac90): while Turbo Charge / Perma Charge
            // runs, the tower's weapons' projectiles are made with its projectileDisplay; ones
            // already flying keep theirs.
            macroquad::set_draw_tag("projectiles");
            // CreateEffectOnExhaustFractionModel (display only): each effect once a projectile's age
            // reaches that fraction of its lifespan (age + frames left), where it is; one marked
            // destroyOnTransformDestroy ends with it (the Exploding Pineapple's 3-2-1 countdown at
            // 0.7 of its fall, its explosion at the end).
            for p in game.projectiles.iter().filter(|p| !p.spec.fraction_fx.is_empty()) {
                let total = p.age + p.frames_left;
                if !total.is_finite() || total <= 0.0 {
                    continue;
                }
                for (k, (fx, scale, fraction, follow)) in p.spec.fraction_fx.iter().enumerate() {
                    if p.age + 1.0 >= fraction * total && fraction_fired.insert((p.uid, k as u8)) {
                        if *follow {
                            particles.spawn_tagged(fx, [p.pos[0], p.z, p.pos[1]], FRACTION_FX_TAG | p.uid);
                        } else {
                            particles.spawn(fx, [p.pos[0], 0.0, p.pos[1]], *scale);
                        }
                    }
                }
            }
            if !fraction_fired.is_empty() {
                let live: std::collections::HashSet<u64> = game.projectiles.iter().filter(|p| !p.spec.fraction_fx.is_empty()).map(|p| p.uid).collect();
                fraction_fired.retain(|&(uid, _)| {
                    let alive = live.contains(&uid);
                    if !alive {
                        particles.kill(FRACTION_FX_TAG | uid);
                    }
                    alive
                });
            }
            proj_look.retain(|uid, _| game.projectiles.iter().any(|p| p.uid == *uid));
            for p in &game.projectiles {
                proj_look.entry(p.uid).or_insert_with(|| {
                    let t = game.towers.iter().find(|o| o.id == p.owner).filter(|o| o.turbo.is_some())?;
                    let tr = game.tier_of(t)?;
                    let own = tr.attacks.iter().flat_map(|a| a.weapons.iter()).any(|w| w.projectile.display.is_some() && w.projectile.display == p.spec.display);
                    own.then(|| tr.abilities.iter().find_map(|a| a.turbo.as_ref().and_then(|tb| tb.display.clone()))).flatten()
                });
            }
            // Projectile displays seen for the first time this frame (projectiles and orbits).
            let wanted: Vec<String> = game
                .projectiles
                .iter()
                .flat_map(|p| p.spec.pierce_sprites.iter().chain(p.spec.display.iter()).chain(proj_look.get(&p.uid).into_iter().flatten()))
                .chain((0..game.towers.len()).filter_map(|t| game.tier(t).and_then(|tr| tr.orbit.as_ref()).and_then(|o| o.display.as_ref())))
                .filter_map(|g| proj_index.get(g))
                .filter(|n| !proj_tried.contains(*n))
                .cloned()
                .collect();
            for n in wanted {
                if proj_tried.insert(n.clone()) {
                    let display=if projectile_upload.as_ref().is_some_and(|(name,_)|name==&n){load_work::complete_asset(projectile_upload.take().unwrap().1)}
                        else{load_work::complete_asset(load_proj(mod_assets::projectile(&root,&n)))};
                    if let Some(d) = display {
                        proj_displays.insert(n, d);
                    }
                }
            }
            // MOAB / ZOMG Takedown hooks (CreateRopeEffectModel): chain links every spriteSpacing
            // from spriteOffset along the line from the tower to the hook, and the hook at its end,
            // facing along the line. STAND-IN: the chain leaves the tower's centre, not its eject.
            // GreatWhiteEffect (CreateGreatWhiteEffect.Collide): the shark holding its bloon, its
            // display (exported animated to models/extra) playing its thrash clip until it exits.
            // INFERRED: the prefab lies along +y like the projectile prefabs (a Rotation node), so it
            // is pitched -90 about x to stand up (screenshot check: the shark breaches beside its
            // bloon). STAND-IN: its animation states (biting, ready, exit, no-grab) aren't followed;
            // merged pets keep the base display.
            // GreatWhiteEffect.Initialise (VERIFIED-DECOMP @0x18102c170) sets bitingAnimationState (4:
            // the display's Thrash clip, its Animator's AnyState jump) at once; exitAnimationState (5:
            // Fall) when it lets go (anim_states.json, read from the display's controller).
            gw_exit.retain(|k, _| game.gw_effects.iter().any(|g| (g.tower, g.created) == *k));
            for gw in &game.gw_effects {
                let exit_at = if gw.exited { Some(*gw_exit.entry((gw.tower, gw.created)).or_insert(game.sim_elapsed)) } else { None };
                // (A merged pet's shark: its display step's projectileDisplay, ContributionMutator.)
                let step_disp = game.towers.iter().find(|o| o.id == gw.tower).and_then(|pet| {
                    let s = pet.sub.as_ref().filter(|s| s.pet)?;
                    let parent = game.towers.iter().position(|o| o.id == s.parent)?;
                    let pp = game.tier(parent)?.leash_power.get(&s.spec.id)?;
                    if pp.max == pp.base {
                        return None;
                    }
                    let frac = (s.power - pp.base) as f32 / (pp.max - pp.base) as f32;
                    if frac <= 0.0 {
                        return None;
                    }
                    pp.steps.iter().filter(|(_, p, pd)| pd.is_some() && *p <= frac).max_by(|a, b| a.1.total_cmp(&b.1)).and_then(|(_, _, pd)| pd.clone())
                });
                let Some(name) = step_disp.or_else(|| gw.spec.display.clone()) else { continue };
                if !extra_models.contains_key(&name) {
                    extra_models.insert(name.clone(), prefetch.load(&root.join("models").join("extra").join(&name)));
                }
                let Some(m) = extra_models.get_mut(&name).and_then(|m| m.as_mut()) else { continue };
                let (clip, t) = match exit_at {
                    // (Fall plays once; past its end the shark is gone.)
                    Some(at) => {
                        let Some(c) = m.clip_containing("Fall") else { continue };
                        let t = (game.sim_elapsed - at) as f32 / 60.0;
                        if t > m.clip_duration(c) {
                            continue;
                        }
                        (c, t)
                    }
                    // (Thrash loops; its phase from the sim clock.)
                    None => (m.clip_containing("Thrash").unwrap_or_else(|| m.idle_clip()), game.sim_elapsed as f32 / 60.0),
                };
                let heading = (gw.to[1] - gw.from[1]).atan2(gw.to[0] - gw.from[0]);
                m.tilt = Some(((-90f32).to_radians(), 0.0, 1.0));
                m.draw(&shaders, clip, t, [gw.pos[0], 0.0, gw.pos[1]], std::f32::consts::FRAC_PI_2 - heading, outline_scale, false, None);
                m.tilt = None;
            }
            for h in &game.hooks {
                let Some(t) = game.towers.iter().position(|o| o.id == h.owner) else { continue };
                let Some((chain, hook, spacing, offset)) = game.tier(t).and_then(|tr| tr.abilities.iter().find_map(|a| a.takedown.as_ref().and_then(|k| k.rope.clone()))) else { continue };
                let from = game.towers[t].pos;
                let (dx, dy) = (h.pos[0] - from[0], h.pos[1] - from[1]);
                let len = (dx * dx + dy * dy).sqrt();
                if len < 1e-3 {
                    continue;
                }
                let heading = dy.atan2(dx);
                let y = if h.z > 0.0 { h.z } else { 10.0 };
                for name in [&chain, &hook].into_iter().flatten() {
                    if !rope_models.contains_key(name) {
                        let m = load_work::complete_asset(render3d::Model::load(&root.join("models").join("extra").join(name)));
                        rope_models.insert(name.clone(), m);
                    }
                }
                if let Some(Some(m)) = chain.as_ref().and_then(|c| rope_models.get(c)) {
                    let mut d = offset;
                    while d < len && spacing > 0.0 {
                        let (x, z) = (from[0] + dx / len * d, from[1] + dy / len * d - y * tilt);
                        m.draw_at([x, y, z], std::f32::consts::FRAC_PI_2 - heading, &mut scratch);
                        d += spacing;
                    }
                }
                if let Some(Some(m)) = hook.as_ref().and_then(|c| rope_models.get(c)) {
                    m.draw_at([h.pos[0], y, h.pos[1] - y * tilt], std::f32::consts::FRAC_PI_2 - heading, &mut scratch);
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
                    wrath_display.as_ref().or(proj_look.get(&p.uid).and_then(|l| l.as_ref())).or(p.spec.display.as_ref())
                };
                // Ring and explosion hits have no projectile graphic (drawn as bursts); "#empty" is
                // a display whose prefab is the game's empty placeholder (nothing drawn).
                if p.spec.link_radius || matches!(display.map(String::as_str), Some("" | "#empty")) {
                    continue;
                }
                // Landed spikes and bananas lie on the ground.
                let (y, z) = if (n > 0 || p.spec.cash.is_some()) && p.vel == [0.0, 0.0] { (1.0, p.pos[1] - tilt) } else { (10.0, z) };
                // RotateModel spins the display (boomerangs) instead of facing the flight direction.
                let heading = if p.spec.spin != 0.0 { (p.spec.spin * p.age / FPS).to_radians() } else { game.track_heading(p).unwrap_or_else(|| p.vel[1].atan2(p.vel[0])) };
                // A Paragon's shot at its display level (ParagonAssetSwapModel: Bomb's cluster
                // missiles, Glaive Dominus' glaives, Ascended Shadow's shurikens, the meteors), the
                // art from models/extra.
                let swapped = display
                    .and_then(|g| proj_index.get(g))
                    .and_then(|n| game.towers.iter().find(|o| o.id == p.owner).and_then(|o| game.paragon_display(o, n)))
                    .map(|sw| format!("extra:{sw}"));
                if let Some(key) = swapped.as_ref().filter(|k| !proj_tried.contains(*k)) {
                    proj_tried.insert(key.clone());
                    if let Some(d) = load_work::complete_asset(load_proj(root.join("models").join("extra").join(&key["extra:".len()..]))) {
                        proj_displays.insert(key.clone(), d);
                    }
                }
                let chosen = swapped.as_ref().and_then(|k| proj_displays.get(k));
                match chosen.or_else(|| display.and_then(|g| proj_index.get(g)).and_then(|n| proj_displays.get(n))) {
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
                        // (Its prefab's tweens play from when it was made: the pineapple's fall.)
                        m.draw_at_aged([x, y, z], std::f32::consts::FRAC_PI_2 - heading, p.age / FPS, &mut scratch);
                        continue;
                    }
                    // A particle-only prefab (Wall of Fire): its own particle effect is all the game
                    // draws.
                    None if display.and_then(|g| proj_index.get(g)).is_some_and(|n| proj_fx.contains_key(n)) => continue,
                    // STAND-IN for particle-only area projectiles whose effect wasn't exported: a
                    // fiery disc.
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
            // CreateEffectFromCollisionToCollision's BouncingBulletTracer: its Tracer sprite (1.11 x
            // effectLength, pivot centre, lying flat) stretched from one collision to the next, its
            // colour animator fading it out over 0.3 s. (Plain lines without a LineEffectModel keep the
            // stand-in streak.)
            if bounce_tracer_tex.is_none() {
                let p = root.join("effects").join("BouncingBulletTracer").join("tex").join("sprite_Tracer.png");
                bounce_tracer_tex = Some(std::fs::read(&p).ok().map(|b| {
                    let t = Texture2D::from_file_with_format(&b, None);
                    t.set_filter(FilterMode::Linear);
                    t
                }));
            }
            for ((a, b), t0) in &tracers {
                let k = 1.0 - (now - t0) * speed as f32 / TRACER_LIFE;
                let (a, b) = ([a[0], a[1] - 10.0 * tilt], [b[0], b[1] - 10.0 * tilt]);
                match bounce_tracer_tex.as_ref().and_then(|t| t.as_ref()) {
                    Some(tex) => {
                        let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
                        let len = (dx * dx + dz * dz).sqrt();
                        if len > 1e-3 {
                            let a8 = (k.clamp(0.0, 1.0) * 255.0) as u8;
                            render3d::flat_sprite_tinted(tex, (a[0] + b[0]) * 0.5, 10.0, (a[1] + b[1]) * 0.5, 1.111, len, [0.5, 0.5], [dx / len, dz / len], [255, 255, 255, a8]);
                        }
                    }
                    None => render3d::flat_segment(a, b, 10.0, 1.2, Color::new(1.0, 0.9, 0.5, 0.15 * k), Color::new(1.0, 1.0, 0.85, 0.9 * k)),
                }
            }
            // LineEffectModel beams (Ray of Doom, Plasma Accelerator): the beam display tiled every
            // lineDisplayLength from the eject point, out to the line's end when endless (else
            // one length). Shown while the weapon keeps firing (a short hold between shots).
            beams.extend(game.beams.drain(..).map(|b| (b, now)));
            // (Held until just past Ray of Doom's next shot, 0.2 s: it stays on while firing.)
            beams.retain(|(_, t0)| (now - t0) * (speed as f32) < BEAM_HOLD);
            {
                // Only the newest beam per start point (a fast weapon queues one a frame).
                let mut seen: Vec<[i32; 2]> = Vec::new();
                for ((a, b, lf), _) in beams.iter().rev() {
                    let key = [a[0].round() as i32, a[1].round() as i32];
                    if seen.contains(&key) {
                        continue;
                    }
                    seen.push(key);
                    let Some(name) = lf.beam.as_ref().and_then(|g| proj_index.get(g)).cloned() else { continue };
                    if !beam_strips.contains_key(&name) {
                        let dir = mod_assets::projectile(&root,&name);
                        let meta: Option<serde_json::Value> = std::fs::read_to_string(dir.join("meta.json")).ok().and_then(|s| serde_json::from_str(&s).ok());
                        let strip = match meta {
                            Some(m) => {
                                let part = &m["parts"][0];
                                let c = |i: usize| (part["tint"][0][i].as_f64().unwrap_or(1.0).clamp(0.0, 1.0) * 255.0) as u8;
                                let w = (m["bbox"][1][0].as_f64().unwrap_or(5.0) - m["bbox"][0][0].as_f64().unwrap_or(-5.0)) as f32;
                                match part["texture"].as_str() {
                                    Some(t) => load_work::complete_asset(tex(dir.join(t))).map(|t| (t, [c(0), c(1), c(2), c(3)], w)),
                                    None => None,
                                }
                            }
                            // A sprite display (Skywarden's LightningLarge3): its texture, untinted, its width.
                            None => {
                                let sm: Option<serde_json::Value> = std::fs::read_to_string(dir.join("sprite.json")).ok().and_then(|s| serde_json::from_str(&s).ok());
                                sm.and_then(|m| {
                                    let w = m["w"].as_f64().unwrap_or(5.0) as f32;
                                    load_work::complete_asset(tex(dir.join("sprite.png"))).map(|t| (t, [255u8; 4], w))
                                })
                            }
                        };
                        beam_strips.insert(name.clone(), strip);
                    }
                    if proj_tried.insert(name.clone()) {
                        if let Some(d) = load_work::complete_asset(load_proj(mod_assets::projectile(&root,&name))) {
                            proj_displays.insert(name.clone(), d);
                        }
                    }
                    let Some(disp) = proj_displays.get(&name) else { continue };
                    let strip = beam_strips.get(&name).and_then(|s| s.as_ref());
                    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                    let dist = dx.hypot(dy).max(1e-3);
                    let (ux, uy) = (dx / dist, dy / dist);
                    let heading = uy.atan2(ux);
                    let seg = lf.len.max(1.0);
                    let reach = if lf.endless { dist } else { seg.min(dist) };
                    let tiles = (reach / seg).ceil().max(1.0) as usize;
                    for i in 0..tiles {
                        let d = seg * (i as f32 + 0.5);
                        let (x, z) = (a[0] + ux * d, a[1] + uy * d - 10.0 * tilt);
                        match disp {
                            ProjDisplay::Mesh(m) => match strip {
                                // INFERRED: BeamLight (a shader graph, not exported) runs the glow's
                                // cross-section along the quad; the mesh maps the whole radial
                                // glow, which tiled would read as beads.
                                Some((t, c, w)) => render3d::flat_sprite_additive(t, x, 10.0, z, *w, seg, [0.5, 0.5], [ux, uy], *c, true),
                                None => m.draw_at([x, 10.0, z], std::f32::consts::FRAC_PI_2 - heading, &mut scratch),
                            },
                            ProjDisplay::Sprite { tex, w, .. } => render3d::flat_sprite(tex, x, 10.0, z, *w, seg, [0.5, 0.5], [ux, uy]),
                            ProjDisplay::Layers(ls) => draw_layers(ls, x, 10.0, z, heading, 0.0),
                        }
                    }
                }
            }
            // CreateLightningEffect bolts: of its displays, the shortest whose sprite reaches the
            // jump (else the longest; one of each size picked by the shot's uid), stretched
            // along it, for the effect's lifeSpan.
            bolts.extend(game.bolts.drain(..).map(|b| (b, now)));
            bolts.retain(|((_, _, bl, _), t0)| (now - t0) * (speed as f32) < bl.life);
            for ((a, b, bl, uid), _) in &bolts {
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                let dist = dx.hypot(dy);
                if dist < 0.5 {
                    continue;
                }
                let names: Vec<String> = bl.displays.iter().filter_map(|g| proj_index.get(g).cloned()).collect();
                for name in &names {
                    if proj_tried.insert(name.clone()) {
                        if let Some(d) = load_work::complete_asset(load_proj(mod_assets::projectile(&root,name))) {
                            proj_displays.insert(name.clone(), d);
                        }
                    }
                }
                let mut opts: Vec<(f32, f32, &Texture2D)> = Vec::new();
                for name in &names {
                    match proj_displays.get(name) {
                        Some(ProjDisplay::Sprite { tex, w, h, .. }) => opts.push((*h, *w, tex)),
                        Some(ProjDisplay::Layers(ls)) if !ls.is_empty() => opts.push((ls[0].h, ls[0].w, &ls[0].tex)),
                        _ => {}
                    }
                }
                if opts.is_empty() {
                    continue;
                }
                let fit = opts.iter().map(|o| o.0).filter(|&h| h >= dist).fold(f32::MAX, f32::min);
                let want = if fit == f32::MAX { opts.iter().map(|o| o.0).fold(0.0, f32::max) } else { fit };
                let same: Vec<_> = opts.iter().filter(|o| o.0 == want).collect();
                let (_, w, t) = same[(*uid as usize) % same.len()];
                let (mx, mz) = ((a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5 - 10.0 * tilt);
                render3d::flat_sprite(t, mx, 12.0, mz, *w, dist, [0.5, 0.5], [dx / dist, dy / dist]);
            }
            if let (Some(def), true) = (placing, in_map || place_mode != 0) {
                let in_hand = tier_index[def].get("000").or(tier_index[def].get("100")).cloned();
                if let Some(m) = in_hand.and_then(|dir| tier_anims.get_mut(&(def, dir))).and_then(|m| m.as_mut()).or(tower_anims[def].as_mut()) {
                    let idle = m.idle_clip();
                    m.draw(&shaders, idle, get_time() as f32, [place_at[0], 0.0, place_at[1]], std::f32::consts::PI, outline_scale, false, None);
                } else if let Some(m) = &tower_models[def] {
                    m.draw_at([place_at[0], 0.0, place_at[1]], std::f32::consts::PI, &mut scratch);
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
            // FreezeNearbyWater (ZoneEnter / ZoneExit): a land tower on water in an Ice Monkey's
            // zone shows its freezeAsset (FrozenPlatform) under it, kept while it stays in the zone.
            for t in 0..game.towers.len() {
                let id = game.towers[t].id;
                let want = game.frozen_platform(t).and_then(|o| game.tier(o).and_then(|tr| tr.freeze_fx.clone()));
                let tag = ICE_FX_TAG | id;
                match (want, ice_seen.get(&id)) {
                    (Some(fx), Some(cur)) if *cur == fx => particles.set_pos(tag, [game.towers[t].pos[0], 0.0, game.towers[t].pos[1]]),
                    (Some(fx), _) => {
                        particles.kill(tag);
                        let p = game.towers[t].pos;
                        particles.spawn_display(&fx, [p[0], 0.0, p[1]], 1.0, 0.0, false, Some(tag));
                        // (On the water, under the bloons.)
                        particles.set_back(tag);
                        ice_seen.insert(id, fx);
                    }
                    (None, Some(_)) => {
                        particles.kill(tag);
                        ice_seen.remove(&id);
                    }
                    (None, None) => {}
                }
            }
            // VigilanteTowerBehavior: the mutator's buffDisplay (VigilanteBuff) on the tower while
            // the buff runs.
            for t in 0..game.towers.len() {
                let id = game.towers[t].id;
                let tag = VIG_FX_TAG | id;
                let fx = game.tier(t).and_then(|tr| tr.vigilante_fx.clone()).filter(|_| game.vigilante_active(t));
                match (fx, vig_seen.contains(&id)) {
                    (Some(fx), false) => {
                        let p = game.towers[t].pos;
                        particles.spawn_display(&fx, [p[0], 0.0, p[1]], 1.0, 0.0, false, Some(tag));
                        vig_seen.insert(id);
                    }
                    (None, true) => {
                        particles.kill(tag);
                        vig_seen.remove(&id);
                    }
                    _ => {}
                }
            }
            // An ability support zone's displayModel on each tower it buffs (Thunder's Decree), kept
            // while the zone reaches it.
            {
                let mut live: Vec<(u64, String)> = Vec::new();
                for t in 0..game.towers.len() {
                    if game.towers[t].sub.is_some() {
                        continue;
                    }
                    let id = game.towers[t].id;
                    let p = game.towers[t].pos;
                    for fx in game.zone_displays(t) {
                        let key = (id, fx.clone());
                        if !zone_seen.contains_key(&key) {
                            fx_tag += 1;
                            let tag = ZONE_FX_TAG | (fx_tag & 0xFFFF_FFFF);
                            particles.spawn_display(&fx, [p[0], 0.0, p[1]], 1.0, 0.0, false, Some(tag));
                            zone_seen.insert(key.clone(), tag);
                        }
                        live.push(key);
                    }
                }
                zone_seen.retain(|k, tag| {
                    let keep = live.contains(k);
                    if !keep {
                        particles.kill(*tag);
                    }
                    keep
                });
            }
            // SpiritOfTheForest.AddObjectAtPosition (VERIFIED-DECOMP @0x180d92f10): each vine the
            // spreading ring reaches, its effect at its rotation and scale, kept with the tower.
            for t in 0..game.towers.len() {
                let id = game.towers[t].id;
                let objs = &game.towers[t].spirit_objs;
                let done = spirit_shown.entry(id).or_insert(0);
                if *done > objs.len() {
                    *done = 0;
                    particles.kill(SPIRIT_FX_TAG | id);
                }
                for (p, name, rot, scale) in &objs[*done..] {
                    particles.spawn_display(name, [p[0], 0.0, p[1]], *scale, *rot, false, Some(SPIRIT_FX_TAG | id));
                }
                // (Ground cover: under the bloons.)
                if *done < objs.len() {
                    particles.set_back(SPIRIT_FX_TAG | id);
                }
                *done = objs.len();
            }
            spirit_shown.retain(|id, _| {
                let live = game.towers.iter().any(|t| t.id == *id);
                if !live {
                    particles.kill(SPIRIT_FX_TAG | *id);
                }
                live
            });
            // Immunity.SpawnEffects / RemoveImmunity (VERIFIED-DECOMP @0x180e36a20 / @0x180e36740):
            // the ability's effect at every track end point (Map.GetTrackEndPoints) while leaks are
            // forgiven.
            {
                let on = game.leak_immune > 0.0;
                if on && !immune_fx_on {
                    let fx = (0..game.towers.len()).find_map(|t| game.tier(t).and_then(|tr| tr.abilities.iter().find_map(|a| a.immunity_fx.clone())));
                    if let Some((fx, k)) = fx {
                        for tr in &game.tracks {
                            if let Some(e) = tr.segments().last() {
                                particles.spawn_display(&fx, [e[0], 0.0, e[1]], k, 0.0, false, Some(IMMUNE_FX_TAG));
                            }
                        }
                    }
                } else if !on && immune_fx_on {
                    particles.kill(IMMUNE_FX_TAG);
                }
                immune_fx_on = on;
            }
            vig_seen.retain(|id| {
                let live = game.towers.iter().any(|t| t.id == *id);
                if !live {
                    particles.kill(VIG_FX_TAG | *id);
                }
                live
            });
            ice_seen.retain(|id, _| {
                let live = game.towers.iter().any(|t| t.id == *id);
                if !live {
                    particles.kill(ICE_FX_TAG | *id);
                }
                live
            });
            // JungleVineEffect.Collide (VERIFIED-DECOMP @0x180cc23b0): the back display, a stand-in
            // bloon (bloonEffectDisplay) and the front display at the grabbed bloon. The stand-in
            // is the static list's entry for its baseId (White / Purple as Black, Lead as Zebra),
            // made with its camo / regrow (BloonType.Construct); effectFrames = duration x 60,
            // degradeStep = effectFrames / (index + 1).
            for (p, kind, frames, front, back) in std::mem::take(&mut game.vine_fx) {
                const LIST: [&str; 9] = ["Red", "Blue", "Green", "Yellow", "Pink", "Black", "Zebra", "Rainbow", "Ceramic"];
                let d = &game.defs[kind];
                let base = match d.base_id.as_str() {
                    "White" | "Purple" => "Black",
                    "Lead" => "Zebra",
                    b => b,
                };
                let Some(idx) = LIST.iter().position(|n| *n == base) else { continue };
                let name = game.bloon_name(kind).unwrap_or("");
                let (camo, regrow) = (d.is_camo || name.contains("Camo"), name.contains("Regrow"));
                let step = frames / (idx as i32 + 1);
                vine_tag += 1;
                let tag = VINE_FX_TAG | vine_tag;
                // (At the bloon's display height, 20, moved back along the camera tilt as the
                // bloons are drawn.)
                let at = [p[0], 20.0, p[1] - 20.0 / 60f32.to_radians().tan()];
                // (The back display draws under the bloons, the stand-in among them, the front over.)
                particles.spawn_display(&back, at, 1.0, 0.0, false, Some(tag | VINE_BACK));
                particles.set_back(tag | VINE_BACK);
                particles.spawn_display(&front, at, 1.0, 0.0, false, Some(tag));
                vines.push((tag, p, idx, camo, regrow, frames, step, step, None));
            }
            // JungleVineEffect.Process (VERIFIED-DECOMP @0x180cc3140), each sim frame: stepCount - 1;
            // below 0 it restarts at degradeStep and the stand-in steps one entry down the list;
            // when effectFrames runs out the stand-in hides and both displays take animation state
            // 2 (their End clips; gone after them).
            let vnow = game.sim_elapsed;
            let steps = vnow.saturating_sub(vine_clock).min(600) as i32;
            vine_clock = vnow;
            for _ in 0..steps {
                for v in vines.iter_mut() {
                    if let Some(e) = &mut v.8 {
                        *e -= 1;
                        continue;
                    }
                    if v.5 < 1 {
                        continue;
                    }
                    v.5 -= 1;
                    v.7 -= 1;
                    if v.7 < 0 {
                        v.7 = v.6;
                        v.2 = v.2.saturating_sub(1);
                    }
                    if v.5 <= 0 {
                        particles.set_anim_state(v.0, 2);
                        particles.set_anim_state(v.0 | VINE_BACK, 2);
                        v.8 = Some(VINE_END_FRAMES);
                    }
                }
            }
            vines.retain(|v| {
                let done = v.8.is_some_and(|e| e <= 0);
                if done {
                    particles.kill(v.0);
                    particles.kill(v.0 | VINE_BACK);
                }
                !done
            });
            // CreateEffectProjectileAfterTime (Process @0x180ca04c0): its effect once timeFrames
            // have passed since it was made (again every timeFrames with repeat), at the projectile
            // or, useTargetPosition, its target (the Sub's missile-pod FXDown, Tack Paragon meteors).
            // STAND-IN: isBuffedByRate's rate scaling of timeFrames isn't applied.
            // FlipFollowPath.Flip (VERIFIED-DECOMP @0x180c81de0) with its effect flag (the hand button,
    // OnTsmThemeAction): effectOnFlip at the tower when its hand changes.
    for t in &game.towers {
        let was = flip_seen.insert(t.id, t.flip);
        if was.is_some_and(|w| w != t.flip) {
            if let Some((fx, k)) = game.tier_of(t).and_then(|tr| tr.attacks.iter().flat_map(|a| a.weapons.iter()).find_map(|w| w.flip_fx.clone())) {
                particles.spawn(&fx, [t.pos[0], 0.0, t.pos[1]], k);
            }
        }
    }
    flip_seen.retain(|id, _| game.towers.iter().any(|t| t.id == *id));
    // SniperAmmoSwapBehavior.DoAbility (VERIFIED-DECOMP @0x180e56250): when the ammo arrives
    // (initialDelay after the ability), mainMonkeyAbilityEffect plays on the tower.
    for t in &game.towers {
        let on = t.ammo.as_ref().is_some_and(|(d, _, _)| *d <= 0.0);
        if on && ammo_seen.insert(t.id) {
            if let Some((fx, k)) = t.ammo.as_ref().and_then(|(_, _, am)| am.fx.clone()) {
                particles.spawn(&fx, [t.pos[0], 0.0, t.pos[1]], k);
            }
        } else if !on {
            ammo_seen.remove(&t.id);
        }
    }
    after_fx_seen.retain(|(uid, _), _| game.projectiles.iter().any(|p| p.uid == *uid));
            for p in &game.projectiles {
                for (i, a) in p.spec.after_fx.iter().enumerate() {
                    if a.frames < 1 {
                        continue;
                    }
                    let next = after_fx_seen.entry((p.uid, i)).or_insert(a.frames as f32);
                    if *next < 0.0 || p.age < *next {
                        continue;
                    }
                    *next = if a.repeat { p.age + a.frames as f32 } else { -1.0 };
                    let at = if a.target { p.target.and_then(|id| game.bloons.iter().find(|b| b.id == id)).map_or(p.pos, |b| b.pos) } else { p.pos };
                    particles.spawn(&a.fx, [at[0], 0.0, at[1]], a.scale);
                }
            }
            // CreateEffectOnParentOnAttackModel: as a sub-tower attacks (a new throw restarts its
            // attack_t), its effect on the parent tower (the Trance Totem's spiral on its Mermonkey).
            for t in 0..game.towers.len() {
                let tw = &game.towers[t];
                let Some((Some(fx), k)) = game.tier(t).and_then(|tr| tr.parent_attack_fx.clone()) else { continue };
                let Some(parent) = tw.sub.as_ref().and_then(|s| game.towers.iter().find(|o| o.id == s.parent)) else { continue };
                let last = parent_fx_seen.insert(tw.id, tw.attack_t).unwrap_or(f32::MAX);
                if tw.attack_t < f32::MAX && tw.attack_t < last {
                    particles.spawn(&fx, [parent.pos[0], 0.0, parent.pos[1]], k);
                }
            }
            // Simulation.CreateTextEffect (Lead to Gold's "$50"): the prefab with its text set,
            // removed after its lifespan.
            for (p, fx, text, life, h) in game.text_fx.drain(..) {
                fx_tag += 1;
                // (At a height, moved back along the camera tilt as the air units are drawn.)
                if particles.spawn_display(&fx, [p[0], h, p[1] - h / 60f32.to_radians().tan()], 1.0, 0.0, false, Some(fx_tag)) {
                    particles.set_text(fx_tag, &text);
                    degree_texts.push((fx_tag, game.sim_elapsed + (life * 60.0).round() as u64));
                }
            }
            // CreateEffectWhileAttacking (Inferno Ring): its loop while the attack has a target, the
            // exit effect as it stops.
            {
                let mut on = std::collections::HashSet::new();
                for t in 0..game.towers.len() {
                    let tw = &game.towers[t];
                    let Some([fx, _]) = game.tier(t).and_then(|tr| tr.attacks.iter().find_map(|a| a.while_fx.as_ref())) else { continue };
                    if !tw.attacking_fx {
                        continue;
                    }
                    on.insert(tw.id);
                    if while_fx_live.insert(tw.id, tw.pos).is_none() {
                        particles.spawn_tagged(fx, [tw.pos[0], 0.0, tw.pos[1]], WHILE_FX_TAG | tw.id);
                    }
                    // LinkDisplayScaleToTowerRangeModel: the loop at range / displayRadius (Inferno
                    // Ring with more range).
                    if let Some(r) = game.tier(t).and_then(|tr| tr.range_links.iter().find(|(n, _)| n == fx).map(|(_, r)| *r)).filter(|r| *r > 0.0) {
                        particles.set_scale(WHILE_FX_TAG | tw.id, game.range(t) / r);
                    }
                }
                // A tier's meshless extra displays (Mermonkey's bases, Hot Spot's glow): looping
                // effects on the tower, scaled to range / displayRadius where linked.
                {
                    const EXTRA_FX_TAG: u64 = 1 << 60;
                    let mut want: HashMap<u64, (u64, String, [f32; 2], f32)> = HashMap::new();
                    for t in 0..game.towers.len() {
                        let tw = &game.towers[t];
                        let Some(tr) = game.tier(t) else { continue };
                        for (k, ex) in tr.extras.iter().enumerate() {
                            let name = game.paragon_display(tw, &ex.name).unwrap_or_else(|| ex.name.clone());
                            if !extra_kind.get(&name).is_some_and(|k| !k.0 && k.1) {
                                continue;
                            }
                            let link = tr.range_links.iter().find(|(n, _)| *n == ex.name).map(|(_, r)| *r).filter(|r| *r > 0.0);
                            let scale = ex.scale * link.map_or(1.0, |r| game.range(t) / r);
                            want.insert(EXTRA_FX_TAG | (tw.id << 4) | k as u64, (tw.id, name, tw.pos, scale));
                        }
                        // A Monkey Fan Club fan's effect (MonkeyFanClubModel.effectId) while it's a fan.
                        // (The tower that used it: its leader effect, MonkeyFanClubModel.effectLeaderId.)
                        let leader = game.fan_leaders.contains(&tw.id);
                        if let Some(name) = tw.fan.as_ref().and_then(|(fc, _)| if leader { fc.leader_fx.clone().or(fc.fan_fx.clone()) } else { fc.fan_fx.clone() }) {
                            if *extra_kind.entry(name.clone()).or_insert_with(|| (false, root.join("effects").join(&name).join("effect.json").is_file())) == (false, true) {
                                want.insert(EXTRA_FX_TAG | (tw.id << 4) | 6, (tw.id, name, tw.pos, 1.0));
                            }
                        }
                        // CreateEffectOnAirUnitModel: kept on the air unit (Heli Downdraft's gust).
                        if let (Some(name), Some(a)) = (tr.air_fx.clone(), tw.air.as_ref()) {
                            if *extra_kind.entry(name.clone()).or_insert_with(|| (false, root.join("effects").join(&name).join("effect.json").is_file())) == (false, true) {
                                want.insert(EXTRA_FX_TAG | (tw.id << 4) | 7, (tw.id, name, a.pos, 1.0));
                            }
                        }
                        // ToggleEffectWhileAbilityActiveModel: the "not active" effect (the Druid
                        // Paragon's cyclone, by degree). STAND-IN: Primordial Wrath isn't simulated,
                        // so its active effect never shows.
                        for (ai, ab) in tr.abilities.iter().enumerate() {
                            let Some(base) = ab.toggle_fx.as_ref().and_then(|(_, off)| off.clone()) else { continue };
                            let name = game.paragon_display(tw, &base).unwrap_or(base);
                            if !extra_kind.entry(name.clone()).or_insert_with(|| (false, root.join("effects").join(&name).join("effect.json").is_file())).1 {
                                continue;
                            }
                            want.insert(EXTRA_FX_TAG | (tw.id << 4) | (8 + ai as u64).min(15), (tw.id, name, tw.pos, 1.0));
                        }
                    }
                    let stale: Vec<u64> = extra_fx_live.iter().filter(|(tag, (_, n))| want.get(*tag).is_none_or(|w| w.1 != *n)).map(|(tag, _)| *tag).collect();
                    for tag in stale {
                        extra_fx_live.remove(&tag);
                        particles.kill(tag);
                    }
                    for (tag, (id, name, p, scale)) in want {
                        if !extra_fx_live.contains_key(&tag) {
                            if !particles.spawn_tagged(&name, [p[0], 0.0, p[1]], tag) {
                                continue;
                            }
                            extra_fx_live.insert(tag, (id, name));
                        }
                        particles.set_pos(tag, [p[0], 0.0, p[1]]);
                        particles.set_scale(tag, scale);
                    }
                }
                let gone: Vec<(u64, [f32; 2])> = while_fx_live.iter().filter(|(id, _)| !on.contains(id)).map(|(id, p)| (*id, *p)).collect();
                for (id, p) in gone {
                    while_fx_live.remove(&id);
                    particles.kill(WHILE_FX_TAG | id);
                    let exit = game.towers.iter().position(|o| o.id == id).and_then(|t| game.tier(t).and_then(|tr| tr.attacks.iter().find_map(|a| a.while_fx.as_ref())).map(|f| f[1].clone()));
                    if let Some(exit) = exit {
                        particles.spawn(&exit, [p[0], 0.0, p[1]], 1.0);
                    }
                }
            }
            // A Paragon purchase (ParagonTower.StartSacrifice / MakeNewInvestmentEffect / Process
            // / Finish): an investment orb at each sacrificed tower at investmentEffectHeight
            // (display height: drawn back along the camera tilt like MOABs, over its tower), sized
            // effectMinScale..effectMaxScale over investStartScale..investEndScale cash (the Tier5
            // orb, unscaled, for a tier-5 tower). The orbs fly to the Paragon (+ its
            // investmentEffectDestYOffset in y) until 1.5 s (serialized as effectsStartMovingAtFrame)
            // along start + dir * dist * e^((t - 1) * investmentEffectExponent), then shrink to
            // nothing over effectScaleDownTime. effectDuring plays on the Paragon until Finish,
            // which plays endEffect. (Finish's degree text, ParagonDegree.prefab, isn't exported
            // yet, so it isn't shown.)
            let disp = |p: [f32; 2], h: f32| [p[0], h, p[1] - h / 60f32.to_radians().tan()];
            for s in game.sacrifices.drain(..) {
                let Some(fx) = game.towers.iter().find(|t| t.id == s.tower)
                    .and_then(|t| game.tower_tiers[t.def].as_ref()).and_then(|x| x.paragon.as_ref()).and_then(|p| p.sacrifice.clone()) else { continue };
                fx_tag += 1;
                let during = fx_tag;
                // (StartSacrifice / MakeNewInvestmentEffect / Finish: Simulation.SpawnEffect at
                // rotation 0, a persistent effect entity whose display shows at angle 180 (recording
                // dart_paragon_fx); sprite or 3D by its prefab (particles::Particles::spawn_display).)
                if let Some((name, k)) = &fx.during {
                    particles.spawn_display(name, [s.at[0], 0.0, s.at[1]], *k, 180.0, false, Some(during));
                }
                let h = fx.get("investmentEffectHeight");
                let dest = [s.at[0], s.at[1] + fx.get("investmentEffectDestYOffset")];
                let mut orbs = Vec::new();
                for &(p, cash, t5) in &s.orbs {
                    let (lo, hi) = (fx.get("investStartScale"), fx.get("investEndScale"));
                    let (mn, mx) = (fx.get("effectMinScale"), fx.get("effectMaxScale"));
                    let c = cash as f32;
                    let k = if c <= lo { mn } else if c >= hi { mx } else { (c - lo) / (hi - lo) * (mx - mn) + mn };
                    let (eff, scale) = match (&fx.invest_t5, &fx.invest) {
                        (Some((n, _)), _) if t5 => (n.clone(), 1.0),
                        (_, Some((n, m))) => (n.clone(), k * m),
                        _ => continue,
                    };
                    fx_tag += 1;
                    if particles.spawn_display(&eff, disp(p, h), scale, 180.0, false, Some(fx_tag)) {
                        let d = [dest[0] - p[0], dest[1] - p[1]];
                        let dist = (d[0] * d[0] + d[1] * d[1]).sqrt();
                        let dir = if dist > 0.0 { [d[0] / dist, d[1] / dist] } else { [0.0, 0.0] };
                        orbs.push((fx_tag, p, dir, dist, scale));
                    }
                }
                sacrifices.push((s, fx, during, orbs, false));
            }
            let frame = game.sim_elapsed;
            for (s, fx, during, orbs, ended) in sacrifices.iter_mut() {
                let el = frame.saturating_sub(s.start) as f32;
                let move_f = (fx.get("effectsStartMovingAtFrame") * 60.0).round().max(1.0);
                let shrink_f = (fx.get("effectScaleDownTime") * 60.0).round().max(1.0);
                let h = fx.get("investmentEffectHeight");
                let kx = fx.get("investmentEffectExponent");
                orbs.retain(|&(tag, p, dir, dist, scale)| {
                    if el < move_f {
                        let f = ((el / move_f - 1.0) * kx).exp() * dist;
                        particles.set_pos(tag, disp([p[0] + dir[0] * f, p[1] + dir[1] * f], h));
                        true
                    } else if el < move_f + shrink_f {
                        particles.set_pos(tag, disp([p[0] + dir[0] * dist, p[1] + dir[1] * dist], h));
                        particles.set_scale(tag, scale * (move_f + shrink_f - el) / shrink_f);
                        true
                    } else {
                        particles.kill(tag);
                        false
                    }
                });
                if !*ended && frame >= s.active_at {
                    *ended = true;
                    particles.kill(*during);
                    if let Some((name, k)) = &fx.end {
                        particles.spawn_display(name, [s.at[0], 0.0, s.at[1]], *k, 180.0, false, None);
                    }
                    // ParagonTower.Finish -> CreateDegreeText (VERIFIED-DISASM @0xd50630, Steam
                    // build): the degree (its mutator's, LocalizationManager.FormatInt) as the
                    // text of degreeTextEffectId (Simulation.CreateTextEffect, its lifespan), at
                    // the tower kept 20 inside worldXMin / worldXMax and 50 above worldZMin
                    // (game y; the clone's y is flipped).
                    if let Some((name, life)) = &fx.degree_text {
                        fx_tag += 1;
                        let at = [s.at[0].clamp(-150.0 + 20.0, 150.0 - 20.0), s.at[1].min(116.0 - 50.0)];
                        // (Scene.PresentText: a sprite display too.)
                        if particles.spawn_display(name, [at[0], 0.0, at[1]], 1.0, 0.0, true, Some(fx_tag)) {
                            particles.set_text(fx_tag, &s.degree.to_string());
                            degree_texts.push((fx_tag, frame + (life * 60.0).round() as u64));
                        }
                    }
                }
            }
            degree_texts.retain(|&(tag, end)| {
                let keep = frame < end;
                if !keep {
                    particles.kill(tag);
                }
                keep
            });
            sacrifices.retain(|(_, _, _, orbs, ended)| !*ended || !orbs.is_empty());
            // Each tower's own particle systems (Ring of Fire's flames...), restarted when its
            // models change (upgrades) and stopped when it's gone.
            {
                let mut alive = std::collections::HashSet::new();
                for t in &game.towers {
                    alive.insert(t.id);
                    let key = model_keys(&game, t, &tier_index[t.def]).0;
                    let mut want: Vec<String> = Vec::new();
                    if t.sub.is_none() && t.paragon.is_none() {
                        // (A model exported with its joints' matrices carries its effects below.)
                        let dirs = tier_index[t.def].get(&key).into_iter().chain(attack_index[t.def].get(&key).into_iter().flatten().map(|(d, _, _)| d));
                        let attached = |d: &String| tier_anims.get(&(t.def, d.clone())).and_then(|m| m.as_ref()).is_some_and(|m| m.has_attach());
                        want.extend(dirs.filter(|d| !attached(d)).filter_map(|d| tier_fx[t.def].get(d).cloned()));
                    }
                    // Avatar of Wrath's glow for how strong Wrath is now.
                    if let Some((lvl, v)) = game.towers.iter().position(|o| o.id == t.id).and_then(|i| game.wrath_level(i)) {
                        if let Some(Some(fx)) = lvl.checked_sub(1).and_then(|k| v.fx.get(k)) {
                            want.push(fx.clone());
                        }
                    }
                    // Overclock / Take Aim: the buffDisplayPath while the boost lasts.
                    if let (Some(_), Some(fx)) = (t.overclock, &t.overclock_fx) {
                        want.push(fx.clone());
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
                // OverclockInput.EnterInputMode (@0x180aad4e0): the selectionObjectPath marker on
                // every tower in the valid list; Update (@0x180aadba0) puts each at its tower every
                // frame; ExitInputMode removes them.
                // BeastHandlerMergeInput.EnterInputMode (@0x180a3e9c0) the same with the leash's
                // validMergeTargetObjectPath on each tower of its list.
                let merge_mark = merge_pick.as_ref().and_then(|(id, _, list)| {
                    let src = game.towers.iter().position(|t| t.id == *id)?;
                    Some((game.merge_marker(src)?, list.clone()))
                });
                let marker = match &merge_mark {
                    Some((fx, _)) => Some(fx.clone()),
                    None => ability_pick.as_deref().filter(|n| game.ability_targets_tower(n) && !game.ability_targets_point(n)).and_then(|n| game.ability_marker(n)),
                };
                let mut want_marks = std::collections::HashSet::new();
                if let Some(fx) = &marker {
                    for (i, t) in game.towers.iter().enumerate() {
                        let ok = match &merge_mark {
                            Some((_, list)) => list.contains(&t.id),
                            None => game.tap_target_valid(i),
                        };
                        if !ok {
                            continue;
                        }
                        want_marks.insert(t.id);
                        let at = [t.pos[0], 0.0, t.pos[1]];
                        if let Some(&tag) = pick_markers.get(&t.id) {
                            particles.set_pos(tag, at);
                        } else {
                            fx_tag += 1;
                            if particles.spawn_display(fx, at, 1.0, 0.0, true, Some(fx_tag)) {
                                pick_markers.insert(t.id, fx_tag);
                            }
                        }
                    }
                }
                pick_markers.retain(|id, tag| {
                    let keep = want_marks.contains(id);
                    if !keep {
                        particles.kill(*tag);
                    }
                    keep
                });
                // Particle systems on a model's joints (Apex Plasma Master's SunBeamCore orb on its
                // weapon): the tier prefab's effect, the display turned by the model's yaw (a 3D
                // display: RotationCache's AngleAxis about up), each joint where the model's clip
                // puts it this frame (export_animated.py attach.bin).
                for ((id, dir), &(c, at, p, yaw)) in &model_poses {
                    let Some(def) = game.towers.iter().find(|t| t.id == *id).map(|t| t.def) else { continue };
                    let Some(m) = tier_anims.get(&(def, dir.clone())).and_then(|m| m.as_ref()).filter(|m| m.has_attach()) else { continue };
                    let Some(fx) = tier_fx[def].get(dir) else { continue };
                    let key = (*id, dir.clone());
                    if attached_fx.get(&key).map(|x| &x.1) != Some(fx) {
                        if let Some((old, _)) = attached_fx.remove(&key) {
                            particles.kill(old);
                        }
                        fx_tag += 1;
                        if particles.spawn_tagged(fx, p, fx_tag) {
                            attached_fx.insert(key.clone(), (fx_tag, fx.clone()));
                        }
                    }
                    if let Some(&(tag, _)) = attached_fx.get(&key) {
                        particles.set_attach(tag, p, yaw, &m.attach_mats(c, at));
                    }
                }
                attached_fx.retain(|key, (tag, _)| {
                    let keep = model_poses.contains_key(key);
                    if !keep {
                        particles.kill(*tag);
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
                    // (A Turbo Charge / Perma Charge shot: its swapped display's trail.)
                    let disp = proj_look.get(&p.uid).and_then(|l| l.as_ref()).or(p.spec.display.as_ref());
                    let Some(fx) = disp.and_then(|g| proj_index.get(g)).and_then(|n| proj_fx.get(n)) else { continue };
                    let tag = PROJ_FX_TAG | p.uid;
                    let at = [p.pos[0], 10.0, p.pos[1] - 10.0 * tilt];
                    seen.insert(p.uid);
                    if proj_fx_live.insert(p.uid) {
                        particles.spawn_tagged(fx, at, tag);
                    } else {
                        particles.set_pos(tag, at);
                    }
                }
                // The orbiting glaives' own effects (Glaive Lord's purple Glow trail).
                for (t, tw) in game.towers.iter().enumerate() {
                    let Some(st) = tw.orbit.as_ref() else { continue };
                    let Some(fx) = game.tier(t).and_then(|tr| tr.orbit.as_ref()).and_then(|o| o.display.as_ref()).and_then(|g| proj_index.get(g)).and_then(|n| proj_fx.get(n)) else { continue };
                    for (k, g) in st.glaives.iter().enumerate() {
                        let uid = ORBIT_FX_TAG | tw.id << 8 | k as u64;
                        let at = [g[0], 10.0, g[1] - 10.0 * tilt];
                        seen.insert(uid);
                        if proj_fx_live.insert(uid) {
                            particles.spawn_tagged(fx, at, PROJ_FX_TAG | uid);
                        } else {
                            particles.set_pos(PROJ_FX_TAG | uid, at);
                        }
                    }
                }
                // ScaleProjectileOverTime's effect (Riptide's aura): ahead of the projectile by
                // effectOffset, growing with its age up to maxScale.
                for p in &game.projectiles {
                    let Some(sf) = p.spec.scale_fx.as_ref() else { continue };
                    let uid = SCALE_FX_TAG | p.uid;
                    let l = p.vel[0].hypot(p.vel[1]);
                    let d = if l > 1e-6 { [p.vel[0] / l, p.vel[1] / l] } else { [0.0, 0.0] };
                    let (x, y) = (p.pos[0] + d[0] * sf.offset, p.pos[1] + d[1] * sf.offset);
                    let at = [x, 10.0, y - 10.0 * tilt];
                    seen.insert(uid);
                    if proj_fx_live.insert(uid) {
                        particles.spawn_tagged(&sf.fx, at, PROJ_FX_TAG | uid);
                    } else {
                        particles.set_pos(PROJ_FX_TAG | uid, at);
                    }
                    particles.set_scale(PROJ_FX_TAG | uid, (sf.base + sf.per_sec * p.age / FPS).min(sf.max));
                }
                proj_fx_live.retain(|uid| {
                    let keep = seen.contains(uid);
                    if !keep {
                        particles.release(PROJ_FX_TAG | uid);
                    }
                    keep
                });
            }
            {
                let want = selected.filter(|&t| t < game.towers.len() && game.is_necro(t)).and_then(|t| game.necro_marker(t));
                if want != necro_fx {
                    particles.kill(NECRO_FX_TAG);
                    if let Some(q) = want {
                        particles.spawn_tagged("TargetGreen", [q[0], 0.0, q[1]], NECRO_FX_TAG);
                    }
                    necro_fx = want;
                }
            }
            if menu.is_none() {
                particles.update(get_frame_time().min(0.1) * speed as f32);
                map_motion_clock += get_frame_time().min(0.1) * speed as f32;
                anim::set_spin_clock(map_motion_clock);
            }
            macroquad::set_draw_tag("particles");
            {
                let a = render3d::PITCH_DEG.to_radians();
                let px_per_unit = vh / cam.fovy.max(1.0);
                particles.draw(vec3(1.0, 0.0, 0.0), render3d::gl(0.0, a.cos(), a.sin()), px_per_unit, vh);
            }
            {
            // (Drawn after the particles: the glaives sit over their own Glow trail.)
            // Glaive Lord's orbiting glaives where the simulation's Orbit put them (its eased speed,
            // Vector2.Rotate on the circle), each turned by its RotateModel: rotationFrames = angle x
            // 0.016666668 a frame (RotateModel.ctor @0x1811faeb0), so -12 degrees; the clone's y is
            // flipped, so it turns the other way here. HYPOTHESIS P-GLAIVEROT0: rotation 0 when made.
            for t in 0..game.towers.len() {
                let Some(orbit) = game.tier(t).and_then(|tier| tier.orbit.clone()) else { continue };
                let Some(disp) = orbit.display.as_ref().and_then(|g| proj_index.get(g)).and_then(|n| proj_displays.get(n)) else {
                    continue;
                };
                let Some(st) = game.towers[t].orbit.as_ref() else { continue };
                let spin = -(orbit.spin * (1.0 / 60.0) * st.age as f32).to_radians();
                for g in &st.glaives {
                    let (x, z) = (g[0], g[1] - 10.0 / 60f32.to_radians().tan());
                    match disp {
                        ProjDisplay::Sprite { tex, w, h, pivot } => render3d::flat_sprite(tex, x, 10.0, z, *w, *h, *pivot, [spin.cos(), spin.sin()]),
                        ProjDisplay::Mesh(m) => m.draw_at([x, 10.0, z], std::f32::consts::FRAC_PI_2 - spin, &mut scratch),
                        ProjDisplay::Layers(ls) => draw_layers(ls, x, 10.0, z, spin, st.age as f32 / FPS),
                    }
                }
            }
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
            // Outside the UIRect: the shop's tiled pattern (render3d::layout, Hud::draw_padding).
            let lay=render3d::layout(vw,vh);
            match hud.as_mut(){
                Some(h)=>h.draw_padding(&lay),
                None=>{
                    let ui=lay.ui;let padding=Color::from_rgba(12,14,12,255);
                    draw_rectangle(0.0,0.0,vw,ui.y,padding);draw_rectangle(0.0,ui.bottom(),vw,vh-ui.bottom(),padding);
                    draw_rectangle(0.0,0.0,ui.x,vh,padding);draw_rectangle(ui.right(),0.0,vw-ui.right(),vh,padding);
                }
            }
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
                h.draw_buffs(at, &game.buff_icons_shown(t));
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
            // CritText.prefab (ShowTextOnHitModel, lifespan 1 s): "Crit!" 18.333 above the hit in
            // TextMeshPro's four-corner gradient (gold top, orange-red bottom), played by its
            // CritTextFloatUp clip: position y 0 -> 20 over 1 s, scale 0 -> 1.5 at 1/6 s -> 1 at
            // 1/3 s, alpha 1 until 0.75 s then 0 at 1 s. INFERRED: fontSize 100 as a 10-unit em
            // (TextMeshPro 3D scale), the title font's black outline material, linear keys.
            // Effects' world-space TextMeshPro nodes (the Paragon degree): projected like the crit
            // text, fontSize / 10 world units an em, scaled by their nodes (animators applied).
            if let Some(h) = hud.as_ref() {
                for wt in particles.texts() {
                    let st = &wt.style;
                    let em = st.size / 10.0 * wt.scale.y;
                    let s0 = render3d::to_screen(&cam, [wt.at.x, wt.at.y, wt.at.z], vw, vh);
                    let (up, rt) = (wt.rot * Vec3::Y * em, wt.rot * Vec3::X);
                    let s1 = render3d::to_screen(&cam, [wt.at.x + up.x, wt.at.y + up.y, wt.at.z + up.z], vw, vh);
                    let sx = render3d::to_screen(&cam, [wt.at.x + rt.x, wt.at.y + rt.y, wt.at.z + rt.z], vw, vh);
                    let px = (s0.y - s1.y).abs();
                    if px > 0.5 {
                        let (ux, uy) = ((sx.x - s0.x).abs(), px / em.max(1e-6));
                        let (w, hh) = (st.rect[0] * wt.scale.x * ux, st.rect[1] * wt.scale.y * uy);
                        let rect = Rect::new(s0.x - w * st.pivot[0], s0.y - hh * (1.0 - st.pivot[1]), w, hh);
                        let c = st.color;
                        let g = st.gradient.unwrap_or([c; 4]).map(|q| Color::new(q[0] * c[0], q[1] * c[1], q[2] * c[2], q[3] * c[3]));
                        h.effect_text(&wt.text, rect, px, g, st.h_align, st.v_align);
                    }
                }
            }
            crit_texts.extend(game.hit_texts.drain(..).map(|p| (p, now)));
            crit_texts.retain(|(_, t0)| now - t0 < 1.0);
            if let Some(h) = hud.as_ref() {
                for (p, t0) in &crit_texts {
                    let k = (now - t0) as f32;
                    let y = 18.333 + 20.0 * k.min(1.0);
                    let scale = if k < 1.0 / 6.0 { 1.5 * k * 6.0 } else if k < 1.0 / 3.0 { 1.5 - 0.5 * (k - 1.0 / 6.0) * 6.0 } else { 1.0 };
                    let a = if k < 0.75 { 1.0 } else { (1.0 - (k - 0.75) / 0.25).max(0.0) };
                    let s0 = render3d::to_screen(&cam, [p[0], y, p[1]], vw, vh);
                    let s1 = render3d::to_screen(&cam, [p[0], y + 10.0, p[1]], vw, vh);
                    let px = (s0.y - s1.y).abs() * scale;
                    if px > 0.5 {
                        let top = Color::new(1.0, 0.7864, 0.2206, a);
                        let bottom = Color::new(1.0, 0.3103, 0.0, a);
                        h.gradient_text("Crit!", s0, px, [top, Color::new(1.0, 0.7882, 0.2196, a), bottom, Color::new(1.0, 0.3098, 0.0, a)]);
                    }
                }
            }
        } else {
            game.cash_pops.clear();
            game.hit_texts.clear();
            game.upgrade_fx.clear();
            game.place_fx.clear();
            game.text_fx.clear();
            // Map.
            let (lo, hi) = game.bounds;
            if !terrain_loaded {
                terrain_loaded = true;
                terrain = load_work::complete_asset(tex(mod_maps::terrain_file(&root,&map_key)));
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

            macroquad::set_draw_tag("2d");
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
                if in_map || place_mode != 0 {
                    let ok = game.can_place_here(def, place_at) && game.cash >= game.place_cost(def, place_at);
                    let c = view.to_screen(place_at);
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
            h.powers.armed = power_pick.clone();
            let coop_blocked=h.coop_blocks(mouse_position().into());
            // In game no front-end popup owns the input: a block a menu left set (the saved-game
            // warning's Start New used to) must not leave every HUD button dead.
            uitree::block_input(coop_blocked);
            if autotest.is_some() && !coop_blocked && game.sandbox
                && std::env::var("RRBTD6_SANDBOX_TOGGLE_FRAME").ok().and_then(|v| v.parse::<u32>().ok()) == Some(frames)
            {
                h.toggle_sandbox(&mut selected);
                eprintln!("autotest: sandbox toggle selected={selected:?} open={}", h.sandbox.open);
            }
            // RRBTD6_SET_VALUE=cash|health: opens Set Cash / Set Health at frame 30.
            if autotest.is_some() && frames == 30 {
                match std::env::var("RRBTD6_SET_VALUE").as_deref() {
                    Ok("cash") => h.sandbox.edit_resource(sandbox::Resource::Cash, &game),
                    Ok("health") => h.sandbox.edit_resource(sandbox::Resource::Lives, &game),
                    _ => {}
                }
            }
            if autotest.is_some()&&frames==30 {
                if let Ok(spec)=std::env::var("RRBTD6_MOD_EVENT"){
                    if let Some((owner,event))=spec.split_once('/') {game.mod_event(event,Some(owner));}
                }
            }
            // An end screen opening this frame: its background is the map alone, blurred
            // (Backdrop::capture_map_blur), so it's taken before the HUD draws.
            let end_opening = open_menu.is_some_and(|m| matches!(m, menus::Menu::Victory | menus::Menu::Defeat))
                || ((game.won || game.game_over) && (!end_shown || (reviewing && is_mouse_button_pressed(MouseButton::Left))));
            let mut end_map = end_opening.then(menus::Backdrop::capture_map_blur);
            // EnterCustomMode (VERIFIED-DECOMP @0x180a60e70): the selected tower is kept and
            // deselected, CancelPurchaseButton.Show, the helper message GetHelperMessage() (no
            // timeout); ExitCustomMode (@0x180a62650): the message cleared, the kept tower
            // selected again (if it's still there), CancelPurchaseButton.Hide.
            if ability_pick != pick_was {
                match (&pick_was, &ability_pick) {
                    (None, Some(name)) => {
                        pick_saved = selected.and_then(|t| game.towers.get(t)).map(|t| t.id);
                        selected = None;
                        // OverclockInput.GetHelperMessage: "Select a Monkey" (TapTowerAbilityBehavior.
                        // GetCustomInputData). The point inputs: RepositionTowerInput / RedeployInput /
                        // DeployInput.GetHelperMessage (@0x180a3ee30 / @0x180a59b20) return their
                        // CIData's helperMsg (Game::ability_point_helper), localized.
                        let tap = game.ability_targets_tower(name) && !game.ability_targets_point(name);
                        let key = if tap { Some("Select a Monkey".to_string()) } else { game.ability_point_helper(name).filter(|k| !k.is_empty()) };
                        let msg = key.map(|k| h.menus.loc.get(&k).cloned().unwrap_or(k)).unwrap_or_default();
                        h.helper.set(&msg, -1.0);
                    }
                    (Some(_), None) => {
                        h.helper.set("", -1.0);
                        selected = pick_saved.take().and_then(|id| game.towers.iter().position(|t| t.id == id));
                    }
                    _ => {}
                }
                pick_was = ability_pick.clone();
            }
            // BeastHandlerMergeInput through Enter / ExitCustomMode, as above; its helper message
            // is the MergeCIData's SelectBeastHandlerToMergeInto (GetHelperMessage @0x180a3ee30).
            if merge_pick.is_some() != merge_was {
                merge_was = merge_pick.is_some();
                if merge_was {
                    merge_saved = selected.and_then(|t| game.towers.get(t)).map(|t| t.id);
                    selected = None;
                    let msg = h.menus.loc.get("SelectBeastHandlerToMergeInto").cloned().unwrap_or_else(|| "Select a Beast Handler to merge into".into());
                    h.helper.set(&msg, -1.0);
                } else {
                    h.helper.set("", -1.0);
                    selected = merge_saved.take().and_then(|id| game.towers.iter().position(|t| t.id == id));
                }
            }
            if std::mem::take(&mut merge_invalid) {
                let msg = h.menus.loc.get("CannotMergeToTower").cloned().unwrap_or_else(|| "Cannot merge to this tower".into());
                h.helper.set(&msg, -1.0);
            }
            // A pet move's DeployCIData.helperMsg ("Select Point In Range"), localized, while it lasts.
            if pet_pick.is_some() != pet_pick_was {
                pet_pick_was = pet_pick.is_some();
                let msg = if pet_pick_was { h.menus.loc.get("Select Point In Range").cloned().unwrap_or_else(|| "Select Point In Range".into()) } else { String::new() };
                h.helper.set(&msg, -1.0);
            }
            // OverclockInput.CursorUp on a tower not in the list: SetHelperMessage(InvalidTarget, -1).
            if std::mem::take(&mut ability_invalid) {
                let msg = h.menus.loc.get("InvalidTarget").cloned().unwrap_or_else(|| "Invalid Target".into());
                h.helper.set(&msg, -1.0);
            }
            h.show_placement_buttons(placing.is_some() || ability_pick.is_some() || merge_pick.is_some(), placing.is_some() && !h.menus.drag_and_drop);
            if selected.is_some() || placing.is_some() { selected_removeable = None; }
            h.removeable = selected_removeable.and_then(|i| game.removeable(i)).map(|r| (r.cost, game.cash >= r.cost as f64));
            macroquad::set_draw_tag("hud");
            let mut actions = h.draw(&game, selected, placing, speed, auto_start);
            if coop_blocked{uitree::block_input(false);actions.clear();}
            h.draw_coop(&game,coop_config.as_ref(),front.coop_slot(),coop_runtime.as_ref().map(|r|&r.replica.requests),coop_runtime.as_ref().map_or(0,|r|r.replica.frame),&mut actions);
            hud_time += get_time() - t0;
            for a in actions {
                if !matches!(a, hud::Action::Upgrade(_) | hud::Action::Sell | hud::Action::Drag(_)) {
                    game.sfx.push(game::Sfx { name: "UIClick01".into(), group: None });
                }
                match a {
                    hud::Action::ModEvent(owner,event)=>{if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::ModEvent{owner,event});}else{game.mod_event(&event,Some(&owner));}},
                    hud::Action::ModDismiss=>{game.mod_runtime.popup=None;game.mod_runtime.revision+=1;},
                    hud::Action::Coop(action)=>{if coop_runtime.is_some(){front.coop_input(action);}},
                    // A tap only selects the shop tower (its name shows); towers are dragged out
                    // of the shop to place them (or picked up with their hotkey).
                    hud::Action::Buy(_) => {
                        placing = None;
                    }
                    hud::Action::Drag(i) => {
                        placing = Some(i);
                        selected = None;
                    }
                    hud::Action::Go => {
                        if coop_runtime.is_some(){front.coop_input(if game.round_active{rrbtd6_coop::Action::Speed{value:if speed==1{3}else{1}}}else{rrbtd6_coop::Action::StartRound});}
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
                            if game.towers[t].tiers[path] == 5 && game.paragon_quote(t).is_some() {
                                h.menus.paragon_tower = t;
                                open_menu = Some(menus::Menu::Paragon);
                            } else if !game.temple_prompt(t, path).is_some_and(|two| h.open_temple(t, path, two)) {
                                if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Upgrade{tower:game.towers[t].id,path:path as u8});}
                                else{game.upgrade(t, path);}
                            }
                        }
                    }
                    // The temple popup's OK (the tower can't have moved: the popup is modal).
                    hud::Action::TempleUpgrade(t, path) => {
                        if coop_runtime.is_some()&&t < game.towers.len() {
                            front.coop_input(rrbtd6_coop::Action::Upgrade{tower:game.towers[t].id,path:path as u8});
                        } else if t < game.towers.len() {
                            game.upgrade(t, path);
                        }
                    }
                    hud::Action::SellRemoveable => {
                        if let Some(i) = selected_removeable.take() {
                            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Removeable{index:i as u16});}else{game.sell_removeable(i);}
                        }
                    }
                    hud::Action::Sell => {
                        if let Some(t) = selected.take() {
                            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Sell{tower:game.towers[t].id});}else{game.sell(t);}
                        }
                    }
                    hud::Action::Target(m) => {
                        if let Some(t) = selected {
                            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Target{tower:game.towers[t].id,mode:coop_game::target_code(m)});}else{game.set_target(t, m);}
                        }
                    }
                    hud::Action::SetPoint => picking = selected,
                    // STAND-IN: co-op has no pet move action yet, so the button does nothing there.
                    hud::Action::MovePet(slot) => {
                        if coop_runtime.is_none() {
                            pet_pick = selected.and_then(|t| game.towers.get(t)).map(|t| (t.id, slot));
                        }
                    }
                    // STAND-IN: co-op has no merge action yet, so the button does nothing there.
                    hud::Action::MergeBeast(path) => {
                        if let Some(t) = selected.filter(|_| coop_runtime.is_none()) {
                            // BeastHandlerLeash.GetTowerBehaviorCustomInputData, MergeButton /
                            // MergeButtonSecond (VERIFIED-DECOMP @0x180cfc930): the towers
                            // IsTowerValidForMerge accepts, in the tower list's order.
                            if let Some(slot) = game.merge_slot(t, path).filter(|_| game.can_merge_button(t, path)) {
                                let list = game.merge_targets(t, slot).into_iter().map(|d| game.towers[d].id).collect();
                                merge_pick = Some((game.towers[t].id, slot, list));
                            }
                        }
                    }
                    hud::Action::DefaultPoint => {
                        if let Some(t) = selected {
                            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Tower{tower:game.towers[t].id,change:rrbtd6_coop::TowerChange::DefaultPoint});}else{game.towers[t].point = None;}
                        }
                    }
                    hud::Action::BankCollect(all) => {
                        if coop_runtime.is_some() {
                            if all {front.coop_input(rrbtd6_coop::Action::CollectBanks);}
                            else if let Some(t)=selected {front.coop_input(rrbtd6_coop::Action::Tower{tower:game.towers[t].id,change:rrbtd6_coop::TowerChange::CollectBank});}
                        } else {
                            let banks: Vec<usize> = if all { (0..game.towers.len()).filter(|&o| game.towers[o].bank > 0.0).collect() } else { selected.into_iter().collect() };
                            for o in banks {
                                game.collect_bank(o);
                            }
                        }
                    }
                    hud::Action::LeftTarget(m) => {
                        if let Some(t) = selected {
                            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Tower{tower:game.towers[t].id,change:rrbtd6_coop::TowerChange::LeftTarget(coop_game::target_code(m))});}else{game.towers[t].left_target = m;}
                        }
                    }
                    hud::Action::Reverse => {
                        if let Some(t) = selected {
                            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Tower{tower:game.towers[t].id,change:rrbtd6_coop::TowerChange::Reverse});}else{game.reverse_air(t);}
                        }
                    }
                    hud::Action::Stance => {
                        if let Some(t) = selected {
                            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Tower{tower:game.towers[t].id,change:rrbtd6_coop::TowerChange::Stance});}else{game.toggle_stance(t);}
                        }
                    }
                    hud::Action::Submerge => {
                        if let Some(t) = selected.filter(|&t| coop_runtime.is_some() && game.can_submerge(t)) {
                            front.coop_input(rrbtd6_coop::Action::Tower{tower:game.towers[t].id,change:rrbtd6_coop::TowerChange::Submerged(!game.towers[t].submerged)});
                        } else if let Some(t) = selected.filter(|&t| game.can_submerge(t)) {
                            game.towers[t].submerged = !game.towers[t].submerged;
                            if !game.towers[t].submerged && game.towers[t].target == game::TargetMode::Submerge { game.towers[t].target = game::TargetMode::First; }
                        }
                    }
                    hud::Action::ShowPops(on) => save_setting(&root, "show_pops", serde_json::json!(on)),
                    hud::Action::Hand(left) => {
                        if let Some(t) = selected {
                            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Tower{tower:game.towers[t].id,change:rrbtd6_coop::TowerChange::Hand{left}});}else{game.towers[t].flip = left;}
                        }
                    }
                    hud::Action::CamoPrio => {
                        if let Some(t) = selected {
                            if coop_runtime.is_some(){front.coop_input(rrbtd6_coop::Action::Tower{tower:game.towers[t].id,change:rrbtd6_coop::TowerChange::CamoPriority(!game.towers[t].camo_prio)});}else{game.towers[t].camo_prio = !game.towers[t].camo_prio;}
                        }
                    }
                    hud::Action::Power(id) => {
                        power_pick = Some(id);
                    }
                    hud::Action::GeraldoItem(i) => {
                        if game.item_needs_target(i) {
                            item_pick = Some(i);
                        } else if coop_runtime.is_some() {
                            front.coop_input(rrbtd6_coop::Action::Item { index: i as u16, at: None });
                        } else {
                            game.use_item(i, [0.0, 0.0]);
                        }
                    }
                    hud::Action::Close => selected = None,
                    hud::Action::Ability(name) => {
                        if let Some(runtime)=&coop_runtime{if let Some(action)=runtime.replica.ability(&game,front.coop_slot(),&name){
                            if game.ability_targets_tower(&name)||game.ability_targets_point(&name){ability_pick=Some(name);}else{front.coop_input(action);}
                        }continue;}
                        if game.ability_targets_tower(&name) || game.ability_targets_point(&name) {
                            ability_pick = Some(name);
                        } else if game.activate_ability(&name) {
                            game.stats.abilities += 1;
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
                if !end_shown && !game.sandbox && std::env::var_os("RRBTD6_AUTOTEST").is_none() && frontend::account_preview::is_local() {
                    let profile_root=root.clone();let won=game.won;let completed_round=game.round;
                    let mode=game.mode_name.clone();let coop=coop_runtime.is_some();
                    let canonical_map=local_profile::map_id(&root,&map_key);
                    let eligible=true; // Daily challenges are removed (DEVIATIONS.md 10).
                    // Trophies for the win (DEVIATIONS.md), shown on the victory screen.
                    let tier=game.map_difficulty.unwrap_or(0);
                    let reward=if won{local_profile::trophy_reward(&local_profile::get(&root),&canonical_map,&mode,coop,tier)}else{0};
                    h.menus.trophy_reward=reward;
                    std::thread::spawn(move||{if local_profile::update(&profile_root,|p|{
                        if eligible && won {p.trophies=p.trophies.saturating_add(reward);}
                        if eligible {local_profile::record_result(p,&canonical_map,&mode,coop,won,completed_round);}
                        else {p.highest_round=p.highest_round.max(completed_round);}
                    }).is_ok(){frontend::account_preview::refresh_local(&profile_root);}});

                }
                h.menus.open_end(game.won, end_shown);
                end_shown = true;
                reviewing = false;
                open_menu = Some(if game.won { menus::Menu::Victory } else { menus::Menu::Defeat });
            }
            // Snapshot the finished game frame (before the cursor) as the menu backdrop.
            if let Some(m) = open_menu.take().filter(|&m| h.menus.available(m)) {
                selected = None;
                placing = None;
                let back = match m {
                    // The end screens: the blurred map, the HUD drawn over it each frame.
                    menus::Menu::Victory | menus::Menu::Defeat => end_map.take().unwrap_or_else(menus::Backdrop::capture_map_blur),
                    menus::Menu::Paragon => menus::Backdrop::capture_sharp(),
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
        if memory_guard.tick(get_time(), get_frame_time() as f64, game.sandbox && !in_front) {
            game.clear_bloons();
        }
        if let Some(message) = memory_guard.message(get_time()) {
            draw_rectangle(0.0, 0.0, screen_width(), 34.0, Color::from_rgba(150, 20, 20, 225));
            draw_text(message, 12.0, 24.0, 22.0, WHITE);
        }
        // The game has no co-op status bar; only a stopped match says why.
        if let (Some(_),Some(e))=(&coop_runtime,coop_error.as_deref()){
            let message=format!("Co-op stopped: {e}. Esc opens Home.");
            draw_rectangle(0.0,screen_height()-30.0,screen_width()-Hud::panel_px(),30.0,Color::from_rgba(0,0,0,190));
            draw_text(&message,12.0,screen_height()-9.0,19.0,RED);
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
                // RRBTD6_END_T=<s>: the end screen that far into its opening clip;
                // RRBTD6_END_REOPEN: opened as after Review Map (Summary In).
                if matches!(m, menus::Menu::Victory | menus::Menu::Defeat) {
                    // RRBTD6_END_REGROW=<n>: that many regrow pops (a Fumigator callout).
                    if let Some(n) = std::env::var("RRBTD6_END_REGROW").ok().and_then(|v| v.parse().ok()) {
                        game.stats.regrow_pops = n;
                    }
                    // RRBTD6_END_LEAKS=Red:1,Ceramic:104: leaks just before the end (Bloons Leaked).
                    for l in std::env::var("RRBTD6_END_LEAKS").unwrap_or_default().split(',').filter(|l| !l.is_empty()) {
                        if let Some((id, d)) = l.split_once(':') {
                            game.leaks.push((id.to_string(), d.parse().unwrap_or(0.0), game.sim_elapsed));
                        }
                    }
                    if let Some(h) = hud.as_mut() {
                        h.menus.open_end(m == menus::Menu::Victory, std::env::var_os("RRBTD6_END_REOPEN").is_some());
                        h.menus.end_rewind(std::env::var("RRBTD6_END_T").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0));
                    }
                }
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
            // RRBTD6_SPAWNFX=a,b,...: those effects as displays at the map's middle at frame 10,
            // kept (tagged); RRBTD6_FXSTATE=f:v sets their animation state v at frame f.
            if frames == 10 {
                if let Ok(list) = std::env::var("RRBTD6_SPAWNFX") {
                    let (lo, hi) = game.bounds;
                    let c = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
                    for (i, n) in list.split(',').enumerate() {
                        let ok = particles.spawn_display(n.trim(), [c[0] + i as f32 * 40.0, 20.0, c[1] - 20.0 / 60f32.to_radians().tan()], 1.0, 0.0, false, Some((1 << 57) | 1));
                        eprintln!("autotest: spawnfx {n} {ok}");
                    }
                }
            }
            if let Some((f, v)) = std::env::var("RRBTD6_FXSTATE").ok().and_then(|s| s.split_once(':').and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<i32>().ok()?)))) {
                if frames == f {
                    particles.set_anim_state((1 << 57) | 1, v);
                }
            }
            // RRBTD6_FLIP=n: every tower switches hands at frame n (the hand button).
            if std::env::var("RRBTD6_FLIP").ok().and_then(|v| v.parse::<u32>().ok()) == Some(frames) {
                for tw in &mut game.towers {
                    tw.flip = !tw.flip;
                }
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
                let (draws, verts, flushes) = macroquad::take_draw_stats();
                let (gn, gv) = render3d::GEOM_STATS.with(|c| c.replace((0, 0)));
                eprintln!("autotest: frame {frames} draws/frame {} verts/frame {} flushes/frame {} geometry-calls/frame {} geometry-verts/frame {}", draws / 100, verts / 100, flushes / 100, gn / 100, gv / 100);
                let mut tags = macroquad::take_draw_tags();
                tags.sort_by(|a, b| b.1.cmp(&a.1));
                eprintln!("autotest: frame {frames} draw calls by part: {}", tags.iter().map(|(t, n)| format!("{t} {}", n / 100)).collect::<Vec<_>>().join(", "));
                let (tex_mb, buf_mb, tex_n) = macroquad::miniquad::gpu_mem::totals();
                eprintln!("autotest: frame {frames} memory private {} MB, gpu textures {tex_mb} MB ({tex_n}), gpu buffers {buf_mb} MB", private_mb());
                // RRBTD6_GPU_MEM: the textures by size, every 1000 frames.
                if frames % 1000 == 0 && std::env::var_os("RRBTD6_GPU_MEM").is_some() {
                    for ((w, h), n, b) in macroquad::miniquad::gpu_mem::by_size().iter().take(15) {
                        eprintln!("gpu-mem: {w}x{h} x{n}: {} MB", b >> 20);
                    }
                }
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
            if let Some((name,n))=bench_spawn.as_ref().filter(|_|bench_round.is_some()&&frames==bench_warmup){
                let ok=game.debug_spawn(name,*n,0.0);
                eprintln!("round-bench: spawned {n} {name} ({ok})");
            }
            let round_complete=bench_round.is_some()&&frames>bench_warmup&&!game.round_active&&game.bloons.is_empty();
            // RRBTD6_SIMFRAMES: the screenshot at that simulation frame instead (rendered frames
            // can carry several sim steps; SimRecorder's shots count sim frames).
            let sim_shot = std::env::var("RRBTD6_SIMFRAMES").ok().and_then(|v| v.parse::<u64>().ok());
            let shot_now = match sim_shot {
                Some(n) => game.sim_elapsed >= n,
                None => frames == std::env::var("RRBTD6_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(420),
            };
            if shot_now || round_complete {
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
