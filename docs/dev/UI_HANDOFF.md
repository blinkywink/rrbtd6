# UI ownership and current work

Codex: main-menu UI. Claude: tower model/display exports.

User correction: copy original appearance AND behavior exactly using reverse engineering.
No guessed replacement layouts, fake shader colors, or explanatory dialogs as features.

Claude commit 53fbc7a included the initial frontend pass together with tower changes.
The subsequent UI correction fixes that pass; do not revert Claude's tower work.

Original sources now exported:
- MonkeySelectUI + MonkeySelectMenu native methods: original 8-slot grid, four group tabs,
  category arrows; selecting a tower opens UpgradeScreen and Back returns to the grid.
- HeroDetailsUI + HeroUpgradeDetails / HeroUpgradeButton native methods.
- All 23 hero background prefabs. SkinData resolves the default backgrounds for all 18 heroes
  (Rosalia uses SaudaBg; do not infer a background filename from hero ID).
- SkinData's descriptions, short descriptions, portrait metadata, tint colors, title materials.
- 6 original hero TMP title materials.
- Animator curve endpoints: 48 HeroDetails animators, 24 MonkeySelect animators.

Tools: export_ui_runtime.py, export_frontend_data.py; dump_ui_prefab.py retains component and
transform IDs to resolve controller refs. Extracted output stays under local_extracted/ui.
Native disassemblies are ignored under target/ui-research.

Current build: target/codex-ui/game/btd6-game.exe. Latest native_monkeys.png and native_heroes.png
are ignored under local_extracted. Build passes; these captures are NOT proof of pixel equality.

Remaining: full Animator playback/transitions, hero level clicks and portrait changes, skins,
all native material/shader behavior (including main-menu water), XP/account state, loading screen,
and comparison against a live original game capture. Heroes is not marked complete.

Shared main.rs edits from Codex: frontend screenshot routes and SelectHero save action only.
Gallery/camera/model diagnostics belong to Claude. Please stage only each agent's own files/hunks.


Latest Heroes correction:
- Full 52-entry SkinData list, ordered with default skins first as verified in native
  UpdateHeroSkinButtons (OrderByDescending(s => s.isDefaultTowerSkin)).
- 258 exported PNGs including the actual background pattern; all skin image/background/font
  references resolve. Asset export bounds UnityPy's decoded-atlas cache to avoid RAM growth.
- HeroDetailsUI uses PowersPatternGreyBg, 15 tiles per height, with VignetteUI's compiled
  math (_Pow 1, _mul .7). The blurred-island replacement has been removed for Heroes.
- Native skin buttons now change previews, backgrounds, tint, title material, description,
  and roster icons; selections persist in settings.hero_skins. In-game skinned tower models
  are NOT applied by this UI change; that needs the model/skin pipeline, owned by Claude.
- Flexible GridLayoutGroup preferred height now uses actual width (native calculation).
  Two regression tests cover the skin column and reflow.
- Final verification build target is target/codex-ui-verify because target/codex-ui is running.
- RRBTD6_HERO_PREVIEW=Quincy:Cyber Quincy captures a skin without modifying settings.

Not claiming complete visual equivalence: full transitions, level portrait changes,
IAP/account state and live original-game comparison are still pending.

Final checks for the skins/background pass: 32 Rust tests passed, 5 ignored. Verification
binary target/codex-ui-verify/game/btd6-game.exe built successfully. Captures
local_extracted/heroes_skins_verified.png and heroes_voidora_verified.png show the fixed
single-column native skin layout. No proprietary assets are staged or committed.

Hero level interaction pass: expand/collapse now applies original UpgradesList endpoints and arrow rotation; clicking a visible level highlights it and uses that skin's level portrait. Scrolling is bounded to content. Transition curve playback remains pending. Native UpdateSelectedUpgrade / GetHeroPortraitForLevel inspected. 32 tests passed (5 ignored) before scroll clamp; only frontend.rs owned changes staged.

Heroes animation pass (Codex):
- export_ui_runtime.py now preserves streamed cubic coefficients, dense samples and constants.
- frontend caches runtime data once; original Open3 and UpgradesListExpanded/ExpandDown
  play at their original duration. No animation file reads per frame.
- Verified 255 level-panel channel samples against the prior Unity decoder, maximum error 0.
- He-Man level descriptions now use SkinData.alternateLocsName, matching SetupHeroTiers.
- RRBTD6_HERO_LEVEL=20 and RRBTD6_HERO_EXPANDED=1 provide reproducible previews.
- 33 tests passed, 5 ignored; final executable built. Expanded He-Man capture verified at
  local_extracted/heroes_heman_expanded_verified.png. Full original-game visual parity,
  transformed double portraits, skin swap effects and selected-card shaders remain pending.
- Only frontend.rs, export_ui_runtime.py and this handoff are part of this commit.

Current menu expansion (Codex, 2026-10-23):
- Heroes roster now binds iconSquare (rectangular source portraits), not SkinData.icon.
  Both folders deliberately share filenames. New index_sprite_guids.py reconstructs exact
  source GUIDs from Sprite.m_RenderDataKey (reverse each word's hex nibbles) and records
  catalog_sprite_paths.json. Ambiguous image filenames get GUID-qualified aliases.
- export_frontend_data.py now preserves Sprite full rect / transparent padding, fixing
  Harlegwen's round button distortion. It applies skin SwapSpriteContainer replacements
  to level portraits, including Scientist Gwendolin. Top roster count is shown.
- Heroes roster / levels / skins support wheel and drag, inertia, gesture ownership and
  click cancellation. Full pre-mask bounds fix the old zero scroll limit. Hidden parent
  pages no longer leave clickable descendants. Four meaningful scroll regressions added.
- menu_pages.rs lazily imports Knowledge + six trees, Play Social, Powers, Achievements,
  Store, Trophy Store, Legends and Mailbox; ordinary navigation + Back are connected.
  Original CommonPanel flags now control foreground currency, Knowledge, hero/friends UI.
- New export_menu_catalogs.py extracts 162 achievements and 439 Trophy Store item assets,
  localized display text, raw metadata and icon refs. Catalog cards use original templates.
  No progression / purchases / unlock state is written. Prices and account ownership are
  intentionally unbound. Live stock cannot be inferred from the local item assets.
- Generic source ScrollRect references drive page dragging / wheel panning, including
  Knowledge trees. Off-screen graphics skip texture decoding and text shaping, while
  retaining layout. Unchanged text / sprite bindings avoid allocating replacements.
- Slim hero_shop.json + hero_levels.json avoid constructing a simulation to browse Heroes.
- Home pause button had no handler: connected to Home action and cleans placement state.
- loading.rs uses original LoadingUI and Open / Idle / Close curves during map transitions;
  asset work remains real, no invented percentage. Initial startup splash still pending.
- Re-exported mainmenu world with translated WaterPanning / ShoreFoam material data.
  HomeScreen's original Open animation plays. World skeletal / prop animation playback
  is still pending; original MainMenuWorld curves are now exported as research data.
- Build target codex-ui-verify became locked by a running user app; do NOT kill it.
  Current new verification target is target/codex-ui-polish.
- Source recipe export_frontend.sh includes pages / LoadingUI / TrophyStoreCard / catalogs.
- Known incomplete: full world animations, all dynamic Social / Store / Mailbox content,
  achievements reward visuals, live account state, full input / tabs on new catalog pages.
  Do not call all menus pixel-identical or complete. User explicitly wants visual extraction
  and future wiring only, no gameplay / progression changes in these menu additions.
- Claude is still editing engine / gameplay in this shared checkout. Stage owned files only.

Latest catalog / home polish:
- Missing menu chrome was embedded in UI scene / asset-reference bundles, not shared atlases.
  export_menu_chrome.py exports 154 missing sprites with full padding and slicing metadata.
- Achievements: 162 entries, original SmallBaseLootItem / SmallInstaTowerLootItem reward
  templates (verified LootThemes theme 1 GUIDs); read-only reward parsing, no unlock writes.
- Trophy Store: 439 entries, source category badges and exact gold frames, neutral preview
  balances; live prices / stock / purchases remain unbound. One text-emote image is unresolved.
- home_world.rs stages two original continuous looping clips off-thread (fountain Dart,
  co-op monkey); uploads at most one ready model/frame. Static parts are replaced by exact
  hierarchy path only when an animated replacement is ready. Old exports safely stay static.
- Windmill uses original CustomRotationSimple axis/speed, source parent/rest transforms.
  Native Update confirmed speed * deltaTime * 360; interaction acceleration remains unwired.
- export_animated.py supports scene:CAB|hierarchy and optional RRBTD6_ANIM_CLIP.
- Current unlocked verification build: target/codex-ui-catalog/game/btd6-game.exe.
  Existing user-run binaries were left running when locked.
- Still incomplete: cloud/vegetation motion, popup interactions and seasonal world behavior,
  initial startup splash, catalog filters / selections / services, and dynamic Social/Store/Mailbox
  content. Full pixel parity is not established. Check captures under local_extracted/.
- Cloud motion now plays from original PlayableAnimatorSimple cubic channels. Both long
  cloud loops use 13.5 KB of curve metadata and update existing scene vertices in place,
  retaining the original materials; no long frame bake or per-frame buffer allocation.
- Final validation: build succeeds; 38 active tests pass (5 ignored); captured and visually
  checked Heroes/Harlegwen, achievements with rewards/portraits, Trophy Store, and Home.
  Vegetation motion / triggered popup world interactions still need original playback wiring.

Hero collection / catalog browsing follow-up:
- Hero count now counts base appearances plus skins, not the hero roster. Extracted metadata
  records default, hidden-when-locked, MM cost and IAP ID. Offline preview exposes 49 non-IAP
  appearances (including the special RedSauda unlock), displayed as 49/49; it is not account
  progress. Three premium appearances remain visible for preview but do not imply ownership.
  Native HeroUpgradeDetails.UpdateHeroCount uses GetOwnedHeroesAndSkins / GetTotalSkinsCount;
  actual ownership, SKU overrides and hidden eligibility need account binding later.
- Trophy Store All / Heroes / Monkeys / Bloons / Co-Op / Game & UI category tabs filter the
  original cards. Clicking a card updates the source bottom panel and selection highlight.
  Dragging more than the 5px threshold suppresses card selection. Filters reset scroll/inertia.
  Featured, inventory, favorites, prices and purchases remain unbound.
- Other menu pages play cached original opening clips on entry/re-entry; only the relevant
  clip is retained, not every controller in the scene. Existing layout templates stay original.
- RRBTD6_TROPHY_CATEGORY supports read-only category captures, e.g. Heroes.
- Shared target/codex-ui-catalog disappeared during a build; isolated UI verification moved to
  C:/Users/Admin/AppData/Local/Temp/rrbtd6-codex-ui-build (do not delete during compilation).

Loading flash / animated Powers follow-up:
- Loading snapshot now occurs on the request frame before the map clear; actual load runs
  at the top of the next frame. Frontend Play and in-game map switching preserve the old
  rendered screen under the source fade. Empty stripped animation channels preserve the
  previous property value instead of writing zero (LoadingUI idle had erased black alpha).
- Loading draw resets the material and clears black under its captured backdrop. Added
  RRBTD6_LOADING_CAPTURE_DIR opt-in phase captures, with no readbacks in normal play.
  Real menu -> game capture verified opaque stages remain black and close reveals the map.
- Powers: source background registry key is InstaPowersCollectionUI; 19 original animated
  portrait prefabs now load lazily on selection into the original animatedIconContainer.
  Only the selected portrait samples original curves; cached templates/runtime are reused.
  Corrected tab highlights, preview count 0, and the source Open clip reactivating an unbound
  white IAP launch image. Selection updates title, price, description and portrait without buying.
- export_ui_runtime.py supports prefab:PATH targets and includes source loop flags.
  export_power_portraits.py + frontend recipe extract all 19 portraits; 33 additional original
  sprites exported; no unresolved Power portrait image references. Description parameters
  use source boost duration, Thrive scale and trap projectile pierce, not guessed constants.
- Achievements: typed search filters cached localized titles/descriptions; original placeholder
  and no-results graphic are bound, scroll resets, Escape first dismisses text input.
  RRBTD6_ACHIEVEMENT_SEARCH / RRBTD6_POWER_PREVIEW support source-faithful visual captures.
- Verification: 45 active tests passed, 5 ignored; 8 focused UI tests passed after final edits.
  Python tools compile. Root BTD6.exe replaced from the isolated build, hash verified.
- RRBTD6_TIMING now reports frontend draw CPU in 100-frame batches, excluding pacing/present.
  Warm batches here: Powers 0.165 / 0.166 ms avg; Trophy Store 0.440 / 0.423 ms avg.
  First page-open maxima ~60 ms include initial parsing and texture loads; not a GPU/FPS claim.
- Still pending: full menus/account data binding, Pro Powers catalogs/upgrades and inventory,
  online cards, startup splash, remaining original world interactions/vegetation animation.

Monkey Knowledge preview follow-up:
- export_knowledge_menu.py binds all 134 KnowledgeModels to the original tree node paths,
  source icons/highlights, localized names/descriptions, MM prices and investment messages.
  All 134 matched; none lacked a description. Raw source model references remain available.
- Selecting a tree node now opens the original SelectedPanel with the native opening clip;
  clicking outside or changing category closes it with the source closing clip. Free nodes
  hide the price container. Paid-node prices come from KnowledgeModel.monkeyMoneyCost.
  No points, currency, ownership or gameplay knowledge are changed.
- Hidden the Knowledge tree's gamepad-only JewelBar that exposed its authoring StoreButtons
  debug label in mouse UI. Existing mouse buttons remain outside that controller bar.
- RRBTD6_KNOWLEDGE_PREVIEW=IncreasedLifespan / MoreCash enables read-only screenshot checks.
- Build, 8 focused UI tests and catalog completeness validation pass. Free/paid panel captures
  inspected; warm draw CPU averages 0.110 / 0.094 ms, excluding GPU presentation. First open
  max 86 ms includes source JSON parsing/images. Root BTD6.exe updated again after this pass.

Legends slideshow follow-up:
- export_legends_menu.py resolves native LegendsSelectPanel/LegendsSelectSubPanel references,
  all three ordered source slides, arrow/About controls and unresolved source localization.
- Original Home/Rogue/Frontier panels are mutually exclusive. Arrows wrap, page pips select
  panels, About/OK opens/closes the native information panel; Escape dismisses About first.
- Source PageTransition curves play only for the selected panel. Only relevant controllers
  are retained; scene/catalog load is lazy and no assets are re-read on panel navigation.
- Frontier description now uses English.xml rather than displaying its key. Removed the
  empty release banner and gamepad JewelBar. Offline expansion previews use source Play
  information panels; source purchase panels had dummy $0.00/$10.50 and tax placeholders.
- RRBTD6_LEGENDS_PREVIEW=Home/Rogue/Frontier/about supports read-only visual checks.
- Build and 8 focused UI tests pass; captured/inspected all three slides and About.
- Expansion launch, product services and ownership remain unbound; no state is written.

Social navigation / co-op screen framework:
- CommonForeground ChangeHeroButton now opens Heroes and Back returns to the originating
  page (Social), while the main-menu Heroes button still returns Home. Legends' native
  Home button now requests the actual Home screen instead of doing nothing.
- Added original CoopQuickMatchUI and CoopJoinMatchUI scenes, source opening animations,
  lazy cached MenuPage routes from Social Quick Game / Join Game, and Back -> Social.
- Quick Match initial Any Game uses source localized AnyMap/AnyDifficulty/AnyMode and only
  its own selection glow. The author's three glows and sample criteria are not runtime state.
- Join code field accepts text/backspace, Enter blurs, Escape first dismisses focus. Its
  original input component allows 200 characters (codes and share links); dumper now keeps
  input constraints. RRBTD6_JOIN_CODE supports deterministic read-only visual captures.
- Captured/inspected both native screens and entered-code display. Build and existing 8 UI
  tests pass. Warm CPU draw averages 0.043 / 0.031 ms (100-frame batches, excludes GPU/present).
- Still unbound: actual matchmaking/join/local discovery, Medium/Long remote preset data;
  these are visual screen frameworks only, not working network services. Join field currently
  supports append/backspace, not full native selection/clipboard/share-link parsing.

Shop local catalog framework:
- export_store_menu.py reads all 87 GameData.storeItems records, preserves raw source data,
  resolves exact Sprite GUID artwork, and exports native StoreCategory labels. All 42 regular
  viewable records have localized names and icons; exporter fails on missing display data.
- Extracted original ProductButton small/large prefabs. Shop now instantiates its native
  small-card template in its own two-row horizontal grid and uses source card art/aspect.
  Uses local source record order; this is a fallback preview, NOT live most-popular stock.
- Fixed blank category titles / simultaneous tab highlights; removed permanent authoring
  loading wheel, Tax Message Placeholder and fake creator-support tick. Source sliced art
  and grid sizing are retained. Catalog and images remain lazy; offscreen graphics cull.
- Products have no invented live prices, sale timers or purchased state. Buy controls remain
  unbound and hidden. Category filters, service stock ordering, product detail popups and
  purchase/inventory state remain pending. Only the local catalog preview is displayed.
- Build/captures checked. Warm Shop CPU draw averages 0.114 / 0.117 ms (100-frame batches,
  excludes GPU/presentation); first open max 76.6 ms includes parsing and image uploads.

Home vegetation source playback:
- export_home_vegetation.py exports 23 active native PlayableAnimatorSimple grass/palm clips,
  cubic channels, hierarchy transforms, bind matrices and 1,983 weighted vertices. ~961 KB
  local JSON; no per-frame mesh caches or replacement material/texture copies.
- HomeMotion parses the file on a worker thread, maps source submeshes to existing world
  meshes, and deforms those vertices using reusable world/skin matrix buffers. Native
  PlayableAnimatorSimple.Init disassembly confirms random speed replaces playbackSpeed
  and random start calls set_NormalizedTime(Random.value); stored ranges/flags are retained.
- Export validates every rest vertex against mainmenu/verts.bin. Max world-coordinate
  discrepancy 0.00003051; no empty transform channels. Triggered nested monkey renderers
  are excluded from the vegetation skinning. Added weighted-bind/reflection regression test.
- Home screenshot inspected; warm full-home draw CPU ~0.506 / 0.527 ms (GPU excluded).
- Full package tests: 47 passed, 5 ignored; perf_budget had 6.225 ms worst during concurrent
  Unity extraction. The unchanged test passed isolated afterward; UI/skin tests all passed.

Pause settings / home interactions / social layouts:
- Pause Hotkeys and Accessibility (Advanced) buttons now dispatch actual menu actions.
  FrontEnd.open_game_options retains the paused menu and shares its existing game backdrop
  (no new GPU readback). Back/Escape returns to the same pause menu, not Home or a new game.
  Rebinding Escape cancels the pending binding first. Options edits use existing persistence.
- RRBTD6_MENU=pause + RRBTD6_PAUSE_OPTIONS=hotkeys|advanced exercises the real action path;
  RRBTD6_OPTIONS_BACK_FRAME=35 exercises the shared Back path. Both settings screens captured,
  retained-pause return confirmed. Removed the pause-overlay text ghost in options backdrops.
- dxbc_glsl.py now accounts for UnityPy's top-down Texture2D exports at shader sampling,
  while vertex inputs restore native Unity UVs. Includes explicit-gradient sampling handling.
  Rebuilt 13 original shader translations from retained DXBC via rebuild_shader_glsl.py.
  Home sea now shows its shallow-water shading and bright shoreline foam. No new paint/art.
- export_home_interactions.py exports six native trigger clips, enabled BoxCollider shapes,
  transforms, clip durations and no-interrupt flags. Windmill targets Ace's own Animator.
  export_animated.py's explicit RRBTD6_ANIM_INTERACT path selects a source interaction clip
  even when it is absent from the ordinary Animator controller's idle list.
- Ray/OBB picking uses the real camera projection and source collider transforms, including
  moving palm bones. UI buttons block world clicks. Bomb, Dart, Sub, Ninja, Boomerang and
  windmill/Ace clips load lazily off-thread, upload at most one model/frame and cache on use.
  Static/idle geometry is hidden during playback and restored afterward; cached replay hides
  it immediately to avoid a duplicate first frame. Native no-interrupt behavior is retained.
- RRBTD6_HOME_INTERACT=<source path fragment> projects a source collider center and exercises
  the actual ray hit; all six hit correct targets and captures show native clips in motion.
  Timed home audio/particle events, chest/temple interactions and seasonal overrides pending.
- Added original LinkFriendsUI, TeamsBrowserUI and PerksUI with source controller animations.
  export_social_menu.py resolves original component refs for tabs/dialogs/fields; SocialPage
  is a reusable local UI binder. Common Friends/Perks controls remember their originating
  screen, My Team opens the team browser, and tab changes reset scroll. Help/Add Friend/
  advanced-search panels open/close with source animations; Back dismisses dialogs first.
- Text fields preserve native limits (friend code 11, team search 40). Native empty states
  replace sample player/team cards. Perks help uses PerksInfo1..4/Subtitle, including the
  missing fourth section. Team intro service text is unbound: native TeamNoInfo is shown.
  Online actions, authorization, team filter submission and account perk ownership are not
  implemented. Advanced team-search controls currently retain original visual defaults.
- Modal content doesn't trigger background dismiss; Done/OK and outside-close areas are
  distinct. Underlying scroll regions are blocked while a dialog is open. Added regression
  coverage for modal hit handling and rotated/scaled native collider ray intersections.
- 25 additional source menu sprites exported. Source pages and only relevant popup runtime
  controllers are lazy/cached; no account state is changed by visual previews.
- Debug captures: RRBTD6_FRONT=friends|teams|perks, RRBTD6_SOCIAL_TAB=<index>,
  RRBTD6_SOCIAL_DIALOG=<index>. Inspected pages, request text, help and advanced-search views;
  automated Back closes Add Friend and leaves the Friends page visible.
- Final verification: 50 active package tests passed, 5 ignored (serial run); all 11 UI tests
  pass separately. Root BTD6.exe replaced from the verified build and source/destination hashes match.

## Home resting poses and tutorial marker
- Export native Patch, EventMonkey, Ninja, Bomb and Boomerang idle clips; preserve source loop flags (non-looping clips hold their final pose).
- Stage interaction models off-thread on Home entry, hide raw skin bind meshes, and use the first source interaction pose when no dedicated idle exists. Completed clicks return to the native idle rather than a T-pose.
- Hide Patch's TutorialArrow3D without an active bound tutorial. His quest exclamation marker remains.
- Verified Home screenshots and 51 passing package tests, 5 ignored. Warm frontend CPU in the capture was about 0.67 ms/frame, excluding present/GPU.
- Still pending: original menu/dialog handlers for Patch, TV, gift box and event/co-op monkeys. Native colliders and MonoBehaviour handlers confirmed in source scene; do not substitute arbitrary menu destinations. TV has DigitalContent script, gift box owns dialog and chest sounds, event monkey owns event dialog/state; source QuestMonkey.OnInteract calls MenuManager.OpenMenu.

## Home menu actions and Ninja puff
- Added export_home_buttons.py: four native BoxCollider menu regions extracted from MainMenuWorld, independent of popup animation triggers. Buttons use the same nearest ray-hit path as native animation interactions; a foreground UI button blocks world picking.
- Patch opens QuestBrowserUI, gift opens TowerGiftBoxUI, EventMonkey opens GameEventsUI. Alternate co-op preview lacks a serialized collider: its rendered animated source triangles are picked on mouse press only, also opening Events. This is a preview interaction fallback, not a claim that the native event-state controller is reproduced.
- Source AnimatedSeriesTV.OnInteract calls ShowAnimatedSeriesPopup. Resolved the popup asset GUID via catalog_prefabs.json to BloonsMonkeyTalesPopup, extracted its native prefab/art, and wired TV to that popup. Native X/Cancel and Escape return Home; suppress extra CommonForeground over the TV popup. External video playback remains unwired.
- Loaded native QuestBrowserAnim/Open, TowerGiftBoxPopUp/Open and GameEventsAnimation/Open; hide gift authoring debug controls. These are navigable visual shells: quest records, gift progress/selection, live event data/actions and account writes are not implemented. Source authoring content remains in those new shells.
- Exported native NinjaPuff (2 particle systems) and BombShooterMuzzleFlashPopUp (4). Exporter samples animated source event markers when inside the animated hierarchy, otherwise uses their original world marker transform. Native event delays: Ninja 1.75 sec, Bomb 1.1/1.85 sec. Home owns its separate particle player; these do not affect gameplay simulation.
- Captured actual native-collider click navigation for Patch, gift, event monkey and TV, plus Ninja puff and root-exe TV->Back->Home. Verified green tutorial arrow is absent on return; yellow quest exclamation remains intentionally.
- 52 tests passed, 5 ignored, serial full renderer package run. Canonical root BTD6.exe rebuilt/copied and SHA256 matches build output. No running game was present during version check; do not assert which version user previously launched.
