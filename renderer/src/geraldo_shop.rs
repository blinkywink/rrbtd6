//! Geraldo's shop panel (GeraldoShop prefab, GeraldoShopMenuUi): while Geraldo is selected it
//! opens on the map's side away from his upgrade menu. One ShopItem card per unlocked item with
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
/// The panel's width on the canvas (the prefab stretches 50 units narrower than its parent).
const PANEL_W: f32 = 1000.0;

pub struct GeraldoShop {
    node: Option<Node>,
    card: Option<Node>,
    /// Item shown in the info panel.
    selected: Option<usize>,
    /// Item in hand: the next map click uses it (set back by main.rs).
    pub armed: Option<usize>,
    /// Item indices of the cards, in order.
    shown: Vec<usize>,
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
        Self { node, card, selected: None, armed: None, shown: Vec::new() }
    }
}

impl Hud {
    /// The panel covers this screen point (map clicks there are the shop's).
    pub fn geraldo_blocks(&self, p: Vec2) -> bool {
        self.geraldo.node.as_ref().is_some_and(|n| n.active && n.rect(ROOT).is_some_and(|r| r.contains(p)))
    }

    pub(crate) fn draw_geraldo(&mut self, game: &Game, selected: Option<usize>, canvas: Vec2, shop_w: f32, s: f32, out: &mut Vec<Action>) {
        let open = selected.filter(|&t| game.towers.get(t).is_some_and(|tw| tw.sub.is_none() && game.tower_defs[tw.def].0 == "Geraldo"));
        let Some(n) = self.geraldo.node.as_mut() else { return };
        n.active = open.is_some();
        let Some(g) = open else {
            self.geraldo.armed = None;
            return;
        };
        // Opposite the upgrade menu (which opens right of a tower on the map's left half).
        let open_right = game.towers[g].pos[0] < (game.bounds.0[0] + game.bounds.1[0]) / 2.0;
        let x = if open_right { 0.0 } else { canvas.x - shop_w - PANEL_W };
        let parent = R { min: vec2(x, 0.0), size: vec2(PANEL_W, canvas.y) };

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

        // Cards for the unlocked items (rebuilt when that changes).
        let items = game.geraldo_items();
        let shown: Vec<usize> = (0..items.len()).filter(|&i| game.item_unlocked(i)).collect();
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
        }
        for &i in &shown {
            let it = &items[i];
            let c = format!("{GRID}/Item{i}/Available");
            let (stock, ready, supported) = (game.item_stock(i), game.item_ready(i), game.item_supported(i));
            let cost = game.item_cost(i);
            // The upgraded look ("...Plus") once the item's last level description applies.
            let plus = format!("{}Plus", it.name);
            let upgraded = level_desc(loc, &it.loc, level).1 > 0 && self.sprites.get(&plus).is_some();
            n.set_sprite(&format!("{c}/StoreIcon"), Some(if upgraded { &plus } else { &it.name }));
            n.set_img_gray(&format!("{c}/StoreIcon"), !supported || stock == 0);
            n.set_text(&format!("{c}/Quantity"), &format!("{stock}/{}", it.max));
            if let Some(img) = n.find_mut(&format!("{c}/Fillbar/Fill")).and_then(|f| f.img.as_mut()) {
                img.fill_method = 0;
                img.fill_amount = if it.max > 0 { stock as f32 / it.max as f32 } else { 0.0 };
            }
            n.set_text(&format!("{c}/CostBanner/Cost"), &money(cost));
            n.set_text_color(&format!("{c}/CostBanner/Cost"), if game.cash >= cost { WHITE } else { Color::from_rgba(255, 60, 40, 255) });
            n.set_active(&format!("{c}/AvailableGlow"), ready);
            n.set_active(&format!("{c}/Strikethrough"), stock == 0);
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
        if n.clicked(&format!("{TOP}/GamepadCloseContainer/CloseButton")) {
            out.push(Action::Close);
        }
        let mut ctx = Ctx { sprites: &mut self.sprites, fonts: &self.fonts, scale: s, canvas_h: canvas.y, clip: None };
        draw_tree(n, parent, &mut ctx);
        // The item in hand follows the cursor.
        if let Some(i) = self.geraldo.armed {
            let name = items[i].name.clone();
            if let Some(sp) = self.sprites.get(&name) {
                let (mx, my) = mouse_position();
                let size = 180.0 * s;
                draw_texture_ex(&sp.tex, mx - size / 2.0, my - size / 2.0, Color::new(1.0, 1.0, 1.0, 0.85),
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
