# Contributing

Thanks for helping out. A few rules keep the project accurate and legally clean.

## The two hard rules

1. **Never commit anything from the game.** That means art, audio, maps, game data, dumps and
   recordings. `local_extracted/` and `research/dumps/` are git-ignored for this reason. Check
   `git status` before you commit.
2. **Match the real game.** A behaviour should come from the original game's logic or a recording of
   it, not a guess. Anything that differs on purpose goes in [docs/DEVIATIONS.md](docs/DEVIATIONS.md).
   If something isn't exact yet, say so in a comment (the code marks these `STAND-IN`).

## Setting up

Follow [Building](README.md#building-developers). Extraction needs your own copy of BTD6.

## Before opening a pull request

Run what your change touches:

```sh
cargo check --workspace
cargo test -p btd6-game
python -I tools/tests/test_extraction_plan.py
```

- A change to a setup step needs its version bumped in `STEP_VERSIONS` (`tools/extract_all.py`), so
  existing installs pick it up.
- A new export has to run from `tools/extract_all.py`, or a fresh install won't get it.
- A change to the mod API needs [mods/README.md](mods/README.md) and the website docs updated
  (`python website/build_docs.py`).
- If it can only be checked in-game, add it to [docs/dev/TO_TEST.md](docs/dev/TO_TEST.md).

## Style

Match the code around you. Keep comments for what the code can't say, such as which method of the
original game a behaviour comes from. Keep each commit to one change, with a message that says what
changed for the player or developer.

## Reporting bugs

Use the [bug report form](https://github.com/blinkywink/rrbtd6/issues/new/choose). The map, tower,
round and a screenshot help most. Please don't attach game files.
