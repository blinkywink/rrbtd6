//! `local_extracted/game/game.json`, produced by tools/assets/build_game_data.py
//! from the user's own install. Never committed.

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct GameData {
    /// CollisionGrid.orderedNeighborOffsets: (dx, dy) cell offsets in the order a collision
    /// query visits them (from a SimRecorder dump; empty: storage order).
    #[serde(default)]
    pub grid_offsets: Vec<i32>,
    pub start: Start,
    pub bloons: HashMap<String, BloonDef>,
    pub towers: HashMap<String, TowerDef>,
    #[serde(default)]
    pub tower_tiers: HashMap<String, TowerTiers>,
    pub rounds: Vec<Vec<Group>>,
    /// Other round sets by name (RoundSetModModel: "AlternateRoundSet").
    #[serde(default)]
    pub round_sets: HashMap<String, Vec<Vec<Group>>>,
    /// GameModel.freeplayGroups: rounds past the round set are built from these.
    #[serde(default)]
    pub freeplay: Vec<FreeplayGroup>,
    /// Geraldo's shop items (GeraldoItemModel).
    #[serde(default)]
    pub geraldo: Vec<GeraldoItem>,
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
    /// ParagonDegreeDataModel.
    #[serde(default)]
    pub paragon_degree: Option<ParagonDegree>,
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
    /// BonusCashPerRoundModModel (baseCash, roundMultiple); none in CHIMPS.
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
    /// CollisionGroupData (MOAB-class): (offset x, offset y along the heading, radius), game frame.
    #[serde(default)]
    pub circles: Vec<[f32; 3]>,
    /// BloonModel.rotateToFollowPath: turns towards the path's heading as it moves.
    #[serde(default)]
    pub rotate_to_follow_path: bool,
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
}

fn minus_one() -> f32 {
    -1.0
}

fn one() -> f32 {
    1.0
}

#[derive(Deserialize, Clone)]
pub struct TowerDef {
    /// TowerModel.areaTypes: where it can be placed (1 water, 2 land).
    #[serde(default)]
    pub areas: Vec<i32>,
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
    /// Every track (one per entrySplitter) with its active flag; empty in older exports.
    #[serde(default)]
    pub paths: Vec<MapPath>,
    pub areas: Vec<Area>,
    /// The terrain mesh's extent; some maps' exports have none (their ground is part of the
    /// scene model).
    #[serde(default)]
    pub terrain: Option<Terrain>,
}

#[derive(Deserialize, Clone)]
pub struct ParagonDef {
    /// ParagonTowerModel.displayDegreePaths count: display levels.
    #[serde(default = "one_usize")]
    pub levels: usize,
    pub tier: Tier,
    pub cost: f64,
    pub name: String,
    /// ParagonTowerModel.inactiveDurationFrames.
    #[serde(default)]
    pub inactive_frames: u64,
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
    /// PointInfo.disablePathTargeting per point (imported from the game's map dump).
    #[serde(default)]
    pub no_target: Vec<bool>,
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
    #[serde(rename = "type")]
    pub kind: i32,
    pub points: Vec<[f32; 2]>,
    /// Holes: AreaModel.IsPointInside excludes points inside any of them.
    #[serde(default)]
    pub holes: Vec<Vec<[f32; 2]>>,
    /// `AreaModel.height` (the Area transform's world y).
    #[serde(default)]
    pub height: f32,
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

/// Every Dart Monkey tier combination ("000".."520") from its decoded TowerModel.
#[derive(Deserialize, Clone)]
pub struct TowerTiers {
    /// The Paragon: its TowerModel's tier data and its upgrade.
    #[serde(default)]
    pub paragon: Option<ParagonDef>,
    pub tiers: HashMap<String, Tier>,
    pub upgrades: Vec<Upgrade>,
    /// Heroes: XP leveling (tiers are [level, 0, 0]).
    #[serde(default)]
    pub hero: Option<HeroDef>,
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
    /// BeastHandlerLeashModel pets (towerModel / towerModelSecond), as permanent sub-towers.
    #[serde(default)]
    pub pets: Vec<std::sync::Arc<SubTowerSpec>>,
    pub range: f32,
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
    /// AddMakeshiftAreaModel: deck areas towers can stand on.
    #[serde(default)]
    pub decks: Vec<Deck>,
    /// PreEmptiveStrikeLauncherModel.projectileModel: launched at each MOAB-class bloon.
    #[serde(default)]
    pub preempt: Option<Proj>,
    /// MerchantShipModel: a merchant ship (counted by Trade Empire).
    #[serde(default)]
    pub merchant: bool,
    #[serde(default)]
    pub trade_empire: Option<TradeEmpire>,
    #[serde(default)]
    pub cashback: Option<Cashback>,
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
    /// PerRoundCashBonusTowerModel.cashPerRound: paid at each round end.
    #[serde(default)]
    pub round_cash: f64,
    #[serde(default)]
    pub round_cash_shared: f64,
    /// SpiritOfTheForestModel (Druid): thorn damage along the track.
    #[serde(default)]
    pub spirit: Option<Spirit>,
    /// CollectCashZoneModel: collects landed cash within this radius (0: the tower's range).
    #[serde(default)]
    pub collect_zone: Option<f32>,
    /// ActivateAbilityAfterIntervalModel.interval: seconds between automatic uses.
    #[serde(default)]
    pub auto_ability: Option<f32>,
    /// Grow Blocker zone radius (0: the tower's range).
    #[serde(default)]
    pub grow_block: Option<f32>,
    /// FreezeNearbyWaterModel.radius (0: the tower's range): water there takes land towers.
    #[serde(default)]
    pub freeze_water: Option<f32>,
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
/// x mult), "discount" (cost x(1 - mult) up to `tier_cap`).
#[derive(Deserialize, Clone)]
pub struct Support {
    pub kind: String,
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
    #[serde(default)]
    pub bases: Vec<String>,
    #[serde(default = "five")]
    pub tier_cap: u32,
    /// Buff icon (UiBuffIcons sprite) shown over towers it reaches (showBuffIcon).
    #[serde(default)]
    pub icon: Option<String>,
    /// Seconds an ability's support zone lasts (0 for a tower's own supports).
    #[serde(default)]
    pub lifespan: f32,
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
}

#[derive(Deserialize, Clone)]
pub struct FreeTower {
    pub tower: String,
    pub mult: f64,
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
    /// PrinceOfDarknessEmissionModel.alternateProjectile: sent when the graveyard is empty.
    #[serde(default)]
    pub prince: Option<Box<Proj>>,
    #[serde(default, rename = "princePierce")]
    pub prince_pierce: f32,
}

#[derive(Deserialize, Clone)]
pub struct Attack {
    pub range: f32,
    pub camo: bool,
    /// AttackModel has a RotateToTargetModel (the tower turns to face its target).
    #[serde(default = "yes")]
    pub rotate: bool,
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
    /// FilterWithTagModel / FilterOutTagModel tags (MOAB Press only targets "Moabs", never "Bad").
    #[serde(default)]
    pub require_tags: Vec<String>,
    #[serde(default)]
    pub exclude_tags: Vec<String>,
    /// AttackModel.fireWithoutTarget: fires on its own timer, target or not (Glaive Lord's orbit hits).
    #[serde(default)]
    pub fire_without_target: bool,
    /// AttackModel.attackThroughWalls: targets aren't checked for a clear line (Map.IsTargetLineClear).
    #[serde(default)]
    pub through_walls: bool,
    #[serde(default)]
    pub glue_filter: u32,
    #[serde(default)]
    pub skip_frozen: bool,
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
    /// FilterMutatedTargetModel: skips bloons carrying an effect with one of these ids.
    #[serde(default)]
    pub skip_effects: Vec<String>,
    /// TargetTrackModel.maxOffset: aims at a random spot on the track in range (caltrops).
    #[serde(default)]
    pub track_target: Option<f32>,
    /// TargetTrackOrDefaultModel.useClosestTrack: the track point closest to the tower instead
    /// (Wall of Fire).
    #[serde(default)]
    pub track_closest: bool,
    /// RandomPositionBasicModel min/max radius: aims at a random spot around the tower (bananas).
    #[serde(default)]
    pub random_position: Option<[f32; 2]>,
    /// RandomPositionModel (areaType Land): the random spot must be placeable land, clear by this radius.
    #[serde(default)]
    pub random_land: Option<f32>,
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
    /// RotateToTargetModel.onlyRotateDuringThrow (and rotateOnlyOnThrow) off: turns to its
    /// target every frame it has one.
    #[serde(default)]
    pub rotate_always: bool,
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
}

fn yes() -> bool {
    true
}

#[derive(Deserialize, Clone)]
pub struct Weapon {
    /// WeaponModel name without its prefix ("MainWeapon", "LongWeapon"...).
    #[serde(default)]
    pub name: String,
    pub rate: f32,
    /// WeaponRateMinModel.min: the shortest interval buffs can bring it to.
    #[serde(default)]
    pub rate_min: f32,
    /// LeakDangerAttackSpeedModel.maxRateIncreasePercent.
    #[serde(default)]
    pub leak_danger: f32,
    /// BloonDistanceRateBonusModel: (rateBonusPerRange, rangeMax).
    #[serde(default)]
    pub dist_rate: Option<(f32, f32)>,
    /// WeaponModel.animationOffset: the attack animation starts this many seconds before the shot.
    #[serde(default)]
    pub animation_offset: f32,
    /// WeaponModel.fireBetweenRounds: the weapon's clock is Simulation.time (always running)
    /// instead of roundTime (stopped between rounds).
    #[serde(default)]
    pub between_rounds: bool,
    /// WeaponModel.startInCooldown / customStartCooldown (seconds).
    #[serde(default)]
    pub start_cooldown: bool,
    #[serde(default)]
    pub custom_start: f32,
    /// WeaponModel.ejectX/Y/Z: launch point relative to the tower (x right, y forward, z up).
    #[serde(default)]
    pub eject: [f32; 3],
    pub emission: Emission,
    pub projectile: Proj,
    pub crit: Option<Crit>,
    /// AlternateProjectileModel: every `interval`-th shot fires this instead (Flash Bomb).
    #[serde(default)]
    pub alternate: Option<Alternate>,
    /// RandomAngleOffsetModel: shots deviate by up to this many degrees either way.
    #[serde(default)]
    pub spread: f32,
    /// EmissionsPerRoundFilterModel.count: at most this many shots per round (bananas).
    #[serde(default)]
    pub per_round: Option<u32>,
    /// LimitProjectileModel.limit: no shot while the tower has this many unfilled traps out.
    #[serde(default)]
    pub limit: Option<u32>,
    /// SendToBankModel emission: the cash goes into the tower's bank instead of the map.
    #[serde(default)]
    pub to_bank: bool,
    /// BurstWeaponBehaviorModel: each shot is `count` shots, `frames` apart.
    #[serde(default)]
    pub burst: Option<BurstFire>,
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
    /// FireWhenAlternateWeaponIsReadyModel.weaponId: fires only when the weapon with this
    /// FireAlternateWeaponModel id fires, ignoring its own rate.
    #[serde(default)]
    pub with_main: Option<i32>,
    /// FireAlternateWeaponModel.weaponId: firing this also fires the weapons paired with it.
    #[serde(default)]
    pub alt_id: Option<i32>,
    /// FilterTargetAngleFilterModel [fieldOfView, baseTowerRotationOffset] (degrees): fires
    /// only with a bloon in range inside that arc around the tower's facing.
    #[serde(default)]
    pub fov: Option<[f32; 2]>,
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
}

/// CashbackZoneModel (Favored Trades): towers in range sell for more.
#[derive(Deserialize, Clone)]
pub struct Cashback {
    pub mult: f64,
    pub max: f64,
    pub stacks: u32,
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
}

#[derive(Deserialize, Clone)]
pub struct TrackTravel {
    pub speed: f32,
    pub lifespan: f32,
    #[serde(default)]
    pub reverse: bool,
}

#[derive(Deserialize, Clone)]
pub struct PlaceAt {
    pub projectile: Proj,
    pub delay: f32,
}

#[derive(Deserialize, Clone)]
pub struct Biohack {
    pub count: u32,
    pub damage: f32,
    pub lifespan: f32,
}

#[derive(Deserialize, Clone)]
pub struct Barrage {
    pub targets: u32,
    pub damage: f32,
    pub interval: f32,
    pub lifespan: f32,
    #[serde(default)]
    pub immune: u32,
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
    /// "acid" (extra damage by tag) or "berserk" (damage, pierce, rate and range).
    pub kind: String,
    pub id: String,
    /// Brews this one replaces (mutatorsToRemove).
    pub replaces: Vec<String>,
    /// Shots it lasts (cap) and seconds (lifespan; <= 0 = no time limit).
    pub shots: u32,
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

/// GiveCashOnPopModel given to bloons with `tag` (Lead to Gold).
#[derive(Deserialize, Clone)]
pub struct CashOnPop {
    pub amount: f32,
    pub tag: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ParentDamage {
    pub damage: f32,
    pub immune: u32,
}

#[derive(Deserialize, Clone)]
pub struct Exhaust {
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
}

#[derive(Deserialize, Clone)]
pub struct Contact {
    pub projectile: Proj,
    pub emission: Emission,
    /// passOnCollidedWith: the sub-projectiles can't hit the bloon that triggered them.
    #[serde(default)]
    pub pass_on_collided: bool,
    /// EmitOnDamageModel (Shrapnel Shot): only on hits that deal damage.
    #[serde(default)]
    pub on_damage: bool,
    /// EmitOnPopModel: only on hits that pop the bloon.
    #[serde(default)]
    pub on_pop: bool,
}

/// RetargetOnContactModel (Bouncing Bullet): after each hit, jump to the closest bloon within
/// `distance` after `delay` seconds.
#[derive(Deserialize, Clone)]
pub struct Retarget {
    pub distance: f32,
    pub max_bounces: u32,
    pub delay: f32,
    pub expire_if_none: bool,
}

#[derive(Deserialize, Clone)]
pub struct Proj {
    /// It has a DamageModel.
    #[serde(default)]
    pub damage_model: bool,
    /// DamageModel.distributeToChildren / overrideDistributeBlocker: whether damage past a
    /// layer goes on to its children (with the bloon's distributeDamageToChildren, or regardless).
    #[serde(default = "yes")]
    pub distribute: bool,
    #[serde(default)]
    pub distribute_override: bool,
    /// KnockbackModel: hit bloons go backwards for a while.
    #[serde(default)]
    pub knockback: Option<Knockback>,
    /// AgeModel.lifespan (seconds), when it has one.
    #[serde(default)]
    pub age_lifespan: Option<f32>,
    pub pierce: f32,
    pub radius: f32,
    /// ProjectileModel.vsBlockerRadius / ignoreBlockers: its radius against map blockers, and
    /// whether they stop it.
    #[serde(default)]
    pub vs_blocker_radius: f32,
    #[serde(default)]
    pub ignore_blockers: bool,
    pub damage: f32,
    pub immune: u32,
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
    /// ProjectileBlockerCollisionReboundModel: bounces off blocker areas' edges.
    #[serde(default)]
    pub blocker_rebound: Option<BlockerRebound>,
    /// CreateProjectileOnExhaustFractionModel: sub-projectiles spawned where it ends.
    #[serde(default)]
    pub on_exhaust: Vec<Exhaust>,
    /// CollideExtraPierceReductionModel: pierce each hit uses (1 + extraAmount).
    #[serde(default = "one")]
    pub pierce_cost: f32,
    /// CreateProjectileOnContactModel: sub-projectiles spawned at each bloon hit (bomb explosions, frags).
    #[serde(default)]
    pub on_contact: Vec<Contact>,
    /// DisplayModel asset GUID (keys models/projectiles/index.json).
    #[serde(default)]
    pub display: Option<String>,
    /// The tower has LinkProjectileRadiusToTowerRangeModel: radius = attack range (Ring of Fire).
    #[serde(default)]
    pub link_radius: bool,
    /// InstantModel: hits its target bloon (FilterAllExceptTargetModel) the frame it is fired.
    #[serde(default)]
    pub instant: bool,
    #[serde(default)]
    pub retarget: Option<Retarget>,
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
    /// FilterAllModel: hits nothing (a Mortar shell in flight; its explosion does the damage).
    #[serde(default)]
    pub no_hit: bool,
    /// RotateModel.angle: display spin in degrees per second.
    #[serde(default)]
    pub spin: f32,
    /// Projectile FilterOutTagModel tags (Ice Monkey: "Moabs").
    #[serde(default)]
    pub exclude_tags: Vec<String>,
    /// FilterGlueLevelModel.glueLevel: skips bloons already glued at this level or higher (0 = off).
    #[serde(default)]
    pub glue_filter: u32,
    /// FilterFrozenBloonsModel: skips frozen bloons.
    #[serde(default)]
    pub skip_frozen: bool,
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
    /// TrackTargetModel / TrackTargetWithinTimeModel: homes in on its target (Seeking Shuriken).
    #[serde(default)]
    pub seek: Option<Seek>,
    /// WindModel: blows bloons back along the track (Distraction).
    #[serde(default)]
    pub wind: Option<Wind>,
    /// FilterOveridingMutatedTargetModel: hits only bloons with one of these effects.
    #[serde(default)]
    pub require_effects: Vec<String>,
    /// DamageModifierForCashAmountModel: (multiplier, additive, cash threshold).
    #[serde(default)]
    pub cash_damage: Option<[f32; 3]>,
    /// AccelerateModel: (acceleration, maxSpeed) in units per second.
    #[serde(default)]
    pub accel: Option<[f32; 2]>,
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
    /// RefreshPierceModel.interval: pierce is restored every this many seconds (Druid vines).
    #[serde(default)]
    pub refresh_pierce: Option<f32>,
    /// RemoveMutatorsFromBloonModel keys: effects whose id starts with one are removed (Tornado).
    #[serde(default)]
    pub remove_effects: Vec<String>,
    /// CreateProjectileOnIntervalModel: every `frames` frames, a sub-projectile at a random bloon
    /// within `range` (Ball Lightning).
    #[serde(default)]
    pub on_interval: Option<Box<OnInterval>>,
    #[serde(default)]
    pub acid_pool: Option<AcidPool>,
    #[serde(default)]
    pub cash_on_pop: Option<CashOnPop>,
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
    #[serde(default)]
    pub eat: Option<Eat>,
    /// ScaleDamageWithTimeModel [baseDamage, scalePerSecond, maxDamage].
    #[serde(default)]
    pub scale_damage: Option<[f32; 3]>,
    /// FilterAllExceptTargetModel: hits only its target bloon.
    #[serde(default)]
    pub only_target: bool,
    /// TravelAlongPathModel: moves along the track.
    #[serde(default)]
    pub track_travel: Option<TrackTravel>,
}

/// EatBloonModel (Bloon Trap): bloons it touches vanish whole until `capacity` RBE is inside.
#[derive(Deserialize, Clone)]
pub struct Eat {
    pub capacity: f32,
    pub cash_mult: f32,
}

/// A temporary tower spawned by a projectile (Sentry): its own tier, gone after `lifespan` seconds.
/// AbilityModel: a button that, off cooldown, turns on extra attacks and / or a rate boost.
#[derive(Deserialize, Clone)]
pub struct Ability {
    /// CreateProjectileOnAbilityActivateModel: fired from the tower when it's used.
    #[serde(default)]
    pub on_activate: Vec<Contact>,
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
    /// SwordChargeModel: a projectile sent along the track.
    #[serde(default)]
    pub track_charge: Option<Proj>,
    /// LeapingSwordModel: an impact projectile that lands after `delay`.
    #[serde(default)]
    pub leap: Option<PlaceAt>,
    /// PlaceProjectileAtModel (Mega Mine): placed at a picked point.
    #[serde(default)]
    pub place_at: Option<PlaceAt>,
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
    pub name: String,
    /// Seconds.
    pub cooldown: f32,
    /// AbilityModel.addedViaUpgrade: the upgrade whose icon the button shows.
    #[serde(default)]
    pub upgrade: Option<String>,
    /// AbilityModel.startOffCooldown.
    #[serde(default)]
    pub ready: bool,
    #[serde(default)]
    pub activate: Vec<ActivateAttack>,
    #[serde(default)]
    pub turbo: Option<Turbo>,
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
    /// OverclockModel: the ability is used on a tower the player picks.
    #[serde(default)]
    pub overclock: Option<Overclock>,
    /// ImmunityModel.lifespan (seconds): leaks cost no lives.
    #[serde(default)]
    pub immunity: f32,
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
    pub radius: f32,
    pub rate: f32,
}

/// SpiritOfTheForestModel: within `close` of the Druid zone 0, within `middle` zone 1, else 2.
#[derive(Deserialize, Clone)]
pub struct Spirit {
    pub close: f32,
    pub middle: f32,
    pub zones: Vec<SpiritZone>,
}

/// DamageOverTimeCustomModel: `damage` every `interval` s (x mult + add on `tags`).
#[derive(Deserialize, Clone)]
pub struct SpiritZone {
    pub damage: f32,
    pub interval: f32,
    pub immune: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "one")]
    pub mult: f32,
    #[serde(default)]
    pub add: f32,
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
    pub lifespan: f32,
    pub tier: std::sync::Arc<Tier>,
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
}

/// ActivateAttackModel: `attacks` run for `lifespan` seconds (one volley when `one_shot`).
#[derive(Deserialize, Clone)]
pub struct ActivateAttack {
    pub lifespan: f32,
    pub one_shot: bool,
    /// turnOffExisting: the tower's own attacks pause meanwhile.
    pub turn_off_existing: bool,
    pub attacks: Vec<Attack>,
    /// Seconds before the attacks start (MarkedToPopModel's marking time).
    #[serde(default)]
    pub delay: f32,
}

/// TurboModel: weapon intervals x `mult`, +`extra_damage`, for `lifespan` seconds.
#[derive(Deserialize, Clone)]
pub struct Turbo {
    pub lifespan: f32,
    pub mult: f32,
    pub extra_damage: f32,
}

/// ToggleFocusStanceModel: in Focus stance range x `range`, weapon intervals x `rate`, projectile
/// lifespans x `lifespan`, and a LongWeapon fires in place of the others.
#[derive(Deserialize, Clone)]
pub struct Stance {
    pub range: f32,
    pub rate: f32,
    pub lifespan: f32,
}

/// DamageBasedAttackSpeedModel: +`increase` attack speed per `threshold` damage dealt, up to
/// `max_stacks`; all lost after `idle_frames` frames without damage.
#[derive(Deserialize, Clone)]
pub struct DamageSpeed {
    pub threshold: f32,
    pub increase: f32,
    pub max_stacks: u32,
    pub idle_frames: u32,
}

/// GyrfalconPatternModel: the bird waits `cooldown` seconds, dives at a bloon (maxSpeed x
/// bloonTargetSpeedMultiplier), grabs bloons within `radius` of it until `amount` is used up (each
/// costs 1 + its type's penalty), hits them for `initial` and then every `rate` seconds for
/// `grapple` (+ the MOAB / regrow modifiers), holding them still.
#[derive(Deserialize, Clone)]
pub struct Grapple {
    pub speed: f32,
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
    pub keep_range: f32,
    /// Extra grab cost per bloon tag (Lead, Ceramic, Moab, Bfb, Zomg, Ddt).
    pub penalty: std::collections::HashMap<String, u32>,
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
    /// Circle, FigureEight, Infinite or Centered.
    pub mode: String,
    pub rx: f32,
    pub ry: f32,
}

#[derive(Deserialize)]
pub struct SubTowerSpec {
    /// TowerModel name ("ShootyTurretTower").
    #[serde(default)]
    pub name: String,
    /// Footprint radius.
    #[serde(default)]
    pub radius: f32,
    pub tier: Tier,
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
}

/// DarkshiftModel: the new spot is within `radius` of the tower (placementZoneAssetRadius) when
/// `restrict` (restrictToTowerRadius).
#[derive(Deserialize, Clone)]
pub struct Darkshift {
    pub radius: f32,
    pub restrict: bool,
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
    /// MonkeyTempleModel.weaponDelay: seconds it holds fire while transforming.
    #[serde(default)]
    pub delay: f32,
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
    /// AddBehaviorToTowerMutatorModel support zones.
    #[serde(default)]
    pub supports: Vec<Support>,
    /// AddBehaviorToTowerMutatorModel PerRoundCashBonusTowerModel.cashPerRound.
    #[serde(default)]
    pub round_cash: f32,
    /// DamageTowerMutatorModel.damage, PierceTowerMutatorModel.pierce.
    #[serde(default)]
    pub damage: f32,
    #[serde(default)]
    pub pierce: f32,
    /// ProjectileSpeedTowerMutatorModel.speedModifier, ProjectileSizeTowerMutatorModel.sizeModifier.
    #[serde(default)]
    pub speed: f32,
    #[serde(default)]
    pub size: f32,
    /// RangeTowerMutatorModel.rangeIncrease.
    #[serde(default)]
    pub range: f32,
    /// ReloadTimeTowerMutatorModel.multiplier.
    #[serde(default)]
    pub reload: f32,
    /// WindChanceTowerMutatorModel.windChance: added to the chance of its WindModels.
    #[serde(default)]
    pub wind: f32,
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
}

/// CashModel (+ PickupModel.collectRadius when it has to be collected by hand).
#[derive(Deserialize, Clone)]
pub struct Cash {
    pub min: f32,
    pub max: f32,
    #[serde(default)]
    pub bonus: f32,
    pub pickup: Option<f32>,
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
}

#[derive(Deserialize, Clone)]
pub struct Wind {
    pub min: f32,
    pub max: f32,
    pub chance: f32,
    pub moabs: bool,
    /// Distance multiplier for bloons with one of `tags` (0 = unused).
    pub tag_scale: f32,
    pub tags: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct StateMod {
    pub states: Vec<String>,
    pub mult: f32,
    pub add: f32,
}

/// A bloon effect: "slow" (mult = speed multiplier; glue level), "freeze", "dot" or "brittle"
/// (damage = extra damage per hit).
#[derive(Deserialize, Clone)]
pub struct EffectSpec {
    pub kind: String,
    pub id: String,
    #[serde(default = "one")]
    pub mult: f32,
    pub lifespan: f32,
    /// Layers it lasts through: 1 = only the bloon hit, 2 = also its children, ...
    pub layers: u32,
    #[serde(default)]
    pub glue: u32,
    #[serde(default = "yes")]
    pub refresh: bool,
    /// SlowForBloonModel: only bloons with one of these tags.
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub moabs: bool,
    #[serde(default)]
    pub damage: f32,
    #[serde(default)]
    pub interval: f32,
    #[serde(default)]
    pub immune: u32,
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
    /// "emit": ProjectileOverTimeModel, fired at the bloon every `interval` (Sticky Bomb splash).
    /// "pop_emit": EmitOnPopModel, fired (with `emission`) where the bloon pops.
    #[serde(default)]
    pub projectile: Option<Box<Proj>>,
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
}

/// SlowBloonsZoneModel (Arctic Wind): bloons in the radius move at `scale` speed.
#[derive(Deserialize, Clone)]
pub struct Zone {
    pub radius: f32,
    pub bind: bool,
    pub offset: f32,
    pub scale: f32,
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
}

#[derive(Deserialize, Clone)]
pub struct PushBack {
    pub amount: f32,
    pub tag: String,
    /// Per-MOAB-class multipliers, keyed "MOAB", "BFB", "DDT", "ZOMG".
    pub mult: HashMap<String, f32>,
}

fn one_usize() -> usize {
    1
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
