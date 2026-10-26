//! The Sun Temple / True Sun God confirmation (CommonPopup with the "Sun Temple Title" /
//! "Sun Temple Body" or "True Sun Temple Title" / "True Sun Temple Body" texts, OK / Cancel):
//! buying the upgrade that makes a new temple asks first, since it sacrifices the towers around.

use crate::hud::{Action, Hud};
use crate::uitree::{draw_tree, Ctx, Node, R};
use macroquad::prelude::*;
use std::path::Path;

const LAYOUT: &str = "AnimatedObject/Layout";
/// Popup.TransitionAnim.Scale: seconds the popup takes to scale in.
const OPEN_SECS: f64 = 0.25;

pub struct TemplePopup {
    node: Option<Node>,
    /// (tower, path, True Sun God) waiting for an answer.
    pub open: Option<(usize, usize, bool)>,
    opened_at: f64,
}

impl TemplePopup {
    pub fn load(prefabs: &Path) -> Self {
        let node = Node::load(&prefabs.join("CommonPopup.json")).map(|mut n| {
            // Only the title, body and the two buttons: no cash readouts, debug fields, spinner.
            for p in ["Buttons/ConfirmButton/CashInfo", "CashInfo", "UserId", "GameVersion", "Cross", "Tick", "Spinner"] {
                n.set_active(&format!("{LAYOUT}/{p}"), false);
            }
            n.hide_named(&["Jewel"]);
            n
        });
        Self { node, open: None, opened_at: 0.0 }
    }
}

impl Hud {
    /// Asks before buying `path` on tower `t` (a Sun Temple, or with `two` a True Sun God).
    /// Returns false when the popup can't be shown (the caller then just upgrades).
    pub fn open_temple(&mut self, t: usize, path: usize, two: bool) -> bool {
        if self.temple.node.is_none() {
            return false;
        }
        self.temple.open = Some((t, path, two));
        self.temple.opened_at = get_time();
        true
    }

    /// The popup is up: it takes all input.
    pub fn temple_blocks(&self) -> bool {
        self.temple.open.is_some()
    }

    /// Draws the open popup over everything; OK gives `Action::TempleUpgrade`. Other HUD
    /// actions this frame are dropped (the popup is modal).
    pub fn draw_temple(&mut self, full: R, canvas_h: f32, s: f32, out: &mut Vec<Action>) {
        let Some((t, path, two)) = self.temple.open else { return };
        out.clear();
        let (title_key, body_key) = if two { ("True Sun Temple Title", "True Sun Temple Body") } else { ("Sun Temple Title", "Sun Temple Body") };
        let text = |k: &str| self.menus.loc.get(k).cloned().unwrap_or_default().replace("\\n", "\n");
        let (title, body, ok, cancel) = (text(title_key), text(body_key), text("OK"), text("Cancel"));
        let Some(n) = self.temple.node.as_mut() else { return };
        n.set_text(&format!("{LAYOUT}/Banner/Title"), &title);
        n.set_text(&format!("{LAYOUT}/Body"), &body);
        n.set_text(&format!("{LAYOUT}/Buttons/ConfirmButton/ConfirmText"), &ok);
        n.set_text(&format!("{LAYOUT}/Buttons/CancelButton/CancelText"), &cancel);
        // Scale in with a little overshoot.
        let k = ((get_time() - self.temple.opened_at) / OPEN_SECS).clamp(0.0, 1.0) as f32;
        let back = 1.70158;
        let e = 1.0 + (back + 1.0) * (k - 1.0).powi(3) + back * (k - 1.0).powi(2);
        if let Some(a) = n.find_mut("AnimatedObject") {
            a.rt.scale = vec2(e, e);
        }
        if k >= 1.0 {
            if n.clicked(&format!("{LAYOUT}/Buttons/ConfirmButton")) {
                out.push(Action::TempleUpgrade(t, path));
                self.temple.open = None;
            } else if n.clicked(&format!("{LAYOUT}/Buttons/CancelButton")) || is_key_pressed(KeyCode::Escape) {
                self.temple.open = None;
            }
        }
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.5 * k));
        let mut ctx = Ctx { sprites: &mut self.sprites, fonts: &self.fonts, scale: s, canvas_h, clip: None };
        draw_tree(n, full, &mut ctx);
    }
}
