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
