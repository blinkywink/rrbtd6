# Intentional deviations from the original game

Everything not listed here should match the original BTD6 exactly. These are the only
places the clone is meant to differ, by the owner's request.

## 1. No purchases: support-Ninja-Kiwi popup

- Opening the in-game store (any store/shop purchase screen) shows the store as it looks in
  the original, but blurred and with nothing clickable behind it.
- Over it, a small popup reads exactly:

  > Please support ninja kiwi through their official client. They also have an official
  > webstore where you can purchase items.

- Buttons:
  - **take me to the webstore** — opens https://btd6store.ninjakiwi.com/ in the browser.
  - **return home** — closes the store and returns to the main menu.
- Anywhere else that lets you buy something with real money (e.g. buying Powers, Monkey
  Money packs, Trophy Store real-money items, Insta Monkey packs) shows the same popup when
  the purchase button is clicked, instead of buying.
- The "+" (buy Monkey Money) button beside the Monkey Money counter is removed: the counter
  shows alone (front end, defeat screen).
- The Upgrades screen's "+" (buy Monkey XP) button is hidden, as the game hides it when the
  store has no product. Monkey XP is only a record of each tower's use: everything is unlocked.

## 2. FPS counter in the window title bar

- The "show FPS" setting shows the frame rate in the window's title bar (e.g.
  `RustTD6 - 144 FPS`), not drawn inside the game.

## 3. Advanced settings

- Advanced settings has three collapsible sections, closed whenever the screen opens:
  **Default** (original settings), **Graphics** (client rendering options), and **UI**
  (FPS display, menu visibility and startup options).
- The menu UI should be a bit nicer than a plain list (section headers, consistent spacing).
- The screen's title reads **Advanced Options** (the original says "Accessibility and Advanced Options").
- **Hide Boss Button** (off by default) hides the boss event button on the right of the main
  menu. The Legends, Shop, Odyssey and Mailbox buttons no longer exist (see 10), so they have
  no toggles; the Trophy Store sits in the Shop's slot.
- **Hide Double Cash / Fast Track Button** switches the respective client preference
  off before hiding its control; unhiding does not switch it back on.
- The Bloons TV and its hit volume are removed from the home island at the user's request.
- Imported official saves show their native markers and a confirmation before a new local game.
  They cannot be resumed or overwritten yet; the imported snapshot is retained.
- **Skip Startup Screen** (off by default) — go straight to the main menu instead of the Ninja
  Kiwi splash, loading screen and title screen.
- Planned settings (added once the screens they skip exist):
  - **Always Use Cloud Save** — skip the "use your local save or your cloud save?" question
    after pressing Play on the startup screen, and always load the newest cloud save.
  - **Mods Button** — implemented as Show Mods Button in UI settings (see 7).

## 4. No Legends DLC modes

- The two Legends DLC modes (Rogue Legends, Frontier Legends) are not included, and none of
  their assets are extracted.
- The main menu has no Legends button (see 10), and nothing else in the client opens the
  Legends page.

## 5. Account login (planned)

- Logging in to and out of a Ninja Kiwi account from the game's own UI, and the startup
  screen, as in the original. Not built yet.

## 6. Co-op codes and servers (planned)

- Co-op games hosted by this client use peer-to-peer (iroh) connections, with join codes that
  look like the official BTD6 codes.
- Entering a code first looks for an iroh game hosted by this client, then the official BTD6
  co-op servers, so it can play with both official and RustTD6 clients.
- Not built yet: connecting to the official co-op servers needs Ninja Kiwi's network protocol.

## 7. Mods

- Show Mods Button in Advanced > UI is off by default. The manager installs/selects/toggles
  file-authored mods and optional settings; it is not an in-game authoring editor.
- API 1 supports cloned towers/bloons/modes, weapon/projectile/stat/round data edits,
  changes to all existing modes, and load-time text/visibility edits on frontend nodes.
- Authors can use tools/modsdk.py (Python SDK) or emit the manifest from another language.
  No author scripts or injected native DLLs run in the game. See mods/README.md and /mods/
  on the website. Existing Practice Start is only an optional Sandbox example.
- Reload applies to new/restarted games without restarting the client. Already running
  entities retain their definitions. Unsupported arbitrary behavior hooks, new scripted
  widgets/profile counters and custom art imports are future work.
- Private iroh rooms have a room-specific host-selected mod list. Data manifests transfer
  P2P, are hash/size checked, stay in the room snapshot rather than installed folders, and
  are compiled on every client. Guests inspect but cannot change the list. Changes reset
  readiness. Room-provided custom modes appear in the host's mode dropdown.
- Client executable/base game identity must match. The load barrier also verifies shared
  initial state. Official-server interoperability remains unimplemented.
- Local content mods bypass official events; co-op uses room mods rather than individual
  local selections. The original Sandbox cash/lives hook remains Sandbox-only.

## 8. RustTD6 branding

- The window title is **RustTD6**. The window and taskbar use the game's own icon, read from the
  player's BloonsTD6.exe during setup; the executable itself carries no icon (nothing of the
  game's art is shipped).
- The title screen shows the original BTD6 logo, from the player's own game files.

## 9. User requirements: mod platform, co-op and updates (2026-10-28)

These are requested behavior, not a claim that everything below is implemented.

- Sandbox: click cash or hearts/lives to enter testing values. Zero lives must never open
  defeat; towers stop attacking and editing lives back above zero allows testing to resume.
- Mods are a whole-client feature, not Sandbox-only: new modes, changes to one/all existing
  modes, tower and projectile changes, custom towers/projectiles/bloons, UI/menu changes and
  statistics extensions. Authoring should be simpler than BTD6 DLL injection.
- Apply/reload supported mods without restarting the client. Compile definitions into existing
  runtime structures, with no injected DLLs or mandatory per-frame scripting overhead.
  Do not pretend data definitions implement arbitrary new simulation behavior.
- Add a website mod documentation page explaining installation, manifest/API, examples,
  reload lifecycle, game integration, supported hooks and limitations.
- Private iroh co-op: the host chooses the room's mods before the room opens (see 14).
  Clients missing a selected version receive its data over P2P.
  Validate bounded content and hashes, isolate it from local installed mods, never auto-run
  native code, and require identical client/assets/mod/settings identities before starting.
  Changing room mods invalidates readiness. Official-server rooms must not use this path.
- Continue fleshing out playable co-op and the in-BTD6 setup/extractor DLL workflow.
- On launch, check GitHub for a newer client build and install compatible updates.
  If an update requires newly extracted assets, offer to keep playing the installed version
  or rerun the setup DLL inside BTD6 to extract the required data. Preserve the older working
  build/assets until the new extraction and update are complete; support offline fallback.
- Co-op is only between identical client versions. Make incompatible builds/assets clear
  in the room UI rather than allowing a desynchronized game.

- Authoring clarification: mods are authored as code/data files against a documented API.
  Do not build a cluttered in-game mod/tower editor. The in-game UI only manages installed
  mods, optional author-defined settings, and host-selected co-op mods.

## 10. Removed online/social modes (2026-10-28)

The owner wants the core BTD6 experience without these online/social modes. They are removed
entirely: no buttons, screens, settings, network fetches or game code.

- **Races**: the Home and Events race buttons, the race detail/rules screens, race practice
  (concurrent sent rounds, race timer HUD, Send Round button, practice results file) and the
  Send Next Round hotkey.
- **Boss Rush** and **Contested Territory**: their Home and Events buttons. Their original
  in-game elements (Boss Rush lives/tickets, CT tile ID) stay hidden.
- **Social Seasons**: the season overview and goals screens, Monkey Teams (team browser, My
  Team, Team Island) and Social Season Perks (Perks screen and Perks buttons). The co-op hub
  (Play Social screen) keeps Quick Match / Create Match / Join Match only.
- **Friends**: the friends page, Friends buttons, friends' map records on map select and
  difficulty select, and the friend code / friend request settings.
- Their Advanced settings toggles (Hide Race / Boss Rush / Contested Territory / Social Season
  Button, Hide Perks Button, Hide Friends' Map Records) are gone.
- The live-events fetcher no longer downloads the race, Contested Territory or Boss Rush feeds
  and event data; the account import no longer keeps race or friend settings.
- **Odyssey** and **daily challenges**: their Home buttons, the Events screen (and the events
  monkey on the island that opened it), the daily challenge pages, rules and challenge limits,
  and their live feeds. Boss events stay reachable from the boss button on the main menu.
- **Home screen**: the Legends, Shop and Mailbox buttons, and on the island the daily chest,
  Patch (quests), the tower gift box and the events and co-op event monkeys, along with the Daily Reward,
  Quests and Mailbox pages they opened. Their Advanced toggles (Hide Legends / Shop / Odyssey
  Button) are gone; Hide Boss Button and Show Mods Button remain.
- **Twitch**: the pause menu's Twitch button. Hotkeys and Advanced move up a slot in its
  button column, and the side panel loses that slot (a button and its spacing) from its bottom.
- **Kept**: boss events (Home/Events boss buttons and rules), boss bloons, custom boss matches,
  and co-op (including co-op boss matches).

## 11. Local co-op and mod playground (2026-10-28)

This direction supersedes earlier login, cloud-sync and progression-unlock plans.
- No sign-in, linking codes, cloud account import or upload in the client.
- No mandatory tutorial, tower gift unlocks, tower/upgrade/hero/map progression gates.
  All gameplay content is available immediately; mode restrictions and in-match upgrade costs still apply.
- Focus: co-op, authored mods, Sandbox and late-game experiments. Normal map runs remain available.
- A local profile stores name, cosmetic level, stats and separate solo/co-op map medals.
  Level uses 100 XP per completed non-Sandbox round and 1,000 XP per level; it grants nothing.
- Settings/Profile supports renaming and confirmed reset of local progression. Reset keeps
  the name, settings, installed mods and extracted assets.
- The setup DLL may seed a fresh local profile with the loaded game's display name only.
  It must not copy credentials, sessions or official account progression.
- Standard unmodified map wins award medals; challenge/custom/modded sessions do not.
  Black CHIMPS requires verified no-retry tracking and is not awarded by this prototype.

## 13. General-purpose mod platform direction (2026-10-28)

- Mods should eventually author custom maps, interactive menus/UI and whole gameplay
  rules, not just Sandbox settings or numerical tweaks. No in-game authoring editor.
- Replace the native three-path storage/UI assumption with extensible upgrade graphs
  before advertising fourth paths. Support per-upgrade prices, names, descriptions,
  icons, stats, attacks/behaviors, projectile changes and model/animation changes.
- Custom maps need registration plus path routing, placement areas, visuals, interactions
  and mode compatibility. Custom UI needs new widgets/screens plus functional actions.
- New behavior hooks must preserve fast native execution and deterministic co-op.
- Co-op rooms send mods' data, event scripts and model art (assets/, by hash, fetched in
  chunks on Download); custom maps are future work.
- Current implementation: upgrade_patch edits cost/tier stats/primary weapon/projectile
  and per-tier model/muzzle across later and crosspath states. Three paths remain the
  current limit. Arbitrary custom UI and custom maps are still pending.

## 14. Simplified co-op entry (2026-10-29)

- Hide the retired Social Seasons progress badge and Social notification pip on Home.
- Social contains Create Game, Create Match (Mods) and Join Game, using the native co-op
  cards and opening/button animations. Remove Quick Match, team/event, creator and accolade
  sections from this entry screen. Create opens map setup; Join opens room-code entry.
- Create Match (Mods) first opens the Mods page to pick the room's mods (Continue goes on to
  map setup). The match lobby has no mods menu.
- A player joining a room that uses mods gets a popup listing them: Download keeps a copy of
  each one they don't have in their mods folder (left disabled) and joins; Leave leaves.

## 15. Extensible mod runtime (2026-10-29)

- Add optional embedded Rhai callbacks and persistent per-match state, native UI trees,
  native modal popups and validated game commands. Python remains a build tool only.
- Allow new/existing towers to choose shop position/category/visibility and manipulate
  native attack/weapon/ability arrays at load time. No in-game mod authoring editor.
- Unmodified matches allocate no script engine. Event subscriptions and bounded execution
  keep ordinary simulation native. In co-op scripts run in the lockstep simulation on every
  player (no wall clock); widget buttons are co-op inputs, never unsynchronized callbacks.
- Fourth paths, arbitrary renderer/physics behaviors and per-entity script access remain
  future work. Do not advertise unlimited capability until those interfaces exist.

## 16. Enter confirms sandbox Set Cash / Set Health (2026-10-29)

- In the sandbox Set Cash / Set Health popup, pressing Enter (or keypad Enter) does the same
  as OK. The original's popup only confirms through the OK button.

## 16. Entity and interception API (2026-10-29)

- Provide paged entity queries, stable ID/layer-aware edits and queued native bloon damage.
- before_damage/fire/place/sell/upgrade may replace supported values or cancel an action;
  interception returns decisions rather than arbitrary collision-time commands.
- Preserve native hot paths without subscribers, exclude recursive child damage from a
  second interception, and bypass vetoes during forced dependent-sale cleanup.
- Runtime scripts remain local-only. Add shared per-tick operation limits and remove failed
  subscriptions from hot paths. Ship a code-authored Entity Lab example, not an editor.

Mod runtime subscriptions: ordinary callbacks may return owner-scoped `listen` commands; interception decisions may `unsubscribe` their current hook after applying the final decision. State and UI survive manual unsubscribe; failure still removes owner UI. Entity Lab mutes inactive features and closed-panel ticks.

### Mod terminal and co-op entry (2026-10-29)

The custom mod browser uses a clean Benjamin-inspired terminal palette, native BTD6
artwork and fonts. Each installed-mod row has an instant enable switch; selecting
its row separately opens details/settings. Host/guest permissions are unchanged.
The co-op entry sits at the lower right, opposite the shared lower-left hero selector,
with the same mod icon as the home screen. No per-frame file scanning or image decode.

### Mods screen refinement (2026-10-29)

- Header is Mods with the existing clipboard/pencil icon. Switch position/color is the
  sole normal enabled-state indicator; there are no ON/OFF captions or repeated statuses.
- Green utility/settings buttons match the dark panels. The regular footer is removed;
  validation errors retain a dedicated notification. Both content panes have more room.
- Optional local icon.png artwork beside mod.json is bounded to 512px and 1 MiB, confined
  to the mod folder, decoded lazily and cached, with native fallback for invalid images.
  Reload Mods refreshes artwork. Icons stay local; co-op sends only assets/ model files.
- A faint header signal moves along a thin rail using two existing native panel sprites;
  no full-screen shader pass, particle system, or per-frame asset I/O is introduced.

### Mods menu follows the original game (2026-10-29, supersedes terminal styling)

Use the game's native blue panels, inset fields, blue utility buttons, gold selected
rows and native toggle artwork. Keep the Mods title and clipboard icon, custom mod
icons, simple descriptions, and separate row selection/switches. Remove terminal
colors and effects. Co-op's Mods entry uses the same native blue button artwork.

Numeric mod settings accept direct keyboard entry: click the value, type, Enter to
save or click another control; Esc cancels. Ctrl+A and paste are supported. Existing
minus/plus buttons remain for quick adjustments. Author bounds and step alignment are
validated before persisting; invalid input keeps the previous value and shows an error.

### Mods visual consistency follow-up (2026-10-29)

The reference is the original Extras/Advanced settings UI, rather than a mix of
unrelated BTD6 sprites. Mod list items use rectangular blue panel cards, not oversized
pill-shaped action buttons. Selection uses a narrow highlight with brighter card tint.
Toggles use the original 250x85 track and 150x150 BlueBtnCircleSmall knob, with the
same +/-80 positions and green-on/brown-off track colors as advanced_toggle().
Numeric fields and pane backgrounds share the rounded blue inset panel family.
The title has an inline mod icon and no oversized separate backplate. Existing direct
entry, validation, custom artwork, scrolling and host/guest permissions are retained.

### Mods header and list summaries (2026-10-29)

Keep the title/icon and utility buttons aligned inside the main panel, with clearance
above the Installed/Details headings. Mod cards show a smaller, two-line preview of
manifest description beneath the title; long previews end at a word boundary with an
ellipsis, and the details pane retains the full description. Empty descriptions do not
leave a blank subtitle row. Preview strings are prepared on rebuild, not every frame.

### Mod settings control polish (2026-10-29)

Reset to Defaults is a compact secondary button beside the version in the selected
mod header. It discards an uncommitted invalid edit before restoring the defaults.
Numeric fields use a complete, thin blue frame and darker rounded fill, with a gold
focus frame. They are text fields rather than press-scaling buttons. Native square
blue +/- buttons replace compressed long-button sprites and align with field height.

### Mods selection, search and status filters (2026-10-29)

Use the native gold panel card for the currently inspected mod instead of a narrow
selection stripe. This selection is separate from the mod's enabled switch. Add a
search field with clear button above the mod list and icon-labelled All / Enabled /
Disabled filter buttons. Search matches names, IDs and descriptions without changing
activation or rescanning mod files on each keystroke. Search and status filters combine;
changing results resets list scroll and chooses a visible mod for the details pane.
Guest room browsing remains read-only, with the same local search/filter controls.


### Achievement list rendering / Mods selection (2026-10-29)

The achievement catalogue retains all 162 entries from the installed game. Its
viewport explicitly clips, and fixed-size cards opt into skipping offscreen
subtrees while preserving full scroll extent. This changes rendering work only;
achievement progress/reward behavior is unchanged. Mods selection uses a gold
outline over a blue card rather than a solid yellow card.


### Co-op room Mods placement and menu snapshots (2026-10-29)

Room Mods belongs to the main match-settings card header, alongside its title,
instead of floating near the lower corner. Room authority and mod selection are
unchanged. Menu backdrop averaging runs on the GPU before reading 1/64 as many
pixels back; blur, vignette and tint remain as before. Actual-context comparisons
matched the CPU reference within 1/255 per channel across four window shapes.
Visible opening sprites use bounded parallel decoding before the entrance begins.


## Achievement card pooling (2026-10-29)

Only viewport cards plus two buffered rows are instantiated; the normal 2160-unit
viewport retains 10 card trees rather than 162. Full metadata/search remain loaded.
Grid logical ranges preserve native layout and full scroll extent. Existing cards
are recycled on scroll/search; nearby sprites prefetch using bounded workers and
cooperative render-thread uploads. Progress binds by catalogue ID, and the header
counts the full catalogue even during search. Verified top, final row, one/no
results and reset; 3 achievement checks and 2 existing grid checks passed. Final
optimized build passed and BTD6.exe was replaced (SHA256
1B06F1A8175216559374D61A71311D08CED8009A12C21F58960DBBF54B4BD09E).
This reduces instantiated card trees by 94%; no additional FPS improvement is
claimed from the single final capture (concurrent asset work affects timings).

### Setup test build: achievements removed (2026-10-29)
- Supersedes the earlier achievement pooling work: Achievements is removed from menus and future exports. Trophy Store occupies its original Achievements slot. Shared Trophy Store artwork remains.
- Existing generated achievement cleanup removed 125 unused files (about 4.1 MB); player saves are untouched.
- Setup reuses Unity bundle state for bounded groups of 16 model jobs. A two-model comparison took 4.42 s batched versus 7.58 s serial (42% less time), with all 18 output files identical. This is not a whole-install timing.
- Terminal progress follows completed work, shows a percentage/bar and adjusting estimated remaining time, and retains successful checkpoints on failure. The setup prompt directs users to the MelonLoader terminal and no longer claims a fixed 25 GB requirement.
- Installed a hash-verified preview DLL and local preview payload in Steam BTD6 Mods for a fresh-folder setup test. The preview bypasses the older GitHub payload. Full first-install time and the under-30-minute target still need the user test.

### Parallel setup workers (2026-10-29)
- Model batches now run concurrently: up to four subprocesses, limited by logical CPU count and currently available physical RAM. The policy reserves 6 GiB and budgets 4 GiB per worker; unknown/low memory falls back to one. RRBTD6_SETUP_WORKERS can lower the limit. NumPy nested thread pools are limited to avoid oversubscription.
- Bundle caches use OS-owned locks so workers cannot overwrite an incomplete shared decompression cache. Locks are released when a process exits, including termination. Conflicting model output destinations are rejected; progress aggregates actual completed models and remains monotonic. Export indexes remain unpublished after failure.
- Measured 12 fresh Dart models in four groups of three: parallel 10.44 s vs serial 17.61 s (40.7% less time), all 106 output files byte-identical. This compares model preparation only, not a whole-install duration. Proof: local_extracted/setup-parallel-proof-20261029-203344/result.json.
- Steam local preview tools ZIP updated and hash-verified. Existing installer DLL uses the new payload on the next setup attempt. An already-running extraction keeps its existing tools; no active install was modified.
- Tests: setup/progress/concurrency/cache locks 12 passed, extraction plan 15 passed, bootstrap 4 passed, packaging 2 passed. Includes real multiprocess lock contention and release after termination.

### Parallel maps and simpler setup prompt (2026-10-29)
- Setup prompt now says welcome, select a folder, allow time to extract/prepare assets, do not close BTD6, and follow the MelonLoader terminal. Removed the extra description, storage paragraph and test paragraph from that prompt.
- Map preparation now uses memory/CPU-limited parallel workers, preserving data-before-geometry order within each map and propagating failures. Nested NumPy thread pools are limited across exporters. Shared shader JSON is published via unique temporary files and atomic replacement so concurrent exports cannot leave a partially written shader.
- Four-map proof (Logs, Resort, Tinkerton, Tutorial): 6.59 s parallel vs 13.23 s serial (50.2% less elapsed time), all 34 output files identical. This is a map-stage sample, not a total-install benchmark. Proof: local_extracted/setup-map-proof-20261029-203857/result.json.
- Installer DLL rebuilt (zero errors) and replaced in Steam Mods after backing up the previous copy. Preview tools ZIP rebuilt with a private packaging index to avoid changing the other agent's staging area; installed DLL/payload hashes verified. No game process was running during replacement.
- Validation: setup tests 12 passed, extraction plan 17 passed (including parallel map ordering and failure), bootstrap 4 passed, packaging 2 passed. Full clean installation duration remains unmeasured.

### Reuse decoded setup assets (2026-10-29)
- Profiled four models: 6.54 s in type-tree decoding out of 12.69 s instrumented time. Animated export now indexes prefab paths/objects once per loaded environment. Scene actor resolution remains unchanged.
- Reuse exact PNG bytes with a 64 MiB LRU per process, still writing separate model files. Reuse decoded clips with a 32-entry LRU and private copies for per-model role/weight annotations. No change to pose samples, skinning math, quality or frame counts. These caches are setup-only.
- Twelve Dart models: 12.38 s versus 17.51 s (29.3% less time), all 105 files identical. Mixed Dart/Alchemist/Druid/Dartling: 16.76 s versus 20.47 s (18.1% less time), all 91 files identical. These are model-export samples, not whole-install timings.
- Proofs: local_extracted/setup-lookup-proof-20261029-212038/result.json and local_extracted/setup-mixed-proof-20261029-212107/result.json. Previous exporter sources were captured before edits.
- Tests: setup/cache 15 passed; extraction plan 17 passed; bootstrap 4 passed; packaging 2 passed. Steam preview tools updated, including required export_cache.py. Did not modify any active user extraction.

### Batch tower artwork preparation (2026-10-29)
- export_textures.py supports ordered JSON jobs in one interpreter, retaining at most two loaded bundle/container pairs. Existing single-output CLI remains supported. Upgrade-art stage now uses one job manifest; the later tower/hero asset pass also batches its artwork. Base/tier/paragon/projectile selection is unchanged.
- Preserve output order for shared icon filenames, numeric upgrade prefixes, ASCII-folded upgrade filenames and canonical tower portrait suffixes. Existing legacy-output cleanup remains; missing fresh output directories are supported. Temporary manifests are removed on failure.
- Eight jobs across Dart, Desperado, Druid and Engineer (64 files): 10.39 s batched versus 69.97 s separate processes (85.1% less elapsed time), every output file hash identical. Proof: local_extracted/setup-art-proof-20261029-214034/result.json. This is an artwork-stage sample, not a whole-install timing.
- Tests: setup/cache/artwork 17 passed, extraction-plan 18 passed, bootstrap 4 passed, packaging 2 passed. Include bounded bundle reuse/eviction, shared-name overwrite order, numeric/Unicode/portrait naming, fresh extended-export directories and failed-manifest cleanup.
- Updated and hash-verified Steam preview tools.zip. No DLL or gameplay changes needed; next setup attempt uses this payload. Full install time remains to be measured.

### Precompute invariant animation-export data (2026-10-29)
- Precompute unchanged local transform matrices once per model and homogeneous position/normal arrays once per renderer. Reuse them while baking frames; animated transform attributes, Euler/quaternion precedence, procedural rotations, world transform order and skinning arithmetic are unchanged.
- Mixed 12-model proof: 14.92 s versus 16.58 s with the immediately previous exporter (10.0% less elapsed time), all 91 output files identical. Both use the same current prefab/clip/PNG cache helpers. Proof: local_extracted/setup-pose-proof-20261029-214738/result.json.
- Added independent byte-exact math comparisons against the pre-optimization reference for static/partially animated transforms, quaternion/Euler poses, procedural rotating bones, mirrored scales, and static/weighted mesh vertex outputs. Both tests pass; added to release CI after NumPy installation.
- Other checks: setup/artwork 17 passed, extraction-plan 18 passed, bootstrap 4 passed, packaging 2 passed. Updated and hash-verified Steam preview tools. Runtime renderer/simulation are untouched. Full-install timing remains unmeasured.

### Indexed animation sampling during extraction (2026-10-29)
- Clip.values now uses binary lookup for monotonic streamed key timestamps instead of rescanning/evaluating every earlier key per sample. Duplicate timestamps retain last-key selection. Nonmonotonic or NaN-key data use the original scan; NaN sample handling remains consistent.
- Precompute binding offsets once per decoded clip. Replace NumPy scalar clamp calls in dense sampling with equivalent scalar bounds; vector interpolation and polynomial evaluation order remain unchanged. Shared clip sampling also benefits particle animation export.
- Mixed 12-model proof: 12.61 s new vs 14.86 s immediately previous sampler (15.1% less elapsed time), all 91 output files identical. Proof: local_extracted/setup-sampling-proof-20261029-215432/result.json. These are component timings, not a full-install speed claim.
- Asset-free reference tests compare sampled value and pose bytes across empty/single/long/duplicate/unsorted/NaN keys, dense and constant data, bounds/infinite times, and duplicate/non-transform bindings. Two tests pass; included in release CI. Existing checks also pass: setup 17, extraction-plan 18, bootstrap 4, packaging 2, matrix/vertex precompute 2.
- Updated and hash-verified Steam preview tools. No simulation or runtime renderer changes. Existing active extraction processes retain their own tooling.

### Measured worker memory budget (2026-10-29)
- A representative 32-model generated-bundle batch (Dart, Super, Wizard, Dartling, including upper tiers) peaked at 1115 MiB working set / 1137 MiB commit. Proof: local_extracted/setup-memory-proof-20261029-220026/result.json.
- Generated-only jobs may now use up to eight workers with 2 GiB budget each, still reserving 6 GiB available memory and respecting CPU limits. Original/unknown/mixed bundles and maps retain the conservative 4 GiB / four-worker cap. Lower user-specified limits remain honored.
- On this machine the measured policy selected four instead of two workers: 64 models took 17.65 s vs 31.79 s (44.5% less elapsed time), all 497 files identical. Proof: local_extracted/setup-workers-proof-20261029-220253/result.json. Not a whole-install timing.
- Steam preview updated and verified. Setup tests 17, extraction-plan tests 18 and packaging tests 2 passed. Full fresh extraction measurement started separately with no existing prepared assets (Python dependencies already installed).

## Our own startup branding (2026-10-31)

- The Ninja Kiwi splash (its logo and the NZ On Air "Game Sector" logo) is replaced by our own
  screen: **RUST TD6** in white Comic Sans (the system's font; the default font where it is
  missing) on the website's blue (#2f80f5), and under it "Unofficial client." and "BTD6 and all
  of its assets are owned by Ninja Kiwi. Ninja Kiwi does not endorse this project." It fades in,
  shows for 3 seconds and fades out.
- The title screen draws **RUST TD6** on a rounded panel of the same blue where the BTD6 logo
  was (it moves and fades as the logo did); the Arcade+ / Kids+ marks are not drawn.
- The window, taskbar and EXE icon (BTD6.exe and the launcher) is ours: a white Comic Sans "R"
  on a rounded square of the same blue (renderer/assets/icon, tools/assets/make_icon.py).

## Jukebox: your own music (2026-10-31)

- The jukebox screen has a **My Music** button (the Add All button's look, on the Songs title
  line). It opens the `Custom Music` folder beside the data folder, making it if needed.
- MP3, WAV, FLAC and Ogg files put there join the Songs list (named after the file, after the
  game's tracks, always available) and can be added to the playlist like any track. The list
  picks up new or removed files every 2 seconds while the screen is open.

## Profile stats: Overall Stats only (2026-10-31)

- The stats dropdown's Main Game, Rogue and Frontier categories are gone (their modes aren't in
  this game). Overall Stats is the only list, so the dropdown shows no arrow and doesn't open.

## Home island: no Race / Contested Territory / Odyssey props (2026-10-31)

- The home island's seasonal props for removed modes are gone and not extracted: the Race flag and
  tyres, the Contested Territory banner, and the Odyssey fire pit, sword and helmet.

## First-run welcome popup (2026-10-31)

- The first time the game is ever opened, the home screen shows a popup (the game's CommonPopup):
  **Welcome to RUSTTD6!** "In RUSTTD6, you own everything! There are no tower unlocks or Monkey
  Money. However, when you complete maps, you get trophies to buy the cosmetic items." with one
  **Let's Go!** button. settings.json `welcome_seen` remembers it was closed.

## Owned upgrade description: hold, then release (2026-11-01)

- In the upgrade menu, holding the mouse on an upgrade the tower already has shows its name and
  description in that path's info popup after 0.2 s; releasing the mouse hides it. Those icons have
  no press animation. (The game shows it after hovering 0.2 s and hides it when the pointer leaves.)

## Trophies instead of Monkey Money (2026-10-31)

- Winning a map gives **trophies** (there is no Monkey Money): the game's Monkey Money reward for the
  difficulty and mode (75 Easy, 125 Medium, 200 Hard, 300 Impoppable / CHIMPS) times the map tier
  + 1 (beginner to expert), a fifth on replays, divided by 7.5. A first Easy win on a beginner map
  gives 10. The victory screen shows them in its rewards with the trophy icon.
- Each rank level gained gives 10 trophies.
- The Trophy Store sells every item, all always available: nothing is hidden, nothing says Limited
  Time, and quest / Nexus items can be bought too. Prices are the cached store's (50 where it has
  none). Bought items are enabled and can be toggled and favourited; they don't change anything in
  game yet.
- Monkey Teams isn't in this game: the victory screen never shows its card.
- Golden bloons aren't in this game: none spawn, and the victory screen never shows the golden bloon.
- The balance at the top of the menus is trophies, in Monkey Money's place, and the difficulty screen
  shows its rewards in trophies. There are no Monkey Knowledge points.
- Powers cost their Monkey Money price divided by 7.5, rounded up; ones the game only sells for real
  money cost 50. Monkey Knowledge costs 10 trophies (a rank level's worth, what a point was), plus any
  Monkey Money price divided by 7.5. Bought Powers can't be used in a game yet, and Knowledge doesn't
  change anything in game yet.

## Co-op lobby player info (2026-10-31)

- Each player in the co-op lobby shows their hero, rank and banner (sent between clients; the
  protocol version is now 9). Their info button opens a small popup with their name, rank, hero,
  avatar and banner, rather than the game's full profile screen (other players' stats aren't
  shared).
- Trophy Store avatars and banners must be bought before they can be equipped; unbought ones show
  their price in the picker, as the game does.

## Memory failsafe (2026-11-01)

- In the sandbox, the bloons are cleared with a message when the game holds too much memory, when it's large
  and the PC is nearly out, or after 3 frames in a row of over a second. Sending 10,000 ZOMGs froze Windows and left
  the display at 60 Hz with broken scaling until a restart.
- In every mode, the process has a hard memory limit of half the PC's total commit (6 to 16 GB). Past it the
  game crashes, instead of taking the PC down.
