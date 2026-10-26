# SimRecorder

A MelonLoader mod that plays one scripted scenario in the real BTD6 and records the simulation
every frame, so the clone can be checked against it.

- Build and install: `sh tools/mod/build.sh` (Visual Studio's `csc` against the .NET 6 runtime,
  MelonLoader and the game's interop assemblies; no .NET SDK needed). Writes
  `<game>/Mods/SimRecorder.dll`.
- Scenario: `<game>/UserData/SimRecorder/scenario.json`, for example
  `{"name": "dart_r1", "map": "Tutorial", "difficulty": "Hard", "mode": "Clicks", "round": 1,
  "frames": 3600, "towers": [{"id": "DartMonkey", "x": -112.8, "y": -30.9}], "quit": true}`.
  Coordinates are the game's: the clone's y is the game's -y. CHIMPS ("Clicks") turns off
  Monkey Knowledge, which the clone doesn't have.
- Run: launch the game through Epic. The mod presses Play and Continue on the start-up screens,
  opens the mode screen for the map, picks the mode, places the towers, starts the round and
  quits when done. Output: `<name>.rec` (per frame: `F time roundTime round cash`, `B` bloons,
  `W` weapons, `P` projectiles) and `<map>.map` (world bounds and the simulation's paths).
- Compare: copy the `.rec` to `research/dumps/recordings/` and run
  `CARGO_TARGET_DIR=target/game cargo test --profile game -p btd6-game compare_recording -- --ignored --nocapture`
  (`RRBTD6_REC` picks another file).

Each run starts a new game on the scenario's map, which replaces a saved game on that map.
