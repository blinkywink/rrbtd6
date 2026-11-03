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
    AfterRound { round: i32, nth: i32, primes: bool },
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
    /// Every Area of the map: DestroyArea and CreateArea move them out of and into `Game::areas`.
    pool: Vec<Area>,
    /// The active paths' data (parallel to `Game::tracks`), and paths not yet on the map.
    paths: Vec<MapPath>,
    dormant: Vec<MapPath>,
    splitter_ids: Vec<i64>,
    spawner: Option<crate::map_routes::SpawnerDef>,
    reversed: bool,
    /// Model parts newly hidden; the renderer takes them.
    pub hide_parts: Vec<usize>,
    /// Lives leaked since the last step's check (a leak happens while its bloon is borrowed).
    pub lives_lost: f64,
}

impl LogicState {
    pub fn new(map: &crate::data::MapData, paths: Vec<MapPath>, splitter_ids: Vec<i64>, reversed: bool) -> Self {
        let def = map.logic.clone().unwrap_or_default();
        LogicState {
            removeables: def.removeables.iter().map(|r| RemoveableState { active: r.active, sold: false }).collect(),
            events_active: def.events.iter().map(|e| e.active).collect(),
            triggered: def.events.iter().map(|e| vec![false; e.triggers.len()]).collect(),
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

    /// AfterRoundTrigger.TriggerCond for round index `r`.
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
        self.logic_trigger(|t| match t {
            Trigger::StartMap => true,
            Trigger::AfterRound { round: at, nth, primes } => LogicState::after_round(round - 1, *at, *nth, *primes),
            _ => false,
        });
    }

    /// AfterRoundTrigger.OnRoundEnd: TriggerCond(1) is the round index that just finished.
    pub(super) fn logic_round_end(&mut self) {
        let round = self.round as i32;
        self.logic_trigger(|t| matches!(t, Trigger::AfterRound { round: at, nth, primes } if LogicState::after_round(round, *at, *nth, *primes)));
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
        let lost = std::mem::take(&mut self.logic.lives_lost);
        if lost > 0.0 {
            self.logic_lives_lost(lost, self.start_lives);
        }
        self.logic_trigger(|t| matches!(t, Trigger::SimTime { frames, round_time: rt } if if *rt { round_time } else { play } >= *frames));
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
        for e in 0..self.logic.def.events.len() {
            let mut any = false;
            for (i, t) in self.logic.def.events[e].triggers.iter().enumerate() {
                if hit(t) {
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
            p.points.reverse();
            std::mem::swap(&mut p.entry, &mut p.exit);
            std::mem::swap(&mut p.entry_splitter, &mut p.exit_splitter);
        }
        self.tracks.push(Track::with_info(p.points.clone(), &p.dists, &p.speeds, &p.quick));
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
        Area { id, kind: 3, points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]], holes: Vec::new(), height, blockers: Vec::new(), is_disabled: false, is_blocker: false }
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
        let event = EventDef { id: 2, once: true, all: false, active: true, triggers: vec![Trigger::AfterRound { round: 39, nth: 0, primes: false }], actions: vec![Action::Removeable { id: 1, active: true }] };
        let mut g = game_with(LogicDef { removeables: vec![hidden], events: vec![event] }, vec![area(10, 1.0)]);
        g.round = 38;
        g.logic_round_end();
        assert_eq!(g.removeable_at([5.0, 5.0]), None);
        g.round = 39;
        g.logic_round_end();
        assert_eq!(g.removeable_at([5.0, 5.0]), Some(0));
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
