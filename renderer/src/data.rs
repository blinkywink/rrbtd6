//! `local_extracted/game/game.json`, produced by tools/assets/build_game_data.py
//! from the user's own install. Never committed.

use serde::Deserialize;
use std::collections::HashMap;

/// GameData.bloonMutatorData: BadImmunity.FilterMutation keeps a mutator off a BAD when its id
/// starts with one of `bad_immunity_ids` or contains one of `bad_immunity_tags`.
#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct BloonMutatorData {
    pub bad_immunity_ids: Vec<String>,
    pub bad_immunity_tags: Vec<String>,
}

/// A BloonOverlayScriptable: its displayLayer and its prefab per BloonOverlayClass name.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct OverlayDef {
    #[serde(default)]
    pub layer: i32,
    #[serde(default)]
    pub assets: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct GameData {
    #[serde(default)]
    pub mod_scripts: Vec<crate::mod_runtime::Definition>,
    #[serde(default)]
    pub mod_maps: HashMap<String,crate::mod_maps::Definition>,
    /// CollisionGrid.orderedNeighborOffsets: (dx, dy) cell offsets in the order a collision
    /// query visits them (from a SimRecorder dump; empty: storage order).
    #[serde(default)]
    pub grid_offsets: Vec<i32>,
    /// Bloon mutators' IncreaseDamageFromTypeModel damage types by mutator id (Silas's Rime): its
    /// amount (the effect's `bonus`, per model) more on a hit whose immunity mask is one of `masks`.
    #[serde(default)]
    pub effect_type_bonus: HashMap<String, TypeBonus>,
    pub start: Start,
    pub bloons: HashMap<String, BloonDef>,
    pub towers: HashMap<String, TowerDef>,
    #[serde(default)]
    pub tower_tiers: HashMap<String, TowerTiers>,
    pub rounds: Vec<Vec<Group>>,
    /// Other round sets by name (RoundSetModModel: "AlternateRoundSet").
    #[serde(default)]
    pub round_sets: HashMap<String, Vec<Vec<Group>>>,
    /// GameModel's IncomeSetModel (DefaultIncomeSet): (threshold, multiplier) pairs and
    /// finalMultiplier, for Simulation.GetIncomeMultiplierForRound.
    #[serde(default)]
    pub income_set: Option<IncomeSet>,
    /// GameData.bloonMutatorData.
    #[serde(default)]
    pub bloon_mutator_data: BloonMutatorData,
    /// GameData.bloonOverlayData.overlayTypes (tools/assets/export_overlays.py): each mutator
    /// overlayType's displayLayer and prefab per BloonOverlayClass name.
    #[serde(default)]
    pub overlays: HashMap<String, OverlayDef>,
    /// GameModel.freeplayGroups: rounds past the round set are built from these.
    #[serde(default)]
    pub freeplay: Vec<FreeplayGroup>,
    /// Geraldo's shop items (GeraldoItemModel).
    #[serde(default)]
    pub geraldo: Vec<GeraldoItem>,
    /// The projectile a Power puts down, by Power id (Cash Drop's crate).
    #[serde(default)]
    pub power_projectiles: HashMap<String, Proj>,
    /// The tower a Power places, by Power id.
    #[serde(default)]
    pub power_towers: HashMap<String, PowerTower>,
    /// EndRoundModModel of the difficulty (Medium: 60).
    #[serde(default)]
    pub end_round: Option<usize>,
    /// In-game tower shop order (hero first).
    #[serde(default)]
    pub shop: Vec<ShopItem>,
    #[serde(default = "one")]
    pub round_scale: f32,
    pub map: MapData,
    /// Difficulties by name (Easy, Medium, Hard, Impoppable, CHIMPS).
    #[serde(default)]
    pub modes: HashMap<String, Mode>,
    /// The chosen one (set by the caller; Medium if unknown).
    #[serde(default)]
    pub mode: String,
    /// The map's MapModel.mapDifficulty (Beginner 0 .. Expert 3; set by the caller from
    /// mapset.json). None for the clone's mod maps, which earn no XP, as the game's creation-mode
    /// maps don't (ReadonlyInGameData.CanEarnPlayerAndTowerXp).
    #[serde(default)]
    pub map_difficulty: Option<usize>,
    /// ParagonDegreeDataModel.
    #[serde(default)]
    pub paragon_degree: Option<ParagonDegree>,
    /// GameModel.buffIndicatorModels' FullNames (iconName + buffName) in order: the buff icons'
    /// slots under a tower (BuffIndicatorUi.Awake).
    #[serde(default)]
    pub buff_order: Vec<String>,
    /// All heroes' shop entries; the chosen one replaces the shop's hero slot.
    #[serde(default)]
    pub heroes: Vec<ShopItem>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ShopItem {
    pub id: String,
    pub cost: f64,
    /// TowerSet flags: 1 Primary, 2 Military, 4 Magic, 8 Support, 16 Hero.
    pub set: u32,
    pub hero: bool,
    pub icon: Option<String>,
}

#[derive(Deserialize)]
pub struct Start {
    pub cash: f64,
    pub lives: f64,
}

/// A difficulty's ModModels (data/modes.json).
#[derive(Deserialize, Clone)]
pub struct Mode {
    pub cash: f64,
    pub lives: f64,
    pub sell: f64,
    /// GlobalCostModModel.multiplier: TowerModel / UpgradeModel costs + (mult - 1) x cost.
    pub cost_mult: f32,
    /// GlobalSpeedModModel: every bloon's speed x mult + add.
    pub speed_mult: f32,
    pub speed_add: f32,
    pub start_round: usize,
    pub end_round: usize,
    /// BonusCashPerRoundModModel (baseCash, roundMultiple); CHIMPS keeps Hard's.
    pub bonus: Option<[f64; 2]>,
    #[serde(default)]
    pub locked: Vec<String>,
    #[serde(default)]
    pub no_sell: bool,
    #[serde(default)]
    pub health: Vec<HealthMod>,
    /// Difficulty (Easy / Medium / Hard) and sub-mode (None: Standard) this mode combines.
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub sub: Option<String>,
    /// LockTowerSetModModel: tower sets (1 primary, 2 military, 4 magic, 8 support) that can't
    /// be bought.
    #[serde(default)]
    pub locked_sets: Vec<u32>,
    /// ModifyAllCashModModel: every income x this (Half Cash 0.5, Deflation 0).
    #[serde(default = "one_f64")]
    pub cash_mult: f64,
    /// BloonHealthModel on the "Moabs" tag (Double HP MOABs: 2).
    #[serde(default = "one")]
    pub moab_health: f32,
    /// RoundSetModModel: the round set played (Alternate Bloons Rounds).
    #[serde(default)]
    pub round_set: Option<String>,
    #[serde(default)]
    pub sandbox: bool,
    #[serde(default)]
    pub reverse: bool,
    #[serde(default)]
    pub apopalypse: bool,
    #[serde(default)]
    pub deflation: bool,
}

fn one_f64() -> f64 {
    1.0
}

/// SetHealthForBloonModModel: bloons of this id that round spawns get maxHealth x mult.
#[derive(Deserialize, Clone)]
pub struct HealthMod {
    pub round: usize,
    pub bloon: String,
    pub mult: f32,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BloonDef {
    #[serde(default)]
    pub modded:bool,
    /// BloonModel.baseId ("Pink" for every Pink variant).
    #[serde(default, rename = "baseId")]
    pub base_id: String,
    /// CollisionGroupData (MOAB-class): (offset x, offset y along the heading, radius), game frame.
    #[serde(default)]
    pub circles: Vec<[f32; 3]>,
    /// BloonModel.rotateToFollowPath: turns towards the path's heading as it moves.
    #[serde(default)]
    pub rotate_to_follow_path: bool,
    /// BloonModel.overlayClass's name ("White", "WhiteRegrow", "Moab"...; empty: none): with a
    /// mutator's overlayType, the overlay prefab shown on it.
    #[serde(default)]
    pub overlay_class: String,
    /// BloonModel.distributeDamageToChildren.
    #[serde(default)]
    pub distribute_damage_to_children: bool,
    /// BloonModel.danger (Strong targeting).
    #[serde(default)]
    pub danger: f32,
    pub speed: f32,
    pub max_health: i32,
    pub radius: f32,
    pub leak_damage: f32,
    pub children: Vec<String>,
    pub sprite: Option<String>,
    #[serde(default)]
    pub display: Vec<DisplayLayer>,
    #[serde(default)]
    pub is_camo: bool,
    /// `BloonModel.bloonProperties` (Lead = 1, Black = 2, White = 4, Purple = 8, Frozen = 16).
    #[serde(default)]
    pub bloon_properties: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    /// BloonModel.layerNumber (Red 1 ... Ceramic 9, MOABs 10+).
    #[serde(default)]
    pub layer_number: u32,
    /// BloonModel.isMoab.
    #[serde(default)]
    pub is_moab: bool,
    /// BloonModel.isBoss.
    #[serde(default)]
    pub is_boss: bool,
    /// BadImmunityModel (BAD).
    #[serde(default)]
    pub bad_immunity: bool,
    /// Place in GameModel.bloons (ProgressiveDifficultyManager walks the models in this order).
    #[serde(default)]
    pub index: u32,
    /// BloonModel.isFixedMaxHealth (Diamondback segments).
    #[serde(default)]
    pub is_fixed_max_health: bool,
    /// GrowModel: regrow bloons turn into `to` every `rate` seconds.
    #[serde(default)]
    pub grow: Option<Grow>,
    /// PopEffectModel sounds: one of `names` when a layer pops, limited per sound group.
    #[serde(default, rename = "pop_sound")]
    pub pop_sound: Option<PopSound>,
    /// CreateSoundOnDamageBloonModel: one of these when it takes damage without popping.
    #[serde(default, rename = "hit_sounds")]
    pub hit_sounds: Vec<String>,
    /// 3D display prefab (MOAB class), exported to models/bloons/<model>.
    #[serde(default)]
    pub model: Option<String>,
    /// DamageStateModels: `model` replaces the display once health is at or below `health`.
    #[serde(default, rename = "damage_models")]
    pub damage_models: Vec<DamageModel>,
}

#[derive(Deserialize, Clone)]
pub struct DamageModel {
    pub model: String,
    pub health: f32,
}

impl BloonDef {
    /// The 3D display for a bloon at `frac` of its health: the most damaged state reached.
    pub fn model_at(&self, frac: f32) -> Option<&str> {
        let state = self.damage_models.iter().filter(|d| frac <= d.health).min_by(|a, b| a.health.total_cmp(&b.health));
        state.map(|d| d.model.as_str()).or(self.model.as_deref())
    }
}

/// PopEffectModel sound fields. `limit`: soundEffectGroupLimit; `frames` / `frames_ff`:
/// soundEffectTimeDelay at normal / fast-forward speed.
#[derive(Deserialize, Clone)]
pub struct PopSound {
    pub names: Vec<String>,
    pub group: String,
    pub limit: u32,
    pub frames: u32,
    pub frames_ff: u32,
}

/// CreateSoundOnTowerPlaceModel / CreateSoundOnUpgradeModel / CreateSoundOnSellModel clips.
#[derive(Deserialize, Clone, Default)]
pub struct TowerSounds {
    #[serde(default)]
    pub place: Vec<String>,
    #[serde(default)]
    pub upgrade: Vec<String>,
    #[serde(default)]
    pub sell: Vec<String>,
    /// Hero voice lines (build_game_data.py): placed, a level (8 slots, "" for none), selected
    /// (and the alternates), a leak, a MOAB-class bloon popped, and a warning per MOAB class
    /// entering ("moab", "bfb", "zomg", "ddt", "bad").
    /// CreateSoundOnAttached: played when a tower reaches this tier; CreateSoundOnEndOfRound.
    #[serde(default)]
    pub attached: Vec<String>,
    #[serde(default)]
    pub round_end: Vec<String>,
    #[serde(default)]
    pub hero_place: Vec<String>,
    #[serde(default)]
    pub hero_upgrade: Vec<String>,
    #[serde(default)]
    pub select: Vec<String>,
    #[serde(default)]
    pub select_alt: Vec<String>,
    #[serde(default)]
    pub leak: Vec<String>,
    #[serde(default)]
    pub moab_pop: Vec<String>,
    #[serde(default)]
    pub enter: HashMap<String, String>,
}

/// One SpriteRenderer of the bloon's display prefab (size from SpriteRenderer.m_Size).
#[derive(Deserialize, Clone)]
pub struct DisplayLayer {
    pub png: String,
    pub w: f32,
    pub h: f32,
    /// Where the tight art's centre sits from the sprite rect's pivot (sprite plane: x, y up).
    #[serde(default)]
    pub trim: [f32; 2],
}

fn minus_one() -> f32 {
    -1.0
}

fn one() -> f32 {
    1.0
}

#[derive(Deserialize, Clone)]
pub struct TowerDef {
    /// Shared visual model of a data-authored custom tower.
    #[serde(default)]
    pub model:Option<String>,
    /// TowerModel.areaTypes: where it can be placed (1 water, 2 land).
    #[serde(default)]
    pub areas: Vec<i32>,
    /// TowerModel.footprint (placement overlap).
    #[serde(default)]
    pub foot: Option<Footprint>,
    pub cost: f64,
    pub range: f32,
    pub radius: f32,
    pub icon: String,
    /// TowerModel.isGlobalRange (Sniper Monkey): no range circle.
    #[serde(default)]
    pub global: bool,
    /// CreateEffectOnPlaceModel: (effect prefab, scale) played where it's placed.
    #[serde(default)]
    pub place_fx: Option<(String, f32)>,
}

#[derive(Deserialize, Clone)]
pub struct Group {
    pub bloon: String,
    pub count: u32,
    pub start: f32,
    pub duration: f32,
}

/// GrowModel.growToId / rate.
#[derive(Deserialize, Clone)]
pub struct Grow {
    pub to: String,
    pub rate: f32,
    /// GrowModel.overrideChildWithColor: on a bloon of this colour later, grow into its parent's.
    #[serde(default)]
    pub child_color: Option<String>,
}

/// GeraldoItemModel: price, stock rules, the hero level that unlocks it, and what it does
/// (`kind`: tower / track / pickles / stone / potion / fertilizer / cape / sauce / rabbit / rejuv).
/// `towers` / `projectiles`: (from Geraldo level, what it places).
#[derive(Deserialize, Clone)]
pub struct GeraldoItem {
    pub name: String,
    pub loc: String,
    pub cost: f64,
    pub start: u32,
    pub max: u32,
    pub every: u32,
    pub amount: u32,
    pub unlock: u32,
    #[serde(default)]
    pub only_if_none: bool,
    #[serde(default)]
    pub banned: Vec<String>,
    #[serde(default)]
    pub block_if_placed: String,
    pub kind: String,
    #[serde(default)]
    pub towers: Vec<(u32, std::sync::Arc<SubTowerSpec>)>,
    #[serde(default)]
    pub projectiles: Vec<(u32, Proj)>,
    #[serde(default)]
    pub params: serde_json::Map<String, serde_json::Value>,
    /// PetRabbitMergeModel: this many of the item's towers merge into this one.
    #[serde(default)]
    pub merge: Option<(u32, std::sync::Arc<SubTowerSpec>)>,
}

/// FreeplayBloonGroupModel: a bloon group, the rounds it may appear in (inclusive bounds) and
/// its score (CalculateScore).
#[derive(Deserialize, Clone)]
pub struct FreeplayGroup {
    pub bloon: String,
    pub count: u32,
    pub start: f32,
    pub duration: f32,
    pub bounds: Vec<[i32; 2]>,
    pub score: f64,
}

#[derive(Deserialize)]
pub struct MapData {
    pub path: Vec<[f32; 2]>,
    /// The BaseArea's AreaType (from the SimRecorder map dump; land when absent).
    #[serde(default)]
    pub base_type: Option<i32>,
    /// Every track (one per entrySplitter) with its active flag; empty in older exports.
    #[serde(default)]
    pub paths: Vec<MapPath>,
    pub areas: Vec<Area>,
    /// The terrain mesh's extent; some maps' exports have none (their ground is part of the
    /// scene model).
    #[serde(default)]
    pub terrain: Option<Terrain>,
    /// The spawner's splitter, when it isn't a plain one (map_routes.rs).
    #[serde(default)]
    pub spawner: Option<crate::map_routes::SpawnerDef>,
    /// Removeables and map events (map_logic.rs).
    #[serde(default)]
    pub logic: Option<crate::game::map_logic::LogicDef>,
    /// MapMods.bloonSpeed (MapModel / Map.mapWideBloonSpeed): every bloon's speed multiplier on
    /// the map (Workshop 1.5).
    #[serde(default = "one_f32")]
    pub bloon_speed: f32,
}

#[derive(Deserialize, Clone)]
pub struct ParagonDef {
    /// TowerModel.portrait: the upgrade panel's portrait sprite.
    #[serde(default)]
    pub portrait: String,
    /// The Paragon UpgradeModel.icon.
    #[serde(default)]
    pub icon: String,
    /// ParagonAssetSwapModel: (display, its replacement, display degree, applyToAllAbove).
    #[serde(default)]
    pub swaps: Vec<(String, String, usize, bool)>,
    /// ParagonAssetSwapPerDegreeModel: a display's name per display degree (first = the original).
    #[serde(default)]
    pub per_degree: Vec<Vec<String>>,
    /// ParagonTowerModel.displayDegreePaths count: display levels.
    #[serde(default = "one_usize")]
    pub levels: usize,
    pub tier: Tier,
    pub cost: f64,
    pub name: String,
    /// ParagonTowerModel.inactiveDurationFrames.
    #[serde(default)]
    pub inactive_frames: u64,
    /// ParagonTowerModel.endAnimationState (-1: none).
    #[serde(default = "minus_one_i32")]
    pub end_anim: i32,
    #[serde(default)]
    pub sacrifice: Option<SacrificeFx>,
}

/// ParagonDegreeDataModel fields (camelCase as in the game).
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ParagonDegree {
    pub attack_cooldown_reduction_x: f32,
    pub pierce_percent_per_degree: f32,
    pub pierce_increase_per_degree: f32,
    pub damage_percent_per_degree: f32,
    pub damage_increase_per_degree: f32,
    pub damage_increase_for_degrees: f32,
    pub degree_count: u32,
    pub max_power_from_pops: f32,
    pub max_power_from_money_spent: f32,
    pub max_power_from_non_tier5_count: f32,
    pub max_power_from_tier5_count: f32,
    pub pops_over_x: f32,
    pub money_spent_over_x: f32,
    pub non_tier5_towers_mult_by_x: f32,
    pub tier5_towers_mult_by_x: f32,
    /// ParagonTower.GetTowerInvestment: cash earned counts this much as damage dealt.
    #[serde(default)]
    pub cash_earned_contribution_modifier: f32,
    /// powerDegreeRequirements: total power needed for each degree (index 0 = degree 1).
    #[serde(rename = "requirements")]
    pub requirements: Vec<f32>,
}

#[derive(Deserialize, Clone)]
pub struct MapPath {
    #[serde(default)]
    pub name: String,
    /// The game's own PathModel point distances and bloonSpeedMultiplier, when imported from a
    /// SimRecorder dump (tools/mod/import_paths.py).
    #[serde(default)]
    pub dists: Vec<f32>,
    #[serde(default)]
    pub speeds: Vec<f32>,
    /// Path.pointInfos: the point every CacheDistance (0.01) along it, x and y in turn (the
    /// game's, when imported).
    #[serde(default)]
    pub quick: Vec<f32>,
    /// QuickPointInfo.rotation per cache step (game frame, degrees; imported).
    #[serde(default)]
    pub quick_rot: Vec<f32>,
    /// Path.pointInfos' point.z per cache step (imported; empty: 0): a bloon's Position.z is it
    /// plus 20 (120 for MOAB-class).
    #[serde(default)]
    pub quick_z: Vec<f32>,
    /// PointInfo.disablePathTargeting per point (imported from the game's map dump).
    #[serde(default)]
    pub no_target: Vec<bool>,
    /// PointInfo.bloonsInvulnerable / moabsInvulnerable per point (from the dump; empty: none):
    /// Bloon.Process sets nonTargetable from the point at its distance.
    #[serde(default)]
    pub invuln: Vec<bool>,
    #[serde(default)]
    pub moab_invuln: Vec<bool>,
    /// Path.isHidden (from the dump): an inactive hidden path has no segments to target.
    #[serde(default)]
    pub hidden: bool,
    /// Path.spawnDistance / leakDistance from the game's map dump (-1: none); without them the
    /// clone works them out from where the path crosses the map's edge.
    #[serde(default)]
    pub spawn: Option<f32>,
    #[serde(default)]
    pub leak: Option<f32>,
    /// Path.maximumDistanceUntilLeak from the dump: leakDist on an exit path, else its length plus
    /// the exit router's paths' (what PathSegment.distanceUntilLeak and First count from).
    #[serde(default)]
    pub until_leak: Option<f32>,
    #[serde(default = "yes")]
    pub active: bool,
    /// The Path component's id; `entry` / `exit` list path ids from its splitters, and
    /// `exit_splitter` is the exit splitter's id (paths sharing it share its round-robin).
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub entry: Vec<i64>,
    #[serde(default)]
    pub exit: Vec<i64>,
    #[serde(default)]
    pub exit_splitter: i64,
    #[serde(default)]
    pub entry_splitter: i64,
    pub points: Vec<[f32; 2]>,
}

/// ProjectileBlockerCollisionReboundModel.
#[derive(Deserialize, Clone, Debug)]
pub struct BlockerRebound {
    /// clearCollidedWith: it can hit the same bloons again after a bounce.
    #[serde(default)]
    pub clear: bool,
}

#[derive(Deserialize, Clone)]
pub struct Area {
    /// The Area script's id, what map events name it by (0: not from a scene).
    #[serde(default)]
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: i32,
    pub points: Vec<[f32; 2]>,
    /// Holes: AreaModel.IsPointInside excludes points inside any of them.
    #[serde(default)]
    pub holes: Vec<Vec<[f32; 2]>>,
    /// `AreaModel.height` (the Area transform's world y).
    #[serde(default)]
    pub height: f32,
    /// `AreaModel.renderHeightOffset` (Map.GetTerrainHeight adds it to the height).
    #[serde(default, rename = "renderHeightOffset")]
    pub render_offset: f32,
    /// `blockerCircles` of blocker areas: (x, z, radius) in our ground frame.
    #[serde(default)]
    pub blockers: Vec<[f32; 3]>,
    /// Copied to each `BlockerModel.isDisabled`.
    #[serde(default, rename = "isDisabled")]
    pub is_disabled: bool,
    /// AreaModel.isBlocker: projectiles that bounce off blockers bounce off its edges.
    #[serde(default, rename = "isBlocker")]
    pub is_blocker: bool,
}

#[derive(Deserialize)]
pub struct Terrain {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

/// `Models.Map.AreaType` (from metadata): track=0, water=1, land=2, unplaceable=3.
pub const AREA_LAND: i32 = 2;
/// Area type that `Map.CanPlace` skips entirely.
pub const AREA_SKIPPED: i32 = 5;

/// The tower a Power places (GameData.power_towers).
#[derive(Deserialize, Clone)]
pub struct PowerTower {
    /// Its TowerModel name (the def's name).
    pub name: String,
    pub tower: TowerDef,
    pub tiers: TowerTiers,
    /// Its TowerModel behaviours the clone doesn't simulate for it yet.
    #[serde(default)]
    pub unmodelled: Vec<String>,
}

/// Every Dart Monkey tier combination ("000".."520") from its decoded TowerModel.
#[derive(Deserialize, Clone)]
pub struct TowerTiers {
    /// Optional author-owned visual overrides, keyed by the native tier state.
    #[serde(default)]
    pub visuals: HashMap<String,String>,
    /// The Paragon: its TowerModel's tier data and its upgrade.
    #[serde(default)]
    pub paragon: Option<ParagonDef>,
    /// (Shared: the sim reads a tower's model every step without deep-cloning it.)
    pub tiers: HashMap<String, std::sync::Arc<Tier>>,
    /// `tiers` by tier triple (packed a << 16 | b << 8 | c, sorted): `by_tiers`, built on first use.
    #[serde(skip)]
    pub packed: std::sync::OnceLock<Vec<(u32, std::sync::Arc<Tier>)>>,
    pub upgrades: Vec<Upgrade>,
    /// Heroes: XP leveling (tiers are [level, 0, 0]).
    #[serde(default)]
    pub hero: Option<HeroDef>,
}

impl TowerTiers {
    /// `tiers[key]` for the key a tower on `t` is filed under (each path's tier in decimal, run
    /// together: [1, 0, 2] is "102", a level-12 hero's [12, 0, 0] "1200"), without hashing a string.
    pub fn by_tiers(&self, t: [u8; 3]) -> Option<&std::sync::Arc<Tier>> {
        let v = self.packed.get_or_init(|| {
            let mut v: Vec<(u32, std::sync::Arc<Tier>)> = Vec::new();
            for (k, tier) in &self.tiers {
                for c in tier_key_splits(k) {
                    v.push(((c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32, tier.clone()));
                }
            }
            v.sort_by_key(|e| e.0);
            v
        });
        let key = (t[0] as u32) << 16 | (t[1] as u32) << 8 | t[2] as u32;
        v.binary_search_by_key(&key, |e| e.0).ok().map(|i| &v[i].1)
    }
}

/// Every triple whose decimal key (no leading zeros) is `k`.
fn tier_key_splits(k: &str) -> Vec<[u8; 3]> {
    let part = |s: &str| -> Option<u8> { if s.is_empty() || (s.len() > 1 && s.starts_with('0')) || !s.bytes().all(|b| b.is_ascii_digit()) { None } else { s.parse().ok() } };
    let mut out = Vec::new();
    for i in 1..k.len() {
        for j in i + 1..k.len() {
            if let (Some(a), Some(b), Some(c)) = (part(&k[..i]), part(&k[i..j]), part(&k[j..])) {
                out.push([a, b, c]);
            }
        }
    }
    out
}

/// `HeroModel.xpScale` / `costPerXpToLevel` and each level's `UpgradeModel.xpCost`.
#[derive(Deserialize, Clone)]
pub struct HeroDef {
    pub xp_scale: f32,
    pub cost_per_xp: f32,
    pub levels: Vec<HeroLevel>,
}

#[derive(Deserialize, Clone)]
pub struct HeroLevel {
    /// The level this upgrade reaches (2..=20).
    pub level: u8,
    pub name: String,
    pub xp: f32,
}

#[derive(Deserialize, Clone)]
pub struct Upgrade {
    pub name: String,
    pub cost: f64,
    pub path: usize,
    pub tier: usize,
}

#[derive(Deserialize, Clone)]
pub struct Tier {
    /// CreateEffectOnAirUnitModel: an effect kept on the tower's air unit (display only).
    #[serde(default)]
    pub air_fx: Option<String>,
    /// PointLineEffectModel: a line display along its line shots (display only).
    #[serde(default)]
    pub point_line: Option<LineFx>,
    /// CreateEffectOnParentOnAttackModel: (effect, scale) on the parent tower as this one attacks.
    #[serde(default)]
    pub parent_attack_fx: Option<(Option<String>, f32)>,
    /// CreateEffectOnRoundEndModel: an effect on the tower as each round ends (display only).
    #[serde(default)]
    pub round_end_fx: Option<String>,
    /// CreateEffectOnUpgradeModel: the effect this tier plays when bought (display only).
    #[serde(default)]
    pub upgrade_fx: Option<String>,
    /// The tower's extra displays (display only): prefab name and scale.
    #[serde(default)]
    pub extras: Vec<TierExtra>,
    /// LinkDisplayScaleToTowerRangeModel: (prefab name, displayRadius): that display drawn at
    /// range / displayRadius.
    #[serde(default)]
    pub range_links: Vec<(String, f32)>,
    /// MagusPerfectusGraveyardStateManagerModel: max mana and each state's attack names.
    #[serde(default)]
    pub magus: Option<Magus>,
    /// TowerCreateParagonTowerModel: the tower a paragon makes once active, by degree / 20.
    #[serde(default)]
    pub paragon_towers: Vec<std::sync::Arc<SubTowerSpec>>,
    /// MonkeyopolisModel: absorbs the farms in range once (Game::monkeyopolis_absorb).
    #[serde(default)]
    pub mopolis: Option<Mopolis>,
    /// MonkeyopolisUpgradeCostModel: that path's next tier costs per farm in range, blocked with none.
    #[serde(default)]
    pub mopolis_cost: Option<MopolisCost>,
    /// TapTowerAbilityBehavior.IsBanned's model checks: Overclock / Take Aim can't pick it.
    #[serde(default)]
    pub tap_banned: bool,
    /// A Beast Handler of tier 1+: pickable only with an active beast other than a Gyrfalcon.
    #[serde(default)]
    pub tap_beasts: bool,
    /// BeastHandlerLeashModel pets (towerModel / towerModelSecond), as permanent sub-towers.
    #[serde(default)]
    pub pets: Vec<std::sync::Arc<SubTowerSpec>>,
    /// BeastHandlerLeashModel.rotateToLeashedTower: a pet move turns the handler to the pet.
    #[serde(default)]
    pub leash_rotate: bool,
    /// BeastHandlerLeashModel.validMergeTargetObjectPath (+0x88): the marker on each tower a
    /// merge may go into (BeastHandlerMergeInput.EnterInputMode @0x180a3e9c0).
    #[serde(default)]
    pub merge_fx: Option<String>,
    /// BeastHandlerLeashModel.mergedTextDisplayPrefab / powerTextDisplayPrefab (+0x78 / +0x80)
    /// and mergeSound (+0xa8): what Contribute shows and plays.
    #[serde(default)]
    pub merged_fx: Option<String>,
    #[serde(default)]
    pub power_fx: Option<String>,
    #[serde(default)]
    pub merge_sound: Vec<String>,
    /// BeastHandlerLeashModel's piranhaPetModel / microraptorPetModel / gyrfalconPetModel by
    /// beast (GetMatchingPetModel).
    #[serde(default)]
    pub leash_power: std::collections::HashMap<String, PetPower>,
    pub range: f32,
    /// OverrideCamoDetectionModel.detectCamo (Monkey Sense): it and its sub-towers see camo.
    #[serde(default)]
    pub detect_camo: bool,
    #[serde(default)]
    pub global: bool,
    /// AirUnitModel (Heli Pilot): the attacks fire from a flying unit, not the tower.
    #[serde(default)]
    pub air: Option<AirSpec>,
    /// GyrfalconPatternModel: the air unit is a Beast Handler bird.
    #[serde(default)]
    pub grapple: Option<Grapple>,
    /// ToggleFocusStanceModel (Skywarden).
    #[serde(default)]
    pub stance: Option<Stance>,
    /// DamageBasedAttackSpeedModel (Skywarden).
    #[serde(default)]
    pub damage_speed: Option<DamageSpeed>,
    /// StatePoppedBasedPierceModel (Skywarden x-x-2+).
    #[serde(default)]
    pub state_pierce: Option<StatePierce>,
    /// LightningRodManagerModel (Skywarden x-5-x): damageTypesToZap.
    #[serde(default)]
    pub rod_manager: Option<Vec<String>>,
    /// RoundBasedAttackSpeedModel (Engineer Paragon): bonusAttackSpeedPerRoundPercent,
    /// maxWeaponAttackSpeed.
    #[serde(default)]
    pub round_speed: Option<[f32; 2]>,
    /// AddMakeshiftAreaModel: deck areas towers can stand on.
    #[serde(default)]
    pub decks: Vec<Deck>,
    /// PreEmptiveStrikeLauncherModel.projectileModel: launched at each MOAB-class bloon.
    #[serde(default)]
    pub preempt: Option<Proj>,
    /// PreEmptiveStrikeLauncherModel throwOffsetX / Y / Z.
    #[serde(default)]
    pub preempt_offset: [f32; 3],
    /// throwOffsetSecondX / Y / Z (zero: unused).
    #[serde(default)]
    pub preempt_offset2: [f32; 3],
    /// PreEmptiveStrikeLauncherModel (animationState, animationStateSecond) (display only).
    #[serde(default)]
    pub preempt_anim: [i32; 2],
    /// PreEmptiveStrikeLauncherModel.ejectEffectModel (prefab, scale), display only.
    #[serde(default)]
    pub preempt_fx: Option<(String, f32)>,
    /// RotateToDefaultPositionTowerModel: (rotation, onlyOnReachingTier).
    #[serde(default)]
    pub default_rot: Vec<[f32; 2]>,
    /// MerchantShipModel: a merchant ship (counted by Trade Empire).
    #[serde(default)]
    pub merchant: bool,
    #[serde(default)]
    pub trade_empire: Option<TradeEmpire>,
    #[serde(default)]
    pub cashback: Option<Cashback>,
    /// TowerModel.sellbackModifierAdd (Banana Farm x-x-2+: 0.1), added to the sell share.
    #[serde(default)]
    pub sellback: f32,
    /// TowerModel.ignoreMaxSellPercent: the sale value isn't capped at 95% / the worth.
    #[serde(default)]
    pub ignore_max_sell: bool,
    /// MonkeyTemple.StartSacrifice spares it: a power, paragon, ignoreTowerForSelection, Geraldo
    /// item, Phoenix / AirUnit sub-tower or the TranceTotem.
    #[serde(default)]
    pub no_sacrifice: bool,
    #[serde(default)]
    pub discount_zones: Vec<DiscountZone>,
    #[serde(default)]
    pub tower_count_icon: Option<TowerCountIcon>,
    /// FlagshipAttackSpeedIncreaseModel.attackSpeedIncrease.
    #[serde(default)]
    pub flagship: Option<f32>,
    /// ChurchillBaseRotationModel: the tower's base (Tower.Rotation) turns towards its turret.
    #[serde(default)]
    pub base_rotation: bool,
    /// PlacementAreaTypeRangeBuffModel.rangeMultiplier (Mermonkey): range on water.
    #[serde(default = "one")]
    pub water_range: f32,
    /// SubmergeModel (Sub x-3-x+): the attack used in the "Submerge" target mode.
    #[serde(default)]
    pub submerge: Option<Submerge>,
    /// The air unit's movement modes (Pointer = Follow Touch, Point = Lock In Place, Pursuit).
    #[serde(default)]
    pub air_modes: Vec<String>,
    /// PursuitSettingModel.pursuitDistance, when it has the Pursuit mode.
    #[serde(default)]
    pub pursuit_distance: Option<f32>,
    pub attacks: Vec<Attack>,
    #[serde(default)]
    pub abilities: Vec<Ability>,
    /// OrbitModel (Glaive Lord): projectiles circling the tower (display only; the hits are an attack).
    #[serde(default)]
    pub orbit: Option<Orbit>,
    /// DruidVengeanceEffectModel (Avatar of Wrath): display only.
    #[serde(default)]
    pub vengeance: Option<Vengeance>,
    /// MonkeyTempleModel (Sun Temple / True Sun God): what sacrificing towers gives.
    #[serde(default)]
    pub temple: Option<Temple>,
    #[serde(default)]
    pub zones: Vec<Zone>,
    /// StartOfRoundRateBuffModel: weapon intervals times `mult` for the first `duration` seconds of a round.
    #[serde(default)]
    pub round_start_rate: Option<RoundStartRate>,
    /// VigilanteTowerBehaviorModel: loseLifeAttackSpeedBuff, loseLifeRangeBuff,
    /// loseLifeBuffDurationFrames, loseLifeBuffCooldownFrames.
    #[serde(default)]
    pub vigilante: Option<[f32; 4]>,
    /// VigilanteTowerBehaviorModel.buffDisplay (effect prefab): on the tower while it runs (display only).
    #[serde(default)]
    pub vigilante_fx: Option<String>,
    /// PerRoundCashBonusTowerModel.cashPerRound: paid at each round end.
    #[serde(default)]
    pub round_cash: f64,
    #[serde(default)]
    pub round_cash_shared: f64,
    /// SpiritOfTheForestModel (Druid): thorn damage along the track.
    #[serde(default)]
    pub spirit: Option<Spirit>,
    /// CollectCashZoneModel (Banana Farm x-x-5, Banana Farmer).
    #[serde(default)]
    pub collect_zone: Option<CollectZone>,
    /// ActivateAbilityAfterIntervalModel.interval: seconds between automatic uses.
    #[serde(default)]
    pub auto_ability: Option<f32>,
    /// AddBehaviorToBloonInZone + IgnoreDmgImmunity (Black Resistance): [bloon properties removed,
    /// chance per projectile].
    #[serde(default)]
    pub ignore_zone: Option<(u32, f32)>,
    /// HexManagerModel's HexModel [damagePercentOfMax, interval] (Ezili's MOAB Hex).
    #[serde(default)]
    pub hex: Option<[f32; 2]>,
    /// Grow Blocker zone radius (0: the tower's range).
    #[serde(default)]
    pub grow_block: Option<f32>,
    /// FreezeNearbyWaterModel.radius (0: the tower's range): water there takes land towers.
    #[serde(default)]
    pub freeze_water: Option<f32>,
    /// FreezeNearbyWaterModel.freezeAsset (effect prefab): under the towers in its zone (display only).
    #[serde(default)]
    pub freeze_fx: Option<String>,
    /// BonusLivesPerRoundModel: (livesRestored, maxLivesAboveStartingLives).
    #[serde(default)]
    pub bonus_lives: Option<(f64, f64)>,

    #[serde(default)]
    pub bank: Option<Bank>,
    /// Buffs this tier gives towers in its range (Monkey Village, Shinobi Tactics).
    #[serde(default)]
    pub supports: Vec<Support>,
    /// TowerModel.towerSet (1 Primary, 2 Military, 4 Magic, 8 Support).
    #[serde(default)]
    pub set: u32,
    #[serde(default)]
    pub sounds: TowerSounds,
    /// TowerModel.towerSelectionMenuThemeId: which upgrade panel theme it uses.
    #[serde(default)]
    pub tsm_theme: String,
    /// TowerModel.portrait: the upgrade panel's portrait sprite.
    #[serde(default)]
    pub portrait: String,
    /// MonkeyCityModel: one tower of this kind a round costs x `mult` (a free Dart Monkey).
    #[serde(default)]
    pub free_tower: Option<FreeTower>,
    /// ComancheDefenceModel: mini Comanches called in (x-x-5: always there).
    #[serde(default)]
    pub comanche: Option<Comanche>,
    /// MoabShoveZoneModel: MOAB-class bloons near the Heli move at most this fraction of speed.
    #[serde(default)]
    pub moab_shove: Option<MoabShove>,
    /// PrinceOfDarknessZombieBuffModel: [damage added, distance multiplier] for every undead bloon.
    #[serde(default)]
    pub zombie_buff: Option<[f32; 2]>,
    /// SwitchTargetSupplierOnUpgradeModel: buying this tier switches to this target mode.
    #[serde(default)]
    pub switch_target: Option<String>,
}

/// A *SupportModel. `kind`: "range" (x(1 + mult) + add), "rate" (interval x mult), "pierce" (+add),
/// "speed" (projectile speed x(1 + mult)), "camo", "immune" (damage type -> add), "cash" (pop cash
/// x mult), "discount" (FreeUpgradeSupportModel: free up to `tier_cap`).
#[derive(Deserialize, Clone)]
pub struct Support {
    pub kind: String,
    /// An ability zone's displayModel (prefab): on each tower it buffs while it runs (display only).
    #[serde(default)]
    pub display: Option<String>,
    /// The ricochet zone's RetargetOnContactModel (kind "ricochet").
    #[serde(default)]
    pub retarget: Option<Retarget>,
    pub mult: f32,
    pub add: f32,
    pub id: String,
    pub global: bool,
    pub radius: Option<f32>,
    #[serde(rename = "self")]
    pub on_self: bool,
    pub stacks: u32,
    #[serde(default)]
    pub sets: Vec<u32>,
    /// FilterTowerParentModel: not the support tower's own parent.
    #[serde(default)]
    pub not_parent: bool,
    #[serde(default)]
    pub bases: Vec<String>,
    /// FilterTowerByPlaceableAreaModel.areaTypes (non-exclusive): the tower must take one of
    /// these and stand on an area of it (water 1 also takes type 7).
    #[serde(default)]
    pub area_types: Vec<i32>,
    /// Only heroes (MonkeySubParagonSupport.Filter: TowerModel.IsHero).
    #[serde(default)]
    pub heroes: bool,
    #[serde(default = "five")]
    pub tier_cap: u32,
    /// FilterInTowerTiersModel: the least tier on each path a tower it buffs must have.
    #[serde(default)]
    pub min_tiers: [u8; 3],
    /// Buff icon (UiBuffIcons sprite) shown over towers it reaches (showBuffIcon).
    #[serde(default)]
    pub icon: Option<String>,
    /// Its BuffIndicatorModel's buffName (buffLocsName) and maxStackSize.
    #[serde(default)]
    pub buff_name: String,
    #[serde(default)]
    pub buff_max: i32,
    /// Seconds an ability's support zone lasts (0 for a tower's own supports).
    #[serde(default)]
    pub lifespan: f32,
    /// An ability that mutates the towers inside Activate (Call to Arms), not through a zone
    /// process that reaches them the next frame.
    #[serde(default)]
    pub instant: bool,
    /// RangeSupportModel's MutatorTower: the range buff also lengthens TravelStrait lifespans.
    #[serde(default)]
    pub range_life: bool,
    /// SupportModel.onlyAffectParagon: put on with AddParagonMutator (Paragons only).
    #[serde(default)]
    pub only_paragon: bool,
}

/// ParagonTowerModel's purchase effects: [effect, scale] of the investment orb (and its tier-5
/// version), the Paragon's effect during the sacrifice and at its end, and their timing.
#[derive(Deserialize, Clone, Debug)]
pub struct SacrificeFx {
    pub invest: Option<(String, f32)>,
    pub invest_t5: Option<(String, f32)>,
    pub during: Option<(String, f32)>,
    pub end: Option<(String, f32)>,
    /// ParagonTowerModel.degreeTextEffectId (its effect) and degreeTextEffectLifespan (seconds).
    #[serde(default)]
    pub degree_text: Option<(String, f32)>,
    /// The model's numeric fields as serialized (build_game_data.py "raw").
    #[serde(default)]
    pub raw: std::collections::HashMap<String, f32>,
}

impl SacrificeFx {
    pub fn get(&self, k: &str) -> f32 {
        self.raw.get(k).copied().unwrap_or(0.0)
    }
}

fn minus_one_i32() -> i32 {
    -1
}

fn five() -> u32 {
    5
}

#[derive(Deserialize, Clone)]
pub struct RoundStartRate {
    pub mult: f32,
    pub duration: f32,
}

#[derive(Deserialize, Clone)]
pub struct Orbit {
    pub count: u32,
    pub range: f32,
    pub display: Option<String>,
    /// The glaives' RotateModel.angle (degrees a second; Glaive Lord -720).
    #[serde(default)]
    pub spin: f32,
}

#[derive(Deserialize, Clone)]
pub struct FreeTower {
    pub tower: String,
    /// MonkeyCityModel.multiplier: read by nothing in the game's code.
    pub mult: f64,
    /// MonkeyCityModel.roundsTillMultiplier: round ends before the first free tower.
    #[serde(default)]
    pub rounds: i32,
}

#[derive(Deserialize, Clone)]
pub struct Comanche {
    /// Frames between call-ins, frames they stay, call-ins a round, how many.
    pub cooldown: f32,
    pub duration: f32,
    pub per_round: u32,
    pub count: u32,
    /// Comanche Commander: they're there from the start and stay.
    pub immediate: bool,
    pub tower: std::sync::Arc<SubTowerSpec>,
}

#[derive(Deserialize, Clone)]
pub struct MoabShove {
    pub range: f32,
    pub moab: f32,
    pub bfb: f32,
    pub zomg: Option<f32>,
    /// zomgPushSpeedScaleCap: a Zomg's or Ddt's cap, even with Zomgs filtered out.
    #[serde(default = "one")]
    pub zomg_cap: f32,
    #[serde(default)]
    pub camo: bool,
}

#[derive(Deserialize, Clone)]
pub struct Necro {
    #[serde(rename = "damageRbePerStack")]
    pub damage_rbe_per_stack: f32,
    #[serde(rename = "damageStackMax")]
    pub damage_stack_max: f32,
    #[serde(rename = "maxBloonsSpawnedPerWave")]
    pub max_per_wave: u32,
    #[serde(rename = "minBloonsSpawnedPerWave")]
    pub min_per_wave: u32,
    #[serde(rename = "maxPiercePerBloon")]
    pub max_pierce: f32,
    #[serde(rename = "maxRbeSpawnedPerSecond")]
    pub rbe_per_second: f32,
    #[serde(rename = "maxRbeStored")]
    pub max_stored: f32,
    #[serde(rename = "rateRbePerStack")]
    pub rate_rbe_per_stack: f32,
    #[serde(rename = "rateStackMax")]
    pub rate_stack_max: f32,
    #[serde(rename = "roundsBeforeDecay")]
    pub decay_rounds: u32,
    #[serde(rename = "maxPathRandomRange")]
    pub path_range: f32,
    /// NecromancerEmissionModel.maxPathOffset: each undead bloon's random offset from the track.
    #[serde(default, rename = "maxPathOffset")]
    pub path_offset: f32,
    /// PrinceOfDarknessEmissionModel.alternateProjectile: sent when the graveyard is empty.
    #[serde(default)]
    pub prince: Option<Box<Proj>>,
    #[serde(default, rename = "princePierce")]
    pub prince_pierce: f32,
    /// PrinceOfDarknessEmissionModel.maxPathRandomRange / maxPathOffset: the ghost's own.
    #[serde(default, rename = "princeRange")]
    pub prince_range: f32,
    #[serde(default, rename = "princeOffset")]
    pub prince_offset: f32,
}

#[derive(Deserialize, Clone)]
pub struct Attack {
    /// AttackModel name (an upgrade keeps the Attack whose model has the same name).
    #[serde(default)]
    pub model_name: String,
    pub range: f32,
    pub camo: bool,
    /// AttackModel has a RotateToTargetModel (the tower turns to face its target).
    #[serde(default = "yes")]
    pub rotate: bool,
    /// RotateToPointerModel.weaponEjectZ: the height its aim point is raised by.
    #[serde(default)]
    pub turn_eject_z: f32,
    /// RotateToPointerModel.rotateOnlyOnEmit: the turn steps once as each throw starts.
    #[serde(default)]
    pub turn_on_emit: bool,
    /// RotateToPointerModel.rotateTower (default true).
    #[serde(default = "yes")]
    pub turn_tower: bool,
    /// RotateToPointerModel.rate: degrees per second the tower turns towards its aim point.
    #[serde(default)]
    pub turn_rate: Option<f32>,
    /// Targeting modes from the attack's Target*Model behaviours ("First", "Last", "Close",
    /// "Strong", "Pointer" = the cursor, "Point" = a point the player sets).
    #[serde(default)]
    pub targets: Vec<String>,
    /// "left" / "right": a Robo Monkey arm, targeting with that hand's mode.
    #[serde(default)]
    pub hand: Option<String>,
    /// NecromancerEmissionModel: the weapon spends the graveyard on undead bloons.
    #[serde(default)]
    pub necro: Option<Necro>,
    pub weapons: Vec<Weapon>,
    /// targetProvider RandomTargetModel: a random visible bloon (RandomTarget.GetTarget).
    #[serde(default)]
    pub random_target: bool,
    /// FilterWithTagModel / FilterOutTagModel tags (MOAB Press only targets "Moabs", never "Bad").
    #[serde(default)]
    pub require_tags: Vec<String>,
    /// FilterOnlyCamoInModel: targets camo bloons only (Shimmer).
    #[serde(default)]
    pub only_camo: bool,
    /// FilterTargetAngleModel in its filter: [fieldOfView, baseTowerRotationOffset,
    /// ignoreTowerRotation (0 / 1)].
    #[serde(default)]
    pub target_angle: Option<[f32; 3]>,
    /// RotateToTargetAttackOffsetModel: [offsetX, offsetY, additionalRotation, startRotation].
    #[serde(default)]
    pub offset_rot: Option<[f32; 4]>,
    /// AttackModel.offsetZ.
    #[serde(default)]
    pub offset_z: f32,
    /// RotateToTargetAttackOffsetModel.ignoreParentRotationTime (seconds).
    #[serde(default)]
    pub offset_ignore: f32,
    #[serde(default)]
    pub exclude_tags: Vec<String>,
    /// AttackModel.fireWithoutTarget: fires on its own timer, target or not (Glaive Lord's orbit hits).
    /// CreateEffectWhileAttackingModel: [the loop effect, the effect as it stops].
    #[serde(default)]
    pub while_fx: Option<[String; 2]>,
    #[serde(default)]
    pub fire_without_target: bool,
    /// AttackModel.attackThroughWalls: targets aren't checked for a clear line (Map.IsTargetLineClear).
    #[serde(default)]
    pub through_walls: bool,
    /// CheckTargetsWithoutOffsetsModel on the attack: no CheckFromOffset line (Attack.CheckFromOffset
    /// @0x180dac780 returns zero).
    #[serde(default)]
    pub los_no_offset: bool,
    /// FighterPilotPattern target suppliers: FilterTargetAngleModel fieldOfView.
    #[serde(default)]
    pub fighter: Option<f32>,
    /// TargetSelectedPointModel.startWithClosestTrackPoint (Wall of Fire past 3-x-x).
    #[serde(default)]
    pub point_start_track: bool,
    /// TargetSelectedPointModel.lockToInsideTowerRange: a point set for it must be within its range.
    #[serde(default)]
    pub point_lock: bool,
    #[serde(default)]
    pub glue_filter: u32,
    #[serde(default)]
    pub skip_frozen: bool,
    /// FilterOutOffscreenModel (Map.IsPointOnMap of the bloon's position): 0 none, 1 with the
    /// bloon's radius as border (includeBloonRadius), 2 its centre on the map.
    #[serde(default)]
    pub offscreen: u8,
    /// FilterBloonIfDamageTypeModel.ifCantHitBloonProperties: bloons with these properties
    /// that the damage is immune to are skipped.
    #[serde(default)]
    pub cant_hit: u32,
    /// TargetMoabModel: MOAB-class bloons first.
    #[serde(default)]
    pub prio_moab: bool,
    /// Target*PrioCamoModel: camo bloons are picked before any others (Ninja Monkey).
    #[serde(default)]
    pub prio_camo: bool,
    /// TargetDesperado*Model.canPrioritiseCamo / canTargetMarked: the upgrade panel's camo and
    /// bounty hunter toggles.
    #[serde(default)]
    pub desperado_camo: bool,
    #[serde(default)]
    pub desperado_marked: bool,
    /// FilterMutatedTargetModel: skips bloons carrying an effect with one of these ids.
    #[serde(default)]
    pub skip_effects: Vec<String>,
    /// FilterMutatedTargetModel inverse: only bloons carrying an effect with one of these ids.
    #[serde(default)]
    pub need_effects: Vec<String>,
    /// FilterMarkedToPopModel: skips bloons whose lineage this activation's shots targeted.
    #[serde(default)]
    pub mtp_filter: bool,
    /// FilterIfAttackHasTargetModel.attackName: no target while that attack's hadValidTarget is set.
    #[serde(default)]
    pub if_target_attack: Option<String>,
    /// AttackModel.fixedRange: VigilanteMutator leaves its range alone.
    #[serde(default)]
    pub fixed_range: bool,
    /// TargetTrackModel.maxOffset: aims at a random spot on the track in range (caltrops).
    #[serde(default)]
    pub track_target: Option<f32>,
    /// TargetTrackOrDefaultModel.useClosestTrack: the track point closest to the tower instead
    /// (Wall of Fire).
    #[serde(default)]
    pub track_closest: bool,
    /// The track spot comes from TargetTrackOrDefault (forceTargetTrack), not CloseTargetTrack.
    /// TargetTrackModel.onlyTargetPathsWithBloons: GetTargets skips paths without bloons, and
    /// its points are remade at each round end (OnEarlyRoundEnd).
    #[serde(default)]
    pub track_only_bloons: bool,
    /// TargetTrackOrDefaultModel without forceTargetTrack: track points only when no bloon.
    #[serde(default)]
    pub track_fallback: bool,
    #[serde(default)]
    pub track_default: bool,
    /// RandomPositionBasicModel min/max radius: aims at a random spot around the tower (bananas).
    #[serde(default)]
    pub random_position: Option<[f32; 2]>,
    /// RandomPositionBasicModel.mapBorder, when the random position is that model's.
    #[serde(default)]
    pub random_basic: Option<f32>,
    /// RandomPositionModel (areaType Land): the random spot must be placeable land, clear by this radius.
    #[serde(default)]
    pub random_land: Option<f32>,
    /// RandomPositionModel (Land): minDistance, maxDistance, pointDistance, targetRadius,
    /// idealDistanceWithinTrack.
    #[serde(default)]
    pub random_pos: Option<RandomPos>,
    /// Throws this brew at towers instead of attacking bloons.
    #[serde(default)]
    pub brew: Option<Brew>,
    /// TargetTrackOrDefaultAcidPoolModel: with no bloon in range, aims at the track (acid pools).
    #[serde(default)]
    pub pool_fallback: bool,
    /// AttackMinimumRangeModel: bloons closer than this are ignored (Desperado's rifle).
    #[serde(default)]
    pub min_range: f32,
    /// RotateToTargetModel.additionalRotation (degrees): the tower turns this far past its
    /// target (Buccaneer: -90, broadside on).
    #[serde(default)]
    pub extra_rotation: f32,
    /// RotateToTargetModel.rotateTower: false keeps the tower's facing while it aims.
    #[serde(default = "yes")]
    pub turn_body: bool,
    /// RotateToTargetModel.useThrowMarkerHeight: it aims from the tower raised by this (its
    /// weapons' highest ejectZ), not 10.
    #[serde(default)]
    pub marker_z: Option<f32>,
    /// RotateToTargetModel.onlyRotateDuringThrow (and rotateOnlyOnThrow) off: turns to its
    /// target every frame it has one.
    #[serde(default)]
    pub rotate_always: bool,
    /// RotateToTargetAirUnitModel: while it has a target the air unit faces it.
    #[serde(default)]
    pub air_rotate: bool,
    /// TargetFirst / Last / StrongAirUnitModel: BloonManager.GetTarget from the air unit, through
    /// Targetting.GetTargets (a MOAB at its centre, never a circle).
    #[serde(default)]
    pub air_target: bool,
    /// RotateToMiddleOfTargetsModel: the body faces the midpoint of every attack's target.
    #[serde(default)]
    pub face_middle: bool,
    /// An effect played at the tower each time it fires (a submerged Sub's radar pulse).
    #[serde(default)]
    pub fire_fx: Option<String>,
    /// Target*SharedRangeModel.isSharedRangeEnabled (Sub Advanced Intel): bloons in range of
    /// any other tower count as in range.
    #[serde(default)]
    pub shared_range: bool,
    /// Target*SharedRangeModel.shareLineOfSight: pressed segments take line of sight from the
    /// grid alone (else also the attack's own sight of the segment).
    #[serde(default)]
    pub shared_los: bool,
    /// FilterInvisibleSubIntel: can't see camo itself (its grid entries don't), but camo bloons
    /// pass its filter, so it hits them on pressed segments another tower sees camo on.
    #[serde(default)]
    pub intel_camo: bool,
    /// AttackModel.addsToSharedGrid / sharedGridRange (0: its own range).
    #[serde(default)]
    pub adds_shared: bool,
    #[serde(default)]
    pub shared_grid_range: f32,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize, Clone)]
pub struct Weapon {
    /// CritRollModel / CritRollWithDistanceModel: [damageAdditive, damageMultiplier, chance, bonusPerDistance].
    #[serde(default)]
    pub crit_roll: Option<[f32; 4]>,
    /// The crit roll model has a display (CritText): shown where a rolled crit hits (display only).
    #[serde(default)]
    pub crit_roll_text: bool,
    /// StandoffModel: [maxRateIncrease, rateDecreasePerBloon].
    #[serde(default)]
    pub standoff: Option<[f32; 2]>,
    /// TheBlazingSunModel: [maxStacks, stacksToBurn, attackSpeedPerStack, debuffExpiresAfterFrames].
    #[serde(default)]
    pub blazing_sun: Option<[f32; 4]>,
    /// CycleAnimationModel.randomize: a synced draw at each throw.
    #[serde(default)]
    pub cycle_random: bool,
    /// WeaponModel name without its prefix ("MainWeapon", "LongWeapon"...).
    #[serde(default)]
    pub name: String,
    pub rate: f32,
    /// WeaponRateMinModel.min: the shortest interval buffs can bring it to.
    #[serde(default)]
    pub rate_min: f32,
    /// PauseOtherAttacksModel.durationFrames: a shot pauses the tower's other attacks until
    /// roundTime passes the shot's + this.
    #[serde(default)]
    pub pause_others: Option<i64>,
    /// LeakDangerAttackSpeedModel.maxRateIncreasePercent.
    #[serde(default)]
    pub leak_danger: f32,
    /// BloonDistanceRateBonusModel: (rateBonusPerRange, rangeMax).
    #[serde(default)]
    pub dist_rate: Option<(f32, f32)>,
    /// VagrantWeaponBehaviorModel: noTowersRateBonus, towerRangeBonusReduction,
    /// bloonInRangeAttackSpeedBuff, maxBloonAttackSpeedBuff.
    #[serde(default)]
    pub vagrant: Option<[f32; 4]>,
    /// The Vagrant's buff icon (iconName, buffName).
    #[serde(default)]
    pub vagrant_icon: Option<(String, String)>,
    /// SpinModel.rotationPerSecond (0: none).
    #[serde(default)]
    pub spin: f32,
    /// WeaponModel.animationOffset: the attack animation starts this many seconds before the shot.
    #[serde(default)]
    pub animation_offset: f32,
    /// WeaponModel.animation: the animationStates index a throw plays (display only).
    #[serde(default)]
    pub anim_state: i32,
    /// AlternateAnimationModel (states, originState): each emission sets state
    /// emissionCount % states + originState + 1 (display only).
    #[serde(default)]
    pub alt_anim: Option<[i32; 2]>,
    /// CycleAnimationModel: the animation state each throw starts (display only; the random
    /// draw is made either way: `cycle_random`).
    #[serde(default)]
    pub cycle_anim: Option<CycleAnim>,
    /// RateBasedAnimationOffsetModel: (baseRate, offset).
    #[serde(default)]
    pub rate_anim_offset: Option<[f32; 2]>,
    /// WeaponModel.fireBetweenRounds: the weapon's clock is Simulation.time (always running)
    /// instead of roundTime (stopped between rounds).
    #[serde(default)]
    pub between_rounds: bool,
    /// FlipFollowPathModel: (flipTowerDisplayX, flipTowerDisplayY), when the weapon has one.
    #[serde(default)]
    pub flip_eject: Option<[bool; 2]>,
    /// FlipFollowPathModel.effectOnFlip (prefab, scale): played at the tower on a hand switch.
    #[serde(default)]
    pub flip_fx: Option<(String, f32)>,
    /// WeaponModel.startInCooldown / customStartCooldown (seconds).
    #[serde(default)]
    pub start_cooldown: bool,
    /// WeaponModel.storeTargetOnAnimationStart (the Skywarden's bolts): a throw whose Target is
    /// gone at its release aims at where it was as the throw started.
    #[serde(default)]
    pub store_target: bool,
    #[serde(default)]
    pub custom_start: f32,
    /// WeaponModel.ejectX/Y/Z: launch point relative to the tower (x right, y forward, z up).
    #[serde(default)]
    pub eject: [f32; 3],
    /// HYPOTHESIS P-ABSMARKER: throw markers each throw starts at in turn, from (0, 0) (Mortar 0-4-x+).
    #[serde(default)]
    pub abs_markers: Vec<[f32; 3]>,
    /// EjectEffectModel / AlternatingEjectEffectModel: muzzle effect (prefab, scale) at the
    /// eject point on each throw, visual only.
    #[serde(default)]
    pub eject_fx: Vec<(String, f32)>,
    /// TheBlazingSun's muzzle effects (prefab, scale): [under stacksToBurn, overheated].
    #[serde(default)]
    pub blazing_fx: Option<[Option<(String, f32)>; 2]>,
    /// TheBlazingSun.ProjectilesCreated: burnBehaviorModel's effects, burnBehaviorArea and the
    /// emission it goes out through.
    #[serde(default)]
    pub blazing_burn: Vec<EffectSpec>,
    #[serde(default)]
    pub blazing_area: Option<Box<Proj>>,
    #[serde(default)]
    pub blazing_emission: Option<Emission>,
    /// LineEffectModel (Ray of Doom): the beam drawn along its line shots.
    #[serde(default)]
    pub line_fx: Option<LineFx>,
    pub emission: Emission,
    pub projectile: Proj,
    pub crit: Option<Crit>,
    /// AlternateProjectileModel: every `interval`-th shot fires this instead (Flash Bomb).
    #[serde(default)]
    pub alternate: Option<Alternate>,
    /// BonusProjectileAfterIntervalModel: every `interval`-th emission, `cooldown` frames of round
    /// time after the last, `projectile` too (from the tower's position with `tower_pos`).
    #[serde(default)]
    pub bonus_after: Option<BonusAfter>,
    /// RandomAngleOffsetModel: shots deviate by up to this many degrees either way.
    #[serde(default)]
    pub spread: f32,
    /// EmissionsPerRoundFilterModel.count: at most this many shots per round (bananas).
    #[serde(default)]
    pub per_round: Option<u32>,
    /// MagusPerfectusGraveyardModel.graveyardManaChange (0: none).
    #[serde(default)]
    pub mana: i32,
    /// LimitProjectileModel.limit: no shot while the tower has this many unfilled traps out.
    #[serde(default)]
    pub limit: Option<u32>,
    /// LimitProjectileModel.projectileId / delayInFrames / limitByDestroyedPriorProjectile.
    #[serde(default)]
    pub limit_id: String,
    #[serde(default)]
    pub limit_delay: i64,
    #[serde(default)]
    pub limit_destroyed: bool,
    /// JungleVineLimitProjectileModel: [limit, delayInFrames].
    #[serde(default)]
    pub vine_limit: Option<[i64; 2]>,
    /// GreatWhiteLimitProjectileModel: one bite / grab at a time (FilterEmission @0x180c796a0).
    #[serde(default)]
    pub gw_limit: bool,
    /// CheckAirUnitOverTrackModel: only while the plane is over the track.
    #[serde(default)]
    pub over_track: bool,
    /// FireFromAirUnitModel: ejected from the air unit (else from the tower itself).
    #[serde(default)]
    pub from_air: bool,
    /// UseParentEjectModel: shots leave from the tower's first Attack's first Weapon's eject.
    #[serde(default)]
    pub parent_eject: bool,
    /// AngleToMapCenterModel: (range, offset) in degrees.
    #[serde(default)]
    pub map_center: Option<[f32; 2]>,
    /// ResetRateOnInitialiseModel.weaponModel's name: made, it readies the attack's weapons of it.
    #[serde(default)]
    pub reset_rate: Option<String>,
    /// SendToBankModel emission: the cash goes into the tower's bank instead of the map.
    #[serde(default)]
    pub to_bank: bool,
    /// RandomRotationWeaponBehavior: each shot draws a synced NextDouble as it's made.
    #[serde(default)]
    pub random_rot: bool,
    /// BurstWeaponBehaviorModel: each shot is `count` shots, `frames` apart.
    #[serde(default)]
    pub burst: Option<BurstFire>,
    /// PsiLimitProjectileModel: fires only once the tower's Psi canFireAt <= roundTime.
    #[serde(default)]
    pub psi_limit: bool,
    /// RandomRateModel.offsetRateFrames: lastEmission + (int)(this x syncedRandom.NextDouble())
    /// after each throw.
    #[serde(default)]
    pub random_rate: Option<i32>,
    /// CreateSoundOnProjectileCreatedModel: one of these per shot.
    #[serde(default)]
    pub fire_sounds: Vec<String>,
    /// SubTowerFilterModel: no shot while the tower has `max` live sub-towers with these ids.
    #[serde(default)]
    pub max_subs: Option<MaxSubs>,
    /// Emission*RotationOff*Model.offsetRotation (degrees): shots leave at the tower's facing
    /// plus this, not at the target (Buccaneer broadsides).
    #[serde(default)]
    pub aim_offset: Option<f32>,
    /// EmissionRotationOffDisplayModel: laid out on the tower's rotation, shot at the display's.
    #[serde(default)]
    pub disp_rot: bool,
    /// EmissionRotationOffBloonDirectionModel: shots head for the target bloon from the eject
    /// point ("eject", OnEmit) or the tower ("tower", dontSetAfterEmit), whatever the facing;
    /// "air" (useAirUnitPosition: from the plane, Monkey Ace) isn't applied yet.
    #[serde(default)]
    pub bloon_dir: Option<String>,
    /// PrinceOfDarknessEmissionModel: the weapon sends the Prince of Darkness's ghost.
    #[serde(default)]
    pub prince: bool,
    /// FireWhenAlternateWeaponIsReadyModel.weaponId: fires only when the weapon with this
    /// FireAlternateWeaponModel id fires, ignoring its own rate.
    #[serde(default)]
    pub with_main: Option<i32>,
    /// FireAlternateWeaponModel.weaponId: firing this also fires the weapons paired with it.
    #[serde(default)]
    pub alt_id: Option<i32>,
    /// OffsetModel: (range, angleOffset).
    #[serde(default)]
    pub offset_model: Option<[f32; 2]>,
    /// LifeBasedAttackSpeedModel: ratePerLife, lifeCap, baseRateIncrease.
    #[serde(default)]
    pub life_speed: Option<[f32; 3]>,
    /// FilterTargetAngleFilterModel [fieldOfView, baseTowerRotationOffset] (degrees): fires
    /// only with a bloon in range inside that arc around the tower's facing.
    #[serde(default)]
    pub fov: Option<[f32; 3]>,
}

/// TradeEmpireBuffModel (Buccaneer 0-0-5).
#[derive(Deserialize, Clone)]
pub struct TradeEmpire {
    pub per_ship: f64,
    pub per_favoured: f64,
    pub cap: u32,
    pub damage: f32,
    pub ceramic: f32,
    pub moab: f32,
    /// mutatorId ("TradeEmpire", the Paragon's "TradeEmpireParagon"): Trade Empires group by it.
    #[serde(default)]
    pub id: String,
    /// Its BuffIndicatorModel (TowerBehaviorBuffModel.GetBuffIndicatorModel @0x18124c210).
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub buff_name: String,
    #[serde(default)]
    pub buff_max: i32,
}

/// DiscountZoneModel (Monkey Business / Commerce, Sun Temple, ...): the fields
/// TowerManager.GetZoneDiscount (@0x180c2bd90) tests.
#[derive(Deserialize, Clone)]
pub struct DiscountZone {
    /// discountMultiplier (+0x40).
    pub mult: f32,
    /// stackLimit (+0x44): of a stackName group, the largest multipliers kept.
    pub stack_limit: i32,
    /// stackName (+0x48).
    pub group: String,
    /// affectSelf (+0x50).
    pub affect_self: bool,
    /// tierCap (+0x54), tierMin (+0x58).
    pub tier_cap: i32,
    pub tier_min: i32,
    /// towerBaseIdList (+0x68).
    pub bases: Vec<String>,
    /// upgradeId (+0x78): only that upgrade when set.
    pub upgrade_id: String,
    /// isGlobal (+0x81): no range test.
    pub global: bool,
    /// Its BuffIndicatorModel (TowerBehaviorBuffModel.GetBuffIndicatorModel).
    pub icon: Option<String>,
    pub buff_name: String,
    pub buff_max: i32,
}

/// BuffIconPerTowerInRangeModel: an icon over its own tower counting the towers its
/// Zoned<Tower> holds (BuffIconPerTowerInRange.GetBuffs MoveNext @0x180d11ab0).
#[derive(Deserialize, Clone)]
pub struct TowerCountIcon {
    pub icon: String,
    pub buff_name: String,
    /// rangeOverride (GetTowerRange @0x180d04ef0: used when > 0, else TowerModel.range).
    pub range: f32,
    /// towerIdFilter (Filter @0x180d04d70: the tower's baseId).
    pub filter: Vec<String>,
}

/// CashbackZoneModel (Favored Trades): towers in range sell for more.
#[derive(Deserialize, Clone)]
pub struct Cashback {
    pub mult: f32,
    pub max: f32,
    pub stacks: i32,
    /// CashbackZoneModel.groupName: zones add up per group.
    #[serde(default)]
    pub group: String,
    /// CashbackZoneModel.baseIdsArray: the towers it reaches (empty: all).
    #[serde(default)]
    pub bases: Vec<String>,
    /// Its BuffIndicatorModel (TowerBehaviorBuffModel.GetBuffIndicatorModel @0x18124c210:
    /// iconName buffIconName, buffName buffLocsName, maxStackSize).
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub buff_name: String,
    #[serde(default)]
    pub buff_max: i32,
}

#[derive(Deserialize, Clone)]
pub struct Submerge {
    pub attack: Attack,
    /// abilityCooldownSpeedScale: ability cooldowns of towers in range run this much faster.
    #[serde(default = "one")]
    pub cooldown_scale: f32,
    /// abilityCooldownSpeedScaleGlobal: the same for every tower on the map.
    #[serde(default = "one")]
    pub cooldown_scale_global: f32,
    /// heroXpScale: heroes in range earn this much more XP.
    #[serde(default = "one")]
    pub hero_xp: f32,
    /// submergeDepth: how far the model sinks.
    #[serde(default)]
    pub depth: f32,
}

#[derive(Deserialize, Clone)]
pub struct Deck {
    /// Polygon in the tower's local units: x to its right, y forward.
    pub points: Vec<[f32; 2]>,
    pub height: f32,
    /// filterOutSpecificTowers: tower base ids that can't go on it.
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct ProjBoost {
    pub frames: f32,
    /// Immune properties the projectiles get instead (ChangeDamageTypeModel).
    #[serde(default)]
    pub immune: Option<u32>,
    #[serde(default)]
    pub damage: f32,
    #[serde(default)]
    pub tag_mods: Vec<TagMod>,
    /// Projectile radius multiplier (Long Arm of Light).
    #[serde(default = "one")]
    pub radius_mult: f32,
    /// LongArmOfLightMutator.Mutate (@0x181287d60): every AdoraTrackTargetModel's rotation,
    /// speeds, acceleration and lifespan x this.
    #[serde(default = "one")]
    pub adora_mult: f32,
    /// LongArmOfLightMutator.Mutate (VERIFIED-DECOMP @0x181287d60): every ProjectileModel under
    /// the tower gets pierce x multiplier (+0x3c; recording adora_l20_r80 rt 1027, the bolts' 22).
    #[serde(default = "one")]
    pub pierce_mult: f32,
    /// MutateProjectileOnAbilityModel.projectileModel: only projectiles with this id (their
    /// ProjectileModel name; ProjectileMutator.Mutate, VERIFIED-DECOMP @0x181291870) take the
    /// damage and tag modifiers (Armor Piercing Shells: the cannon's Explosion).
    #[serde(default)]
    pub proj_id: Option<String>,
    /// Its projectileBehaviorModel when a DetonateEffectModel (Silas's Frostbite).
    #[serde(default)]
    pub detonate: Option<Detonate>,
}

/// DetonateEffectModel (DetonateEffect.Collide, VERIFIED-DECOMP @0x180cb8d70): on a bloon with a
/// timed mutator of one of these keys, Bloon.Damage(damagePerSecondRemaining x its frames left /
/// 60), then the mutator goes.
#[derive(Deserialize, Clone)]
pub struct Detonate {
    pub tags: Vec<String>,
    pub dps: f32,
}

#[derive(Deserialize, Clone)]
pub struct TrackTravel {
    pub speed: f32,
    pub lifespan: f32,
    #[serde(default)]
    pub reverse: bool,
    /// TravelAlongPathModel.disableRotateWithPathDirection: the display keeps its starting rotation.
    #[serde(default)]
    pub fixed_rotation: bool,
}

impl TrackTravel {
    /// TravelAlongPathModel.ctor (VERIFIED-DECOMP @0x181202680): speedFrames = speed x the f32
    /// literal 0.016666668 (0x183e4fca0), not speed / 60 (one ulp apart for speeds 25, 300, 500).
    pub fn speed_frames(&self) -> f32 {
        self.speed * (1.0f32 / 60.0)
    }

    /// lifespanFrames = (int)(lifespan x 60f): 0.8333333 s is 49 frames.
    pub fn lifespan_frames(&self) -> f32 {
        (self.lifespan * 60.0) as i32 as f32
    }
}

#[derive(Deserialize, Clone)]
pub struct PlaceAt {
    pub projectile: Proj,
    pub delay: f32,
}

/// LeapingSwordModel (Sauda's Leaping Sword): the impact and DoT projectiles made where she lands,
/// landDelayFrames after the jump, and timeUntilFinishFrames until her attacks resume.
#[derive(Deserialize, Clone)]
pub struct Leap {
    pub projectile: Proj,
    #[serde(default)]
    pub dot: Option<Proj>,
    pub land: i64,
    pub finish: i64,
}

/// SwordChargeModel (Sauda's Sword Charge; SwordCharge.Process @0x180e5bcc0): `projectile` made
/// at every valid exit path's leakDist, `iterations` rounds of them (the next once all of the last
/// are gone), after initialDelayFrames (`initial`); endDelayFrames (`end`) after the last, Finish.
#[derive(Deserialize, Clone)]
pub struct SwordCharge {
    pub projectile: Proj,
    #[serde(default)]
    pub initial: i64,
    #[serde(default)]
    pub end: i64,
    #[serde(default = "one_u32")]
    pub iterations: u32,
}

fn one_u32() -> u32 {
    1
}

#[derive(Deserialize, Clone)]
pub struct Biohack {
    pub count: u32,
    pub damage: f32,
    pub lifespan: f32,
    /// filterTowers: baseIds Biohack.FilterTower refuses (MonkeyVillage only below a top-path 5).
    #[serde(default)]
    pub filter: Vec<String>,
    /// Frames the BehaviorMutator (the DelayedShutoff) stays past the lifespan:
    /// BiohackModel.GetShutOffFrames, None when `behaviors` is empty (no BehaviorMutator).
    #[serde(default)]
    pub shutoff: Option<f32>,
}

#[derive(Deserialize, Clone)]
pub struct Barrage {
    pub targets: u32,
    pub damage: f32,
    pub interval: f32,
    pub lifespan: f32,
    #[serde(default)]
    pub immune: u32,
    /// MoabBarrageBloonBehaviorModel.initialDelay / randomDelayMax (seconds) and numOfMissiles.
    #[serde(default)]
    pub initial_delay: f32,
    #[serde(default)]
    pub random_delay: f32,
    #[serde(default)]
    pub missiles: u32,
    /// MoabBarrageModel.durationFrames: the mutator's time on the bloon.
    #[serde(default)]
    pub duration_frames: u32,
}

#[derive(Deserialize, Clone)]
pub struct TrackExpire {
    pub projectile: Proj,
    pub emission: Emission,
    pub count: u32,
    pub range: f32,
    #[serde(default)]
    pub use_rotation: bool,
}

#[derive(Deserialize, Clone)]
pub struct FinalStrike {
    /// Seconds before the first missile, and between missiles.
    pub countdown: f32,
    pub launch_rate: f32,
    /// throwOffsetX / Y / Z.
    pub offset: [f32; 3],
    pub projectile: Proj,
    pub emission: Emission,
}

#[derive(Deserialize, Clone)]
pub struct Takedown {
    /// TargetGrapplableModel.hooks, and hooks a ZOMG / BAD needs (others need one).
    pub hooks: u32,
    pub zomg: u32,
    pub bad: u32,
    /// MoabTakedownModel cash: pop cash x mult + add.
    pub mult: f64,
    #[serde(default)]
    pub add: f64,
    /// FilterWithTagsModel (inclusive): tags never hooked.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// The ability attack's range (TargetGrapplable.GetTargets searches within it).
    #[serde(default)]
    pub range: f32,
    /// CreateRopeEffectModel (display only): chain prefab, hook prefab, spriteSpacing, spriteOffset.
    #[serde(default)]
    pub rope: Option<(Option<String>, Option<String>, f32, f32)>,
    /// The ability attack's RotateToTarget additionalRotation and its weapon's eject point.
    #[serde(default)]
    pub turn: f32,
    #[serde(default)]
    pub eject: [f32; 3],
    /// The hook projectile's CreateEffectOnContactModel effect (prefab, scale), display only.
    #[serde(default)]
    pub hit_fx: Option<(String, f32)>,
}

#[derive(Deserialize, Clone)]
pub struct MaxSubs {
    pub ids: Vec<String>,
    pub max: u32,
}

#[derive(Deserialize, Clone)]
pub struct BurstFire {
    pub count: u32,
    pub frames: f32,
    /// EmissionOverTimeModel (`count` shots per throw), not BurstWeaponBehaviorModel.
    #[serde(default)]
    pub over_time: bool,
    /// BurstWeaponBehaviorModel.alwaysEmptyClip: a partly used clip fires on ahead when ready.
    #[serde(default)]
    pub always_empty: bool,
    /// EmissionOverTimeModel.initialFireDelay x 60.
    #[serde(default)]
    pub delay: f32,
    /// On an AttackAirUnitModel: each shot leaves from the air unit (EmissionOverTime.Process).
    #[serde(default)]
    pub air: bool,
    /// BurstWeaponIncreasingArcBehavior.angleIncreases: each shot's RandomArc randomAngle.
    #[serde(default)]
    pub arcs: Vec<f32>,
    /// The EmissionOverTime's EmissionSupplyRandomTargetModels, in order: [minRadius, maxRadius,
    /// mapBorder, spawnProjectilesOnTarget (0 / 1)].
    #[serde(default)]
    pub supply: Vec<[f32; 4]>,
}

#[derive(Deserialize, Clone)]
pub struct BonusAfter {
    pub interval: u32,
    pub cooldown: i64,
    #[serde(default)]
    pub tower_pos: bool,
    pub projectile: Proj,
    pub emission: Emission,
}

#[derive(Deserialize, Clone)]
pub struct Alternate {
    pub interval: u32,
    pub projectile: Proj,
    pub emission: Emission,
}

#[derive(Deserialize, Clone)]
pub struct Emission {
    pub kind: String,
    /// AdoraEmissionModel.angleBetween: the bolts' own spacing rule (AdoraEmission.GetDirection).
    #[serde(default)]
    pub adora_between: Option<f32>,
    /// EmissionOverTimeModel: [count, timeBetween x 60, initialFireDelay x 60].
    #[serde(default)]
    pub over_time: Option<[f32; 3]>,
    /// Its EmissionSupplyRandomTargetModels: [minRadius, maxRadius, mapBorder, spawnOnTarget].
    #[serde(default)]
    pub supply: Vec<[f32; 4]>,
    /// EmissionClosestOffsetModel's ThrowMarkerOffsetModels [ejectX, ejectY, ejectZ, rotation]:
    /// the shot leaves from the one closest to the Target (Brickell's sea mines).
    #[serde(default)]
    pub closest: Vec<[f32; 4]>,
    /// ArcEmissionModel.useProjectileRotation.
    #[serde(default)]
    pub proj_rot: bool,
    /// SingleEmmisionTowardsTarget: the shot flies at its Target from the eject point, not along
    /// the rotation.
    #[serde(default)]
    pub towards: bool,
    /// AcidPoolEmissionModel.alwaysPoolOnCount (0: none).
    #[serde(default)]
    pub pool_every: u32,
    /// RandomEmissionModel: ejectPointRandomness, useSpeedMultiplier, speedMultiplierMin / Max.
    #[serde(default)]
    pub eject_random: f32,
    #[serde(default)]
    pub use_speed: bool,
    #[serde(default)]
    pub speed_min: f32,
    #[serde(default)]
    pub speed_max: f32,
    #[serde(default = "one_u")]
    pub count: u32,
    #[serde(default)]
    pub angle: f32,
    #[serde(default)]
    pub spread: f32,
    #[serde(default)]
    pub offset_start: f32,
    /// RandomTargetSpreadModel.spread: the aim point moves up to this far at random (Mortar).
    #[serde(default)]
    pub target_spread: f32,
    /// EmissionWithOffsetsModel: launch points (as WeaponModel.eject) the shots take in turn.
    #[serde(default)]
    pub markers: Vec<[f32; 3]>,
    /// EmissionWithOffsetsModel.randomRotationCone: each shot turned within this cone (degrees).
    #[serde(default)]
    pub cone: f32,
    /// EmissionWithOffsetsModel.rotateProjectileWithTower (false: shots go at the Target).
    #[serde(default = "yes")]
    pub with_tower: bool,
    /// RandomEmissionModel.useMainAttackRotation: from the main attack's rotation (the tower's).
    #[serde(default)]
    pub main_rot: bool,
    /// RandomArcEmissionModel.randomAngle: each shot turned by up to this many degrees.
    #[serde(default)]
    pub random_angle: f32,
    /// ArcEmissionModel.offset: degrees the whole arc is turned by. ParallelEmissionModel:
    /// its yOffset (and random_angle 1 when it is linear).
    #[serde(default)]
    pub offset: f32,
    /// MultiEmissionModel.secondaryProjectileModel: also fired from the tower each shot.
    #[serde(default)]
    pub secondary: Option<Box<Proj>>,
    /// EmissionRotationOffProjectileDirectionModel: (startingOffset, angleInBetween,
    /// alwaysCentre as 0 / 1).
    #[serde(default)]
    pub dir_off: Option<[f32; 3]>,
    /// EmissionArcRotationOffTowerDirectionModel.offsetRotation (Grape Shot).
    #[serde(default)]
    pub arc_off: Option<f32>,
    /// ArcEmissionModel.useAirUnitRotation: the arc is laid round the air unit's rotation.
    #[serde(default)]
    pub air_rot: bool,
    /// LineProjectileEmissionModel with none of its extras: collided at once along GetInLine.
    #[serde(default)]
    pub line_plain: bool,
    /// LineProjectileEmissionModel.useTowerRotation.
    #[serde(default)]
    pub line_tower_rot: bool,
    /// LineProjectileEmissionModel.ignoreBlockers.
    #[serde(default)]
    pub line_ignore_blockers: bool,
    /// emitFromTarget: the line starts on the Target and is emitFromTargetLength long.
    #[serde(default)]
    pub line_from_target: Option<f32>,
    /// useLengthSpeed: lengthPerFrame, the line's length growing toward the target each frame.
    #[serde(default)]
    pub line_grow: Option<f32>,
    /// projectileAtEndModel (emissionAtEndModel a Single): made where the line ends.
    #[serde(default)]
    pub line_end: Option<Box<Proj>>,
    /// useTargetAsEndPoint (the length follows the target); off: 400 (Ray of Doom).
    #[serde(default)]
    pub line_target_end: bool,
    /// projectileInitialHitModel: made on the first bloon the line meets and collided at once.
    #[serde(default)]
    pub line_initial: Option<Box<Proj>>,
    /// LineProjectileEmission.Emit's effects (visual): effectAtStart / effectAtEnd at the line's
    /// ends, effectAtCollision on up to `max_hits` of the bloons it hit.
    #[serde(default)]
    pub line_fx: Option<LineEmitFx>,
}

fn one_u() -> u32 {
    1
}

#[derive(Deserialize, Clone)]
pub struct Crit {
    pub damage: f32,
    pub lower: u32,
    #[serde(default)]
    pub upper: u32,
    /// CritMultiplierModel.displayModel: what a crit shot looks like.
    #[serde(default)]
    pub display: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct TagMod {
    /// One tag or a comma-separated list ("Lead, Ddt").
    pub tag: String,
    pub mult: f32,
    pub add: f32,
    /// mustIncludeAllTags: every listed tag is needed, not just one.
    #[serde(default)]
    pub all: bool,
}

/// A brew thrown at another tower (Alchemist: AddAcidicMixtureToProjectileModel,
/// AddBerserkerBrewToProjectileModel).
#[derive(Deserialize, Clone)]
pub struct Brew {
    /// Its mutator's BuffIndicatorModel: iconName, buffName (not stackable).
    #[serde(default)]
    pub buff_icon: Option<String>,
    #[serde(default)]
    pub buff_name: String,
    /// "acid" (extra damage by tag) or "berserk" (damage, pierce, rate and range).
    pub kind: String,
    pub id: String,
    /// Brews this one replaces (mutatorsToRemove).
    pub replaces: Vec<String>,
    /// Shots it lasts (cap) and seconds (lifespan; <= 0 = no time limit).
    pub shots: u32,
    /// The Check model's maxCount: shots a new brew gives, and what a re-brew adds (up to `shots`).
    #[serde(default)]
    pub max_count: u32,
    /// The targeting model's ignore list (TargetFriendlyModel / BrewTargettingModel).
    #[serde(default)]
    pub target_ignore: Vec<String>,
    pub lifespan: f32,
    pub damage: f32,
    pub pierce: f32,
    pub rate: f32,
    pub range: f32,
    pub tag_mods: Vec<TagMod>,
    /// Base tower ids it can't be thrown at.
    pub ignore: Vec<String>,
}

/// AcidPoolModel: a splash that hits nothing stays on the track as a pool.
#[derive(Deserialize, Clone)]
pub struct AcidPool {
    pub lifespan: f32,
    pub radius: f32,
    pub pierce: f32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ParentDamage {
    pub damage: f32,
    pub immune: u32,
}

#[derive(Deserialize, Clone)]
pub struct Bolts {
    pub displays: Vec<String>,
    pub life: f32,
}

#[derive(Deserialize, Clone, Default)]
pub struct LineEmitFx {
    pub start: Option<(String, f32)>,
    pub end: Option<(String, f32)>,
    pub hit: Option<(String, f32)>,
    #[serde(default)]
    pub max_hits: i64,
}

/// LineEffectModel: `beam` (display GUID) tiled every `len` along the shot, out to the map's
/// edge when `endless`; `start` / `end` effects (prefab names) at its ends.
#[derive(Deserialize, Clone)]
pub struct LineFx {
    pub beam: Option<String>,
    pub len: f32,
    pub endless: bool,
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct OnDestroy {
    pub projectile: Proj,
    pub emission: Emission,
    /// useTowerRotation: each projectile takes the tower's rotation.
    #[serde(default)]
    pub use_rot: bool,
}

#[derive(Deserialize, Clone)]
pub struct Exhaust {
    /// CreateProjectileOnExhaustFractionModel.collideOnSubProjectile: each new projectile hits the
    /// shot's still-valid Target at once (Projectile.CollideBloon).
    #[serde(default)]
    pub collide_sub: bool,
    /// (Runtime: the uid of the projectile it came from, for the process-list order.)
    #[serde(skip)]
    pub src: u64,
    /// (Runtime: that projectile's place in the process list, Projectile.born.)
    #[serde(skip)]
    pub src_born: u64,
    /// (Runtime: the bloon its ArriveAtTarget was flying at, when it went at the Target.)
    #[serde(skip)]
    pub arrive_target: Option<u64>,
    pub projectile: Proj,
    pub emission: Emission,
    /// Fires once the share of its pierce used reaches `fraction`, or (when >= 0) the share
    /// of its lifespan reaches `duration_fraction`; else when it goes.
    #[serde(default = "one")]
    pub fraction: f32,
    #[serde(default = "minus_one")]
    pub duration_fraction: f32,
    /// canCreateInBetweenRounds: else nothing comes of it while no round is on.
    #[serde(default)]
    pub between_rounds: bool,
    /// CreateDistanceProjectileOnExhaustFractionModel: (distanceInterval, piercePerInterval,
    /// radiusPerInterval) by distance flown.
    #[serde(default)]
    pub per_distance: Option<[f32; 3]>,
    /// CreateProjectileOnExpireModel: only when its lifespan runs out (Projectile.Expire), not
    /// when its pierce does (Projectile.Exhausted).
    #[serde(default)]
    pub expire: bool,
    /// CreateProjectileOnExpireModel.useTargetPosition: made where the shot's target is.
    #[serde(default)]
    pub use_target: bool,
    /// Runtime: the factory object (and aim offset) of a Target whose bloon is gone, looked up
    /// when the projectile is made: a child made this frame may have taken it over.
    #[serde(skip)]
    pub stale_target: Option<(u32, [f32; 2])>,
    /// Run time: the emitting projectile's Target collision-circle offset (Target.Position).
    #[serde(skip)]
    pub target_off: [f32; 2],
}

#[derive(Deserialize, Clone)]
pub struct Contact {
    /// CreateProjectileOnAbilityActivateModel.chance (> 0: one syncedRandom draw, fired when it is
    /// >= 1 - chance) and followTower.
    #[serde(default)]
    pub act_chance: f32,
    #[serde(default)]
    pub follow_tower: bool,
    pub projectile: Proj,
    pub emission: Emission,
    /// A map-only one made this far back along the shot's direction (ScaleProjectileOverTime
    /// .CollideMap: vsBlockerRadius x 3).
    #[serde(default)]
    pub back: f32,
    /// Runtime: the rotation an emission with useProjectileRotation starts from.
    #[serde(skip)]
    pub rot: Option<f32>,
    /// passOnCollidedWith: the sub-projectiles can't hit the bloon that triggered them.
    #[serde(default)]
    pub pass_on_collided: bool,
    /// dontCreateAtBloon: made where the shot is instead of on the bloon.
    #[serde(default)]
    pub at_shot: bool,
    /// passOnDirectionToContact: the new shots' Direction is Normalized(bloon - shot), 3D.
    #[serde(default)]
    pub pass_dir: bool,
    /// CreateProjectileOnBlockerCollideModel: only when the shot meets a map blocker.
    #[serde(default)]
    pub map_only: bool,
    /// CreateProjectileOnExhaustPierceModel.minimumTimeDifferenceInFrames / count (0: no gate).
    #[serde(default)]
    pub gap: f32,
    #[serde(default)]
    pub max_count: u32,
    /// CreateProjectileOnExhaustPierceModel.destroyProjectile: the shot Expires once `max_count`
    /// have been made (Collide @0x180ca4050).
    #[serde(default)]
    pub destroy_at_count: bool,
    /// Runs after the hit's damage (a later collisionPass than the DamageModel, or after it in
    /// the same pass): made where the shot is when that destroyed the bloon.
    #[serde(default)]
    pub after_damage: bool,
    /// EmitOnDamageModel (Shrapnel Shot): only on hits that deal damage.
    #[serde(default)]
    pub on_damage: bool,
    /// EmitOnPopModel: only on hits that pop the bloon.
    #[serde(default)]
    pub on_pop: bool,
}

/// AddTagToBloonModel: `tag` on the bloon's model for `frames`, `layers` layers, mutator id `id`.
#[derive(Deserialize, Clone)]
pub struct AddTag {
    pub tag: String,
    #[serde(default)]
    pub id: String,
    pub frames: i32,
    #[serde(default)]
    pub layers: u32,
}

/// FootprintModel: a rectangle's (xWidth, yWidth) (else a circle of the tower's radius),
/// doesntBlockTowerPlacement, ignoresTowerOverlap.
#[derive(Deserialize, Clone, Default)]
pub struct Footprint {
    #[serde(default)]
    pub rect: Option<[f32; 2]>,
    #[serde(default)]
    pub no_block: bool,
    #[serde(default)]
    pub ignore_overlap: bool,
}

/// RetargetOnContactModel (Bouncing Bullet): after each hit, jump to the closest bloon within
/// `distance` after `delay` seconds.
#[derive(Deserialize, Clone)]
pub struct Retarget {
    pub distance: f32,
    pub max_bounces: u32,
    pub delay: f32,
    pub expire_if_none: bool,
    /// Its TravelStrait comes first in the behaviours: a delayed bounce re-aims after the move.
    #[serde(default)]
    pub after_travel: bool,
}

#[derive(Deserialize, Clone)]
pub struct Proj {
    /// CreateProjectilesOnTrackOnExhaustFractionModel.range (Spike Paragon pile): at Initialise
    /// the path segments within it of the target are shuffled.
    #[serde(default)]
    pub track_exhaust_range: f32,
    /// DamageModifierUnstableConcoction (set as an Unstable Concoction explosion is made): extra
    /// damage on (non-MOAB, MOAB-class) bloons.
    #[serde(skip)]
    pub unstable_add: Option<(f32, f32)>,
    /// TravelStraitSlowdownModel: (slowdownSpeed, minSpeed, maxDistance, lifespan).
    #[serde(default)]
    pub slowdown: Option<[f32; 4]>,
    /// TravelCurvyModel: travels straight here, but draws a syncedRandom Next() as it's made.
    #[serde(default)]
    pub curvy: bool,
    /// TravelCurvyModel: (turnRate, maxTurnAngle).
    #[serde(default)]
    pub curvy_turn: Option<[f32; 2]>,
    /// RemoveBloonModifiersModel.cleanseRegen / cleanseFortified / cleanseLead.
    #[serde(default)]
    pub cleanse_regrow: bool,
    #[serde(default)]
    pub cleanse_fortified: bool,
    #[serde(default)]
    pub cleanse_lead: bool,
    /// RemoveBloonModifiersModel.bloonTagExcludeList / bloonTagExplicitList.
    #[serde(default)]
    pub cleanse_exclude: Vec<String>,
    #[serde(default)]
    pub cleanse_require: Vec<String>,
    /// ProjectileModel.id ("Collidable" for a Bloon Trap).
    #[serde(default)]
    pub pid: String,
    /// BrickellFreezeOtherMinesModel.freezeDuration: its collision freezes every mine of its
    /// tower (BrickellFreezeOtherMines.Collide @0x180c88a10).
    #[serde(default)]
    pub freeze_others: Option<f32>,
    /// RandomRangeTravelStraitModel (minRange, maxRange, speed): its lifespan is drawn when made.
    #[serde(default)]
    pub random_range: Option<[f32; 3]>,
    /// ProjectileModel.checkCollisionIntervalFrames ((int)(checkCollisionInterval x 60)).
    #[serde(default)]
    pub check_frames: i32,
    /// ProjectileModel.maxPierce: caps the starting pierce when above 0 (get_CappedPierce).
    #[serde(default)]
    pub max_pierce: f32,
    /// It has a DamageModel.
    #[serde(default)]
    pub damage_model: bool,
    /// DamageInRingRadiusModel.innerRingRadius: no damage nearer than radius x this (0: none).
    #[serde(default)]
    pub ring_inner: f32,
    /// Its DamageModel's name contains "FirstStrike".
    #[serde(default)]
    pub first_strike_dm: bool,
    /// DamageModel.distributeToChildren / overrideDistributeBlocker: whether damage past a
    /// layer goes on to its children (with the bloon's distributeDamageToChildren, or regardless).
    #[serde(default = "yes")]
    pub distribute: bool,
    #[serde(default)]
    pub distribute_override: bool,
    /// DistributeToChildrenBloonModifierModel: a hit bloon with one of these tags (any, when
    /// empty) passes this hit's overkill on to its children.
    #[serde(default)]
    pub distribute_set: Option<Vec<String>>,
    /// KnockbackModel: hit bloons go backwards for a while.
    #[serde(default)]
    pub knockback: Option<Knockback>,
    /// BloonSlapModel [valueForNormalBloons, valueForCeramicBloons]: what each slap counts
    /// against the projectile's pierce (maxPiercedAmount).
    #[serde(default)]
    pub slap: Option<[i32; 2]>,
    /// WindlashModel (Skywarden): see `Windlash`.
    #[serde(default)]
    pub windlash: Option<Box<Windlash>>,
    /// ChilledModel.chilledDuration (s): its stage mutator's life (see Game::pre_damage_effects).
    #[serde(default)]
    pub chill: Option<f32>,
    /// FilterWithChanceModel.filterChance.
    #[serde(default)]
    pub filter_chance: Option<f32>,
    /// Chilled.SlowNearby: see `ChillNearby`.
    #[serde(default)]
    pub chill_nearby: Option<ChillNearby>,
    /// Chilled.EmitIceShrapnel: see `ChillShrapnel`.
    #[serde(default)]
    pub chill_shrapnel: Option<std::sync::Arc<ChillShrapnel>>,
    /// LightningRodEmitModel (Skywarden x-5-x): see `RodEmit`.
    #[serde(default)]
    pub rod_emit: Option<Box<RodEmit>>,
    /// EmitOnDamageWithStateModel (Skywarden x-x-4+ FrozenRemains): see `StateEmit`.
    #[serde(default)]
    pub state_emits: Vec<StateEmit>,
    /// LightningRodModel: this projectile is a lightning rod (see `Rod`).
    #[serde(default)]
    pub rod: Option<Box<Rod>>,
    /// CollideReducePierceForBloonStateModel (collisionPass -1): Collide (VERIFIED-DECOMP
    /// @0x180c9cf60) draws a synced NextDouble on every hit, then adds extraPierce when the bloon
    /// has one of its states and percentChanceOfPierceReduce <= the draw; the models' 50 against
    /// a 0..1 draw never passes, so only the draw is kept (recording sky_300 rt 28).
    #[serde(default)]
    pub pierce_draw: bool,
    /// GalvanizedModel: [stage duration s, stunDuration s, stunDamage, stunSpeedMultiplier].
    #[serde(default)]
    pub galvanize: Option<[f32; 4]>,
    /// Galvanized.mutatorDurations: (bloon id, stun seconds) checked before stunDuration.
    #[serde(default)]
    pub galvanize_durations: Vec<(String, f32)>,
    /// GalvanizedModel's chain lightning (GalvanizedBehavior.OnBloonDamaged / EmitProjectile).
    #[serde(default)]
    pub galvanize_chain: Option<std::sync::Arc<GalvChain>>,
    /// It has a TravelStraitModel (GalvanizedBehavior.ProjectileFilter takes only those).
    #[serde(default)]
    pub travel_strait: bool,
    /// AgeModel.lifespan (seconds), when it has one.
    #[serde(default)]
    pub age_lifespan: Option<f32>,
    /// AgeRandomModel: lifespanFrames drawn in [min, max) frames as the shot is made.
    #[serde(default)]
    pub age_random: Option<[i32; 2]>,
    /// CreateEffectProjectileAfterTimeModel with useTargetPosition: its timeFrames.
    #[serde(default)]
    pub effect_after_target: Option<i32>,
    /// CreateEffectProjectileAfterTimeModel effects (display only; build_game_data.py).
    #[serde(default)]
    pub after_fx: Vec<AfterFx>,
    /// ScaleProjectileOverTimeModel's effect (display only).
    #[serde(default)]
    pub scale_fx: Option<ScaleFx>,
    pub pierce: f32,
    /// Shinobi Tactics' base: the pierce of the tower's first projectile with this one's id.
    #[serde(default)]
    pub shinobi_base: Option<f32>,
    pub radius: f32,
    /// ProjectileModel.vsBlockerRadius / ignoreBlockers: its radius against map blockers, and
    /// whether they stop it.
    #[serde(default)]
    pub vs_blocker_radius: f32,
    #[serde(default)]
    pub ignore_blockers: bool,
    /// ProjectileModel.canCollisionBeBlockedByMapLos.
    #[serde(default)]
    pub los_blocked: bool,
    /// ProjectileModel.ignoreNonTargetable.
    #[serde(default)]
    pub ignore_nontarget: bool,
    pub damage: f32,
    pub immune: u32,
    /// DetonateEffect (an ability's MutateProjectileOnAbility: Frostbite).
    #[serde(default)]
    pub detonate: Option<Detonate>,
    /// DamageModel.immuneBloonPropertiesOriginal: `immune` before a buff's immunity mask
    /// stripped it (None: never stripped).
    #[serde(skip)]
    pub immune_orig: Option<u32>,
    /// DamageModel.ignoreImmunityDestroy: survives hitting a bloon it can't damage (else that
    /// hit destroys it: Damage.Collide passes !ignoreImmunityDestroy to Bloon.Damage).
    #[serde(default)]
    pub immune_keep: bool,
    pub speed: f32,
    pub lifespan: f32,
    #[serde(default)]
    pub tag_mods: Vec<TagMod>,
    /// MapBorderReboundModel: bounces off the map's edges.
    #[serde(default)]
    pub rebound: bool,
    /// MapBorderReboundModel.clearCollidedWith: a bounce empties its hit list.
    #[serde(default)]
    pub rebound_clear: bool,
    /// ProjectileBlockerCollisionReboundModel: bounces off blocker areas' edges.
    #[serde(default)]
    pub blocker_rebound: Option<BlockerRebound>,
    /// CreateProjectilesOnTrackOnExpireModel: projectiles on path segments near it as it expires.
    #[serde(default)]
    pub track_expire: Option<Box<TrackExpire>>,
    /// CreateProjectileOnExhaustFractionModel: sub-projectiles spawned where it ends.
    #[serde(default)]
    pub on_exhaust: Vec<Exhaust>,
    /// CollideExtraPierceReductionModel: pierce each hit uses (1 + extraAmount).
    #[serde(default = "one")]
    pub pierce_cost: f32,
    /// ProjectileModel.usePointCollisionWithBloons: a bloon is met when its position is inside the
    /// projectile's radius (no bloon radius, no collision circles).
    #[serde(default)]
    pub point_collision: bool,
    /// DamagePercentOfMaxModel: (uint)(maxHealth x percent) more damage on bloons with all of
    /// `tags`; `offscreen`: only in the hit OnBloonSpawned makes as a bloon spawns.
    #[serde(default)]
    pub pct_max: Option<PctMax>,
    /// PierceUpTowers / HeatItUpDamageBuff: timed mutators for nearby towers as it expires.
    #[serde(default)]
    pub tower_buffs: Vec<TowerBuffOnExpire>,
    /// AddTagToBloonModel: a tag mutator put on each bloon it meets (before the damage).
    #[serde(default)]
    pub add_tags: Vec<AddTag>,
    /// MorphBloonModel.bloonId: what a hit bloon becomes.
    #[serde(default)]
    pub morph_to: Option<String>,
    /// CollideExtraPierceReduction without destroyProjectileIfPierceNotEnough: (tag, extra):
    /// such a bloon is only met while pierce >= extra + 1.
    #[serde(default)]
    pub pierce_tags_need: Vec<(String, f32)>,
    /// CollideExtraPierceReductionModel with a bloonTag: (tag, extra) on hits on such bloons.
    #[serde(default)]
    pub pierce_tags: Vec<(String, f32)>,
    /// ProjectileModel.ignorePierceExhaustion: stays at 0 pierce instead of being exhausted.
    #[serde(default)]
    pub keep_spent: bool,
    /// CreateProjectileOnContactModel: sub-projectiles spawned at each bloon hit (bomb explosions, frags).
    #[serde(default)]
    pub on_contact: Vec<Contact>,
    /// DisplayModel asset GUID (keys models/projectiles/index.json).
    #[serde(default)]
    pub display: Option<String>,
    /// The tower has LinkProjectileRadiusToTowerRangeModel: radius = attack range + `link_offset`
    /// (its projectileRadiusOffset; Ring of Fire 2).
    #[serde(default)]
    pub link_radius: bool,
    #[serde(default)]
    pub link_offset: f32,
    /// LinkProjectileRadiusToTowerRange also sets this projectile's radius (same id as the
    /// linked one): the tower's range + this offset.
    #[serde(default)]
    pub range_radius: Option<f32>,
    /// SpawnZombieOnBloonDestroyedModel.
    #[serde(default)]
    pub zombie: Option<Box<Zombie>>,
    /// DestroyProjectileIfTowerDestroyedModel: gone once its tower is.
    #[serde(default)]
    pub tower_bound: bool,
    /// InstantModel: hits its target bloon (FilterAllExceptTargetModel) the frame it is fired.
    #[serde(default)]
    pub instant: bool,
    /// InstantModel.destroyIfInvalid: it expires once its target is gone (else it stays).
    #[serde(default)]
    pub instant_destroy: bool,
    /// InstantModel.dontFollowTarget: placed on its target once only.
    #[serde(default)]
    pub instant_once: bool,
    /// InstantModel.ignoreTargetZ: Instant.Process sets x and y only, keeping its height.
    #[serde(default)]
    pub instant_keep_z: bool,
    /// Effects (prefab under effects/, scale), visual only: CreateEffectOnContactModel at each
    /// hit, CreateEffectOnExhausted / ExhaustFraction when its pierce runs out,
    /// CreateEffectOnExpireModel when it goes.
    #[serde(default)]
    pub contact_fx: Vec<(String, f32)>,
    #[serde(default)]
    pub exhaust_fx: Vec<(String, f32)>,
    /// CreateEffectOnExhaustFractionModel: (effect, scale, fraction of the lifespan, ends with the
    /// projectile) (display only; main.rs plays them).
    #[serde(default)]
    pub fraction_fx: Vec<(String, f32, f32, bool)>,
    #[serde(default)]
    pub expire_fx: Vec<(String, f32)>,
    /// ProjectileCreateEffectOnEmitModel: effects where it's emitted (display only).
    #[serde(default)]
    pub emit_fx: Vec<(String, f32)>,
    /// CreateLightningEffectModel: bolt displays drawn along each jump, for `life` seconds.
    #[serde(default)]
    pub bolts: Option<Bolts>,
    #[serde(default)]
    pub retarget: Option<Retarget>,
    /// LightningModel: splitRange, splits, targetLast (1), delay (s).
    #[serde(default)]
    pub lightning_split: Option<[f32; 4]>,
    /// FollowPathModel (boomerangs): a fixed path, x right and y forward of the throw.
    #[serde(default)]
    pub path: Option<FollowPath>,
    /// ArriveAtTargetModel: flies to the target's position in `time` seconds.
    #[serde(default)]
    pub arrive: Option<Arrive>,
    #[serde(default)]
    pub cash: Option<Cash>,
    /// PushBackModel: hit bloons with `tag` are pushed back along the track.
    #[serde(default)]
    pub push: Option<PushBack>,
    /// ClearHitBloonsModel.interval: the hit list is forgotten every this many seconds.
    #[serde(default)]
    pub clear_hits: Option<f32>,
    /// ClearHitBloonsWhenNoLongerCollidingModel: forgets bloons that stop touching it.
    #[serde(default)]
    pub clear_off: bool,
    /// FilterOutBloonModel ids: bloons this projectile ignores.
    #[serde(default)]
    pub exclude_bloons: Vec<String>,
    /// FilterMutatedTargetModel: it ignores bloons mutated by any of these (`Bloon::mutated_by`),
    /// or with `need_mutated` (inverse) those not mutated by every one.
    #[serde(default)]
    pub skip_mutated: Vec<String>,
    #[serde(default)]
    pub need_mutated: Vec<String>,
    /// FilterAllModel: hits nothing (a Mortar shell in flight; its explosion does the damage).
    #[serde(default)]
    pub no_hit: bool,
    /// ProjectileModel.dontUseCollisionChecker: never in CollisionChecker.Process (Bomb Blitz's
    /// Cluster fragments; instant shots hit through their own path).
    #[serde(default)]
    pub no_checker: bool,
    /// LightningModel: Lightning.Process hits the bloons itself (VERIFIED-DISASM @0x180cc84d0:
    /// BloonManager.GetTargets, then Projectile.CollideBloons), dontUseCollisionChecker or not.
    #[serde(default)]
    pub lightning: bool,
    /// RotateModel.angle: display spin in degrees per second.
    #[serde(default)]
    pub spin: f32,
    /// Projectile FilterOutTagModel tags (Ice Monkey: "Moabs").
    #[serde(default)]
    pub exclude_tags: Vec<String>,
    /// FilterWithTagModel on the projectile: every tag here must be on the bloon.
    #[serde(default)]
    pub require_tags: Vec<String>,
    /// FilterGlueLevelModel.glueLevel: skips bloons already glued at this level or higher (0 = off).
    #[serde(default)]
    pub glue_filter: u32,
    /// FilterFrozenBloonsModel: skips frozen bloons.
    #[serde(default)]
    pub skip_frozen: bool,
    /// ProjectileFilterModel's filter types (FilterInvisibleModel: no camo bloons without
    /// camo detection).
    #[serde(default)]
    pub filters: Vec<String>,
    /// FilterBloonIfDamageTypeModel.ifCantHitBloonProperties: bloons with these properties
    /// that the damage is immune to are skipped.
    #[serde(default)]
    pub cant_hit: u32,
    /// Effects put on the bloons it hits (SlowModel, FreezeModel, DamageOverTime, ...).
    #[serde(default)]
    pub effects: Vec<EffectSpec>,
    /// SlowModifierForTagModel / FreezeModifierForTagsModel.
    #[serde(default)]
    pub effect_mods: Vec<EffectMod>,
    /// AddBehaviorModifierForTagModel: an AddBehaviorToBloon mutator's lifespan x mult on a
    /// bloon with the tag (mutatorId equal to its mutationId).
    #[serde(default)]
    pub ab_mods: Vec<AbMod>,
    /// TrackTargetModel / TrackTargetWithinTimeModel: homes in on its target (Seeking Shuriken).
    #[serde(default)]
    pub seek: Option<Seek>,
    /// WindModel: blows bloons back along the track (Distraction).
    #[serde(default)]
    pub wind: Option<Wind>,
    /// FilterOveridingMutatedTargetModel: (mutationOverideOrder, highestPriorityMutationId,
    /// defaultMutationId).
    #[serde(default)]
    pub override_filter: Option<(Vec<String>, String, String)>,
    /// DamageModifierForCashAmountModel: (multiplier, additive, cash threshold).
    #[serde(default)]
    pub cash_damage: Vec<[f32; 4]>,
    /// AccelerateModel: (acceleration, maxSpeed) in units per second.
    #[serde(default)]
    pub accel: Option<[f32; 4]>,
    /// RemoveBloonModifiersModel.cleanseCamo: damaged bloons lose camo (Counter-Espionage).
    #[serde(default)]
    pub decamo: bool,
    /// cleanseOnlyIfDamaged (Shimmer: false).
    #[serde(default = "yes")]
    pub decamo_needs_damage: bool,
    /// DamageModifierForBloonStateModel: extra damage to bloons in any of these states.
    #[serde(default)]
    pub state_mods: Vec<StateMod>,
    /// DamageModifierForBloonTypeModel: extra damage to one bloon type (with its variants if `children`).
    #[serde(default)]
    pub type_mods: Vec<TypeMod>,
    /// DamageModifierWrathModel (Avatar of Wrath).
    #[serde(default)]
    pub wrath: Option<Wrath>,
    /// SetSpriteFromPierceModel: display GUIDs picked by pierce left (spike piles shrink as used).
    #[serde(default)]
    pub pierce_sprites: Vec<String>,
    /// AgeModel.useRoundTime: the lifespan only runs while a round is on (spikes).
    #[serde(default)]
    pub round_time: bool,
    /// AgeModel.rounds: round ends it survives (Long Life Spikes); otherwise cleared at round end.
    #[serde(default)]
    pub rounds: u32,
    /// CreateEffectFromCollisionToCollisionModel: (effect prefab, effectLength), display only.
    #[serde(default)]
    pub bounce_tracer: Option<(String, f32)>,
    /// JungleVineEffectModel.ageingDestroyModel.timeFrames: added to the grab's limiter frames.
    #[serde(default)]
    pub vine_frames: Option<i64>,
    /// JungleVineEffectModel's displays (front, back effect prefabs), display only.
    #[serde(default)]
    pub vine_display: Option<(String, String)>,
    /// CreateGreatWhiteEffectModel: its hit grabs and pulls the bloon (Beast Handler 3xx+).
    #[serde(default)]
    pub great_white: Option<Box<GreatWhite>>,
    /// LatchToBloonModel: it rides the bloon it is made on (the Great White's BloonFollowProjectile).
    #[serde(default)]
    pub latch: Option<Latch>,
    /// RefreshPierceModel.interval: pierce is restored every this many seconds (Druid vines).
    #[serde(default)]
    pub refresh_pierce: Option<f32>,
    /// RefreshPierceModel.isBuffedByRate: intervalFrames x the tower's GetTotalRateModifier.
    #[serde(default)]
    pub refresh_pierce_rate: bool,
    /// RemoveMutatorsFromBloonModel keys: effects whose id starts with one are removed (Tornado).
    #[serde(default)]
    pub remove_effects: Vec<String>,
    /// PsiEffectModel: [damagePerTick, tickRate, superCeramicPenalty, maxDurationFrames].
    #[serde(default)]
    pub psi: Option<[f32; 4]>,
    /// SilasTrackTargetModel: [minSpeedFrames, maxSpeedFrames, accelerationFrames, lifespanFrames,
    /// waitFrames].
    #[serde(default)]
    pub silas: Option<[f32; 5]>,
    /// SilasApplyRimeModel's freeze and explosion (on a bloon already rimed).
    #[serde(default)]
    pub silas_rime: Option<std::sync::Arc<SilasRime>>,
    /// PsiEffectModel.projectileOnBloonDestroyModel / emissionOnBloonDestroy.
    #[serde(default)]
    pub psi_destroy: Option<std::sync::Arc<Contact>>,
    /// CreateProjectileOnIntervalModel: every `frames` frames, a sub-projectile at a random bloon
    /// within `range` (Ball Lightning).
    #[serde(default)]
    pub on_interval: Option<Box<OnInterval>>,
    #[serde(default)]
    pub acid_pool: Option<AcidPool>,
    /// CreateSoundOnProjectileCollisionModel / ...ExpireModel / ...ExhaustModel clips.
    #[serde(default)]
    pub hit_sounds: Vec<String>,
    #[serde(default)]
    pub expire_sounds: Vec<String>,
    #[serde(default)]
    pub exhaust_sounds: Vec<String>,
    /// CreateSoundOnPickupModel: played when its cash is collected.
    #[serde(default)]
    pub pickup_sounds: Vec<String>,
    /// ShowTextOnHitModel: "Crit!" over what it hits.
    #[serde(default)]
    pub hit_text: bool,
    /// ExpireProjectileAtScreenEdgeModel: gone when it leaves the map.
    #[serde(default)]
    pub expire_at_edge: bool,
    /// CreateTowerModel / CreateTypedTowerModel: sub-towers placed where it lands (Engineer
    /// sentries); typed ones take turns.
    #[serde(default)]
    pub create_tower: Vec<std::sync::Arc<SubTowerSpec>>,
    /// CreateTowerModel.positionAtTarget: the sub-tower goes on the projectile's target point.
    #[serde(default)]
    pub create_at_target: bool,
    /// CreateTowerModel.useParentTargetPriority: the sub-tower takes its parent's target type.
    #[serde(default)]
    pub create_parent_target: bool,
    /// CreateTowerModel.useProjectileRotation (or a typed tower): the tower takes the projectile's rotation.
    #[serde(default)]
    pub create_proj_rot: bool,
    #[serde(default)]
    pub eat: Option<Eat>,
    /// ScaleDamageWithTimeModel [baseDamage, scalePerSecond, maxDamage].
    #[serde(default)]
    pub scale_damage: Option<[f32; 3]>,
    /// ScaleProjectileOverTimeModel [baseScale, maxScale, scalePerSecond].
    #[serde(default)]
    pub scale_over_time: Option<[f32; 3]>,
    /// FallToGroundModel: (timeToTake, groundOffset, expireOnContact).
    #[serde(default)]
    pub fall: Option<(f32, f32, bool)>,
    /// AdoraTrackTargetModel (Neva-Miss darts).
    #[serde(default)]
    pub adora: Option<AdoraSpec>,
    /// FilterAllExceptTargetModel: hits only its target bloon.
    #[serde(default)]
    pub only_target: bool,
    /// CollideOnlyWithTargetModel.collideWithFirstIfInvalidTarget.
    #[serde(default)]
    pub first_if_invalid: bool,
    /// TravelAlongPathModel: moves along the track.
    #[serde(default)]
    pub track_travel: Option<TrackTravel>,
}

/// EatBloonModel (Bloon Trap): bloons it touches vanish whole until `capacity` RBE is inside.
#[derive(Deserialize, Clone)]
pub struct Eat {
    pub capacity: f32,
    pub cash_mult: f32,
    /// The Full trap's lifespan (seconds; EatBloonModel.projectile's AgeModel).
    #[serde(default)]
    pub full_lifespan: Option<f32>,
    /// The Full trap's CashModel / PickupModel (EmitCash overwrites its min / max).
    #[serde(default)]
    pub full_cash: Option<Cash>,
    /// effectOnEatModel (prefab, scale), played at the trap on each eat (visual).
    #[serde(default)]
    pub fx: Option<(String, f32)>,
}

/// A temporary tower spawned by a projectile (Sentry): its own tier, gone after `lifespan` seconds.
/// AbilityModel: a button that, off cooldown, turns on extra attacks and / or a rate boost.
#[derive(Deserialize, Clone)]
pub struct Ability {
    /// ToggleEffectWhileAbilityActiveModel: (effect while active, effect otherwise), display only.
    #[serde(default)]
    pub toggle_fx: Option<(Option<String>, Option<String>)>,
    /// CreateEffectOnAbilityModel: the effect prefabs played on the tower as it's used, (name, scale).
    #[serde(default)]
    pub fx: Vec<(String, f32)>,
    /// AbilityModel.activateOnRoundEnd: used by itself as each round ends.
    #[serde(default)]
    pub round_end: bool,
    /// CashPerTowerInRangeModel (Jungle's Bounty).
    #[serde(default)]
    pub bounty: Option<Bounty>,
    /// BonusLivesOnAbilityModel.amount.
    #[serde(default)]
    pub lives: f64,
    /// CreateProjectileOnAbilityActivateModel: fired from the tower when it's used.
    #[serde(default)]
    pub on_activate: Vec<Contact>,
    /// FrozenCascadeModel (Silas's Frozen Cascade).
    #[serde(default)]
    pub cascade: Option<std::sync::Arc<Cascade>>,
    /// FrozenBurialModel (Silas's Frozen Burial: Ice Walls along the track).
    #[serde(default)]
    pub burial: Option<std::sync::Arc<Burial>>,
    /// ActivateAbilitiesOnAbilityModel.abilityToFind: used along with this one.
    #[serde(default)]
    pub also: Option<String>,
    /// AbilityReduceCooldownOnLivesLostModel: (reductionSeconds, delaySeconds).
    #[serde(default)]
    pub leak_cdr: Option<(f32, f32)>,
    #[serde(default)]
    pub biohack: Option<Biohack>,
    #[serde(default)]
    pub barrage: Option<Barrage>,
    /// SwordChargeModel (Sauda's Sword Charge): see `SwordCharge`.
    #[serde(default)]
    pub track_charge: Option<SwordCharge>,
    /// LeapingSwordModel: an impact projectile that lands after `delay`.
    #[serde(default)]
    pub leap: Option<Leap>,
    /// PlaceProjectileAtModel (Mega Mine): placed at a picked point.
    #[serde(default)]
    pub place_at: Option<PlaceAt>,
    /// ActivateLightningRodZapModel (Skywarden Thunder Charge).
    #[serde(default)]
    pub rod_zap: Option<RodZap>,
    /// SyphonFundingModel.lifespan (seconds).
    #[serde(default)]
    pub syphon: f32,
    /// Timed changes to the tower's projectiles (ChangeDamageTypeModel,
    /// MutateProjectileOnAbilityModel).
    #[serde(default)]
    pub boosts: Vec<ProjBoost>,
    /// MOAB / ZOMG Takedown (MoabTakedownModel + TargetGrapplableModel).
    #[serde(default)]
    pub takedown: Option<Takedown>,
    /// FinalStrikeModel (the Navarch's Final Strike).
    #[serde(default)]
    pub final_strike: Option<FinalStrike>,
    pub name: String,
    /// Seconds.
    pub cooldown: f32,
    /// AbilityModel.addedViaUpgrade: the upgrade whose icon the button shows.
    #[serde(default)]
    pub upgrade: Option<String>,
    /// AbilityModel.startOffCooldown.
    #[serde(default)]
    pub ready: bool,
    /// AbilityModel.activateOnPreLeak: it goes off (if ready) when a bloon is about to leak.
    #[serde(default)]
    pub pre_leak: bool,
    /// AbilityModel.activateOnLivesLost: Ability.OnLivesLost activates it (Retaliation).
    #[serde(default)]
    pub on_lives_lost: bool,
    /// The tower's attacks listed before it in the model's behaviours.
    #[serde(default = "big_usize")]
    pub attack_pos: usize,
    /// AbilityModel.maxActivationsPerRound (0 or less: no limit).
    #[serde(default)]
    pub per_round: i32,
    /// AbilityModel.resetCooldownOnTierUpgrade: it starts on its full cooldown.
    #[serde(default)]
    pub full_start: bool,
    /// AbilityModel.cooldownSpeedScale: Ability.Process (@0x180dfafc0) takes 1 / (1 - it) off
    /// the cooldown a frame.
    #[serde(default)]
    pub speed_scale: f32,
    /// AbilityModel.canActivateBetweenRounds: usable while no round is running.
    #[serde(default)]
    pub between_rounds: bool,
    /// A ToggleableAbilityBehavior that starts toggled off: Ability.CanUseAbility is false for it.
    #[serde(default)]
    pub toggled_off: bool,
    #[serde(default)]
    pub activate: Vec<ActivateAttack>,
    #[serde(default)]
    pub turbo: Option<Turbo>,
    /// BrickellFreezeMinesAbilityBuffModel (Blast Chain): [multiplier (freeze duration),
    /// projectileSpeedMultiplier (SeaMineSeeking), lifespan].
    #[serde(default)]
    pub freeze_mines: Option<[f32; 3]>,
    /// AbilityDamageAllModel (Bomb Blitz's Bomb Storm): damage to every bloon with (inclusive) or
    /// without one of the tags.
    #[serde(default)]
    pub damage_all: Vec<AbilityDamageAll>,
    /// Timed support zones (Call to Arms, Absolute Zero's attack-speed boost...).
    #[serde(default)]
    pub supports: Vec<Support>,
    /// ImfLoanModel.
    #[serde(default)]
    pub loan: Option<Loan>,
    #[serde(default)]
    pub ammo: Option<AmmoSwap>,
    /// MonkeyFanClubModel.
    #[serde(default)]
    pub fan_club: Option<FanClub>,
    /// MorphTowerModel (Total Transformation).
    #[serde(default)]
    pub morph: Option<Morph>,
    /// MorphTowerModel with mutateSelf: the tower pauses for `delay` frames, then becomes `tier`.
    #[serde(default)]
    pub morph_self: Option<std::sync::Arc<MorphSelf>>,
    /// OverclockModel: the ability is used on a tower the player picks.
    #[serde(default)]
    pub overclock: Option<Overclock>,
    /// ImmunityModel.lifespan (seconds): leaks cost no lives.
    #[serde(default)]
    pub immunity: f32,
    /// ImmunityModel.effectModel (prefab, scale): at each track's end while it lasts (display only).
    #[serde(default)]
    pub immunity_fx: Option<(String, f32)>,
    /// DarkshiftModel: teleport to a picked spot.
    #[serde(default)]
    pub darkshift: Option<Darkshift>,
    /// IncreaseRangeModel: range x `mult` + `add` for `frames`.
    #[serde(default)]
    pub range_up: Option<RangeUp>,
    /// AbilityCreateTowerModel (Summon Phoenix).
    #[serde(default)]
    pub create_tower: Vec<std::sync::Arc<SubTowerSpec>>,
    #[serde(default)]
    pub switch: Option<SwitchDisplay>,
    /// MutateRemoveAllAttacksOnAbilityActivateModel.lifespan (seconds).
    #[serde(default)]
    pub mute: f32,
    #[serde(default)]
    pub sounds: Vec<String>,
}

/// MonkeyFanClubModel: up to `count` towers of the same kind within `range` whose upgrades are all
/// at most `max_tier` become fans for `lifespan` seconds: weapon intervals x `rate`, +`pierce`,
/// +`damage`, and immuneBloonProperties `immune`.
#[derive(Deserialize, Clone)]
pub struct FanClub {
    /// Display only: the effect on each fan as it turns, as it turns back, and on it meanwhile.
    #[serde(default)]
    pub start_fx: Option<String>,
    #[serde(default)]
    pub end_fx: Option<String>,
    #[serde(default)]
    pub fan_fx: Option<String>,
    /// MonkeyFanClubModel.effectLeaderId: on the tower that used it instead (display only).
    #[serde(default)]
    pub leader_fx: Option<String>,
    /// MonkeyFanClub.Mutator (VERIFIED-DISASM @0xe61060) sets each fan's projectile radius to
    /// projectileRadius and its weapons' eject point to ejectX / Y / Z.
    #[serde(default)]
    pub projectile_radius: f32,
    #[serde(default)]
    pub eject: [f32; 3],
    pub lifespan: f32,
    pub max_tier: u8,
    pub count: u32,
    pub range: f32,
    pub rate: f32,
    pub immune: u32,
    #[serde(default)]
    pub pierce: f32,
    #[serde(default)]
    pub damage: f32,
    /// towerDisplay: the fans' model (models/fans/<prefab>).
    #[serde(default)]
    pub display: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct OrbitTower {
    /// OrbitingTowerModel.offset: added to the game's y (recording wizard_paragon_r40).
    #[serde(default)]
    pub offset: f32,
    pub radius: f32,
    pub rate: f32,
}

/// SpiritOfTheForestModel: within `close` of the Druid zone 0, within `middle` zone 1, else 2.
#[derive(Deserialize, Clone)]
pub struct Spirit {
    pub close: f32,
    pub middle: f32,
    /// offsetMax, circleRadius, generateRadius, amountPerFrame.
    #[serde(default)]
    pub offset: f32,
    #[serde(default)]
    pub circle: f32,
    #[serde(default)]
    pub step: f32,
    #[serde(default)]
    pub per_frame: f32,
    pub zones: Vec<SpiritZone>,
    /// AddObjectAtPosition's effect prefabs [close, middle, far] and its rotation / scale ranges
    /// (display only).
    #[serde(default)]
    pub objects: Vec<Vec<Option<String>>>,
    #[serde(default)]
    pub rot: [f32; 2],
    #[serde(default)]
    pub obj_scale: [f32; 2],
    /// The objects take their position's segment rotation (SpikeParagonDamageZone), display only.
    #[serde(default)]
    pub pos_rot: bool,
}

/// DamageOverTimeCustomModel: `damage` every `interval` s (x mult + add on `tags`).
#[derive(Deserialize, Clone)]
pub struct SpiritZone {
    /// The zone's mutator id (mutatorId, or the zone model's name).
    #[serde(default)]
    pub id: String,
    pub damage: f32,
    pub interval: f32,
    pub immune: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "one")]
    pub mult: f32,
    #[serde(default)]
    pub add: f32,
    /// Its DamageOverTimeCustomModel's DamageModifierForTagModels.
    #[serde(default)]
    pub tag_mods: Vec<TagMod>,
}

fn morph_max_cost() -> f32 {
    9999999.0
}

/// MorphTowerModel (mutateAll): `max` towers on `affect` (none with an `ignore` mutator, no
/// path above `max_tier`) become `tier` with model `display` for `lifespan` seconds.
#[derive(Deserialize, Clone)]
pub struct Morph {
    pub max: u32,
    pub max_tier: u8,
    pub affect: Vec<String>,
    #[serde(default)]
    pub ignore: Vec<String>,
    /// maxCost: a tower worth more is skipped (9999999 in every model).
    #[serde(default = "morph_max_cost")]
    pub max_cost: f32,
    pub lifespan: f32,
    pub tier: std::sync::Arc<Tier>,
    #[serde(default)]
    pub display: Option<String>,
}

/// FrozenCascadeModel: towers of these baseIds (towerId, comma separated) explode with
/// `projectile` as a ring grows from the activating tower at `time_scale` a frame.
#[derive(Deserialize)]
pub struct Cascade {
    pub bases: Vec<String>,
    pub max: usize,
    pub time_scale: f32,
    pub projectile: Proj,
    pub emission: Emission,
}

/// FrozenBurialModel: icewallSpacing, icewallDelay, timePerIcewall (frames), the wall.
#[derive(Deserialize)]
pub struct Burial {
    pub spacing: i32,
    pub delay: i64,
    pub per: i64,
    pub projectile: Proj,
    pub emission: Emission,
}

#[derive(Deserialize, Clone)]
pub struct TypeBonus {
    pub masks: Vec<u32>,
    /// useOriginalType: the hit's immuneBloonPropertiesOriginal, else its current mask.
    #[serde(default)]
    pub original: bool,
}

#[derive(Deserialize)]
pub struct SilasRime {
    pub rime_id: String,
    pub freeze_id: String,
    pub frames: i32,
    pub layers: u32,
    pub immune: u32,
    #[serde(default)]
    pub moabs: bool,
    #[serde(default)]
    pub explosion: Option<Contact>,
}

#[derive(Deserialize)]
pub struct MorphSelf {
    pub tier: std::sync::Arc<Tier>,
    /// morphDelayFrames.
    pub delay: i64,
    /// Seconds; -1: for good.
    pub lifespan: f32,
    #[serde(default)]
    pub display: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct Loan {
    pub amount: f64,
    /// incomeRecoveryRate: share of income that repays it.
    pub rate: f64,
}

/// SniperAmmoSwapBehaviorModel: after `delay` seconds the tower's next `shots` shots are boosted.
#[derive(Deserialize, Clone)]
pub struct AmmoSwap {
    pub delay: f32,
    pub shots: u32,
    pub pierce: f32,
    pub damage_mult: f32,
    pub lifespan_mult: f32,
    /// newImmuneBloonProperties when overrideDamageType.
    pub immune: Option<u32>,
    /// mainMonkeyAbilityEffect (prefab, scale): on the tower when the ammo arrives (display only).
    #[serde(default)]
    pub fx: Option<(String, f32)>,
}

/// ActivateAttackModel: `attacks` run for `lifespan` seconds (one volley when `one_shot`).
#[derive(Deserialize, Clone)]
pub struct ActivateAttack {
    pub lifespan: f32,
    pub one_shot: bool,
    /// processOnActivate: the attacks act at the activation itself.
    #[serde(default)]
    pub process_on_activate: bool,
    /// turnOffExisting: the tower's own attacks pause meanwhile.
    pub turn_off_existing: bool,
    pub attacks: Vec<Attack>,
    /// Seconds before the attacks start.
    #[serde(default)]
    pub delay: f32,
    /// MarkedToPopModel: [markingTimeFrames, executionTimeMaxFrames]; attacks [marking, execution].
    #[serde(default)]
    pub mtp: Option<[f32; 2]>,
}

/// One AbilityDamageAllModel (see Ability::damage_all).
#[derive(Deserialize, Clone)]
pub struct AbilityDamageAll {
    pub damage: i32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub inclusive: bool,
}

/// TurboModel: weapon intervals x `mult`, +`extra_damage`, for `lifespan` seconds.
#[derive(Deserialize, Clone)]
pub struct Turbo {
    pub lifespan: f32,
    pub mult: f32,
    pub extra_damage: f32,
    /// projectileDisplay: the weapons' projectiles' display while it runs (display only).
    #[serde(default)]
    pub display: Option<String>,
}

/// ToggleFocusStanceModel: in Focus stance range x `range`, weapon intervals x `rate`, projectile
/// lifespans x `lifespan`, and a LongWeapon fires in place of the others.
#[derive(Deserialize, Clone)]
pub struct Stance {
    pub range: f32,
    pub rate: f32,
    pub lifespan: f32,
}

/// StatePoppedBasedPierceModel: OnBloonDamaged on a bloon in one of `states` puts PierceMutator
/// on the tower (and its sub-towers) for `duration`.
#[derive(Deserialize, Clone)]
pub struct StatePierce {
    /// bloonStates: a damaged bloon with a mutator whose id contains one of these.
    pub states: Vec<String>,
    /// buffDuration (s).
    pub duration: f32,
    pub mult: f32,
    pub add: f32,
    pub cap: f32,
    /// mustBeModified: the mutator must also have mutated the bloon's model.
    pub must_mod: bool,
}

/// DamageBasedAttackSpeedModel: +`increase` attack speed per `threshold` damage dealt, up to
/// `max_stacks`; all lost after `idle_frames` frames without damage.
#[derive(Deserialize, Clone)]
pub struct DamageSpeed {
    pub threshold: f32,
    pub increase: f32,
    pub max_stacks: u32,
    pub idle_frames: u32,
    /// damageCap: OnBloonDamaged counts at most this per hit (0: no cap).
    #[serde(default)]
    pub cap: f32,
}

/// GyrfalconPatternModel: the bird waits `cooldown` seconds, dives at a bloon (maxSpeed x
/// bloonTargetSpeedMultiplier), grabs bloons within `radius` of it until `amount` is used up (each
/// costs 1 + its type's penalty), hits them for `initial` and then every `rate` seconds for
/// `grapple` (+ the MOAB / regrow modifiers), holding them still.
#[derive(Deserialize, Clone)]
pub struct Grapple {
    pub speed: f32,
    /// extraGrabAmount: added to `amount` (a merge sets it).
    #[serde(default)]
    pub extra: f32,
    /// rotationSpeed (display only here).
    #[serde(default)]
    pub rot_speed: f32,
    /// The attack's filters.
    #[serde(default)]
    pub exclude_tags: Vec<String>,
    #[serde(default)]
    pub exclude_bloons: Vec<String>,
    #[serde(default)]
    pub camo_filter: bool,
    #[serde(default)]
    pub immune: u32,
    #[serde(default)]
    pub snap: f32,
    #[serde(default)]
    pub y_offset: f32,
    #[serde(default)]
    pub moab_scale: f32,
    #[serde(default)]
    pub idle_min: i32,
    #[serde(default)]
    pub idle_max: i32,
    pub dive_mult: f32,
    pub cooldown: f32,
    pub rate: f32,
    pub initial: f32,
    pub grapple: f32,
    pub initial_moab: f32,
    pub grapple_moab: f32,
    pub regrow_add: f32,
    pub amount: u32,
    pub radius: f32,
    pub keep: bool,
    /// enableAdjacentHover: the bird may hover over a handler contributing it (GetTowerTarget).
    #[serde(default)]
    pub adjacent: bool,
    pub keep_range: f32,
    /// Extra grab cost per bloon tag (Lead, Ceramic, Moab, Bfb, Zomg, Ddt).
    pub penalty: std::collections::HashMap<String, u32>,
}

/// HeliMovementModel: what HeliMovement.ApplyMovement, UpdateRotation and GetRepulsionVector read.
#[derive(Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct HeliSpec {
    pub maxSpeed: f32,
    pub rotationSpeed: f32,
    pub strafeDistanceSquared: f32,
    pub otherHeliRepulsionRange: f32,
    pub movementForceStart: f32,
    pub movementForceEnd: f32,
    pub otherHeliRepulsonForce: f32,
    pub slowdownRadiusMax: f32,
    pub slowdownRadiusMin: f32,
    pub slowdownRadiusMinSquared: f32,
    pub minVelocityCapScale: f32,
}

#[derive(Deserialize, Clone)]
pub struct AirSpec {
    /// Prefab name under `models/air/`.
    pub display: Option<String>,
    /// HeliMovementModel.maxSpeed / PathMovementModel.speed (units/s).
    pub speed: f32,
    /// HeliMovementModel.movementForceStart (units/s²).
    #[serde(default)]
    pub accel: f32,
    /// HeliMovementModel.slowdownRadiusMax: slows down within this distance of its goal.
    #[serde(default)]
    pub slowdown: f32,
    /// PathMovementModel.catchUpSpeed: speed multiplier while rejoining its path.
    #[serde(default)]
    pub catch_up: f32,
    /// Monkey Ace flight patterns (selectable *PatternModels); empty for a Heli.
    #[serde(default)]
    pub patterns: Vec<AirPattern>,
    /// FighterMovementModel: maxSpeed, turningSpeed, minDistanceToTargetBeforeFlyover,
    /// distanceOfFlyover (Aircraft Carrier planes).
    #[serde(default)]
    pub fighter: Option<[f32; 4]>,
    /// HeliMovementModel (a Heli); None for the other air units.
    #[serde(default)]
    pub heli: Option<HeliSpec>,
    /// PathMovementModel.rotation: degrees per second it can turn while rejoining its path.
    #[serde(default = "full_turn")]
    pub turn: f32,
    /// PathMovementModel.bankRotation (max degrees) and bankRotationMul.
    #[serde(default)]
    pub bank: f32,
    #[serde(default)]
    pub bank_mul: f32,
    #[serde(default)]
    pub takeoff: Option<TakeOff>,
}

fn full_turn() -> f32 {
    360.0
}

/// PathMovementModel take-off: takeOffTime / takeOffExponent, and the display's
/// takeOffAnimTime, takeOffScale(Exponent), takeOffPitch(Exponent).
#[derive(Deserialize, Clone)]
pub struct TakeOff {
    pub time: f32,
    pub exp: f32,
    pub anim: f32,
    pub scale: f32,
    pub scale_exp: f32,
    pub pitch: f32,
    pub pitch_exp: f32,
}

#[derive(Deserialize, Clone)]
pub struct AirPattern {
    /// Circle, FigureEight, Infinite (FigureEightPatternModel.rotated) or Centered.
    pub mode: String,
    pub rx: f32,
    pub ry: f32,
    /// CirclePatternModel / CenterElipsePatternModel.reverse: round the other way.
    #[serde(default)]
    pub reverse: bool,
    /// PathSupplier.UseTowerPosition: the path is round the tower (FigureEightPatternModel's
    /// useTowerPosition; CenterElipsePattern: no, round its selected point).
    #[serde(default = "yes")]
    pub use_tower: bool,
}

#[derive(Deserialize, Clone)]
pub struct SubTowerSpec {
    /// TowerModel.radius and footprint, for Map.OverlapsOtherEntity.
    #[serde(default)]
    pub place_radius: f32,
    #[serde(default)]
    pub foot: Option<Footprint>,
    /// BeastHandlerPetModel's merge numbers (a Beast Handler pet).
    #[serde(default)]
    pub power: Option<PetPower>,
    /// BeastHandlerPetModel.canMove: the handler's TSM move button moves it.
    #[serde(default)]
    pub can_move: bool,
    /// BeastHandlerPetModel.rotateWhenMoved: a move turns it to face the new point.
    #[serde(default)]
    pub rotate_moved: bool,
    /// TowerCreateParagonTowerModel.isAirBasedTower: made at z 250 (the Dark Phoenix).
    #[serde(default)]
    pub air_based: bool,
    /// TowerModel.dontAddMutatorsFromParent: a parent's AddMutatorIncludeSubTowers skips it.
    #[serde(default)]
    pub no_parent_mutators: bool,
    /// TowerModel.isSubTower (+0x100): CashbackZone / GenericTowerBehaviorBuff
    /// TestPlacementTowerBuff give it no buff icon.
    #[serde(default)]
    pub is_sub: bool,
    /// TowerModel name ("ShootyTurretTower").
    #[serde(default)]
    pub name: String,
    /// Footprint radius.
    #[serde(default)]
    pub radius: f32,
    pub tier: std::sync::Arc<Tier>,
    /// The sub-tower's TowerModel.areaTypes (pets: Piranha = water [1], others land [2]).
    #[serde(default)]
    pub areas: Vec<i32>,
    pub lifespan: f32,
    /// TowerExpireModel.rounds: expires after this many round ends (0: no limit).
    #[serde(default)]
    pub rounds: u32,
    /// TowerExpireModel.expireOnRoundComplete.
    #[serde(default)]
    pub round_end: bool,
    /// Prefabs under models/subtowers/: the base (legs) and the turret that turns to aim.
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    /// CreditPopsToParentTowerModel.
    #[serde(default)]
    pub credit_parent: bool,
    /// TowerModel.baseId (SubTowerFilterModel counts sub-towers by it).
    #[serde(default)]
    pub id: String,
    /// TowerExpireOnParentDestroyedModel: goes when its parent is sold.
    #[serde(default)]
    pub with_parent: bool,
    /// CreateProjectileOnTowerDestroyModel: fired at the tower when it expires.
    #[serde(default)]
    pub on_destroy: Vec<OnDestroy>,
    /// !TowerModel.ignoreTowerForSelection: it can be selected (targeted, sold) on its own.
    #[serde(default)]
    pub selectable: bool,
    /// PathMovementFromScreenCenterModel: circles the map centre.
    #[serde(default)]
    pub circle: Option<Circle>,
    /// OrbitingTowerModel: circles its parent at `radius`, `rate` degrees a frame.
    #[serde(default)]
    pub orbit_tower: Option<OrbitTower>,
    /// CreateEffectOnPlaceModel / CreateEffectOnExpireModel: (effect prefab, scale).
    #[serde(default)]
    pub place_fx: Option<(Option<String>, f32)>,
    #[serde(default)]
    pub expire_fx: Option<(Option<String>, f32)>,
    /// CreateEffectOnAirUnitDestroyModel: the effect where its air unit goes (display only).
    #[serde(default)]
    pub air_destroy_fx: Option<String>,
}

/// OverclockModel: the picked tower's weapon intervals x `rate` (a Village's range x
/// `village_range`) for `frames` x tier_mult[its highest tier]. `permanent`: Ultraboost's stack.
#[derive(Deserialize, Clone)]
pub struct Overclock {
    #[serde(default)]
    pub frames: f32,
    #[serde(default = "one")]
    pub rate: f32,
    #[serde(default = "one")]
    pub village_range: f32,
    /// TakeAimModel.rangeModifier (any tower).
    #[serde(default = "one")]
    pub range: f32,
    #[serde(default)]
    pub tier_mult: Vec<f32>,
    #[serde(default)]
    pub permanent: Option<OverclockStack>,
    /// initialEffect (spawned on the picked tower) and its scale.
    #[serde(default)]
    pub fx: Option<String>,
    #[serde(default = "one")]
    pub fx_scale: f32,
    /// buffDisplayPath: shown on the tower while it's boosted.
    #[serde(default)]
    pub buff_fx: Option<String>,
    /// selectionObjectPath: the marker on each tower it may pick.
    #[serde(default)]
    pub marker_fx: Option<String>,
    /// TakeAim: Tower.AddMutatorIncludeSubTowers, the picked tower's child towers get it too.
    #[serde(default)]
    pub include_subs: bool,
    /// Its mutator's BuffIndicatorModel: iconName and buffName (not stackable).
    #[serde(default)]
    pub buff_icon: Option<String>,
    #[serde(default)]
    pub buff_name: String,
}

/// DarkshiftModel: the new spot is within `radius` of the tower (placementZoneAssetRadius) when
/// `restrict` (restrictToTowerRadius).
#[derive(Deserialize, Clone)]
pub struct Darkshift {
    pub radius: f32,
    pub restrict: bool,
    /// Darkshift: frames from the activation to the move (0 for Redeploy: at once).
    #[serde(default)]
    pub delay: u32,
    /// The input's helper message (a localization key): the model's helperMessageLocsKey.
    #[serde(default)]
    pub helper: String,
    /// disappearEffectModel / reappearEffectModel (prefab, scale), display only.
    #[serde(default)]
    pub out_fx: Option<(String, f32)>,
    #[serde(default)]
    pub in_fx: Option<(String, f32)>,
}

#[derive(Deserialize, Clone)]
pub struct RangeUp {
    pub frames: f32,
    pub mult: f32,
    pub add: f32,
}

#[derive(Deserialize, Clone)]
pub struct OverclockStack {
    pub rate: f32,
    pub village_range: f32,
    pub max_stacks: u32,
    /// Its mutator's BuffIndicatorModel: iconName, buffName, maxStackSize (0 unless stackable).
    #[serde(default)]
    pub buff_icon: Option<String>,
    #[serde(default)]
    pub buff_name: String,
    #[serde(default)]
    pub buff_max: i32,
}

#[derive(Deserialize, Clone)]
pub struct Circle {
    pub speed: f32,
    pub radius: f32,
}

/// SwitchDisplayModel: the tower shows `display` (models/fans/) for `lifespan` seconds.
#[derive(Deserialize, Clone)]
pub struct SwitchDisplay {
    pub display: Option<String>,
    pub lifespan: f32,
}

#[derive(Deserialize, Clone)]
pub struct OnInterval {
    pub frames: f32,
    /// isBuffedByRate: OnEmit scales intervalFrames by the tower's GetTotalRateModifier.
    #[serde(default)]
    pub buffed: bool,
    pub range: f32,
    pub only_if_target: bool,
    /// onlyEmitOnce: a single shot.
    #[serde(default)]
    pub once: bool,
    pub projectile: Proj,
    pub emission: Emission,
}

/// DruidVengeanceEffectModel: the glow (small / medium / epic) and bolt (default / weak /
/// medium / epic) shown for how much damage Wrath adds, from `stacks` up.
#[derive(Deserialize, Clone)]
pub struct Vengeance {
    pub fx: Vec<Option<String>>,
    pub stacks: Vec<f32>,
    pub displays: Vec<Option<String>>,
}

impl Vengeance {
    /// 0 below the small glow's stacks, else 1 / 2 / 3 for small / medium / epic.
    pub fn level(&self, boost: f32) -> usize {
        self.stacks.iter().filter(|&&s| boost >= s).count()
    }
}

/// MonkeyTempleModel: the TempleTowerMutatorGroups upgrading to this tier can earn.
#[derive(Deserialize, Clone)]
pub struct Temple {
    pub groups: Vec<TempleGroup>,
    /// MonkeyTempleModel.towerGroupCount: the tower sets kept, highest worth first.
    #[serde(default)]
    pub group_count: u32,
    /// MonkeyTempleModel.weaponDelay: seconds it holds fire while transforming.
    #[serde(default)]
    pub delay: f32,
    /// effectModel (on the temple) and towerEffectModel (on each tower consumed), display only.
    #[serde(default)]
    pub fx: Option<String>,
    #[serde(default)]
    pub tower_fx: Option<String>,
}

/// TempleTowerMutatorGroupModel: applies when the towers of `set` sacrificed are worth at
/// least `cost`. `two`: a True Sun God (Tier Two) group.
#[derive(Deserialize, Clone)]
pub struct TempleGroup {
    pub set: u32,
    pub cost: f32,
    #[serde(default)]
    pub two: bool,
    /// AddAttackTowerMutatorModel: (attack family, attack).
    #[serde(default)]
    pub attacks: Vec<(String, Attack)>,
    /// Every mutator, in model order (see Game::temple_tier).
    #[serde(default)]
    pub muts: Vec<TempleMut>,
}

/// One TowerMutatorModel of a temple group.
#[derive(Deserialize, Clone)]
pub struct TempleMut {
    /// BehaviorMutator.priority (Reload / Damage / Speed / Pierce / Size / Wind 20,
    /// AddBehavior 10, AddAttack 5, Range 1).
    pub p: u8,
    /// AddAttack: index into the group's `attacks`; else -1.
    pub a: i32,
    /// The model type without "TowerMutatorModel" (AddAttack, ReloadTime, ProjectileSpeed, ...).
    pub k: String,
    /// damage / pierce / speedModifier / sizeModifier / rangeIncrease / multiplier / windChance.
    #[serde(default)]
    pub v: f32,
    /// conditionalId CheckTempleUnderLevelModel (tower set, cost): applied on a sacrifice only
    /// while the temple holds that set with worth < cost (VERIFIED-DECOMP @0x180c1ce10).
    #[serde(default)]
    pub u: Option<(u32, f32)>,
    /// AddBehaviorToTowerMutatorModel support zones / PerRoundCashBonusTowerModel.cashPerRound.
    #[serde(default)]
    pub supports: Vec<Support>,
    #[serde(default)]
    pub round_cash: f32,
}

/// DamageModifierWrath.GetDamageAdditive (VERIFIED-DISASM @0xcd45f0): + active bloons' total
/// RBE / threshold x damage, clamped to [0, max].
#[derive(Deserialize, Clone)]
pub struct Wrath {
    pub threshold: f32,
    pub damage: f32,
    pub max: f32,
}

#[derive(Deserialize, Clone)]
pub struct TypeMod {
    pub id: String,
    pub mult: f32,
    pub add: f32,
    pub children: bool,
}

#[derive(Deserialize, Clone)]
pub struct Arrive {
    pub time: f32,
    /// expireOnArrival; otherwise it stops there for the rest of its lifespan (caltrops).
    pub expire: bool,
    /// filterCollisionWhileMoving: hits nothing until it arrives.
    pub filter_moving: bool,
    /// altSpeed: units per second, used when `time` is 0 (bananas).
    #[serde(default)]
    pub alt_speed: f32,
    /// ArriveAtTargetModel.curveSamples (Curve.Evaluate).
    #[serde(default)]
    pub samples: Vec<f32>,
    /// keepUpdatingTargetPos: targetPos = Target.Position each Process while the Target is valid.
    #[serde(default)]
    pub keep_update: bool,
    /// rotateToTarget, maxTurnAngle: the direction turns to the Target when within the angle.
    #[serde(default)]
    pub rotate: bool,
    #[serde(default)]
    pub max_turn: f32,
    /// positionAboveMoabTypes: z 181 at a boss, 121 at a MOAB, while its Target is valid.
    #[serde(default)]
    pub above_moab: bool,
}

/// CollectCashZoneModel: attractRange, collectRange, lerpMultiplier, useTowerRange,
/// onlyAttractsCamo, minEmitWaitTimeFrames, towerFilter.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct CollectZone {
    pub attract: f32,
    pub collect: f32,
    pub lerp: f32,
    #[serde(default)]
    pub use_range: bool,
    #[serde(default)]
    pub camo: bool,
    #[serde(default)]
    pub wait: i64,
    #[serde(default)]
    pub filter: String,
}

/// CashModel (+ PickupModel.collectRadius when it has to be collected by hand).
#[derive(Deserialize, Clone)]
pub struct Cash {
    pub min: f32,
    pub max: f32,
    #[serde(default)]
    pub bonus: f32,
    pub pickup: Option<f32>,
    /// PickupModel.delayFrames and isCamoPickup.
    #[serde(default)]
    pub pickup_delay: f32,
    #[serde(default)]
    pub pickup_camo: bool,
    #[serde(default)]
    pub bonus_shared: bool,
}

/// BankModel (Monkey Bank): income is stored up to `capacity` and grows by `interest` each round.
#[derive(Deserialize, Clone)]
pub struct Bank {
    pub capacity: f32,
    pub interest: f32,
    pub auto: bool,
}

#[derive(Deserialize, Clone)]
pub struct Seek {
    pub distance: f32,
    /// Degrees per second.
    pub turn_rate: f32,
    /// Only bloons within this many degrees of the heading (360 = any).
    pub max_angle: f32,
    pub new_targets: bool,
    /// useLifetimeAsDistance: its range for a new target is the way it has left to fly.
    #[serde(default)]
    pub lifetime: bool,
    /// constantlyAquireNewTarget: a new pick on every look, not only once it has none.
    #[serde(default)]
    pub constantly: bool,
    /// TrackTarget before TravelStrait in the behaviours: it turns before the first move too
    /// (recording brickell_l10_r40: a SeaMineSeeking's first step already at its MOAB).
    #[serde(default)]
    pub before_travel: bool,    /// TrackTargetWithinTimeModel.timeInFrames: the clamped turn is divided by this / elapsed.
    #[serde(default)]
    pub within: Option<f32>,
}

/// AdoraTrackTargetModel: rotation (degrees a frame), speeds / acceleration (units a second),
/// the angles that switch accelerating, lifespan (seconds).
#[derive(Deserialize, Clone)]
pub struct AdoraSpec {
    pub rotation: f32,
    pub min: f32,
    pub max: f32,
    pub accel: f32,
    pub accel_in: f32,
    pub decel_gt: f32,
    pub lifespan: f32,
    /// TrackTargetAfterDelayModel (Sniper Paragon shrapnel): the same homing after `delay`
    /// seconds, with `target_type` ("Random": First / Last / Close / Strong by its own
    /// System.Random).
    #[serde(default)]
    pub after_delay: Option<f32>,
    #[serde(default)]
    pub target_type: String,
}

#[derive(Deserialize, Clone)]
pub struct Wind {
    /// Listed before the DamageModel: it rolls before the damage.
    #[serde(default)]
    pub first: bool,
    pub min: f32,
    pub max: f32,
    pub chance: f32,
    pub moabs: bool,
    /// Distance multiplier for bloons with one of `tags` (0 = unused).
    pub tag_scale: f32,
    pub tags: Vec<String>,
    /// WindModel.speedMultiplier: MoveWithWind carries the bloon at speedFrames x this.
    #[serde(default = "one")]
    pub speed_mult: f32,
    /// WindModel.tagFilter / tagInclusive (Wind.Collide's filter).
    #[serde(default)]
    pub tag_filter: Vec<String>,
    #[serde(default)]
    pub tag_inclusive: bool,
}

#[derive(Deserialize, Clone)]
pub struct StateMod {
    pub states: Vec<String>,
    pub mult: f32,
    pub add: f32,
    /// bloonState alone (empty: none).
    #[serde(default)]
    pub state: String,
}

/// BeastHandlerPetModel (tier, basePower, maxPower, enableContribution, cooldownScaleRange,
/// pierceRange, damageRange, destroyTimeScaleRangePiranha, thrashKnockbackLifetimeRange,
/// stunBonusDivideMicroraptor, damageRangeGrappleGyrfalcon, speedRangeGyrfalcon).
#[derive(Deserialize, Clone, Debug, Default)]
pub struct PetPower {
    pub tier: i32,
    pub base: i32,
    pub max: i32,
    pub enable: bool,
    pub cd: f32,
    pub pierce: i32,
    pub damage: i32,
    pub destroy: f32,
    pub knockback: f32,
    pub stun_div: f32,
    pub grapple: i32,
    pub speed: f32,
    /// displayStepModels: (display prefab under models/extra, percent, projectileDisplay prefab),
    /// display only.
    #[serde(default)]
    pub steps: Vec<(Option<String>, f32, Option<String>)>,
}

/// AddBehaviorModifierForTagModel (see `Proj::ab_mods`).
#[derive(Deserialize, Clone)]
pub struct AbMod {
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub mult: f32,
}

/// A bloon effect: "slow" (mult = speed multiplier; glue level), "freeze", "dot" or "brittle"
/// (damage = extra damage per hit).
#[derive(Deserialize, Clone)]
pub struct EffectSpec {
    /// AddBehaviorToSingleBloonModel.firstHitOnly (Collide @0x180c81f70): applied on the
    /// behaviour's first collision only (hasCollided, kept by a pooled behaviour object).
    #[serde(default)]
    pub single_first: bool,
    /// AddBehaviorToSingleBloonModel.chance: above 0, a synced NextDouble (chance < it: skip).
    #[serde(default)]
    pub single_chance: f32,
    /// Chilled.hitBloon: only on the projectile's first hit that will damage a bloon.
    #[serde(default)]
    pub first_hit_only: bool,
    pub kind: String,
    pub id: String,
    /// The mutator's overlayType ("GlueBasic", "Ice", "Bleed"...; empty: none): the bloon shows
    /// "<its overlay class><this>" while it has it (display only).
    #[serde(default)]
    pub overlay: String,
    /// "pop_fx": CreateEffectOnPopModel's effect (name, scale), where the bloon pops.
    #[serde(default)]
    pub fx: Option<(String, f32)>,
    #[serde(default = "one")]
    pub mult: f32,
    /// A slow mutator's IncreaseDamageFromTypeModel amount (Silas's Rime: 1 at level 3, more
    /// later), for hits of GameData.effect_type_bonus's masks.
    #[serde(default)]
    pub bonus: f32,
    pub lifespan: f32,
    /// Layers it lasts through: 1 = only the bloon hit, 2 = also its children, ...
    pub layers: u32,
    /// BehaviorMutator.dontRemoveOnBloonDegrade: kept when a MOAB-class bloon pops.
    #[serde(default)]
    pub keep_on_degrade: bool,
    /// DamageOverTimeModel.damageModifierModels (DamageModifierForTag).
    #[serde(default)]
    pub tag_mods: Vec<TagMod>,
    /// DamageOverTimeModel.triggerImmediate: the first tick comes at once.
    #[serde(default)]
    pub trigger_immediate: bool,
    /// DamageOverTimeModel.initialDelay (seconds): the first tick waits this on top.
    #[serde(default)]
    pub initial_delay: f32,
    /// FreezeModel.applyAfterDamage, or a collisionPass after the damage's: it goes on after
    /// the damage, on what the bloon is then.
    #[serde(default)]
    pub after_damage: bool,
    /// An after-damage AddBehaviorToBloonModel's cascadeMutators: the children the hit made get
    /// it too (None: not from one; they do).
    #[serde(default)]
    pub kids: Option<bool>,
    /// AddBehaviorToBloonModel.dontCopy: a degrade's other children don't get it.
    #[serde(default)]
    pub dont_copy: bool,
    /// FreezeModel.bossImmunity: Bads and Minibosses aren't frozen.
    #[serde(default)]
    pub boss_immunity: bool,
    /// EmitOnPopModel.ignoreSameFrameDegrade: no emission for a pop on the frame it went on.
    #[serde(default)]
    pub ignore_same_frame: bool,
    /// UnstableConcoctionSplashModel's damages (kind "unstable").
    #[serde(default)]
    pub unstable: Option<UnstableDmg>,
    #[serde(default)]
    pub glue: u32,
    #[serde(default = "yes")]
    pub refresh: bool,
    /// SlowModel.removeMutatorIfNotMatching: a same-id slow from another model is replaced.
    #[serde(default)]
    pub replace: bool,
    /// SlowModel.isUnique.
    #[serde(default)]
    pub unique: bool,
    /// SlowModel.cascadeMutators.
    #[serde(default)]
    pub cascade: bool,
    /// SlowMinusAbilityDurationModel: the lifespan less the time since the ability went off.
    #[serde(default)]
    pub minus_age: bool,
    /// SlowForBloonModel: only bloons with one of these tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// SlowForBloonModel.bloonId / bloonIds / excluding: only bloons whose id contains bloonId
    /// (one id) or is contained in one of bloonIds (several); excluding: all but those.
    #[serde(default)]
    pub bloon_id: String,
    #[serde(default)]
    pub bloon_ids: Vec<String>,
    #[serde(default)]
    pub excluding: bool,
    /// From an AddBehaviorToBloonModel (its lifespan takes `Proj::ab_mods`).
    #[serde(default)]
    pub abtb: bool,
    /// From a SlowForBloonModel (its own Collide: see `for_bloon` in the export).
    #[serde(default)]
    pub for_bloon: bool,
    #[serde(default)]
    pub moabs: bool,
    #[serde(default)]
    pub damage: f32,
    #[serde(default)]
    pub interval: f32,
    /// "emit": ProjectileOverTimeModel.emitOnDestroy (the Ninja paragon's sticky splash).
    #[serde(default)]
    pub emit_on_destroy: bool,
    #[serde(default)]
    pub immune: u32,
    /// SlowOnPopModel: only on a hit Bloon.WillPopBloon says pops it.
    #[serde(default)]
    pub only_if_pop: bool,
    /// AddBehaviorToBloonModel.chance (0: always).
    #[serde(default)]
    pub chance: f32,
    /// applyOnlyIfDamaged: it only goes on a bloon that its parentDamageModel (`parent`: damage,
    /// immunities) and the projectile's own DamageModel would damage (Bloon.WillDamageBloon).
    #[serde(default)]
    pub only_if_damaged: bool,
    #[serde(default)]
    pub parent: Option<ParentDamage>,
    /// DamageOverTimeModel.distributeToChildren / overrideDistributionBlocker.
    #[serde(default)]
    pub distribute: bool,
    #[serde(default)]
    pub distribute_override: bool,
    #[serde(default)]
    pub moabs_only: bool,
    /// SlowMaimMoabModel: lifespan per MOAB class ("MOAB", "BFB", "ZOMG", "DDT", "BAD"),
    /// replacing `lifespan`.
    #[serde(default)]
    pub by_class: HashMap<String, f32>,
    /// A slow's multiplier by MOAB class (SilasApplyRimeModel's moab / ddt / bfb / zomg
    /// SpeedModifier); other bloons take `mult`.
    #[serde(default)]
    pub mult_by_class: HashMap<String, f32>,
    /// "emit": ProjectileOverTimeModel, fired at the bloon every `interval` (Sticky Bomb splash).
    /// "pop_emit": EmitOnPopModel, fired (with `emission`) where the bloon pops.
    #[serde(default)]
    pub projectile: Option<std::sync::Arc<Proj>>,
    #[serde(default)]
    pub emission: Option<Emission>,
    /// "trance": orbit radius, the speed factor for MOAB-class bloons (`mult` for the rest),
    /// and seconds after it ends before the bloon can be entranced again.
    #[serde(default)]
    pub radius: f32,
    #[serde(default = "one")]
    pub moab_mult: f32,
    #[serde(default)]
    pub cooldown: f32,
}

/// Per-tag change to an effect id: skip it (prevent), replace the slow multiplier (reset), scale
/// the freeze time, or override the lifespan.
#[derive(Deserialize, Clone)]
pub struct EffectMod {
    pub tags: Vec<String>,
    pub id: String,
    pub mult: f32,
    pub reset: bool,
    pub prevent: bool,
    pub lifespan: f32,
    pub time_mult: f32,
    /// SlowModifierForTagModel.makeNotTag: it's for bloons without the tag.
    #[serde(default)]
    pub not_tag: bool,
    /// A FreezeModifierForTagsModel (else a SlowModifierForTagModel).
    #[serde(default)]
    pub freeze: bool,
}

/// SlowBloonsZoneModel (Arctic Wind): bloons in the radius move at `scale` speed.
#[derive(Deserialize, Clone)]
pub struct Zone {
    pub radius: f32,
    pub bind: bool,
    pub offset: f32,
    pub scale: f32,
    /// mutationId: the zone's slow mutator (isUnique: one per id on a bloon).
    #[serde(default)]
    pub id: String,
    /// bloonTag: with one, the zone takes only bloons with all of `tags` (bloonTags), or with
    /// `inclusive` false only bloons without them (SlowBloonsZone.FilterBloon).
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub inclusive: bool,
    #[serde(default)]
    pub exclude_tags: Vec<String>,
    #[serde(default = "yes")]
    pub camo: bool,
}

#[derive(Deserialize, Clone)]
pub struct FollowPath {
    pub points: Vec<[f32; 2]>,
    /// FollowPathModel.easePath (see FollowPath.ProcessFrame).
    #[serde(default)]
    pub ease: Option<Vec<[f32; 2]>>,
    pub speed: f32,
    #[serde(default = "yes")]
    pub destroy_at_end: bool,
    /// A TravelStraitModel's speed beside it (0: none): see the path move.
    #[serde(default)]
    pub straight: f32,
}

#[derive(Deserialize, Clone)]
pub struct PushBack {
    pub amount: f32,
    pub tag: String,
    /// PushBackModel.onlyIfDamaged: only bloons the shot's damage can hurt (Bloon.WillDamageBloon).
    #[serde(default)]
    pub only_if_damaged: bool,
    /// Per-MOAB-class multipliers, keyed "MOAB", "BFB", "DDT", "ZOMG".
    pub mult: HashMap<String, f32>,
}

fn one_usize() -> usize {
    1
}

/// EmitOnDamageWithStateModel (EmitOnDamageWithState.OnBloonDegrade @0x180cbafd0 / OnBloonDestroy
/// @0x180cbb010): when this projectile degrades a bloon (mustBePopped: only one it destroys)
/// that has one of `states` (none: any) and one of `ids` (none: any), `projectile` through
/// EmissionAtClosestPathSegment at the bloon.
#[derive(Deserialize, Clone)]
pub struct StateEmit {
    #[serde(default)]
    pub must_pop: bool,
    #[serde(default)]
    pub states: Vec<String>,
    #[serde(default)]
    pub ids: Vec<String>,
    pub projectile: Proj,
}

/// LightningRodEmitModel (LightningRodEmit.Collide @0x180cc9580, collisionPass -1): a rod at the
/// hit bloon once per `cooldown` s, unless one of the tower's rods is within `min_dist`.
#[derive(Deserialize, Clone)]
pub struct RodEmit {
    pub cooldown: f32,
    pub min_dist: f32,
    #[serde(default)]
    pub rod: Option<Proj>,
}

/// ActivateLightningRodZapModel: rate / delayPerJump / duration in seconds; `proj` the
/// lightningRod (ThunderLightning) shot along its LineProjectileEmission (useTargetAsEndPoint).
#[derive(Deserialize, Clone)]
pub struct RodZap {
    pub rate: f32,
    pub damage: f32,
    pub pierce_per_jump: f32,
    pub delay: f32,
    pub duration: f32,
    #[serde(default)]
    pub proj: Option<Proj>,
}

/// ChilledModel's ice shrapnel (Chilled.EmitIceShrapnel @0x180c9a170): shrapnelRange,
/// shrapnelCount, its projectile and emission.
#[derive(Deserialize, Clone)]
pub struct ChillShrapnel {
    pub range: f32,
    pub count: usize,
    pub projectile: Proj,
    pub emission: Emission,
}

/// ChilledModel's SlowNearby (Chilled.SlowNearby @0x180c9b870): slowRadius, radiusMultiplier,
/// moabStageTwoSlowMod, affectMoab, slowDuration (s).
#[derive(Deserialize, Clone)]
pub struct ChillNearby {
    pub radius: f32,
    pub mult: f32,
    pub moab_mult: f32,
    pub moab: bool,
    pub lifespan: f32,
}

/// LightningRodModel (LightningRod.GetRadius @0x180cca1f0 / TryEmitLightning @0x180cca610).
#[derive(Deserialize, Clone)]
pub struct Rod {
    pub radius: f32,
    #[serde(default)]
    pub scale: bool,
    #[serde(default)]
    pub base: f32,
    #[serde(default)]
    pub zap_cd: f32,
    #[serde(default)]
    pub zap: Option<Proj>,
}

/// WindlashModel (Skywarden): stage mutators for `duration` s; from stage 2 the arrows.
#[derive(Deserialize, Clone)]
pub struct Windlash {
    pub duration: f32,
    pub min: f32,
    pub max: f32,
    pub interval: f32,
    pub speed: f32,
    #[serde(default)]
    pub moab: bool,
    #[serde(default)]
    pub arrow: Option<Proj>,
    #[serde(default)]
    pub arrow_emission: Option<Emission>,
    /// vacuumProjectile (WindVacuum: wind along the track) and its EmissionAtClosestPathSegment
    /// [count, spacing].
    #[serde(default)]
    pub vacuum: Option<Proj>,
    #[serde(default)]
    pub vacuum_at: Option<[f32; 2]>,
}

/// KnockbackModel: the KnockbackMutator for `lifespan` seconds; the bloon's speed becomes
/// speed - |m x speed|, m the MOAB / heavy (Lead, Ceramic, Fortified) / light multiplier.
#[derive(Deserialize, Clone, Debug)]
pub struct Knockback {
    #[serde(rename = "lightMultiplier", default)]
    pub light: f32,
    #[serde(rename = "heavyMultiplier", default)]
    pub heavy: f32,
    #[serde(rename = "moabMultiplier", default)]
    pub moab: f32,
    #[serde(default)]
    pub lifespan: f32,
}

/// CashPerTowerInRangeModel: baseCash, plus extraCashPerTier[tier] for each tower in range with
/// one of `towers`' base ids, up to maxCashGeneration.
#[derive(Deserialize, Clone, Debug)]
pub struct Bounty {
    #[serde(default)]
    pub towers: Vec<String>,
    #[serde(default)]
    pub base: f32,
    #[serde(default)]
    pub per_tier: Vec<f32>,
    #[serde(default)]
    pub max: f32,
    #[serde(default)]
    pub range_add: f32,
}

#[derive(Deserialize, Clone, Copy, PartialEq)]
pub struct RandomPos {
    pub min: f32,
    pub max: f32,
    pub point: f32,
    pub radius: f32,
    pub ideal: f32,
    /// areaType Water (Piranha's spawner).
    #[serde(default)]
    pub water: bool,
}

/// UnstableConcoctionSplashModel: the explosion's damage by the exploding bloon's baseId.
#[derive(Deserialize, Clone, Debug)]
pub struct UnstableDmg {
    #[serde(default)]
    pub bloon_map: Vec<(String, f32)>,
    #[serde(default)]
    pub bloon: f32,
    #[serde(default)]
    pub moab_map: Vec<(String, f32)>,
    #[serde(default)]
    pub boss_moab: f32,
    #[serde(default)]
    pub moab: f32,
}

/// MonkeyopolisModel.
#[derive(Deserialize, Clone)]
pub struct Mopolis {
    pub filter: String,
    /// valueRequiredForCrate.
    pub value: i32,
    /// cashFromCrate.
    pub crate_cash: f32,
    /// baseIncome.
    pub base: i32,
    /// cratesPerRound.
    pub crates: i32,
}

/// MonkeyopolisUpgradeCostModel.
#[derive(Deserialize, Clone)]
pub struct MopolisCost {
    pub path: usize,
    pub per_farm: f32,
    pub filter: String,
}

/// MagusPerfectusGraveyardStateManagerModel.
#[derive(Deserialize, Clone)]
pub struct Magus {
    pub max: i32,
    /// Attack model names of Consuming (0), Generating (1), DrainingOverTime (2).
    pub states: Vec<Vec<String>>,
    /// rateBonusThreshold / rateMultplierAmount: the rate mutator while the graveyard is that full.
    #[serde(default)]
    pub rate_threshold: f32,
    #[serde(default = "one_f32")]
    pub rate_mult: f32,
}

/// SpawnZombieOnBloonDestroyedModel: graveyardManaCost, zombieProjectile, singleEmissionModel.
#[derive(Deserialize, Clone)]
pub struct Zombie {
    pub cost: i32,
    pub projectile: Proj,
    pub emission: Emission,
}

fn one_f32() -> f32 {
    1.0
}

/// GalvanizedModel: chainLightningDamageTypes (as immunity values), projectile, emission.
#[derive(Deserialize, Clone)]
pub struct GalvChain {
    pub types: Vec<u32>,
    /// GalvanizedBehaviorModel's magnet: radius, turnStrength, maxAngle, magnetBloonProperties (bits).
    #[serde(default)]
    pub magnet: [f32; 3],
    #[serde(default)]
    pub magnet_props: u32,
    pub projectile: Proj,
    pub emission: Emission,
}

/// A TowerModel DisplayModel besides the tower's body (Mermonkey's base, Arctic Wind's snow).
#[derive(Deserialize, Clone, Debug, Default)]
pub struct TierExtra {
    pub name: String,
    #[serde(default = "one_f32")]
    pub scale: f32,
}

/// CycleAnimationModel (minAnimationState, maxAnimationState, loopMode "Loop" / "Clamp", randomize).
#[derive(Deserialize, Clone, Debug, Default)]
pub struct CycleAnim {
    pub min: i32,
    pub max: i32,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub random: bool,
}

/// CreateGreatWhiteEffectModel (frames as GreatWhiteEffectModel.ctor: (int)(seconds x 60)).
#[derive(Deserialize, Clone)]
pub struct GreatWhite {
    /// CreateGreatWhiteEffectModel.display (model under models/extra), display only.
    #[serde(default)]
    pub display: Option<String>,
    /// layerNumber of maxGrabBloonName's model (None: any).
    #[serde(default)]
    pub max_layer: Option<u32>,
    pub max_pull_rbe: f32,
    pub pop_scale: f32,
    pub post_drag_frames: i32,
    pub max_moab_time: f32,
    /// maxMoabGrabTimeMegalodon: the cap in SetGrabbedBloon's percent branch.
    #[serde(default)]
    pub max_moab_time_mega: f32,
    pub moab_dps: i32,
    pub moab_pct: f32,
    pub exit_frames: i32,
    pub retreat_frames: i32,
    pub thrash_frames: i32,
    #[serde(default)]
    pub thrash: Option<Proj>,
    pub emission: Emission,
    /// bloonFollowProjectileModel: latched onto a bloon it can't pull.
    #[serde(default)]
    pub follow: Option<Proj>,
}

/// LatchToBloonModel (rotateWithBloon, postBloonDestroyTimeFrames, postBloonDestroyTime,
/// bloonSpeedMultiplier).
#[derive(Deserialize, Clone)]
pub struct Latch {
    pub rotate: bool,
    pub post_frames: i64,
    pub post_time: f32,
    pub speed: f32,
}

/// CreateEffectProjectileAfterTime: `fx` after `frames` (from creation; again every `frames` with
/// `repeat`), at the projectile or its target.
#[derive(Deserialize, Clone, Debug)]
pub struct AfterFx {
    pub fx: String,
    #[serde(default = "one")]
    pub scale: f32,
    pub frames: i32,
    #[serde(default)]
    pub repeat: bool,
    #[serde(default)]
    pub target: bool,
}

/// ScaleProjectileOverTime's effect: at the projectile + its direction x `offset`, scaled
/// min(`base` + `per_sec` x seconds, `max`).
#[derive(Deserialize, Clone, Debug)]
pub struct ScaleFx {
    pub fx: String,
    #[serde(default)]
    pub offset: f32,
    #[serde(default = "one")]
    pub base: f32,
    #[serde(default = "one")]
    pub max: f32,
    #[serde(default)]
    pub per_sec: f32,
}

/// IncomeSetModel: a round below a threshold pays its multiplier (the first such), else `final_mult`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct IncomeSet {
    pub thresholds: Vec<(i32, f32)>,
    pub final_mult: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PctMax {
    pub percent: f32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub offscreen: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TowerBuffOnExpire {
    pub kind: String,
    #[serde(default)]
    pub value: f32,
    #[serde(default)]
    pub frames: i64,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub tower_pos: bool,
    #[serde(default)]
    pub closest: bool,
}

fn big_usize() -> usize {
    usize::MAX
}
