# Tools

Everything that isn't the game itself. Run Python with `python -I` (isolated mode). These tools only
read the game install and write under `local_extracted/`, which is never committed.

## Building and shipping

| Script | What it does |
|---|---|
| `build_exe.sh` | Builds the one playable Windows build, `BTD6.exe` |
| `build_local_assets.sh` | Extracts the game data (runs `extract_all.py`) |
| `build_macos.sh`, `build_linux.sh`, `build_ios.sh` | Personal builds for other systems (`ios_cc.py` is the iOS linker wrapper) |
| `package_tools.py`, `package_desktop.sh` | What the launcher downloads |
| `publish.sh`, `public_exclude.txt` | Publishes to the public repository, leaving out listed paths |
| `site_social_card.sh` | Regenerates the site's link preview image |
| `checkpoint.sh` | Saves progress with an updated `docs/dev/HANDOFF.md` |

## Setup pipeline

| Path | What it does |
|---|---|
| [`extract_all.py`](extract_all.py) | The setup driver: every step, in order, with resuming and progress |
| `extraction_progress.py` | The progress window's stages and timings |
| [`assets/`](assets/README.md) | The exporters and inspection helpers |
| [`EXTRACTION.md`](EXTRACTION.md) | How the pipeline and asset formats work |
| `requirements.txt`, `bundle_lock.py`, `unitypy_lowmem.py` | Pinned packages and memory-bounded bundle loading |

## Mods

| Path | What it does |
|---|---|
| `modsdk.py`, `mod_model.py`, `blender_mod_model.py` | The Python mod SDK and model import |
| `mod_examples/` | Example mods |

## Accuracy and research

| Path | What it does |
|---|---|
| [`mod/`](mod/README.md) | SimRecorder, a MelonLoader mod that records the real game, and its 1,100 scenarios |
| `rec/` | Replaying recordings and comparing them with the game |
| `il2cpp/` | Decompiling and inspecting the game's native code |
| `dotnet_meta/`, `unityfs/`, `sim_model/`, `data_export/` | Metadata, bundle and data format readers |
| `benchmark_frames.py` | Frame timing runs |

`external/` holds third-party repositories cloned for review (git-ignored). See
[`research/00-tooling.md`](../research/00-tooling.md).

## Tests

`tests/` has the Python tests. Each runs on its own and doesn't need the game:

```sh
python -I tools/tests/test_extraction_plan.py
```

`test_coop_game.py`, `test_coop_transport.py` and `test_launcher_bootstrap.py` take a built executable
as an argument. CI runs the list in [`release.yml`](../.github/workflows/release.yml).
