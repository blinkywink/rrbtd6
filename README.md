# rrbtd6

A from-scratch Rust reimplementation of Bloons TD 6, aiming to play identically to the
original except for the changes listed in [DEVIATIONS.md](DEVIATIONS.md).

**This repository contains code only.** It ships no Ninja Kiwi art, audio, maps or game data.
Everything the game needs is extracted on your own PC from your own legitimately owned copy
of Bloons TD 6. Nothing extracted from the game is ever committed or redistributed.

Not affiliated with or endorsed by Ninja Kiwi.

## Installing (players)

1. Own Bloons TD 6 on PC and install [MelonLoader 0.7.3](https://github.com/LavaGang/MelonLoader/releases/tag/v0.7.3) into it.
2. Download `RRBTD6Installer.dll` from the [latest release](https://github.com/blinkywink/rrbtd6/releases/latest)
   and put it in BTD6's `Mods` folder.
3. Start BTD6. At the main menu it offers to install rrbtd6: pick a folder.
4. The launcher (`rrbtd6-launcher.exe`, also on your desktop) downloads the game, sets up a private
   Python, extracts the art and data from your BTD6 install (first run: long, ~25 GB) and starts it.
   Start rrbtd6 from the launcher from then on: it updates itself and the game every time, and
   re-extracts after BTD6 updates.

The installer and launcher only contact GitHub, python.org and PyPI. They never contact
Ninja Kiwi, and never change BTD6, your save or your account.

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
