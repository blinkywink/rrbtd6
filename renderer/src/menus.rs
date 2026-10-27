//! Full-screen menus opened from the in-game HUD, built from the game's UI scenes
//! (tools/assets/dump_ui_prefab.py): PauseScreenUI (gear button) and TowerUpgradesUI's
//! UpgradeScreen (shop "Upgrades" button) with CommonForegroundUI's back button. Both sit on
//! CommonBackgroundUI's backdrop: a blurred, tinted snapshot of the game (BlurRawImg).

use crate::game::Game;
use crate::hud::{money, Hud};
use crate::ui::{find_icon, Sprites};
use crate::uitree::{draw_tree, Ctx, Node, Rt, R};
use macroquad::prelude::*;
use std::collections::HashMap;
use std::path::Path;

/// CommonBackgroundScreen BlurRawImg colour.
const BLUR_TINT: Color = Color::new(0.4039, 0.4706, 0.4941, 1.0);
const VIGNETTE_MUL: f32 = 0.7;
const VIGNETTE_POW: f32 = 1.0;

thread_local! {
    /// The Upgrades screen's tiled background, created on first draw (GPU objects stay on
    /// the render thread; `Menus` itself is built on a loader thread).
    static UPGRADE_PATTERN: std::cell::RefCell<Option<Option<crate::frontend::HeroPattern>>> = const { std::cell::RefCell::new(None) };
}

const PAUSE: &str = "Canvas/PauseMenuPanel";
const VICTORY: &str = "Canvas/VictoryPanel";
const DEFEAT: &str = "Canvas/DefeatPanel";
const UPG: &str = "MainMenuCanvas/UpgradeScreen";
const FORE: &str = "Canvas/ForegroundScreen";
const PORTRAIT: &str = "MainMenuCanvas/UpgradeScreen/UpgradeInfo/UpgradeDetails/UpgradePortrait";
const PARAGON: &str = "MainMenuCanvas/UpgradeScreen/UpgradeInfo/ParagonContainer/ParagonPanel/Upgrade";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Pause,
    Upgrades,
    /// VictoryUI: the mode's last round beaten.
    Victory,
    /// DefeatUI: out of lives.
    Defeat,
    /// ParagonConfirmationPopup: buy the selected tower's Paragon, investing extra cash.
    Paragon,
}

pub enum MenuAction {
    Close,
    Hotkeys,
    Advanced,
    Restart,
    /// Victory screen: keep playing (freeplay).
    Freeplay,
    /// Back to the main menu.
    Home,
    /// Buy the Paragon of tower `menus.paragon_tower` with this much extra cash.
    BuyParagon(f64),
    /// Victory / defeat: hide the screen to look at the map.
    ReviewMap,
    ToggleAuto,
    /// Pause-screen sliders changed: (music, sound effects, hero voice), each 0..=1.
    Volume([f32; 3]),
    /// Jukebox: previous (-1) / next (+1) track.
    Track(i32),
    ToggleLoop,
    ToggleShuffle,
}

/// Pause-screen slider objects, in `MenuAction::Volume` order.
const SLIDERS: [&str; 3] = ["MusicBG", "FXBG", "HeroVOBG"];

/// The game frame behind a menu, blurred and tinted (or as it was, for the end screens,
/// whose own Lightbox dims it).
#[derive(Clone)]
pub struct Backdrop {
    tex: Texture2D,
    sharp: bool,
}

impl Backdrop {
    /// Snapshot of what has been drawn so far this frame.
    pub fn capture() -> Backdrop {
        let img = get_screen_data();
        let (w, h) = (img.width as usize, img.height as usize);
        // Downsample 8x, then two separable box-blur passes.
        const K: usize = 8;
        let (sw, sh) = ((w / K).max(1), (h / K).max(1));
        let mut px = vec![[0f32; 3]; sw * sh];
        for y in 0..sh {
            for x in 0..sw {
                let mut acc = [0f32; 3];
                for dy in 0..K {
                    for dx in 0..K {
                        let i = ((y * K + dy).min(h - 1) * w + (x * K + dx).min(w - 1)) * 4;
                        for c in 0..3 {
                            acc[c] += img.bytes[i + c] as f32;
                        }
                    }
                }
                px[y * sw + x] = acc.map(|v| v / (K * K) as f32);
            }
        }
        for _ in 0..2 {
            px = box_blur(&px, sw, sh, 3, true);
            px = box_blur(&px, sw, sh, 3, false);
        }
        // NinjaKiwi/BTD6/VignetteUI (CommonBackground's vignetteMat, _Pow 1, _mul 0.7):
        // colour *= saturate(1 - _mul * pow(|uv * 2 - 1|, _Pow)).
        for y in 0..sh {
            for x in 0..sw {
                let uv = vec2((x as f32 + 0.5) / sw as f32, (y as f32 + 0.5) / sh as f32);
                let d = (uv * 2.0 - Vec2::ONE).length();
                let k = (1.0 - VIGNETTE_MUL * d.powf(VIGNETTE_POW)).clamp(0.0, 1.0);
                px[y * sw + x] = px[y * sw + x].map(|c| c * k);
            }
        }
        let bytes: Vec<u8> = px.iter().flat_map(|p| [p[0] as u8, p[1] as u8, p[2] as u8, 255]).collect();
        let tex = Texture2D::from_rgba8(sw as u16, sh as u16, &bytes);
        tex.set_filter(FilterMode::Linear);
        Backdrop { tex, sharp: false }
    }

    /// The frame unchanged.
    pub fn capture_sharp() -> Backdrop {
        let img = get_screen_data();
        let tex = Texture2D::from_image(&img);
        tex.set_filter(FilterMode::Linear);
        Backdrop { tex, sharp: true }
    }

    pub fn draw(&self) {
        self.draw_tinted(if self.sharp { WHITE } else { BLUR_TINT });
    }

    /// CommonBackgroundScreen's per-screen colour (e.g. hero SkinData's tint override).
    pub(crate) fn draw_tinted(&self, tint: Color) {
        // Screen data rows run bottom-up.
        draw_texture_ex(&self.tex, 0.0, 0.0, tint, DrawTextureParams {
            dest_size: Some(vec2(screen_width(), screen_height())),
            flip_y: true,
            ..Default::default()
        });
    }
}

fn box_blur(src: &[[f32; 3]], w: usize, h: usize, r: usize, horizontal: bool) -> Vec<[f32; 3]> {
    let mut out = vec![[0f32; 3]; src.len()];
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0f32; 3];
            let mut n = 0.0;
            for k in 0..=2 * r {
                let o = k as isize - r as isize;
                let (sx, sy) = if horizontal { (x as isize + o, y as isize) } else { (x as isize, y as isize + o) };
                if sx < 0 || sy < 0 || sx >= w as isize || sy >= h as isize {
                    continue;
                }
                let p = src[sy as usize * w + sx as usize];
                for c in 0..3 {
                    acc[c] += p[c];
                }
                n += 1.0;
            }
            out[y * w + x] = acc.map(|v| v / n);
        }
    }
    out
}

/// English.xml `<T id="..">text</T>` entries.
pub(crate) fn load_loc(path: &Path) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Ok(s) = std::fs::read_to_string(path) else { return out };
    for part in s.split("<T id=\"").skip(1) {
        let Some((id, rest)) = part.split_once("\">") else { continue };
        let Some((text, _)) = rest.split_once("</T>") else { continue };
        let unescape = |t: &str| t.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&");
        out.entry(unescape(id)).or_insert_with(|| unescape(text));
    }
    out
}

/// Scene dumps wrap the Canvas, whose RectTransform the Canvas component sizes to the screen.
fn scene(path: &Path) -> Option<Node> {
    let mut n = Node::load(path)?;
    for c in &mut n.children {
        c.rt = Rt::default();
    }
    // Gamepad button glyphs, assigned only when a controller is in use.
    n.hide_named(&["Jewel", "JewelIcon", "JewelBar"]);
    Some(n)
}

#[derive(Clone, Copy, PartialEq)]
enum Pick {
    /// (path, tier index 0-4).
    Upgrade(usize, usize),
    Paragon,
}

pub struct Menus {
    pause: Option<Node>,
    paragon: Option<Node>,
    /// The tower the Paragon popup is for, and its investment slider (0..=1).
    pub paragon_tower: usize,
    paragon_slide: f32,
    victory: Option<Node>,
    defeat: Option<Node>,
    upgrades: Option<Node>,
    fore: Option<Node>,
    /// UpgradePopup (description bubble) and its first-column variant.
    popup: Option<Node>,
    popup_col1: Option<Node>,
    pub loc: HashMap<String, String>,
    /// MenuBackgrounds.TowerUpgradesUI (image, colour, tiles, vignette) and the asset root.
    upgrade_bg: serde_json::Value,
    root: std::path::PathBuf,
    /// Upgrades screen: shop index of the shown tower and the picked upgrade.
    tower: usize,
    pick: Option<Pick>,
    /// UpgradeModel.xpCost by name (ui/runtime/upgrade_xp.json), for the default selection.
    upgrade_xp: HashMap<String, f64>,
    /// UpgradeDetails beginShowPopup / popupTimer: the card under the pointer, since when, and
    /// whether its description bubble is showing.
    hover: Option<(Pick, f64, bool)>,
    /// The current pick was chosen before the account had loaded; redo it once it arrives.
    pick_without_account: bool,
    /// The Paragon TowerModel's portrait sprite per base tower (ui/runtime/paragon_portraits.json).
    paragon_portraits: HashMap<String, String>,
    game_hints: bool,
    /// Slider values (music, effects, hero voice) and the slider being dragged.
    pub volumes: [f32; 3],
    drag: Option<usize>,
    /// Jukebox state shown on the pause screen (set by the caller each frame).
    pub track_name: String,
    pub loop_on: bool,
    pub shuffle_on: bool,
}

/// Unity `Slider` (left-to-right, whole-number off): the fill spans anchors x 0..v, the
/// handle sits at anchor x = v inside its slide area.
pub(crate) fn set_slider(p: &mut Node, sl: &str, v: f32) {
    if let Some(f) = p.find_mut(&format!("{sl}/Fill Area/Fill")) {
        f.rt.amin = Vec2::ZERO;
        f.rt.amax = vec2(v, 1.0);
        f.rt.size = vec2(0.0, f.rt.size.y.min(0.0).max(0.0));
    }
    if let Some(hd) = p.find_mut(&format!("{sl}/Handle Slide Area/Handle")) {
        hd.rt.amin = vec2(v, 0.0);
        hd.rt.amax = vec2(v, 1.0);
    }
}

impl Menus {
    pub fn private_coop(&mut self,enabled:bool){
        if let Some(p)=&mut self.pause{p.set_active("Canvas/PauseMenuPanel/Bg/Buttons/Restart",!enabled);}
        if let Some(p)=&mut self.defeat{p.set_active("Canvas/DefeatPanel/Container/Buttons/RetryButton",!enabled);}
    }
    pub fn load(root: &Path) -> Menus {
        let dir = root.join("ui").join("prefabs");
        let mut pause = scene(&dir.join("PauseScreen.json"));
        if let Some(p) = pause.as_mut() {
            for n in [
                "BossRushTickets",
                "RogueStagePause",
                "ReportButton",
                "NkVerified",
                "ReportConfirmPopup",
                "Bg/ChallengeRules",
                "Bg/CTTileID",
                "Bg/Goals",
                "Bg/DragAndDrop/LockAndDrop",
                "Bg/HorzLayout/PetSoundsObject",
                "Bg/Buttons/Store",
                "Bg/Buttons/ConsoleControls",
                "Bg/Buttons/RetryFromCheckpointBtn",
                "Bg/Buttons/PurchaseRaceTickets",
                // Perks are an account feature; the side tab holds nothing else.
                "Bg/SidePanelR",
            ] {
                p.set_active(&format!("{PAUSE}/{n}"), false);
            }
            p.set_text(&format!("{PAUSE}/Bg/DifficultyText"), "Monkey Meadow - Medium");
            // Jukebox toggles start off (the On variants are swapped in by the script).
            p.hide_named(&["LoopTrackOn", "ShuffleOn"]);
            // Volume sliders: values are applied each frame in `draw_pause`.
            for s in SLIDERS {
                set_slider(p, &format!("{PAUSE}/Bg/{s}/Slider"), 1.0);
            }
        }
        let mut upgrades = scene(&dir.join("UpgradeScreen.json"));
        let card = Node::load(&dir.join("UpgradeDetails.json"));
        if let Some(u) = upgrades.as_mut() {
            // Store and tower gift parts (the tower XP panel stays: UpdateUi shows it).
            for n in [
                "PurchaseButtons",
                "TowerGiftUnlock",
                "GamepadInfoButton",
                "UpgradeInfo/UpgradeDetails/UpgradePortrait/Portrait/Tier5PortraitUnlockEffect",
            ] {
                u.set_active(&format!("{UPG}/{n}"), false);
            }
            if let Some(card) = card {
                for path in 1..=3 {
                    if let Some(row) = u.find_mut(&format!("{UPG}/UpgradeInfo/UpgradePaths/Path {path} Upgrades")) {
                        row.children = (0..5)
                            .map(|t| {
                                let mut c = card.clone();
                                c.name = format!("Upgrade{t}");
                                c
                            })
                            .collect();
                    }
                }
            }
            u.hide_named(&["NewUnlock", "RestrictedIcon", "AbilityImage"]);
            for p in [format!("{PORTRAIT}/Portrait/PortraitImage")] {
                if let Some(i) = u.find_mut(&p).and_then(|n| n.img.as_mut()) {
                    i.preserve = true;
                }
            }
        }
        let mut victory = scene(&dir.join("VictoryUI.json"));
        if let Some(v) = victory.as_mut() {
            // Account / event parts: rewards, accolades, golden bloon, Monkey Teams.
            for n in ["Container/Background/Rewards", "Container/Background/Completed", "Container/Background/UserChalObject",
                "Container/Background/AccoladeFloats", "Container/GoldenBloonContainer", "Container/MonkeyTeamsContainer", "CalloutData"] {
                v.set_active(&format!("{VICTORY}/{n}"), false);
            }
        }
        let mut defeat = scene(&dir.join("DefeatUI.json"));
        if let Some(d) = defeat.as_mut() {
            for n in ["Container/UserChalObject", "Container/AccoladeFloats", "Container/AlternateGameOverMessage",
                "Container/LivesLostBloons", "Container/Buttons/ContinueButton", "StatsContainer"] {
                d.set_active(&format!("{DEFEAT}/{n}"), false);
            }
            for n in ["Container/RegularObject", "Container/Body"] {
                d.set_active(&format!("{DEFEAT}/{n}"), true);
            }
        }
        let mut fore = scene(&dir.join("CommonForeground.json"));
        if let Some(f) = fore.as_mut() {
            for n in ["Heading", "CashInfo", "Knowledge", "ChangeHeroButton", "PerksButton", "ExtraSettings"] {
                f.set_active(&format!("{FORE}/{n}"), false);
            }
        }
        Menus {
            paragon: Node::load(&dir.join("ParagonConfirmationPopup.json")).map(|mut n| {
                n.hide_named(&["Jewel"]);
                n
            }),
            paragon_tower: 0,
            paragon_slide: 0.0,
            pause,
            victory,
            defeat,
            upgrades,
            fore,
            popup: Node::load(&dir.join("UpgradePopup.json")),
            popup_col1: Node::load(&dir.join("UpgradePopupLeft.json")),
            loc: load_loc(&root.join("text").join("English.xml")),
            upgrade_bg: std::fs::read_to_string(root.join("ui/runtime/menu_backgrounds.json")).ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .map(|v| v["TowerUpgradesUI"].clone()).unwrap_or_default(),
            root: root.to_owned(),
            tower: 0,
            pick: None,
            upgrade_xp: std::fs::read_to_string(root.join("ui/runtime/upgrade_xp.json")).ok()
                .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            hover: None,
            pick_without_account: false,
            paragon_portraits: std::fs::read_to_string(root.join("ui/runtime/paragon_portraits.json")).ok()
                .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            game_hints: true,
            volumes: [1.0; 3],
            drag: None,
            track_name: String::new(),
            loop_on: false,
            shuffle_on: false,
        }
    }

    /// Display name of a music clip: the game's `GameUIMusicTrack<clip>` text if it has one,
    /// else the clip name without its "Music" prefix and variant letter, spaced ("Upbeat 1").
    pub fn track_display(&self, clip: Option<&str>) -> String {
        let Some(c) = clip else { return String::new() };
        if let Some(t) = self.loc.get(&format!("GameUIMusicTrack{c}")) {
            return t.clone();
        }
        let core = c.trim_start_matches("Music");
        let core = core.strip_suffix('A').filter(|x| x.ends_with(|ch: char| ch.is_ascii_alphanumeric())).unwrap_or(core);
        let mut out = String::new();
        let mut prev: Option<char> = None;
        for ch in core.chars() {
            if let Some(p) = prev {
                let boundary = (ch.is_ascii_uppercase() && (p.is_ascii_lowercase() || p.is_ascii_digit())) || (ch.is_ascii_digit() && !p.is_ascii_digit());
                if boundary {
                    out.push(' ');
                }
            }
            out.push(ch);
            prev = Some(ch);
        }
        out
    }

    pub fn available(&self, m: Menu) -> bool {
        match m {
            Menu::Pause => self.pause.is_some(),
            Menu::Upgrades => self.upgrades.is_some() && self.fore.is_some(),
            Menu::Victory => self.victory.is_some(),
            Menu::Paragon => self.paragon.is_some(),
            Menu::Defeat => self.defeat.is_some(),
        }
    }

    pub fn open_upgrades(&mut self, shop_index: usize) {
        self.tower = shop_index;
        self.pick = None;
    }

    /// Picks the path 1 tier 3 upgrade (autotest screenshots of the description bubble).
    pub fn pick_first(&mut self) {
        self.pick = Some(Pick::Upgrade(0, 2));
    }

    fn text(&self, key: &str) -> String {
        self.loc.get(key).cloned().unwrap_or_else(|| crate::spaced(key))
    }

    /// English.xml "<name> Description" (upgrade and tower descriptions).
    pub fn description(&self, name: &str) -> Option<&str> {
        self.loc.get(&format!("{name} Description")).map(String::as_str)
    }
}

/// On/off toggle look (the toggles' animators slide the knob and tint it).
fn set_toggle(n: &mut Node, knobs: &[&str], on: bool) {
    for k in knobs {
        if let Some(base)=k.strip_suffix("/BtnOn"){
            if let Some(bg)=n.find_mut(&format!("{base}/Bg")).and_then(|n|n.img.as_mut()){
                bg.color=if on{Color::from_rgba(129,255,0,255)}else{Color::from_rgba(56,32,15,255)};
            }
            if let Some(text)=n.find_mut(&format!("{base}/TxtOn")).and_then(|n|n.txt.as_mut()){
                text.color=if on{WHITE}else{Color::from_rgba(205,152,97,255)};
            }
        }
        if let Some(knob) = n.find_mut(k) {
            knob.rt.pos.x = if on { 80.0 } else { -80.0 };
            if let Some(i) = knob.img.as_mut() {i.color=WHITE;}
        }
    }
}

impl Hud {
    /// Draws an open menu over its backdrop; returns what was clicked.
    pub fn draw_menu(&mut self, menu: Menu, game: &Game, auto_start: bool) -> Vec<MenuAction> {
        let s = Hud::canvas_scale();
        let canvas = vec2(screen_width() / s, screen_height() / s);
        let full = R { min: Vec2::ZERO, size: canvas };
        match menu {
            Menu::Pause => self.draw_pause(full, canvas.y, s, auto_start),
            Menu::Upgrades => self.draw_upgrades(game, full, canvas.y, s),
            Menu::Victory | Menu::Defeat => self.draw_end(menu, game, full, canvas.y, s),
            Menu::Paragon => self.draw_paragon(game, full, canvas.y, s),
        }
    }

    /// ParagonConfirmationPopup: "Upgrade to Paragon", what gets absorbed, the "invest more
    /// cash" slider (0 .. the most that still adds power, capped by cash), the total, OK / Cancel.
    fn draw_paragon(&mut self, game: &Game, full: R, canvas_h: f32, s: f32) -> Vec<MenuAction> {
        let mut out = Vec::new();
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.5));
        let t = self.menus.paragon_tower;
        let Some((cost, max, _)) = (t < game.towers.len()).then(|| game.paragon_quote(t)).flatten() else {
            out.push(MenuAction::Close);
            return out;
        };
        let max = max.min((game.cash - cost).max(0.0));
        let id = game.tower_defs[game.towers[t].def].0.clone();
        let name = self.menus.loc.get(&id).cloned().unwrap_or_else(|| crate::spaced(&id));
        let title = self.menus.loc.get("Paragon Title").cloned().unwrap_or_else(|| "Upgrade to Paragon".into());
        let body = self.menus.loc.get("Paragon Body").cloned().unwrap_or_default().replace("{0}", &name).replace("{1}", &format!("{name}s"));
        let mut slide = self.menus.paragon_slide;
        let mut drag = self.menus.drag;
        let Some(p) = self.menus.paragon.as_mut() else { return out };
        const A: &str = "AnimatedObject";
        p.set_text(&format!("{A}/Banner/Title"), &title);
        p.set_text(&format!("{A}/Description"), &body);
        let sl = format!("{A}/Panel/InvestmentSlider/SliderCurrent");
        if !is_mouse_button_down(MouseButton::Left) {
            drag = None;
        }
        if p.clicked(&sl) {
            drag = Some(9);
        }
        if drag == Some(9) {
            if let Some(r) = p.rect(&format!("{sl}/Handle Slide Area")) {
                slide = ((mouse_position().0 - r.x) / r.w.max(1.0)).clamp(0.0, 1.0);
            }
        }
        if p.clicked(&format!("{A}/Panel/InvestmentSlider/LeftBtn")) {
            slide = (slide - 0.05).max(0.0);
        }
        if p.clicked(&format!("{A}/Panel/InvestmentSlider/RightBtn")) {
            slide = (slide + 0.05).min(1.0);
        }
        let extra = (max * slide as f64).round();
        set_slider(p, &sl, if max > 0.0 { slide } else { 0.0 });
        p.set_text(&format!("{sl}/Handle Slide Area/Handle/ValueCurrent"), &crate::hud::money(extra));
        p.set_text(&format!("{A}/Total/ValueTotal"), &crate::hud::money(cost + extra));
        if p.clicked(&format!("{A}/Confirm")) {
            out.push(MenuAction::BuyParagon(extra));
        }
        if p.clicked(&format!("{A}/Cancel")) {
            out.push(MenuAction::Close);
        }
        let mut ctx = Ctx { sprites: &mut self.sprites, fonts: &self.fonts, scale: s, canvas_h, clip: None };
        draw_tree(p, full, &mut ctx);
        self.menus.paragon_slide = slide;
        self.menus.drag = drag;
        out
    }

    /// VictoryScreen / DefeatScreen: the mode (victory) or the round lost on (defeat), and the
    /// Home / Restart / Review Map / Freeplay buttons.
    fn draw_end(&mut self, menu: Menu, game: &Game, full: R, canvas_h: f32, s: f32) -> Vec<MenuAction> {
        let mut out = Vec::new();
        let mode = game.mode_name.clone();
        let won = menu == Menu::Victory;
        let Some(n) = (if won { self.menus.victory.as_mut() } else { self.menus.defeat.as_mut() }) else { return out };
        if won {
            n.set_active(&format!("{VICTORY}/Container/Buttons/FreePlayBtn"),game.race.is_none());
            n.set_text(&format!("{VICTORY}/Container/Background/Body"), &format!("Congratulations! you completed this track on: {mode}"));
            if let Some(race)=&game.race{n.set_text(&format!("{VICTORY}/Container/Background/Body"),&format!("Race practice completed in {}",race.time_text()));}
            if n.clicked(&format!("{VICTORY}/Container/Buttons/HomeBtn")) {
                out.push(MenuAction::Home);
            }
            if n.clicked(&format!("{VICTORY}/Container/Buttons/ReviewMapBtn")) {
                out.push(MenuAction::ReviewMap);
            }
            if n.clicked(&format!("{VICTORY}/Container/Buttons/FreePlayBtn")) {
                out.push(MenuAction::Freeplay);
            }
        } else {
            n.set_text(&format!("{DEFEAT}/Container/RegularObject/DiedRoundTxt"), &format!("Round {}", game.round.max(1)));
            if n.clicked(&format!("{DEFEAT}/Container/Buttons/HomeButton")) {
                out.push(MenuAction::Home);
            }
            if n.clicked(&format!("{DEFEAT}/Container/Buttons/RetryButton")) {
                out.push(MenuAction::Restart);
            }
            if n.clicked(&format!("{DEFEAT}/Container/Buttons/ReviewMapBtn")) {
                out.push(MenuAction::ReviewMap);
            }
        }
        let mut ctx = Ctx { sprites: &mut self.sprites, fonts: &self.fonts, scale: s, canvas_h, clip: None };
        draw_tree(n, full, &mut ctx);
        out
    }

    fn draw_pause(&mut self, full: R, canvas_h: f32, s: f32, auto_start: bool) -> Vec<MenuAction> {
        let mut out = Vec::new();
        let hints = self.menus.game_hints;
        let (mut vols, mut drag) = (self.menus.volumes, self.menus.drag);
        let (track_name, loop_on, shuffle_on) = (self.menus.track_name.clone(), self.menus.loop_on, self.menus.shuffle_on);
        let Some(p) = self.menus.pause.as_mut() else { return out };
        // Volume sliders: press on a slider (or its handle) and drag; value follows the cursor
        // across the handle slide area, like UnityEngine.UI.Slider.
        let (mx, my) = mouse_position();
        if !is_mouse_button_down(MouseButton::Left) {
            drag = None;
        }
        let before = vols;
        for (k, sname) in SLIDERS.iter().enumerate() {
            let sl = format!("{PAUSE}/Bg/{sname}/Slider");
            let area = p.rect(&format!("{sl}/Handle Slide Area")).or_else(|| p.rect(&sl));
            let hit = p.rect(&sl).is_some_and(|r| r.contains(vec2(mx, my)));
            if is_mouse_button_pressed(MouseButton::Left) && hit {
                drag = Some(k);
            }
            if drag == Some(k) {
                if let Some(r) = area {
                    vols[k] = ((mx - r.x) / r.w.max(1.0)).clamp(0.0, 1.0);
                }
            }
            set_slider(p, &sl, vols[k]);
        }
        if vols != before {
            out.push(MenuAction::Volume(vols));
        }
        for (k, sname) in SLIDERS.iter().enumerate() {
            p.set_text(&format!("{PAUSE}/Bg/{sname}/PercentText"), &format!("{}%", (vols[k] * 100.0).round()));
        }
        // Jukebox (MiniJukeBoxPlayer): track name, previous / next, loop and shuffle toggles.
        let jb = format!("{PAUSE}/Bg/Jukebox");
        p.set_text(&format!("{jb}/TrackName"), &track_name);
        p.set_active(&format!("{jb}/Controls/LoopTrackButton/LoopTrackOn"), loop_on);
        p.set_active(&format!("{jb}/Controls/LoopTrackButton/LoopTrackOff"), !loop_on);
        p.set_active(&format!("{jb}/Controls/ShufflePlaylistBtn/ShuffleOn"), shuffle_on);
        p.set_active(&format!("{jb}/Controls/ShufflePlaylistBtn/ShuffleOff"), !shuffle_on);
        if p.clicked(&format!("{jb}/Controls/PreviousTrack")) {
            out.push(MenuAction::Track(-1));
        }
        if p.clicked(&format!("{jb}/Controls/NextTrack")) {
            out.push(MenuAction::Track(1));
        }
        if p.clicked(&format!("{jb}/Controls/LoopTrackButton")) {
            out.push(MenuAction::ToggleLoop);
        }
        if p.clicked(&format!("{jb}/Controls/ShufflePlaylistBtn")) {
            out.push(MenuAction::ToggleShuffle);
        }
        let cp = format!("{PAUSE}/Bg/ContinuousPlay");
        set_toggle(p, &[&format!("{cp}/BtnOn"), &format!("{cp}/ContinuousPlayHitBox/Position/BtnOnWithTint")], auto_start);
        set_toggle(p, &[&format!("{PAUSE}/Bg/GameHints/BtnOn")], hints);
        if p.clicked(&format!("{cp}/ContinuousPlayHitBox")) {
            out.push(MenuAction::ToggleAuto);
        }
        if p.clicked(&format!("{PAUSE}/Bg/SidePanel/ButtonGroup/Hotkeys")) {out.push(MenuAction::Hotkeys);}
        if p.clicked(&format!("{PAUSE}/Bg/SidePanel/ButtonGroup/Accessibility")) {out.push(MenuAction::Advanced);}
        let toggle_hints = p.clicked(&format!("{PAUSE}/Bg/GameHints/GameHintsHitBox"));
        if p.clicked(&format!("{PAUSE}/Bg/Buttons/ContinueBtn")) {
            out.push(MenuAction::Close);
        }
        if p.clicked(&format!("{PAUSE}/Bg/Buttons/Home")) {
            out.push(MenuAction::Home);
        }
        if p.clicked(&format!("{PAUSE}/Bg/Buttons/Restart")) {
            out.push(MenuAction::Restart);
        }
        // Clicking outside the panel resumes (PauseScreen.backgroundBtn).
        let on_panel = p.rect(&format!("{PAUSE}/Bg")).is_some_and(|r| r.contains(vec2(mx, my)));
        if !on_panel && drag.is_none() && p.clicked(&format!("{PAUSE}/ClickBG")) {
            out.push(MenuAction::Close);
        }
        let mut ctx = Ctx { sprites: &mut self.sprites, fonts: &self.fonts, scale: s, canvas_h, clip: None };
        draw_tree(p, full, &mut ctx);
        self.menus.volumes = vols;
        self.menus.drag = drag;
        if toggle_hints {
            self.menus.game_hints = !hints;
        }
        out
    }

    fn draw_upgrades(&mut self, game: &Game, full: R, canvas_h: f32, s: f32) -> Vec<MenuAction> {
        let mut out = Vec::new();
        // CommonBackgroundScreen draws MenuBackgrounds.TowerUpgradesUI (PowersPatternGreyBg,
        // 15 tiles per height, vignette .7) over the snapshot, in game and from the main menu.
        // UpgradeScreen.UpdateUi tints it with backgroundColour, or backgroundParagonColour
        // (0.52, 0, 1) when the shown tower's Paragon is owned (Btd6Player.HasUpgrade); the
        // clone has no owned upgrades yet, so it is always backgroundColour (= the table colour).
        let bg = &self.menus.upgrade_bg;
        if let Some(name) = bg["image"].as_str() {
            UPGRADE_PATTERN.with(|p| {
                let mut p = p.borrow_mut();
                let pattern = p.get_or_insert_with(|| crate::frontend::HeroPattern::from_file(&self.menus.root, name));
                if let Some(pattern) = pattern {
                    pattern.draw_config(crate::ui::color(&bg["color"]).unwrap_or(WHITE),
                        bg["tiles"].as_f64().unwrap_or(-1.0) as f32, bg["vignette"].as_f64().unwrap_or(-1.0) as f32);
                }
            });
        }
        let towers: Vec<usize> = game.shop.iter().enumerate().filter(|(_, it)| !it.hero).map(|(i, _)| i).collect();
        let Some(item) = game.shop.get(self.menus.tower) else { return out };
        let id = item.id.clone();
        let def = game.tower_defs.iter().position(|(n, _)| *n == id);
        let tt = def.and_then(|d| game.tower_tiers[d].as_ref());
        let name_of = |path: usize, tier: usize| tt.and_then(|t| t.upgrades.iter().find(|u| u.path == path && u.tier == tier));
        let paragon = self.menus.loc.get(&format!("{id} Paragon")).cloned();
        let tower_name = self.menus.text(&id);
        let description = self.menus.text(&format!("{id} Description"));
        let icon_of = |icons: &HashMap<String, String>, name: &str, path: usize, tier: usize| find_icon(icons, name, &id, path, tier);
        let set = item.set;
        let tower_cost = item.cost;
        let menus = &mut self.menus;
        let Some(u) = menus.upgrades.as_mut() else { return out };

        u.set_text(&format!("{UPG}/TowerInfo/TowerTitle"), &tower_name);
        // UpdateUi: xpToSpend = "XP: " + the tower's XP (Btd6Player.GetTowerXP, N0).
        let xp = crate::frontend::account_preview::monkey_account().and_then(|a| a.xp.get(&id).copied()).unwrap_or(0.0).trunc();
        u.set_text(&format!("{UPG}/TowerInfo/XPToSpend"), &format!("{}: {}", menus.loc.get("XP").map(String::as_str).unwrap_or("XP"),
            crate::frontend::account_preview::number(xp)));
        u.set_text(&format!("{UPG}/TowerInfo/TowerDescription"), &description);

        // UpgradeScreen.PopulatePaths: with no upgrade chosen, select the ResetUpgradeUnlocks
        // result (start at path 1's first card; CompareDetails against each unowned upgrade).
        crate::frontend::account_preview::ensure(&menus.root);
        let account = crate::frontend::account_preview::monkey_account();
        if menus.pick_without_account && account.is_some() {
            menus.pick = None;
            menus.pick_without_account = false;
        }
        if menus.pick.is_none() && account.is_none() {
            menus.pick_without_account = true;
        }
        let has = |name: &str| account.as_ref().is_some_and(|a| a.acquired.contains(name));
        // ShowParagonPanel: the tower has a Paragon and all three tier 5 upgrades are owned.
        let paragon_panel = paragon.is_some() && (0..3).all(|path| name_of(path, 4).is_some_and(|u| has(&u.name)));
        if menus.pick.is_none() && paragon_panel {
            // UpdateUi selects the Paragon after PopulatePaths when its panel shows.
            menus.pick = Some(Pick::Paragon);
        }
        if menus.pick.is_none() {
            let owned = |name: &str| account.as_ref().is_some_and(|a| a.acquired.contains(name));
            let xp = account.as_ref().and_then(|a| a.xp.get(&id).copied()).unwrap_or(0.0).trunc();
            let affordable = |name: &str| menus.upgrade_xp.get(name).is_some_and(|&cost| cost <= xp);
            let mut best = (0usize, 0usize);
            for path in 0..3 {
                for tier in 0..5 {
                    let Some(cand) = name_of(path, tier) else { continue };
                    if owned(&cand.name) { continue; }
                    let Some(b) = name_of(best.0, best.1) else { best = (path, tier); continue };
                    let (cand_aff, best_aff) = (affordable(&cand.name), affordable(&b.name));
                    let prefer = if b.tier < cand.tier || (cand_aff && !best_aff) { cand_aff || !best_aff } else { false };
                    if owned(&b.name) || prefer { best = (path, tier); }
                }
            }
            menus.pick = Some(Pick::Upgrade(best.0, best.1));
        }
        // Upgrade cards: all unlocked (blue, gold at tier 5), the picked one glowing.
        for path in 0..3 {
            for tier in 0..5 {
                let c = format!("{UPG}/UpgradeInfo/UpgradePaths/Path {} Upgrades/Upgrade{tier}", path + 1);
                let up = name_of(path, tier);
                let container = if up.is_none() {
                    "UpgradeContainerGrey"
                } else if tier == 4 {
                    "UpgradeContainerTier5"
                } else {
                    "UpgradeContainerBlue"
                };
                u.set_sprite(&format!("{c}/UpgradeButton"), Some(container));
                let icon = up.and_then(|x| icon_of(&self.icons, &x.name, path, tier));
                u.set_sprite(&format!("{c}/UpgradeButton/UpgradeIcon"), icon.as_deref());
                u.set_active(&format!("{c}/UpgradeButton/UpgradeIcon"), icon.is_some());
                if let Some(i) = u.find_mut(&format!("{c}/UpgradeButton/UpgradeIcon")).and_then(|n| n.img.as_mut()) {
                    i.preserve = true;
                }
                let label = up.map(|x| menus.loc.get(&x.name).cloned().unwrap_or_else(|| x.name.clone())).unwrap_or_default();
                u.set_text(&format!("{c}/UpgradeButton/UpgradeName"), &label);
                u.set_active(&format!("{c}/SelectedGlow"), menus.pick == Some(Pick::Upgrade(path, tier)));
                // UpgradeDetails.OnClick -> SelectUpgrade: clicking selects (never deselects).
                if up.is_some() && u.clicked(&format!("{c}/UpgradeButton")) {
                    menus.pick = Some(Pick::Upgrade(path, tier));
                    menus.pick_without_account = false;
                }
            }
        }
        // Paragon card (towers that have one).
        u.set_active(&format!("{UPG}/UpgradeInfo/ParagonContainer"), paragon.is_some() && tt.is_some());
        u.set_active(&format!("{UPG}/UpgradeInfo/ParagonContainer/ParagonPanel"), paragon_panel);
        // SwitchTower / UpdateUi: once the Paragon itself is owned, the Paragon title panel shows and
        // the title and description switch to their Paragon-styled copies (same text).
        let paragon_owned = has(&format!("{id} Paragon"));
        u.set_active(&format!("{UPG}/TowerInfo/ParagonTitlePanel"), paragon_owned);
        for (normal, styled) in [("TowerTitle", "TowerTitleParagon"), ("TowerDescription", "TowerDescriptionParagon")] {
            u.set_active(&format!("{UPG}/TowerInfo/{normal}"), !paragon_owned);
            u.set_active(&format!("{UPG}/TowerInfo/{styled}"), paragon_owned);
        }
        u.set_text(&format!("{UPG}/TowerInfo/TowerTitleParagon"), &tower_name);
        u.set_text(&format!("{UPG}/TowerInfo/TowerDescriptionParagon"), &description);
        if let Some(pname) = &paragon {
            u.set_sprite(&format!("{PARAGON}/UpgradeButton"), Some("UpgradeContainerParagon"));
            let icon = icon_of(&self.icons, &format!("{id} Paragon"), 3, 0).or_else(|| icon_of(&self.icons, pname, 3, 0));
            u.set_sprite(&format!("{PARAGON}/UpgradeButton/UpgradeIcon"), icon.as_deref());
            u.set_active(&format!("{PARAGON}/UpgradeButton/UpgradeIcon"), icon.is_some());
            u.set_text(&format!("{PARAGON}/UpgradeButton/UpgradeName"), pname);
            u.set_active(&format!("{PARAGON}/SelectedGlowParagon"), menus.pick == Some(Pick::Paragon));
            if u.clicked(&format!("{PARAGON}/UpgradeButton")) {
                menus.pick = Some(Pick::Paragon);
                menus.pick_without_account = false;
            }
        }

        // Big portrait, name and in-game cost of the picked upgrade (or the base tower).
        let (title, cost, portrait_key) = match menus.pick {
            Some(Pick::Upgrade(path, tier)) => {
                let up = name_of(path, tier);
                let mut key = [b'0'; 3];
                key[path] = b'1' + tier as u8;
                (
                    up.map(|x| menus.loc.get(&x.name).cloned().unwrap_or_else(|| x.name.clone())).unwrap_or_default(),
                    up.map(|x| x.cost),
                    String::from_utf8(key.to_vec()).unwrap_or_default(),
                )
            }
            Some(Pick::Paragon) => (paragon.clone().unwrap_or_default(), None, "Paragon".into()),
            None => (tower_name.clone(), Some(tower_cost), "000".into()),
        };
        u.set_text(&format!("{PORTRAIT}/UpgradeName"), &title);
        u.set_active(&format!("{PORTRAIT}/LayoutGroup/Cost"), cost.is_some());
        u.set_text(&format!("{PORTRAIT}/LayoutGroup/Cost/InGameCostValue"), &cost.map(money).unwrap_or_default());
        let portrait = if portrait_key == "Paragon" {
            menus.paragon_portraits.get(&id).filter(|n| self.sprites.has(n)).cloned()
                .or_else(|| portrait_name(&mut self.sprites, &id, &portrait_key, paragon.as_deref()))
        } else {
            portrait_name(&mut self.sprites, &id, &portrait_key, paragon.as_deref())
        };
        u.set_sprite(&format!("{PORTRAIT}/Portrait/PortraitImage"), portrait.as_deref());
        u.set_active(&format!("{PORTRAIT}/Portrait/PortraitImage"), portrait.is_some());
        // UpgradeScreen.UpdatePortraitBackground: TowerContainerSpritesAsset.large[towerSet], or
        // large.paragon (TowerSet 0x20) while the Paragon is selected.
        let bg = match (menus.pick, set) {
            (Some(Pick::Paragon), _) => "TowerContainerParagonLarge",
            (_, 2) => "TowerContainerMilitaryLarge",
            (_, 4) => "TowerContainerMAgicLarge",
            (_, 8) => "TowerContainerSupportLarge",
            _ => "TowerContainerPrimaryLarge",
        };
        u.set_sprite(&format!("{PORTRAIT}/Portrait/PortraitBackground"), Some(bg));

        // Previous / next tower (shop order, wrapping).
        let at = towers.iter().position(|&i| i == menus.tower).unwrap_or(0);
        let n = towers.len().max(1);
        if u.clicked(&format!("{UPG}/TowerInfo/NextTower/Button")) {
            menus.tower = towers[(at + 1) % n];
            menus.pick = None;
        }
        if u.clicked(&format!("{UPG}/TowerInfo/PrevTower/Button")) {
            menus.tower = towers[(at + n - 1) % n];
            menus.pick = None;
        }
        if let Some(f) = menus.fore.as_ref() {
            if f.clicked(&format!("{FORE}/Back/Back")) {
                out.push(MenuAction::Close);
            }
        }

        let mut ctx = Ctx { sprites: &mut self.sprites, fonts: &self.fonts, scale: s, canvas_h, clip: None };
        draw_tree(u, full, &mut ctx);
        if let Some(f) = menus.fore.as_mut() {
            draw_tree(f, full, &mut ctx);
        }

        // UpgradeDetails.OnPointerEnter / Update / OnPointerExit: the bubble shows for the card under
        // the pointer after 0.5 s and hides as soon as the pointer leaves it; selecting doesn't show it.
        let pointer: Vec2 = mouse_position().into();
        let hovered = self.menus.upgrades.as_ref().and_then(|u| {
            let mut cards: Vec<(Pick, String)> = (0..3).flat_map(|p| (0..5).map(move |t| (Pick::Upgrade(p, t),
                format!("{UPG}/UpgradeInfo/UpgradePaths/Path {} Upgrades/Upgrade{t}", p + 1)))).collect();
            cards.push((Pick::Paragon, PARAGON.to_string()));
            cards.into_iter().find(|(_, c)| u.find(c).is_some_and(|n| n.active) && u.rect(&format!("{c}/UpgradeButton")).is_some_and(|r| r.contains(pointer))).map(|(p, _)| p)
        });
        let now = get_time();
        self.menus.hover = match (hovered, self.menus.hover) {
            (Some(p), Some((q, began, shown))) if p == q => Some((p, began, shown || now - began >= 0.5)),
            (Some(p), _) => Some((p, now, false)),
            (None, _) => None,
        };
        let shown = self.menus.hover.filter(|h| h.2).map(|h| h.0);
        // The bubble shows the hovered upgrade's text (UpgradeModel.LocsKey + " Description").
        let card = match shown {
            Some(Pick::Upgrade(path, tier)) => Some((format!("{UPG}/UpgradeInfo/UpgradePaths/Path {} Upgrades/Upgrade{tier}", path + 1), tier == 0,
                name_of(path, tier).map(|x| x.name.clone()).unwrap_or_default())),
            Some(Pick::Paragon) => Some((PARAGON.to_string(), false, paragon.clone().unwrap_or_default())),
            None => None,
        };
        if let Some((card, first_col, key)) = card {
            let desc = self.menus.loc.get(&format!("{key} Description")).cloned().unwrap_or_default();
            let r = self.menus.upgrades.as_ref().and_then(|u| u.rect(&card));
            let popup = if first_col { self.menus.popup_col1.as_mut() } else { self.menus.popup.as_mut() };
            if let (Some(r), Some(pop), false) = (r, popup, desc.is_empty()) {
                pop.set_text("PopUpDescription", &desc);
                // The prefab's root pivot sits below the bubble, with the Point tip at (-133, +120)
                // from it and the bubble's top edge at +568. When there is no room above the card
                // the bubble goes below it, its point mirrored.
                let cx = (r.x + r.w / 2.0) / s;
                let top = canvas_h - r.y / s;
                let bottom = canvas_h - (r.y + r.h) / s;
                let below = top - 110.0 + 568.0 > canvas_h;
                let at = if below { vec2(cx + 133.0, bottom + 10.0 - 616.0) } else { vec2(cx + 133.0, top - 110.0) };
                if let Some(p) = pop.find_mut("Point") {
                    p.rt.pos.y = if below { 301.0 } else { -125.0 };
                    p.rt.scale.y = if below { -1.0 } else { 1.0 };
                }
                draw_tree(pop, R { min: at, size: Vec2::ZERO }, &mut ctx);
            }
        }
        out
    }
}

/// MonkeyPortraits sprite for a tier key ("300") or the paragon.
pub(crate) fn portrait_name(sprites: &mut Sprites, id: &str, key: &str, paragon: Option<&str>) -> Option<String> {
    let mut names = vec![format!("{key}-{id}"), format!("{key}-{}{}", &id[..1], id[1..].to_lowercase())];
    if key == "Paragon" {
        names = vec![format!("Paragon-{}", paragon.unwrap_or("").replace(' ', ""))];
    }
    names.into_iter().find(|n| sprites.get(n).is_some())
}
