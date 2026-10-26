# rrbtd6

A from-scratch Rust reimplementation of Bloons TD 6, aiming to play identically to the
original except for the changes listed in [DEVIATIONS.md](DEVIATIONS.md).

**This repository contains code only.** It ships no Ninja Kiwi art, audio, maps or game data.
Everything the game needs is extracted on your own PC from your own legitimately owned copy
of Bloons TD 6. Nothing extracted from the game is ever committed or redistributed.

Not affiliated with or endorsed by Ninja Kiwi.

## Layout

- `engine/` – simulation
- `renderer/` – the playable game (macroquad)
- `coop/` – peer-to-peer co-op
- `tools/` – asset extraction from a local BTD6 install, research helpers
- `research/` – notes on the original game's formats and behaviour

## Building (developers)

1. Install Bloons TD 6 and MelonLoader, and run the game once.
2. Extract the game data from your install: `BTD6_DIR="<BTD6 folder>" sh tools/build_local_assets.sh`
3. Build and run: `sh tools/build_exe.sh`, then `BTD6.exe`.

A one-click installer (a BTD6 mod that installs and auto-updates the game) is in progress.
