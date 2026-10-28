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

## Lossless animation storage

The final extraction step now runs `tools/assets/pack_anims.py` after the existing
quantization pass. It streams each anim.bin / anim_q.bin through zlib level 1,
verifies the full decompressed payload against the source hash and size, and
atomically installs a .bin.z file before removing the uncompressed copy. Files
that do not shrink stay unpacked. A changed source aborts migration. A failure
keeps the source intact and does not record the extraction step as complete.
No animation frames, metadata, textures, geometry, or simulation data are changed.
Up to four streaming workers pack separate files concurrently, bounding temporary
RAM instead of loading whole animations during setup. --workers 1 is available
when competing with other work.

The current client accepts both layouts. Fresh unpacked developer exports take
precedence over older packed files. Packing requires the updated client; older
clients do not understand .bin.z. The launcher installs its matching client and
tools before extraction. `--keep-original` is available for developer comparisons.
Grafting tools also accept packed source/target animations. Every normal model
load stages disk reads/decompression in a worker, then yields through texture
uploads; compression adds no work to per-frame sampling. Synchronous demand
callers can still wait for an unprepared asset; background staging does not
guarantee zero load latency.

Runtime validation checks decoded length against clip/vertex metadata, checksum,
stream completion and trailing data. Input is streamed through a 1 MiB buffer
instead of retaining the entire compressed file beside the decoded animation.
Synthetic coverage: tools/test_animation_storage.py and the renderer's
animation_storage tests. All game data remains local and ignored by Git.

Local migration measurement (2026-10-27): 1,843 animation sources; 1,841 packed
and two kept unpacked because compression would increase size. Animations went
from 17.640 GB to 8.091 GB (54.13% smaller), and the whole models folder from
19.488 GB to 9.939 GB (49.00% smaller), reclaiming 9.549 GB. This excludes audio,
UI, maps, staging caches and development benchmarks; it is not total install size.
Every packed source was verified before removal; no original animation precision
was reduced by this migration. The launcher keeps using its prior quantization
pass for new exports, followed by this lossless pass.

The real Python-packed 60.594 MB Pat Fusty fixture occupies 20.034 MB and decodes
with the Rust streaming reader in 98.58 ms average across ten exact comparisons
(on this PC, warm input). This measures load-time CPU work, not an FPS gain.
Runtime checks: 123 Rust tests passed / 15 ignored, 29 offline tooling checks
passed (including six new storage checks), and four exact GPU comparisons for
background-prefetched versus directly loaded MOAB/BAD models. Gameplay and the
home menu were smoke-tested with the migrated local assets. BTD6.exe was rebuilt.

## Predictive storage and installation-time runtime conversion

Finish step v5 uses the matching client in headless --canonicalize-anims mode
before compaction/packing. Legacy float animations are stored using precisely
the runtime's existing f32 quantizer (same arithmetic, bounds and rounding), not
a new quality level. The old runtime already discarded this float precision on
every load. The exported source floats are removed after the new representation
and metadata are verified/installed; the original BTD6 bundles are untouched.
A developer running build_local_assets.sh must rebuild the matching client first.

After ordinary zlib packing, delta_anims.py predicts each byte from that byte in
the preceding frame, with wrapping arithmetic, then zlib-compresses it. The .dz
header contains RRDIF001 and a little-endian u64 frame-byte stride. Each converted
payload is reconstructed and checked against the complete original hash/length
before replacement. Files which do not shrink retain .z. This prediction pass
is byte-lossless, including original float bits when used without canonicalizing.
The runtime accepts .bin, .bin.z and .bin.dz in that priority order, reconstructing
prediction once in place. No extra full-animation decode buffer is needed.
Grafting tools read all layouts. An older client cannot read .dz.

Byte-identical whole animation frames may also share RAM if the map plus unique
frames saves at least 10 percent. Clip timing/counts stay unchanged and two frame
bases are resolved once per pose; the vertex interpolation loop is unchanged.
Changing frames retain the direct layout. Legacy float decoding also avoids the
previous temporary Vec<f32>, which duplicated the entire source float buffer.

Component measurements on this PC: Pat Fusty Level20, 20.034 -> 10.568 MB on disk;
Rust warm decode 97.17 -> 80.69 ms (ten exact comparisons). Village animation RAM
80,910 -> 9,062 bytes, with all frames/vertices compared at three interpolation
weights. Mermonkey float representation 22,351,680 -> 9,313,200 bytes decoded;
conversion CPU 24.38 -> 3.63 ms, with every quantized position/normal exact. These
are asset-specific measurements, not whole-game RAM/FPS or clean-install claims.

Full local follow-up (2026-10-27): the prediction migration processed 1,841
packed animations, saving 4.435 GB. Native canonicalization converted all 93
remaining float models from 5.731 GB of decoded floats to 2.388 GB of the exact
existing runtime representation. After repacking, models occupy 5,079,651,863
bytes versus the initial 19,487,978,396: 14.408 GB reclaimed, 73.93% smaller.
This follow-up alone saved 4.860 GB versus the previous 9.939 GB models folder.
1,696 animation files use prediction, 145 retain ordinary zlib, and two remain
unpacked; no pending files remain. Old raw-float bytes are not retained after
canonicalization, while their prior runtime positions/normals are retained.

The installed official game measured 3,362,276,112 bytes. Our audited runtime
asset folders still total 8,447,731,412 bytes, excluding Python/tools/executables,
private caches and development outputs. This is not yet smaller than BTD6.
Audio remains 1,989,858,500 bytes of WAV data. Native bone/curve animation storage
and audio format work remain the major opportunities; do not advertise victory
on total install size or universal RAM/FPS improvements from these measurements.

Validation: 130 Rust tests passed / 17 ignored, plus the real-asset prediction,
Village RAM and float-representation comparisons described above; 36 offline
Python tooling checks passed. The rebuilt client passed four exact MOAB/BAD
GPU comparisons and migrated gameplay (including converted Mermonkey) and home
menu smoke captures. No account/cloud writes or simulation changes were made.


Lossless audio storage (2026-10-27, audio step v2)
------------------------------------------------
AudioClip exports now write verified PCM16 FLAC directly, with no intermediate
WAV file on disk. Existing installs migrate through compress_audio.py. A WAV is
removed only after matching rate/channels/frame count and a hash of every decoded
16-bit sample; failure retains the source. Temporary output is flushed and
atomically replaced. Original installed-game files are never modified.

All 2,806 local clips migrated: 1,989,858,500 -> 844,680,867 bytes, saving
1,145,177,633 bytes (57.55%); zero runtime WAVs remain. Conversion and complete
sample verification took 13.8 seconds on this PC (four workers, warm files).
Direct exports avoid writing then rereading all WAVs. New soundfile dependency
is pinned in tools/requirements.txt. Older executables cannot read FLAC; install
the matching executable before migrating. Fresh WAV exports remain readable.

The same quad-snd mixer, mono duplication and resampler are retained. Music
file reading, FLAC decoding and shared-buffer allocation run on CPU workers;
queueing prepared shared PCM does not copy the track on the mixer thread. At
most two background jobs and one prepared music lookahead are retained. Looping
reuses the current buffer. Cold unprepared effects can still wait to preserve
event timing; this is not a claim that all loading stutters have been eliminated.
Decoded playback buffers remain PCM; disk savings do not imply equal RAM savings,
and near track changes the next decoded track temporarily occupies extra RAM.

Mixer equivalence compared every f32 bit for mono/stereo and 44.1/48/22.05 kHz
synthetic pairs plus the largest game track (27,256,386 output samples).
Largest track: 59.334 -> 28.880 MB; a single component decode measured 286.17 ms WAV
versus 543.46 ms FLAC. The slower codec is why music decoding runs off the render
thread. This is not an FPS improvement measurement.

Latest audited runtime asset folders total 7,098,872,522 bytes; the official
install previously measured 3,362,276,112 bytes. We are still larger, mostly due
to baked vertex animations. These totals exclude developer/tool/private caches.
Validation: 131 Rust game tests passed / 18 ignored, explicit mixer-equivalence
and prepared-buffer mixer tests passed, and audio-enabled gameplay/home captures
passed. Gameplay loaded five effect clips with zero missing and music playing.

To reproduce mixer checks without game files, run tools/test_audio_storage.py
--fixtures target/audio-fixtures with the extraction Python, then set
RRBTD6_AUDIO_FIXTURES=target/audio-fixtures and run cargo test --profile game
-p btd6-game lossless_mixer_equivalence -- --ignored --nocapture.

Audio follow-up tooling validation: 42 offline Python checks passed; the release
ZIP includes the compressor and excludes private account tools and scenarios.
Synthetic FLAC/mixer equivalence is also wired into the release workflow.
