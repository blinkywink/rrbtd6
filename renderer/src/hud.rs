//! The in-game HUD built from the game's own UI prefabs (tools/assets/dump_ui_prefab.py):
//! MainHudLeftAlign (lives, cash), MainHudRightAlign (round), RightPanel (tower shop, Go /
//! fast forward) and TowerSelectionMenu + TSMThemeDefault (upgrades). Prefab objects that
//! only other game modes use are switched off here, as the game's scripts would.

use crate::game::{Game, TargetMode};
use crate::ui::{find_icon, icon_key, Fonts, Sprites, TextBox, TmpStyle};
use crate::uitree::{draw_tree, Ctx, Node, R};
use macroquad::prelude::*;
use std::collections::HashMap;
use std::path::Path;
#[path = "coop_hud.rs"]
mod coop_hud;

/// CanvasScaler: ScaleWithScreenSize, 3840x2160, Expand.
const REF: Vec2 = vec2(3840.0, 2160.0);
/// ShopMenu16:9 width (canvas units).
const SHOP_W: f32 = 600.0;

const SHOP: &str = "ShopMenu16:9";
const TSM_OPTS: &str = "Ratio/Scaler/TowerElements/SelectedTowerOptions";
const THEME: &str = "Ratio/Scaler/TowerElements/SelectedTowerInformation/TowerSelectionMenuThemes/TSMThemeDefault";
const INFO: &str = "Ratio/Scaler/InfoPanel";
/// TowerSelectionMenu.GetAdvancedStat's rows.
const ADV_STATS: usize = 7;

pub enum Action {
    Coop(rrbtd6_coop::Action),
    Buy(usize),
    /// A tower pulled sideways out of the shop: placed where the mouse is released.
    Drag(usize),
    Go,
    Upgrade(usize),
    Sell,
    /// The advanced stats' eye button: the panel shows Pops (true) or Damage (false); saved.
    ShowPops(bool),
    Target(TargetMode),
    /// "Set Target": the next click on the map sets the selected tower's aim point.
    SetPoint,
    Close,
    /// Gear button: the pause screen.
    Pause,
    /// Shop "Upgrades" button: the upgrade tree of this shop tower.
    Upgrades(usize),
    /// An ability button (by ability name).
    Ability(String),
    /// The sandbox bloon menu.
    Sandbox(crate::sandbox::SandboxAction),
    /// The temple popup's OK: buy (tower, path).
    TempleUpgrade(usize, usize),
    /// A Geraldo shop item picked: used now, or taken in hand for the next map click.
    GeraldoItem(usize),
    /// The camo priority toggle (TSMThemeCamo...): flip it on the selected tower.
    CamoPrio,
    /// AmbidextrousRangs: throw with the left (true) or right hand.
    Hand(bool),
    /// TSMThemeMonkeySub's submerge button: submerge / surface the selected Sub.
    Submerge,
    /// TSMThemeMonkeyAce's reverse button.
    Reverse,
    /// TSMThemeSuperMonkey's left-hand target row: set the off hand's mode.
    LeftTarget(TargetMode),
    /// TSMThemeBananaFarm: collect this bank (false) or every bank (true).
    BankCollect(bool),
}

/// A press on the shop list: becomes a scroll drag once it moves past the drag threshold,
/// otherwise a click when released (ScrollRectWithDragCancel).
struct Press {
    start: Vec2,
    from: f32,
    dragging: bool,
}

pub struct Hud {
    pub(crate) account_ui:crate::account_ui::AccountUi,
    coop: coop_hud::CoopHud,
    left: Node,
    race_ui:Option<Node>,
    right: Node,
    shop: Node,
    tsm: Node,
    pub(crate) sprites: Sprites,
    pub(crate) fonts: Fonts,
    /// icon_key(upgrade name) -> sprite name.
    pub(crate) icons: HashMap<String, String>,
    /// Ability ("...AA") icons by normalised ability / upgrade name.
    pub(crate) ability_icons: HashMap<String, String>,
    pub scale: f32,
    /// Shop button shown as selected (name box at the top of the shop).
    shop_sel: usize,
    /// Shop list scroll offset (canvas units) and ScrollRect inertia (units/s).
    scroll: f32,
    scroll_vel: f32,
    press: Option<Press>,
    /// TSM "i" button: the InfoPanel (next upgrade descriptions, stats) is shown.
    info_open: bool,
    /// The panel shows Pops instead of Damage (the player's setting; Damage by default).
    pub show_pops: bool,
    /// Seconds since the upgrade menu opened on the left (lives/cash slide out of its way).
    stats_slide: Option<f32>,
    green: TmpStyle,
    brown: TmpStyle,
    pub(crate) menus: crate::menus::Menus,
    /// Ability buttons drawn this frame (screen px).
    ability_rects: Vec<Rect>,
    paragon_portraits: HashMap<String, String>,
    pub(crate) sandbox: crate::sandbox::SandboxUi,
    pub(crate) temple: crate::temple_popup::TemplePopup,
    pub(crate) support: crate::support_popup::SupportPopup,
    pub(crate) geraldo: crate::geraldo_shop::GeraldoShop,
    /// The grafted controls of each upgrade panel theme, by theme id.
    theme_controls: HashMap<String, Vec<String>>,
}

/// EventSystem pixelDragThreshold (screen px).
const DRAG_THRESHOLD: f32 = 10.0;
/// ScrollRect decelerationRate (velocity kept per second).
const DECELERATION: f32 = 0.135;
/// Shop scroll per mouse-wheel notch (canvas units, half a button row).
const WHEEL_STEP: f32 = 162.0;

/// MainHudLeftAlign "StatsPanel" controller: LeftGroup anchoredPosition.x, sampled over
/// StatsSlideInFromLeft (0.167 s); StatsSlideInFromRight snaps back to the first value.
const STATS_SLIDE: [f32; 11] = [22.0, 225.6, 429.2, 632.8, 836.4, 1040.0, 1034.2, 1019.5, 999.7, 978.6, 960.0];
const STATS_SLIDE_LEN: f32 = 0.1667;

fn stats_slide_x(t: f32) -> f32 {
    let f = (t / STATS_SLIDE_LEN).clamp(0.0, 1.0) * (STATS_SLIDE.len() - 1) as f32;
    let i = (f as usize).min(STATS_SLIDE.len() - 2);
    STATS_SLIDE[i] + (STATS_SLIDE[i + 1] - STATS_SLIDE[i]) * (f - i as f32)
}

/// BTD6 money text: "$1,400".
pub fn money(v: f64) -> String {
    let digits = format!("{:.0}", v.max(0.0).floor());
    let mut out = String::from("$");
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn hide(n: &mut Node, paths: &[&str]) {
    for p in paths {
        n.set_active(p, false);
    }
}

impl Hud {
    pub(crate) fn draw_account_ui(&mut self) {
        let mut ui=std::mem::take(&mut self.account_ui);
        let player=crate::frontend::account_preview::player_name();
        ui.draw_account(self,player.as_deref());
        self.account_ui=ui;
    }

    pub async fn load(root: &Path, game: &Game) -> Option<Hud> {
        let mut budget=crate::load_work::Budget::new();
        let dir = root.join("ui").join("prefabs");
        let worker_dir=dir.clone();
        let (mut left,mut right,mut shop,mut tsm,theme,button)=crate::load_work::spawn(move||Some((
            Node::load(&worker_dir.join("MainHudLeftAlign.json"))?,Node::load(&worker_dir.join("MainHudRightAlign.json"))?,
            Node::load(&worker_dir.join("RightPanel.json"))?,Node::load(&worker_dir.join("TowerSelectionMenu.json"))?,
            Node::load(&worker_dir.join("TSMThemeDefault.json"))?,Node::load(&worker_dir.join("TowerPurchaseButton.json"))?
        ))).await?;
        // TowerSelectionMenu.GetAdvancedStat: Damage, Pops, Round Placed, Current Value, Cash
        // Spent, Cash Earnt, Lives Gained, each a copy of the Stat row.
        if let Some(content) = tsm.find_mut(&format!("{INFO}/AdvancedStats/Anim/Viewport/Content")) {
            if let Some(tpl) = content.children.iter().find(|c| c.name == "Stat").cloned() {
                content.children.retain(|c| c.name != "Stat");
                for i in 0..ADV_STATS {
                    let mut row = tpl.clone();
                    row.name = format!("Stat{i}");
                    content.children.push(row);
                }
            }
        }

        let lg = "LeftGroup/LayoutGroup";
        hide(&mut left, &["LeftGroup/TwitchButton"]);
        for p in ["ManaShield", "SoulBarrier", "LivesDrain", "BossRushLives"] {
            left.set_active(&format!("{lg}/HealthGroup/{p}"), false);
        }
        for p in ["ThriveGlow", "CashGoal", "Debt", "DoubleCashMode"] {
            left.set_active(&format!("{lg}/CashGroup/{p}"), false);
        }
        let rp = "Anim/RoundPanel";
        hide(&mut right, &[
            "Anim/RoundPanel/RoundGroup/FastTrack",
            "Anim/RoundPanel/RoundMixUp",
            "Anim/RoundPanel/Checkpoint",
            "Anim/RoundPanel/RogueInventoryBtn",
            "Anim/RoundPanel/RogueQueuedBoostBtn",
            "Anim/RoundPanel/PauseButton/DisabledImg",
            "Anim/RoundPanel/PauseButton/PauseCooldown",
        ]);
        right.set_text(&format!("{rp}/RoundGroup/Title"), "Round");

        hide(&mut shop, &["CancelPurchaseButton", "ConfirmlPurchaseButton", "ShopMenu4:3", "ShopMenu3D"]);
        for p in ["UITowersFrontier", "UITowersRogue", "ToggleBloons", "SendRound", "ReviewMapReturn", "EditorBackBtn", "EditorTowerBackBtn", "GoButtonGlowPulse"] {
            shop.set_active(&format!("{SHOP}/{p}"), false);
        }
        shop.set_active(&format!("{SHOP}/PowersButton"), true);
        let portraits = root.join("ui").join("portraits");
        let mut extra = vec![root.join("ui").join("upgrades"), root.join("ui").join("panels")];
        for e in std::fs::read_dir(&portraits).into_iter().flatten().flatten() {
            extra.push(e.path());
        }
        extra.push(root.join("ui/hero_skins"));
        extra.push(root.join("ui/menu_icons"));
        let mut sprites = Sprites::load(root, &extra).await;
        // Tower -> its Paragon portrait sprite (ui/portraits/<tower>/Paragon-*.png).
        let paragon_portraits: HashMap<String, String> = std::fs::read_dir(&portraits)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|d| {
                let tower = d.file_name().to_string_lossy().to_string();
                let f = std::fs::read_dir(d.path()).ok()?.flatten().find(|f| f.file_name().to_string_lossy().starts_with("Paragon-"))?;
                Some((tower, f.file_name().to_string_lossy().trim_end_matches(".png").to_string()))
            })
            .collect();
        if let Some(sorter) = shop.find_mut(&format!("{SHOP}/UITowers/Mask/Sorter")) {
            sorter.children.clear();
            for (i, item) in game.shop.iter().enumerate() {
                let mut b = button.clone();
                b.name = format!("Tower{i}");
                hide(&mut b, &["Button/AbleToUpgradeIcon", "Button/TowerCountIcon", "Button/ParagonAvailable"]);
                // TowerPurchaseButton2D.DetermineBackgroundSprite: primary / military / magic /
                // support / hero sprite fields (BigPanels TowerContainer*, not the portrait frames).
                let container = match item.set {
                    2 => "TowerContainerMilitary",
                    4 => "TowerContainerMagic",
                    8 => "TowerContainerSupport",
                    16 => "TowerContainerHero",
                    _ => "TowerContainerPrimary",
                };
                b.set_sprite("Button", Some(container));
                let icon = item.icon.clone().filter(|n| sprites.get(n).is_some()).unwrap_or_else(|| format!("{}Icon", item.id));
                b.set_sprite("Button/Icon", Some(&icon));
                // The portraits aren't square (e.g. 201x259); the button keeps their shape.
                if let Some(i) = b.find_mut("Button/Icon").and_then(|n| n.img.as_mut()) {
                    i.preserve = true;
                }
                b.set_text("Button/Text", &money(item.cost));
                sorter.children.push(b);
                budget.checkpoint().await;
            }
        }

        hide(&mut tsm, &[
            "Ratio/Scaler/Abilities",
            "Ratio/Scaler/InfoPanel",
            "Ratio/Scaler/TowerElements/GeraldoShopContainer",
            "Ratio/Scaler/TowerElements/FrontierTraitsTab",
            "Ratio/Scaler/TowerElements/FrontierTraits",
            "Ratio/Scaler/TowerElements/RemoveTower",
        ]);
        for p in ["SheriffDetails", "HeroDetails", "ParagonDetails", "PowerProDetails"] {
            tsm.set_active(&format!("{TSM_OPTS}/{p}"), false);
        }
        tsm.set_active(&format!("{TSM_OPTS}/TowerDetails"), true);
        for row in 1..=3 {
            let u = format!("{TSM_OPTS}/TowerDetails/UpgradeObject_{row}");
            for p in ["PreviousUpgrade/BeastMergedBg", "PreviousUpgrade/BeastPower", "PreviousUpgrade/Cost", "NextUpgrade/UnlockIcon",
                "NextUpgrade/LockIcon", "NextUpgrade/MonkeyVillageRequirement", "NextUpgrade/CostBeastMerge", "NextUpgrade/NeedsBeasts",
                "NextUpgrade/BeastPowerAdd", "NextUpgrade/RestrictedIcon", "RestrictedPath"] {
                tsm.set_active(&format!("{u}/{p}"), false);
            }
            for t in 1..=5 {
                tsm.set_active(&format!("{u}/TierIndicators/Tier {t}/Restricted"), false);
            }
        }
        let mut theme = theme;
        hide(&mut theme, &["TSMCoopPlayerID", "TSMPopInfoAll/CashEarnt", "TargetType/Bar", "TargetType/Count", "UpgradeToggleButton"]);
        theme.set_sprite("BackgroundImage", Some("PortraitContainerPrimary"));
        // "Set Target" buttons from the Mortar and select-point themes, shown when needed.
        let action = |file: &str| Node::load(&dir.join(file)).and_then(|n| n.find("TSMActionButton").cloned());
        if let Some(mut b) = action("TSMThemeMortarMonkey.json") {
            b.active = false;
            theme.children.push(b);
        }
        if let Some(mut b) = action("TSMThemeSelectPointInput.json") {
            b.name = "TSMActionButtonSquare".into();
            b.active = false;
            let mut icon = b.clone();
            icon.name = "Icon".into();
            icon.children.clear();
            icon.button = false;
            icon.rt.amin = vec2(0.5, 0.5);
            icon.rt.amax = vec2(0.5, 0.5);
            icon.rt.pos = Vec2::ZERO;
            icon.rt.size = vec2(130.0, 130.0);
            icon.active = true;
            if let Some(i) = icon.img.as_mut() {
                i.sprite = Some("TargetSetIcon".into());
                i.kind = 0;
            }
            b.children.push(icon);
            theme.children.push(b);
        }
        // Every other theme's own controls (TowerModel.towerSelectionMenuThemeId: the camo
        // priority toggle, the Sub's submerge button, Banana Farm bank buttons...), grafted on
        // as "<Theme>_<control>" and shown for towers using that theme. Mortar and select-point
        // have their buttons above.
        let mut theme_controls: HashMap<String, Vec<String>> = HashMap::new();
        let base: std::collections::HashSet<String> = theme.children.iter().map(|c| c.name.clone()).collect();
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            budget.checkpoint().await;
            let f = e.file_name().to_string_lossy().to_string();
            let Some(id) = f.strip_prefix("TSMTheme").and_then(|x| x.strip_suffix(".json")) else { continue };
            let id = if id == "WithActionButton" { "ActionButton" } else { id };
            if ["Default", "MortarMonkey", "SelectPointInput", "DefaultProp"].contains(&id) {
                continue;
            }
            let Some(t) = Node::load(&e.path()) else { continue };
            for mut c in t.children.into_iter().filter(|c| !base.contains(&c.name)) {
                let name = format!("{id}_{}", c.name);
                c.name = name.clone();
                c.active = false;
                theme_controls.entry(id.to_string()).or_default().push(name);
                theme.children.push(c);
            }
        }
        if let Some(n) = tsm.find_mut("Ratio/Scaler/TowerElements/SelectedTowerInformation/TowerSelectionMenuThemes") {
            n.children.push(theme);
        }

        let fonts = Fonts::load(root, &[("Btd6FontTitleSDF", "fonts/title"), ("Btd6FontBodySDF", "fonts/body")]).await;
        // Start reading the cached account now, for the upgrade screen's selection and Paragon panel.
        crate::frontend::account_preview::ensure(root);
        let _ = crate::ui::UPGRADE_ICONS.set(std::fs::read_to_string(root.join("ui/runtime/upgrade_icons.json")).ok()
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default());
        let mut icons = HashMap::new();
        // Ability icons ("...AA"), keyed without "UpgradeIcon" / "Icon" / "AA".
        let mut ability_icons = HashMap::new();
        for e in std::fs::read_dir(root.join("ui").join("upgrades")).into_iter().flatten().flatten() {
            let f = e.file_name().to_string_lossy().to_string();
            if let Some(stem) = f.strip_suffix(".png") {
                icons.insert(icon_key(stem), stem.to_string());
                if stem.contains("AA") {
                    let k = icon_key(&stem.replace("UpgradeIcon", "").replace("Icon", "").replace("AA", ""));
                    ability_icons.entry(k).or_insert_with(|| stem.to_string());
                }
            }
        }
        // Upgrade text styles: the prefab's green (unlocked) material, and OutlineBrown for locked.
        let green = tsm
            .find(&format!("{TSM_OPTS}/TowerDetails/UpgradeObject_1/NextUpgrade/Cost"))
            .and_then(|n| n.txt.as_ref())
            .map(|t| t.style.clone())
            .unwrap_or_default();
        let brown_c = Color::new(0.290, 0.188, 0.129, 1.0);
        let mut brown = green.clone();
        brown.outline = brown_c;
        if let Some(u) = brown.underlay.as_mut() {
            u.0 = brown_c;
        }
        let worker_root=root.to_owned();let worker_dir=dir.clone();
        let (menus,coop,sandbox,temple,support,geraldo)=crate::load_work::spawn(move||(
            crate::menus::Menus::load(&worker_root),coop_hud::CoopHud::load(&worker_dir),
            crate::sandbox::SandboxUi::load(&worker_dir),crate::temple_popup::TemplePopup::load(&worker_dir),
            crate::support_popup::SupportPopup::load(&worker_dir),crate::geraldo_shop::GeraldoShop::load(&worker_dir)
        )).await;
        let mut race_ui=if game.race.is_some(){Node::load(&dir.join("RaceUi.json"))}else{None};
        if let Some(race)=&mut race_ui{race.rt=right.rt.clone();}
        Some(Hud {
            account_ui:Default::default(),
            coop,
            left,race_ui,
            right,
            shop,
            tsm,
            sprites,
            fonts,
            icons,
            ability_icons,
            scale: 1.0,
            shop_sel: 0,
            scroll: 0.0,
            scroll_vel: 0.0,
            press: None,
            info_open: std::env::var("RRBTD6_INFO").is_ok(),
            // settings.json "show_pops" (the advanced stats' eye button), Damage by default.
            show_pops: std::fs::read_to_string(root.join("settings.json"))
                .ok()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .and_then(|v| v["show_pops"].as_bool())
                .unwrap_or(false),
            stats_slide: None,
            green,
            brown,
            menus,
            ability_rects: Vec::new(),
            paragon_portraits,
            sandbox,temple,support,geraldo,
            theme_controls,
        })
    }

    /// Screen width taken by the tower shop (the map viewport is the rest).
    pub fn panel_px() -> f32 {
        SHOP_W * Self::canvas_scale()
    }

    pub(crate) fn canvas_scale() -> f32 {
        (screen_width() / REF.x).min(screen_height() / REF.y)
    }

    /// True if the point is over a HUD panel or button (map input should ignore it).
    pub fn blocks(&self, p: Vec2) -> bool {
        p.x >= screen_width() - Self::panel_px()
            || self.tsm.find("Ratio/Scaler/BG1").is_some_and(|n| self.tsm.active && n.screen.contains(p))
            || self.tsm.find(INFO).is_some_and(|n| self.tsm.active && n.active && n.screen.contains(p))
            || self.right.rect("Anim/RoundPanel/PauseButton").is_some_and(|r| r.contains(p))
            || self.ability_rects.iter().any(|r| r.contains(p))
            || self.sandbox_blocks(p)
            || self.temple_blocks()
            || self.geraldo_blocks(p)
            || self.coop.blocks(p)
    }

    /// Ability buttons along the bottom left of the map (AbilityButtons: 250-unit slots, 180-unit
    /// icons): the granting upgrade's icon, a dark pie for the cooldown left with the seconds,
    /// and how many are ready when several towers have it.
    fn draw_abilities(&mut self, game: &Game, out: &mut Vec<Action>) {
        let s = Self::canvas_scale();
        self.ability_rects.clear();
        let m: Vec2 = mouse_position().into();
        for (i, g) in game.ability_groups().iter().enumerate() {
            let size = 200.0 * s;
            let r = Rect::new((40.0 + i as f32 * 230.0) * s, screen_height() - (40.0 * s + size), size, size);
            let c = r.center();
            let ready = g.ready > 0;
            draw_circle(c.x, c.y, size * 0.5, Color::from_rgba(40, 30, 20, 200));
            draw_circle_lines(c.x, c.y, size * 0.5, 6.0 * s, if ready { Color::from_rgba(255, 220, 60, 255) } else { Color::from_rgba(90, 70, 50, 255) });
            let tw = &game.towers[g.tower];
            let tower = &game.tower_defs[tw.def].0;
            // The ability's own icon ("...AA": MonkeyPiratesUpgradeIconAA, RapidShotAA), by the
            // ability's or its upgrade's name; else the upgrade's icon, as the game shows.
            let ab = game.tier(g.tower).and_then(|tr| tr.abilities.iter().find(|a| a.name == g.name));
            let upgrade = ab.map(|a| a.upgrade.clone().unwrap_or_else(|| a.name.clone()));
            let own = std::iter::once(g.name.clone())
                .chain(upgrade.iter().flat_map(|u| [u.clone(), self.menus.loc.get(u).cloned().unwrap_or_default()]))
                .map(|n| crate::ui::icon_key(n.rsplit('-').next().unwrap_or(&n)))
                .filter(|k| !k.is_empty())
                .find_map(|k| self.ability_icons.get(&k).cloned());
            let icon = own.or_else(|| {
                let up = game.tower_tiers[tw.def].as_ref()?.upgrades.iter().find(|x| Some(&x.name) == upgrade.as_ref())?;
                self.icon(&up.name, tower, up.path, up.tier)
            });
            let isz = 180.0 * s;
            match icon.as_deref().and_then(|n| self.sprites.get(n)) {
                Some(sp) => {
                    let (w, h) = (sp.tex.width(), sp.tex.height());
                    let k = isz / w.max(h);
                    draw_texture_ex(&sp.tex, c.x - w * k / 2.0, c.y - h * k / 2.0, WHITE, DrawTextureParams {
                        dest_size: Some(vec2(w * k, h * k)),
                        ..Default::default()
                    });
                }
                None => draw_circle(c.x, c.y, isz * 0.4, Color::from_rgba(120, 200, 255, 255)),
            }
            if !ready {
                // Clockwise pie from 12 o'clock over the part of the cooldown still to go.
                let n = 48;
                let a0 = -std::f32::consts::FRAC_PI_2;
                let sweep = g.frac_left.clamp(0.0, 1.0) * std::f32::consts::TAU;
                let rr = size * 0.5;
                for k in 0..n {
                    let (u0, u1) = (a0 + sweep * k as f32 / n as f32, a0 + sweep * (k + 1) as f32 / n as f32);
                    draw_triangle(c, c + vec2(u0.cos(), u0.sin()) * rr, c + vec2(u1.cos(), u1.sin()) * rr, Color::from_rgba(0, 0, 0, 140));
                }
                self.fonts.draw("Btd6FontTitleSDF", &TextBox { gradient: None,
                    text: &format!("{:.0}", g.secs_left.ceil()),
                    rect: r,
                    size: 90.0 * s,
                    color: WHITE,
                    h_align: 2,
                    v_align: 512,
                    style: &self.green,
                    bold: false,
                    spacing: 0.0,
                });
            }
            if g.total > 1 {
                self.fonts.draw("Btd6FontTitleSDF", &TextBox { gradient: None,
                    text: &format!("{}", g.ready),
                    rect: Rect::new(r.right() - 70.0 * s, r.y - 10.0 * s, 80.0 * s, 80.0 * s),
                    size: 70.0 * s,
                    color: WHITE,
                    h_align: 2,
                    v_align: 512,
                    style: &self.green,
                    bold: false,
                    spacing: 0.0,
                });
            }
            if ready && is_mouse_button_pressed(MouseButton::Left) && m.distance(c) <= size * 0.5 {
                out.push(Action::Ability(g.name.clone()));
            }
            self.ability_rects.push(r);
        }
    }

    /// Large centred title text over the map (e.g. "Game Over"), OutlineBlack style.
    pub fn banner(&self, text: &str, color: Color) {
        let style = TmpStyle {
            outline_width: 0.25,
            face_dilate: 0.2,
            ratio_a: 0.8333,
            ratio_c: 0.5104,
            underlay: Some((Color::new(0.0, 0.0, 0.0, 0.5), vec2(0.0, -0.4), 0.4, 0.0)),
            ..Default::default()
        };
        let s = Self::canvas_scale();
        let w = screen_width() - Self::panel_px();
        self.fonts.draw("Btd6FontTitleSDF", &TextBox { gradient: None,
            text,
            rect: Rect::new(0.0, 0.0, w, screen_height()),
            size: 260.0 * s,
            color,
            h_align: 2,
            v_align: 512,
            style: &style,
            bold: false,
            spacing: 0.0,
        });
    }

    /// BuffManager.prefab over the selected tower at screen `at`: 100-unit icons in rows of up to
    /// four (GridLayoutGroup: cell 100, spacing 12 x 30, lower-centre, filling upward) whose
    /// bottom sits 200 units above the tower, with "x2"... for stacked buffs.
    pub fn draw_buffs(&mut self, at: Vec2, buffs: &[(String, u32)]) {
        if buffs.is_empty() {
            return;
        }
        let s = Self::canvas_scale();
        let (cell, sx, sy) = (100.0 * s, 12.0 * s, 30.0 * s);
        let per_row = 4;
        let style = TmpStyle::default();
        for (i, (icon, n)) in buffs.iter().enumerate() {
            let (row, col) = (i / per_row, i % per_row);
            let in_row = (buffs.len() - row * per_row).min(per_row) as f32;
            let w = in_row * cell + (in_row - 1.0) * sx;
            let x = at.x - w / 2.0 + col as f32 * (cell + sx);
            let y = at.y - 200.0 * s - cell - row as f32 * (cell + sy);
            if let Some(sp) = self.sprites.get(icon) {
                draw_texture_ex(&sp.tex, x, y, WHITE, DrawTextureParams { dest_size: Some(vec2(cell, cell)), ..Default::default() });
            }
            if *n > 1 {
                self.fonts.draw("Btd6FontTitleSDF", &TextBox { gradient: None,
                    text: &format!("x{n}"),
                    rect: Rect::new(x, y - 11.7 * s, cell, 132.1 * s),
                    size: 60.0 * s,
                    color: WHITE,
                    h_align: 2,
                    v_align: 1024,
                    style: &style,
                    bold: false,
                    spacing: 0.0,
                });
            }
        }
    }

    /// Floating world text (CashUp / LivesUp prefabs): the title font with a black outline,
    /// centred on `at` (screen px), `size` in canvas units.
    pub fn float_text(&self, text: &str, at: Vec2, size: f32, color: Color) {
        let style = TmpStyle {
            outline: Color::new(0.0, 0.0, 0.0, color.a),
            outline_width: 0.25,
            face_dilate: 0.2,
            ratio_a: 0.8333,
            ratio_c: 0.5104,
            ..Default::default()
        };
        let s = Self::canvas_scale();
        let (w, h) = (600.0 * s, 200.0 * s);
        self.fonts.draw("Btd6FontTitleSDF", &TextBox { gradient: None,
            text,
            rect: Rect::new(at.x - w / 2.0, at.y - h / 2.0, w, h),
            size: size * s,
            color,
            h_align: 2,
            v_align: 512,
            style: &style,
            bold: false,
            spacing: 0.0,
        });
    }

    fn icon(&self, name: &str, tower: &str, path: usize, tier: usize) -> Option<String> {
        find_icon(&self.icons, name, tower, path, tier)
    }

    pub fn draw(&mut self, game: &Game, selected: Option<usize>, placing: Option<usize>, speed: u32, auto_start: bool) -> Vec<Action> {
        let mut out = Vec::new();
        let s = Self::canvas_scale();
        self.scale = s;
        let canvas = vec2(screen_width() / s, screen_height() / s);
        let full = R { min: Vec2::ZERO, size: canvas };
        let map = R { min: Vec2::ZERO, size: vec2(canvas.x - SHOP_W, canvas.y) };

        // Lives / cash / round.
        let lg = "LeftGroup/LayoutGroup";
        self.left.set_text(&format!("{lg}/HealthGroup/Health"), &format!("{:.0}", game.lives.max(0.0)));
        self.left.set_text(&format!("{lg}/CashGroup/Cash"), &money(game.cash));
        let round = (game.round + 1).min(game.total_rounds());
        self.right.set_text("Anim/RoundPanel/RoundGroup/Round/Value", &format!("{round}/{}", game.total_rounds()));

        // Shop: Go / fast-forward state, affordability, hovered tower name.
        // Sandbox: the ToggleBloons button opens the bloon menu.
        let tb = format!("{SHOP}/ToggleBloons");
        self.shop.set_active(&tb, game.sandbox);
        if game.sandbox && self.shop.clicked(&tb) {
            self.sandbox.open = !self.sandbox.open;
        }
        let go = format!("{SHOP}/FastFoward-Go");
        // Sandbox: the button only toggles fast forward (rounds come from the sandbox menu's
        // round field and its send arrow).
        let running = game.round_active || game.sandbox;
        self.shop.set_active(&format!("{go}/Go"), !running);
        self.shop.set_active(&format!("{go}/FastForwardOff"), running && speed == 1);
        self.shop.set_active(&format!("{go}/FastForwardOn"), running && speed > 1);
        self.shop.set_active(&format!("{SHOP}/SendRound"),game.race.is_some());
        self.shop.set_active(&go,game.race.is_none());
        if game.race.is_some()&&self.shop.clicked(&format!("{SHOP}/SendRound")){out.push(Action::Go);}
        if let (Some(node),Some(race))=(&mut self.race_ui,&game.race){
            node.set_text("RaceTimer/RaceTimeTxt",&race.time_text());
            node.set_text("RaceTimer/Title",&race.name);
            node.hide_named(&["GhostTimeObject","Flash","AdditionalHudsPanel","RaceEventFlag"]);
        }
        // Shop list: a ScrollRect inside its RectMask2D (wheel, drag with inertia). A press
        // that turns into a drag doesn't click the tower under it.
        let list = format!("{SHOP}/UITowers/Mask");
        let m: Vec2 = mouse_position().into();
        let dt = get_frame_time().min(0.1);
        let mut tap: Option<Vec2> = None;
        let mut pull: Option<Vec2> = None;
        if let (Some(view), Some(sorter)) = (self.shop.find(&list), self.shop.find(&format!("{list}/Sorter"))) {
            let max = (sorter.rt.size.y - view.screen.h / s).max(0.0);
            let over = view.screen.contains(m);
            let wheel = mouse_wheel().1;
            if over && wheel != 0.0 {
                self.scroll -= wheel.clamp(-3.0, 3.0) * WHEEL_STEP;
                self.scroll_vel = 0.0;
            }
            if over && is_mouse_button_pressed(MouseButton::Left) {
                self.press = Some(Press { start: m, from: self.scroll, dragging: false });
                self.scroll_vel = 0.0;
            }
            if let Some(p) = self.press.as_mut() {
                if is_mouse_button_down(MouseButton::Left) {
                    let (dx, dy) = (m.x - p.start.x, m.y - p.start.y);
                    if !p.dragging && -dx > DRAG_THRESHOLD && -dx > dy.abs() {
                        pull = Some(p.start);
                    }
                    p.dragging |= dy.abs() > DRAG_THRESHOLD;
                    if p.dragging {
                        let to = (p.from - dy / s).clamp(0.0, max);
                        if dt > 0.0 {
                            let v = (to - self.scroll) / dt;
                            self.scroll_vel += (v - self.scroll_vel) * (dt * 10.0).min(1.0);
                        }
                        self.scroll = to;
                    }
                } else {
                    if !p.dragging {
                        tap = Some(p.start);
                    }
                    self.press = None;
                }
            } else if self.scroll_vel.abs() > 1.0 {
                self.scroll += self.scroll_vel * dt;
                self.scroll_vel *= DECELERATION.powf(dt);
            } else {
                self.scroll_vel = 0.0;
            }
            if pull.is_some() {
                self.press = None;
            }
            let clamped = self.scroll.clamp(0.0, max);
            if clamped != self.scroll {
                self.scroll = clamped;
                self.scroll_vel = 0.0;
            }
        }
        if let Some(sorter) = self.shop.find_mut(&format!("{list}/Sorter")) {
            sorter.rt.pos.y = self.scroll;
        }
        if let Some(def) = placing {
            if let Some(i) = game.shop.iter().position(|it| it.id == game.tower_defs[def].0) {
                self.shop_sel = i;
            }
        }
        for (i, item) in game.shop.iter().enumerate() {
            let b = format!("{list}/Sorter/Tower{i}");
            let afford = game.cash >= item.cost;
            // TowerPurchaseButton2D.SetNotEnoughCash: bg and icon get the GrayscaleUI material,
            // the cost text notEnoughCashTextColor (1, 0, 0).
            // LockTowerModModel / LockTowerSetModModel: SetLocked shows the lock over a grey tile.
            let locked = game.locked.iter().any(|l| *l == item.id);
            // One hero per game: once it's placed its tile is greyed out and can't be picked up.
            let hero_out = item.hero && game.towers.iter().any(|t| game.hero_def(t.def).is_some());
            self.shop.set_active(&format!("{b}/Button/Locked"), locked);
            self.shop.set_active(&format!("{b}/Button/Text"), !locked);
            self.shop.set_img_gray(&format!("{b}/Button"), !afford || locked || hero_out);
            self.shop.set_img_gray(&format!("{b}/Button/Icon"), !afford || locked || hero_out);
            self.shop.set_text_color(&format!("{b}/Button/Text"), if afford { WHITE } else { Color::new(1.0, 0.0, 0.0, 1.0) });
            self.shop.set_active(&format!("{b}/Button/Selected"), self.shop_sel == i);
            let hit = self.shop.rect(&format!("{b}/Button"));
            let tapped = tap.is_some_and(|p| hit.is_some_and(|r| r.contains(p) && r.contains(m)));
            let pulled = pull.is_some_and(|p| hit.is_some_and(|r| r.contains(p)));
            if tapped || pulled {
                self.shop_sel = i;
                // Only towers with simulation data can be placed so far.
                if let Some(def) = game.tower_defs.iter().position(|(n, _)| n == &item.id) {
                    if afford && !locked && !hero_out {
                        out.push(if pulled { Action::Drag(def) } else { Action::Buy(def) });
                    }
                }
            }
        }
        let sel_name = game.shop.get(self.shop_sel).map(|it| crate::spaced(&it.id)).unwrap_or_default();
        self.shop.set_text(&format!("{SHOP}/TowerSelected/Text"), &sel_name);
        if self.shop.clicked(&go) {
            out.push(Action::Go);
        }
        if self.right.clicked("Anim/RoundPanel/PauseButton") {
            out.push(Action::Pause);
        }
        if self.shop.clicked(&format!("{SHOP}/UpgradeTreeButton")) {
            // Heroes have their own screen; show the first tower instead.
            let i = if game.shop.get(self.shop_sel).is_some_and(|it| !it.hero) {
                self.shop_sel
            } else {
                game.shop.iter().position(|it| !it.hero).unwrap_or(0)
            };
            out.push(Action::Upgrades(i));
        }

        // Upgrade menu.
        let tw = selected.and_then(|t| game.towers.get(t).map(|tw| (t, tw)));
        self.tsm.active = tw.is_some();
        // MainHudRightAlign "OpenRight" state: the round panel slides left of an open menu.
        let menu_w = self.tsm.find("Ratio/Scaler/BG1").map_or(0.0, |n| n.screen.w / s);
        let open_right = tw.is_some_and(|(_, tw)| tw.pos[0] < (game.bounds.0[0] + game.bounds.1[0]) / 2.0);
        self.right.rt.pos.x = -25.0 - if open_right { menu_w } else { 0.0 };
        self.stats_slide = (tw.is_some() && !open_right).then(|| self.stats_slide.map_or(0.0, |t| t + dt));
        if let Some(g) = self.left.find_mut("LeftGroup") {
            g.rt.pos.x = self.stats_slide.map_or(STATS_SLIDE[0], stats_slide_x);
        }
        if let Some((t, tw)) = tw {
            let x = if open_right { 1.0 } else { 0.0 };
            self.tsm.rt.amin.x = x;
            self.tsm.rt.amax.x = x;
            self.tsm.rt.pivot.x = x;
            let (name, _) = &game.tower_defs[tw.def];
            let pdef = game.tower_tiers[tw.def].as_ref().and_then(|x| x.paragon.clone());
            let display = match (&tw.paragon, &pdef) {
                (Some(_), Some(p)) => self.menus.loc.get(&p.name).cloned().unwrap_or_else(|| crate::spaced(&p.name)),
                _ => crate::spaced(name),
            };
            self.tsm.set_text(&format!("{THEME}/TowerNameText"), &display);
            self.tsm.set_text(&format!("{THEME}/TowerNameText/Text"), &display);
            // TowerSelectionMenu.UpdatePopInfo: Damage, or Pops with the player's setting;
            // TSMThemeDefault.HideDamagePopsStatsCheck: neither for a tower that doesn't attack.
            let peaceful = game.tier(t).is_none_or(|x| x.attacks.is_empty());
            self.tsm.set_text(&format!("{THEME}/TSMPopInfoAll/Pops/PopText"), &tw.pops.to_string());
            self.tsm.set_text(&format!("{THEME}/TSMPopInfoAll/Damage/DamageText"), &tw.damage.to_string());
            self.tsm.set_active(&format!("{THEME}/TSMPopInfoAll/Pops"), !peaceful && self.show_pops);
            self.tsm.set_active(&format!("{THEME}/TSMPopInfoAll/Damage"), !peaceful && !self.show_pops);
            // Towers whose attacks have a single Target*Model (Tack Shooter) can't change mode.
            let modes = game.target_modes(t);
            let fixed = (modes.len() <= 1).then(|| modes.first().copied()).flatten();
            // A tower that only aims at a set point (Mortar) has the long "Set Target" button
            // instead of targeting arrows (TSMThemeMortarMonkey); the Dartling's "Locked" mode
            // adds the square one (TSMThemeSelectPointInput).
            let only_point = fixed == Some(TargetMode::Point);
            self.tsm.set_active(&format!("{THEME}/TargetType"), !modes.is_empty() && !only_point);
            self.tsm.set_active(&format!("{THEME}/TSMActionButton"), only_point);
            self.tsm.set_active(&format!("{THEME}/TSMActionButtonSquare"), !only_point && game.uses_point(t));
            for b in ["ForwardsButton", "BackwardsButton"] {
                self.tsm.set_active(&format!("{THEME}/TargetType/{b}"), modes.len() > 1);
            }
            self.tsm.set_text(&format!("{THEME}/TargetType/Text"), {
                let m = fixed.unwrap_or(tw.target);
                if tw.air.is_some() { m.air_label() } else { m.label() }
            });
            if self.tsm.clicked(&format!("{THEME}/TSMActionButton")) || self.tsm.clicked(&format!("{THEME}/TSMActionButtonSquare")) {
                out.push(Action::SetPoint);
            }
            // This tier's theme's own controls (towerSelectionMenuThemeId).
            let theme_id = game.tier(t).map(|x| x.tsm_theme.clone()).unwrap_or_default();
            for (id, names) in &self.theme_controls {
                for n in names {
                    self.tsm.set_active(&format!("{THEME}/{n}"), *id == theme_id);
                }
            }
            // The camo priority toggle; HYPOTHESIS P-CAMOTICK: its cross shows while it's on.
            // Boomerang Monkey (AmbidextrousRangs): the hand it throws with; the other greyed.
            let left = format!("{THEME}/AmbidextrousRangs_LeftHandButton");
            let right = format!("{THEME}/AmbidextrousRangs_RightHandButton");
            if theme_id == "AmbidextrousRangs" {
                self.tsm.set_img_gray(&format!("{left}/Icon"), !tw.flip);
                self.tsm.set_img_gray(&format!("{right}/Icon"), tw.flip);
                if self.tsm.clicked(&left) {
                    out.push(Action::Hand(true));
                } else if self.tsm.clicked(&right) {
                    out.push(Action::Hand(false));
                }
            }
            // Monkey Ace: reverse (with its cooldown clock); the long button sets the centre of
            // Centered Path. Heli Pilot: the long button sets the Lock In Place point.
            if theme_id == "MonkeyAce" {
                let rev = format!("{THEME}/MonkeyAce_ButtonGroup/ReverseToggle");
                self.tsm.set_active(&format!("{THEME}/MonkeyAce_ButtonGroup/ParagonCarpetBombSelectMultiPoint"), false);
                self.tsm.set_active(&format!("{rev}/CooldownEffect"), !game.can_reverse(t));
                if self.tsm.clicked(&rev) && game.can_reverse(t) {
                    out.push(Action::Reverse);
                }
            }
            for (id, mode) in [("MonkeyAce", TargetMode::Centered), ("HeliPilot", TargetMode::Point)] {
                if theme_id == id {
                    let b = format!("{THEME}/{id}_TSMActionButton");
                    self.tsm.set_active(&b, tw.target == mode);
                    self.tsm.set_active(&format!("{THEME}/TSMActionButtonSquare"), false);
                    if tw.target == mode && self.tsm.clicked(&b) {
                        out.push(Action::SetPoint);
                    }
                }
            }
            // Super Monkey with two arms (Robo Monkey...): a target row per hand instead of one.
            if theme_id == "SuperMonkey" {
                let hands = game.has_hands(t);
                let both = format!("{THEME}/SuperMonkey_BothHandsContainer");
                self.tsm.set_active(&both, hands);
                self.tsm.set_active(&format!("{THEME}/SuperMonkey_Text"), hands);
                // Made from a Dart Monkey by Geraldo's Worn Hero's Cape (its tiers are capped).
                self.tsm.set_active(&format!("{THEME}/SuperMonkey_TSMWornHeroCape"), tw.tier_cap < 5);
                if hands {
                    self.tsm.set_active(&format!("{THEME}/TargetType"), false);
                    self.tsm.set_text(&format!("{THEME}/SuperMonkey_Text"), tw.target.label());
                    self.tsm.set_text(&format!("{both}/TSMTargetTypeLeftHand/Text"), tw.left_target.label());
                    let r = format!("{both}/TSMTargetTypeRightHand");
                    let l = format!("{both}/TSMTargetTypeLeftHand");
                    if self.tsm.clicked(&format!("{r}/ForwardsButton")) {
                        out.push(Action::Target(game.cycle_target(t, false)));
                    } else if self.tsm.clicked(&format!("{r}/BackwardsButton")) {
                        out.push(Action::Target(game.cycle_target(t, true)));
                    } else if self.tsm.clicked(&format!("{l}/ForwardsButton")) {
                        out.push(Action::LeftTarget(game.cycle_left_target(t, false)));
                    } else if self.tsm.clicked(&format!("{l}/BackwardsButton")) {
                        out.push(Action::LeftTarget(game.cycle_left_target(t, true)));
                    }
                }
            }
            // Banana Farm: a bank's Collect (its balance) and Collect All (every bank's), and the
            // IMF Loan's debt.
            if theme_id == "BananaFarm" {
                let g = format!("{THEME}/BananaFarm_BankButtonGroup");
                let bank = game.tier(t).is_some_and(|tr| tr.bank.is_some());
                self.tsm.set_active(&g, bank);
                self.tsm.set_active(&format!("{g}/BankDeposit"), false);
                if bank {
                    let all: f64 = game.towers.iter().map(|o| o.bank).sum();
                    self.tsm.set_text(&format!("{g}/BankCollect/Cost/Text"), &money(tw.bank.floor()));
                    self.tsm.set_text(&format!("{g}/BankCollectAll/Cost/Text"), &money(all.floor()));
                    if self.tsm.clicked(&format!("{g}/BankCollect/BankCollectBtn")) {
                        out.push(Action::BankCollect(false));
                    } else if self.tsm.clicked(&format!("{g}/BankCollectAll/BankCollectAllBtn")) {
                        out.push(Action::BankCollect(true));
                    }
                }
                self.tsm.set_active(&format!("{THEME}/BananaFarm_Debt"), game.debt > 0.0);
                self.tsm.set_text(&format!("{THEME}/BananaFarm_Debt/Text"), &format!("-{}", money(game.debt.ceil())));
            }
            // Monkey Sub: submerge / surface, for tiers that can (SubmergeModel).
            let sub = format!("{THEME}/MonkeySub_SubmergeButton");
            if theme_id == "MonkeySub" {
                let can = game.can_submerge(t);
                self.tsm.set_active(&sub, can);
                self.tsm.set_active(&format!("{sub}/SubmergeIcon"), !tw.submerged);
                self.tsm.set_active(&format!("{sub}/UnsubmergeIcon"), tw.submerged);
                if can && self.tsm.clicked(&sub) {
                    out.push(Action::Submerge);
                }
            }
            // Necromancer (UnpoppedArmy): the graveyard's stored RBE.
            if theme_id == "UnpoppedArmy" {
                let stored: f32 = tw.grave.iter().map(|g| g.1).sum();
                let c = format!("{THEME}/UnpoppedArmy_TSMCounterContainer");
                self.tsm.set_text(&format!("{c}/Graveyard/GraveyardText"), &format!("{}", stored.floor()));
                self.tsm.set_text(&format!("{c}/Damage/DamageText"), &tw.pops.to_string());
                self.tsm.set_active(&format!("{THEME}/UnpoppedArmy_ButtonGroup/TSMActionButton"), false);
                self.tsm.set_active(&format!("{THEME}/TSMPopInfoAll"), false);
            } else {
                self.tsm.set_active(&format!("{THEME}/TSMPopInfoAll"), true);
            }
            // Some themes keep the camo toggle in their button group.
            let camo = [format!("{THEME}/{theme_id}_CamoTargetPriorityToggle"), format!("{THEME}/{theme_id}_ButtonGroup/CamoTargetPriorityToggle")]
                .into_iter()
                .find(|p| self.tsm.find(p).is_some_and(|n| n.active))
                .unwrap_or_default();
            if !camo.is_empty() {
                self.tsm.set_active(&format!("{camo}/Tick"), tw.camo_prio);
                if self.tsm.clicked(&camo) {
                    out.push(Action::CamoPrio);
                }
            }
            // Portrait: the highest path's tier art (MonkeyPortraits/<tower>/<key>-<Tower>.png).
            let (p, &tier) = tw.tiers.iter().enumerate().max_by_key(|&(i, v)| (*v, std::cmp::Reverse(i))).unwrap();
            let mut key = [0u8; 3];
            key[p] = tier;
            let key = format!("{}{}{}", key[0], key[1], key[2]);
            // File names vary in case ("000-DartMonkey", "030-Dartmonkey").
            // Heroes: <Hero>Portrait, then the highest <Hero>PortraitLvl<N> reached (the levels
            // with art differ per hero: Quincy 3/7/10/20, Rosalia 2/5/7/13/20...).
            let hero_portrait = game.hero_def(tw.def).map(|_| {
                let lvl = tw.tiers[0];
                (1..=lvl)
                    .rev()
                    .map(|l| format!("{name}PortraitLvl{l}"))
                    .find(|n| self.sprites.get(n).is_some())
                    .unwrap_or_else(|| format!("{name}Portrait"))
            });
            // Paragon portrait: the tower's one "Paragon-*" portrait (named after the Paragon,
            // sometimes misspelt: "Paragon-NauticSeigeCore").
            let paragon_portrait = tw.paragon.as_ref().zip(pdef.as_ref()).and_then(|_| self.paragon_portraits.get(name.as_str()).cloned());
            let portrait = paragon_portrait
                .into_iter()
                .chain(hero_portrait)
                .chain([format!("{key}-{name}"), format!("{key}-{}{}", &name[..1], name[1..].to_lowercase())])
                .find(|n| self.sprites.get(n).is_some());
            self.tsm.set_sprite(&format!("{THEME}/MonkeyImage"), portrait.as_deref());
            // The portrait's frame follows the tower set (PortraitContainer<Set>), purple for a
            // Paragon.
            let set = game.shop.iter().find(|it| it.id == *name).map_or(1, |it| it.set);
            let frame = match (tw.paragon.is_some(), game.hero_def(tw.def).is_some(), set) {
                (true, _, _) => "PortraitContainerParagon",
                (_, true, _) => "PortraitContainerHero",
                (_, _, 2) => "PortraitContainerMilitary",
                (_, _, 4) => "PortraitContainerMagic",
                (_, _, 8) => "PortraitContainerSupport",
                _ => "PortraitContainerPrimary",
            };
            self.tsm.set_sprite(&format!("{THEME}/BackgroundImage"), Some(frame));
            self.tsm.set_active(&format!("{THEME}/MonkeyImage"), portrait.is_some());
            if let Some(n) = self.tsm.find_mut(&format!("{THEME}/MonkeyImage")).and_then(|n| n.img.as_mut()) {
                n.preserve = true;
            }

            // Heroes: the menu's HeroDetails panel (level badge, XP bar, this / next level
            // text, Unlock Next Level) replaces the three upgrade paths.
            let hero = game.hero_def(tw.def).is_some();
            // A Paragon: ParagonDetails (its portrait frame, icon, name, description, degree).
            let paragon = tw.paragon.as_ref().map(|p| p.0);
            self.tsm.set_active(&format!("{TSM_OPTS}/TowerDetails"), !hero && paragon.is_none());
            self.tsm.set_active(&format!("{TSM_OPTS}/HeroDetails"), hero);
            self.tsm.set_active(&format!("{TSM_OPTS}/ParagonDetails"), paragon.is_some());
            if let (Some(degree), Some(p)) = (paragon, pdef.as_ref()) {
                let pd = format!("{TSM_OPTS}/ParagonDetails");
                let title = self.menus.loc.get(&p.name).cloned().unwrap_or_else(|| p.name.clone());
                let desc = self.menus.description(&format!("{name} Paragon")).or_else(|| self.menus.description(&p.name)).unwrap_or("").to_string();
                let icon = self.icon(&p.name, name, 0, 5).or_else(|| self.icon(&title, name, 0, 5));
                self.tsm.set_sprite(&format!("{pd}/CurrentUpgrade/UpgradeImage"), icon.as_deref());
                if let Some(i) = self.tsm.find_mut(&format!("{pd}/CurrentUpgrade/UpgradeImage")).and_then(|n| n.img.as_mut()) {
                    i.preserve = true;
                }
                self.tsm.set_text(&format!("{pd}/CurrentUpgrade/UpgradeTitle"), &title);
                self.tsm.set_text(&format!("{pd}/CurrentUpgrade/UpgradeDescription"), &desc);
                self.tsm.set_text(&format!("{pd}/ParagonInfo/Degree/Count"), &degree.to_string());
            }
            if hero {
                let hd = format!("{TSM_OPTS}/HeroDetails");
                let tower = game.tower_defs[tw.def].0.clone();
                let level = tw.tiers[0] as usize;
                let (frac, next_cost) = match (game.hero_progress(t), game.next_upgrade(t, 0)) {
                    (Some((xp, need)), Some((_, c))) => ((xp / need.max(1.0)).clamp(0.0, 1.0), Some(c)),
                    _ => (1.0, None),
                };
                if let Some(bar) = self.tsm.find_mut(&format!("{hd}/CurrentUpgrade/UpgradeBar/Mask/Bar")) {
                    bar.rt.amin = vec2(0.0, bar.rt.amin.y);
                    bar.rt.amax = vec2(frac, bar.rt.amax.y);
                    bar.rt.size.x = 0.0;
                    bar.rt.pos.x = 0.0;
                }
                self.tsm.set_text(&format!("{hd}/CurrentUpgrade/UpgradeBar/Star/Text"), &level.to_string());
                self.tsm.set_text(&format!("{hd}/CurrentUpgrade/UpgradeTitle"), &format!("Level {level}"));
                let cur_desc = self.menus.description(&format!("{tower} Level {level}")).unwrap_or("").to_string();
                self.tsm.set_text(&format!("{hd}/CurrentUpgrade/UpgradeDescription"), &cur_desc);
                let has_next = next_cost.is_some();
                self.tsm.set_active(&format!("{hd}/NextUpgrade"), has_next);
                if has_next {
                    let nd = self.menus.description(&format!("{tower} Level {}", level + 1)).unwrap_or("").to_string();
                    self.tsm.set_text(&format!("{hd}/NextUpgrade/UpgradeTitle"), &format!("Level {}", level + 1));
                    self.tsm.set_text(&format!("{hd}/NextUpgrade/UpgradeDescription"), &nd);
                }
                // Hero boosters are a store item.
                self.tsm.set_active(&format!("{hd}/Buttons/HeroBoosterButton"), false);
                self.tsm.set_active(&format!("{hd}/Buttons/UnlockNextLevel"), has_next);
                if let Some(c) = next_cost {
                    self.tsm.set_text(&format!("{hd}/Buttons/UnlockNextLevel/Cost"), &money(c));
                    let afford = game.cash >= c;
                    // Can't afford: the locked text material, like the upgrade buttons.
                    let style = if afford { &self.green } else { &self.brown };
                    if let Some(tx) = self.tsm.find_mut(&format!("{hd}/Buttons/UnlockNextLevel/Cost")).and_then(|x| x.txt.as_mut()) {
                        tx.style = style.clone();
                    }
                    if afford && self.tsm.clicked(&format!("{hd}/Buttons/UnlockNextLevel")) {
                        out.push(Action::Upgrade(0));
                    }
                }
            }
            for path in 0..3 {
                if hero || paragon.is_some() {
                    break;
                }
                let u = format!("{TSM_OPTS}/TowerDetails/UpgradeObject_{}", path + 1);
                let cur = tw.tiers[path] as usize;
                let tt = game.tower_tiers[tw.def].as_ref();
                let name_at = |tier: usize| tt.and_then(|tt| tt.upgrades.iter().find(|u| u.path == path && u.tier == tier)).map(|u| u.name.clone());
                let owned = if cur > 0 { name_at(cur - 1) } else { None };
                let tower = game.tower_defs[tw.def].0.clone();
                let owned_icon = owned.as_deref().and_then(|n| self.icon(n, &tower, path, cur.saturating_sub(1)));
                self.tsm.set_active(&format!("{u}/PreviousUpgrade/UpgradeImage"), owned.is_some());
                self.tsm.set_active(&format!("{u}/PreviousUpgrade/UpgradeName"), owned.is_some());
                self.tsm.set_active(&format!("{u}/PreviousUpgrade/NotUpgradedText"), owned.is_none());
                self.tsm.set_sprite(&format!("{u}/PreviousUpgrade/UpgradeImage"), owned_icon.as_deref());
                // Upgrade icons aren't square; keep their shape like the game.
                if let Some(i) = self.tsm.find_mut(&format!("{u}/PreviousUpgrade/UpgradeImage")).and_then(|n| n.img.as_mut()) {
                    i.preserve = true;
                }
                let owned_label = owned.as_deref().map(|n| self.menus.loc.get(n).cloned().unwrap_or_else(|| n.to_string())).unwrap_or_default();
                self.tsm.set_text(&format!("{u}/PreviousUpgrade/UpgradeName"), &owned_label);

                let next = game.next_upgrade(t, path);
                let afford = next.as_ref().is_some_and(|(_, c)| game.cash >= *c);
                // Only an unused path that can't be started is "Path Closed"; a started path that
                // the crosspath rule caps (2-3-0's top path) shows "Max Upgrades", as in the game.
                let maxed = cur >= 5 || (cur > 0 && next.is_none());
                let n = format!("{u}/NextUpgrade");
                self.tsm.set_active(&n, maxed || next.is_some());
                self.tsm.set_active(&format!("{u}/ClosedPath"), !maxed && next.is_none());
                self.tsm.set_active(&format!("{n}/UpgradeIcon"), next.is_some());
                self.tsm.set_active(&format!("{n}/UpgradeName"), next.is_some());
                self.tsm.set_active(&format!("{n}/Cost"), next.is_some());
                self.tsm.set_active(&format!("{n}/LockedText"), maxed);
                self.tsm.set_text(&format!("{n}/LockedText"), "Max Upgrades");
                if let Some((nm, c)) = &next {
                    // Paragon icons are named after the Paragon ("Apex Plasma Master").
                    let icon = self.icon(nm, &tower, path, cur).or_else(|| self.menus.loc.get(nm).and_then(|l| self.icon(l, &tower, path, cur)));
                    self.tsm.set_sprite(&format!("{n}/UpgradeIcon"), icon.as_deref());
                    if let Some(i) = self.tsm.find_mut(&format!("{n}/UpgradeIcon")).and_then(|n| n.img.as_mut()) {
                        i.preserve = true;
                    }
                    let label = self.menus.loc.get(nm).cloned().unwrap_or_else(|| nm.clone());
                    self.tsm.set_text(&format!("{n}/UpgradeName"), &label);
                    self.tsm.set_text(&format!("{n}/Cost"), &money(*c));
                }
                // UpgradeButton: lockedMaterial (OutlineBrown) + the locked arrow sprite when the
                // upgrade can't be bought, unlockedMaterial (OutlineGreen) otherwise.
                let unlocked = afford && !maxed;
                // The Paragon (a tier-5 path's next upgrade): ParagonArrowBtn with the purple
                // UpgradeNameParagon / CostParagon texts.
                let is_paragon = cur >= 5 && next.is_some();
                self.tsm.set_active(&format!("{n}/UpgradeNameParagon"), is_paragon);
                self.tsm.set_active(&format!("{n}/CostParagon"), is_paragon);
                if is_paragon {
                    self.tsm.set_active(&format!("{n}/UpgradeName"), false);
                    self.tsm.set_active(&format!("{n}/Cost"), false);
                    self.tsm.set_active(&format!("{n}/LockedText"), false);
                    if let Some((nm, c)) = &next {
                        let label = self.menus.loc.get(nm).cloned().unwrap_or_else(|| nm.clone());
                        self.tsm.set_text(&format!("{n}/UpgradeNameParagon"), &label);
                        self.tsm.set_text(&format!("{n}/CostParagon"), &money(*c));
                    }
                }
                let arrow = match (is_paragon, unlocked) {
                    (true, _) => "ParagonArrowBtn",
                    (false, true) => "GreenArrowBtn",
                    (false, false) => "GreenArrowBtnLocked",
                };
                self.tsm.set_sprite(&n, Some(arrow));
                let style = if unlocked { &self.green } else { &self.brown };
                for p in ["UpgradeName", "Cost", "LockedText"] {
                    if let Some(t) = self.tsm.find_mut(&format!("{n}/{p}")).and_then(|x| x.txt.as_mut()) {
                        t.style = style.clone();
                    }
                }
                for k in 1..=5 {
                    self.tsm.set_active(&format!("{u}/TierIndicators/Tier {k}/TierOn"), k <= cur);
                    self.tsm.set_active(&format!("{u}/TierIndicators/Tier {k}/TierOff"), k > cur);
                }
                if afford && self.tsm.clicked(&n) {
                    out.push(Action::Upgrade(path));
                }
            }
            let sell = (tw.invested * game.sell_fraction(t)).floor();
            self.tsm.set_text("Ratio/Scaler/TowerElements/SelectedTowerWorth/SellTxt", &money(sell));
            if self.tsm.clicked("Ratio/Scaler/TowerElements/SelectedTowerWorth/Sell") {
                out.push(Action::Sell);
            }
            if self.tsm.clicked(&format!("{THEME}/TargetType/ForwardsButton")) {
                out.push(Action::Target(game.cycle_target(t, false)));
            }
            if self.tsm.clicked(&format!("{THEME}/TargetType/BackwardsButton")) {
                out.push(Action::Target(game.cycle_target(t, true)));
            }
            if self.tsm.clicked(&format!("{THEME}/CloseButton")) {
                out.push(Action::Close);
            }
            if self.tsm.clicked(&format!("{THEME}/InfoToggleButton")) {
                self.info_open = !self.info_open;
            }
            self.tsm.set_active(INFO, self.info_open);
            if self.info_open {
                // InfoPanel sits beside the menu, on the map side.
                if let Some(p) = self.tsm.find_mut(INFO) {
                    let w = p.rt.size.x;
                    p.rt.amin.x = if open_right { 0.0 } else { 1.0 };
                    p.rt.amax.x = p.rt.amin.x;
                    p.rt.pos.x = if open_right { -w / 2.0 } else { w / 2.0 };
                }
                // STAND-IN icons for Round Placed and the cash rows (the game sets them in code
                // not yet read).
                let sell = (tw.invested * game.sell_fraction(t)).floor();
                let rows: [(&str, &str, String); ADV_STATS] = [
                    ("DamageIcon", "Damage", tw.damage.to_string()),
                    ("PopIcon", "Pops", tw.pops.to_string()),
                    ("LivesIcon", "RoundPlaced", tw.placed_round.to_string()),
                    ("CoinIcon", "CurrentValue", money(sell)),
                    ("CoinIcon", "CashSpent", money(tw.invested)),
                    ("CoinGainIcon", "CashEarnt", money(tw.cash_earned.floor())),
                    ("LivesIcon", "LivesGained", format!("{}", tw.lives_gained.floor())),
                ];
                for (i, (icon, key, value)) in rows.iter().enumerate() {
                    let stat = format!("{INFO}/AdvancedStats/Anim/Viewport/Content/Stat{i}");
                    let label = self.menus.loc.get(*key).cloned().unwrap_or_else(|| crate::spaced(key));
                    self.tsm.set_sprite(&format!("{stat}/Icon"), Some(icon));
                    self.tsm.set_text(&format!("{stat}/Name"), &label);
                    self.tsm.set_text(&format!("{stat}/Count"), value);
                    // SetAdvStatsToggleMethods: Damage and Pops have the eye button, lit for the
                    // one the panel shows.
                    let eye = i < 2 && !peaceful;
                    self.tsm.set_active(&format!("{stat}/VisibilityButton"), eye);
                    if eye {
                        let on = (i == 1) == self.show_pops;
                        self.tsm.set_active(&format!("{stat}/VisibilityButton/On"), on);
                        self.tsm.set_active(&format!("{stat}/VisibilityButton/Off"), !on);
                        if self.tsm.clicked(&format!("{stat}/VisibilityButton")) && !on {
                            self.show_pops = i == 1;
                            out.push(Action::ShowPops(self.show_pops));
                        }
                    }
                }
                for path in 0..3 {
                    let row = format!("{INFO}/Upgrade_{}/InfoContainer/NextUpgrade", path + 1);
                    let next = game.next_upgrade(t, path).map(|(nm, _)| nm);
                    let desc = next.as_deref().and_then(|nm| self.menus.description(nm)).map(str::to_string);
                    self.tsm.set_active(&row, desc.is_some());
                    if let (Some(nm), Some(d)) = (next, desc) {
                        self.tsm.set_text(&format!("{row}/Title"), &nm);
                        self.tsm.set_text(&format!("{row}/Description"), &d);
                    }
                }
            }
        }
        let _ = auto_start;

        let mut ctx = Ctx { sprites: &mut self.sprites, fonts: &self.fonts, scale: s, canvas_h: canvas.y, clip: None };
        if self.tsm.active {
            draw_tree(&mut self.tsm, map, &mut ctx);
        }
        draw_tree(&mut self.left, map, &mut ctx);
        draw_tree(&mut self.right, map, &mut ctx);
        if let Some(race)=&mut self.race_ui{draw_tree(race,map,&mut ctx);}
        draw_tree(&mut self.shop, full, &mut ctx);
        self.draw_geraldo(game, selected, canvas, SHOP_W, s, &mut out);
        self.draw_sandbox(game, full, canvas.y, s, &mut out);
        self.draw_abilities(game, &mut out);
        self.draw_temple(full, canvas.y, s, &mut out);
        // IMF Loan: what's still owed, in red under the cash.
        if game.debt > 0.0 {
            if let Some(r) = self.left.rect(&format!("{lg}/CashGroup/Cash")) {
                self.fonts.draw("Btd6FontTitleSDF", &TextBox { gradient: None,
                    text: &format!("-{}", money(game.debt.ceil())),
                    rect: Rect::new(r.x, r.bottom(), r.w, r.h * 0.7),
                    size: r.h * 0.55,
                    color: Color::from_rgba(255, 70, 60, 255),
                    h_align: 1,
                    v_align: 512,
                    style: &self.green,
                    bold: false,
                    spacing: 0.0,
                });
            }
        }
        out
    }
}
