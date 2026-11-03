//! Boss schedule and triggers from game/bosses.json (tools/assets/export_bosses.py), without the
//! simulation: `Game` isn't hooked up to this yet. docs/dev/BOSSES.md has the plan.
//!
//! VERIFIED-DISASM: BossBloonManager.GetTierFromCurrentRound, OnRoundStart, GetBossMustBeDefeatedByRound;
//! HealthPercentTrigger.Damage and Trigger. Not run against a
//! recording. DEFAULT_SPAWN_ROUNDS is read from the game's metadata by its shape, not through the field.
#![allow(dead_code)]

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// BossGameData.DefaultSpawnRounds: round indices, so rounds 40, 60, 80, 100 and 120.
pub const DEFAULT_SPAWN_ROUNDS: [i32; 5] = [39, 59, 79, 99, 119];

#[derive(Deserialize, Clone, Debug)]
pub struct BossTier {
    pub family: String,
    pub elite: bool,
    pub tier: u32,
    pub model: String,
    pub health: i32,
    pub speed: f32,
    pub radius: f32,
    pub behaviors: Vec<Value>,
}

impl BossTier {
    /// The behaviours of a model type.
    pub fn of_type<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a Value> + 'a {
        self.behaviors.iter().filter(move |b| b["type"] == kind)
    }

    /// The action models a trigger's id runs (HealthPercentTrigger gathers them by actionId).
    pub fn actions<'a>(&'a self, id: &'a str) -> impl Iterator<Item = &'a Value> + 'a {
        self.behaviors.iter().filter(move |b| b["actionId"] == id)
    }

    pub fn health_triggers(&self) -> Vec<HealthTrigger> {
        self.of_type("HealthPercentTriggerModel").map(|b| HealthTrigger::new(b, self.health)).collect()
    }
}

/// game/bosses.json by bloon id; empty when it isn't there.
pub fn load(root: &Path) -> HashMap<String, BossTier> {
    std::fs::read(root.join("game/bosses.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// BossBloonManager.OnRoundStart's GameModel.GetBloon: the name, "Elite" if elite, then the tier from 1.
pub fn bloon_id(family: &str, elite: bool, tier_index: usize) -> String {
    format!("{family}{}{}", if elite { "Elite" } else { "" }, tier_index + 1)
}

/// The tier index that spawns at the start of `round` (OnRoundStart).
pub fn tier_spawning(spawn_rounds: &[i32], round: i32) -> Option<usize> {
    spawn_rounds.iter().position(|&r| r == round)
}

/// GetTierFromCurrentRound: the first tier whose spawn round isn't past, skipping -1; else 0.
pub fn tier_from_round(spawn_rounds: &[i32], round: i32) -> usize {
    spawn_rounds.iter().position(|&r| r != -1 && round <= r).unwrap_or(0)
}

/// GetBossMustBeDefeatedByRound: the round before the next tier's, or 19 after the last tier's.
pub fn must_be_defeated_by(spawn_rounds: &[i32], current_tier: usize) -> Option<i32> {
    match spawn_rounds.get(current_tier) {
        Some(r) => Some(r - 1),
        None => spawn_rounds.get(current_tier.checked_sub(1)?).map(|r| r + 19),
    }
}

/// HealthPercentTrigger's state on one boss.
#[derive(Clone, Debug)]
pub struct HealthTrigger {
    percentages: Vec<f32>,
    pub action_ids: Vec<String>,
    repeat_first: bool,
    lowest: i32,
    max: i32,
    /// currentSkull: how many of `percentages` have been passed.
    pub skull: usize,
}

impl HealthTrigger {
    pub fn new(model: &Value, max_health: i32) -> Self {
        let list = |key: &str| model[key].as_array().cloned().unwrap_or_default();
        HealthTrigger {
            percentages: list("percentageValues").iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect(),
            action_ids: list("actionIds").iter().filter_map(|v| v.as_str().map(String::from)).collect(),
            repeat_first: model["repeatFirst"] == true,
            lowest: max_health,
            max: max_health,
            skull: 0,
        }
    }

    /// HealthPercentTrigger.Damage: how many times to run every action, given the health before a
    /// hit and its damage. Only a new lowest health counts, so healing can't fire one twice.
    pub fn damage(&mut self, health: i32, damage: i32) -> u32 {
        let after = health - damage;
        if self.lowest <= after {
            return 0;
        }
        let max = self.max as f32;
        let previous = self.lowest as f32 / max;
        let low = after.max(0);
        self.lowest = low;
        if self.repeat_first {
            // Every whole step of the first value between the health before and after.
            let Some(&step) = self.percentages.first() else { return 0 };
            let steps = |h: f32| ((1.0 - h / max) / step) as i32;
            return (steps(low as f32) - steps(health as f32)).max(0) as u32;
        }
        let new = low as f32 / max;
        let mut fired = 0;
        for (i, &p) in self.percentages.iter().enumerate() {
            if p < previous && new <= p {
                fired += 1;
                self.skull = i + 1;
            }
        }
        fired
    }
}

/// Frames per second of the simulation's clock.
const FPS: f32 = 60.0;

/// What a boss does: the action models its triggers run, for the simulation to carry out.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// SetInvulnerableActionModel.set
    Invulnerable(bool),
    /// PlayAnimTriggerActionModel: trigger name, seconds the boss stops moving.
    Animation(String, f32),
    /// SetSpeedPercentActionModel: speed fraction, seconds, distance moved (a knockback is negative).
    Speed { percent: f32, seconds: f32, distance: f32, revert: bool },
    /// SpawnBloonsActionModel, whole.
    Spawn(Value),
    /// Every other action model, whole: its "type" says what.
    Other(Value),
}

impl Effect {
    fn of(model: &Value) -> Self {
        let num = |key: &str| model[key].as_f64().unwrap_or(0.0) as f32;
        match model["type"].as_str().unwrap_or("") {
            "SetInvulnerableActionModel" => Effect::Invulnerable(model["set"] == true),
            "PlayAnimTriggerActionModel" => Effect::Animation(model["triggerName"].as_str().unwrap_or("").to_owned(), num("pauseMovementDuration")),
            "SetSpeedPercentActionModel" => Effect::Speed { percent: num("percent"), seconds: num("time"), distance: num("distance"), revert: model["revert"] == true },
            "SpawnBloonsActionModel" => Effect::Spawn(model.clone()),
            _ => Effect::Other(model.clone()),
        }
    }
}

struct TimeTrigger {
    interval: u32,
    last: u32,
    ids: Vec<String>,
    pauses: u32,
}

/// One boss's triggers and timers. Feed it damage and frames; it says what to do.
///
/// VERIFIED-DISASM: TimeTrigger.Process, WaitForSecondsAction.PerformAction and Process.
/// STAND-IN: triggerImmediately fires on the first frame.
pub struct BossScript {
    boss: BossTier,
    health: Vec<HealthTrigger>,
    timers: Vec<TimeTrigger>,
    /// WaitForSecondsAction: (frame it's due, the action ids it then runs).
    waiting: Vec<(u32, Vec<String>)>,
}

fn ids(model: &Value) -> Vec<String> {
    model["actionIds"].as_array().into_iter().flatten().filter_map(|v| v.as_str().map(String::from)).collect()
}

impl BossScript {
    pub fn new(boss: BossTier, frame: u32) -> Self {
        let timers = boss.of_type("TimeTriggerModel").map(|t| {
            let interval = (t["interval"].as_f64().unwrap_or(0.0) as f32 * FPS) as u32;
            let last = if t["triggerImmediately"] == true { frame.saturating_sub(interval) } else { frame };
            TimeTrigger { interval, last, ids: ids(t), pauses: 0 }
        }).collect();
        BossScript { health: boss.health_triggers(), timers, waiting: Vec::new(), boss }
    }

    /// The highest skull passed, for the HUD's GetCurrentSkull.
    pub fn skull(&self) -> usize {
        self.health.iter().map(|t| t.skull).max().unwrap_or(0)
    }

    /// TimeTrigger.timeTriggerPauseCount: timers don't run while paused.
    pub fn pause_timers(&mut self, paused: bool) {
        for t in &mut self.timers {
            t.pauses = if paused { t.pauses + 1 } else { t.pauses.saturating_sub(1) };
        }
    }

    /// Runs the actions with these ids; a WaitForSecondsAction starts its timer instead.
    fn run(&mut self, action_ids: &[String], frame: u32, out: &mut Vec<Effect>) {
        for id in action_ids {
            let models: Vec<Value> = self.boss.actions(id).cloned().collect();
            for model in models {
                if model["type"] == "WaitForSecondsActionModel" {
                    let delay = (model["delayTime"].as_f64().unwrap_or(0.0) as f32 * FPS) as u32;
                    self.waiting.push((frame + delay, ids(&model)));
                } else {
                    out.push(Effect::of(&model));
                }
            }
        }
    }

    /// A hit: the health before it and its damage.
    pub fn damage(&mut self, health: i32, damage: i32, frame: u32) -> Vec<Effect> {
        let mut out = Vec::new();
        for t in 0..self.health.len() {
            let times = self.health[t].damage(health, damage);
            let action_ids = self.health[t].action_ids.clone();
            for _ in 0..times {
                self.run(&action_ids, frame, &mut out);
            }
        }
        out
    }

    /// One frame of the simulation's clock.
    pub fn tick(&mut self, frame: u32) -> Vec<Effect> {
        let mut out = Vec::new();
        for t in 0..self.timers.len() {
            let timer = &mut self.timers[t];
            if timer.pauses == 0 && frame >= timer.last + timer.interval {
                timer.last = frame;
                let action_ids = timer.ids.clone();
                self.run(&action_ids, frame, &mut out);
            }
        }
        let due: Vec<Vec<String>> = {
            let (now, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.waiting).into_iter().partition(|w| w.0 <= frame);
            self.waiting = later;
            now.into_iter().map(|w| w.1).collect()
        };
        for action_ids in due {
            self.run(&action_ids, frame, &mut out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn boss(behaviors: Value) -> BossTier {
        serde_json::from_value(json!({"family":"Test","elite":false,"tier":1,"model":"","health":1000,"speed":1.0,"radius":10.0,"behaviors":behaviors})).unwrap()
    }

    #[test]
    fn every_action_runs_at_a_threshold_and_a_wait_finishes_later() {
        let mut script = BossScript::new(boss(json!([
            {"type":"HealthPercentTriggerModel","percentageValues":[0.5],"actionIds":["On","Later"]},
            {"type":"SetInvulnerableActionModel","actionId":"On","set":true},
            {"type":"WaitForSecondsActionModel","actionId":"Later","delayTime":0.5,"actionIds":["Off"]},
            {"type":"SetInvulnerableActionModel","actionId":"Off","set":false}])), 0);
        assert_eq!(script.damage(1000, 600, 100), vec![Effect::Invulnerable(true)]);
        assert_eq!(script.skull(), 1);
        assert!(script.tick(129).is_empty());
        assert_eq!(script.tick(130), vec![Effect::Invulnerable(false)]);
        assert!(script.tick(131).is_empty());
    }

    #[test]
    fn timers_repeat_at_their_interval_unless_paused() {
        let mut script = BossScript::new(boss(json!([
            {"type":"TimeTriggerModel","interval":5.0,"actionIds":["Go"]},
            {"type":"PlayAnimTriggerActionModel","actionId":"Go","triggerName":"Hit","pauseMovementDuration":1.0}])), 10);
        assert!(script.tick(309).is_empty());
        assert_eq!(script.tick(310), vec![Effect::Animation("Hit".into(), 1.0)]);
        script.pause_timers(true);
        assert!(script.tick(700).is_empty());
        script.pause_timers(false);
        assert_eq!(script.tick(701).len(), 1);
    }

    /// Real data: Bloonarius's skull runs its spawns, Vortex's stun ends its own invulnerability.
    #[test]
    fn real_bosses_run_their_scripts() {
        let bosses = load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_extracted"));
        let (Some(bloonarius), Some(vortex)) = (bosses.get("Bloonarius1"), bosses.get("Vortex1")) else { return };
        let mut script = BossScript::new(bloonarius.clone(), 0);
        let effects = script.damage(bloonarius.health, bloonarius.health / 4, 1);
        assert_eq!(effects.iter().filter(|e| matches!(e, Effect::Spawn(_))).count(), 2 + 25, "both spawns at 75%, plus a bleed per 1% of health");
        let mut script = BossScript::new(vortex.clone(), 0);
        let effects = script.damage(vortex.health, vortex.health / 4, 1);
        assert!(effects.contains(&Effect::Invulnerable(true)));
        assert!(effects.iter().any(|e| matches!(e, Effect::Speed { distance, .. } if *distance < 0.0)), "knockback");
        let later: Vec<Effect> = (2..120).flat_map(|f| script.tick(f)).collect();
        assert!(later.contains(&Effect::Invulnerable(false)));
    }

    fn skulls() -> HealthTrigger {
        HealthTrigger::new(&json!({"percentageValues":[0.75,0.5,0.25],"actionIds":["A","B"]}), 1000)
    }

    #[test]
    fn schedule_follows_the_spawn_rounds() {
        assert_eq!(bloon_id("Lych", true, 2), "LychElite3");
        assert_eq!(bloon_id("Vortex", false, 0), "Vortex1");
        assert_eq!(tier_spawning(&DEFAULT_SPAWN_ROUNDS, 59), Some(1));
        assert_eq!(tier_spawning(&DEFAULT_SPAWN_ROUNDS, 60), None);
        assert_eq!(tier_from_round(&DEFAULT_SPAWN_ROUNDS, 0), 0);
        assert_eq!(tier_from_round(&DEFAULT_SPAWN_ROUNDS, 60), 2);
        assert_eq!(tier_from_round(&[-1, 79], 5), 1);
        assert_eq!(must_be_defeated_by(&DEFAULT_SPAWN_ROUNDS, 1), Some(58));
        assert_eq!(must_be_defeated_by(&DEFAULT_SPAWN_ROUNDS, 5), Some(138));
    }

    #[test]
    fn skulls_fire_once_each_as_health_passes_them() {
        let mut t = skulls();
        assert_eq!(t.damage(1000, 100), 0);
        assert_eq!(t.damage(900, 150), 1, "exactly 75%");
        assert_eq!(t.skull, 1);
        assert_eq!(t.damage(750, 10), 0);
        assert_eq!(t.damage(740, 600), 2, "one hit past 50% and 25%");
        assert_eq!(t.skull, 3);
    }

    #[test]
    fn healing_does_not_fire_a_skull_again() {
        let mut t = skulls();
        assert_eq!(t.damage(1000, 300), 1);
        // Healed back to full, then hit down to where it had already been.
        assert_eq!(t.damage(1000, 300), 0);
        assert_eq!(t.skull, 1);
    }

    #[test]
    fn overkill_stops_at_zero() {
        let mut t = skulls();
        assert_eq!(t.damage(1000, 5000), 3);
        assert_eq!(t.damage(0, 5), 0);
    }

    #[test]
    fn repeating_trigger_fires_per_step() {
        let mut t = HealthTrigger::new(&json!({"percentageValues":[0.01],"actionIds":["Bleed"],"repeatFirst":true}), 1000);
        assert_eq!(t.damage(1000, 5), 0);
        assert_eq!(t.damage(995, 30), 3);
    }

    /// The real export, when it's there: every tier parses, every trigger's actions exist.
    #[test]
    fn exported_bosses_are_usable() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_extracted");
        let bosses = load(&root);
        if bosses.is_empty() {
            return;
        }
        assert_eq!(bosses.len(), 60);
        for (id, boss) in &bosses {
            assert_eq!(&bloon_id(&boss.family, boss.elite, boss.tier as usize - 1), id);
            assert!(boss.health > 0, "{id}");
            assert!(root.join("models/bosses").join(&boss.model).join("meta.json").exists(), "{id} model");
            let triggers = boss.health_triggers();
            // Lych runs on LychBossSuperScriptModel instead.
            assert_eq!(triggers.is_empty(), boss.family == "Lych", "{id}");
            for trigger in triggers {
                for action in &trigger.action_ids {
                    assert!(boss.actions(action).next().is_some(), "{id}: no action {action}");
                }
            }
        }
    }
}
