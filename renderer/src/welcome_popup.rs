//! Not in BTD6 (see DEVIATIONS.md): the first time the game is ever opened, the home screen
//! shows a CommonPopup welcoming the player and explaining that everything is owned and maps
//! earn trophies. settings.json "welcome_seen" remembers it was closed; RRBTD6_WELCOME shows it
//! again (screenshots).

use crate::uitree::{block_input, draw_tree, Ctx, Node, R};
use macroquad::prelude::*;
use std::path::Path;

const LAYOUT: &str = "AnimatedObject/Layout";
/// Popup.TransitionAnim.Scale: seconds the popup takes to scale in (as the support popup).
const OPEN_SECS: f64 = 0.25;
const TITLE: &str = "Welcome to RUSTTD6!";
const BODY: &str = "In RUSTTD6, you own everything! There are no tower unlocks or Monkey Money. \
However, when you complete maps, you get trophies to buy the cosmetic items.";
const BUTTON: &str = "Let's Go!";

pub struct WelcomePopup {
    node: Option<Node>,
    opened_at: Option<f64>,
    /// Shown (or not needed) already this run.
    decided: bool,
    /// Showing something else (`show`), not the welcome.
    other: bool,
}

impl WelcomePopup {
    pub fn load(prefabs: &Path) -> Self {
        let node = Node::load(&prefabs.join("CommonPopup.json")).map(|mut n| {
            for p in ["Buttons/ConfirmButton/CashInfo", "Buttons/CancelButton", "CashInfo", "UserId", "GameVersion", "Cross", "Tick", "Spinner"] {
                n.set_active(&format!("{LAYOUT}/{p}"), false);
            }
            n.hide_named(&["Jewel"]);
            if let Some(b) = n.find_mut(&format!("{LAYOUT}/Body")).and_then(|b| b.txt.as_mut()) {
                b.size = 72.0;
            }
            let label = format!("{LAYOUT}/Buttons/ConfirmButton/ConfirmText");
            n.set_text(&label, BUTTON);
            n.set_text(&format!("{LAYOUT}/Banner/Title"), TITLE);
            n.set_text(&format!("{LAYOUT}/Body"), BODY);
            n
        });
        Self { node, opened_at: None, decided: false, other: false }
    }

    /// On the home screen: opens the popup once, the first time the game is ever run.
    pub fn maybe_open(&mut self, root: &Path) {
        if self.decided {
            return;
        }
        self.decided = true;
        let seen = std::fs::read(root.join("settings.json")).ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .is_some_and(|v| v["welcome_seen"] == true);
        let forced = std::env::var_os("RRBTD6_WELCOME").is_some();
        if (!seen && std::env::var_os("RRBTD6_AUTOTEST").is_none()) || forced {
            self.opened_at = Some(get_time());
        }
    }

    /// The same popup for other one-button notes (a co-op player's profile).
    pub fn show(&mut self, title: &str, body: &str) {
        if let Some(n) = self.node.as_mut() {
            n.set_text(&format!("{LAYOUT}/Banner/Title"), title);
            n.set_text(&format!("{LAYOUT}/Body"), body);
            n.set_text(&format!("{LAYOUT}/Buttons/ConfirmButton/ConfirmText"), "OK");
            self.opened_at = Some(get_time());
            self.other = true;
        }
    }

    /// Whether the popup open now is the welcome (closing it is remembered).
    pub fn is_welcome(&self) -> bool {
        !self.other
    }

    /// Remembers the popup was closed (settings.json "welcome_seen").
    pub fn mark_seen(root: &Path) {
        let path = root.join("settings.json");
        let mut v: serde_json::Value = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_else(|| serde_json::json!({}));
        if !v.is_object() { v = serde_json::json!({}); }
        v["welcome_seen"] = serde_json::Value::Bool(true);
        let _ = std::fs::write(path, v.to_string());
    }

    pub fn is_open(&self) -> bool {
        self.opened_at.is_some() && self.node.is_some()
    }

    /// Draws the popup over the dimmed screen; true on the frame it is closed.
    pub fn draw(&mut self, full: R, ctx: &mut Ctx) -> bool {
        let Some(opened) = self.opened_at else { return false };
        let Some(n) = self.node.as_mut() else { return false };
        let k = ((get_time() - opened) / OPEN_SECS).clamp(0.0, 1.0) as f32;
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.5 * k));
        // Scale in with a little overshoot.
        let back = 1.70158;
        let e = 1.0 + (back + 1.0) * (k - 1.0).powi(3) + back * (k - 1.0).powi(2);
        if let Some(a) = n.find_mut("AnimatedObject") {
            a.rt.scale = vec2(e, e);
        }
        block_input(false);
        draw_tree(n, full, ctx);
        let closed = k >= 1.0 && (n.clicked(&format!("{LAYOUT}/Buttons/ConfirmButton")) || is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Enter));
        if closed {
            self.opened_at = None;
            if self.other {
                // Back to the welcome's own text for any later use.
                self.other = false;
                n.set_text(&format!("{LAYOUT}/Banner/Title"), TITLE);
                n.set_text(&format!("{LAYOUT}/Body"), BODY);
                n.set_text(&format!("{LAYOUT}/Buttons/ConfirmButton/ConfirmText"), BUTTON);
                return false;
            }
        }
        block_input(!closed);
        closed
    }
}
