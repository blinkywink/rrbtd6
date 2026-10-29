# RustTD6 mod API 1

[Website guide](https://rusttd6.com/mods/) · [Schema](mod.schema.json) · [SDK](../tools/modsdk.py)

Mods are authored in files, not an in-game editor. Author scripts run only on an explicit build
command. The native client loads generated data, never Python scripts or injected DLLs.

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
are staged before the recoverable release swap; preferences, local profiles and account cache
are preserved. A failed extraction never replaces the working assets. Installed game EXEs
route through the launcher; development builds without a release manifest stay direct.
