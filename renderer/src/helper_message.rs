//! GameMessageManager's HelperMessage: the blue panel that slides in from the top with a hint
//! (InputManager.SetHelperMessage -> HelperMessage.OnShowHelperMessage @0x180ac2a90). A non-empty
//! message sets the animator's "Trigger" bool and the localized text (clearAt = now + time when
//! time > 0); an empty one clears the bool. Update (@0x180ac2b80) clears it at clearAt.
//! The HintMessage controller (read from the AnimatorController): AnyState -> SlideIn while
//! Trigger is true, SlideIn -> SlideOut when it goes false, SlideOut (the Slide clip at speed -1)
//! -> Idle at its exit time; Idle holds the container at y 1000 with the panel inactive.
//! Used now by the custom input modes (OverclockInput's "Select a Monkey" / "InvalidTarget").

use crate::uitree::{draw_tree, Ctx, Node, R};
use macroquad::prelude::*;
use serde_json::Value;
use std::path::Path;

const CONTROLLER: &str = "HintMessage";
const TEXT: &str = "HelperMessage/MessageContainer/Anim/TextMeshPro Text";

#[derive(Clone, Copy, PartialEq)]
enum State {
    Idle,
    SlideIn(f64),
    SlideOut(f64),
}

pub struct HelperMessage {
    node: Option<Node>,
    /// HelperMessage's own animator (its MessageContainer).
    runtime: Value,
    trigger: bool,
    state: State,
    /// HelperMessage.clearAt (get_time seconds), when the message times out.
    clear_at: Option<f64>,
}

impl HelperMessage {
    pub fn load(prefabs: &Path) -> Self {
        let mut all: Value = std::fs::read_to_string(prefabs.join("../runtime/GameMessageManager.json")).ok()
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        // The clips name paths from the prefab root.
        for animator in all["animators"].as_array_mut().into_iter().flatten() {
            for clip in animator["clips"].as_array_mut().into_iter().flatten() {
                for binding in clip["bindings"].as_array_mut().into_iter().flatten() {
                    if let Some(p) = binding["path"].as_str().and_then(|p| p.strip_prefix("GameMessageManager/")) {
                        binding["path"] = p.to_owned().into();
                    }
                }
            }
        }
        let node = Node::load(&prefabs.join("GameMessageManager.json")).map(|mut n| {
            // Every message's animator starts in Idle (off screen).
            crate::frontend::apply_animation(&mut n, &all, CONTROLLER, "Idle", None);
            n
        });
        let own = all["animators"].as_array().into_iter().flatten()
            .find(|a| a["path"].as_str() == Some("GameMessageManager/HelperMessage/MessageContainer")).cloned().unwrap_or_default();
        Self { node, runtime: serde_json::json!({ "animators": [own] }), trigger: false, state: State::Idle, clear_at: None }
    }

    /// InputManager.SetHelperMessage(text, time): `text` already localized; empty hides it.
    pub fn set(&mut self, text: &str, time: f32) {
        let now = get_time();
        self.trigger = !text.is_empty();
        if self.trigger {
            if let Some(n) = &mut self.node {
                n.set_text(TEXT, text);
            }
            self.clear_at = (time > 0.0).then(|| now + time as f64);
        } else {
            self.clear_at = None;
        }
    }

    pub fn draw(&mut self, full: R, ctx: &mut Ctx) {
        let now = get_time();
        if self.clear_at.is_some_and(|t| t <= now) {
            self.trigger = false;
            self.clear_at = None;
        }
        // AnyState -> SlideIn (not to itself); SlideIn -> SlideOut on !Trigger.
        self.state = match self.state {
            State::Idle | State::SlideOut(_) if self.trigger => State::SlideIn(now),
            State::SlideIn(_) if !self.trigger => State::SlideOut(now),
            s => s,
        };
        let Some(n) = &mut self.node else { return };
        let stop = self.runtime["animators"][0]["clips"].as_array().into_iter().flatten()
            .find(|c| c["name"] == "Slide").and_then(|c| c["stop"].as_f64()).unwrap_or(0.0) as f32;
        match self.state {
            State::SlideIn(t0) => {
                crate::frontend::apply_animation(n, &self.runtime, CONTROLLER, "Slide", Some((now - t0) as f32));
            }
            State::SlideOut(t0) => {
                let e = (now - t0) as f32;
                if e >= stop {
                    self.state = State::Idle;
                    crate::frontend::apply_animation(n, &self.runtime, CONTROLLER, "Idle", None);
                } else {
                    crate::frontend::apply_animation(n, &self.runtime, CONTROLLER, "Slide", Some(stop - e));
                }
            }
            State::Idle => {}
        }
        if self.state != State::Idle {
            draw_tree(n, full, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The local export (skipped without one): it starts idle at y 1000, the panel off.
    #[test]
    fn starts_idle() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_extracted");
        if !root.join("ui/prefabs/GameMessageManager.json").exists() || !root.join("ui/runtime/GameMessageManager.json").exists() {
            return;
        }
        let m = HelperMessage::load(&root.join("ui/prefabs"));
        let n = m.node.as_ref().unwrap();
        let c = n.find("HelperMessage/MessageContainer").unwrap();
        assert_eq!(c.rt.pos.y, 1000.0);
        assert!(!n.find("HelperMessage/MessageContainer/Anim").unwrap().active);
        assert!(!n.find("MaxMonkeysMessage/MessageContainer/Anim").unwrap().active);
    }
}
