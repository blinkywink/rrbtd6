//! The out-of-game screens, from the game's own UI scenes (tools/assets/dump_ui_prefab.py
//! `scenefile:`): MainMenuUi (over the MainMenuWorld island), MapSelectUI, DifficultySelectUI
//! and ModeSelectUI, with CommonForegroundUI's back button. Choosing a map, a difficulty and a
//! mode starts a game.
//!
//! Scene layouts and CommonPanel controls follow the original data; account services and
//! progression are not yet bound to these UI shells.

#[path = "menu_pages.rs"]
mod menu_pages;
#[path = "season_goals.rs"]
mod season_goals;
#[path = "coop_session.rs"]
mod coop_session;
#[path = "coop_menu.rs"]
mod coop_menu;
#[path = "social_pages.rs"]
mod social_pages;
#[path = "loading.rs"]
mod loading;
#[path = "home_world.rs"]
mod home_world;
#[path = "account_preview.rs"]
pub(crate) mod account_preview;
#[path = "event_details.rs"]
mod event_details;
#[path = "home_events.rs"]
pub(crate) mod home_events;
#[path = "profile_browser.rs"]
mod profile_browser;
#[path = "jukebox.rs"]
pub(crate) mod jukebox;
#[path = "mod_browser.rs"]
mod mod_browser;
#[path = "daily_reward.rs"]
mod daily_reward;
#[path = "save_warning.rs"]
mod save_warning;
#[path = "button_motion.rs"]
mod button_motion;
pub use menu_pages::MenuPage;
use crate::hud::Hud;
use crate::uitree::{draw_tree, Ctx, Node, Rt, R};
use macroquad::prelude::*;
use std::path::{Path, PathBuf};

const MONKEYS: &str = "MainMenuCanvas/MonkeySelectScreen";

const HERO: &str = "Canvas/HeroDetailsScreen";
const SKIN_GRID: &str = "Canvas/HeroDetailsScreen/RightSideGroup/SkinButtons/Viewport/Content";
const HERO_LEVELS: &str = "Canvas/HeroDetailsScreen/UpgradeDescriptions/UpgradesScrollExpand/Viewport/Upgrades";
const HERO_GRID: &str = "Canvas/HeroDetailsScreen/LeftSideGroup/VerticalGroup/HeroSelectButtons/SelectHero/Heroes";

const MAIN: &str = "MainMenuCanvas/MainMenu";
const MAPS: &str = "MapSelectCanvas/MapSelectScreen";
const DIFF: &str = "DifficultySelectCanvas/DifficultySelectScreen";
const MODES: &str = "ModeSelectCanvas/ModeSelectScreen";
const FORE: &str = "Canvas/ForegroundScreen";
const SETTINGS: &str = "Canvas/SettingsScreen";
const ACC: &str = "Canvas/AccessibilitySettings";
const HOTKEYS: &str = "Canvas/HotkeysScreen";
const HK_CONTENT: &str = "Canvas/HotkeysScreen/Scroll/Viewport/Content";
const ACC_VIEW: &str = "Canvas/AccessibilitySettings/Panel/Scroll/Viewport";
const ACC_CONTENT: &str = "Canvas/AccessibilitySettings/Panel/Scroll/Viewport/Content";
/// AccessibleRangeCircles' options, in Advanced.range_circle order, with their loc keys.
const RANGE_CIRCLES: [(&str, &str); 4] = [("DefaultTexture", "AccessibilityRangeCircleDefault"), ("HighContrast", "AccessibilityRangeCircleHC"),
    ("ColourBlind", "AccessibilityRangeCircleCB"), ("CBHighContrast", "AccessibilityRangeCircleHCCB")];
/// The original's on/off rows: (row, the group holding its Anim toggle).
const ACC_TOGGLES: [(&str, &str); 4] = [("AmbientMapFx", "Ambient"), ("TowerSnapping", "TowerSnapping"),
    ("NumericalAbilityTimers", "NumericalAbilityTimers"), ("CashSpent", "ToggleGroup")];
/// Map select tabs, by MapDetails.difficulty.
const TIERS: [&str; 4] = ["BeginnerBtn", "IntermediateBtn", "AdvancedBtn", "ExpertBtn"];
const DIFFICULTIES: [&str; 3] = ["Easy", "Medium", "Hard"];
/// Each difficulty's mode buttons (ModeSelectScreen) -> game.json mode name.
const MODE_BUTTONS: [&[(&str, &str)]; 3] = [
    &[("Standard", "Easy"), ("PrimaryOnly", "Easy PrimaryOnly"), ("Deflation", "Easy Deflation"), ("Sandbox", "Easy Sandbox")],
    &[("Standard", "Medium"), ("MilitaryOnly", "Medium MilitaryOnly"), ("Apopalypse", "Medium Apopalypse"),
      ("Reverse", "Medium Reverse"), ("Sandbox", "Medium Sandbox")],
    &[("Standard", "Hard"), ("MagicOnly", "Hard MagicOnly"), ("DoubleMoabHealth", "Hard DoubleMoabHealth"), ("HalfCash", "Hard HalfCash"),
      ("AlternateBloonsRounds", "Hard AlternateBloonsRounds"), ("Impoppable", "Impoppable"), ("Clicks", "CHIMPS"), ("Sandbox", "Hard Sandbox")],
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Main,
    Loading,
    Menu(MenuPage),
    Maps,
    Heroes,
    Monkeys,
    Upgrades,
    Difficulty,
    Modes,
    Settings,
    /// Our own options (ExtraSettingsUI's look): see advanced.rs.
    Advanced,
    /// HotkeysUI: rebind tower and gameplay keys.
    Hotkeys,
}

pub enum FrontAction {
    /// Start a game: map directory key and game.json mode name.
    Play { map: String, mode: String },
    Coop(rrbtd6_coop::MatchStart),
    Race(serde_json::Value),
    Quit,
    Jukebox(jukebox::Change),
    Mods(Vec<String>),
    ModSettings(crate::modding::Overrides),
    ReturnToPause,
    SelectHero(String),
    SelectSkin { hero: String, skin: String },
    /// Settings sliders: (music, sound effects, hero voice), each 0..=1.
    Volume([f32; 3]),
    /// Cursor size slider: the cursor's scale (0.5 ..= 1.5).
    CursorScale(f32),
    /// The Advanced page changed a setting.
    Advanced(crate::advanced::Advanced),
    /// A hotkey was rebound (or defaults restored).
    Hotkeys(crate::keybinds::Keybinds),
}

/// Settings slider rows, in `FrontEnd::sliders` order (cursor speed only moves a gamepad's
/// cursor, so it's hidden).
const SLIDER_ROWS: [&str; 4] = ["MusicBG", "FXBG", "VOICEBG", "CursorScale"];

#[derive(serde::Deserialize, Clone)]
pub struct MapEntry {
    pub id: String,
    pub difficulty: usize,
    #[serde(default)]
    pub sprite: Option<String>,
}

#[derive(Clone, Copy, Default)]
struct ScrollState { offset: f32, velocity: f32 }

struct ScrollDrag { region: usize, start: Vec2, last: Vec2, moved: bool }

#[derive(Clone, Copy)]
struct ScrollInput { pointer: Vec2, pressed: bool, held: bool, released: bool, wheel: f32, dt: f32, scale: f32 }

impl ScrollState {
    /// Drag ownership prevents a drag in one viewport from activating another list.
    fn update(&mut self, drag: &mut Option<ScrollDrag>, region: usize, area: Rect, limit: f32, input: ScrollInput) -> bool {
        let inside = area.w > 0.0 && area.h > 0.0 && area.contains(input.pointer);
        let dt = input.dt.clamp(0.0001, 0.1);
        let mut click = false;
        if input.pressed && inside && drag.is_none() {
            *drag = Some(ScrollDrag { region, start: input.pointer, last: input.pointer, moved: false });
            self.velocity = 0.0;
        }
        let owned = drag.as_ref().is_some_and(|d| d.region == region);
        if owned {
            let d = drag.as_mut().unwrap();
            d.moved |= d.start.distance_squared(input.pointer) >= 25.0;
            if d.moved && (input.held || input.released) {
                let delta = (d.last.y - input.pointer.y) / input.scale;
                self.offset += delta;
                self.velocity += (delta / dt - self.velocity) * (dt * 10.0).min(1.0);
            }
            d.last = input.pointer;
            if input.released || !input.held {
                click = input.released && !d.moved && inside;
                *drag = None;
            }
        } else {
            // Original ScrollRects use inertia with decelerationRate .135.
            self.velocity *= 0.135_f32.powf(dt);
            if self.velocity.abs() < 1.0 { self.velocity = 0.0; }
            self.offset += self.velocity * dt;
        }
        if inside && input.wheel != 0.0 {
            // Source HeroDetailsUI ScrollRect.scrollSensitivity = 150 canvas units.
            self.offset -= input.wheel * 150.0;
            self.velocity = 0.0;
        }
        let bounded = self.offset.clamp(0.0, limit.max(0.0));
        if bounded != self.offset { self.velocity = 0.0; }
        self.offset = bounded;
        click
    }
}

fn scroll_limit(tree: &Node, viewport: &str, content: &str, scale: f32) -> f32 {
    let Some(view) = tree.rect(viewport) else { return 0.0 };
    let Some(content) = tree.find(content) else { return 0.0 };
    ((content.unclipped_screen.h - view.h) / scale).max(0.0)
}

const ADV_SECTIONS: [&str;3] = ["Default","Graphics","UI"];

fn advanced_row_section(name:&str)->usize{
    match name.strip_prefix("Ours").and_then(|v|v.parse::<usize>().ok()){
        Some(1..=6 | 20)=>1,
        Some(_)=>2,
        None=>0,
    }
}

fn advanced_sections(tree:&mut Node,expanded:[bool;3]){
    if let Some(content)=tree.find_mut(ACC_CONTENT){
        for row in &mut content.children{
            if let Some(i)=row.name.strip_prefix("Section").and_then(|v|v.parse::<usize>().ok()){
                row.set_active("ToggleButton/OpenIcon",!expanded[i]);
                row.set_active("ToggleButton/CloseIcon",expanded[i]);
            }else{
                row.active=expanded[advanced_row_section(&row.name)];
            }
        }
    }
}

fn advanced_toggle(tree:&mut Node,path:&str,on:bool){
    // ToggleOn/ToggleOff endpoints from the original Toggle controller.
    if let Some(bg)=tree.find_mut(&format!("{path}/Bg")).and_then(|n|n.img.as_mut()){
        bg.color=if on{Color::from_rgba(129,255,0,255)}else{Color::from_rgba(56,32,15,255)};
    }
    if let Some(knob)=tree.find_mut(&format!("{path}/BtnOn")){
        knob.rt.pos.x=if on{80.0}else{-80.0};
        if let Some(img)=knob.img.as_mut(){img.color=WHITE;}
    }
    if let Some(label)=tree.find_mut(&format!("{path}/TxtOn")).and_then(|n|n.txt.as_mut()){
        label.color=if on{WHITE}else{Color::from_rgba(205,152,97,255)};
    }
}

pub struct FrontEnd {
    account_generation:u64,
    map_group_key:Option<(usize,usize)>,map_group_began:Option<f64>,
    motion_screen:Option<Screen>,motion_began:Option<f64>,motion_pending:bool,
    screen_motions:Vec<(Screen,serde_json::Value,button_motion::ButtonMotion,&'static str,&'static str)>,
    dir: PathBuf,
    pub screen: Screen,
    main: Option<Node>,
    loading: Option<loading::LoadingScreen>,
    main_runtime: serde_json::Value,
    main_open_animation: Option<f64>,
    home_buttons:Option<button_motion::ButtonMotion>,
    pages: std::collections::HashMap<MenuPage, menu_pages::Page>,
    knowledge_category: usize,
    /// KnowledgeSkillTree pagePos / pageStartPos / pageTimer (page turns ease over pageTurnTime).
    knowledge_page: (f32,f32,f32),
    heroes_ui: Option<Node>,
    backdrop: Option<(u32, u32, crate::menus::Backdrop)>,
    catalog: Option<Box<crate::game::Game>>,
    monkeys: Option<Node>,
    monkey_set: usize,
    /// UpgradeModel.xpCost by upgrade name (ui/runtime/upgrade_xp.json).
    upgrade_xp: std::collections::HashMap<String,f64>,
    heroes: Vec<crate::data::ShopItem>,
    hero_index: usize,
    hero_from: Screen,
    social_from: Screen,
    hero_scrolls: [ScrollState; 3],
    hero_drag: Option<ScrollDrag>,
    hero_levels_expanded: bool,
    hero_level: usize,
    hero_runtime: serde_json::Value,
    hero_panel_animation: Option<(&'static str, f64)>,
    hero_open_animation: Option<f64>,
    hero_bg: Option<String>,
    hero_data: serde_json::Value,
    hero_level_data: serde_json::Value,
    hero_materials: serde_json::Value,
    hero_skins: std::collections::HashMap<String,String>,
    skin_template: Option<Node>,
    skin_owner: Option<String>,
    hero_pattern: Option<HeroPattern>,
    menu_backgrounds: serde_json::Value,
    menu_patterns: std::collections::HashMap<String, HeroPattern>,
    pub selected_hero: Option<String>,
    maps: Option<Node>,
    /// MapSelectUI animators by object path (ui/runtime/MapSelectUI.json), each as its own runtime.
    map_animators: std::collections::HashMap<String, serde_json::Value>,
    diff: Option<Node>,
    save_warning:save_warning::Warning,
    modes: Option<Node>,
    settings: Option<Node>,
    /// Where the settings screen goes back to.
    settings_from: Screen,
    game_options_root: Option<Screen>,
    back_requested: bool,
    fore: Option<Node>,
    /// Maps that are exported (local_extracted/maps/<key>), in the game's order.
    list: Vec<(MapEntry, String)>,
    tier: usize,
    page: usize,
    map: usize,
    coop_setup:bool,
    net:coop_session::Session,
    difficulty: usize,
    /// The MainMenuWorld island (models/mainmenu), loaded with the menu.
    world: Option<crate::render3d::Model>,
    world_tried: bool,
    world_motion: home_world::HomeMotion,
    root: PathBuf,
    /// Settings slider values (music, effects, voice, cursor size) and the one being dragged.
    pub sliders: [f32; 4],
    drag: Option<usize>,
    advanced: Option<Node>,
    pub adv: crate::advanced::Advanced,
    /// The Trophy Store button's original height (it takes the Shop's slot when that's hidden).
    trophy_store_y: Option<f32>,
    adv_scroll: f32,
    adv_expanded: [bool; 3],
    hotkeys: Option<Node>,
    pub binds: crate::keybinds::Keybinds,
    /// The hotkey row waiting for a key press.
    rebinding: Option<String>,
    hk_scroll: f32,
    /// Advanced Hotkeys expanded.
    hk_advanced: bool,
    /// The cursor slider's slide area, to mark the default size on.
    cursor_tick: Option<Rect>,
}

/// Scene dumps wrap the Canvas, whose RectTransform the Canvas component sizes to the screen.
fn scene(path: &Path) -> Option<Node> {
    Some(scene_node(Node::load(path)?))
}
fn scene_node(mut n: Node) -> Node {
    for c in &mut n.children {
        c.rt = Rt::default();
    }
    n.hide_named(&["Jewel", "JewelIcon", "ToggleJewel"]);
    n
}

fn selected_skin<'a>(data: &'a serde_json::Value, selected: &std::collections::HashMap<String,String>, hero: &str) -> &'a serde_json::Value {
    let base = &data[hero];
    selected.get(hero).and_then(|id| base["skins"].as_array()?.iter().find(|s| s["id"].as_str() == Some(id)))
        .unwrap_or(base)
}

fn hero_collection_total(data: &serde_json::Value) -> usize {
    data.as_object().into_iter().flat_map(|heroes|heroes.values())
        .flat_map(|hero|hero["skins"].as_array().into_iter().flatten())
        .filter(|skin|skin["collection_counted"].as_bool().unwrap_or(true)).count()
}

pub(crate) struct HeroPattern { texture: Texture2D, material: Material }
impl HeroPattern {
    async fn load(root: &Path) -> Option<Self> {
        Self::load_image(root, "PowersPatternGreyBg.png").await
    }
    async fn load_image(root: &Path, name: &str) -> Option<Self> {
        Self::from_file(root, name)
    }
    /// A MenuBackgrounds texture (ui/hero_skins) with the VignetteUI tiling material.
    pub(crate) fn from_file(root: &Path, name: &str) -> Option<Self> {
        let texture = Texture2D::from_file_with_format(&std::fs::read(root.join("ui/hero_skins").join(name)).ok()?, None);
        texture.set_filter(FilterMode::Linear);
        unsafe { get_internal_gl().quad_context.texture_set_wrap(texture.raw_miniquad_id(),
            macroquad::miniquad::TextureWrap::Repeat, macroquad::miniquad::TextureWrap::Repeat); }
        let material = load_material(ShaderSource::Glsl {
            vertex: r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
uniform mat4 Model;
uniform mat4 Projection;
varying vec2 uv;
varying vec4 color;
void main() { gl_Position=Projection*Model*vec4(position,1.0); uv=texcoord; color=color0/255.0; }
"#,
            fragment: r#"#version 100
precision highp float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
uniform vec4 Parameters;
void main() {
    float vignette=clamp(1.0-pow(length(uv*2.0-1.0),Parameters.z)*Parameters.w,0.0,1.0);
    vec4 sampleColor=texture2D(Texture,uv*Parameters.xy);
    sampleColor.rgb*=vignette;
    gl_FragColor=sampleColor*color;
}
"# }, MaterialParams { uniforms: vec![macroquad::miniquad::UniformDesc::new("Parameters",macroquad::miniquad::UniformType::Float4)], ..Default::default() }).ok()?;
        Some(Self {texture,material})
    }
    fn draw(&self, tint: Color) {
        // MenuBackgrounds.HeroDetailsUI: 15 tiles per height, vignette multiplier .7;
        // VignetteUI material: _Pow=1. Native callback sets X scale to tiles*width/height.
        self.draw_config(tint, 15.0, 0.7);
    }
    pub(crate) fn draw_config(&self, tint: Color, tiles: f32, vignette: f32) {
        self.material.set_uniform("Parameters", if tiles > 0.0 {
            [tiles*screen_width()/screen_height(), tiles, 1.0, vignette.max(0.0)]
        } else { [1.0, 1.0, 1.0, 0.0] });
        gl_use_material(&self.material);
        draw_texture_ex(&self.texture,0.0,0.0,tint,DrawTextureParams {dest_size:Some(vec2(screen_width(),screen_height())),..Default::default()});
        gl_use_default_material();
    }
}

/// Settled Animator state decoded from the original streamed/dense/constant curves.
/// Missing paths remain visible in the export for investigation; no guessed coordinates.
fn apply_pose(root: &mut Node, file: &Path, controller: &str, clip: &str) {
    let Some(data) = std::fs::read_to_string(file).ok().and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()) else { return };
    apply_animation(root, &data, controller, clip, None);
}

pub(crate) fn curve_value(keys: &serde_json::Value, time: f32) -> Option<f32> {
    let key = keys.as_array()?.iter().rev().find(|k| k[0].as_f64().is_some_and(|t| t <= time as f64))?;
    let dt = time - key[0].as_f64()? as f32;
    let a = key[1].as_f64()? as f32;
    let b = key[2].as_f64()? as f32;
    let c = key[3].as_f64()? as f32;
    let d = key[4].as_f64()? as f32;
    Some(((a * dt + b) * dt + c) * dt + d)
}

/// Evaluate Unity's original streamed polynomials / dense linear samples at clip time.
pub(crate) fn apply_animation(root: &mut Node, data: &serde_json::Value, controller: &str, clip: &str, elapsed: Option<f32>) -> bool {
    let mut complete = true;
    for animator in data["animators"].as_array().into_iter().flatten() {
        if animator["controller"].as_str() != Some(controller) { continue; }
        for state in animator["clips"].as_array().into_iter().flatten() {
            if state["name"].as_str() != Some(clip) { continue; }
            let start = state["start"].as_f64().unwrap_or(0.0) as f32;
            let stop = state["stop"].as_f64().unwrap_or(0.0) as f32;
            let time = elapsed.map(|t| start + t).unwrap_or(stop).min(stop);
            complete &= time >= stop;
            for binding in state["bindings"].as_array().into_iter().flatten() {
                let Some(path) = binding["path"].as_str() else { continue };
                let Some(n) = root.find_mut(path) else { continue };
                let end = &binding["end"];
                let component = |i: usize| {
                    elapsed.and_then(|_| curve_value(&binding["curves"][i], time))
                        .or_else(|| {
                            let fallback = if elapsed.is_some() { &binding["start"] } else { end };
                            fallback[i].as_f64().map(|v|v as f32)
                        })
                };
                // Stripped/empty channels do not write a zero into the previous pose.
                // In LoadingUI's idle clip that would make the black background transparent.
                let Some(value) = component(0) else { continue };
                match binding["property"].as_str() {
                    Some("m_AnchoredPosition.x") => n.rt.pos.x = value,
                    Some("m_AnchoredPosition.y") => n.rt.pos.y = value,
                    Some("m_SizeDelta.x") => n.rt.size.x = value,
                    Some("m_SizeDelta.y") => n.rt.size.y = value,
                    Some("m_IsActive") => n.active = value != 0.0,
                    Some("m_Alpha") => n.alpha = value,
                    // Older local runtime exports leave TMP's m_fontColor channels unnamed.
                    _ if binding["property"].as_str().is_some_and(|p| p.starts_with("m_fontColor."))
                        || (binding["type"].as_i64()==Some(114) && binding["attribute"].as_u64()==Some(4185109675)) => {
                        if let Some(text)=n.txt.as_mut() {
                            match binding["property"].as_str().and_then(|p|p.rsplit('.').next()) {
                                Some("r")=>text.color.r=value,Some("g")=>text.color.g=value,
                                Some("b")=>text.color.b=value,_=>text.color.a=value,
                            }
                        }
                    }
                    Some("m_Color.r" | "m_Color.g" | "m_Color.b" | "m_Color.a") => {
                        let component = binding["property"].as_str().unwrap().rsplit('.').next().unwrap();
                        for color in n.img.as_mut().map(|i| &mut i.color).into_iter().chain(n.txt.as_mut().map(|t| &mut t.color)) {
                            match component { "r" => color.r=value, "g" => color.g=value, "b" => color.b=value, _ => color.a=value }
                        }
                    }
                    _ if binding["type"].as_i64() == Some(4) && binding["attribute"].as_i64() == Some(4) => {
                        if let Some(value)=component(2) { n.rt.rot=value; }
                    }
                    _ if binding["type"].as_i64() == Some(4) && binding["attribute"].as_i64() == Some(3) => {
                        n.rt.scale = vec2(value, component(1).unwrap_or(n.rt.scale.y));
                    }
                    _ => {}
                }
            }
        }
    }
    complete
}

/// Settings' slider row (icon, slider, percent text), for reuse on other screens (icon hidden).
fn slider_row(dir: &Path) -> Option<Node> {
    let st = scene(&dir.join("SettingsUI.json"))?;
    let mut row = st.find(&format!("{SETTINGS}/Panel/MainPanel/MainSettings/SFX/CursorScale"))?.clone();
    row.rt = Rt::default();
    row.le_min = Vec2::splat(-1.0);
    row.img = None;
    row.set_active("Icon", false);
    row.set_active("JewelBar", false);
    Some(row)
}

/// A UnityEngine.UI.Slider at `path` being dragged: pressing on it claims `slot`; while held the
/// value follows the cursor across the handle slide area. Returns the new value while dragging.
fn drag_slider(m: &Node, path: &str, slot: usize, drag: &mut Option<usize>) -> Option<f32> {
    if m.clicked(path) {
        *drag = Some(slot);
    }
    if *drag != Some(slot) {
        return None;
    }
    let r = m.rect(&format!("{path}/Handle Slide Area"))?;
    Some(((mouse_position().0 - r.x) / r.w.max(1.0)).clamp(0.0, 1.0))
}

/// Cursor size slider: 0..=1 -> 32..=128 px; the game's Normal cursor is 64 px (Assets.Cursor
/// hardware HandCursor 64x64; Large 96, X-Large 128).
pub const CURSOR_PX: (f32, f32, f32) = (32.0, 128.0, 64.0);

fn m_click(n: Option<&Node>, path: &str) -> bool {
    n.is_some_and(|n| n.clicked(path))
}

/// Directory key of a map id ("Tutorial" -> "tutorial", "#ouch" -> "-ouch").
pub fn map_key(id: &str) -> String {
    id.to_lowercase().replace('#', "-")
}

impl FrontEnd {
    pub fn coop_poll(&mut self){self.net.poll();}
    pub fn coop_send(&mut self,command:rrbtd6_coop::Command){self.net.send(command);}
    pub fn coop_input(&mut self,action:rrbtd6_coop::Action){self.net.input(action);}
    pub fn coop_frame(&mut self)->Option<rrbtd6_coop::Frame>{self.net.frames.pop_front()}
    pub fn coop_running(&self)->bool{self.net.running&&!self.net.error&&self.net.active()}
    pub fn coop_host(&self)->bool{self.net.host}
    pub fn coop_slot(&self)->u8{self.net.slot.unwrap_or(0)}
    pub fn coop_status(&self)->&str{&self.net.status}
    pub fn coop_failed(&self)->bool{self.net.error||!self.net.active()}
    pub fn coop_stop(&mut self){self.net.leave();}
    pub fn new(root: &Path) -> Self {
        let list: Vec<MapEntry> = std::fs::read_to_string(root.join("ui").join("mapset.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let list = list
            .into_iter()
            .map(|m| {
                let k = map_key(&m.id);
                (m, k)
            })
            .filter(|(_, k)| root.join("maps").join(k).join("map.json").exists())
            .collect();
        Self {
            account_generation:0,map_group_key:None,map_group_began:None,motion_screen:None,motion_began:None,motion_pending:false,screen_motions:Vec::new(),
            dir: root.join("ui").join("prefabs"),
            screen: Screen::Main,
            main: None,
            loading: None,
            main_runtime: std::fs::read_to_string(root.join("ui/runtime/MainMenuUi.json")).ok()
                .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            main_open_animation: None,
            home_buttons:None,
            pages: std::collections::HashMap::new(),
            knowledge_category: 0,
            knowledge_page: (0.0,0.0,1.0),
            heroes_ui: None,
            backdrop: None,
            catalog: None,
            monkeys: None,
            monkey_set: 0,
            upgrade_xp: std::fs::read_to_string(root.join("ui/runtime/upgrade_xp.json")).ok()
                .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            heroes: std::fs::read_to_string(root.join("ui/runtime/hero_shop.json")).ok()
                .and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default(),
            hero_index: 0,
            hero_from: Screen::Main,
            social_from: Screen::Main,
            hero_scrolls: [ScrollState::default(); 3],
            hero_drag: None,
            hero_levels_expanded: false,
            hero_level: 1,
            hero_runtime: std::fs::read_to_string(root.join("ui/runtime/HeroDetailsUI.json")).ok()
                .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            hero_panel_animation: None,
            hero_open_animation: None,
            hero_bg: None,
            hero_skins: std::fs::read_to_string(root.join("settings.json")).ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| serde_json::from_value(v["hero_skins"].clone()).ok()).unwrap_or_default(),
            skin_template: None,
            skin_owner: None,
            hero_pattern: None,
            menu_backgrounds: std::fs::read_to_string(root.join("ui/runtime/menu_backgrounds.json")).ok()
                .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            menu_patterns: std::collections::HashMap::new(),
            hero_materials: std::fs::read_to_string(root.join("ui/runtime/hero_materials.json")).ok()
                .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            hero_level_data: std::fs::read_to_string(root.join("ui/runtime/hero_levels.json")).ok()
                .and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default(),
            hero_data: std::fs::read_to_string(root.join("ui/runtime/hero_data.json")).ok()
                .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            selected_hero: std::fs::read_to_string(root.join("settings.json")).ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| v["hero"].as_str().map(str::to_owned)),
            maps: None,
            map_animators: std::fs::read_to_string(root.join("ui/runtime/MapSelectUI.json")).ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .map(|d| d["animators"].as_array().into_iter().flatten().filter_map(|a| Some((a["path"].as_str()?.to_owned(),
                    serde_json::json!({"animators":[a]})))).collect()).unwrap_or_default(),
            diff: None,
            save_warning:Default::default(),
            modes: None,
            settings: None,
            settings_from: Screen::Main,
            game_options_root: None,
            back_requested: false,
            fore: None,
            list,
            tier: 0,
            page: 0,
            map: 0,
            coop_setup:std::env::var_os("RRBTD6_COOP_SETUP").is_some(),
            net:Default::default(),
            difficulty: 1,
            world: None,
            world_tried: false,
            world_motion: home_world::HomeMotion::default(),
            root: root.to_path_buf(),
            sliders: [1.0, 1.0, 1.0, 0.5],
            drag: None,
            advanced: None,
            adv: crate::advanced::Advanced::default(),
            trophy_store_y: None,
            adv_scroll: 0.0,
            adv_expanded: [false; 3],
            hotkeys: None,
            binds: crate::keybinds::Keybinds::defaults(),
            rebinding: None,
            hk_scroll: 0.0,
            hk_advanced: false,
            cursor_tick: None,
        }
    }

    /// Loads a screen's scene the first time it's shown (the map select dump is 4 MB).
    fn ensure(&mut self) {
        let d = self.dir.clone();
        match self.screen {
            Screen::Main if self.main.is_none() => {
                self.main = scene(&d.join("MainMenuUI.json"));
                self.main_open_animation = Some(get_time());
                if let Some(m) = self.main.as_mut() {
                    apply_pose(m, &self.root.join("ui/runtime/MainMenuUi.json"), "HomeScreen", "Open");
                    mod_browser::add_home_button(m,&mut self.main_runtime);
                    m.set_active(&format!("{MAIN}/Mods"),self.adv.show_mods);
                    for n in ["EventButtonGroup", "LoadGame"] {
                        m.set_active(&format!("{MAIN}/{n}"), false);
                    }
                    m.hide_named(&["NewRibbon", "NewIcon", "NewTowerDot", "Pip", "LimitedTime", "ParagonAvailable", "SaleBanner",
                        "CoopButtonUnlockEffect", "HeroButtonUnlockEffect", "LockMessage", "Featured", "Updated", "Sale", "Flag", "MrBeast"]);
                }
            }
            Screen::Menu(page) if !self.pages.contains_key(&page) => {
                if let Some(node) = page.load(&self.dir, &self.root) { self.pages.insert(page, node); }
            }
            Screen::Monkeys if self.monkeys.is_none() => {
                self.monkeys = scene(&d.join("MonkeySelectUI.json"));
                if let Some(m) = self.monkeys.as_mut() {
                    m.hide_named(&["SeenCount", "Locked", "Giftbox", "GiftboxFillbar", "Quest",
                        "QuestLevelUnlock", "GiftboxUnlock", "NewTowerDot", "ParagonAvailable"]);
                    apply_pose(m, &self.root.join("ui/runtime/MonkeySelectUI.json"), "MonkeySelectAnimation", "Open");
                    apply_pose(m, &self.root.join("ui/runtime/MonkeySelectUI.json"), "MonkeyGroupAnimation", "Open");
                }
            }
            Screen::Heroes if self.heroes_ui.is_none() => {
                self.heroes_ui = scene(&d.join("HeroDetailsUI.json"));
                self.hero_open_animation = Some(get_time());
                if self.selected_hero.is_none() { self.selected_hero = self.heroes.first().map(|h| h.id.clone()); }
                self.hero_index = self.heroes.iter().position(|h| Some(&h.id) == self.selected_hero.as_ref()).unwrap_or(0);
                // Reproducible skin captures without changing the user's saved selection.
                if let Some((hero,skin)) = std::env::var("RRBTD6_HERO_PREVIEW").ok()
                    .and_then(|v| v.split_once(':').map(|(a,b)|(a.to_string(),b.to_string()))) {
                    if let Some(index) = self.heroes.iter().position(|h| h.id == hero) {
                        self.hero_index = index;
                        self.hero_skins.insert(hero,skin);
                    }
                }
                if let Some(m) = self.heroes_ui.as_mut() {
                    for n in ["RogueLoadout", "DescriptionPopup", "RightSideGroup/BuyHeroStoreBundle",
                        "UpgradeDescriptions/UpgradesScrollExpand/Viewport/Upgrades/DescriptionPanel",
                        "HeroDetail/HeroPortraitDouble", "HeroDetail/BuyHeroStoreBundleLarge",
                        "HeroSelectCosts/Hero Requirements", "HeroSelectCosts/Buy Hero Requirements", "HeroSelectCosts/Quest",
                        "LeftSideGroup/VerticalGroup/Fade",
                        "LeftSideGroup/VerticalGroup/HeroSelectButtons/Scrollbar"] {
                        m.set_active(&format!("{HERO}/{n}"), false);
                    }
                    self.hero_level = std::env::var("RRBTD6_HERO_LEVEL").ok().and_then(|v| v.parse().ok()).unwrap_or(1).clamp(1,20);
                    self.hero_levels_expanded = std::env::var_os("RRBTD6_HERO_EXPANDED").is_some();
                    apply_animation(m, &self.hero_runtime, "UpgradesList",
                        if self.hero_levels_expanded { "UpgradesListExpanded" } else { "UpgradesListMini" }, None);
                    m.hide_named(&["SelectFx", "ImageName", "PipNotification", "HeroButtonUnlockEffect",
                        "Lock", "StoreIcon", "Buy", "SaleBanner", "OrigDescription"]);
                    self.skin_template = m.find(&format!("{SKIN_GRID}/SkinBtn")).cloned();
                    if let Some(grid) = m.find_mut(HERO_GRID) {
                        if let Some(template) = grid.children.first().cloned() {
                            grid.children = self.heroes.iter().enumerate().map(|(i,h)| {
                                let mut n = template.clone();
                                n.name = format!("Hero{i}");
                                n.set_sprite("Hero", h.icon.as_deref());
                                if let Some(img) = n.find_mut("Hero").and_then(|n| n.img.as_mut()) { img.preserve = true; }
                                n
                            }).collect();
                        }
                    }

                }
            }
            Screen::Maps if self.maps.is_none() => {
                self.maps = scene(&d.join(if self.coop_setup{"CoopMapSelectPrepared.json"}else{"MapSelectUI.json"}));
                if let Some(m) = self.maps.as_mut() {
                    for n in ["CommunityBtn", "SearchBtnObject", "DifficultyLockFillbar", "EditorMaps"] {
                        m.set_active(&format!("{MAPS}/{n}"), false);
                    }
                    // Per-map account state: medals, unlock locks, "new", saved games.
                    m.hide_named(&["NewRibbon", "NewMapDot", "NewMapGlow", "LockedState", "Continue", "EmptyMedalContainer",
                        "SilverSparkles", "BronzeSparkles", "ChimpsSparklesFxHematite", "ChimpsGlintsFxGold", "CoopPlayersInfo",
                        "Friend", "CollectionEventIcon", "GoldenBloon", "Easy", "Medium", "Hard", "MonkeyTeams", "Medals"]);
                }
            }
            Screen::Difficulty if self.diff.is_none() => {
                self.diff = scene(&d.join("DifficultySelectUI.json"));
                if let Some(m) = self.diff.as_mut() {
                    m.hide_named(&["Rewards", "DifficultyMilestoneRewards", "BadgeImage", "Panel", "Medals"]);
                    for n in ["FriendsPanel", "ContinueSavedGame"] {
                        m.set_active(&format!("{DIFF}/{n}"), false);
                    }
                }
            }
            Screen::Modes if self.modes.is_none() => {
                self.modes = scene(&d.join("ModeSelectUI.json"));
                if let Some(m) = self.modes.as_mut() {
                    m.hide_named(&["Medal", "Lock", "Stars", "Rewards"]);
                }
            }
            Screen::Settings if self.settings.is_none() => {
                self.settings = scene(&d.join("SettingsUI.json"));
                if let Some(m) = self.settings.as_mut() {
                    // Account / platform parts.
                    for n in ["Panel/Buttons/LogoutButton", "Panel/Buttons/BackupButton/BackingDataInfo",
                        "Panel/Buttons/LanguageButton", "Panel/Buttons/Notifications", "ButtonsBottomRight", "UpdateButton",
                        "FrontierGroup", "ScreenSizePanel"] {
                        m.set_active(&format!("{SETTINGS}/{n}"), false);
                    }
                    m.set_text(&format!("{SETTINGS}/Panel/Buttons/BackupButton/Button/Text"),"Account");
                    m.hide_named(&["Ribbon"]);
                    // SettingsScreen sizes the panel at runtime (the scene's layout alone gives the
                    // slider column no width): the panel spans the Panel's width, sliders fill it.
                    for row in ["CursorSpeed", "CursorScale"] {
                        m.set_active(&format!("{SETTINGS}/Panel/MainPanel/MainSettings/SFX/{row}"), false);
                    }
                    if let Some(n) = m.find_mut(&format!("{SETTINGS}/Panel/MainPanel")) {
                        n.le_pref[0] = 2900.0;
                    }
                    for p in ["Panel/MainPanel/MainSettings", "Panel/MainPanel/MainSettings/SFX"] {
                        if let Some(n) = m.find_mut(&format!("{SETTINGS}/{p}")) {
                            n.le_flex[0] = 1.0;
                        }
                    }
                }
            }
            Screen::Advanced if self.advanced.is_none() => {
                // Preserve the original rows and their paths; section headers use the native
                // Hotkeys collapsible control, sized to this popup's content width.
                self.advanced = scene(&d.join("AccessibilitySettingsUI.json"));
                let header=scene(&d.join("HotkeysUI.json"))
                    .and_then(|m|m.find(&format!("{HK_CONTENT}/CollapsiblePanel")).cloned());
                if let Some(m) = self.advanced.as_mut() {
                    if let Some(v) = m.find_mut(ACC_VIEW) {
                        v.clip = true;
                    }
                    let rows = self.adv.rows();
                    if let Some(c) = m.find_mut(ACC_CONTENT) {
                        // ContentSizeFitter owns the height; do not also stretch it by the
                        // viewport height, which leaves a false scrollbar when collapsed.
                        c.rt.amin.y=1.0;c.rt.amax.y=1.0;
                        let tpl = |n: &str| c.children.iter().find(|r| r.name == n).cloned();
                        if let (Some(title), Some(toggle), Some(slider)) = (tpl("AdvancedTitle"), tpl("CashSpent"), tpl("EffectsScale")) {
                        let mut extra = Vec::new();
                        for (i, row) in rows.iter().enumerate() {
                            let mut r = if matches!(row, crate::advanced::Row::Slider(..)) {
                                let mut r = slider.clone();
                                // No icon: the slider takes its place; a shorter description.
                                r.set_active("FxSlider/Icon", false);
                                if let Some(sl) = r.find_mut("FxSlider/Slider") {
                                    sl.rt.pos.x = 40.0;
                                    sl.rt.size.x = -369.0;
                                }
                                if let Some(dsc) = r.find_mut("FxDescription") {
                                    dsc.rt.size.y = 200.0;
                                }
                                r.rt.size.y = 512.0;
                                r.le_pref.y = 512.0;
                                r
                            } else {
                                toggle.clone()
                            };
                            r.name = format!("Ours{i}");
                            extra.push(r);
                        }
                        extra.sort_by_key(|row| match row.name.as_str(){"Ours20"=>0,"Ours0"=>1,"Ours21"=>2,_=>3});
                        let original=std::mem::take(&mut c.children);
                        for (section,name) in ADV_SECTIONS.iter().enumerate(){
                            let mut head=header.clone().unwrap_or_else(||title.clone());
                            head.name=format!("Section{section}");
                            head.le_min=vec2(1800.0,160.0);head.le_pref=vec2(2400.0,160.0);
                            head.le_flex=vec2(1.0,-1.0);head.rt.size=vec2(2400.0,160.0);
                            head.set_text("Header",name);head.set_text("Title2",name);
                            // The whole bar toggles, including its label.
                            head.set_active("ToggleButton/Hit",false);
                            if let Some(label)=head.find_mut("Header"){
                                label.rt.amin=vec2(0.0,0.5);label.rt.amax=vec2(1.0,0.5);
                                label.rt.pos=vec2(80.0,0.0);label.rt.size=vec2(-220.0,150.0);
                            }
                            c.children.push(head);
                            if section==0{c.children.extend(original.iter().cloned());}
                            else{c.children.extend(extra.iter().filter(|r|advanced_row_section(&r.name)==section).cloned());}
                        }
                        advanced_sections(m,[false;3]);
                        }
                    }
                }
            }
            Screen::Hotkeys if self.hotkeys.is_none() => {
                self.hotkeys = scene(&d.join("HotkeysUI.json"));
                // The cursor slider (Settings' CursorScale row) replaces the three cursor boxes.
                let slider = slider_row(&d);
                if let Some(m) = self.hotkeys.as_mut() {
                    m.set_active(&format!("{HOTKEYS}/RestoreBtn/AnimatedObject"), false);
                    if let Some(v) = m.find_mut(&format!("{HOTKEYS}/Scroll/Viewport")) {
                        v.clip = true;
                    }
                    let cursor = format!("{HK_CONTENT}/TopGroup/SandboxContent/Cursor/Bg ");
                    for n in ["NormalCursorToggle", "LargeSoftwareCursorToggle", "XLargeSoftwareCursorToggle"] {
                        m.set_active(&format!("{cursor}/{n}"), false);
                    }
                    m.set_active(&format!("{HK_CONTENT}/TopGroup/SandboxContent/Cursor/Subtext"), false);
                    if let (Some(bg), Some(mut row)) = (m.find_mut(&cursor), slider) {
                        // The row (icon, slider, percent) centred under the "Cursor" title.
                        row.name = "CursorSlider".into();
                        row.rt = Rt::default();
                        row.rt.amin = vec2(0.5, 1.0);
                        row.rt.amax = vec2(0.5, 1.0);
                        row.rt.pivot = vec2(0.5, 0.5);
                        row.rt.pos = vec2(0.0, -200.0);
                        row.rt.size = vec2(1150.0, 222.0);
                        row.le_min = Vec2::splat(-1.0);
                        row.img = None;
                        row.set_active("Icon", false);
                        row.set_active("JewelBar", false);
                        bg.children.push(row);
                    }
                    // One HotkeyItem per action, named by its action id.
                    let towers: Vec<String> = crate::keybinds::TOWER_DEFAULTS.iter().map(|(t, _)| format!("tower:{t}")).collect();
                    let ids = |rows: &[crate::keybinds::Row]| rows.iter().map(|r| r.0.to_string()).collect::<Vec<_>>();
                    let mut monkeys = towers;
                    monkeys.extend(ids(&crate::keybinds::MONKEYS_EXTRA));
                    let cols: [(&str, Vec<String>); 6] = [
                        ("TopGroup/MonkeyContent", monkeys),
                        ("TopGroup/GameplayContent", ids(&crate::keybinds::GAMEPLAY)),
                        ("TopGroup/SandboxContent", ids(&crate::keybinds::SANDBOX)),
                        ("AdvancedHotkeys/MonkeysAdvanced", Vec::new()),
                        ("AdvancedHotkeys/GameplayAdvanced", ids(&crate::keybinds::GAMEPLAY_ADVANCED)),
                        ("AdvancedHotkeys/SandboxAdvanced", ids(&crate::keybinds::SANDBOX_ADVANCED)),
                    ];
                    for (col, acts) in cols {
                        if let Some(c) = m.find_mut(&format!("{HK_CONTENT}/{col}")) {
                            let Some(tpl) = c.children.iter().find(|r| r.name == "HotkeyItem").cloned() else { continue };
                            // Keep the column's other children (the sandbox Cursor box) first.
                            c.children.retain(|r| r.name != "HotkeyItem");
                            for a in acts {
                                let mut r = tpl.clone();
                                r.name = a;
                                r.set_active("BgSelected", false);
                                c.children.push(r);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        if self.screen != Screen::Main && self.fore.is_none() {
            self.fore = scene(&d.join("CommonForeground.json"));
            if let Some(f) = self.fore.as_mut() {
                for n in ["Heading", "CashInfo", "Knowledge", "ChangeHeroButton", "PerksButton", "ExtraSettings"] {
                    f.set_active(&format!("{FORE}/{n}"), false);
                }
            }
        }
    }

    pub fn begin_loading(&mut self) {
        if self.loading.is_none() { self.loading = loading::LoadingScreen::load(&self.root); }
        if let Some(screen) = &mut self.loading { screen.begin(); }
    }

    pub fn loading_open_done(&self) -> bool { self.loading.as_ref().is_none_or(|s| s.open_done()) }
    pub fn finish_loading(&mut self) { if let Some(s) = &mut self.loading { s.finish(); } }
    pub fn loading_closing(&self) -> bool { self.loading.as_ref().is_some_and(|s| s.closing()) }

    pub fn draw_loading(&mut self, hud: &mut Hud) {
        if self.loading.is_none() { self.loading = loading::LoadingScreen::load(&self.root); }
        if let Some(screen) = &mut self.loading { screen.draw(hud); }
    }

    pub fn open_game_options(&mut self,screen:Screen,backdrop:crate::menus::Backdrop) {
        self.game_options_root=Some(screen);self.screen=screen;self.back_requested=false;
        self.motion_screen=None;
        self.backdrop=Some((screen_width() as u32,screen_height() as u32,backdrop));
        self.drag=None;self.rebinding=None;
    }
    pub fn request_back(&mut self) {self.back_requested=true;}
    pub fn return_home(&mut self) {
        self.net.leave();
        self.game_options_root=None;self.backdrop=None;

        self.screen = Screen::Main;
        self.main_open_animation = Some(get_time());
        self.hero_drag = None;
    }

    fn tier_maps(&self) -> Vec<usize> {
        (0..self.list.len()).filter(|&i| self.list[i].0.difficulty == self.tier).collect()
    }

    pub(crate) async fn prepare_home(&mut self) {
        if !self.world_tried {
            self.world_tried = true;
            self.world = crate::render3d::Model::load(&self.root.join("models").join("mainmenu")).await;
        }
    }

    pub(crate) async fn prepare_home_entry(&mut self)->Vec<String>{
        self.ensure();
        self.prepare_home().await;
        if let Some(world)=self.world.as_mut(){
            while self.world_motion.preparing(){
                self.world_motion.update(&self.root,world);
                if self.world_motion.preparing(){next_frame().await;}
            }
        }
        fn visit(n:&Node,names:&mut Vec<String>){
            if !n.active{return;}
            if let Some(name)=n.img.as_ref().filter(|i|i.enabled).and_then(|i|i.sprite.as_ref()){names.push(name.clone());}
            for child in &n.children{visit(child,names);}
        }
        let mut names=Vec::new();
        if let Some(node)=&self.main{visit(node,&mut names);}
        if let Some(node)=&self.fore{visit(node,&mut names);}
        names.sort();names.dedup();names
    }

    fn prepare_screen_motion(&mut self){
        if self.motion_screen==Some(self.screen){return;}
        let map_entrance=self.motion_screen.is_none() || self.motion_screen==Some(Screen::Main) || self.motion_screen==Some(Screen::Menu(MenuPage::Social));
        self.motion_screen=Some(self.screen);self.motion_began=None;self.motion_pending=true;
        if self.screen==Screen::Maps && !map_entrance{self.motion_pending=false;}
        if self.screen==Screen::Maps{self.map_group_key=None;self.map_group_began=None;}
        if self.screen==Screen::Advanced {
            self.adv_expanded=[false;3];self.adv_scroll=0.0;self.drag=None;
            // Screenshot previews never change the saved settings.
            if std::env::var_os("RRBTD6_AUTOTEST").is_some(){
                if let Ok(groups)=std::env::var("RRBTD6_ADVANCED_SECTIONS"){
                    for (i,name) in ADV_SECTIONS.iter().enumerate(){
                        self.adv_expanded[i]=groups.split(',').any(|v|v.eq_ignore_ascii_case(name));
                    }
                }
            }
        }
        if self.screen_motions.iter().any(|(s,..)|*s==self.screen){
            if let Some((_,_,buttons,..))=self.screen_motions.iter_mut().find(|(s,..)|*s==self.screen){buttons.reset();}return;
        }
        let spec=match self.screen{
            Screen::Maps=>("MapSelectUI","MapSelectScreen","Open"),
            Screen::Difficulty=>("DifficultySelectUI","DifficultySelectScreen","Open"),
            Screen::Modes=>("ModeSelectUI","ModeSelectScreen","Open"),
            Screen::Settings=>("SettingsUI","SettingsAnimation","Open"),
            Screen::Advanced=>("AccessibilitySettingsUI","PopupWithBgAnim","PopupWithBgOpen"),
            Screen::Hotkeys=>("HotkeysUI","HotKeysAnimation","Open"),
            Screen::Monkeys=>("MonkeySelectUI","MonkeySelectAnimation","Open"),
            _=>{self.motion_pending=false;return;}
        };
        if let Some(mut data)=std::fs::read(self.root.join(format!("ui/runtime/{}.json",spec.0))).ok().and_then(|b|serde_json::from_slice::<serde_json::Value>(&b).ok()){
            let buttons=button_motion::ButtonMotion::from_runtime(&data);
            if let Some(animators)=data["animators"].as_array_mut(){animators.retain(|a|a["controller"]==spec.1 || (self.screen==Screen::Monkeys && a["controller"]=="MonkeyGroupAnimation") || (self.screen==Screen::Maps && a["controller"]=="MapGroup"));}
            self.screen_motions.push((self.screen,data,buttons,spec.1,spec.2));
        }
    }

    pub(crate) fn finish_startup(&mut self) {
        self.screen = Screen::Main;
        self.ensure();
        self.main_open_animation = Some(get_time());
    }

    pub(crate) async fn draw_home_underlay(&mut self) {
        self.draw_world_with_input(false).await;
    }

    /// Draws the island behind the menus (MainMenuWorld's camera: perspective, 27 degrees,
    /// at (0, -510, -726) pitched 19.2 degrees down).
    async fn draw_world(&mut self) {
        self.draw_world_with_input(true).await;
    }

    async fn draw_world_with_input(&mut self, allow_input: bool) {
        self.prepare_home().await;
        clear_background(Color::from_rgba(120, 196, 236, 255));
        let Some(w) = &mut self.world else { return };
        self.world_motion.update(&self.root, w);
        let pitch = 2.0 * 0.166_949_42f32.asin();
        let pos = crate::render3d::gl(0.0, -510.0, -726.0);
        let fwd = crate::render3d::gl(0.0, -pitch.sin(), pitch.cos());
        set_camera(&Camera3D {
            position: pos,
            target: pos + fwd,
            up: crate::render3d::gl(0.0, pitch.cos(), pitch.sin()),
            fovy: 26.991_467f32.to_radians(),
            aspect: Some(screen_width() / screen_height()),
            projection: Projection::Perspective,
            z_near: 100.0,
            z_far: 1300.0,
            ..Default::default()
        });
        if self.screen==Screen::Main && allow_input {
            let vp=unsafe {get_internal_gl()}.quad_gl.get_projection_matrix();
            self.world_motion.debug_interact(vp,w);
            if is_mouse_button_pressed(MouseButton::Left) {
                fn over_button(node:&Node,p:Vec2)->bool {node.active && ((node.button && node.screen.contains(p)) || node.children.iter().any(|n|over_button(n,p)))}
                let pointer=mouse_position().into();
                if !self.main.as_ref().is_some_and(|n|over_button(n,pointer)){self.world_motion.interact(pointer,vp,w);}
            }
        }
        w.draw();
        self.world_motion.draw(fwd);
        set_default_camera();
    }
}

impl Hud {
    /// Draws the front end; returns a game to start (or quit).
    pub async fn draw_front(&mut self, fe: &mut FrontEnd) -> Option<FrontAction> {
        self.account_ui.poll();
        if fe.account_generation!=self.account_ui.generation {
            fe.account_generation=self.account_ui.generation;
            fe.main=None;fe.fore=None;fe.maps=None;fe.heroes_ui=None;fe.monkeys=None;fe.pages.clear();
        }
        if self.account_ui.is_open() {
            if let Some(bg)=&self.account_ui.backdrop{bg.draw();}else{clear_background(Color::from_rgba(30,80,110,255));}
            self.draw_account_ui();return None;
        }
        if fe.screen == Screen::Loading {
            fe.draw_loading(self);
            return None;
        }
        fe.net.poll();
        if let Some(setup)=fe.net.prepared.take(){return Some(FrontAction::Coop(setup));}
        if fe.screen==Screen::Menu(MenuPage::CoopLobby){
            if let Some(room)=&fe.net.room{if let Some(index)=fe.list.iter().position(|(m,_)|m.id==room.settings.map){fe.map=index;}}
            if !fe.net.active()&&!fe.net.error&&std::env::var_os("RRBTD6_COOP_HOST").is_some(){
                let map=fe.list.get(fe.map).map(|(m,_)|m.id.clone()).unwrap_or_else(||"Tutorial".into());
                fe.net.host_or_update(&fe.root,rrbtd6_coop::Settings{map,..Default::default()});
            }
            if !fe.net.active()&&!fe.net.error{if let Ok(code)=std::env::var("RRBTD6_COOP_JOIN"){fe.net.join(&fe.root,&code);}}
        }
        fe.ensure();
        fe.prepare_screen_motion();
        if fe.game_options_root.is_some() {
            clear_background(BLACK);
            if let Some((_,_,bg))=&fe.backdrop {bg.draw();}
        } else if fe.screen == Screen::Heroes {
            if fe.hero_pattern.is_none() {
                fe.hero_pattern = HeroPattern::load(&fe.root).await;
            }
            let tint = fe.heroes.get(fe.hero_index).map(|h| selected_skin(&fe.hero_data, &fe.hero_skins, &h.id))
                .and_then(|skin| crate::ui::color(&skin["color"])).unwrap_or(WHITE);
            if let Some(pattern) = &fe.hero_pattern { pattern.draw(tint); }
        } else if let Screen::Menu(page) = fe.screen {
            clear_background(BLACK);
            let config = &fe.menu_backgrounds[page.background_key()];
            if let Some(name) = config["image"].as_str() {
                if !fe.menu_patterns.contains_key(name) {
                    if let Some(pattern) = HeroPattern::load_image(&fe.root, name).await {
                        fe.menu_patterns.insert(name.to_owned(),pattern);
                    }
                }
                if let Some(pattern) = fe.menu_patterns.get(name) {
                    let tint = fe.pages.get(&page).and_then(|p| p.background_tint())
                        .or_else(|| crate::ui::color(&config["color"])).unwrap_or(WHITE);
                    pattern.draw_config(tint,
                        config["tiles"].as_f64().unwrap_or(-1.0) as f32,
                        config["vignette"].as_f64().unwrap_or(-1.0) as f32);
                }
            } else {
                let size = (screen_width() as u32, screen_height() as u32);
                if fe.backdrop.as_ref().is_none_or(|(w,h,_)| (*w,*h) != size) {
                    fe.draw_world().await;
                    fe.backdrop = Some((size.0,size.1,crate::menus::Backdrop::capture()));
                }
                if let Some((_,_,bg)) = &fe.backdrop { bg.draw(); }
            }
        } else if fe.screen == Screen::Main {
            fe.draw_world().await;
        } else {
            let size = (screen_width() as u32, screen_height() as u32);
            if fe.backdrop.as_ref().is_none_or(|(w,h,_)| (*w,*h) != size) {
                fe.draw_world().await;
                fe.backdrop = Some((size.0, size.1, crate::menus::Backdrop::capture()));
            }
            if let Some((_,_,bg)) = &fe.backdrop {
                let tint = (fe.screen == Screen::Heroes).then(|| fe.heroes.get(fe.hero_index))
                    .flatten().and_then(|h| fe.hero_data[&h.id]["color"].as_array());
                if let Some(c) = tint.filter(|c| c.len() == 4) {
                    bg.draw_tinted(Color::new(c[0].as_f64().unwrap_or(1.0) as f32, c[1].as_f64().unwrap_or(1.0) as f32,
                        c[2].as_f64().unwrap_or(1.0) as f32, c[3].as_f64().unwrap_or(1.0) as f32));
                } else { bg.draw(); }
            }
        }
        if matches!(fe.screen, Screen::Monkeys | Screen::Upgrades) {
            // The collection browser includes every tower, regardless of the last game's mode.
            if fe.catalog.is_none() {
                fe.catalog = std::fs::read_to_string(fe.root.join("game/game.json")).ok()
                    .and_then(|s| serde_json::from_str::<crate::data::GameData>(&s).ok())
                    .map(|mut data| { data.mode = "Medium".into(); Box::new(crate::game::Game::new(data)) });
                if let Some(g) = &fe.catalog {
                    self.menus.open_upgrades(g.shop.iter().position(|s| !s.hero).unwrap_or(0));
                }
            }
        }
        if fe.screen == Screen::Upgrades {
            if let Some(g) = &fe.catalog {
                if is_key_pressed(KeyCode::Escape) || self.draw_menu(crate::menus::Menu::Upgrades, g, false).iter()
                    .any(|a| matches!(a, crate::menus::MenuAction::Close)) {
                    fe.screen = Screen::Monkeys;
                }
            } else { fe.screen = Screen::Monkeys; }
            return None;
        }
        let loc = &self.menus.loc;
        let s = Hud::canvas_scale();
        let canvas = vec2(screen_width() / s, screen_height() / s);
        let full = R { min: Vec2::ZERO, size: canvas };
        let mut out = None;
        let mut next = fe.world_motion.take_action().and_then(|a|MenuPage::from_name(&a)).map(Screen::Menu);
        let mut browse = None;
        let mut open_legends = false;
        let options_can_close=fe.rebinding.is_none();
        // The support popup (see support_popup.rs) leaves the screen behind it inert.
        let support = self.support_open()||fe.save_warning.is_open();
        crate::uitree::block_input(support);
        let back = fe.screen != Screen::Main && !support
            && (std::mem::take(&mut fe.back_requested) || is_key_pressed(KeyCode::Escape) || fe.fore.as_ref().is_some_and(|f| f.clicked(&format!("{FORE}/Back/Back"))));
        match fe.screen {
            Screen::Main => {
                if let Some(m) = fe.main.as_mut() {
                    // Our main menu toggles (Advanced settings, see DEVIATIONS.md).
                    m.set_active(&format!("{MAIN}/Mods"),fe.adv.show_mods);
                    if m.clicked(&format!("{MAIN}/Mods/Button"))||m.clicked(&format!("{MAIN}/Mods/Button/Text")){next=Some(Screen::Menu(MenuPage::Mods));}
                    m.set_active(&format!("{MAIN}/BottomButtonGroup/Legends"), !fe.adv.hide_legends);
                    m.set_active(&format!("{MAIN}/Store"), !fe.adv.hide_store);
                    // Without the Shop, the Trophy Store moves up into its slot.
                    let store_y = m.find(&format!("{MAIN}/Store")).map(|n| n.rt.pos.y);
                    if let (Some(store_y), Some(trophy)) = (store_y, m.find_mut(&format!("{MAIN}/TrophyStore"))) {
                        let home = *fe.trophy_store_y.get_or_insert(trophy.rt.pos.y);
                        trophy.rt.pos.y = if fe.adv.hide_store { store_y } else { home };
                    }
                    if let Some(page)=home_events::update(&fe.root,m,s,&fe.adv.hidden_events()){next=Some(Screen::Menu(page));}
                    if let Some(hero)=fe.selected_hero.as_deref().or_else(||fe.heroes.first().map(|h|h.id.as_str())) {
                        let skin=selected_skin(&fe.hero_data,&fe.hero_skins,hero);
                        m.set_sprite(&format!("{MAIN}/BottomButtonGroup/Heroes/HeroesAnim/Button"),skin["icon"].as_str().and_then(|s|s.strip_suffix(".png")));
                    }
                    if let Some(began) = fe.main_open_animation {
                        if apply_animation(m, &fe.main_runtime, "HomeScreen", "Open", Some((get_time()-began) as f32)) {
                            fe.main_open_animation = None;
                        }
                    }
                    fe.home_buttons.get_or_insert_with(||button_motion::ButtonMotion::from_runtime(&fe.main_runtime)).update(m);
                    if m.clicked(&format!("{MAIN}/BottomButtonGroup/Play/PlayAnim/Button")) {
                        fe.net.leave();fe.coop_setup=false;fe.maps=None;next = Some(Screen::Maps);
                    }
                    if m.clicked(&format!("{MAIN}/BottomButtonGroup/Heroes/HeroesAnim/Button")) {
                        fe.hero_from=Screen::Main;
                        next = Some(Screen::Heroes);
                    }
                    if m.clicked(&format!("{MAIN}/BottomButtonGroup/Monkeys/MonkeysAnim/Button")) {
                        next = Some(Screen::Monkeys);
                    }
                    // Legends DLC modes aren't supported: a popup over the blurred menu instead.
                    // RRBTD6_LEGENDS_POPUP opens it once, for screenshots.
                    static PREVIEW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                    let preview = std::env::var_os("RRBTD6_LEGENDS_POPUP").is_some() && !PREVIEW.swap(true, std::sync::atomic::Ordering::Relaxed);
                    if !support && (preview || m.clicked(&format!("{MAIN}/BottomButtonGroup/Legends/LegendsAnim/Button"))) {
                        open_legends = true;
                    }
                    if m.clicked(&format!("{MAIN}/Settings/Button")) {
                        fe.settings_from = Screen::Main;
                        next = Some(Screen::Settings);
                    }
                    for (path,page) in [
                        ("BottomButtonGroup/Knowledge/KnowledgeAnim/Button", MenuPage::Knowledge),
                        ("BottomButtonGroup/CoOp/CoopAnim/Button", MenuPage::Social),
                        ("BottomButtonGroup/Powers/PowersAnim/Button", MenuPage::Powers),
                        ("Achievements/Button", MenuPage::Achievements), ("Store/Button", MenuPage::Store),
                        ("PlayerInfo/PlayerAvatar/Hit", MenuPage::Profile),
                        ("TrophyStore/Button", MenuPage::TrophyStore), ("PlayerInfo/Group/MailboxButton", MenuPage::Mailbox),
                    ] {
                        if m.clicked(&format!("{MAIN}/{path}")) { next = Some(Screen::Menu(page)); }
                    }
                    if m.clicked(&format!("{MAIN}/ExitButton/Button")) || is_key_pressed(KeyCode::Escape) {
                        out = Some(FrontAction::Quit);
                    }
                }
            }
            Screen::Menu(page) => {
                let mut back=back;
                if let Some(menu) = fe.pages.get_mut(&page) {
                    if page==MenuPage::DailyReward && menu.node.clicked("Canvas/DailyRewardScreen/BgPanel/ImgClose"){back=true;}
                    if let Some(ids)=menu.mods.as_mut().and_then(|m|m.pending.take()){out=Some(FrontAction::Mods(ids));}
                    else if let Some(settings)=menu.mods.as_mut().and_then(|m|m.pending_settings.take()){out=Some(FrontAction::ModSettings(settings));}
                    if let Some(change)=menu.jukebox.as_mut().and_then(|j|j.pending.take()){out=Some(FrontAction::Jukebox(change));}
                    if back && menu.dismiss_search() {back=false;}
                    menu.animate();
                    menu.update_scroll(ScrollInput { pointer:mouse_position().into(),pressed:!support&&is_mouse_button_pressed(MouseButton::Left),
                        held:!support&&is_mouse_button_down(MouseButton::Left),released:!support&&is_mouse_button_released(MouseButton::Left),
                        wheel:if support {0.0} else {mouse_wheel().1},dt:get_frame_time(),scale:s });
                    if std::mem::take(&mut menu.home_requested){fe.net.leave();next=Some(Screen::Main);}
                    if page==MenuPage::CoopLobby{
                        if menu.coop_changed(){if let Some((entry,_))=fe.list.get(fe.map){fe.net.send(rrbtd6_coop::Command::Settings(menu.coop_settings(entry.id.clone())));}}
                        menu.bind_coop(&fe.net);
                    }
                    let join_submit=menu.take_join_submit();
                    let join_text=menu.join_text().map(str::to_owned);
                    let coop_modal=menu.coop_modal_open();
                    let m = &mut menu.node;
                    if page==MenuPage::Profile && m.clicked("Canvas/PlayerStatsScreen/Stats/Own/SettingsBtn") {self.account_ui.open(&fe.root);}
                    if page==MenuPage::Television {
                        let base="BloonsMonkeyTalesPopup/AnimatedObject/Panel";
                        if m.clicked(&format!("{base}/CloseButtonX")) || m.clicked(&format!("{base}/Buttons/CancelButton")){next=Some(Screen::Main);}
                    } else if page==MenuPage::Events {
                        let base="Canvas/GameEventsScreen/Content";
                        for (path,target) in [("DailyChallengesGroup/Standard",MenuPage::DailyStandard),("DailyChallengesGroup/Advanced",MenuPage::DailyAdvanced),("DailyChallengesGroup/Coop",MenuPage::DailyCoop),("RotatingEvents/Scroll/LayoutGroup/Content/SocialSeasons",MenuPage::SocialSeason),("RotatingEvents/Scroll/LayoutGroup/Content/Race",MenuPage::Race),("RotatingEvents/Scroll/LayoutGroup/Content/BossEvent",MenuPage::BossRules)] {
                            if m.clicked(&format!("{base}/{path}")){next=Some(Screen::Menu(target));}
                        }
                    } else if let Some(target)=page.rules_page() {
                        if page==MenuPage::Race&&m.clicked("Canvas/RaceEventScreen/Content/PlayBtn"){
                            if let Some(data)=home_events::detail(&fe.root,"raceEvent"){match crate::game::Game::validate_race_rules(&data){Ok(())=>out=Some(FrontAction::Race(data)),Err(error)=>m.set_text("Canvas/RaceEventScreen/TopInfo/Title",&error)}}
                        }
                        if page!=MenuPage::Race && m.clicked("Canvas/DailyChallengeScreen/Content/Buttons/PlayBtn") {
                            if let Some((key,_))=page.event_spec(){if let Some(mut data)=home_events::detail(&fe.root,key){
                                match crate::game::Game::validate_daily_rules(&data){
                                    Ok(())=>{data["localDaily"]=serde_json::json!(true);out=Some(FrontAction::Race(data));},
                                    Err(error)=>m.set_text("Canvas/DailyChallengeScreen/TopInfo/Title",&error),
                                }
                            }}
                        }
                        let stats="Canvas/DailyChallengeScreen/Content/StatsPopup";
                        if m.clicked("Canvas/DailyChallengeScreen/Content/StatsBtn"){m.set_active(stats,true);}
                        if m.clicked(&format!("{stats}/CloseStatsBtn")){m.set_active(stats,false);}

                        let base=if page==MenuPage::Race{"Canvas/RaceEventScreen/Content"}else{"Canvas/DailyChallengeScreen/Content"};
                        if !m.find(stats).is_some_and(|n|n.active)&&(m.clicked(&format!("{base}/ChallengeRules/RulesButton")) || m.clicked(&format!("{base}/TowerScrollRect"))){next=Some(Screen::Menu(target));}
                    } else if page==MenuPage::CoopLobby {
                        let base="Canvas/CoopLobbyScreen/BgPanel/MapGroup/MapInfoPane";
                        if let Some((entry,_))=fe.list.get(fe.map){
                            m.set_sprite(&format!("{base}/MapMask/MapImage"),entry.sprite.as_deref());
                            m.set_text(&format!("{base}/MapName"),&loc.get(&entry.id).cloned().unwrap_or_else(||crate::spaced(&entry.id)));
                        }
                        if m.clicked("Canvas/CoopLobbyScreen/LoadingObject/LoadingGame/Button"){fe.net.leave();next=Some(Screen::Menu(MenuPage::Social));}
                        if m.clicked("Canvas/CoopLobbyScreen/BgPanel/Code/SocialShareBtn"){
                            if let Some(invite)=&fe.net.invite{macroquad::miniquad::window::clipboard_set(invite);}
                        }
                        if !coop_modal&&m.clicked("Canvas/CoopLobbyScreen/SelectionButton/ReadyButton"){fe.net.ready();}
                        if !coop_modal&&(!fe.net.active()||(fe.net.host&&fe.net.room.is_some()))&&!fe.net.error&&m.clicked(base){fe.coop_setup=true;fe.maps=None;next=Some(Screen::Maps);}
                    } else if page==MenuPage::JoinMatch {
                        if join_submit||m.clicked("CoopJoinMatchCanvas/CoopJoinScreen/ButtonGroup/BgPanel/Code/GoButton"){
                            if fe.net.join(&fe.root,join_text.as_deref().unwrap_or("")){next=Some(Screen::Menu(MenuPage::CoopLobby));}
                        }
                        let popup="CoopJoinMatchCanvas/CoopJoinScreen/LoadingObject";
                        m.set_active(popup,fe.net.error);
                        if fe.net.error{m.set_text(&format!("{popup}/LoadingGame/Text"),&fe.net.status);}
                        if m.clicked(&format!("{popup}/LoadingGame/Button")){fe.net.error=false;fe.net.status.clear();m.set_active(popup,false);}
                    } else if page==MenuPage::SocialSeason {
                        let second=get_time() as u64;
                        if menu.season_second!=Some(second){home_events::bind_social_season(&fe.root,m);menu.season_second=Some(second);}
                        if m.clicked("Canvas/SocialSeasonsEventScreen/CompetitionActive/TeamsSection/GoalsBtn"){next=Some(Screen::Menu(MenuPage::SocialGoals));}
                        let base="Canvas/SocialSeasonsEventScreen/Buttons";
                        if m.clicked(&format!("{base}/TeamIsland/TeamIslandBtn")){next=Some(Screen::Menu(MenuPage::Teams));}
                        if m.clicked(&format!("{base}/Perks/PerksBtn")){fe.social_from=fe.screen;next=Some(Screen::Menu(MenuPage::Perks));}
                    } else if page==MenuPage::SocialGoals {
                        if m.clicked("Canvas/SocialSeasonsGoalsUI/Buttons/Play/PlayBtn"){next=Some(Screen::Maps);}
                    } else if page == MenuPage::QuickMatch {
                        let base="CoopQuickMatchCanvas/CoopQuickMatchScreen/BgPanel";
                        for (path,key) in [("TrackText/InfoText","AnyMap"),("DifficultyText/InfoText","AnyDifficulty"),("ModeText/InfoText","AnyMode")] {
                            m.set_text(&format!("{base}/{path}"),loc.get(key).map(String::as_str).unwrap_or(key));
                        }
                    } else if page == MenuPage::Social {
                        let base="PlaySocialCanvas/PlaySocialScreen/TopGroup/Social/Buttons";
                        for (button,target) in [("QuickMatch",MenuPage::QuickMatch),("JoinMatch",MenuPage::JoinMatch)] {
                            if m.clicked(&format!("{base}/{button}")){next=Some(Screen::Menu(target));}
                        }
                        if m.clicked(&format!("{base}/CreateMatch")){fe.coop_setup=true;fe.maps=None;next=Some(Screen::Maps);}
                        if m.clicked("PlaySocialCanvas/PlaySocialScreen/TopGroup/Teams/Buttons/SocialSeasons"){next=Some(Screen::Menu(MenuPage::SocialSeason));}
                        if m.clicked("PlaySocialCanvas/PlaySocialScreen/TopGroup/Teams/Buttons/TeamIsland"){next=Some(Screen::Menu(MenuPage::Teams));}
                        if m.clicked("PlaySocialCanvas/PlaySocialScreen/MyTeam/MyTeamButton"){next=Some(Screen::Menu(MenuPage::Teams));}
                    } else if page == MenuPage::Knowledge {
                        for (i,name) in ["Primary", "Military", "Magic", "Support", "Heroes", "Powers"].iter().enumerate() {
                            if m.clicked(&format!("Canvas/KnowledgeMain/Buttons/{name}Btn")) {
                                fe.knowledge_category = i;
                                // KnowledgeSkillTree.Open: ShowPage places the page at once.
                                fe.knowledge_page = (i as f32,i as f32,1.0);
                                next = Some(Screen::Menu(MenuPage::KnowledgeTree));
                            }
                        }
                    } else if page == MenuPage::KnowledgeTree {
                        let base = "Canvas/KnowledgeSkillTree";
                        // PageLeft / PageRight: start from the current position, wrapping past either end.
                        let (pos,_,_) = fe.knowledge_page;
                        if m.clicked(&format!("{base}/TitleBox/PageLeftBtn")) {
                            fe.knowledge_page = if fe.knowledge_category == 0 {(pos,6.0+pos,0.0)} else {(pos,pos,0.0)};
                            fe.knowledge_category = (fe.knowledge_category+5)%6;
                        }
                        if m.clicked(&format!("{base}/TitleBox/PageRightBtn")) {
                            fe.knowledge_page = if fe.knowledge_category == 5 {(pos,pos-6.0,0.0)} else {(pos,pos,0.0)};
                            fe.knowledge_category = (fe.knowledge_category+1)%6;
                        }
                        // KnowledgeSkillTree.Update: smoothstep over pageTurnTime (0.3 s), then
                        // PositionPages puts page i at -pageWidth * (pagePos - i), wrapped to +-3 pages.
                        let (mut pos,start,mut timer) = fe.knowledge_page;
                        let current = fe.knowledge_category as f32;
                        if pos != current {
                            timer = (timer + get_frame_time() / 0.3).min(1.0);
                            let e = timer * -2.0 * timer * timer + timer * 3.0 * timer;
                            pos = current * e + (1.0 - e) * start;
                        }
                        fe.knowledge_page = (pos,start,timer);
                        for i in 0..6 {
                            let mut f = pos - i as f32;
                            if f < -3.0 { f += 6.0; } else if f > 3.0 { f -= 6.0; }
                            let path = format!("{base}/Pages/Page{}",i+1);
                            m.set_active(&path, f.abs() < 1.0);
                            if let Some(page) = m.find_mut(&path) { page.rt.pos.x = -6220.0 * f; }
                        }
                        m.set_text(&format!("{base}/TitleBox/TitleTxt"), ["PRIMARY KNOWLEDGE","MILITARY KNOWLEDGE","MAGIC KNOWLEDGE","SUPPORT KNOWLEDGE","HEROES KNOWLEDGE","POWERS KNOWLEDGE"][fe.knowledge_category]);
                        menu.sync_knowledge_category(fe.knowledge_category);
                    }
                }
                if back {if page==MenuPage::CoopLobby{fe.net.leave();} next = Some(match page {
                    MenuPage::SocialGoals=>Screen::Menu(MenuPage::SocialSeason),
                    MenuPage::KnowledgeTree=>Screen::Menu(MenuPage::Knowledge),
                    MenuPage::CoopLobby|MenuPage::QuickMatch|MenuPage::JoinMatch|MenuPage::Teams=>Screen::Menu(MenuPage::Social),
                    MenuPage::Friends|MenuPage::Perks=>fe.social_from,
                    MenuPage::Jukebox=>Screen::Settings,
                    p if p.event_spec().is_some()=>Screen::Menu(p.event_parent()),
                    _=>Screen::Main,
                }); }
            }
            Screen::Loading | Screen::Upgrades => unreachable!(),
            Screen::Monkeys => {
                if let (Some(m), Some(g)) = (fe.monkeys.as_mut(), fe.catalog.as_ref()) {
                    const SETS: [&str; 4] = ["Primary", "Military", "Magic", "Support"];
                    for (i, set) in SETS.iter().enumerate() {
                        let p = format!("{MONKEYS}/TowerSetButtons/{set}");
                        if m.clicked(&p) { fe.monkey_set = i; }
                    }
                    if m.clicked(&format!("{MONKEYS}/PageLeftBtn/Button")) || is_key_pressed(KeyCode::Left) {
                        fe.monkey_set = (fe.monkey_set + 3) % 4;
                    }
                    if m.clicked(&format!("{MONKEYS}/PageRightBtn/Button")) || is_key_pressed(KeyCode::Right) {
                        fe.monkey_set = (fe.monkey_set + 1) % 4;
                    }
                    for (i, set) in SETS.iter().enumerate() {
                        m.set_active(&format!("{MONKEYS}/TowerSetButtons/{set}/BgSelectedGlow"), i == fe.monkey_set);
                    }
                    let towers: Vec<_> = g.shop.iter().enumerate().filter(|(_,t)| !t.hero && t.set == 1 << fe.monkey_set).collect();
                    let account = account_preview::monkey_account();
                    let goals = home_events::tower_unlock_goals(&fe.root);
                    for i in 0..8 {
                        let row = if i < 4 { "TopRow" } else { "BottomRow" };
                        let p = format!("{MONKEYS}/Towers/{row}/Monkey_{}", i + 1);
                        m.set_active(&p, i < towers.len());
                        let Some((index,t)) = towers.get(i) else { continue };
                        let button = format!("{p}/Button");
                        // MonkeyButton.Init loads TowerModel.portrait (the 000 MonkeyPortraits sprite), not the
                        // shop icon; the icon is only a fallback for a missing export.
                        let fallback = format!("{}Icon", t.id);
                        let portrait = crate::menus::portrait_name(&mut self.sprites, &t.id, "000", None);
                        let icon = portrait.as_deref().or_else(|| t.icon.as_deref().filter(|n| self.sprites.has(n)))
                            .or_else(|| self.sprites.has(&fallback).then_some(fallback.as_str()));
                        m.set_sprite(&format!("{button}/Monkey"), icon);
                        m.set_active(&format!("{button}/Monkey"), icon.is_some());
                        // MonkeyButton.Init: TowerContainerSpritesAsset.standard[towerSet] (the plain cards;
                        // the "Large" sunburst set is the upgrade screen portrait's).
                        m.set_sprite(&format!("{button}/Bg"), Some(["TowerContainerPrimary", "TowerContainerMilitary", "TowerContainerMagic", "TowerContainerSupport"][fe.monkey_set]));
                        m.set_text(&format!("{button}/MonkeyName"), loc.get(&t.id).map(String::as_str).unwrap_or(&t.id));
                        // MonkeyButton.Init: "XP: {towerXp:N0}", white, green (0,1,0) when the next upgrade
                        // of any path is affordable (Btd6Player.CanAcquireUpgrade: xpCost <= XP); the red pip
                        // counts those not yet viewed. Locked towers show the lock and hide their XP.
                        if let Some(account) = &account {
                            let xp = account.xp.get(&t.id).copied().unwrap_or(0.0).trunc();
                            let tiers = g.tower_defs.iter().position(|(n,_)| *n == t.id).and_then(|d| g.tower_tiers[d].as_ref());
                            let (mut affordable, mut unseen) = (false, 0);
                            for path in 0..3 {
                                let mut ups: Vec<_> = tiers.into_iter().flat_map(|tt| tt.upgrades.iter()).filter(|u| u.path == path).collect();
                                ups.sort_by_key(|u| u.tier);
                                let Some(next) = ups.into_iter().find(|u| !account.acquired.contains(&u.name)) else { continue };
                                if fe.upgrade_xp.get(&next.name).is_some_and(|&cost| cost <= xp) {
                                    affordable = true;
                                    if !account.viewed.contains(&next.name) { unseen += 1; }
                                }
                            }
                            let locked = !account.unlocked.contains(&t.id);
                            m.set_active(&format!("{button}/XpAmount"), !locked);
                            m.set_text(&format!("{button}/XpAmount"), &format!("{}: {}", loc.get("XP").map(String::as_str).unwrap_or("XP"), account_preview::number(xp)));
                            if let Some(txt) = m.find_mut(&format!("{button}/XpAmount")).and_then(|n| n.txt.as_mut()) {
                                txt.color = if affordable { Color::new(0.0,1.0,0.0,1.0) } else { WHITE };
                            }
                            m.set_active(&format!("{button}/SeenCount"), unseen > 0);
                            m.set_text(&format!("{button}/SeenCount/Text"), &unseen.to_string());
                            m.set_active(&format!("{button}/Locked"), locked);
                            // Tower gift: a locked goal tower shows the gift box (rank 30+, else "Unlock at
                            // level 30"); the selected one also shows its pops progress, capped at the goal.
                            let goal = goals.get(&t.id).copied().filter(|_| locked);
                            let level_locked = account.rank < 30.0;
                            m.set_active(&format!("{button}/Giftbox"), goal.is_some() && !level_locked);
                            m.set_active(&format!("{button}/GiftboxUnlock"), goal.is_some() && level_locked);
                            m.set_text(&format!("{button}/GiftboxUnlock/UnlockText"), &format!("{} 30", loc.get("Unlock at level").map(String::as_str).unwrap_or("Unlock at level")));
                            let selected = goal.filter(|_| account.gift_selected == t.id && !level_locked);
                            m.set_active(&format!("{button}/GiftboxFillbar"), selected.is_some());
                            if let Some(goal) = selected {
                                let done = account.gift_progress.get(&t.id).copied().unwrap_or(0.0).trunc().min(goal);
                                if let Some(img) = m.find_mut(&format!("{button}/GiftboxFillbar/Bar")).and_then(|n| n.img.as_mut()) { img.fill_amount = (done / goal) as f32; }
                                m.set_text(&format!("{button}/GiftboxFillbar/Top/Count"), &format!("{}/{}", account_preview::number(done), account_preview::number(goal)));
                            }
                            for part in ["Monkey","Bg"] {
                                if let Some(img) = m.find_mut(&format!("{button}/{part}")).and_then(|n| n.img.as_mut()) { img.gray = locked; }
                            }
                        } else {
                            m.set_active(&format!("{button}/XpAmount"), false);
                        }
                        if m.clicked(&button) {
                            browse = Some(*index);
                            next = Some(Screen::Upgrades);
                        }
                    }
                }
                if back { next = Some(Screen::Main); }
            }
            Screen::Heroes => {
                if let Some(m) = fe.heroes_ui.as_mut() {
                    if let Some(began) = fe.hero_open_animation {
                        if apply_animation(m, &fe.hero_runtime, "HeroDetailsAnimation3", "Open3", Some((get_time()-began) as f32)) {
                            fe.hero_open_animation = None;
                        }
                    }
                    let total = hero_collection_total(&fe.hero_data);
                    m.set_text(&format!("{HERO}/LeftSideGroup/HeroCount"), &format!("{total}/{total}"));
                    let input = ScrollInput { pointer: mouse_position().into(), pressed: is_mouse_button_pressed(MouseButton::Left),
                        held: is_mouse_button_down(MouseButton::Left), released: is_mouse_button_released(MouseButton::Left),
                        wheel: mouse_wheel().1, dt: get_frame_time(), scale: s };
                    let roster_view = format!("{HERO}/LeftSideGroup/VerticalGroup/HeroSelectButtons/SelectHero");
                    let limit = scroll_limit(m, &roster_view, HERO_GRID, s);
                    let roster_click = fe.hero_scrolls[0].update(&mut fe.hero_drag, 0,
                        m.rect(&roster_view).unwrap_or_default(), limit, input);
                    if let Some(grid) = m.find_mut(HERO_GRID) { grid.rt.pos.y = fe.hero_scrolls[0].offset; }
                    for (i, h) in fe.heroes.iter().enumerate() {
                        let p = format!("{HERO_GRID}/Hero{i}");
                        if roster_click && m.rect(&p).is_some_and(|r| r.contains(input.pointer)) { fe.hero_index = i; fe.hero_level = 1; fe.hero_scrolls[1] = ScrollState::default(); }
                        let skin = selected_skin(&fe.hero_data, &fe.hero_skins, &h.id);
                        let icon = skin["icon_square"].as_str().and_then(|s| s.strip_suffix(".png"));
                        m.set_sprite(&format!("{p}/Hero"), icon);
                        m.set_active(&format!("{p}/BgSelectedGlow"), i == fe.hero_index);
                        m.set_active(&format!("{p}/SelectedTick"), Some(&h.id) == fe.selected_hero.as_ref());
                    }
                    if let Some(h) = fe.heroes.get(fe.hero_index) {
                        let name = loc.get(&h.id).cloned().unwrap_or_else(|| crate::spaced(&h.id));
                        m.set_text(&format!("{HERO}/TopDetail/HeroName/TextName"), &name);
                        if fe.skin_owner.as_deref() != Some(&h.id) {
                            if let (Some(grid),Some(template)) = (m.find_mut(SKIN_GRID),fe.skin_template.as_ref()) {
                                grid.children = fe.hero_data[&h.id]["skins"].as_array().into_iter().flatten().enumerate().map(|(i,skin)| {
                                    let mut node = template.clone();
                                    node.name = format!("Skin{i}");
                                    node.active = true;
                                    node.hide_named(&["NewBanner", "NewGlow", "Notification", "OrigionalCashText", "Buy", "ComingSoonBanner", "StoreIcon", "HeroButtonUnlockEffect"]);
                                    node.set_sprite("HeroIcon",skin["icon"].as_str().and_then(|s|s.strip_suffix(".png")));
                                    node
                                }).collect();
                            }
                            fe.skin_owner = Some(h.id.clone());
                            fe.hero_scrolls[2] = ScrollState::default();
                        }
                        let skin_view = format!("{HERO}/RightSideGroup/SkinButtons/Viewport");
                        let skin_limit = scroll_limit(m, &skin_view, SKIN_GRID, s);
                        let skin_click = fe.hero_scrolls[2].update(&mut fe.hero_drag, 2,
                            m.rect(&skin_view).unwrap_or_default(), skin_limit, input);
                        if let Some(grid) = m.find_mut(SKIN_GRID) { grid.rt.pos.y = fe.hero_scrolls[2].offset; }
                        for (i,skin) in fe.hero_data[&h.id]["skins"].as_array().into_iter().flatten().enumerate() {
                            let path = format!("{SKIN_GRID}/Skin{i}");
                            if skin_click && m.rect(&path).is_some_and(|r| r.contains(input.pointer)) {
                                if let Some(id) = skin["id"].as_str() {
                                    fe.hero_skins.insert(h.id.clone(),id.into());
                                    out = Some(FrontAction::SelectSkin { hero:h.id.clone(), skin:id.into() });
                                }
                            }
                            let current = selected_skin(&fe.hero_data, &fe.hero_skins, &h.id);
                            m.set_active(&format!("{path}/SelectedIcon"), current["id"] == skin["id"]);
                        }
                        let skin = selected_skin(&fe.hero_data, &fe.hero_skins, &h.id);
                        let short = skin["short_description"].as_str().and_then(|k| loc.get(k)).map(String::as_str).unwrap_or("");
                        m.set_text(&format!("{HERO}/TopDetail/HeroName/HeroType"), short);
                        let description = skin["description"].as_str().and_then(|k| loc.get(k)).map(String::as_str).unwrap_or("");
                        m.set_text(&format!("{HERO_LEVELS}/InfoPanel/Description"), description);
                        if fe.hero_bg.as_deref() != skin["id"].as_str() {
                            if let Some(name) = skin["font_material"].as_str().and_then(|s| s.split(',').nth(1)) {
                                if let (Some(style), Some(text)) = (fe.hero_materials.get(name),
                                    m.find_mut(&format!("{HERO}/TopDetail/HeroName/TextName")).and_then(|n| n.txt.as_mut())) {
                                    text.style = crate::ui::TmpStyle::from_json(style);
                                }
                            }
                            let bg = skin["background"].as_str().and_then(|file| {
                                Node::load(&fe.dir.join(file.replace(".prefab", ".json")))
                            });
                            if let (Some(container), Some(bg)) = (m.find_mut(&format!("{HERO}/HeroBgContainer")), bg) {
                                container.children = vec![bg];
                            }
                            fe.hero_bg = skin["id"].as_str().map(str::to_owned);
                        }
                        let panel = format!("{HERO}/UpgradeDescriptions/UpgradesScrollExpand");
                        if m.clicked(&format!("{panel}/SwapInfo/Button")) || m.clicked(&format!("{panel}/SwapInfo/Button (1)")) {
                            fe.hero_levels_expanded = !fe.hero_levels_expanded;
                            fe.hero_scrolls[1].offset = 0.0;
                            fe.hero_panel_animation = Some((
                                if fe.hero_levels_expanded { "UpgradesListExpanded" } else { "UpgradesListExpandDown" }, get_time()));
                        }

                        if let Some((clip, began)) = fe.hero_panel_animation {
                            if apply_animation(m, &fe.hero_runtime, "UpgradesList", clip, Some((get_time()-began) as f32)) {
                                fe.hero_panel_animation = None;
                            }
                        }
                        let level_view = format!("{panel}/Viewport");
                        let limit = scroll_limit(m, &level_view, HERO_LEVELS, s);
                        let level_click = fe.hero_scrolls[1].update(&mut fe.hero_drag, 1,
                            m.rect(&level_view).unwrap_or_default(), limit, input);
                        if let Some(levels) = m.find_mut(HERO_LEVELS) { levels.rt.pos.y = fe.hero_scrolls[1].offset; }
                        for level in 1..=20 {
                            let suffix = if level == 1 { String::new() } else { format!(" ({})", level - 1) };
                            let card = format!("{HERO_LEVELS}/UpgradePanel{suffix}");
                            m.set_text(&format!("{card}/GroupLayout/LevelContainer/Text"), &level.to_string());
                            // SetupHeroTiers uses SkinData.alternateLocsName when provided (e.g. He-Man).
                            let loc_id = skin["alternate_locs"].as_str().filter(|s| !s.is_empty()).unwrap_or(&h.id);
                            let text = loc.get(&format!("{loc_id} Level {level} Description")).map(String::as_str).unwrap_or("");
                            m.set_text(&format!("{card}/GroupLayout/Description"), text);
                            if level_click && m.rect(&card).is_some_and(|r| r.contains(input.pointer)) { fe.hero_level = level; }
                            m.set_active(&format!("{card}/SelectedGlow"), fe.hero_level == level);
                            m.set_active(&format!("{card}/GroupLayout/StoreItem"), false);
                            let ability = fe.hero_level_data[&h.id][level.to_string()]["ability"].as_str();
                            let icon = ability.and_then(|name| self.ability_icons.get(&crate::ui::icon_key(name)));
                            let active = ability.is_some();
                            let group = format!("{card}/GroupLayout");
                            m.set_active(&format!("{group}/LevelContainer/Image"), !active && level != 20);
                            m.set_active(&format!("{group}/LevelContainer/Image Gold"), active && level != 20);
                            m.set_active(&format!("{group}/LevelContainer/Image Gold20"), level == 20);
                            m.set_active(&format!("{group}/Ability"), icon.is_some());
                            m.set_sprite(&format!("{group}/Ability/Icon"), icon.map(String::as_str));
                        }
                        let image = skin["portraits"].as_array().and_then(|ps| ps.iter().filter(|p| p["level"].as_u64().unwrap_or(1) <= fe.hero_level as u64).max_by_key(|p| p["level"].as_u64().unwrap_or(1)))
                            .and_then(|p| p["image"].as_str()).and_then(|s| s.strip_suffix(".png"));
                        m.set_sprite(&format!("{HERO}/HeroDetail/HeroPortrait"), image);
                        if let Some(img) = m.find_mut(&format!("{HERO}/HeroDetail/HeroPortrait")).and_then(|n| n.img.as_mut()) { img.preserve = true; }
                        let selected = Some(&h.id) == fe.selected_hero.as_ref();
                        m.set_active(&format!("{HERO}/HeroSelectCosts/Selected Text"), selected);
                        m.set_active(&format!("{HERO}/HeroSelectCosts/SelectHero"), !selected);
                        m.set_text(&format!("{HERO}/HeroSelectCosts/SelectHero/Text"), "SELECT");
                        for p in ["InGameCost", "Selected Text/InGameCost"] {
                            m.set_text(&format!("{HERO}/HeroSelectCosts/{p}/InGameCostText"), &crate::hud::money(h.cost));
                        }
                        if !selected && m.clicked(&format!("{HERO}/HeroSelectCosts/SelectHero")) {
                            fe.selected_hero = Some(h.id.clone());
                            out = Some(FrontAction::SelectHero(h.id.clone()));
                        }
                    }
                }
                if back { next = Some(fe.hero_from); }
            }
            Screen::Maps => {
                let maps = fe.tier_maps();
                let pages = maps.len().div_ceil(6).max(1);
                fe.page = fe.page.min(pages - 1);
                if let Some(m) = fe.maps.as_mut() {
                    for (t, name) in TIERS.iter().enumerate() {
                        let p = format!("{MAPS}/MapDifficulties/{name}");
                        m.set_active(&format!("{p}/BgSelectedGlow"), t == fe.tier);
                        if m.clicked(&p) {
                            fe.tier = t;
                            fe.page = 0;
                        }
                    }
                    // Six maps a page (Map_1..Map_6), paged with the mouse wheel / arrow keys.
                    for k in 0..6 {
                        let slot = format!("{MAPS}/LayoutGroup/MapsAndPipsGroup/Maps/Map_{}", k + 1);
                        let b = format!("{slot}/Map_{}", k + 1);
                        let i = maps.get(fe.page * 6 + k).copied();
                        m.set_active(&slot, i.is_some());
                        let Some(i) = i else { continue };
                        let (e, _) = &fe.list[i];
                        m.set_active(&format!("{b}/Continue"),!fe.coop_setup&&account_preview::saved_game(&e.id).is_some());
                        account_preview::bind_map_friends(m,&format!("{b}/Friend"),&e.id,fe.adv.hide_map_friends,false);
                        m.set_sprite(&format!("{b}/MapMask/MapImage"), e.sprite.as_deref());
                        let name = loc.get(&e.id).cloned().unwrap_or_else(|| crate::spaced(&e.id));
                        m.set_text(&format!("{b}/Text"), &name);
                        if let Some(level) = account_preview::bind_map_mode(m,&b,&e.id,fe.coop_setup) {
                            // borderAnimator (EmptyMedalContainerStates) holds the state for the level;
                            // the CHIMPS glint / sparkle trail clips loop while their objects are shown.
                            let play = |m: &mut Node, path: &str, controller: &str, clip_index: usize, looped: bool| {
                                let Some(data) = fe.map_animators.get(path) else { return };
                                let clip = &data["animators"][0]["clips"][clip_index];
                                let length = (clip["stop"].as_f64().unwrap_or(0.0) - clip["start"].as_f64().unwrap_or(0.0)).max(1e-3);
                                let time = if looped { Some(get_time().rem_euclid(length) as f32) } else { None };
                                apply_animation(m, data, controller, clip["name"].as_str().unwrap_or(""), time);
                            };
                            play(m, &format!("{b}/EmptyMedalContainer"), "EmptyMedalContainerStates", level as usize, false);
                            if level == 3 { play(m, &format!("{b}/ChimpsGlintsFxGold"), "MapGlintVfxAnimation", 0, true); }
                            if level == 4 { play(m, &format!("{b}/ChimpsSparklesFxHematite"), "MapSparkleVfxAnimation", 0, true); }
                            // MapButton.ShowMedal: a completed (already seen) medal sits in MapMedal "Unlocked";
                            // the CHIMPS medals' GodRay/NewMedal then loops MedalGlowVfxAnimation.
                            for group in ["Medals", "CoopMedals"] {
                                for difficulty in ["Easy", "Medium", "Hard"] {
                                    let row = format!("{b}/{group}/{difficulty}");
                                    let shown: Vec<String> = m.find(&row).filter(|r| r.active).map(|r| r.children.iter()
                                        .filter(|c| c.active).map(|c| c.name.clone()).collect()).unwrap_or_default();
                                    for medal in shown {
                                        let path = format!("{row}/{medal}");
                                        play(m, &path, "MapMedal", 2, false);
                                        if medal == "Clicks" || medal == "SpecialClicks" {
                                            play(m, &format!("{path}/GodRay/NewMedal"), "MedalGlowVfxAnimation", 0, true);
                                        }
                                    }
                                }
                            }
                        }
                        if m.clicked(&b) {
                            fe.map = i;
                            if fe.coop_setup{
                                let settings=fe.pages.get(&MenuPage::CoopLobby).map(|p|p.coop_settings(e.id.clone())).unwrap_or(rrbtd6_coop::Settings{map:e.id.clone(),..Default::default()});
                                fe.net.host_or_update(&fe.root,settings);
                            }
                            next = Some(if fe.coop_setup{Screen::Menu(MenuPage::CoopLobby)}else{Screen::Difficulty});
                        }
                    }
                    let (_, wheel) = mouse_wheel();
                    let right = m.clicked(&format!("{MAPS}/PageRightBtn/Button")) || is_key_pressed(KeyCode::Right) || wheel < 0.0;
                    let left = m.clicked(&format!("{MAPS}/PageLeftBtn/Button")) || is_key_pressed(KeyCode::Left) || wheel > 0.0;
                    if right{
                        if fe.page+1<pages{fe.page+=1;}
                        else if fe.tier+1<TIERS.len(){fe.tier+=1;fe.page=0;}
                    }else if left{
                        if fe.page>0{fe.page-=1;}
                        else if fe.tier>0{
                            fe.tier-=1;fe.page=fe.list.iter().filter(|(map,_)|map.difficulty as usize==fe.tier).count().div_ceil(6).saturating_sub(1);
                        }
                    }
                    m.set_active(&format!("{MAPS}/PageRightBtn"),fe.page+1<pages || fe.tier+1<TIERS.len());
                    m.set_active(&format!("{MAPS}/PageLeftBtn"),fe.page>0 || fe.tier>0);
                }
                if back {
                    next = Some(if fe.coop_setup{Screen::Menu(MenuPage::Social)}else{Screen::Main});
                }
            }
            Screen::Difficulty => {
                if let Some(m) = fe.diff.as_mut() {
                    if let Some((entry,_))=fe.list.get(fe.map){
                        account_preview::bind_map_friends(m,&format!("{DIFF}/FriendsPanel"),&entry.id,fe.adv.hide_map_friends,true);
                        m.set_active(&format!("{DIFF}/FriendsButton"),!fe.adv.hide_map_friends);
                        if m.clicked(&format!("{DIFF}/FriendsButton")){fe.social_from=fe.screen;next=Some(Screen::Menu(MenuPage::Friends));}
                        let saved=(!fe.coop_setup).then(||account_preview::saved_game(&entry.id)).flatten();
                        let base=format!("{DIFF}/ContinueSavedGame");
                        m.set_active(&base,saved.is_some());
                        if let Some(saved)=saved{
                            m.set_text(&format!("{base}/Body"),&format!("{} - {}\nRound {}",saved["modeName"].as_str().unwrap_or("Standard"),
                                saved["mapDifficulty"].as_str().unwrap_or(""),saved["round"].as_u64().unwrap_or(0)));
                            if m.clicked(&format!("{base}/Continue"))||m.clicked(&format!("{base}/Replay")){
                                let map=fe.list[fe.map].1.clone();
                                let difficulty=DIFFICULTIES.iter().position(|&d|Some(d)==saved["mapDifficulty"].as_str()).unwrap_or(1);
                                let mode=MODE_BUTTONS[difficulty].iter().find(|(id,_)|Some(*id)==saved["modeName"].as_str())
                                    .map(|(_,mode)|*mode).unwrap_or(DIFFICULTIES[difficulty]);
                                fe.save_warning.open(&fe.root,map,mode.into(),&saved);
                            }
                        }
                    }
                    for (d, name) in DIFFICULTIES.iter().enumerate() {
                        if m.clicked(&format!("{DIFF}/Difficulties/{name}/Button")) {
                            fe.difficulty = d;
                            next = Some(Screen::Modes);
                        }
                    }
                }
                if back {
                    next = Some(Screen::Maps);
                }
            }
            Screen::Settings => {
                if m_click(fe.settings.as_ref(),&format!("{SETTINGS}/Panel/MainPanel/MainSettings/Jukebox/JukeboxButton")){
                    next=Some(Screen::Menu(MenuPage::Jukebox));
                }
                if let Some(m) = fe.settings.as_mut() {
                    // Sliders: press on one and drag (UnityEngine.UI.Slider, left to right).
                    let (mx, _) = mouse_position();
                    if !is_mouse_button_down(MouseButton::Left) {
                        fe.drag = None;
                    }
                    let before = fe.sliders;
                    for (k, row) in SLIDER_ROWS.iter().enumerate() {
                        let sl = format!("{SETTINGS}/Panel/MainPanel/MainSettings/SFX/{row}/Slider");
                        if m.clicked(&sl) {
                            fe.drag = Some(k);
                        }
                        if fe.drag == Some(k) {
                            if let Some(r) = m.rect(&format!("{sl}/Handle Slide Area")) {
                                fe.sliders[k] = ((mx - r.x) / r.w.max(1.0)).clamp(0.0, 1.0);
                            }
                        }
                        crate::menus::set_slider(m, &sl, fe.sliders[k]);
                        let pct = if k == 3 { 50.0 + fe.sliders[k] * 100.0 } else { fe.sliders[k] * 100.0 };
                        m.set_text(&format!("{SETTINGS}/Panel/MainPanel/MainSettings/SFX/{row}/PercentText"), &format!("{}%", pct.round()));
                    }
                    if fe.sliders[..3] != before[..3] {
                        out = Some(FrontAction::Volume([fe.sliders[0], fe.sliders[1], fe.sliders[2]]));
                    } else if fe.sliders[3] != before[3] {
                        out = Some(FrontAction::CursorScale(0.5 + fe.sliders[3]));
                    }
                }
                if m_click(fe.settings.as_ref(), &format!("{SETTINGS}/Panel/Buttons/Accessibility/Button")) {
                    next = Some(Screen::Advanced);
                }
                if m_click(fe.settings.as_ref(), &format!("{SETTINGS}/Panel/Buttons/HotkeysButton/Button")) {
                    next = Some(Screen::Hotkeys);
                }
                if m_click(fe.settings.as_ref(), &format!("{SETTINGS}/Panel/Buttons/BackupButton/Button")) {
                    self.account_ui.open(&fe.root);
                }
                if m_click(fe.settings.as_ref(), &format!("{SETTINGS}/Panel/Buttons/ProfileButton/Button")) {
                    next = Some(Screen::Menu(MenuPage::Profile));
                }
                if back {
                    next = Some(fe.settings_from);
                }
            }
            Screen::Hotkeys => {
                let mut back = back;
                if let Some(m) = fe.hotkeys.as_mut() {
                    // Waiting for a key: the next one pressed is bound; Escape cancels.
                    if let Some(a) = fe.rebinding.clone() {
                        back = false;
                        if is_key_pressed(KeyCode::Escape) {
                            fe.rebinding = None;
                        } else if let Some(k) = crate::keybinds::any_pressed() {
                            fe.binds.set(&a, k);
                            fe.rebinding = None;
                            out = Some(FrontAction::Hotkeys(fe.binds.clone()));
                        }
                    }
                    use crate::keybinds as kb;
                    let mut rows: Vec<(String, String, &str)> = kb::TOWER_DEFAULTS
                        .iter()
                        .map(|(t, _)| (format!("tower:{t}"), loc.get(*t).cloned().unwrap_or_else(|| crate::spaced(t)), "TopGroup/MonkeyContent"))
                        .collect();
                    for (list, col) in [
                        (&kb::MONKEYS_EXTRA[..], "TopGroup/MonkeyContent"),
                        (&kb::GAMEPLAY[..], "TopGroup/GameplayContent"),
                        (&kb::SANDBOX[..], "TopGroup/SandboxContent"),
                        (&kb::GAMEPLAY_ADVANCED[..], "AdvancedHotkeys/GameplayAdvanced"),
                        (&kb::SANDBOX_ADVANCED[..], "AdvancedHotkeys/SandboxAdvanced"),
                    ] {
                        rows.extend(list.iter().map(|(a, l, _)| (a.to_string(), loc.get(*l).cloned().unwrap_or_else(|| l.to_string()), col)));
                    }
                    for (a, label, col) in rows {
                        let r = format!("{HK_CONTENT}/{col}/{a}");
                        m.set_text(&format!("{r}/Text"), &label);
                        let waiting = fe.rebinding.as_deref() == Some(a.as_str());
                        let key = fe.binds.keys.get(&a).map_or("", |k| kb::key_name(*k));
                        m.set_text(&format!("{r}/Button/Text"), if waiting { "..." } else { key });
                        m.set_active(&format!("{r}/BgSelected"), waiting);
                        if fe.rebinding.is_none() && m.clicked(&format!("{r}/Button")) {
                            fe.rebinding = Some(a.clone());
                        }
                    }
                    // Advanced Hotkeys: the collapsible panel's +/- shows or hides them.
                    let panel = format!("{HK_CONTENT}/CollapsiblePanel/ToggleButton");
                    if m.clicked(&panel) || m.clicked(&format!("{panel}/Hit")) {
                        fe.hk_advanced = !fe.hk_advanced;
                    }
                    m.set_active(&format!("{HK_CONTENT}/AdvancedHotkeys"), fe.hk_advanced);
                    m.set_active(&format!("{panel}/CloseIcon"), fe.hk_advanced);
                    m.set_active(&format!("{panel}/OpenIcon"), !fe.hk_advanced);
                    // Cursor size slider: pixel size, shown relative to the game's Normal cursor.
                    let sl = format!("{HK_CONTENT}/TopGroup/SandboxContent/Cursor/Bg /CursorSlider");
                    if !is_mouse_button_down(MouseButton::Left) {
                        fe.drag = None;
                    }
                    if let Some(t) = drag_slider(m, &format!("{sl}/Slider"), 3, &mut fe.drag) {
                        // Snap to the default near it.
                        let def = (CURSOR_PX.2 - CURSOR_PX.0) / (CURSOR_PX.1 - CURSOR_PX.0);
                        let t = if (t - def).abs() < 0.03 { def } else { t };
                        if t != fe.sliders[3] {
                            fe.sliders[3] = t;
                            out = Some(FrontAction::CursorScale(CURSOR_PX.0 + t * (CURSOR_PX.1 - CURSOR_PX.0)));
                        }
                    }
                    crate::menus::set_slider(m, &format!("{sl}/Slider"), fe.sliders[3]);
                    let px = CURSOR_PX.0 + fe.sliders[3] * (CURSOR_PX.1 - CURSOR_PX.0);
                    m.set_text(&format!("{sl}/PercentText"), &format!("{}%", (px / CURSOR_PX.2 * 100.0).round()));
                    fe.cursor_tick = m.rect(&format!("{sl}/Slider/Handle Slide Area"));
                    if m.clicked(&format!("{HOTKEYS}/RestoreBtn")) {
                        fe.binds = kb::Keybinds::defaults();
                        out = Some(FrontAction::Hotkeys(fe.binds.clone()));
                    }
                    let (_, wheel) = mouse_wheel();
                    if wheel != 0.0 {
                        fe.hk_scroll = (fe.hk_scroll - wheel.signum() * 150.0).clamp(0.0, if fe.hk_advanced { 7000.0 } else { 4200.0 });
                    }
                    if let Some(c) = m.find_mut(HK_CONTENT) {
                        c.rt.pos.y = fe.hk_scroll;
                    }
                }
                if back {
                    next = Some(Screen::Settings);
                }
            }
            Screen::Advanced => {
                if let Some(m) = fe.advanced.as_mut() {
                    let l = |k: &str, or: &str| loc.get(k).cloned().unwrap_or_else(|| or.to_string());
                    let mut changed = false;
                    if !is_mouse_button_down(MouseButton::Left) {
                        fe.drag = None;
                    }
                    let c = ACC_CONTENT;
                    let mut sections_changed=false;
                    for i in 0..ADV_SECTIONS.len(){
                        if m.clicked(&format!("{c}/Section{i}")){
                            fe.adv_expanded[i]=!fe.adv_expanded[i];
                            fe.drag=None;sections_changed=true;
                        }
                    }
                    advanced_sections(m,fe.adv_expanded);
                    // Skip row input on the layout-changing click: its old bounds belong to
                    // the previous arrangement, and must not receive the same click.
                    // The original's options.
                    m.set_text(&format!("{c}/AccessibilityTitle/Title"), &l("Accessibility", "Accessibility"));
                    for (row, title, desc, slot) in [("EffectsScale/FxSlider", "EffectsScale", "EffectsScaleDescription", 0usize),
                        ("Stereo/StereoSlider", "Stereoscopic", "StereoscopicDescription", 1)] {
                        let r = format!("{c}/{row}");
                        m.set_text(&format!("{r}/Title"), &l(title, title));
                        let dpath = if slot == 0 { "EffectsScale/FxDescription/Text" } else { "Stereo/StereoDescription/Text" };
                        m.set_text(&format!("{c}/{dpath}"), &l(desc, ""));
                        let sl = format!("{r}/Slider");
                        if let Some(t) = (!sections_changed).then(||drag_slider(m, &sl, 40 + slot, &mut fe.drag)).flatten() {
                            *(if slot == 0 { &mut fe.adv.effects_scale } else { &mut fe.adv.stereo }) = (t * 20.0).round() / 20.0;
                            changed = true;
                        }
                        let v = if slot == 0 { fe.adv.effects_scale } else { fe.adv.stereo };
                        crate::menus::set_slider(m, &sl, v);
                        m.set_text(&format!("{r}/PercentText"), &format!("{}%", (v * 100.0).round()));
                    }
                    m.set_text(&format!("{c}/RangeCircles/Textures/Title"), &l("AccessibilityRangeCircle", "Tower Range Circle"));
                    for (k, (name, key)) in RANGE_CIRCLES.iter().enumerate() {
                        let b = format!("{c}/RangeCircles/Textures/OptionsGroup/{name}");
                        m.set_text(&format!("{b}/Text"), &l(key, name));
                        m.set_active(&format!("{b}/SelectedIcon"), fe.adv.range_circle as usize == k);
                        if !sections_changed && m.clicked(&b) {
                            fe.adv.range_circle = k as u8;
                            changed = true;
                        }
                    }
                    for (k, (row, group)) in ACC_TOGGLES.iter().enumerate() {
                        let anim = format!("{c}/{row}/{group}/Anim");
                        let (key, or) = [("AmbientMapEffects", "Ambient Map Effects"), ("TowerPlacementSnapping", "Tower Placement Snapping"),
                            ("NumericalAbilityTimers", "Numerical Ability Timers"), ("CashSpentTracker", "Cash Spent Tracker")][k];
                        m.set_text(&format!("{anim}/TxtOn"), &l(key, or));
                        let on = [&mut fe.adv.ambient_fx, &mut fe.adv.tower_snapping, &mut fe.adv.numerical_timers, &mut fe.adv.cash_spent];
                        let v = on.into_iter().nth(k).unwrap();
                        if !sections_changed && m.clicked(&format!("{anim}/HitBox")) {
                            *v = !*v;
                            changed = true;
                        }
                        advanced_toggle(m, &anim, *v);
                    }
                    m.set_text(&format!("{c}/CashSpent/Description/Text"), &l("CashSpentTrackerInfo", "Display overall cash spent in-game"));
                    m.set_text(&format!("{c}/ReleaseMemory/ToggleGroup/Anim/TxtOn"), &l("MemoryOptimization", "Memory Optimization"));
                    m.set_text(&format!("{c}/ReleaseMemory/Description/Text"), &l("MemoryOptimizationInfo", ""));
                    if !sections_changed && m.clicked(&format!("{c}/ReleaseMemory/ToggleGroup/Anim/HitBox")) {
                        fe.adv.memory_opt = !fe.adv.memory_opt;
                        changed = true;
                    }
                    advanced_toggle(m, &format!("{c}/ReleaseMemory/ToggleGroup/Anim"), fe.adv.memory_opt);
                    // Ours.
                    for (i, row) in fe.adv.rows().iter().enumerate() {
                        let r = format!("{c}/Ours{i}");
                        match row {
                            crate::advanced::Row::Toggle(label, on, desc) => {
                                let anim = format!("{r}/ToggleGroup/Anim");
                                m.set_text(&format!("{anim}/TxtOn"), label);
                                m.set_text(&format!("{r}/Description/Text"), desc);
                                let clicked=!sections_changed && m.clicked(&format!("{anim}/HitBox"));
                                if clicked {fe.adv.toggle(i);changed=true;}
                                advanced_toggle(m,&anim,*on ^ clicked);
                            }
                            // Stepped sliders snap to their stops.
                            crate::advanced::Row::Slider(label, t, value, _, desc) => {
                                m.set_text(&format!("{r}/FxSlider/Title"), label);
                                m.set_text(&format!("{r}/FxDescription/Text"), desc);
                                let sl = format!("{r}/FxSlider/Slider");
                                if let Some(nt) = (!sections_changed).then(||drag_slider(m, &sl, 10 + i, &mut fe.drag)).flatten() {
                                    fe.adv.slide(i, nt);
                                    changed = true;
                                }
                                crate::menus::set_slider(m, &sl, *t);
                                m.set_text(&format!("{r}/FxSlider/PercentText"), value);
                            }
                        }
                    }
                    // ScrollRect: the mouse wheel moves the rows (scrollSensitivity 150), with
                    // the scrollbar's handle following.
                    let limit = scroll_limit(m, ACC_VIEW, ACC_CONTENT, s);
                    let (_, wheel) = mouse_wheel();
                    fe.adv_scroll = (fe.adv_scroll - wheel.signum() * 150.0 * (wheel != 0.0) as i32 as f32).clamp(0.0, limit);
                    if let Some(cn) = m.find_mut(ACC_CONTENT) {
                        cn.rt.pos.y = fe.adv_scroll;
                    }
                    let view_h = m.rect(ACC_VIEW).map_or(1.0, |r| r.h / s);
                    let size = (view_h / (view_h + limit).max(1.0)).clamp(0.1, 1.0);
                    let top = 1.0 - if limit > 0.0 { fe.adv_scroll / limit } else { 0.0 } * (1.0 - size);
                    if let Some(h) = m.find_mut(&format!("{ACC}/Panel/Scroll/Scrollbar/Sliding Area/Handle")) {
                        h.rt.amin = vec2(0.0, top - size);
                        h.rt.amax = vec2(1.0, top);
                    }
                    if !sections_changed && m.clicked(&format!("{ACC}/Panel/OkBtn")) {
                        fe.back_requested = true;
                    }
                    if changed {
                        out = Some(FrontAction::Advanced(fe.adv.clone()));
                    }
                }
                if back {
                    next = Some(Screen::Settings);
                }
            }
            Screen::Modes => {
                if let Some(m) = fe.modes.as_mut() {
                    let d = DIFFICULTIES[fe.difficulty];
                    for (k, name) in DIFFICULTIES.iter().enumerate() {
                        m.set_active(&format!("{MODES}/{name}Modes"), k == fe.difficulty);
                    }
                    m.set_text(&format!("{MODES}/MapInfo/DifficultyText"), &d.to_uppercase());
                    let desc = loc.get(&format!("{d} Description")).cloned().unwrap_or_default();
                    m.set_text(&format!("{MODES}/DifficultyInfo/DifficultyDescription"), &desc);
                    m.set_sprite(&format!("{MODES}/MapInfo/DifficultyImage"), Some(["MedalBronze", "MedalSilver", "MedalGold"][fe.difficulty]));
                    for (button, mode) in MODE_BUTTONS[fe.difficulty] {
                        if m.clicked(&format!("{MODES}/{d}Modes/{button}")) {
                            if let Some((_, map)) = fe.list.get(fe.map) {
                                out = Some(FrontAction::Play { map: map.clone(), mode: mode.to_string() });
                            }
                        }
                    }
                }
                if back {
                    next = Some(Screen::Difficulty);
                }
            }
        }
        if back && fe.game_options_root==Some(fe.screen) && options_can_close {
            fe.game_options_root=None;fe.backdrop=None;
            next=None;out=Some(FrontAction::ReturnToPause);
        }
        // Not in BTD6 (DEVIATIONS.md): the store, and buttons that would buy something, open
        // the support popup instead.
        if !support {
            let store = Screen::Menu(MenuPage::Store);
            let mut purchase = fe.screen == store || next == Some(store);
            if fe.screen != Screen::Main {
                purchase |= ["CashInfo/MoreBtn", "Knowledge/MorePointsBtn"].iter().any(|p| m_click(fe.fore.as_ref(), &format!("{FORE}/{p}")));
            }
            if let Screen::Menu(page) = fe.screen {
                let buttons: &[&str] = match page {
                    MenuPage::Powers => &["Canvas/PowersSelectScreen/SafeArea/Selections/LayoutGroup/SideInfo/Info/AddButton",
                        "Canvas/PowersSelectScreen/SafeArea/IapPackButton"],
                    MenuPage::TrophyStore => &["Canvas/TrophyStoreScreen/Container/BottomInfo/AddButton"],
                    _ => &[],
                };
                purchase |= buttons.iter().any(|p| m_click(fe.pages.get(&page).map(|m| &m.node), p));
            }
            if purchase {
                self.open_support();
            }
        }
        if let Some(index) = browse { self.menus.open_upgrades(index); }
        if let Some(fore)=fe.fore.as_mut(){
            for (name,hidden,on) in [("DoubleCashMode",fe.adv.hide_double_cash,fe.adv.double_cash_enabled),
                ("FastTrackMode",fe.adv.hide_fast_track,fe.adv.fast_track_enabled)]{
                let path=format!("{FORE}/ExtraSettings/{name}");
                fore.set_active(&path,!hidden);fore.set_active(&format!("{path}/Tick"),on);
                if !hidden && fore.clicked(&path){
                    match name{"DoubleCashMode"=>fe.adv.double_cash_enabled=!on,_=>fe.adv.fast_track_enabled=!on}
                    out=Some(FrontAction::Advanced(fe.adv.clone()));
                }
            }
            fore.set_active(&format!("{FORE}/ExtraSettings/SocialSeasonPerks"),false);
            if fe.adv.hide_perks{fore.set_active(&format!("{FORE}/PerksButton"),false);}
        }
        if let Screen::Menu(page)=fe.screen{if let Some(page)=fe.pages.get_mut(&page){page.warm(&mut self.sprites);}}
        // Sandbox has no saves, so starting one never asks about the map's saved game.
        if let Some(FrontAction::Play{map,mode})=out.as_ref().filter(|a|!matches!(a,FrontAction::Play{mode,..} if mode.ends_with("Sandbox"))) {
            if let Some((entry,_))=fe.list.iter().find(|(_,key)|key==map){
                if let Some(saved)=account_preview::saved_game(&entry.id){
                    if fe.save_warning.open(&fe.root,map.clone(),mode.clone(),&saved){out=None;}
                }
            }
        }
        let mut ctx = Ctx { sprites: &mut self.sprites, fonts: &self.fonts, scale: s, canvas_h: canvas.y, clip: None };
        let node = match fe.screen {
            Screen::Main => fe.main.as_mut(),
            Screen::Menu(page) => fe.pages.get_mut(&page).map(|p| &mut p.node),
            Screen::Maps => fe.maps.as_mut(),
            Screen::Heroes => fe.heroes_ui.as_mut(),
            Screen::Monkeys => fe.monkeys.as_mut(),
            Screen::Loading | Screen::Upgrades => None,
            Screen::Difficulty => fe.diff.as_mut(),
            Screen::Modes => fe.modes.as_mut(),
            Screen::Settings => fe.settings.as_mut(),
            Screen::Advanced => fe.advanced.as_mut(),
            Screen::Hotkeys => fe.hotkeys.as_mut(),
        };
        if let Some(n) = node {
            account_preview::bind(&fe.root,n);
            home_events::bind_browser(&fe.root,n);
            // CommonPanel flags are exported from the original scene, rather than hiding
            // the same currency / hero / friends controls on every page.
            if let (Some(panel), Some(fore)) = (n.common_panel(), fe.fore.as_mut()) {
                for (path,on) in [("CashInfo",panel.money),("CashInfo/MoreBtn",panel.buy_money),
                    ("Knowledge",panel.knowledge),("Knowledge/MorePointsBtn",panel.buy_knowledge),
                    ("Back",panel.back),("ChangeHeroButton",panel.change_hero),("Friends",panel.friends),
                    ("ExtraSettings",panel.extras),("PerksButton",panel.perks && !fe.adv.hide_perks),("Heading",!panel.heading.is_empty())] {
                    fore.set_active(&format!("{FORE}/{path}"),on);
                }
                fore.set_active(&format!("{FORE}/CashInfo/DEBUGBtn"),false);
                fore.set_text(&format!("{FORE}/Heading/Title"),self.menus.loc.get(&panel.heading).map(String::as_str).unwrap_or(&panel.heading));
            }
            if let Some((_,data,buttons,controller,clip))=fe.screen_motions.iter_mut().find(|(s,..)|*s==fe.screen){
                if fe.motion_pending{button_motion::warm_visible(n,ctx.sprites);}
                buttons.update(n);
                if fe.screen==Screen::Maps && fe.map_group_key!=Some((fe.tier,fe.page)){
                    if fe.map_group_key.is_some(){button_motion::warm_visible(n,ctx.sprites);}
                    fe.map_group_key=Some((fe.tier,fe.page));
                }
                if fe.screen==Screen::Maps{if let Some(start)=fe.map_group_began{
                    if apply_animation(n,data,"MapGroup","Next",Some((get_time()-start) as f32)){fe.map_group_began=None;}
                }}
                if fe.motion_pending || fe.motion_began.is_some(){
                    let t=fe.motion_began.map(|start|(get_time()-start) as f32).unwrap_or(0.0);
                    let mut complete=apply_animation(n,data,controller,clip,Some(t));
                    if fe.screen==Screen::Monkeys{complete &= apply_animation(n,data,"MonkeyGroupAnimation","Open",Some(t));}
                    if fe.screen==Screen::Maps && fe.map_group_began.is_none(){complete &= apply_animation(n,data,"MapGroup","Open",Some(t));}
                    if complete && !fe.motion_pending{fe.motion_began=None;}
                }
            }
            draw_tree(n, full, &mut ctx);
            if fe.motion_pending{fe.motion_pending=false;fe.motion_began=Some(get_time());}
            // RRBTD6_DUMP_RECTS=<depth>: print the laid-out screen rects (layout debugging).
            if let Some(d) = std::env::var("RRBTD6_DUMP_RECTS").ok().and_then(|v| v.parse::<usize>().ok()) {
                fn walk(n: &Node, path: &str, depth: usize, max: usize) {
                    if !n.active || depth > max {
                        return;
                    }
                    let p = format!("{path}/{}", n.name);
                    eprintln!("rect {p} {:.0} {:.0} {:.0}x{:.0}", n.screen.x, n.screen.y, n.screen.w, n.screen.h);
                    for c in &n.children {
                        walk(c, &p, depth + 1, max);
                    }
                }
                walk(n, "", 0, d);
            }
        }
            if let Screen::Menu(page)=fe.screen{if let Some(page)=fe.pages.get_mut(&page){page.presented();}}
        // Hotkeys: a tick under the cursor slider at the game's Normal size.
        if let (Screen::Hotkeys, Some(r)) = (fe.screen, fe.cursor_tick) {
            let def = (CURSOR_PX.2 - CURSOR_PX.0) / (CURSOR_PX.1 - CURSOR_PX.0);
            let x = r.x + r.w * def;
            draw_rectangle(x - 3.0 * s, r.y + r.h * 0.78, 6.0 * s, r.h * 0.3, Color::new(1.0, 1.0, 1.0, 0.9));
        }
        if fe.screen != Screen::Main && fe.screen != Screen::Menu(MenuPage::Television) {
            if let Some(f) = fe.fore.as_mut() {
                account_preview::bind(&fe.root,f);
                draw_tree(f, full, &mut ctx);
            }
        }
        if std::env::var_os("RRBTD6_AUTOTEST").is_some(){
            static PREVIEW:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
            if let Ok(id)=std::env::var("RRBTD6_SAVE_WARNING"){
                if let Some(saved)=account_preview::saved_game(&id){
                    if !PREVIEW.swap(true,std::sync::atomic::Ordering::Relaxed){
                        fe.save_warning.open(&fe.root,fe.list.get(fe.map).map(|(_,key)|key.clone()).unwrap_or_default(),"Medium".into(),&saved);
                    }
                }
            }
        }
        if let Some((map,mode))=fe.save_warning.draw(full,&mut ctx){out=Some(FrontAction::Play{map,mode});}
        // Whether a popup is still open now: one closed this frame (Start New) unblocks.
        crate::uitree::block_input(self.support_open()||fe.save_warning.is_open());
        // Every way into the Legends page (home world, menus) shows the not-supported popup.
        if next == Some(Screen::Menu(MenuPage::Legends)) {
            next = None;
            open_legends = true;
        }
        if open_legends {
            self.open_legends();
        }
        if let Some(choice) = self.draw_support(full, canvas.y, s) {
            // The store itself can't be used, so either way it closes.
            if matches!(choice, crate::support_popup::SupportChoice::Webstore) {
                crate::support_popup::open_webstore();
            }
            if matches!(choice, crate::support_popup::SupportChoice::Home) || fe.screen == Screen::Menu(MenuPage::Store) {
                next = Some(Screen::Main);
            }
        }
        if fe.screen!=Screen::Main && fe.fore.as_ref().is_some_and(|f|f.clicked(&format!("{FORE}/ChangeHeroButton"))) {
            fe.hero_from=fe.screen;next=Some(Screen::Heroes);
        }
        for (path,page) in [("Friends/FriendsButton",MenuPage::Friends),("PerksButton",MenuPage::Perks)] {
            if fe.screen!=Screen::Main && fe.screen!=Screen::Menu(page) && fe.fore.as_ref().is_some_and(|f|f.clicked(&format!("{FORE}/{path}"))) {
                fe.social_from=fe.screen;next=Some(Screen::Menu(page));
            }
        }
        if let Some(n) = next {
            if let Screen::Menu(page)=n {if let Some(page)=fe.pages.get_mut(&page){page.reopen();}}
            if n == Screen::Main && fe.main.is_some() { fe.main_open_animation = Some(get_time()); }
            if n == Screen::Heroes && fe.heroes_ui.is_some() { fe.hero_open_animation = Some(get_time()); }
            fe.drag = None;
            fe.hero_drag = None;
            fe.rebinding = None;
            fe.screen = n;
        }
        if std::env::var_os("RRBTD6_AUTOTEST").is_some() && fe.screen==Screen::Settings && std::env::var_os("RRBTD6_SETTINGS_ACCOUNT").is_some() && self.account_ui.backdrop.is_none(){self.account_ui.open(&fe.root);}
        if self.account_ui.is_open() && self.account_ui.backdrop.is_none(){self.account_ui.backdrop=Some(crate::menus::Backdrop::capture_sharp());}
        out
    }
}

#[cfg(test)]
mod animation_tests {
    use super::{curve_value,hero_collection_total,apply_animation,Node};
    use serde_json::json;

    #[test]
    fn streamed_cubic_switches_at_original_key_time() {
        let keys = json!([[0.0, 2.0, -3.0, 0.0, 1.0], [1.0, 0.0, 0.0, 2.0, 5.0]]);
        assert_eq!(curve_value(&keys, 0.5), Some(0.5));
        assert_eq!(curve_value(&keys, 1.0), Some(5.0));
        assert_eq!(curve_value(&keys, 1.25), Some(5.5));
    }
    #[test]
    fn title_name_obeys_native_tmp_alpha_reveal_in_old_and_new_exports() {
        let mut node=Node::from_json(&json!({"name":"Scene","children":[{"name":"Name",
            "comps":[{"type":"TextMeshProUGUI","m_text":"Synthetic","color":[1.0,1.0,1.0,1.0]}]}]}));
        let mut runtime=json!({"animators":[{"controller":"Title","clips":[{"name":"Open",
            "start":0.0,"stop":4.8,"bindings":[{"path":"Name","type":114,"attribute":4185109675u64,
            "property":null,"curves":[[[-3.4e38,0.0,0.0,0.0,0.0],
                [4.5,0.0,0.0,4.0,0.0],[4.75,0.0,0.0,0.0,1.0]]]}]}]}]});
        for property in [json!(null),json!("m_fontColor.a")] {
            runtime["animators"][0]["clips"][0]["bindings"][0]["property"]=property;
            for (time,alpha) in [(2.0,0.0),(4.5,0.0),(4.625,0.5),(4.8,1.0)] {
                apply_animation(&mut node,&runtime,"Title","Open",Some(time));
                assert_eq!(node.find("Name").unwrap().txt.as_ref().unwrap().color.a,alpha);
            }
        }
    }
    #[test]
    fn hero_counter_counts_base_appearances_and_skins_with_source_eligibility() {
        let data=json!({"Quincy":{"skins":[{"default":true,"collection_counted":true},
            {"default":false,"collection_counted":true}]},
            "Sauda":{"skins":[{"default":true,"collection_counted":true},
            {"id":"RedSauda","default":false,"collection_counted":false}]}});
        assert_eq!(hero_collection_total(&data),3);
    }
    #[test]
    fn stripped_idle_channel_preserves_opacity_but_real_zero_is_applied() {
        let mut node=Node::from_json(&json!({"name":"Scene","children":[{"name":"Background",
            "comps":[{"type":"Image","color":[0.0,0.0,0.0,1.0]}]}]}));
        let mut runtime=json!({"animators":[{"controller":"Loading","clips":[{"name":"Idle",
            "start":0.0,"stop":1.0,"bindings":[{"path":"Background","property":"m_Color.a",
            "start":[],"end":[],"curves":[]}]}]}]});
        apply_animation(&mut node,&runtime,"Loading","Idle",Some(0.5));
        assert_eq!(node.find("Background").unwrap().img.as_ref().unwrap().color.a,1.0);
        runtime["animators"][0]["clips"][0]["bindings"][0]["curves"]=json!([[[0.0,0.0,0.0,0.0,0.0]]]);
        apply_animation(&mut node,&runtime,"Loading","Idle",Some(0.5));
        assert_eq!(node.find("Background").unwrap().img.as_ref().unwrap().color.a,0.0);
    }
}

#[cfg(test)]
mod scroll_tests {
    use super::*;
    fn input(y: f32, pressed: bool, held: bool, released: bool, wheel: f32) -> ScrollInput {
        ScrollInput { pointer: vec2(50.0,y), pressed, held, released, wheel, dt: 1.0/60.0, scale: 1.0 }
    }
    #[test]
    fn drag_scrolls_without_clicking_and_keeps_ownership_outside_viewport() {
        let area = Rect::new(0.0,0.0,100.0,100.0);
        let mut s = ScrollState::default(); let mut drag = None;
        assert!(!s.update(&mut drag, 0, area, 1000.0, input(80.0,true,true,false,0.0)));
        assert!(!s.update(&mut drag, 0, area, 1000.0, input(-20.0,false,true,false,0.0)));
        assert_eq!(s.offset,100.0);
        assert!(!s.update(&mut drag, 0, area, 1000.0, input(-20.0,false,false,true,0.0)));
        assert!(drag.is_none());
    }
    #[test]
    fn scroll_limit_uses_full_content_not_its_clipped_screen_rect() {
        let mut root=Node::from_json(&serde_json::json!({"name":"Root","active":true,"children":[
            {"name":"View","active":true},{"name":"Content","active":true}]}));
        root.find_mut("View").unwrap().screen=Rect::new(0.0,0.0,200.0,100.0);
        let content=root.find_mut("Content").unwrap();
        content.screen=Rect::new(0.0,0.0,200.0,100.0);
        content.unclipped_screen=Rect::new(0.0,0.0,200.0,1000.0);
        assert_eq!(scroll_limit(&root,"View","Content",1.0),900.0);
    }
    #[test]
    fn hidden_parent_cannot_leave_clickable_children() {
        let mut root=Node::from_json(&serde_json::json!({"name":"Root","active":true,"children":[
            {"name":"HiddenPage","active":false,"children":[{"name":"Button","active":true}]}]}));
        root.find_mut("HiddenPage/Button").unwrap().screen=Rect::new(0.0,0.0,200.0,100.0);
        assert!(root.rect("HiddenPage/Button").is_none());
    }
    #[test]
    fn click_releases_without_scrolling_and_wheel_clamps() {
        let area=Rect::new(0.0,0.0,100.0,100.0);
        let mut s=ScrollState::default(); let mut drag=None;
        s.update(&mut drag,0,area,300.0,input(50.0,true,true,false,0.0));
        assert!(s.update(&mut drag,0,area,300.0,input(50.0,false,false,true,0.0)));
        s.update(&mut drag,0,area,300.0,input(50.0,false,false,false,-10.0));
        assert_eq!(s.offset,300.0);
        s.update(&mut drag,0,area,300.0,input(50.0,false,false,false,10.0));
        assert_eq!(s.offset,0.0);
    }
}

pub fn race_entry(root:&Path)->Option<serde_json::Value>{home_events::detail(root,"raceEvent")}

pub fn daily_entry(root:&Path,key:&str)->Option<serde_json::Value>{let mut data=home_events::detail(root,key)?;crate::game::Game::validate_daily_rules(&data).ok()?;data["localDaily"]=serde_json::json!(true);Some(data)}
