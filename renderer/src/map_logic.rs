//! A map's removeables and map events (Map.SellRemoveable, Triggers.MapEvent and its triggers and
//! actions), from map.json's "logic" (tools/assets/export_map_mechanics.py). A child of game.rs.
//!
//! VERIFIED-DISASM: Map.SellRemoveable @0x180f5e810, MapEvent.IsTriggered, AfterRoundTrigger.TriggerCond
//! @0x180f7a5b0 and OnRoundEnd, PreBloonSpawnTrigger.OnPreBloonSpawn, Map.CreateArea. Not run against a
//! recording of these maps. Not modelled: the triggers and actions the export leaves out (clicks, sim
//! time, start of round, moving areas, projectiles...), towers removed by a new area
//! (FindAndDestroyTowersIfInInvalidLocations), RegenRemovable.

use super::*;
use crate::data::{Area, MapPath};
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct LogicDef {
    #[serde(default)]
    pub removeables: Vec<RemoveableDef>,
    #[serde(default)]
    pub events: Vec<EventDef>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct RemoveableDef {
    pub id: i64,
    pub name: String,
    pub cost: i32,
    pub active: bool,
    pub destroy_area: bool,
    /// The Area scripts below it (Map.GetAreasLinkedToRemovable).
    pub areas: Vec<i64>,
    /// Parts of the map's model that go when it's sold.
    pub hide: Vec<usize>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct EventDef {
    pub id: i64,
    pub once: bool,
    pub all: bool,
    pub active: bool,
    pub triggers: Vec<Trigger>,
    pub actions: Vec<Action>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind")]
pub enum Trigger {
    /// missed: triggerOnceIfRoundMissed (TriggerCond: a round index past `round` fires it once).
    AfterRound { round: i32, nth: i32, primes: bool, #[serde(default)] missed: bool },
    StartMap,
    PreBloonSpawn { tag: String },
    RemoveableSold { id: i64 },
    StartRound { round: i32, nth: i32, primes: bool, from_start: bool },
    /// Frames of play, or of round time.
    SimTime { frames: u32, round_time: bool },
    /// percent: of the starting lives, instead of `lives`.
    LivesLower { lives: i32, percent: Option<f64> },
    /// A newer export's kind: never fires, and the map still loads.
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "kind")]
pub enum Action {
    Path { id: i64, active: bool },
    Area { ids: Vec<i64>, create: bool },
    Removeable { id: i64, active: bool },
    Sell { id: i64 },
    Event { id: i64, active: bool },
    /// RotateAreaAction: `area` turns `rotation` degrees (Unity's sense) over `duration` seconds
    /// about `pivot` (clone frame), with the towers on it.
    RotateArea { id: i64, area: i64, rotation: f32, duration: f32, pivot: [f32; 2] },
    /// CycleAreasAction: one of `ids` off (invert) or on, the rest the other way, turning with the round.
    CycleAreas { ids: Vec<i64>, invert: bool, from_start: bool, per: i32 },
    /// A newer export's kind: does nothing, and the map still loads.
    #[serde(other)]
    Other,
}

/// One removeable's state in `LogicState::removeables`.
#[derive(Clone, Copy, Default)]
struct RemoveableState {
    active: bool,
    sold: bool,
}

#[derive(Default)]
pub struct LogicState {
    pub def: LogicDef,
    removeables: Vec<RemoveableState>,
    events_active: Vec<bool>,
    /// MapTrigger.triggered per event, per trigger.
    triggered: Vec<Vec<bool>>,
    /// AfterRoundTrigger hasTriggeredFromRoundSkip and its trigger count (CheckIfTriggered), per
    /// event, per trigger (filled on first use).
    round_skip: Vec<Vec<(bool, i32)>>,
    /// Every Area of the map: DestroyArea and CreateArea move them out of and into `Game::areas`.
    pool: Vec<Area>,
    /// The active paths' data (parallel to `Game::tracks`), and paths not yet on the map.
    paths: Vec<MapPath>,
    pub(crate) dormant: Vec<MapPath>,
    splitter_ids: Vec<i64>,
    spawner: Option<crate::map_routes::SpawnerDef>,
    reversed: bool,
    /// Model parts newly hidden; the renderer takes them.
    pub hide_parts: Vec<usize>,
    /// Lives leaked since the last step's check (a leak happens while its bloon is borrowed).
    pub lives_lost: f64,
    /// RotateAreaAction per action id: (area, pivot, rotationPerFrame (radians), durationFrames,
    /// spinList (start frames), currRotation).
    spins: Vec<(i64, i64, [f32; 2], f32, i64, Vec<i64>, f32)>,
    /// While logic_start runs: each caught-up event's trigger count.
    catch_up: Option<Vec<(i64, i32)>>,
    /// CycleAreasAction.lastRotationIndex per action (keyed by its first area id).
    cycle_last: Vec<(i64, i32)>,
    /// While logic_round_end runs.
    in_round_end: bool,
}

impl LogicState {
    pub fn new(map: &crate::data::MapData, paths: Vec<MapPath>, splitter_ids: Vec<i64>, reversed: bool) -> Self {
        let def = map.logic.clone().unwrap_or_default();
        LogicState {
            removeables: def.removeables.iter().map(|r| RemoveableState { active: r.active, sold: false }).collect(),
            events_active: def.events.iter().map(|e| e.active).collect(),
            triggered: def.events.iter().map(|e| vec![false; e.triggers.len()]).collect(),
            round_skip: Vec::new(),
            spins: Vec::new(),
            catch_up: None,
            cycle_last: Vec::new(),
            in_round_end: false,
            pool: map.areas.clone(),
            paths,
            dormant: map.paths.iter().filter(|p| !p.active && p.points.len() > 1).cloned().collect(),
            splitter_ids,
            spawner: map.spawner.clone(),
            reversed,
            hide_parts: Vec::new(),
            lives_lost: 0.0,
            def,
        }
    }

    /// AfterRoundTrigger.TriggerCond for round index `r` with triggerOnceIfRoundMissed
    /// (VERIFIED-DECOMP @0x180f7a5b0): once the skip has fired, a count of 0 always fires; before
    /// it, an index past `round` fires and sets it; otherwise the plain condition.
    fn after_round_cond(r: i32, round: i32, nth: i32, primes: bool, missed: bool, skip: &mut (bool, i32)) -> bool {
        if missed {
            if skip.0 {
                if skip.1 == 0 {
                    return true;
                }
            } else if r > round {
                skip.0 = true;
                return true;
            }
        }
        LogicState::after_round(r, round, nth, primes)
    }

    /// AfterRoundTrigger.GetTriggerCount's count for round index `r` (CheckIfTriggered @0x180f79f20).
    fn after_round_count(r: i32, round: i32, nth: i32, primes: bool) -> i32 {
        if primes {
            return (1..=r + 1).filter(|&k| LogicState::after_round(k, i32::MAX, 0, true)).count() as i32;
        }
        if round > r {
            return 0;
        }
        if nth > 0 { (r - round) / nth + 1 } else { 1 }
    }

    /// AfterRoundTrigger.TriggerCond for round index `r` (without triggerOnceIfRoundMissed).
    fn after_round(r: i32, round: i32, nth: i32, primes: bool) -> bool {
        if r == round {
            return true;
        }
        if r > round && nth > 0 {
            return (r - round) % nth == 0;
        }
        primes && r >= 2 && (2..r).take_while(|d| d * d <= r).all(|d| r % d != 0)
    }
}

impl Game {
    /// StartMapTrigger and the catch-up of AfterRoundTrigger.CheckIfTriggered at a later start round.
    pub(super) fn logic_start(&mut self) {
        let round = self.round as i32;
        // CheckIfTriggered: TriggerCond(0) || TriggerCond(1), then the trigger count.
        let hits = self.logic_after_round_hits(|r, at, nth, primes, missed, skip| {
            let hit = LogicState::after_round_cond(r - 1, at, nth, primes, missed, skip)
                || LogicState::after_round_cond(r, at, nth, primes, missed, skip);
            if hit {
                skip.1 = LogicState::after_round_count(r - 1, at, nth, primes);
            }
            hit
        }, round);
        // (A RotateAreaAction caught up here turns at once: GotoPostAction with the trigger count.)
        self.logic.catch_up = Some(hits.iter().map(|&(e, i)| (self.logic.def.events[e].id, self.logic.round_skip[e][i].1)).collect());
        self.logic_trigger_at(|e, i, t| matches!(t, Trigger::StartMap) || hits.contains(&(e, i)));
        self.logic.catch_up = None;
    }

    /// The (event, trigger) pairs of AfterRound triggers `cond` passes for round `r`.
    fn logic_after_round_hits(&mut self, mut cond: impl FnMut(i32, i32, i32, bool, bool, &mut (bool, i32)) -> bool, r: i32) -> Vec<(usize, usize)> {
        let l = &mut self.logic;
        if l.round_skip.len() != l.def.events.len() {
            l.round_skip = l.def.events.iter().map(|e| vec![(false, 0); e.triggers.len()]).collect();
        }
        let mut hits = Vec::new();
        for (e, ev) in l.def.events.iter().enumerate() {
            for (i, t) in ev.triggers.iter().enumerate() {
                if let Trigger::AfterRound { round: at, nth, primes, missed } = t {
                    if cond(r, *at, *nth, *primes, *missed, &mut l.round_skip[e][i]) {
                        hits.push((e, i));
                    }
                }
            }
        }
        hits
    }

    /// AfterRoundTrigger.OnRoundEnd: TriggerCond(1) is the round index that just finished.
    pub(super) fn logic_round_end(&mut self) {
        let round = self.round as i32;
        let hits = self.logic_after_round_hits(|r, at, nth, primes, missed, skip| LogicState::after_round_cond(r, at, nth, primes, missed, skip), round);
        self.logic.in_round_end = true;
        self.logic_trigger_at(|e, i, _| hits.contains(&(e, i)));
        self.logic.in_round_end = false;
    }

    /// AfterRoundTrigger.OnRoundSet (VERIFIED-DECOMP @0x180f7a500): the round set (a later start,
    /// sandbox Set Round) checks TriggerCond(1), the round index now (recording
    /// map_coveredgarden_dart_205_r40: the game set to round 40 has cycled its windows to round
    /// index 39's).
    /// (A RotateAreaAction turns at once here too, to the trigger count: recording
    /// map_geared_dart_205_r40, no spin before round 40's end.)
    pub(crate) fn logic_round_set(&mut self) {
        let round = self.round as i32;
        let hits = self.logic_after_round_hits(|r, at, nth, primes, missed, skip| LogicState::after_round_cond(r, at, nth, primes, missed, skip), round);
        let counts: Vec<(i64, i32)> = hits
            .iter()
            .filter_map(|&(e, i)| match self.logic.def.events[e].triggers[i] {
                Trigger::AfterRound { round: at, nth, primes, .. } => Some((self.logic.def.events[e].id, LogicState::after_round_count(round, at, nth, primes))),
                _ => None,
            })
            .collect();
        self.logic.catch_up = Some(counts);
        self.logic_trigger_at(|e, i, _| hits.contains(&(e, i)));
        self.logic.catch_up = None;
    }

    /// StartRoundTrigger.OnRoundStart (TriggerCond): `round` is the round starting, counted from the
    /// game's start round with countFromStartRound.
    pub(super) fn logic_round_start(&mut self, start_round: i32) {
        let current = self.round as i32;
        self.logic_trigger(|t| matches!(t, Trigger::StartRound { round, nth, primes, from_start }
            if LogicState::after_round(current - if *from_start { start_round } else { 0 }, *round, *nth, *primes)));
    }

    /// SimTimeElapsedTrigger.Process, each frame. `play` and `round_time` are the frame counts it
    /// reads. STAND-IN: counts from the game's start and doesn't restart after firing, so an event
    /// that repeats fires once per frame only if it isn't `once`; RepeatableTrigger isn't modelled.
    pub(super) fn logic_sim_time(&mut self, play: u32, round_time: u32) {
        if self.logic.def.events.is_empty() {
            return;
        }
        self.logic_spin();
        let lost = std::mem::take(&mut self.logic.lives_lost);
        if lost > 0.0 {
            self.logic_lives_lost(lost, self.start_lives);
        }
        self.logic_trigger(|t| matches!(t, Trigger::SimTime { frames, round_time: rt } if if *rt { round_time } else { play } >= *frames));
    }

    /// The map's events have a RotateAreaAction (towers can move with an area).
    pub(crate) fn logic_turns_areas(&self) -> bool {
        self.logic.def.events.iter().any(|e| e.actions.iter().any(|a| matches!(a, Action::RotateArea { .. })))
    }

    /// RotateAreaAction.Process (VERIFIED-DECOMP @0x180f8d720), each frame: spins that have run
    /// durationFrames end; the rest turn the area by their count x rotationPerFrame
    /// (Map.RotateArea @0x180f5e130: the area's outline and holes about its pivot, then each
    /// tower on it: Tower.RotateAroundPivot @0x180c159d0, RotateTower adds the angle in degrees
    /// to its rotation, Vector2.RotateAroundPoint @0x180f45c20 turns its position by -angle).
    /// STAND-IN: a tower is on the area when the area is the first holding its centre; pets,
    /// grounded air units, the tower's own created areas and the catch-up at a later start round
    /// (GotoPostAction) aren't modelled.
    fn logic_spin(&mut self) {
        let now = self.steps as i64;
        for k in 0..self.logic.spins.len() {
            let (_, area, pivot, per, dur, _, _) = self.logic.spins[k];
            self.logic.spins[k].5.retain(|&s| dur > now - s);
            let n = self.logic.spins[k].5.iter().filter(|&&s| s <= now).count();
            if n == 0 {
                continue;
            }
            let a = n as f32 * per;
            self.logic.spins[k].6 += a;
            self.logic_turn_area(area, pivot, a, true);
        }
    }

    /// Map.RotateArea (@0x180f5e130): area `area`'s outline and holes, then (with `towers`) the
    /// towers on it, turned by `a` radians about `pivot`.
    fn logic_turn_area(&mut self, area: i64, pivot: [f32; 2], a: f32, towers: bool) {
        {
            // In the game's frame (y up): x' = dx cos(-a) - dy sin(-a) + px, y' = dx sin(-a) + dy cos(-a) + py.
            let (sn, cs) = crt_sin_cos(-a);
            let pv = [pivot[0], -pivot[1]];
            let turn = |p: [f32; 2]| -> [f32; 2] {
                let (dx, dy) = (p[0] - pv[0], -p[1] - pv[1]);
                [dx * cs - dy * sn + pv[0], -(dy * cs + dx * sn + pv[1])]
            };
            let on: Vec<usize> = (0..self.towers.len())
                // (Map.areaTowers holds sub-towers too: recording map_geared_engineer_204_r40, the
                // Sentries on the gear turn with it.)
                .filter(|_| towers)
                // (The clone's base land, id 0, is first in its list; the game's BaseArea is last.)
                .filter(|&t| self.areas.iter().find(|ar| ar.id != 0 && Self::in_area(self.towers[t].pos, ar)).is_some_and(|ar| ar.id == area))
                .collect();
            for ar in self.areas.iter_mut().chain(self.logic.pool.iter_mut()).filter(|ar| ar.id == area) {
                ar.points = ar.points.iter().map(|&p| turn(p)).collect();
                ar.holes = ar.holes.iter().map(|h| h.iter().map(|&p| turn(p)).collect()).collect();
            }
            let deg = a * (180.0 / std::f32::consts::PI);
            for t in on {
                self.towers[t].pos = turn(self.towers[t].pos);
                self.towers[t].game_rot = Some(self.towers[t].game_rot.unwrap_or(0.0) + deg);
            }
        }
    }

    /// LivesLowerThanTrigger.OnLivesLost: the health went from above the limit to at or below it.
    pub(super) fn logic_lives_lost(&mut self, lost: f64, starting: f64) {
        let health = self.lives;
        self.logic_trigger(|t| match t {
            Trigger::LivesLower { lives, percent } => {
                let limit = percent.map_or(*lives as f64, |p| p * starting);
                health <= limit && health + lost > limit
            }
            _ => false,
        });
    }

    /// PreBloonSpawnTrigger: before Spawner.Emit routes this bloon.
    pub(super) fn logic_pre_spawn(&mut self, bloon: usize) {
        if self.logic.def.events.is_empty() {
            return;
        }
        let tags = self.defs[bloon].tags.clone();
        self.logic_trigger(|t| matches!(t, Trigger::PreBloonSpawn { tag } if tags.contains(tag)));
    }

    /// Sets the triggers `hit` selects, then runs each active event that is now triggered
    /// (MapEvent.TriggerTriggered, IsTriggered: any trigger, or all with allTriggersRequired).
    fn logic_trigger(&mut self, hit: impl Fn(&Trigger) -> bool) {
        self.logic_trigger_at(|_, _, t| hit(t));
    }

    fn logic_trigger_at(&mut self, hit: impl Fn(usize, usize, &Trigger) -> bool) {
        for e in 0..self.logic.def.events.len() {
            let mut any = false;
            for (i, t) in self.logic.def.events[e].triggers.iter().enumerate() {
                if hit(e, i, t) {
                    self.logic.triggered[e][i] = true;
                    any = true;
                }
            }
            if any {
                self.logic_check_event(e);
            }
        }
    }

    fn logic_check_event(&mut self, e: usize) {
        if !self.logic.events_active[e] {
            return;
        }
        let def = &self.logic.def.events[e];
        let flags = &self.logic.triggered[e];
        let fire = if def.all { flags.iter().all(|&f| f) } else { flags.iter().any(|&f| f) };
        if !fire {
            return;
        }
        let actions = def.actions.clone();
        // PostTrigger: a repeating event waits for its triggers again; oneTimeEvent ends.
        if def.once {
            self.logic.events_active[e] = false;
        }
        self.logic.triggered[e].iter_mut().for_each(|f| *f = false);
        for a in actions {
            self.logic_action(a);
        }
    }

    fn logic_action(&mut self, action: Action) {
        match action {
            Action::Other => {}
            Action::RotateArea { id, area, rotation, duration, pivot } => {
                // Initialise (VERIFIED-DECOMP @0x180f8d390): rotationPerFrame = rotation / duration
                // x 1/60, durationFrames = (int)(duration x 60); the model's rotation is radians the
                // other way round (Vector2.RotateAroundPoint turns by its negation; recording
                // map_geared_dart_205_r40: the tower goes +0.1875 deg a frame round the pivot in the
                // game's frame, its own rotation -0.1875). PerformAction (@0x180f8d600): spinList
                // gets the next frame.
                let k = match self.logic.spins.iter().position(|s| s.0 == id) {
                    Some(k) => k,
                    None => {
                        let rad = -rotation * (std::f32::consts::PI / 180.0);
                        self.logic.spins.push((id, area, pivot, rad / duration * (1.0f32 / 60.0), (duration * 60.0) as i64, Vec::new(), 0.0));
                        self.logic.spins.len() - 1
                    }
                };
                // Caught up at the start: GotoPostAction (@0x180f8d190) turns the area at once to
                // count x rotation (no spin). STAND-IN: the count is the event's first caught-up
                // trigger's; recording map_geared_dart_205_r40 starts its gear elsewhere (the tower
                // 90 deg round from its spot, the outline 225 deg from round 1's), so how the game
                // reaches round 40's state isn't settled; the towers placed after don't turn.
                let ev = self.logic.def.events.iter().find(|e| e.actions.iter().any(|x| matches!(x, Action::RotateArea { id: i, .. } if *i == id))).map(|e| e.id);
                if let Some(n) = self.logic.catch_up.as_ref().and_then(|c| c.iter().find(|x| Some(x.0) == ev)).map(|x| x.1) {
                    let rad = -rotation * (std::f32::consts::PI / 180.0);
                    let a = n as f32 * rad - self.logic.spins[k].6;
                    self.logic.spins[k].6 += a;
                    self.logic_turn_area(area, pivot, a, false);
                    return;
                }
                let at = self.steps as i64 + 1;
                self.logic.spins[k].5.push(at);
            }
            Action::CycleAreas { ids, invert, from_start, per } => {
                // PerformAction (VERIFIED-DECOMP @0x180f7b680): index = GetCurrentRound (+ 1 -
                // startRound with fromStartingRound) / roundsPerRotate; unchanged, nothing; else
                // area (k + index) % n for k = 0.. : k = 0 destroyed with invert (else made if
                // missing), the rest made if missing (else destroyed). INFERRED: the areas run the
                // other way round from the scene's list, (k - index) % n (recording
                // map_coveredgarden_dart_205_r40: round 40 starts without WindowUpperLeft, list
                // index 1, and round 41 drops WindowLowerLeft, index 0, under the tower: its z 200
                // -> 0 in the probe's RT lines).
                if ids.is_empty() {
                    return;
                }
                // (At a round's end GetCurrentRound is already the next round's index.)
                let cur = self.round as i32 + self.logic.in_round_end as i32;
                let r = cur + if from_start { 1 - self.start_round as i32 } else { 0 };
                let idx = r / per.max(1);
                let key = ids[0];
                match self.logic.cycle_last.iter_mut().find(|c| c.0 == key) {
                    Some(c) if c.1 == idx => return,
                    Some(c) => c.1 = idx,
                    None => self.logic.cycle_last.push((key, idx)),
                }
                let n = ids.len() as i32;
                for k in 0..n {
                    let id = ids[(k - idx).rem_euclid(n) as usize];
                    if (k == 0) == invert {
                        self.logic_destroy_area(id);
                    } else {
                        self.logic_create_area(id);
                    }
                }
            }
            Action::Path { id, active } => {
                if active {
                    self.logic_activate_path(id);
                }
            }
            Action::Area { ids, create } => {
                for id in ids {
                    if create {
                        self.logic_create_area(id);
                    } else {
                        self.logic_destroy_area(id);
                    }
                }
            }
            Action::Removeable { id, active } => {
                if let Some(i) = self.logic.def.removeables.iter().position(|r| r.id == id) {
                    self.logic.removeables[i].active = active;
                }
            }
            Action::Sell { id } => {
                if let Some(i) = self.logic.def.removeables.iter().position(|r| r.id == id) {
                    self.logic_destroy_removeable(i);
                }
            }
            Action::Event { id, active } => {
                if let Some(e) = self.logic.def.events.iter().position(|ev| ev.id == id) {
                    self.logic.events_active[e] = active;
                    if active {
                        self.logic_check_event(e);
                    }
                }
            }
        }
    }

    /// The active, unsold removeable whose areas contain `p`.
    pub fn removeable_at(&self, p: [f32; 2]) -> Option<usize> {
        let l = &self.logic;
        l.def.removeables.iter().enumerate().find_map(|(i, r)| {
            let live = l.removeables[i].active && !l.removeables[i].sold;
            (live && l.pool.iter().any(|a| r.areas.contains(&a.id) && Self::in_area(p, a))).then_some(i)
        })
    }

    pub fn removeable(&self, i: usize) -> Option<&RemoveableDef> {
        self.logic.def.removeables.get(i)
    }

    /// Map.SellRemoveable: false without the cash.
    pub fn sell_removeable(&mut self, i: usize) -> bool {
        let Some(cost) = self.logic.def.removeables.get(i).map(|r| r.cost as f64) else { return false };
        let s = self.logic.removeables[i];
        if !s.active || s.sold || self.cash < cost {
            return false;
        }
        self.cash -= cost;
        self.logic_destroy_removeable(i);
        true
    }

    /// Map.DestroyRemoveable, then OnRemoveableSold.
    fn logic_destroy_removeable(&mut self, i: usize) {
        if self.logic.removeables[i].sold {
            return;
        }
        self.logic.removeables[i].sold = true;
        let def = self.logic.def.removeables[i].clone();
        if def.destroy_area {
            for id in &def.areas {
                self.logic_destroy_area(*id);
            }
        }
        self.logic.hide_parts.extend(def.hide);
        self.logic_trigger(|t| matches!(t, Trigger::RemoveableSold { id } if *id == def.id));
    }

    fn logic_destroy_area(&mut self, id: i64) {
        if let Some(i) = self.areas.iter().position(|a| a.id == id && id != 0) {
            self.areas.remove(i);
            self.logic_rebuild_blockers();
        }
    }

    /// Map.CreateArea -> InsertArea.
    fn logic_create_area(&mut self, id: i64) {
        if self.areas.iter().any(|a| a.id == id) {
            return;
        }
        let Some(mut area) = self.logic.pool.iter().find(|a| a.id == id).cloned() else { return };
        area.is_disabled = false;
        let at = area_insert_index(&self.areas, area.height);
        self.areas.insert(at, area);
        self.logic_rebuild_blockers();
    }

    /// Map.CreateBlockers / DestroyArea: the blockers of the areas on the map now.
    fn logic_rebuild_blockers(&mut self) {
        self.blockers = self.areas.iter().flat_map(|a| a.blockers.iter().map(move |&c| (c, a.is_disabled))).collect();
        self.blocker_grid = blocker_grid(&self.areas);
    }

    /// Map.ChangePathActiveness: the path joins the tracks (existing indices stay) and every
    /// splitter's active paths are worked out again (Splitter.UpdateActivePaths).
    fn logic_activate_path(&mut self, id: i64) {
        let Some(i) = self.logic.dormant.iter().position(|p| p.id == id) else { return };
        let mut p = self.logic.dormant.remove(i);
        if self.logic.reversed {
            (p.spawn, p.leak, p.until_leak) = (None, None, None);
            p.points.reverse();
            std::mem::swap(&mut p.entry, &mut p.exit);
            std::mem::swap(&mut p.entry_splitter, &mut p.exit_splitter);
        }
        let track = Track::with_info(p.points.clone(), &p.dists, &p.speeds, &p.quick);
        // (Its spawn / leak distances and segment tables too: recording map_cargo_dart_205_r40, the
        // round-40 MOAB on the path turned on at the start is held at PathSpecial's spawnDistance
        // 132.49 - radius x 1.25, 106.24, like any path's.)
        self.track_ends.push(Game::path_ends(&p));
        self.track_until.push(p.until_leak);
        self.track_inv.push(Game::path_inv(&p));
        self.track_segs.push(track.segments());
        self.track_rot.push(p.quick_rot.clone());
        self.track_qz.push(p.quick_z.clone());
        self.track_seg_ok.push(Game::path_seg_ok(&p));
        self.track_seg_inv.push(Game::path_seg_invs(&p));
        self.tracks.push(track);
        self.logic.paths.push(p);
        let paths = &self.logic.paths;
        let ids = &mut self.logic.splitter_ids;
        let mut route = |to: &[i64], splitter: i64| -> (Vec<usize>, usize) {
            let list = to.iter().filter_map(|t| paths.iter().position(|q| q.id == *t)).collect();
            let s = ids.iter().position(|x| *x == splitter).unwrap_or_else(|| {
                ids.push(splitter);
                ids.len() - 1
            });
            (list, s)
        };
        self.track_exit = paths.iter().map(|q| route(&q.exit, q.exit_splitter)).collect();
        self.track_entry = paths.iter().map(|q| route(&q.entry, q.entry_splitter)).collect();
        self.routs.resize(ids.len().max(self.routs.len()), 0);
        self.spawn_tracks = (0..paths.len()).filter(|&k| paths[k].entry.is_empty() && !paths[k].name.contains("Boss")).collect();
        if self.spawn_tracks.is_empty() {
            self.spawn_tracks.push(0);
        }
        self.spawn_route = self.logic.spawner.as_ref().filter(|_| !self.logic.reversed).and_then(|s| {
            crate::map_routes::SpawnRoute::new(s, |id| self.logic.paths.iter().position(|t| t.id == id))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(id: i64, height: f32) -> Area {
        Area { id, kind: 3, points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]], holes: Vec::new(), height, render_offset: 0.0, blockers: Vec::new(), is_disabled: false, is_blocker: false }
    }

    fn game_with(logic: LogicDef, areas: Vec<Area>) -> Game {
        let mut g = super::tests_support::game();
        g.logic = LogicState {
            removeables: logic.removeables.iter().map(|r| RemoveableState { active: r.active, sold: false }).collect(),
            events_active: logic.events.iter().map(|e| e.active).collect(),
            triggered: logic.events.iter().map(|e| vec![false; e.triggers.len()]).collect(),
            pool: areas.clone(),
            def: logic,
            ..Default::default()
        };
        g.areas.extend(areas);
        g
    }

    #[test]
    fn selling_takes_the_cash_and_the_areas_and_fires_events() {
        let tree = RemoveableDef { id: 1, name: "Tree".into(), cost: 500, active: true, destroy_area: true, areas: vec![10], hide: vec![3] };
        let event = EventDef { id: 2, once: true, all: false, active: true, triggers: vec![Trigger::RemoveableSold { id: 1 }], actions: vec![Action::Area { ids: vec![11], create: false }] };
        let mut g = game_with(LogicDef { removeables: vec![tree], events: vec![event] }, vec![area(10, 1.0), area(11, 1.0)]);
        assert_eq!(g.removeable_at([5.0, 5.0]), Some(0));
        g.cash = 499.0;
        assert!(!g.sell_removeable(0), "not enough cash");
        assert_eq!(g.areas.iter().filter(|a| a.id != 0).count(), 2);
        g.cash = 700.0;
        assert!(g.sell_removeable(0));
        assert_eq!(g.cash, 200.0);
        assert_eq!(g.areas.iter().filter(|a| a.id != 0).count(), 0, "its area and the event's");
        assert_eq!(g.logic.hide_parts, vec![3]);
        assert_eq!(g.removeable_at([5.0, 5.0]), None);
        assert!(!g.sell_removeable(0), "only once");
    }

    #[test]
    fn after_round_fires_at_the_end_of_that_round_index() {
        let hidden = RemoveableDef { id: 1, name: "Late".into(), cost: 1, active: false, destroy_area: false, areas: vec![10], hide: vec![] };
        let event = EventDef { id: 2, once: true, all: false, active: true, triggers: vec![Trigger::AfterRound { round: 39, nth: 0, primes: false, missed: false }], actions: vec![Action::Removeable { id: 1, active: true }] };
        let mut g = game_with(LogicDef { removeables: vec![hidden], events: vec![event] }, vec![area(10, 1.0)]);
        g.round = 38;
        g.logic_round_end();
        assert_eq!(g.removeable_at([5.0, 5.0]), None);
        g.round = 39;
        g.logic_round_end();
        assert_eq!(g.removeable_at([5.0, 5.0]), Some(0));
    }

    /// triggerOnceIfRoundMissed: a game started past the round fires it at the start (Erosion:
    /// round 40 starts with the round-18 land gone; the game refused a tower there).
    #[test]
    fn missed_after_round_fires_once_at_a_later_start() {
        let mk = |missed| EventDef { id: 2, once: true, all: false, active: true, triggers: vec![Trigger::AfterRound { round: 18, nth: 0, primes: false, missed }], actions: vec![Action::Area { ids: vec![10], create: false }] };
        for missed in [false, true] {
            let mut g = game_with(LogicDef { removeables: vec![], events: vec![mk(missed)] }, vec![area(10, 1.0)]);
            g.round = 40;
            g.logic_start();
            assert_eq!(g.areas.iter().any(|a| a.id == 10), !missed, "missed {missed}");
        }
    }

    /// Every exported map loads (one that doesn't silently plays game.json's map instead) and its
    /// logic runs: every removeable bought, every round ended, every path turned on with bloons routed.
    #[test]
    fn every_exported_map_loads_and_runs() {
        let maps = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_extracted/maps");
        let Ok(dir) = std::fs::read_dir(&maps) else { return };
        let (mut loaded, mut with_logic) = (0, 0);
        for entry in dir.flatten() {
            let Ok(text) = std::fs::read_to_string(entry.path().join("map.json")) else { continue };
            let name = entry.file_name().to_string_lossy().into_owned();
            let map: crate::data::MapData = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
            loaded += 1;
            if map.logic.is_none() && map.spawner.is_none() {
                continue;
            }
            with_logic += 1;
            let mut g = super::tests_support::game();
            let active: Vec<MapPath> = map.paths.iter().filter(|p| p.active && p.points.len() > 1).cloned().collect();
            let live: Vec<Area> = map.areas.iter().filter(|a| !a.is_disabled).cloned().collect();
            g.tracks = active.iter().map(|p| Track::with_info(p.points.clone(), &p.dists, &p.speeds, &p.quick)).collect();
            g.track_exit = vec![(Vec::new(), 0); active.len()];
            g.track_entry = vec![(Vec::new(), 0); active.len()];
            g.logic = LogicState::new(&map, active, Vec::new(), false);
            g.areas.extend(live);
            g.logic_start();
            g.cash = 1e9;
            for round in 0..200 {
                g.round = round;
                let bloon = round % g.defs.len();
                g.logic_pre_spawn(bloon);
                g.logic_round_end();
                for i in 0..g.logic.def.removeables.len() {
                    g.sell_removeable(i);
                }
                for &to in &g.spawn_tracks {
                    assert!(to < g.tracks.len(), "{name}: spawn track");
                }
                if let Some(route) = &g.spawn_route {
                    for e in 0..4 {
                        let tags = g.defs[bloon].tags.clone();
                        assert!(route.route(e, round as i32, &tags).is_none_or(|t| t < g.tracks.len()), "{name}: route");
                    }
                }
            }
        }
        if loaded > 0 {
            eprintln!("{loaded} maps loaded, {with_logic} with logic or a spawner");
        }
    }

    #[test]
    fn a_newer_exports_kinds_do_not_stop_a_map_loading() {
        let def: LogicDef = serde_json::from_value(serde_json::json!({"removeables":[],"events":[{"id":1,"once":true,"all":false,"active":true,
            "triggers":[{"kind":"SomethingNew","x":1},{"kind":"StartMap"}],"actions":[{"kind":"AlsoNew"}]}]})).unwrap();
        assert_eq!(def.events[0].triggers, vec![Trigger::Other, Trigger::StartMap]);
    }

    #[test]
    fn start_round_and_lives_triggers_fire_at_their_moment() {
        let hidden = |id| RemoveableDef { id, name: "Late".into(), cost: 1, active: false, destroy_area: false, areas: vec![10], hide: vec![] };
        let event = |id, trigger, target| EventDef { id, once: true, all: false, active: true, triggers: vec![trigger], actions: vec![Action::Removeable { id: target, active: true }] };
        let logic = LogicDef { removeables: vec![hidden(1)], events: vec![event(2, Trigger::StartRound { round: 5, nth: 0, primes: false, from_start: false }, 1)] };
        let mut g = game_with(logic, vec![area(10, 1.0)]);
        g.round = 4;
        g.logic_round_start(0);
        assert_eq!(g.removeable_at([5.0, 5.0]), None);
        g.round = 5;
        g.logic_round_start(0);
        assert_eq!(g.removeable_at([5.0, 5.0]), Some(0));

        let logic = LogicDef { removeables: vec![hidden(1)], events: vec![event(2, Trigger::LivesLower { lives: 0, percent: Some(0.5) }, 1)] };
        let mut g = game_with(logic, vec![area(10, 1.0)]);
        g.lives = 60.0;
        g.logic_lives_lost(10.0, 100.0);
        assert_eq!(g.removeable_at([5.0, 5.0]), None);
        g.lives = 50.0;
        g.logic_lives_lost(10.0, 100.0);
        assert_eq!(g.removeable_at([5.0, 5.0]), Some(0));
    }

    #[test]
    fn repeating_after_round_and_primes() {
        assert!(LogicState::after_round(5, 2, 1, false));
        assert!(!LogicState::after_round(4, 2, 3, false));
        assert!(LogicState::after_round(5, 2, 3, false));
        assert!(!LogicState::after_round(1, 2, 0, false));
        assert!(LogicState::after_round(7, 100, 0, true));
        assert!(!LogicState::after_round(9, 100, 0, true));
    }
}
