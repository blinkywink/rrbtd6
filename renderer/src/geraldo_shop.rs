//! Geraldo's shop panel (GeraldoShop prefab, GeraldoShopMenuUi): TowerSelectionMenu's secondary
//! menu, shown over the upgrade menu in its GeraldoShopContainer. One ShopItem card per item with
//! its icon, stock ("2/4" and the stock bar) and price; a click selects it (its name and level
//! description in the info panel) and, when it can be bought, takes it in hand: the next map
//! click uses it there (main.rs). Items that need no spot (Rejuv Potion) are used at once.

use crate::game::Game;
use crate::hud::{money, Action, Hud};
use crate::uitree::{draw_tree, Ctx, Node, R};
use macroquad::prelude::*;
use std::path::Path;

const ROOT: &str = "Animated";
const GRID: &str = "Animated/Shop/Viewport/ItemGroup";
const INFO: &str = "Animated/Info";
const TOP: &str = "Animated/Top";
/// TowerSelectionMenu's GeraldoShopContainer stretches over TowerElements.
const CONTAINER: &str = "Ratio/Scaler/TowerElements";
/// The item card's root in the runtime clips' paths.
const CARD_PATH: &str = "GeraldoShop/Animated/Shop/Viewport/ItemGroup/ShopItem/";
/// GeraldoShopMenuItemUi.UpdateButtonState's states (VERIFIED-DECOMP @0x180b7a850), as the
/// GeraldoShopItemStates clips: 0 below the item's level, 1 buyable, 2 short of cash, 3 out of
/// stock or Geraldo stunned, 4 purchase limit hit or tower restricted, 5 mode/round disabled.
const ITEM_CLIPS: [&str; 6] = ["GeraldoShopItemLocked", "GeraldoShopItemAvailable", "GeraldoShopItemCantAfford",
    "GeraldoShopItemUnavailable", "GeraldoShopItemMaxUsed", "GeraldoShopItemRestricted"];

pub struct GeraldoShop {
    node: Option<Node>,
    card: Option<Node>,
    /// Item shown in the info panel.
    selected: Option<usize>,
    /// Item in hand: the next map click uses it (set back by main.rs).
    pub armed: Option<usize>,
    /// Whether the item in hand can be used under the cursor (main.rs: Game::item_spot_ok).
    pub armed_ok: bool,
    /// Item indices of the cards, in order.
    shown: Vec<usize>,
    /// TowerSelectionMenu's secondary menu shown (TSMThemeGeraldo.OnButtonPress, "ShopButton":
    /// ShowSecondarySelectionMenu / its hide), for the Geraldo tower `open_for`.
    pub open: bool,
    pub open_for: Option<u64>,
    /// When `open` last changed (the GeraldoShopOpen / GeraldoShopClose clip is playing).
    pub changed: Option<f64>,
    /// GeraldoShop.json's runtime clips (paths from the prefab root), and the item states' clips
    /// with paths from the card.
    runtime: serde_json::Value,
    item_runtime: serde_json::Value,
    /// Each card's state and when it entered it (the Available clip's glow plays on entry).
    states: Vec<(u8, f64)>,
}

impl GeraldoShop {
    pub fn load(prefabs: &Path) -> Self {
        let mut node = Node::load(&prefabs.join("GeraldoShop.json"));
        let card = node.as_mut().and_then(|n| {
            let g = n.find_mut(GRID)?;
            let c = g.children.first().cloned();
            g.children.clear();
            c
        });
        let mut runtime: serde_json::Value = std::fs::read_to_string(prefabs.join("../runtime/GeraldoShop.json")).ok()
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let mut item_runtime = runtime.clone();
        for (data, prefix) in [(&mut runtime, "GeraldoShop/"), (&mut item_runtime, CARD_PATH)] {
            for animator in data["animators"].as_array_mut().into_iter().flatten() {
                for clip in animator["clips"].as_array_mut().into_iter().flatten() {
                    for binding in clip["bindings"].as_array_mut().into_iter().flatten() {
                        if let Some(path) = binding["path"].as_str().and_then(|p| p.strip_prefix(prefix)) {
                            binding["path"] = path.to_owned().into();
                        }
                    }
                }
            }
        }
        if let Some(n) = &mut node {
            crate::frontend::apply_animation(n, &runtime, "GeraldoShop", "GeraldoShopIdle", None);
        }
        Self { node, card, selected: None, armed: None, armed_ok: false, shown: Vec::new(), open: false, open_for: None,
            changed: None, runtime, item_runtime, states: Vec::new() }
    }

    /// Shows / hides the shop (ShowSecondarySelectionMenu / HideSecondarySelectionMenu).
    pub fn set_open(&mut self, open: bool) {
        if open != self.open {
            self.open = open;
            self.changed = Some(get_time());
        }
    }

    /// The shop covers the upgrade menu now (its buttons don't take clicks).
    pub fn covers_tsm(&self) -> bool {
        self.open && self.open_for.is_some()
    }
}

impl Hud {
    /// The panel covers this screen point (map clicks there are the shop's).
    pub fn geraldo_blocks(&self, p: Vec2) -> bool {
        self.geraldo.node.as_ref().is_some_and(|n| n.active && n.rect(ROOT).is_some_and(|r| r.contains(p)))
    }

    pub(crate) fn draw_geraldo(&mut self, game: &Game, selected: Option<usize>, canvas_h: f32, s: f32, out: &mut Vec<Action>) {
        // (Only while the theme's Shop button has it shown, for the selected Geraldo: VERIFIED-DECOMP
        // TSMThemeGeraldo.OnButtonPress @0x180b91720; another selection starts with it hidden.)
        let gid = selected.and_then(|t| game.towers.get(t)).filter(|tw| tw.sub.is_none() && game.tower_defs[tw.def].0 == "Geraldo").map(|tw| tw.id);
        if gid != self.geraldo.open_for {
            self.geraldo.open = false;
            self.geraldo.changed = None;
            self.geraldo.open_for = gid;
        }
        // TowerSelectionMenu's GeraldoShopContainer (the upgrade menu's rect, drawn this frame).
        let container = self.tsm.find(CONTAINER).filter(|_| self.tsm.active).map(|c| c.screen);
        let shown = selected.filter(|_| gid.is_some() && (self.geraldo.open || self.geraldo.changed.is_some()));
        let Some(n) = self.geraldo.node.as_mut() else { return };
        let (Some(g), Some(c)) = (shown, container) else {
            n.active = false;
            self.geraldo.armed = None;
            return;
        };
        n.active = true;
        let parent = R { min: vec2(c.x / s, canvas_h - (c.y + c.h) / s), size: vec2(c.w / s, c.h / s) };
        // The menu Animator's VisibleState: GeraldoShopOpen / GeraldoShopClose.
        let clip = if self.geraldo.open { "GeraldoShopOpen" } else { "GeraldoShopClose" };
        let elapsed = self.geraldo.changed.map(|t| (get_time() - t) as f32);
        if crate::frontend::apply_animation(n, &self.geraldo.runtime, "GeraldoShop", clip, elapsed) {
            self.geraldo.changed = None;
        }
        if !self.geraldo.open && self.geraldo.changed.is_none() {
            n.active = false;
            self.geraldo.armed = None;
            return;
        }

        let loc = &self.menus.loc;
        let text = |k: &str| loc.get(k).cloned();
        let level = game.geraldo_level();
        n.set_text(&format!("{TOP}/ShopTitle"), &text("Geraldo's Shop").unwrap_or_else(|| "Geraldo's Shop".into()));
        n.set_text(&format!("{TOP}/UpgradeText"), &format!("Level {level}"));
        let frac = game.hero_progress(g).map_or(1.0, |(left, cost)| if cost > 0.0 { 1.0 - left / cost } else { 1.0 });
        if let Some(img) = n.find_mut(&format!("{TOP}/UpgradeBar/Bar")).and_then(|b| b.img.as_mut()) {
            img.fill_method = 0;
            img.fill_amount = frac.clamp(0.0, 1.0);
        }
        n.set_active(&format!("{TOP}/GamepadCloseContainer/CloseButton/Jewel"), false);

        // A card per item (GeraldoShopMenuUi.Populate, VERIFIED-DECOMP @0x180b7c3c0: every
        // GeraldoItemModel; the ones above Geraldo's level show Locked).
        let items = game.geraldo_items();
        let shown: Vec<usize> = (0..items.len()).collect();
        if shown != self.geraldo.shown {
            if let (Some(grid), Some(card)) = (n.find_mut(GRID), self.geraldo.card.as_ref()) {
                grid.children = shown
                    .iter()
                    .map(|&i| {
                        let mut c = card.clone();
                        c.name = format!("Item{i}");
                        c.active = true;
                        c
                    })
                    .collect();
            }
            self.geraldo.shown = shown.clone();
            self.geraldo.states = vec![(u8::MAX, 0.0); items.len()];
        }
        let now = get_time();
        // (Tower stuns by bloons are not modelled: TowerToSimulation.IsStunned is never set.)
        let stunned = false;
        let _ = g;
        for &i in &shown {
            let it = &items[i];
            let c = format!("{GRID}/Item{i}/Available");
            let (stock, ready, supported) = (game.item_stock(i), game.item_ready(i), game.item_supported(i));
            let cost = game.item_cost(i);
            // The upgraded look ("...Plus") once the item's last level description applies.
            let plus = format!("{}Plus", it.name);
            let upgraded = level_desc(loc, &it.loc, level).1 > 0 && self.sprites.get(&plus).is_some();
            n.set_sprite(&format!("{c}/StoreIcon"), Some(if upgraded { &plus } else { &it.name }));
            n.set_text(&format!("{c}/Quantity"), &format!("{stock}/{}", it.max));
            if let Some(img) = n.find_mut(&format!("{c}/Fillbar/Fill")).and_then(|f| f.img.as_mut()) {
                img.fill_method = 0;
                img.fill_amount = if it.max > 0 { stock as f32 / it.max as f32 } else { 0.0 };
            }
            n.set_text(&format!("{c}/CostBanner/Cost"), &money(cost));
            let state: u8 = if level < it.unlock {
                0
            } else if !game.item_unlocked(i) {
                5
            } else if game.item_restricted(i) {
                4
            } else if stock == 0 || stunned || !supported {
                3
            } else if game.cash < cost {
                2
            } else {
                1
            };
            let st = &mut self.geraldo.states[i];
            if st.0 != state {
                *st = (state, now);
            }
            let elapsed = Some((now - st.1) as f32);
            if let Some(card) = n.find_mut(&format!("{GRID}/Item{i}")) {
                crate::frontend::apply_animation(card, &self.geraldo.item_runtime, "GeraldoShopItemStates", ITEM_CLIPS[state as usize], elapsed);
            }
            n.set_active(&format!("{c}/Selected"), self.geraldo.selected == Some(i) || self.geraldo.armed == Some(i));
            if n.clicked(&format!("{GRID}/Item{i}")) {
                self.geraldo.selected = Some(i);
                if ready {
                    out.push(Action::GeraldoItem(i));
                }
            }
        }
        // Info: the selected item's name and the description for Geraldo's level.
        let sel = self.geraldo.selected.filter(|i| shown.contains(i));
        n.set_active(INFO, sel.is_some());
        if let Some(i) = sel {
            let it = &items[i];
            n.set_text(&format!("{INFO}/ShopItemName"), &text(&format!("{} name", it.loc)).unwrap_or_else(|| crate::spaced(&it.name)));
            let mut d = level_desc(loc, &it.loc, level).0;
            if !game.item_supported(i) {
                d.push_str("\n(Not available in this version yet.)");
            }
            n.set_text(&format!("{INFO}/ShopItemDescription"), &d);
        }
        // (GeraldoShopMenuUi.Close: the shop alone hides; Geraldo stays selected.)
        if self.geraldo.open && n.clicked(&format!("{TOP}/GamepadCloseContainer/CloseButton")) {
            self.geraldo.open = false;
            self.geraldo.changed = Some(get_time());
        }
        let mut ctx = Ctx { sprites: &mut self.sprites, fonts: &self.fonts, scale: s, canvas_h, clip: None };
        draw_tree(n, parent, &mut ctx);
        // The item in hand follows the cursor.
        if let Some(i) = self.geraldo.armed {
            let name = items[i].name.clone();
            if let Some(sp) = self.sprites.get(&name) {
                let (mx, my) = mouse_position();
                let size = 180.0 * s;
                // InGameObjects.IconUpdate (VERIFIED-DECOMP @0x1815b7330): the powerIcon image's colour
                // is (1, 1, 1, 1) where the item can go, (1, 0, 0, 1) where it can't.
                let tint = if self.geraldo.armed_ok { WHITE } else { Color::new(1.0, 0.0, 0.0, 1.0) };
                draw_texture_ex(&sp.tex, mx - size / 2.0, my - size / 2.0, tint,
                    DrawTextureParams { dest_size: Some(vec2(size, size)), ..Default::default() });
            }
        }
    }
}

/// "<loc> description level N" for the highest N reached (else "<loc> description"), and N.
fn level_desc(loc: &std::collections::HashMap<String, String>, base: &str, level: u32) -> (String, u32) {
    (1..=level)
        .rev()
        .find_map(|l| loc.get(&format!("{base} description level {l}")).map(|d| (d.clone(), l)))
        .or_else(|| loc.get(&format!("{base} description")).map(|d| (d.clone(), 0)))
        .unwrap_or_default()
}
