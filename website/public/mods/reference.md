# RustTD6 mod API 1

[Website guide](https://rusttd6.com/mods/) · [Schema](mod.schema.json) · [SDK](../tools/modsdk.py)

Mods are authored in files, not an in-game editor. Author scripts run only on an explicit build
command. The native client loads generated data and optional embedded Rhai event scripts, never Python or injected DLLs.

## Quick start

Copy [the example](../website/public/mods/quickstart.py) to `mods/quickstart/mod.py`, then run:

```text
python tools/modsdk.py build mods/quickstart/mod.py
```

Enable Advanced > UI > Show Mods Button, open Mods, Reload Mods, select the mod and Enable Mod.
Open Mods Folder opens the installation directory. Selecting a row only previews it.
Changes apply to a new/restarted game without restarting the client. Live match entities keep
the definitions they started with. Hiding the home button does not disable enabled mods.

The included Practice Start example changes Sandbox cash/lives only. That optional hook does
not define the scope of the content API: content mods can change ordinary local games too.
Official events bypass local content mods. Private iroh rooms use the host's room mod set,
not individual local enabled lists.

## Manifest

`mods/<folder>/mod.json` is strict JSON. `api_version: 1`, unique lowercase `id` (1-64 ASCII
letters/digits/._-, starting alphanumeric), nonempty `name` (80 UTF-8 bytes), `version` (32
bytes), optional `description` (1024 bytes), optional `content`, `on_new_sandbox`, `settings`.
Text metadata has no control characters. Limit: 64 KiB per file, 256 operations per mod,
256 installed mod folders. Duplicate IDs are invalid. Enabled IDs live in local settings
under `mods`; missing/invalid enabled IDs are reported rather than silently removed.

## Content operations

Operations run in written order; enabled manifests compile in sorted ID order.

- `tower`: `{op,id,base,patch?}` clones a non-hero tower, tiers, shop entry and shared visual
  references. `patch` changes existing base fields such as cost/range/icon. Use namespaced
  IDs (the SDK prefixes the mod ID). No game assets are copied or distributed.
- `bloon`: `{op,id,base,patch?}` clones a bloon and adds it to the Sandbox list. Camel-case
  native fields include `maxHealth`, `isCamo`; children must exist and cannot form cycles.
- `mode`: `{op,id,base,patch?}` clones a mode. New IDs appear in the mode screen's custom
  selector; IDs are stable names, not indices.
- `all_modes`: `{op,patch}` changes every mode present at this point in the operation list.
- `set`: `{op,path,value}` replaces an existing JSON Pointer. Roots: `towers`, `tower_tiers`,
  `bloons`, `modes`, `start`, `rounds`, `round_sets`, `freeplay`. Weapon/projectile/upgrade and
  native statistics changes use these paths. Duplicate exact `set` paths conflict.
- `copy`: `{op,from,path}` copies an existing native definition to another existing path.
  The SDK uses this for reusable projectile templates, e.g.
  `bomb = mod.projectile("bomb", base_tower="BombShooter", damage=10)` and
  `tower.projectile(bomb, pierce=30)`. No runtime script or registry lookup is needed.
- `menu`: `{op,page,path,text?,visible?}` changes an existing frontend scene node when loaded.
  `page` is the prefab filename without `.json`, e.g. `MainMenuUI`; paths are Node-relative.
  Dynamic UI bindings can replace a load-time edit. No arbitrary UI code executes.

Inspect your local `local_extracted/game/game.json` and `ui/prefabs` for supported fields.
Patches reject misspelled/missing fields. Definitions are deserialized before use; an invalid
content transaction leaves the base data unchanged and reports its error in the client log.
The native engine executes existing behavior types: arbitrary new behavior, scripted widgets,
custom profile counters and a custom art importer remain future API work. This is not a
MelonLoader compatibility layer. Do not bundle or redistribute extracted BTD6 content.

## Python authoring helpers

The SDK runner supplies `Mod` to the author script; leave your instance named `mod`.

```python
mod = Mod("example", "Example")
tower = mod.tower("dart", base="DartMonkey", cost=100)
tower.projectile(damage=3, pierce=4)
tower.weapon(rate=0.5)
mod.bloon("strong-red", base="Red", maxHealth=5)
mod.mode("practice", base="Medium", cash=2000)
# Optional global/edit examples:
mod.all_modes(lives=500)
mod.set("/modes/Medium/cash", 1500)
```

Projectile/weapon helpers default to tier `000`, attack 0, weapon 0. Pass `tier`, `attack` or
`weapon` to edit others. Inherited upgrade tiers retain their original values unless edited.
Other languages may emit the same JSON; there is no Python runtime dependency in gameplay.

## Optional Sandbox settings

`on_new_sandbox` may set whole-number `starting_cash` (0..1e12), `starting_lives` (1..1e9).
Omit a value to retain the game's value. `settings` can expose bounded +/- controls:

```json
"on_new_sandbox": { "starting_cash": 100000 },
"settings": { "starting_cash": { "min": 0, "max": 1000000, "step": 10000 } }
```

Bounds, default and step are whole numbers. Step must be positive; both bounds and default
align to it from `min`. Overrides live in local `mod_settings`, survive disable, and use the
same resolver for preview/gameplay. Reset restores defaults, including after an author removes
all controls. Invalid settings in a disabled mod do not block enabled mods. Sandbox rules
only run in new local Sandboxes; two enabled mods setting the same Sandbox value conflict.

## Private co-op

Open Room Mods in an iroh lobby. Only the host selects the set; guests can inspect it. The
host sends hash-checked manifests over the existing encrypted P2P connection, including to
clients without those mods installed. Room data is held in the room snapshot, isolated from
local installed mods; author scripts/native code are never sent or executed. Initial compiled
state is checked by the normal load barrier. Changes clear readiness; match start locks mods.
Limits: 16 data mods / 24 KiB of source manifest text (40 KiB encoded). Art packs and large
chunked transfers are not supported yet. Identical base client/build/data are required.

## Updater

Release `version.json` includes `assets_revision`; full extraction stamps the corresponding
`local_extracted/assets_version.json`. Compatible code updates install through the launcher.
An asset mismatch offers old-version play or rerunning the setup mod inside BTD6. New assets
are staged before the recoverable release swap; preferences, local profiles and local name seeds
are preserved. A failed extraction never replaces the working assets. Installed game EXEs
route through the launcher; development builds without a release manifest stay direct.

## Custom models and timed throws

Start with a complete original example (no game art is included):

```
python tools/mod_model.py template mods/throwbot
python tools/modsdk.py build mods/throwbot/mod.py
```

Enable Throwbot in Mods, then start a new game. It has a blue animated robot body,
a gold custom projectile and a release socket. All importing/baking happens on the
author's computer; gameplay uses the existing native mesh, animation and weapon paths.

In an author script:

```python
mod = Mod("my-mod", "My mod")
tower = mod.tower("robot", base="DartMonkey")
tower.model("robot")
tower.projectile(display=mod.model("bolt"), damage=3, pierce=4)
```

Put the files in `mods/my-mod/assets/robot/` and `assets/bolt/`. The folder must match
the mod ID. `tower.model` reads the baked `mod_socket`: it applies the primary muzzle
and release lead to every inherited upgrade tier. Secondary weapons keep their original
sockets. Projectile stats can be changed per tier using `tower.projectile(tier="100", ...)`.

For a simple mesh, export a triangulated OBJ and a PNG atlas from your modeling tool:

```
python tools/mod_model.py import robot.obj --texture atlas.png --out mods/my-mod/assets/robot --muzzle 4 8 6 --release 0.15
```

OBJ space: Y up, +Z forward, feet at the origin. Coordinates are game units (the starter
is about 14 units tall). `--muzzle X HEIGHT FORWARD` is the local hand/barrel release
point. `--release` is seconds into Attack. With no attack sequence, the model aims and
fires correctly but has no animated throwing motion. Add `--attack frame01.obj frame02.obj ...`
with identical topology/face order to bake that motion. Use `--fps` for the sample rate.

For rigged animation, bake the evaluated Blender timeline directly:

```
blender --background robot.blend --python tools/blender_mod_model.py -- --out mods/my-mod/assets/robot --idle 1 60 --attack 61 90 --release 70 --texture atlas.png
```

Blender space: Z up, -Y forward. Add an Empty named `Muzzle`, parent it to the throwing
hand or barrel, and put export meshes in a `RustTD6` collection. The example uses frames
1–60 for Idle and 61–90 for Attack, with release at frame 70. Rigs and modifiers are
baked into vertex poses. Use one PNG atlas; arbitrary Blender material shaders are not
imported. Keep topology stable. Limits: 60,000 split vertices, 600 frames per clip,
128 MiB uncompressed animation. Lower-poly models and fewer samples load faster.

Reload Mods after rebuilding and start a new match; no client restart is required.
Model assets are currently local-only. Data mods still transfer in private co-op;
rooms reject custom-art mods with a clear message until chunked asset transfer exists.
Models do not add arbitrary simulation behavior: the cloned tower/weapon supplies
its targeting and projectile rules, which you can edit with the content API.

## Editing individual upgrades

```python
tower.upgrade(0, 1, cost=150,
    stats={"range":40},
    weapon={"rate":0.5},
    projectile={"damage":5, "pierce":5, "display":mod.model("bolt")},
    model="robot-upgraded")
```

Paths are currently 0–2 and tiers 1–5. This edits the purchase price and the selected
upgrade's resulting tier states, including later tiers and crosspaths. Values are
absolute replacements; write lower tiers first, then higher tiers to override them.
`stats` edits native tower-tier fields, `weapon` edits the primary weapon, and
`projectile` edits that weapon's projectile. Unknown fields fail validation rather
than being silently ignored. A model swap uses the exported muzzle and release time.
To replace the entire projectile behavior, pass a template rather than a field patch:

```python
bomb = mod.projectile("grenade", base_tower="BombShooter", damage=10)
tower.upgrade(0, 2, projectile=bomb)
```

That copies the complete native projectile configuration, including its existing
behavior models, before applying your overrides. A plain dict retains behaviors
not mentioned in the patch. New behavior *types* still need engine support.

Secondary weapons/other attacks can still be edited through explicit `set` / `copy`
paths or per-tier `tower.weapon` / `tower.projectile` calls.

## What is not implemented yet

This is a growing authoring API, not unrestricted access to every game system.
Extra upgrade paths (fourth paths and beyond) are not supported: the current engine
and upgrade UI store exactly three paths. Existing menu text/visibility can be
patched. Script-created interactive in-match UI is supported; arbitrary new frontend screens remain pending.
Custom-map registration/import is not implemented. Additional native simulation behaviors also
need engine hooks; changing data does not invent an unsupported behavior.

The intended direction is a general mod platform: arbitrary upgrade graphs, complete
per-upgrade behavior/art changes, custom maps with paths/areas/visuals and scripted
interactions, custom UI with actions, and validated co-op dependency/asset sharing.
These capabilities must work through documented code APIs and reload safely, with
no in-game authoring editor or per-frame Python/DLL injection.


## Event scripts, custom UI and shop placement

This is now more than a tower template format. Existing towers have editable handles:

```python
mod = Mod("example", "Example")
mod.existing_tower("DartMonkey").shop(after="SniperMonkey", category="military")
robot = mod.tower("robot", base="DartMonkey", cost=300)
robot.shop(before="DartMonkey")
mod.existing_tower("BananaFarm").shop(visible=False)
mod.script("mechanics", file="mechanics.rhai", tick_interval=60)
```

Shop positioning accepts **one** of `before`, `after`, or zero-based `index`. It changes
both native shop layouts. Categories are primary/military/magic/support. Existing mode
restrictions still apply. `stats(...)` updates a tower definition; `upgrade(...)` updates
its later/crosspath states. `set`/`copy` replace existing native fields. `insert(path,
index, value)` and `remove(path, index)` change arrays, including attacks, weapons,
abilities and round groups. Values must conform to native data types; adding a new
array entry does not create a new native behavior type.

### Actual running code

Rhai is a small embedded language with functions, loops, conditions, arrays and maps.
Python is just the build tool. Scripts compile once when a match loads. No DLL injection
or restarting the client is needed; after rebuilding a mod, Reload Mods and start a new
match. Active matches keep their compiled definitions and script state.

```javascript
fn on_new_game(ctx) { this.reward = 100; [] }
fn on_round_end(ctx) {
    if ctx.sandbox { return []; }
    this.reward += 10;
    [#{op: "cash", amount: this.reward}]
}
```

Define only the callbacks you need. Available hooks: `on_new_game`, `on_round_start`,
`on_round_end`, `on_tower_placed`, `on_tower_sold`, `on_towers_changed`, `on_pop`, and
`on_tick`. Pop callbacks aggregate all pops in one simulation step. Tick interval uses
simulation steps (60 per simulated second), not render frames. No callbacks run while
paused, except explicit UI actions. `this` is a per-script, per-match map; it is not a
save file. Helpers must be functions; top-level script statements are not executed. Call a helper
that uses persistent state as `this.helper(ctx)` so its `this` remains bound.

`ctx` contains `cash`, `lives`, `round` (completed rounds), `tick`, `popped`,
`tower_count`, `bloon_count`, and `sandbox`. It is read-only snapshot data. Return an
array of commands. Supported commands:

- `#{op:"cash", amount:100}` adds cash (negative subtracts).
- `#{op:"lives", value:200}` sets lives; does not revive an ended game.
- `#{op:"spawn", name:"Blue", count:30, spacing:0.2}` queues native spawns in any mode.
- `#{op:"clear_bloons"}` / `#{op:"reset_cooldowns"}`.
- `#{op:"popup", title:"Hello", body:"Your text"}` opens a native modal OK popup.
- `#{op:"ui", widget:#{id:"panel", tree:node, actions:#{"Button":"on_click"}, modal:true}}`
  creates/replaces a custom native UI tree owned by this script.
- `#{op:"remove_ui", id:"panel"}` removes one of this script's widgets.

### Custom UI, not an in-game editor

Widgets use the same `name` / `active` / `rect` / `comps` / `children` data shape as
native exported UI. Images, text, anchoring, masks, layout groups and buttons use the
native UI renderer. Rect coordinates are in a 3840x2160 reference canvas, with Y up;
anchors adapt to the viewport. Bind descendant paths **relative to the widget root**
to your own one-argument script functions. An empty path binds the root itself. Your
callback receives `ctx`, can branch/change `this`, update its UI and issue game commands.
The topmost modal widget owns input. OK popups block the UI and map underneath.

A complete working example is `tools/mod_examples/workshop/`. It adds a launcher,
custom panel, wave button, native popup, mutable round bonus, and shop rearrangement:

```powershell
python -I tools/modsdk.py build tools/mod_examples/workshop/mod.py --out mods/workshop/mod.json
```

Then Mods > Reload Mods > enable Mechanics Workshop > start a new local match.
This example is not enabled automatically.

### Current limits and performance contract

This is the foundation for broader mechanics, **not unlimited engine access yet**.
Runtime UI currently targets in-match overlays; load-time `menu(...)` edits still handle
existing frontend pages. There are no arbitrary new renderer/physics behavior types,
unrestricted new per-entity behavior types, extra upgrade paths, or synchronized runtime script mods yet.
Script mods are explicitly rejected in co-op until their state/input protocol is wired.
Existing data-only co-op mods continue working.

Each callback has a 30,000-operation limit and 16 call levels; persistent state is limited
to 64 KiB. A failed/over-budget script disables itself for the match. Commands validate
as a batch before applying; maximum 32 per callback. Source is limited to 32 KiB within
the existing 64 KiB manifest. There are at most 64 active widgets, 128 nodes per widget,
12 tree levels and 32 action bindings. No filesystem/network/process APIs are exposed.
No scripts means no engine allocation; native attacks and projectiles stay native.
UI trees rebuild only when changed. Prefer round/entity hooks over frequent ticks.


## Entity queries and interception

These interfaces are local-only, like the rest of the script runtime. The native path
is unchanged when no matching hook exists: no entity snapshots or script calls are made.

### Inspect live entities

Return an inspection request from any ordinary callback:

```javascript
fn on_new_game(ctx) {
    [#{op:"inspect", query:#{kind:"towers", offset:0, limit:8, tag:"my-list"}}]
}
fn on_query(ctx) {
    for tower in ctx.event.items {
        // tower.id is a stable entity ID, not an array index.
    }
    []
}
```

Kinds are `towers`, `bloons`, `projectiles`. Responses arrive through the requesting
script's `on_query` at a simulation boundary. `ctx.event` contains `kind`, `tag`,
`offset`, `total`, and `items`. Pages contain 1–128 items; at most eight queries are
serviced per simulation step, with at most 64 pending. Paging uses current storage
order; entities can appear/disappear/reorder between pages. Use IDs to deduplicate,
not page offsets as persistent references.

Snapshots expose:

- Towers: `id`, `tower` (definition ID), `position`, `tiers`, `pops`, `damage`, `target`, `point`.
- Bloons: `id`, `layer`, `bloon` (definition ID), `hp`, `position`, `distance`, `path`.
- Projectiles: `id`, `owner` (tower ID, or 0), `position`, `velocity`, `damage`, `pierce`, `remaining`.

Positions use native world coordinates. Projectile velocity is world units per step;
`remaining` is seconds. Position/remaining snapshots may be null for native infinities.
`on_tower_placed` now includes that tower snapshot in `ctx.event`; `on_tower_sold`
includes its pre-removal snapshot.

### Edit by ID

```javascript
#{op:"entity_patch", patch:#{kind:"bloons", id:b.id, layer:b.layer, fields:#{hp:50}}}
#{op:"entity_patch", patch:#{kind:"projectiles", id:p.id, fields:#{damage:10, pierce:20, remaining:2.0, velocity:[1.0,0.0]}}}
#{op:"entity_patch", patch:#{kind:"towers", id:t.id, fields:#{target:"Strong"}}}
#{op:"damage_bloon", id:b.id, layer:b.layer, amount:5}
```

HP edits set positive current HP directly. Use `damage_bloon` for actual damage, native
child spawning, cash/pops and damage hooks. Damage commands run at the next simulation
boundary. They do not perform a projectile immunity check or credit a tower. `layer` is optional but recommended: the first
child can reuse a bloon's object ID with a new layer ID. Missing entities and mismatched
layers are ignored. State is per match; do not reuse IDs in a later game.

Tower targets support First/Last/Close/Strong/Point when that tower's native model
allows them. `point:[x,y]` updates a tower with point targeting. Unsupported modes are
ignored. Projectile movement controllers can overwrite velocity changes. These are
specific supported fields, not arbitrary writes to engine internals.

### Change or cancel native behavior

Interceptors return a **map**, unlike event callbacks which return command arrays:

```javascript
fn before_damage(ctx) {
    if ctx.event.bloon.bloon=="Red" { return #{cancel:true}; }
    #{damage:ctx.event.damage*2}
}
fn before_fire(ctx) { #{damage:ctx.event.damage*2, count:3} }
fn before_place(ctx) { #{cancel:ctx.event.tower=="BananaFarm"} }
fn before_sell(ctx) { #{cancel:ctx.event.tower=="DartMonkey"} }
fn before_upgrade(ctx) { #{cancel:ctx.event.paragon} }
```

An empty map `#{}` preserves native behavior. `cancel:true` stops processing subsequent
scripts for that action. Otherwise scripts compose in stable manifest/script order;
later scripts see earlier overrides. Interceptors can read `ctx`, update `this`, and
return a decision. They cannot return game commands or mutate the active collision list.
Use ordinary callbacks/queued commands for follow-up actions.

- `before_damage`: event contains `bloon` snapshot and `damage`. Return `damage` (rounded
  to native integer damage) or `cancel`. Runs once for a top-level health-damage call,
  not again for recursive child-layer overflow. Cancelling prevents health loss, not
  other collision effects, pierce consumption or cooldowns. Whole-bloon destruction
  abilities that bypass health damage do not enter this hook.
- `before_fire`: event contains `owner`, `position`, `direction` (radians), `count`,
  `damage`, `speed`, `pierce`, `tiers`. Return any of `damage`, `count`, `speed`,
  `pierce`, `direction`, or `cancel`. Runs for native emissions, including emitted
  sub-projectiles. Cancellation does not refund the weapon's cooldown. Native model
  data is copied only when a returned field requires changing it.
- `before_place`: attempted definition ID (`tower`), `position`, `cost`; cancellation only.
- `before_sell`: tower snapshot; cancellation only. Forced dependency cleanup when a
  carrier/deck parent is sold bypasses child sell vetoes so cleanup cannot loop forever.
- `before_upgrade`: `tower` snapshot, `paragon` flag, plus ordinary `path`/`cost` or
  paragon `extra`; cancellation only. Paths remain zero-based.

Bad decisions disable the offending script and preserve the native action or earlier
valid overrides. Disabled subscriptions are removed from the hot path, along with that script's UI widgets
so a failed modal cannot trap input. Callbacks also
share a 100,000-operation budget per simulation tick across all scripts; the existing
30,000-operation per-call limit still applies. This avoids multiplying the execution
budget by thousands of damage events. Callback output is capped at 256 KiB.

### Working example

```powershell
python -I tools/modsdk.py build tools/mod_examples/entity-lab/mod.py --out mods/entity-lab/mod.json
```

Enable **Entity Lab** and start a new local match. Its panel inspects live towers,
adds HP to a specific bloon, toggles double projectile damage, absorbs five health
hits, and blocks Farm purchases. It demonstrates these hooks without altering any
base data. The example uses ordinary native UI; authors can replace its layout.
