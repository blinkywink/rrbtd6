# Clean installations

`python -I tools/extract_all.py --game "<BTD6 folder>" --root "<install folder>"`
runs the same extraction used by the launcher. The install folder must contain
the shipped tools and the interpreter must have `requirements.txt` installed.
MelonLoader's generated interop assembly is needed; start official BTD6 once first.
No Ninja Kiwi service is contacted by this driver.

The recipe includes the map scene data, geometry and shaders, sprites and fonts,
tower tiers, every tower/hero's supplemental displays and paragons, hero HUD
portraits, core menus and sandbox controls, frontend scenes and artwork, native
particle effects, audio and home-world animations/interactions. Extended tower
and effect exports derive their lists from the decoded game rather than a fixed
selection of towers.

Each finished step has an output version in `local_extracted/.extract_state.json`.
An interrupted step is retried; already complete tier bakes are reused. A BTD6
binary/metadata change invalidates the steps. Raise `STEP_VERSIONS[name]` when an
exporter change requires its output to be regenerated; unrelated code updates do
not force a full extraction. `--only <step> ...` runs selected steps explicitly.
Checkpoint and interpreter-hook replacements are atomic. Interrupted writes leave
the prior checkpoint intact; malformed legacy checkpoints restart extraction
instead of permanently preventing startup.
Nested tower exporter failures propagate, preventing an incomplete bake from
being recorded as a successfully completed step.

The extractor's site hook activates only with its tools/cache environment. It
uses Python's native Windows-info fallback to avoid a stalled WMI service, makes
bundled asset helpers available under isolated/embedded Python, and streams large
UnityFS bundles through a disk cache. It does not change Windows services or add
the current working directory to the import path.

Outputs, shader disassembly and bundle caches are proprietary and stay local.
`package_tools.py` creates a genuine ZIP using Git-tracked tooling only, excluding
the private account tools and recorder scenarios. Never include extracted outputs
in a release archive.

The bootstrap, package, sibling-import and extraction-plan tests use synthetic
fixtures and no game files or network. A full fresh extraction and an actual
startup still need to be tested separately; passing these tests alone does not
establish that a new BTD6 build is fully supported.

Local verification (2026-10-26): downloaded GitHub build 7 launched its main menu
using assets extracted into an initially empty test installation, including home
water and character models; hero selection and an in-game scene also rendered.
The full 24-step extraction completed, including social/startup menus, particle
effects and shader rebuilding. All indexed tower model files were present, and
all 239 translated material entries across 90 map models had shader references.
The installed game also rendered the splash using this test installation's new
startup assets. These checks do not establish complete gameplay/visual parity.

Three subsequent extraction checks with embedded Python took 58–64 ms and ran no
exporters. The separate tier-index repair reuses existing animation bakes rather
than invalidating/re-exporting the base models and projectiles. The embedded map
export produced byte-identical map JSON and terrain PNG to the clean-install
export. Bootstrap/recovery verification is documented in `launcher/README.md`.

## Disk use and setup work

Successful full extraction now removes only `local_extracted/_bundle_cache`,
the regenerable decompressed UnityFS staging data. Interrupted runs and `--only`
runs retain it for resumption. Use `--keep-bundle-cache` for repeated exporter
development. Cleanup validates the resolved output path and refuses symbolic
links/junctions before deleting anything; runtime assets and settings are kept.
An unsuccessful cleanup is retried on the next normal full check.

The sprite step exports each atlas once instead of first exporting four gameplay
atlases and then overwriting those outputs in the all-atlas pass. The final step
also applies the existing animation compactor to models exported by the later
startup/home steps. The earlier compaction remains to limit peak setup space.
No clips or animation frames are removed and no new texture downscaling is added.
The new `menu_additions` step brings the daily chest and jukebox assets to existing
installs without invalidating their map/tower bakes.

Local audit (2026-10-27): the working cache contained 5,678,944,899 bytes (5.68 GB),
which would be reclaimable by successful cleanup; the live development cache was
left intact. Replaying the removed atlas pass into a separate empty output took
4.04 s using the installed Python and an existing bundle cache; all 509 resulting
files matched the development exports byte-for-byte. A copy of one late
home model compacted from 6,025,344 to 2,510,560 bytes in 0.037 s, preserving its
clip metadata and mesh parts. These are component checks, not a complete new
clean-install benchmark or a guaranteed saving on every game version.

Follow-up storage audit (2026-10-27): models occupy 19.488 GB in the live
working directory: 5.731 GB of raw anim.bin, 11.909 GB of anim_q.bin, and
about 1.85 GB of other model data. Applying the existing 24-to-10-byte
compactor to remaining raw frames would put that models directory near
16.145 GB, excluding transient conversion memory/disk needs. Audio is 1.990 GB.
This is a mixed development directory, not a measured clean installation.
The website and player README no longer advertise a 35 GB requirement.
Baked per-vertex animation and decoded WAV storage explain much of the gap
from the official game's compressed install; cleanup alone will not close it.
