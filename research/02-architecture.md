# 02: Original game architecture (static metadata)

Source: `tools/dotnet_meta/dotnet_meta.py` run over `MelonLoader/Il2CppAssemblies/Assembly-CSharp.dll`. It found 13,659 types, 131,172 fields, 219,830 methods and 69,193 properties. The dump is written to `research/dumps/` and git-ignored.

**What this source can and cannot show.** It gives names, types, member lists and signatures. It contains **no method bodies**: these are IL2CPP proxies, and the real code is native x64 inside `GameAssembly.dll`. Every statement below about *behaviour* is therefore an inference from a name or shape. Each one is labelled. Confirming it needs disassembly or a runtime observation.

Labels:
- **[V-meta]**: verified fact about structure, read from metadata.
- **[I]**: inference from names or shapes, behaviour unverified.

## 1. Three-layer split

Namespace sizes (types) show a strict Model / Simulation / Unity separation **[V-meta]**:

| Layer | Example namespaces | Role (inferred) |
|---|---|---|
| `Il2CppAssets.Scripts.Models.*` | `Models.Towers.Behaviors` (309), `Models.Towers.Projectiles.Behaviors` (248), `Models.Bloons.Behaviors` (156) | Immutable data definitions (`*Model : Model`, with `Clone`, `IsEqualAfterReferenceCheck`) |
| `Il2CppAssets.Scripts.Simulation.*` | `Simulation.Towers.Behaviors` (353), `Simulation.Towers.Projectiles.Behaviors` (221), `Simulation.Bloons.Behaviors` (137) | Runtime gameplay logic, one runtime class per model behaviour |
| `Il2CppAssets.Scripts.Unity.*` | `Unity.Towers.Behaviors` (239), `Unity.Bridge` (140), `Unity.UI_New.*` | Views, UI, and the bridge between Unity and the simulation |

The Simulation layer uses its own math types (`Simulation.SMath.Vector2/Vector3`, two or three `f32` fields) instead of `UnityEngine.Vector2` **[V-meta]**. `SMath.Vector2.ToUnity()` exists. This is the standard design for keeping a deterministic simulation independent of the engine **[I]**.

## 2. Game loop and time

- `Simulation.SimulationTime` has exactly one field, `int elapsed` **[V-meta]**. `Simulation` holds `time` and `roundTime` of this type.
- 328 methods have the signature `void Process(int elapsed)`, and 1,338 have `void Initialise(Entity target, Model modelToUse)` **[V-meta]**. Behaviours are per-entity components with a frame-integer process hook **[I]**.
- `Simulation.Simulate()` takes no arguments **[V-meta]**, which suggests a fixed step **[I]**. A static `BeginSimulate / Simulate / EndSimulate(double offset)` and `TimeSinceLastSimulate` also exist on a related driver, consistent with an accumulator-based fixed-step loop **[I]**.
- Processing goes through `NinjaKiwi.Common.LockList<RootObject> process`, and `Simulation.FlushProcessList()` exists **[V-meta]**. That points to deferred add and remove during iteration **[I]**.
- `Bloon.createdAtRoundTime: int` and `RoundData.roundStartTime: int` **[V-meta]** show that timestamps are in frames.
- `GameModel.penaltyGraceTimeFrames: int` **[V-meta]**.

Open questions: frames per second (H-TIME-1), and whether fast-forward runs more `Simulate()` calls or passes `elapsed > 1`.

## 3. Determinism and RNG

- `Simulation.syncedRandom` and `Simulation.unsyncedRandom`, both `NinjaKiwi.Common.BufferedRandom` **[V-meta]**. The synced/unsynced split implies gameplay RNG is kept separate from cosmetic RNG, as co-op lockstep needs **[I]**.
- `BufferedRandom` holds `NinjaKiwi.Common.Random rng`, `int next` and `double inverse`. Its methods are `Next()`, `Next(max)`, `Next(min,max)`, `NextDouble()`, `PeekNext()` and `PeekNextDouble()`, constructed with `(int seed)` **[V-meta]**.
- `SeededRandom` has static `int Multiplier`, `int Modulus` and `long currentSeed`, with `Next`, `NextFloat`, `Range`, `NextNormal` **[V-meta]**. This is an LCG **[I]**. Its constants are assigned in the native `.cctor` and are **not** in the metadata.
- `GameModel.randomSeed: int` **[V-meta]**.
- Headless deterministic testing exists in the shipped code: the `Il2CppAssets.Scripts.SimulationTests` namespace includes `SimulationTestEnvironment`, `RunReplay`, `RunRounds`, `DelayFrames`, `CreateTower`, `UpgradeTower`, and outcomes `FrameCount`, `GameWon`, `Health`, `CashUsed`, `GameModelUnchanged`. There is also `Simulation.IsHeadless` **[V-meta]**. This is strong evidence that the simulation is designed to be deterministic and replayable without rendering **[I]**.

## 4. Anti-tamper value wrapper

Cash, health, shield and revives are `Utils.KonFuze` **[V-meta]**. A KonFuze holds a buffer of doubles, two `SeededRandom` instances, `honey`/`pot` and a `HackDetected` flag. It's a memory-obfuscation wrapper around a `double` **[I]**. For a reimplementation this only means these values are `f64` (H-ECON-1). The obfuscation itself is not reproduced and is not analysed further.

## 5. Bloons, tracks, rounds (first subsystem)

- `BloonModel` fields include `id`, `baseId`, `tags`, `speed`, `speedFrames`, `maxHealth:int`, `leakDamage`, `totalLeakDamage:float?`, `layerNumber`, `radius`, `isMoab`, `isCamo`, `isFortified`, `isGrow`, `bloonProperties`, `behaviors: Model[]`, `childBloonModels` **[V-meta]**.
- `Bloon` (runtime) fields include `health:int`, `distanceTraveled:float`, `trackSpeedMultiplier`, `spawnRound`, `emissionIndex`, `lineage`, `armour` (a queue), `pendingDamageTasks`, `chargedMutators`. Methods include `Move`, `Damage`, `Degrade`, `Leaked`, `UpdatePositionAlongTrack` **[V-meta]**.
- `Track.Path` fields: `totalPathLength`, `segments: PathSegment[]` (each with `min`, `max`, `centre` and a list of bloon proxies), `MaxSegmentSize`, `CacheDistance`, and precomputed `bestFirst/Last/Strong{All,NonMoab,NonCamo,NonMoabNonCamo}` **[V-meta]**. Targeting candidates appear to be cached per path per frame, bucketed by distance segment **[I]**.
- `Models.Map.PointInfo`: `point`, `rotation`, `distance`, `bloonSpeedMultiplier`, `bloonsInvulnerable`, `moabsInvulnerable`, `disablePathTargeting` **[V-meta]**.
- `RoundModel` has `groups: BloonGroupModel[]` (`bloon`, `start`, `end`, `count`) and `emissions: BloonEmissionModel[]` (`bloon`, `time:float`, `emissionIndex`) **[V-meta]**. `Spawner` has `roundData`, `Emit`, `CheckForRoundEnd`, and separate `RoundManager` / `FreeplayRoundManager` **[V-meta]**.

## 6. Collision

`Simulation.Physics`: `CollisionChecker`, `CollisionGrid<T>`, `Bucket<T>`, `Collidable<T>`. `Bloon : Collidable<IBloonBehavior>` **[V-meta]**. This is a uniform-grid broad phase **[I]**.

## 7. Game data source (high value)

- `Data.GameData : ScriptableObject` has a `TextAssetReference gameModel`, along with `roundSets`, `incomeSets`, `mods`, `mapSet` **[V-meta]**.
- `GameModelLoader` holds a `BinaryReader br` and an `object[] m`. Its generated methods are `Create_Records`, `CreateArraySet`, `Link*`, `Read_a_<T>_Array`, and one `Set_v_<ModelType>_Fields(int start, int count)` per model type **[V-meta]**. It's a code-generated binary deserializer **[I]**.
- The data is in `game_model_assets_all_*.bundle` → `CAB-e0d6…` → TextAsset **`sim_model`**, 17,076,980 bytes **[V]** (see `03-sim-model-format.md`).

## 8. Likely CPU and memory overhead sources (hypotheses to profile)

These are all **[I]** and must be benchmarked before anyone relies on them:
1. Per-behaviour virtual `Process` calls across many small heap objects (component-per-class design): poor cache locality.
2. `Il2CppSystem.Collections.Generic.List/Dictionary` everywhere, with IL2CPP write barriers and bounds checks.
3. Event delegates (`Action<...>` on every bloon: `OnBloonDegrade`, `OnBloonLeaked`, ...): allocation and indirect calls.
4. KonFuze reads and writes for cash and health: a buffer shuffle on every access.
5. Late-game RBE: child bloon creation on degrade (`childrenCreatedOut`, `lineage` lists) causes allocation churn.
