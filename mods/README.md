# RustTD6 mods

Turn on **Settings > Advanced > UI > Show Mods Button**, then open **Mods** on the main menu.
The included Practice Start example is disabled until you enable it. Changes take effect in
your next new local Sandbox, including restarting a Sandbox. Hiding the menu button does not
disable mods; use each mod's toggle to disable it.

Put each mod in its own folder next to this file:

```text
mods/
  practice-start/
    mod.json
```

After adding or editing files, press **Reload Mods**. Restarting a game rereads enabled
manifests too. The enabled ID list is local, in `local_extracted/settings.json` under `mods`.
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

Unknown fields and unsupported API versions are reported instead of silently ignored.
Manifests are limited to 64 KiB and the catalogue to 256 folders. Duplicate IDs are invalid.

Tower, projectile and damage hooks will be designed once their simulation interfaces settle.
Co-op mod negotiation, arbitrary scripting, custom asset packs and progression hooks are not
part of API 1. Mods do not edit an official account or upload saves.
