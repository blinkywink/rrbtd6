# Asset tools

The exporters behind [`tools/extract_all.py`](../extract_all.py), which runs them in order during setup, plus inspection helpers for debugging. Each description is the script's own docstring; see [EXTRACTION.md](../EXTRACTION.md) for the pipeline.

| Script | What it does |
|---|---|
| [`audit_behaviors.py`](audit_behaviors.py) | List the model types (behaviours, emissions, filters...) the game's towers use that build_game_data.py never mentions, i.e. |
| [`audit_visuals.py`](audit_visuals.py) | List every tower / hero visual the clone can't draw, so missing projectiles and effects show up without playing every upgrade. |
| [`build_display_plan.py`](build_display_plan.py) | Map every tier of the given towers to its tower and projectile display prefabs. |
| [`build_game_data.py`](build_game_data.py) | Assemble the prototype's game.json from decoded game data. |
| [`cab_index.py`](cab_index.py) | Index the game's asset bundles by their internal CAB file names, so exporters can load the bundles a prefab's materials / textures / shaders live in (PPtr m_FileID -> externals "archive:/CAB-.../CAB-..." -> bundle file). |
| [`canonical_anims.py`](canonical_anims.py) | Use the matching client to store legacy floats in its exact runtime format. |
| [`catalog_audio.py`](catalog_audio.py) | Map Addressables GUIDs to asset file names from catalog.bin via their locations (read-only). |
| [`catalog_guids.py`](catalog_guids.py) | Map Addressables GUIDs to asset file names from catalog.bin (read-only). |
| [`compact_anims.py`](compact_anims.py) | Shrink baked model animations (anim.bin -> anim_q.bin) in place. |
| [`compact_json.py`](compact_json.py) | Remove only JSON whitespace outside strings; preserve every value token exactly. |
| [`compress_audio.py`](compress_audio.py) | Store extracted PCM16 audio as verified lossless FLAC, without resampling. |
| [`delta_anims.py`](delta_anims.py) | Lossless temporal prediction for already packed baked animation files. |
| [`dump_tower.py`](dump_tower.py) | Print a tower tier's whole model tree (every behavior, emission, filter, projectile and field), to check a tower against the game model field by field. |
| [`dump_ui_prefab.py`](dump_ui_prefab.py) | Dump Unity UI prefabs (RectTransform tree + Image / TextMeshPro / layout components) to JSON. |
| [`dxbc_glsl.py`](dxbc_glsl.py) | Translate a Unity shader's forward pass (D3D11 DXBC, shader model 4/5) to GLSL 330. |
| [`export_animated.py`](export_animated.py) | Export an animated prefab |
| [`export_animated_batch.py`](export_animated_batch.py) | Export a bounded group of animated prefabs, sharing loaded Unity bundles. |
| [`export_audio.py`](export_audio.py) | Every AudioClip of the game into local_extracted/audio (read-only) |
| [`export_bloon_displays.py`](export_bloon_displays.py) | Export each bloon display prefab's sprite(s) with their in-world size. |
| [`export_bloon_models.py`](export_bloon_models.py) | Export the 3D bloon displays game.json names (MOAB-class bodies and damage states). |
| [`export_bundle_sprites.py`](export_bundle_sprites.py) | Export named Sprites that live outside the sprite atlases (embedded in other bundles). |
| [`export_cache.py`](export_cache.py) | Reuse immutable prefab lookups within one bounded exporter process. |
| [`export_checkpoint.py`](export_checkpoint.py) | Mark a generated projectile complete only after every export step succeeds. |
| [`export_coop_ui.py`](export_coop_ui.py) | Prepare original base co-op scenes and artwork for the local prototype. |
| [`export_cursor.py`](export_cursor.py) | Export the game's hardware cursor textures (Assets.Cursor |
| [`export_effect_sprites.py`](export_effect_sprites.py) | Export a sprite-flipbook effect prefab (e.g. |
| [`export_event_ui.py`](export_event_ui.py) | Prepare original event layouts/art for local builds; live rules use the CDN at runtime. |
| [`export_font.py`](export_font.py) | Export a TextMeshPro SDF font asset (atlas + glyph table). |
| [`export_frontend_data.py`](export_frontend_data.py) | Export original frontend SkinData and tower-container sprite references (local only). |
| [`export_game_icon.py`](export_game_icon.py) | Export the game's own window icon from the user's BloonsTD6.exe. |
| [`export_game_version.py`](export_game_version.py) | Export the installed BTD6's display version (local only). |
| [`export_hero_portraits.py`](export_hero_portraits.py) | Export hero portraits to local_extracted/ui/portraits/<HeroId>/ with uniform names. |
| [`export_home_animations.py`](export_home_animations.py) | Bake original continuous home-world idle clips in scene coordinates; local assets only. |
| [`export_home_buttons.py`](export_home_buttons.py) | Extract native Home menu hit volumes; proprietary output remains local. |
| [`export_home_events.py`](export_home_events.py) | Export native Home event controls and resolve their sprites in separate RAM-bounded passes. |
| [`export_home_interactions.py`](export_home_interactions.py) | Export original home interaction clips and their native BoxColliders. |
| [`export_home_vegetation.py`](export_home_vegetation.py) | Export source home vegetation curves and skinning, without per-frame mesh bakes. |
| [`export_jukebox.py`](export_jukebox.py) | Export the game's jukebox (AudioJukeBox.musicTrackData, referenced by Assets/Data/GameData.asset) to local_extracted/ui/runtime/jukebox.json |
| [`export_knowledge_menu.py`](export_knowledge_menu.py) | Extract read-only Monkey Knowledge card bindings from the original tree and models. |
| [`export_legends_menu.py`](export_legends_menu.py) | Bind original Legends slideshow references and unresolved localization. |
| [`export_map.py`](export_map.py) | Export one map (track, placement areas, terrain image) from its scene bundle. |
| [`export_map_clips.py`](export_map_clips.py) | The animation of map props, after export_map_mechanics.py |
| [`export_map_mechanics.py`](export_map_mechanics.py) | Everything a map scene does beyond its geometry, into local_extracted/maps/<key>/mechanics.json |
| [`export_map_pair.py`](export_map_pair.py) | Export map logic and geometry in one process, reusing the loaded scene bundle. |
| [`export_mapset.py`](export_mapset.py) | Export GameData's map list (MapSet |
| [`export_menu_catalogs.py`](export_menu_catalogs.py) | Extract Trophy Store display catalogs and icons; never writes game state. |
| [`export_menu_chrome.py`](export_menu_chrome.py) | Export scene-embedded menu sprites absent from the shared atlases, with slicing metadata. |
| [`export_missing_upgrade_icons.py`](export_missing_upgrade_icons.py) | Export every upgrade / ability icon in Assets/ResizedImages/UI/UpgradeIcons/ that local_extracted/ui/upgrades doesn't have yet (hero ability "...AA" icons, Corvus's spells...). |
| [`export_model.py`](export_model.py) | Export 3D renderers (static + skinned, posed by their bones) to a compact format. |
| [`export_particles.py`](export_particles.py) | Export Unity particle-system prefabs (Shuriken) for the renderer's particle player. |
| [`export_power_portraits.py`](export_power_portraits.py) | Extract original animated Power portrait UI prefabs and their animation curves locally. |
| [`export_profile_cosmetics.py`](export_profile_cosmetics.py) | Export the installed avatar/banner catalogs, preserving native order and asset references. |
| [`export_profile_ui.py`](export_profile_ui.py) | The profile page |
| [`export_projectiles.py`](export_projectiles.py) | Export the projectile displays named in local_extracted/display_plans/*.json. |
| [`export_rank_info.py`](export_rank_info.py) | Extract original player-rank thresholds for local account UI; no progress writes. |
| [`export_rounds.py`](export_rounds.py) | Export round sets, income sets, game modes and BloonMutatorData from the game_data bundle to JSON. |
| [`export_season_goals.py`](export_season_goals.py) | Prepare localized native Social Season goal labels and icons (local assets only). |
| [`export_social_menu.py`](export_social_menu.py) | Bind original social-menu controls without online/account mutations. |
| [`export_social_season_art.py`](export_social_season_art.py) | Export native Social Season theme artwork for the local menu renderer. |
| [`export_sprites.py`](export_sprites.py) | Export sprites from the sprite_atlases bundle as PNGs. |
| [`export_startup_plane.py`](export_startup_plane.py) | Bake the original title-intro Monkey Ace in its actor's local space. |
| [`export_store_menu.py`](export_store_menu.py) | Extract the original local Shop catalog and product art; no account or transaction writes. |
| [`export_text.py`](export_text.py) | Export TextAssets (e.g. |
| [`export_textures.py`](export_textures.py) | Export individual textures/sprites by Addressables container path. |
| [`export_tower_assets.py`](export_tower_assets.py) | Per-tower asset export (what build_local_assets.sh does in its tower loops), for adding towers. |
| [`export_tower_tiers.py`](export_tower_tiers.py) | Batch-export every tier's display prefab (animated) per local_extracted/display_plans/<TowerId>.json. |
| [`export_ui_runtime.py`](export_ui_runtime.py) | Export original UI controller references and decoded Animator curves and endpoints. |
| [`export_upgrade_xp.py`](export_upgrade_xp.py) | UpgradeModel.xpCost (MonkeyButton.CanAcquireUpgrade) and UpgradeModel.icon sprite per upgrade name. |
| [`fsb_vorbis.py`](fsb_vorbis.py) | The game's FSB5 Vorbis audio as standard Ogg Vorbis files, without re-encoding. |
| [`graft_clips.py`](graft_clips.py) | Add named animation clips from one exported model to another with the same vertex order. |
| [`index_sprite_guids.py`](index_sprite_guids.py) | Index original Sprite GUIDs to exact container paths, preserving same-name variants. |
| [`inspect_clip_bindings.py`](inspect_clip_bindings.py) | List an animated prefab's non-transform clip bindings (GameObject active, renderer enabled, ...) with their values over each clip. |
| [`link_duplicates.py`](link_duplicates.py) | Store byte-identical runtime asset files once, as hard links to a single copy. |
| [`make_icon.py`](make_icon.py) | Our app icon |
| [`mecanim_clip.py`](mecanim_clip.py) | Mecanim generic AnimationClip decoding (streamed Hermite curves, dense samples, constants), shared by export_animated.py (tower models) and export_particles.py (effect prefabs' Animators). |
| [`model_batch.py`](model_batch.py) | Bound memory with fresh processes between groups; reuse Unity bundles inside a group. |
| [`model_info.py`](model_info.py) | Show a model type's distinct parameter sets and which tower tiers use it. |
| [`pack_anims.py`](pack_anims.py) | Losslessly pack baked animations. |
| [`patch_model_cull.py`](patch_model_cull.py) | Add each model part's shader Cull (export_model.shader_cull) to models exported before the exporters recorded it, matching parts to the game's materials by name. |
| [`patch_tmp_gradients.py`](patch_tmp_gradients.py) | Add TMP vertex colour gradients to already dumped UI JSON (local only). |
| [`plan_catalog_displays.py`](plan_catalog_displays.py) | Plan every projectile display GUID in game.json that no display plan has yet, from the game's own catalog (catalog_guids.json, else catalog_prefabs.json |
| [`prepare_coop_ui.py`](prepare_coop_ui.py) | Prepare original co-op layouts for the shared map renderer; no network operations. |
| [`preview_model.py`](preview_model.py) | Quick software preview of an exported model (painter's algorithm, per-triangle texture colour). |
| [`print_ui_tree.py`](print_ui_tree.py) | Print a dump_ui_prefab.py JSON tree compactly |
| [`prune_achievements.py`](prune_achievements.py) | Remove obsolete generated achievement assets, preserving shared art and user files. |
| [`rebuild_shader_glsl.py`](rebuild_shader_glsl.py) | Regenerate local GLSL from its retained original shader disassembly; no game-state writes. |
| [`reexport_all_clips.py`](reexport_all_clips.py) | Re-export every tower model with all of its clips (export_animated.py + compact_anims.py now keep the placement, idle-variant, attack-idle and upgrade clips the game's MonkeyAnimationController plays, not just idle and attack). |
| [`reexport_skinned.py`](reexport_skinned.py) | Re-export the models that have one-bone-per-vertex skinned parts. |
| [`repack_anims.py`](repack_anims.py) | Recompress fast-packed animations without changing their format or payload. |
| [`resolve_ui_sprites.py`](resolve_ui_sprites.py) | Resolve missing scene sprite references from a separate bundle, keeping peak RAM low. |
| [`share_model_textures.py`](share_model_textures.py) | Share byte-identical model textures through existing relative metadata paths. |
| [`skin_anims.py`](skin_anims.py) | Animated models stored as skinning data (skin.bin) instead of baked vertex frames. |
| [`sort_heroes.py`](sort_heroes.py) | Order ui/runtime/hero_shop.json like HeroUpgradeDetails.SortHerosByUnlockLevel. |
| [`sprite_resolver.py`](sprite_resolver.py) | Resolve late-bound sprites by Unity's render-data key, never by display name. |
| [`static_model_batch.py`](static_model_batch.py) | Reuse the loaded bundle when exporting successive static projectile meshes. |
| [`summarize_map_mechanics.py`](summarize_map_mechanics.py) | Readable overview of every map's mechanics.json |
| [`word_anims.py`](word_anims.py) | Exact word prediction + Zstandard for quantized animation storage. |
