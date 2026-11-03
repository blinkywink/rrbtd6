//! The in-game Powers menu: the game's PowersMenu prefab (RightMenu.Powers.PowersMenu /
//! BasePowersMenu / StandardPowerButton), opened by the shop's PowersButton. Using a Power
//! (BasePowersMenu.StartPowerPlacement) isn't built yet: a press on one does nothing.
//! Insta Monkeys don't exist in the clone, so their parts of the menu stay off.

use crate::hud::Hud;
use crate::uitree::{draw_tree, Ctx, Node, R};
use macroquad::prelude::*;
use serde_json::Value;
use std::path::{Path, PathBuf};

const UI: &str = "UIPowers";
const SORTER: &str = "UIPowers/Mask/Sorter";
/// StandardPowerButton.lockedColor (the prefab's value): the icon of a Power that can't be used.
const LOCKED: Color = Color::new(0.6886792, 0.6886792, 0.6886792, 0.47058824);

struct Item {
    id: String,
    /// GameModel.GetModdedPowers cost (store-only Powers: the clone's trophy price, as the
    /// Powers screen; DEVIATIONS.md), for SkuSettings.CanShowPower.
    cost: i64,
    store: bool,
    between_rounds: bool,
    /// Its game/powers.json behaviours (what using it does).
    behaviors: Value,
}

pub struct PowersMenu {
    node: Option<Node>,
    /// The data root (the profile's held Powers).
    root: PathBuf,
    runtime: Value,
    items: Vec<Item>,
    /// BasePowersMenu.open, and when it last opened or closed (the slide clip's start).
    pub open: bool,
    changed: Option<f64>,
    /// The bottom-bar (4:3) layout's clips were used for the last change.
    wide: bool,
    /// Sorter scroll (canvas units).
    scroll: f32,
    /// The Power in hand (main.rs), shown selected.
    pub armed: Option<String>,
}

impl PowersMenu {
    pub fn load(prefabs: &Path, root: &Path) -> Self {
        let mut runtime: Value = std::fs::read_to_string(prefabs.join("../runtime/PowersMenu.json")).ok()
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        // The clips name paths from the prefab root.
        for animator in runtime["animators"].as_array_mut().into_iter().flatten() {
            for clip in animator["clips"].as_array_mut().into_iter().flatten() {
                for binding in clip["bindings"].as_array_mut().into_iter().flatten() {
                    if let Some(path) = binding["path"].as_str().and_then(|p| p.strip_prefix("PowersMenu/")) {
                        binding["path"] = path.to_owned().into();
                    }
                }
            }
        }
        let powers: Vec<Value> = std::fs::read_to_string(root.join("ui/runtime/powers.json")).ok()
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let rules: Value = std::fs::read_to_string(root.join("game/powers.json")).ok()
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let mut items = Vec::new();
        let node = Node::load(&prefabs.join("PowersMenu.json")).map(|mut n| {
            for p in ["InstaTowerGroupsMenu", "UIPowers/InstaTowersMenu", "UIPowers/ShowInstaMonkeysButton", "UIPowers/Toggles/InstasOnlyToggle", "UIPowers/UnusedPowers"] {
                n.set_active(p, false);
            }
            let Some(template) = n.find(&format!("{SORTER}/PowerPurchaseButton")).cloned() else { return n };
            let mut buttons = Vec::new();
            // PowersMenu.RebuildPowers: a button per available Power, in GameModel order.
            // HYPOTHESIS P-POWERORDER: the order of ui/runtime/powers.json (orderPriority, as the
            // Powers screen); RebuildPowers wraps GetAvailablePowers in a generic call not yet named.
            for p in &powers {
                let id = p["id"].as_str().unwrap_or("").to_owned();
                let store = p["store"].as_str().is_some_and(|s| !s.is_empty());
                let mut cost = p["cost"].as_i64().unwrap_or(0);
                if cost == 0 && store {
                    cost = 375;
                }
                // BasePowersMenu.CreateTowerButton (@0x180abfb00) skips isHidden Powers.
                if p["hidden"] == true {
                    continue;
                }
                let mut b = template.clone();
                b.name = format!("Power{}", items.len());
                b.set_sprite("Button/Icon", p["icon"].as_str().and_then(|s| s.strip_suffix(".png")));
                // StandardPowerButton.UpdateIcon (@0x180ae7110): bgPowerPro for a Pro Power, else
                // bgIAP when it has a storeNavigateToId, else bgPower.
                let pro = p["pro"] == true;
                b.set_active("Button/ProBg", pro);
                b.set_active("Button/StoreBg", !pro && store);
                b.set_active("Button/PowerBg", !pro && !store);
                b.set_active("Button/Selected", false);
                buttons.push(b);
                let between_rounds = rules[&id]["between_rounds"] == true;
                let behaviors = rules[&id]["behaviors"].clone();
                items.push(Item { id, cost, store, between_rounds, behaviors });
            }
            if let Some(sorter) = n.find_mut(SORTER) {
                sorter.children = buttons;
            }
            n
        });
        let mut menu = Self { node, root: root.to_owned(), runtime, items, open: false, changed: None, wide: false, scroll: 0.0, armed: None };
        if let Some(n) = &mut menu.node {
            crate::frontend::apply_animation(n, &menu.runtime, "PowersSlideOut", "PowersSlideOutIdle", None);
        }
        menu
    }

    /// A Power's behaviours (game/powers.json), for Game::use_power.
    pub fn behaviors(&self, id: &str) -> Option<&Value> {
        self.items.iter().find(|i| i.id == id).map(|i| &i.behaviors)
    }

    /// The point is on the open menu (its panel), so the shop and map under it don't take it.
    pub fn covers(&self, p: Vec2) -> bool {
        (self.open || self.changed.is_some())
            && self.node.as_ref().and_then(|n| n.find(UI)).is_some_and(|u| u.active && u.screen.contains(p))
    }
}

/// LocalizationManager.FormatInt with "N0": thousands separated.
fn n0(v: u64) -> String {
    let digits = v.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

impl Hud {
    /// The shop's PowersButton: BasePowersMenu.Open / Close.
    pub fn toggle_powers(&mut self) {
        self.powers.open = !self.powers.open;
        self.powers.changed = Some(get_time());
    }

    /// Draws the menu over the shop: its slide (PowersSlideOut), each button's count and state.
    pub fn draw_powers(&mut self, game: &crate::game::Game, full: R, canvas_h: f32, s: f32, bottom: bool, out: &mut Vec<crate::hud::Action>) {
        let menu = &mut self.powers;
        let Some(n) = &mut menu.node else { return };
        if menu.changed.is_some_and(|_| menu.open) {
            menu.wide = bottom;
        }
        // BasePowersMenu.Open (@0x180ac0460): the Wide clips below 14:9 (ScreenRatio 2).
        // HYPOTHESIS P-POWERSWIDE: ScreenRatio 2 is the clone's bottom-bar layout.
        let clip = match (menu.open, menu.wide) {
            (true, false) => "PowersSlideOutOpen",
            (false, false) => "PowersSlideOutClose",
            (true, true) => "PowersSlideOutWideOpen",
            (false, true) => "PowersSlideOutWideClose",
        };
        let elapsed = menu.changed.map(|t| (get_time() - t) as f32);
        if crate::frontend::apply_animation(n, &menu.runtime, "PowersSlideOut", clip, elapsed) {
            menu.changed = None;
        }
        if !menu.open && menu.changed.is_none() {
            return;
        }
        let held = crate::local_profile::get(&menu.root).powers;
        // PowerButton.ModeDisabled (@0x180ae0130): ReadonlyInGameData.ArePowersAllowed is false in
        // CHIMPS ("Clicks"). RoundDisabled (@0x180ae0420): a Power not usable between rounds while
        // no round runs.
        let mode_off = game.mode_name == "Clicks";
        for (i, item) in menu.items.iter().enumerate() {
            let b = format!("{SORTER}/Power{i}");
            // SkuSettings.CanShowPower, as the Powers screen (menu_pages.rs bind_powers).
            let count = held.get(&item.id).copied().unwrap_or(0);
            let show = item.cost > 0 || !item.store || count > 0;
            n.set_active(&b, show);
            if !show {
                continue;
            }
            // StandardPowerButton.UpdatePowerDisplay (@0x180ae7280): the count badge when there
            // is one ("∞" in sandbox); the icon white when it can be used, else lockedColor.
            // HYPOTHESIS P-SANDBOXCOUNT: the sandbox count is the profile's too.
            n.set_active(&format!("{b}/Button/PowerCountIcon"), count > 0);
            n.set_text(&format!("{b}/Button/PowerCountIcon/Text"), &if game.sandbox { "∞".to_owned() } else { n0(count) });
            let usable = count > 0 && !mode_off && (item.between_rounds || game.round_active);
            n.set_img_color(&format!("{b}/Button/Icon"), if usable { WHITE } else { LOCKED });
            // BasePowersMenu.Select: the picked Power's selectedHighlight. Only Powers the clone
            // can use yet are taken (powers.rs).
            n.set_active(&format!("{b}/Button/Selected"), menu.armed.as_deref() == Some(item.id.as_str()));
            if usable && menu.open && game.power_usable(&item.id, &item.behaviors) && n.clicked(&format!("{b}/Button")) {
                out.push(crate::hud::Action::Power(item.id.clone()));
            }
        }
        // ScrollRectWithDragCancel: the wheel scrolls the grid inside its Mask.
        let m: Vec2 = mouse_position().into();
        if let (Some(mask), Some(sorter)) = (n.find(&format!("{UI}/Mask")), n.find(SORTER)) {
            let max = (sorter.rt.size.y - mask.screen.h / s).max(0.0);
            if mask.screen.contains(m) {
                menu.scroll -= crate::hud::wheel_notches() * crate::hud::WHEEL_STEP;
            }
            menu.scroll = menu.scroll.clamp(0.0, max);
        }
        if let Some(sorter) = n.find_mut(SORTER) {
            sorter.rt.pos.y = menu.scroll;
        }
        let mut ctx = Ctx { sprites: &mut self.sprites, fonts: &self.fonts, scale: s, canvas_h, clip: None };
        draw_tree(n, full, &mut ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The local export (skipped without one): every Power but the hidden ones gets a button,
    /// the menu starts idle off to the right and the Open clip ends at x -312.
    #[test]
    fn lists_powers_and_slides() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_extracted");
        if !root.join("ui/prefabs/PowersMenu.json").exists() || !root.join("ui/runtime/PowersMenu.json").exists() {
            return;
        }
        let mut menu = PowersMenu::load(&root.join("ui/prefabs"), &root);
        let powers: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(root.join("ui/runtime/powers.json")).unwrap()).unwrap();
        assert_eq!(menu.items.len(), powers.iter().filter(|p| p["hidden"] != true).count());
        // Every listed Power finds its rules (game/powers.json is keyed by the same ids).
        let rules: Value = serde_json::from_str(&std::fs::read_to_string(root.join("game/powers.json")).unwrap()).unwrap();
        for item in &menu.items {
            assert!(rules.get(&item.id).is_some(), "{} has no rules", item.id);
        }
        let n = menu.node.as_mut().unwrap();
        assert_eq!(n.find(SORTER).unwrap().children.len(), menu.items.len());
        let ui = n.find(UI).unwrap();
        assert!(!ui.active);
        assert_eq!(ui.rt.pos.x, 1000.0);
        assert!(crate::frontend::apply_animation(n, &menu.runtime, "PowersSlideOut", "PowersSlideOutOpen", Some(1.0)));
        let ui = n.find(UI).unwrap();
        assert!(ui.active);
        assert!((ui.rt.pos.x + 312.0).abs() < 0.01, "{}", ui.rt.pos.x);
        // A Pro Power shows the Pro background only.
        let pro = powers.iter().filter(|p| p["hidden"] != true).position(|p| p["pro"] == true).unwrap();
        let b = n.find(&format!("{SORTER}/Power{pro}/Button")).unwrap();
        assert!(b.find("ProBg").unwrap().active && !b.find("PowerBg").unwrap().active && !b.find("StoreBg").unwrap().active);
    }
}
