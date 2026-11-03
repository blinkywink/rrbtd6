//! RightMenu.cancelPlacementBtn / confirmPlacementBtn: RightPanel's CancelPurchaseButton and
//! ConfirmlPurchaseButton. InputManager.EnterPlacementMode shows the cancel button, and in
//! Lock & Drop the confirm button too; ExitPlacementMode hides both. Show sets the animator's
//! visibleStateLabel to 1 (2 on a 4:3 screen: CancelPurchaseSlideIn / CancelPurchaseSlideIn4-3),
//! Hide to 0 (Idle: off screen, Btn inactive). The Button fires on release over it, like Unity's.
use super::*;

const PATHS: [&str; 2] = ["CancelPurchaseButton", "ConfirmlPurchaseButton"];

#[derive(Default)]
struct PlaceButton {
    /// This button's own animator (both use the CancelPurchase controller).
    runtime: serde_json::Value,
    /// Show(): when, and whether the 4:3 clip was picked.
    shown: Option<(f64, bool)>,
    /// Pressed over the button, waiting for the release.
    down: bool,
}

#[derive(Default)]
pub(super) struct PlaceButtons([PlaceButton; 2]);

pub enum PlaceClick {
    Cancel,
    Confirm,
}

impl PlaceButtons {
    pub fn load(dir: &Path) -> Self {
        let data: serde_json::Value = std::fs::read_to_string(dir.join("../runtime/RightPanel.json")).ok()
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let mut out = Self::default();
        for (b, path) in out.0.iter_mut().zip(PATHS) {
            let mut animator = data["animators"].as_array().into_iter().flatten()
                .find(|a| a["path"].as_str() == Some(&format!("RightPanel/{path}"))).cloned().unwrap_or_default();
            for clip in animator["clips"].as_array_mut().into_iter().flatten() {
                for binding in clip["bindings"].as_array_mut().into_iter().flatten() {
                    if let Some(p) = binding["path"].as_str().and_then(|p| p.strip_prefix("RightPanel/")) {
                        binding["path"] = p.to_owned().into();
                    }
                }
            }
            b.runtime = serde_json::json!({ "animators": [animator] });
        }
        out
    }
}

impl Hud {
    /// A click released over a shown button (InputManager.CancelClicked /
    /// ConfirmPlacementClicked). Uses the last layout, like `Node::clicked`.
    pub fn placement_click(&mut self) -> Option<PlaceClick> {
        let (mx, my) = mouse_position();
        let mut click = None;
        for (i, b) in self.place_btns.0.iter_mut().enumerate() {
            let over = b.shown.is_some() && self.shop.rect(PATHS[i]).is_some_and(|r| r.contains(vec2(mx, my)));
            if is_mouse_button_pressed(MouseButton::Left) {
                b.down = over;
            }
            if is_mouse_button_released(MouseButton::Left) {
                if b.down && over {
                    click = Some(if i == 0 { PlaceClick::Cancel } else { PlaceClick::Confirm });
                }
                b.down = false;
            }
        }
        click
    }

    /// Show / Hide: the cancel button while placing, the confirm button too in Lock & Drop.
    pub fn show_placement_buttons(&mut self, cancel: bool, confirm: bool) {
        let bottom = Self::layout().bottom;
        for (i, want) in [cancel, confirm].into_iter().enumerate() {
            let b = &mut self.place_btns.0[i];
            match (want, b.shown) {
                (true, None) => b.shown = Some((get_time(), bottom)),
                (false, Some(_)) => b.shown = None,
                _ => {}
            }
            let (clip, elapsed) = match b.shown {
                Some((at, four_three)) => (if four_three { "CancelPurchaseSlideIn4-3" } else { "CancelPurchaseSlideIn" }, Some((get_time() - at) as f32)),
                None => ("Idle", None),
            };
            crate::frontend::apply_animation(&mut self.shop, &b.runtime, "CancelPurchase", clip, elapsed);
        }
    }

    /// InputManager.CursorDown: a press inside either button's RectTransform leaves cursorMode.
    pub fn over_placement_button(&self, p: Vec2) -> bool {
        PATHS.iter().any(|path| self.shop.rect(path).is_some_and(|r| r.contains(p)))
    }

    pub(super) fn placement_button_blocks(&self, p: Vec2) -> bool {
        self.place_btns.0.iter().zip(PATHS).any(|(b, path)| b.shown.is_some() && self.shop.rect(path).is_some_and(|r| r.contains(p)))
    }
}
