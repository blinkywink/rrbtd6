//! Not in BTD6 (see DEVIATIONS.md): the store and every purchase button show the screen behind
//! blurred and inert, with a CommonPopup asking players to support Ninja Kiwi through the
//! official client, offering the official webstore or a way back home. The same popup tells
//! players that the Legends DLC modes are not supported.

use crate::hud::Hud;
use crate::uitree::{block_input, draw_tree, Ctx, Node, R};
use macroquad::prelude::*;
use std::path::Path;

const LAYOUT: &str = "AnimatedObject/Layout";
/// Popup.TransitionAnim.Scale: seconds the popup takes to scale in.
const OPEN_SECS: f64 = 0.25;
/// The screen behind is snapshotted once its own opening animation has played.
const CAPTURE_SECS: f64 = 0.4;
pub const WEBSTORE: &str = "https://btd6store.ninjakiwi.com/";
const BODY: &str = "Please support ninja kiwi through their official client. They also have an official webstore where you can purchase items.";
const LEGENDS_BODY: &str = "RustTD6 does not support the Legends DLCs.";

/// Which message the popup shows.
#[derive(Clone, Copy, PartialEq)]
pub enum PopupKind {
    /// Support Ninja Kiwi: webstore or home.
    Support,
    /// Legends is not supported: home only.
    Legends,
}

pub struct SupportPopup {
    node: Option<Node>,
    kind: PopupKind,
    opened_at: Option<f64>,
    backdrop: Option<crate::menus::Backdrop>,
}

/// What the player chose.
pub enum SupportChoice {
    Webstore,
    Home,
}

impl SupportPopup {
    pub fn load(prefabs: &Path) -> Self {
        let node = Node::load(&prefabs.join("CommonPopup.json")).map(|mut n| {
            for p in ["Buttons/ConfirmButton/CashInfo", "CashInfo", "UserId", "GameVersion", "Cross", "Tick", "Spinner"] {
                n.set_active(&format!("{LAYOUT}/{p}"), false);
            }
            n.hide_named(&["Jewel"]);
            if let Some(b) = n.find_mut(&format!("{LAYOUT}/Body")).and_then(|b| b.txt.as_mut()) {
                b.size = 72.0;
            }
            // Wider buttons for the longer labels.
            for (button, text, label) in [("CancelButton", "CancelText", "Return Home"), ("ConfirmButton", "ConfirmText", "Take Me to the Webstore")] {
                if let Some(b) = n.find_mut(&format!("{LAYOUT}/Buttons/{button}")) {
                    b.rt.size.x = 820.0;
                }
                let path = format!("{LAYOUT}/Buttons/{button}/{text}");
                n.set_text(&path, label);
                if let Some(t) = n.find_mut(&path).and_then(|t| t.txt.as_mut()) {
                    t.size = 64.0;
                }
            }
            n
        });
        let mut popup = Self { node, kind: PopupKind::Support, opened_at: None, backdrop: None };
        popup.set_kind(PopupKind::Support);
        popup
    }

    fn set_kind(&mut self, kind: PopupKind) {
        self.kind = kind;
        let Some(n) = self.node.as_mut() else { return };
        let (title, body) = match kind {
            PopupKind::Support => ("Support Ninja Kiwi", BODY),
            PopupKind::Legends => ("Legends", LEGENDS_BODY),
        };
        n.set_text(&format!("{LAYOUT}/Banner/Title"), title);
        n.set_text(&format!("{LAYOUT}/Body"), body);
        n.set_active(&format!("{LAYOUT}/Buttons/ConfirmButton"), kind == PopupKind::Support);
    }
}

impl Hud {
    /// Shows the popup (no-op while it's up).
    pub fn open_support(&mut self) {
        self.open_popup(PopupKind::Support);
    }

    /// Shows the Legends-not-supported popup over the main menu.
    pub fn open_legends(&mut self) {
        self.open_popup(PopupKind::Legends);
    }

    fn open_popup(&mut self, kind: PopupKind) {
        if self.support.node.is_some() && self.support.opened_at.is_none() {
            self.support.set_kind(kind);
            self.support.opened_at = Some(get_time());
            self.support.backdrop = None;
        }
    }

    pub fn support_open(&self) -> bool {
        self.support.opened_at.is_some()
    }

    /// Draws the popup over what has been drawn this frame (which it blurs) and returns the
    /// player's choice. Menu clicks stay blocked (`block_input`) until it closes.
    pub fn draw_support(&mut self, full: R, canvas_h: f32, s: f32) -> Option<SupportChoice> {
        let opened = self.support.opened_at?;
        let t = get_time() - opened;
        if self.support.backdrop.is_none() && t >= CAPTURE_SECS {
            self.support.backdrop = Some(crate::menus::Backdrop::capture());
        }
        let k = (t / OPEN_SECS).clamp(0.0, 1.0) as f32;
        match &self.support.backdrop {
            Some(b) => b.draw_tinted(Color::new(0.75, 0.75, 0.75, 1.0)),
            None => draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.35 * k)),
        }
        let n = self.support.node.as_mut()?;
        // Scale in with a little overshoot.
        let back = 1.70158;
        let e = 1.0 + (back + 1.0) * (k - 1.0).powi(3) + back * (k - 1.0).powi(2);
        if let Some(a) = n.find_mut("AnimatedObject") {
            a.rt.scale = vec2(e, e);
        }
        let mut ctx = Ctx { sprites: &mut self.sprites, fonts: &self.fonts, scale: s, canvas_h, clip: None };
        draw_tree(n, full, &mut ctx);
        if k < 1.0 {
            return None;
        }
        block_input(false);
        let choice = if self.support.kind == PopupKind::Support && n.clicked(&format!("{LAYOUT}/Buttons/ConfirmButton")) {
            Some(SupportChoice::Webstore)
        } else if n.clicked(&format!("{LAYOUT}/Buttons/CancelButton")) || is_key_pressed(KeyCode::Escape) {
            Some(SupportChoice::Home)
        } else {
            None
        };
        block_input(choice.is_none());
        if choice.is_some() {
            self.support.opened_at = None;
            self.support.backdrop = None;
        }
        choice
    }
}

/// Opens the official webstore in the default browser.
pub fn open_webstore() {
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(WEBSTORE).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(WEBSTORE).spawn();
    #[cfg(not(any(windows, target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(WEBSTORE).spawn();
    if let Err(e) = r {
        eprintln!("couldn't open {WEBSTORE}: {e}");
    }
}
