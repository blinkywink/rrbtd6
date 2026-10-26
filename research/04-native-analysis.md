# 04: Native behaviour recovered from GameAssembly.dll

Build 57.2.12199, `GameAssembly.dll` image base `0x180000000`. Everything here was transcribed from machine code with VS 2022 `dumpbin /disasm`, driven by `tools/il2cpp/disasm.py`. Method addresses come from the IL2CPP code-gen module (`tools/il2cpp/codegen.py`): the Assembly-CSharp module is at RVA `0x4190AE0` with 67,383 method pointers, which matches the metadata count exactly. Field offsets come from `research/dumps/field_offsets.json`, extracted by the forked session from `Il2CppMetadataRegistration`.

Labels:
- **[V-asm]**: read directly from the instructions. Constants are quoted with their address.
- **[I]**: inferred.

Re-verify everything after a game update. Addresses change with every build.

## 1. Frame clock: `InGame.UpdateSimulation` @ `0x180AC4AD0` [V-asm]

```
acc += (double)(Time.deltaTime * 60.0f)                 // f32 constant @0x183F75C50
if (this[0xE0] == null) acc = Math.Min(acc, (double)Time.timeScale + 1.0)
TimeManager.Update(...); TimeManager.BeginSimulate()
while (acc >= 1.0) { TimeManager.Simulate(); bridge.Simulate(); acc -= 1.0 }
TimeManager.EndSimulate(acc * 0.016666667535901070)     // (double)(1/60f)
```

- **60 simulation frames per scaled second.** H-TIME-1 is confirmed.
- `Time.timeScale` is set by `TimeManager.Update` to `clamp(base * timeScale * (FF ? 3.0 : 1.0), 0, 100)` (constants `0x183F75E58` = 3.0, `0x183F75F38` = 100.0). Fast-forward is 3×.
- At most `timeScale + 1` ticks run per rendered frame. Under low frame rates the game drops ticks rather than catching up. The cap is skipped when `this[0xE0]` is set (purpose unknown).

## 2. One tick: `Simulation.Simulate` @ `0x180A7C350` [V-asm]

Simulation fields: `time` +0x30, `roundTime` +0x38, `factory` +0x60, `effectLimitManager` +0x128, `process` +0x138, `elapsedSinceFlush` +0x220, `map` +0x2A0.

1. `effectLimitManager.Reset()`
2. `time.elapsed++`
3. `roundTime.elapsed++`, gated by `!spawner.AreRoundsActive()` plus a behaviour check that isn't fully decoded [I]
4. `elapsedSinceFlush++`; when it reaches 60, it resets and `factory.Flush()` runs
5. `FlushProcessList()`
6. For each root object in `process`, call `UnHold()`. Then for each, call `ProcessRootObject(time.elapsed)`. **`Process(int elapsed)` receives the absolute frame number.**
7. `UpdateStagedCash()`, the sandbox health timer, auto-start of the next round (`Spawner.StartRound`), then `Spawner.CheckForRoundEnd()`

## 3. RNG [V-asm, tested]

The Rust implementation is `engine/src/rng.rs`.

### `SeededRandom` (state `long` at +0x10)

| Method | Behaviour |
|---|---|
| `ctor(long s)` | `s >= 1 ? s : -s` |
| `ctor(int s)` | Same, on 32 bits |
| `SetSeed(uint)` | Zero-extend |
| `SetSeed(long)` | `abs` |
| `Next()` | `s = abs(s*16807 % 2147483647)` with 64-bit signed truncating remainder (Park–Miller) |
| `NextFloat()` | `(float)(int)s * 2^-31f` (`0x1840EBF40`) |
| `NextDouble()` | `(double)(int)s / 2147483646.0` (`0x1840EBDE0`) |
| `Range(int a, int b)` | Bounds sorted; equal bounds return without advancing; otherwise `lo + s % (hi - lo)` |
| `Range(float a, float b)` | Bounds sorted; `(s*2^-31f)*(hi-lo)+lo` |

### `NinjaKiwi.Common.Random`

`InternalSample` is **byte-identical** to `System.Random::InternalSample` (the linker folded them to `0x182D5AE90`). The seeding constructor @ `0x182EF7100` is the .NET Framework algorithm:

- `MSEED = 161803398` (`0x9A4EC86`), with `int.MinValue → int.MaxValue`
- Fill with `(21*i) % 55`
- 4 mixing passes with `1 + (i+30) % 55`
- `inext = 0`, `inextp = 21`

`Sample = InternalSample * (1/int.MaxValue)` (`0x183F76540`). `Next(min, max)` uses the large-range path when `max - min > int.MaxValue`.

### `BufferedRandom` (`Simulation.syncedRandom` / `unsyncedRandom`)

Fields: `rng` +0x10, `int next` +0x18.

- **ctor:** `rng = new Random(seed); next = rng.Next()`.
- **`Next()`:** returns the old `next` and refills.
- **`NextDouble()`:** `old * (1/int.MaxValue)`.
- **`Next(max)`:** `(int)((old*scale)*max)`.
- **`Next(min, max)`:** `(int)((old*scale)*(max-min)) + min`. It **throws** when the range exceeds `int.MaxValue`, unlike `System.Random`.

Tests confirm the Park–Miller 10,000th value `1043618065` and .NET `Random(0)` → `1559595546, 1755192844, 1649316166`.

## 4. Paths

### `Path.CalcPointInfo` @ `0x180F84FF0` [V-asm]

**Pass 1.** For each `PointInfo` in order:
- `distance = total` (the stored value is overwritten).
- If `point == PathModel.spawnPoint` → `spawnDist = total`. If `== leakPoint` → `leakDist = total`. Both default to -1.
- `total += Vector3.Distance(p[i], p[i+1])`.

`Vector3.Distance` = `sqrtss((dx*dx + dy*dy) + dz*dz)`, all f32.

**Pass 2.** `n = (int)(total / 0.01f)`; build a `QuickPointInfo[n+1]` table of `{int extra; Vector3 pos; float rot}`:
- For `i in 0..n`: `d = (float)i * 0.01f`.
- Advance segment `s` while `d >= dist[s]`.
- `t = (d - dist[s-1]) / (dist[s] - dist[s-1])`.
- `pos = p0*(1-t) + p1*t`. This is the two-weight form, which settles H-MATH-2.
- `rot = CapAngle(atan2f(dx, dy) * 57.29578f)`. The argument order really is `(x, y)`. `CapAngle` wraps into `[0, 360)`.
- `extra = (p0.id != null || p0.HasExtraSettings() || p1.HasExtraSettings()) ? s-1 : -1`.
- Finally `table[n] = table[n-1]`.

Statics (`Path..cctor`): `CacheDistance = 0.01f` (`0x3C23D70A`), `MaxSegmentSize = 7.0f`.

### `Path.DistanceToPointInfo` @ `0x180F85CD0` [V-asm]

`table[clamp((int)(d / 0.01f), 0, n)]`, with an error log when it's out of range. **There is no interpolation.**

### `Path.DistanceToPointInfoBloon` @ `0x180F85C30` [V-asm]

The point above, with `y -= 4.0f` and `z += {boss: 180, moab: 120, else: 20}` (constants `0x183F75EBC`, `F74`, `F70`, `C48`).

### `Path.DistanceToPath(ref d, model)` @ `0x180F85B20` [V-asm]

- While `d >= total`: with an exit router, `d -= total` and move to the routed path; otherwise `d = total` (exact clamp) and stop.
- While `d < 0`: with an entry router, move to the routed path and `d += total`; otherwise `d = 0`.

### `Path.IsEndOfMap(d, strict)` @ `0x180F874C0` [V-asm]

- False if `exitRouter != null`.
- True if `d == total`.
- Otherwise, if `leakDist != -1`: `strict ? d >= leakDist : d == leakDist`.

## 5. Bloon movement [V-asm]

Bloon fields:

| Field | Offset | Field | Offset |
|---|---|---|---|
| `_path` | +0x160 | `distanceTraveled` | +0x21C |
| `bloonModel` | +0x168 | `offTrackCount` | +0x224 |
| `trackScale` | +0x1F8 | `stopMovementUntilPop` | +0x242 |
| `trackSpeedMultiplier` | +0x200 | `doesBloonBehaviorModifySpeed` | +0x243 |
| `nonTargetable` | +0x218 | radius | +0x150 [I] |

### `Bloon.Process` @ `0x180FCD2C0`

If `_path != null && offTrackCount <= 0`:

```
distanceTraveled = TotalSpeedFrames + distanceTraveled
UpdatePositionAlongTrack(true)
```

### `get_TotalSpeedFrames` @ `0x180FD02F0`

```
if stopMovementUntilPop: return 0
m = 1.0f
if doesBloonBehaviorModifySpeed: m *= each speed-modifying behaviour
return ((model.speedFrames * trackSpeedMultiplier) * map.mapWideBloonSpeed) * m
```

`map.mapWideBloonSpeed` is at Map +0xC0.

### `UpdatePositionAlongTrack(bool smooth)` @ `0x180FCEA40`

1. If `path.leakDist != -1 && dist - radius >= path.leakDist` → `Leaked()`.
2. `_path = DistanceToPath(ref dist)`.
3. If `IsEndOfMap(dist)` → `Leaked()`.
4. Spawn clamp: if `spawnDist != -1 && spawnDist >= radius*1.25 + dist` → `dist = spawnDist - radius*1.25`.
5. Position from `DistanceToPointInfoBloon`.
6. If `model.rotateToFollowPath`: `rot = smooth ? LerpAngle(prev, target, TotalSpeedFrames*0.5) : target`.
7. If the point has extra settings: `trackScale` (bloon or MOAB scale), `nonTargetable` and `trackSpeedMultiplier` come from the extra point info.

### `Bloon.Leaked` @ `0x180FCBFA0`

Lives lost = `Simulation.CalcDamageFromBloon(bloon, model.GetTotalLeakDamage(...))`, after tower hooks (`OnPreLivesLost`, the `LeakDamageMultiplier` mutator). `CalcDamageFromBloon` is not yet transcribed (H-LEAK-2).

## 6. Rounds and spawning [V-asm]

### `BloonGroupModel.GetEmissions` @ `0x1810CC280`

```
span = end - start
count == 1: offset = span * 0.5                  // single bloon spawns at the midpoint
otherwise:  step = span / (count-1), offset = 0
time_i = ((float)i * step + start) + offset      // f32
```

### `Spawner.AddEmissions` @ `0x180F897E0`

`roundStartTime = time.elapsed`. Each emission is cloned with `time = (float)time.elapsed + e.time` (f32 add) and `emissionIndex = k++` in array order. `roundDuration = max(e.time)`.

**Emission times are in frames.** This replaces H-ROUND-1.

### `Spawner.Process(elapsed)` @ `0x180F8C180`

For every active `RoundData`, and every remaining emission in list order: if `e.time <= (float)elapsed`, call `Emit`. Otherwise **continue**: there's no early exit, so every pending emission is scanned every frame.

### `Spawner.Emit` @ `0x180F8A540`

`bloon.spawnRound`, `emissionIndex`, then `SetPath(path, startDist, true)`. `startDist` is 0, or **40.0** for bosses (`0x183F75F84`).

### `Bloon.SetPath` @ `0x180FCDC20`

`dist = startDist`; `_path = DistanceToPath(ref dist)`; `nonTargetable = false`; `trackScale = trackSpeedMultiplier = 1`; then position and rotation from the point table.

## 7. Performance observations

All of these are [I] and need profiling.

- The spawner scans every pending emission every frame (O(pending) per tick).
- `TotalSpeedFrames` enumerates bloon behaviours through an interface-dispatch enumerator for any bloon with speed-modifying behaviours.
- `Simulate` makes two full passes over the process list (`UnHold`, then `Process`).
- The path table costs 20 bytes × `length / 0.01` per path, around 100k entries (2 MB) for a 1000-unit path. Our engine can keep this exact while sharing tables between identical paths.

## 8. Map paths from scenes [V-asm unless marked]

Maps exist only as scene objects. Each path is a `Unity.Map.Path` GameObject whose children are `LineNode` transforms and `Curve` objects carrying `BezierLineNodes`. The forked session found this; see `tools/assets/export_map.py`.

### `Unity.Map.Path.get_Def` @ `0x1814A6520`

Gets `PointInfo[]` from the child nodes and maps every point (plus the spawn and leak transforms) through `Vector3.ToSimulationSpace(p, true, 60.0f)`, then builds the `PathModel`.

### `Vector3.ToSimulationSpace` @ `0x180F605C0`

```
s = (u.x, -u.z, u.y)
if tilt: k = tanf(Deg2Rad * angle)        // Deg2Rad = 0x3C8EFA35; cached in a static (+0x60)
         if k != 0: s.y -= s.z / k
```

### `BezierLineNodes.<GetPoints>d__0.MoveNext` @ `0x1814ABB40`

Walks the base control points as `start, (t1, t2, end)*`. Each triple yields `MakeBezierPoints(prevEnd, end, t1, t2, 30)`. Leftover points are dropped. The shared endpoints are duplicated, which gives zero-length segments; `CalcPointInfo`'s segment walk skips over those.

### `SMath.Math.MakeBezierPoints` @ `0x180EA11D0`

```
t = i / (n-1),  u = 1-t
P = ((s*uuu + ((t1*uu)*t)*3) + ((t2*u)*tt)*3) + e*ttt
```

The constant 3.0 is at `0x183F75D28`.

### `PolyLineNodes.GetPointInfo` @ `0x1814A7330`

For each child Transform, in child order (the Transform enumerator includes inactive children):
- If the child has a `LineNodes` component (such as a Bezier `Curve`), add `child.GetPointInfo()`. A curve's first control point repeats the previous LineNode, and so does the LineNode after it, so the duplicates are real.
- Otherwise, add `PointSource.CreatePointInfo(this, child, child.position)`.

### `PointSource.CreatePointInfo` @ `0x1814A7030`

Defaults are `id = null`, `bloonScale = moabScale = bloonSpeedMultiplier = 1`, not invulnerable, and targeting not disabled. Values are copied from a `PointSource` component when the child has one.

### Spawn and leak points

`Path.get_Def` starts both at `Vector3.invalid`, which never matches a point, so `spawnDist` and `leakDist` stay -1. It overrides them from `GetComponentInChildren<Unity.Map.SpawnPoint>` / `<LeakPoint>`. Monkey Meadow has neither (forked session's export).

### Remaining uncertainty

H-PATH-3: world positions come from the forked session's exporter, which applies the Transform chain in Python. Unity computes them in f32, so the last ulp may differ.

### First real-map result

Monkey Meadow is 32 raw nodes → 352 points, length 1276.3029, and a 127,631-entry table. The predicted round-1 Red leak is at frame 3065 (51.083 s). That's experiment E1 in `tests/observations/README.md`.

## 9. Difficulty speed mod [V-asm]

- **`GlobalSpeedModModel.Mutate`** @ `0x181137150` calls `GameModelUtil.ApplyModToAllBloons(gameModel, ModBloon)`, which invokes the delegate once per element of the bloon array.
- **`GlobalSpeedModModel.ModBloon`** @ `0x1811370F0`: `saved = b.isImmuneToSlow; b.isImmuneToSlow = false; b.set_Speed(b.speed * multiplier + addition); b.isImmuneToSlow = saved`. Fields: `multiplier` +0x28, `addition` +0x2C.
- **Mode data** (`modes.json`): Easy has none, Medium is ×1.1f, Hard is ×1.25. Impoppable's own mod list has no speed mod. Whether it inherits Hard's mods is H-MODE-2.

## 10. Placement areas [V-asm]

- **`Unity.Map.Area.ToModel`** @ `0x18149CF40` (called from `Area.get_Def`, which `MapLoader.GatherMapAreasAndBlockers` @ `0x1814A16B0` calls): `pivot = ToSimulationSpace(pivotPoint.position, true, 60)` (`pivotPoint` is the Transform at +0x60). Polygons come from `PolygonArea.BuildPolygons(transform, polygons)`.
- **`PolygonArea.BuildPolygons`** @ `0x1814A7790`: each polygon becomes `new Polygon(points.Select(p => …).ToArray())`.
- **The point lambda** @ `0x1814B04F0`: `Vector2(ToSimulationSpace(transform.TransformPoint(p), true, 60).xy)`.

**Result:** area polygons get the same 60° tilt as paths. An elevated area, such as Monkey Meadow's tree canopies at y = 100, shifts by `-y/tan60` in sim-y. The forked session found this effect empirically, and the code confirms it. Area fields: `type` +0x38, `isBlocker` +0x58, `pivotPoint` +0x60, `blockerCircles` +0x78.

## 11. Tower placement: `Map.CanPlace` @ `0x180F69360` [V-asm, partly I]

Signature: `CanPlace(Vector2 at, TowerModel tm, Tower existing, CanPlaceReturnData ret, bool checkTowerOverlaps, bool ignoreInputBlockers, bool isTryPlacing)`.

Constants: footprint scale 0.375 (rect half-extent factor) and 0.75 (circle factor) (`0x183F760B8` / `0x183F75DCC`), height window 5.0 / -5.0, height limit 250.

```
if existing && existing.towerModel.footprint.ignoresPlacementCheck && !existing.isBeingRedeployed: return true
if !IsPointOnMap(at, border = -tm.radius): return false
a0 = GetAreaAtPoint(at); if a0 && a0.model.height + a0.model.renderHeightOffset >= 250: return false
if IsPointOnCustomPath(at, tm.radius): return false
if isTryPlacing: tm = artifacts.GetModifiedPlacementTowerModel(tm)
rect = tm.footprint as RectangleFootprintModel
aabb = rect ? AABB(at - (xW, yW)*0.375, at + (xW, yW)*0.375) : null
for area in map.areas (list order; first passing area wins):
    skip if ignoreInputBlockers && area.model.isInputBlocking
    skip if area.model.type == 5
    skip if area.model.dontRemoveExistingTowers && existing && (!tm.isPrimedForRedeploy || !area.isActive)
    skip if !area.model.IsPointInside(at)
    skip if existing && existing.createdAreas.Contains(area)
    if !IsPlaceableInAreaType(at, tm, area.model.type, area.model.lockedArea, existing != null): return false
    if !CanHaveTowerSize(area.model, tm): return false
    for o in map.areas, o != area, o.type != 5, not created by existing:
        if o.type == track && area.model.prohibitTrackPlacement:
            if o.isActive && (o.IsPointInside(at) || aabb ∩ o || circle(at, tm.radius*0.75) ∩ o): return false
        else if 0 <= o.height - area.height <= 5
                && !IsPlaceableInAreaType(at, tm, o.type, o.lockedArea, …)
                && (aabb ∩ o || circle(at, tm.radius*0.75) ∩ o):
            return false
    [I] skip areas created by `existing`'s own source-tower chain (sourceTower / GetAreaWithID)
    [I] if the footprint spills outside `area`, a second pass over areas with |Δheight| <= 5 that contain `at` requires them to be placeable
    if BlockedByBlockerZone(aabb, at, tm.radius): return false
    if checkTowerOverlaps && !tm.footprint.ignoresTowerOverlap:
        for t in towers (t != existing, !t.footprint.doesntBlockTowerPlacement,
                         [I] height filter unless area.model.ignoreZAxisTowerCollision):
            if OverlapsOtherEntity(aabb, at, tm.radius, t.position, t.towerModel.radius, t.footprint): return false
    ret.areaPlacedOn = area; return true
return !tm.IsBeastHandlerPet        // no containing area (the map base area normally covers everything)
```

### `IsPlaceableInAreaType(at, pe, type, lockedArea, forExisting)` @ `0x180F73D00`

Returns `pe.IsPlaceableInAreaType(type)` if true. Otherwise: `lockedArea ? false : ZoneAllowsPlacement(at, pe, type)`. `TowerModel.IsPlaceableInAreaType` checks `areaTypes`; for the Dart Monkey that's `[land]`.

### `OverlapsOtherEntity(aabb, c, r, otherPos, otherR, otherFootprint)` @ `0x180F752C0`

```
if otherR <= 0 || otherPos.z >= 250: return false
if aabb == null:
    if otherFootprint is Rect:
        return AABB(otherPos ± (xW, yW)*0.5).CircleCollision(c, r)
    return (r + otherR)^2 > DistanceSquared(otherPos.xy, c)    // model radius, strict
...rect-vs-* branches not transcribed
```

### `BlockedByBlockerZone(aabb, at, radius)` @ `0x180F67D40` [V-asm]

Walks the map's blockers. Each one has a circle (centre, radius `rb`) and a flag byte at the offset of `BlockerModel.isDisabled` (which field it is: [I]).

```
aabb == null, flag == 0:  blocked if (rb + radius)^2 >= DistanceSquared(centre, at)   // non-strict
aabb == null, flag != 0:  blocked if DistanceSquared(centre, at) >= (rb - radius)^2  // must sit inside
aabb != null, flag == 0:  blocked if aabb.CircleCollision(centre, rb)
aabb != null, flag != 0:  skipped
```

`CanPlace` passes the full `tm.radius` here, not the 0.75 footprint factor.

### `IsPointOnMap(at, border)` @ `0x180F74030` [V-asm]

`worldXMin - border <= x <= worldXMax + border` and the same for `worldZMin/Max` on sim-y. The constants are `worldX = ±150` (`0x183F764DC`, `0x183F75C58`) and `worldZ = ±116` (`0x183F764D8`, `0x183F764D0`). `worldXMin/Max` add 28 (`0x183F76154`) when a flag function returns true, but that function (`0x180530F50`) is compiled to `return false`.

### Map areas and blockers [V-asm]

- **`MapLoader.Load`** calls `GatherMapBaseArea` (`0x1814A1B90`, from `0x1814A37D6`) **before** `GatherMapAreasAndBlockers` (from `0x1814A37E0`). The base area is built from the world bounds, so it is `map.areas[0]` and is the first area that contains any on-map point. The exact base-area shape (assumed: the world-bounds rectangle, land, height 0) is [I].
- **`Area.ToModel`**: `AreaModel.height = transform.position.y`; `renderHeightOffset` comes from the MonoBehaviour.
- **`GatherMapAreasAndBlockers`** iterates `GetComponentsInChildren<Area>` and appends each `Area.get_Def`. For an area with `isBlocker` and `blockerCircles`, each serialized circle becomes `BlockerModel(name, id, Circle(Vector3(x, y, area.height), r), area.isDisabled)`. The circle positions are used **untransformed**: they are authored in sim space. On Monkey Meadow, all 119 tree circles fall inside the tilt-corrected tree polygons, which confirms this.

**Consequence on Monkey Meadow:** the base area always decides. Track polygons (height 0) are tested against the 0.75 × radius circle. Tree polygons (height 100) fall outside the 0–5 height window, so they never block on their own; the trees block placement only through their blocker circles, at full radius.

### Not transcribed

`Area.Intersects` (circle and AABB vs polygon), `AreaModel.IsPointInside`, `CanHaveTowerSize`, `ZoneAllowsPlacement`, and `GetAreaAtPoint`.
