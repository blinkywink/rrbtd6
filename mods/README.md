# RustTD6 mods

Turn on **Settings > Advanced > UI > Show Mods Button**, then open **Mods** on the main menu.
The included Practice Start example is disabled until you enable it. Changes take effect in
your next new local Sandbox, including restarting a Sandbox. Hiding the menu button does not
disable mods; select a mod in the manager and press **Disable Mod** to disable it.

Put each mod in its own folder next to this file:

```text
mods/
  practice-start/
    mod.json
```

The manager lists installed mods on the left and their description and Sandbox effects on the
right. Selecting a row does not enable it; press **Enable Mod** or **Disable Mod** below.
**Open Mods Folder** opens the local installation directory.
Mods may also expose **Mod Settings** in the details pane. Use **+** / **-** to adjust
values, or **Reset to Defaults** to restore the author's values. Changes save locally
on each click, including while the mod is disabled; enabling it is a separate action.
The effects preview always uses the same values as a new local Sandbox.

After adding or editing files, press **Reload Mods** (or reopen the manager). Restarting a
game rereads enabled manifests too. The enabled ID list is local, in `local_extracted/settings.json` under `mods`.
Removing a folder does not silently clear its enabled ID; the menu shows a missing entry you
can switch off. If an enabled manifest is invalid or two mods set the same value, no mod
rules apply and the menu explains the conflict. Disable one or fix its manifest.

## API 1

This first API is deliberately small and data-only. It does not load native libraries or
scripts. There are no per-frame callbacks. Files are read only when opening/reloading the
manager or creating a modded local Sandbox. Normal games, events and co-op ignore these rules.

See [practice-start/mod.json](practice-start/mod.json) for a complete example and
[mod.schema.json](mod.schema.json) for editor validation.

- `api_version`: `1`.
- `id`: unique, 1–64 lowercase ASCII letters, digits, `.`, `_`, `-`; first character must
  be a letter or digit. IDs do not depend on folder names.
- `name`: nonempty, at most 80 UTF-8 bytes.
- `version`: nonempty, at most 32 UTF-8 bytes.
- `description`: optional, at most 1024 UTF-8 bytes. Text fields contain no control characters.
- `on_new_sandbox`: optional hook with `starting_cash` (whole number 0–1,000,000,000,000) and
  `starting_lives` (whole number 1–1,000,000,000). Omit either to keep the game's original value.

Optional `settings` exposes bounded controls for hooks already present in `on_new_sandbox`:

```json
"on_new_sandbox": { "starting_cash": 100000 },
"settings": {
  "starting_cash": { "min": 0, "max": 1000000, "step": 10000 }
}
```

Only `starting_cash` and `starting_lives` are configurable. A corresponding hook value
is required and serves as the default. Bounds and step must be whole numbers within
the hook's existing limits, `min <= max`, and `step >= 1`. Both bounds and the default
must align with the step, measured from `min`. Old manifests without `settings` keep
working unchanged. User overrides live under `mod_settings` by mod ID in the local
settings file, separate from enabled IDs. Disabling or hiding the mod keeps its values.
If an author changes bounds or removes a control, incompatible saved settings are
reported; reset the mod to use its new defaults. Invalid settings in disabled mods do
not block other mods. Reset remains available even after all controls are removed.

Unknown fields and unsupported API versions are reported instead of silently ignored.
Manifests are limited to 64 KiB and the catalogue to 256 folders. Duplicate IDs are invalid.

Tower, projectile and damage hooks will be designed once their simulation interfaces settle.
Co-op mod negotiation, arbitrary scripting, custom asset packs and progression hooks are not
part of API 1. Mods do not edit an official account or upload saves.
