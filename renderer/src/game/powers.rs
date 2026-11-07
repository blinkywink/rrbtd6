//! Using a Power (Simulation.Powers.PowerManager.Activate @0x180f37920): each of the Power's
//! behaviours is made into its Simulation.Powers implementation and Activated at the spot.
//! Built so far: Super Monkey Storm, Monkey Boost, Thrive, Cash Drop, Dart Time and the track items
//! (Road Spikes, Glue Trap, MOAB Mine, Camo Trap). Not yet: the tower Powers; `use_power` refuses
//! them.
//! HYPOTHESIS P-POWERFRAME: a Power acts between steps, before the next step's tower phase.

use super::*;

impl Game {
    /// The owner id of what a Power makes: 0 (the player) in single player; in co-op a stand-in id
    /// per player (u64::MAX - slot), registered to that player's wallet, so a Cash Drop's crate
    /// pays the player who used it.
    pub(crate) fn power_owner(&mut self) -> u64 {
        let Some(w) = self.coop_wallets.as_mut() else { return 0 };
        let Some(slot) = w.actor else { return 0 };
        let id = u64::MAX - slot as u64;
        w.owners.insert(id, slot);
        id
    }

    /// Whether `behaviors` (a Power's game/powers.json "behaviors") can be used by `use_power`.
    pub fn power_supported(behaviors: &serde_json::Value) -> bool {
        let kinds: Vec<&str> = behaviors.as_array().into_iter().flatten().filter_map(|b| b["type"].as_str()).collect();
        const BUILT: [&str; 9] = ["DamageAllModel", "MonkeyBoostModel", "ThriveModel", "CashDropModel", "RoadSpikesModel", "GlueTrapModel", "MoabMineModel", "CamoTrapModel", "DartTimeModel"];
        kinds.iter().any(|k| BUILT.contains(k))
            && kinds.iter().all(|k| BUILT.contains(k) || matches!(*k, "CreateEffectOnPowerModel" | "CreateSoundOnPowerModel"))
    }

    /// Whether Power `id` can be used in the clone: its behaviours are built, or it places a tower
    /// the clone simulates.
    pub fn power_usable(&self, id: &str, behaviors: &serde_json::Value) -> bool {
        Self::power_supported(behaviors) || self.power_tower_defs.contains_key(id)
    }

    /// Activates Power `id` (its game/powers.json behaviours) at `at` (game frame); false when it
    /// can't be used. Not modelled: CreateEffectOnPower (the full-screen effects) and
    /// CreateSoundOnPower.
    pub fn use_power(&mut self, id: &str, behaviors: &serde_json::Value, at: [f32; 2]) -> bool {
        // A tower Power (PowerManager.GetPowerTowerFromId @0x180f383a0): its TowerModel placed free
        // where a tower of it may go. HYPOTHESIS P-POWERTOWERPLACE: Power.CheckLocation is the tower's
        // own placement rule (Game::place).
        if let Some(&def) = self.power_tower_defs.get(id) {
            return self.place(def, at);
        }
        if !Self::power_supported(behaviors) {
            return false;
        }
        // TrackPower.CheckLocation (@0x180f43250): a track Power needs a path segment within 5 of the
        // spot (PathManager.GetPathSegmentsInRange). HYPOTHESIS P-TRACKPOWERRANGE: as the clone's
        // track_within (the distance to the path line).
        let track = behaviors.as_array().into_iter().flatten()
            .any(|b| matches!(b["type"].as_str(), Some("RoadSpikesModel" | "GlueTrapModel" | "MoabMineModel" | "CamoTrapModel")));
        if track && !self.track_within(at, 5.0) {
            return false;
        }
        for b in behaviors.as_array().into_iter().flatten() {
            match b["type"].as_str() {
                Some("DamageAllModel") => self.damage_all(b["amount"].as_f64().unwrap_or(0.0) as f32),
                Some("MonkeyBoostModel") => {
                    // MonkeyBoostModel.ctor (@0x181105d30): durationFrames = (int)(duration x 60).
                    let frames = (b["duration"].as_f64().unwrap_or(0.0) as f32 * 60.0) as i64;
                    self.monkey_boost(b["rateScale"].as_f64().unwrap_or(1.0) as f32, frames);
                }
                // Thrive.Activate (@0x180f42a60): roundsLeft 2 and an IncreaseBloonWorthSimBehavior
                // on the simulation. Not modelled: its ThriveModel mutator on the towers (no
                // Mutate of its own).
                Some("ThriveModel") => {
                    let scale = b["increaseBloonWorthSimBehaviorModel"]["cashScale"].as_f64().or(b["cashScale"].as_f64()).unwrap_or(1.0);
                    self.thrives.push((2, scale as f32));
                }
                // CashDrop.Activate (@0x180f312d0): its projectileModel (a collectable crate, CashModel
                // 2500) made at the spot (terrain height), owned by the player, rotation 0. Picked up
                // like bananas (collect_cash). Not modelled: PickupModel.delay.
                // DartTime.Activate (@0x180f32170): for (int)(duration x 60) frames, endFrame = now + that.
                Some("DartTimeModel") => {
                    let frames = (b["duration"].as_f64().unwrap_or(0.0) as f32 * 60.0) as i64;
                    let (bloon, tower) = (b["bloonSpeed"].as_f64().unwrap_or(1.0) as f32, b["towerAttackSpeed"].as_f64().unwrap_or(1.0) as f32);
                    self.dart_time = Some((self.round_clock + frames, bloon, tower));
                    for i in 0..self.bloons.len() {
                        self.dart_time_slow(i, frames as f32, bloon);
                    }
                }
                // TrackPower.CreateProjectile (@0x180f43420, as RoadSpikes / GlueTrap / MoabMine /
                // CamoTrap.Activate): its projectileModel at the spot, owned by the player. Not
                // modelled: the segment's height and the projectile's power flag (+0x1a5).
                Some("CashDropModel" | "RoadSpikesModel" | "GlueTrapModel" | "MoabMineModel" | "CamoTrapModel") => {
                    let Some(proj) = self.power_projectiles.get(id).cloned() else { return false };
                    let single = Emission { kind: "Single".into(), eject_random: 0.0, use_speed: false, speed_min: 1.0, speed_max: 1.0, count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0, dir_off: None , line_plain: false, line_tower_rot: false, line_ignore_blockers: false, line_from_target: None, pool_every: 0};
                    // Co-op: owned by the player who used it, so its cash goes to them.
                    let owner = self.power_owner();
                    self.emit(at, 0.0, &single, &proj, None, [0; 3], owner);
                }
                _ => {}
            }
        }
        true
    }

    /// DamageAll.Activate (@0x180f31f00): Bloon.Damage(amount) on every bloon in the factory
    /// list, distributed to children (no override); children made on the way aren't in the list.
    fn damage_all(&mut self, amount: f32) {
        let ids: Vec<u64> = self.bloons.iter().map(|b| b.id).collect();
        for id in ids {
            self.mod_damage_bloon(id, None, amount as i32);
        }
    }

    /// MonkeyBoost.Activate (@0x180f34c60): every tower gets the MonkeyBoost mutator, sub-towers
    /// included (Tower.AddMutatorIncludeSubTowers), for durationFrames: its weapons' rate x
    /// rateScale. Towers placed later don't get it. Timed like Vigilante's TimedMutator
    /// (`round_clock` - start < durationFrames). HYPOTHESIS P-BOOSTREPLACE: a second boost
    /// replaces the first (one mutator id).
    fn monkey_boost(&mut self, scale: f32, frames: i64) {
        let at = self.round_clock;
        self.power_boosts.retain(|b| at - b.1 < b.2);
        for t in 0..self.towers.len() {
            let id = self.towers[t].id;
            self.power_boosts.retain(|b| b.0 != id);
            self.power_boosts.push((id, at, frames, scale));
        }
    }

    /// The frames Dart Time has left (DartTime.Process @0x180f32900 ends it at endFrame).
    fn dart_time_left(&self) -> Option<(i64, f32, f32)> {
        self.dart_time.map(|(end, b, t)| (end - self.round_clock, b, t)).filter(|d| d.0 > 0)
    }

    /// TowerTimeSlowMutatorClass.Mutate (@0x1811138a0) on every tower (and, DartTime.TowerCreated,
    /// any placed while it runs): each WeaponModel's rate x 1 / towerAttackSpeed. Not modelled:
    /// animationSpeed x towerAttackSpeed (display) and the Heli / Path / Fighter movement speeds x
    /// towerAttackSpeed. HYPOTHESIS P-DARTTIMESUB: the TowerCreated exceptions (Paragon children)
    /// don't apply.
    pub(super) fn dart_time_rate(&self) -> f32 {
        self.dart_time_left().map_or(1.0, |(_, _, t)| 1.0 / t)
    }

    /// BloonTimeSlowMutatorClass.Mutate (@0x1810fd010) on bloon `i`: BloonModel.speed x bloonSpeed, a
    /// mutator of its own (so it multiplies with other slows). Not modelled: GrowModel's +0x28 /
    /// bloonSpeed (regrow) and the DartTimeBloonBehaviorModel it adds.
    fn dart_time_slow(&mut self, i: usize, frames: f32, speed: f32) {
        let added = self.sim_elapsed;
        self.bloons[i].effects.push(Effect { overlay: Default::default(), seq: 0, kind: EffectKind::Slow { mult: speed, glue: 0 }, id: "DartTime".into(), frames_left: frames, total: frames,
            added, layers: u32::MAX, owner: 0, distribute: (true, false), keep: false });
    }

    /// DartTime.BloonSpawned (@0x180f326e0): a new bloon gets the slow for the frames left.
    pub(super) fn dart_time_spawned(&mut self) {
        if let Some((left, speed, _)) = self.dart_time_left() {
            let i = self.bloons.len() - 1;
            self.dart_time_slow(i, left as f32, speed);
        }
    }

    /// Thrive.OnRoundStart (@0x180f42ee0): roundsLeft - 1; at 0 its mutator and sim behaviour go.
    pub(super) fn thrive_round_start(&mut self) {
        for t in &mut self.thrives {
            t.0 -= 1;
        }
        self.thrives.retain(|t| t.0 > 0);
    }

    /// Simulation.GetCashModifierFromSimBehaviors (@0x180b46ae0): 1 + each sim behaviour's
    /// cashScale - 1, an isUnique one (Thrive's) counted once per name.
    pub(super) fn sim_cash_modifier(&self) -> f64 {
        self.thrives.first().map_or(1.0, |t| 1.0 + (t.1 - 1.0)) as f64
    }

    /// The MonkeyBoost rate multiplier on tower `t` now (compute_buffs).
    pub(super) fn power_boost(&self, t: usize) -> f32 {
        let id = self.towers[t].id;
        self.power_boosts.iter().filter(|b| b.0 == id && self.round_clock - b.1 < b.2).map(|b| b.3).product()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::tests_support::{game, place_near};

    /// Super Monkey Storm pops every bloon on the map (4000 damage, distributed to children);
    /// Monkey Boost halves every tower's rate for 900 frames, towers placed later not included.
    #[test]
    fn storm_and_boost() {
        let mut g = game();
        g.sandbox = true;
        assert!(g.sandbox_spawn("Ceramic", false, false, false, 3, 0.0));
        for _ in 0..30 {
            g.step();
        }
        assert!(!g.bloons.is_empty());
        let storm = serde_json::json!([{"type": "DamageAllModel", "amount": 4000.0}]);
        assert!(g.use_power("SuperMonkeyStorm", &storm, [0.0, 0.0]));
        assert!(g.bloons.iter().all(|b| b.hp <= 0) || g.bloons.is_empty(), "{} left", g.bloons.len());
        let dart = g.tower_defs.iter().position(|(id, _)| id == "DartMonkey").unwrap();
        let t = place_near(&mut g, dart, 300.0);
        let boost = serde_json::json!([{"type": "MonkeyBoostModel", "rateScale": 0.5, "duration": 15.0}]);
        assert!(g.use_power("MonkeyBoost", &boost, [0.0, 0.0]));
        assert_eq!(g.power_boost(t), 0.5);
        let later = place_near(&mut g, dart, 500.0);
        assert_eq!(g.power_boost(later), 1.0);
        g.round_clock += 899;
        assert_eq!(g.power_boost(t), 0.5);
        g.round_clock += 1;
        assert_eq!(g.power_boost(t), 1.0);
    }

    /// Cash Drop puts its 2500-cash crate where it's used.
    #[test]
    fn cash_drop_crate() {
        let mut g = game();
        let drop = serde_json::json!([{"type": "CashDropModel"}]);
        let n = g.projectiles.len();
        assert!(g.use_power("CashDrop", &drop, [10.0, -20.0]));
        assert_eq!(g.projectiles.len(), n + 1);
        let p = g.projectiles.last().unwrap();
        assert_eq!(p.pos, [10.0, -20.0]);
        assert_eq!(p.spec.cash.as_ref().map(|c| (c.min, c.pickup)), Some((2500.0, Some(100.0))));
        assert!(!g.use_power("NoSuchPower", &drop, [0.0, 0.0]));
    }

    /// Road Spikes go only on the track: a pile of 20 at the spot, refused off it.
    #[test]
    fn road_spikes_on_track() {
        let mut g = game();
        let spikes = serde_json::json!([{"type": "RoadSpikesModel"}]);
        let on = g.track.pos(200.0);
        assert!(!g.use_power("RoadSpikes", &spikes, [9999.0, 9999.0]));
        let n = g.projectiles.len();
        assert!(g.use_power("RoadSpikes", &spikes, on));
        assert_eq!(g.projectiles.len(), n + 1);
        assert_eq!(g.projectiles.last().unwrap().pierce, 20.0);
    }

    /// Dart Time: bloons at 0.3 speed and towers' rate x 1 / 0.7 for 600 frames, new bloons too.
    #[test]
    fn dart_time() {
        let mut g = game();
        g.sandbox = true;
        assert!(g.sandbox_spawn("Red", false, false, false, 1, 0.0));
        for _ in 0..5 {
            g.step();
        }
        let dt = serde_json::json!([{"type": "DartTimeModel", "bloonSpeed": 0.3, "towerAttackSpeed": 0.7, "duration": 10.0}]);
        assert!(g.use_power("DartTime", &dt, [0.0, 0.0]));
        let def = &g.defs[g.bloons[0].kind];
        assert!((g.bloons[0].speed_mult(def) - 0.3).abs() < 1e-6);
        assert!((g.dart_time_rate() - 1.0 / 0.7).abs() < 1e-6);
        assert!(g.sandbox_spawn("Red", false, false, false, 1, 0.0));
        for _ in 0..3 {
            g.step();
        }
        let last = g.bloons.last().unwrap();
        assert!(last.effects.iter().any(|e| e.id == "DartTime"));
        g.round_clock += 600;
        assert_eq!(g.dart_time_rate(), 1.0);
    }

    /// Cave Monkey places its tower free; Banana Farmer is usable, Pontoon (makeshift water) isn't yet.
    #[test]
    fn cave_monkey_tower() {
        let mut g = game();
        assert!(g.power_usable("CaveMonkey", &serde_json::json!([])));
        assert!(g.power_usable("BananaFarmer", &serde_json::json!([])));
        assert!(!g.power_usable("Pontoon", &serde_json::json!([])));
        let def = g.power_tower_defs["CaveMonkey"];
        assert!(g.shop.iter().all(|s| s.id != g.tower_defs[def].0));
        g.cash = 0.0;
        let n = g.towers.len();
        let c = g.track.pos(300.0);
        let spot = (0..48).map(|k| { let a = k as f32 / 48.0 * std::f32::consts::TAU; [c[0] + 40.0 * a.cos(), c[1] + 40.0 * a.sin()] })
            .find(|p| g.can_place(def, *p)).unwrap();
        assert!(g.use_power("CaveMonkey", &serde_json::json!([]), spot));
        assert_eq!(g.towers.len(), n + 1);
        assert_eq!(g.cash, 0.0);
    }

    /// Thrive: pop cash x 1.25 for the rest of this round and the next; two Thrives don't stack.
    #[test]
    fn thrive_two_rounds() {
        let mut g = game();
        let thrive = serde_json::json!([{"type": "ThriveModel", "cashScale": 1.25,
            "increaseBloonWorthSimBehaviorModel": {"cashScale": 1.25, "isUnique": true}}]);
        assert!(g.use_power("Thrive", &thrive, [0.0, 0.0]));
        assert!(g.use_power("Thrive", &thrive, [0.0, 0.0]));
        assert_eq!(g.sim_cash_modifier(), 1.25);
        g.thrive_round_start();
        assert_eq!(g.sim_cash_modifier(), 1.25);
        g.thrive_round_start();
        assert_eq!(g.sim_cash_modifier(), 1.0);
    }

    #[test]
    fn supported_kinds() {
        let storm = serde_json::json!([{"type": "DamageAllModel", "amount": 4000.0}, {"type": "CreateSoundOnPowerModel"}]);
        let lake = serde_json::json!([{"type": "CreateEffectOnPowerModel"}]);
        assert!(Game::power_supported(&storm));
        assert!(!Game::power_supported(&lake));
        assert!(!Game::power_supported(&serde_json::json!([{"type": "DamageAllModel"}, {"type": "HealthPotionModel"}])));
    }
}
