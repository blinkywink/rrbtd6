#!/usr/bin/env bash
# Original frontend scenes, runtime controller data and hero appearance metadata.
# Proprietary outputs stay under local_extracted/.
set -euo pipefail
GAME="${1:?usage: export_frontend.sh GAME}"
PY="${PY:-tools/.venv/Scripts/python.exe}"
SCENES=(QuestBrowserUI TowerGiftBoxUI GameEventsUI LinkFriendsUI TeamsBrowserUI PerksUI CoopQuickMatchUI CoopJoinMatchUI MonkeySelectUI HeroDetailsUI KnowledgeMainUI KnowledgeSkillTreeUI PlaySocialUI
        PowersSelectUI AchievementsUI StoreUI TrophyStoreUI LegendsSelectUI MailboxUI)
SCENE_ARGS=()
for scene in "${SCENES[@]}"; do
  SCENE_ARGS+=("scenefile:Assets/UI - New/Scenes/$scene.unity=$scene")
done
$PY tools/assets/dump_ui_prefab.py "$GAME" local_extracted/ui/prefabs "${SCENE_ARGS[@]}" "scenefile:Assets/Main/Scenes/LoadingUI.unity=LoadingUI" "Assets/UI - New/Popups/Prefabs/BloonsMonkeyTalesPopup.prefab"
mapfile -t BACKGROUNDS < <($PY -c "import json; [print(k) for k in json.load(open('local_extracted/all_container_paths.json')) if '/Heroes/HeroBackgrounds/' in k and k.endswith('.prefab')]")
RRBTD6_UI_BUNDLES='*_monoscripts_*.bundle;asset_references_assets_all_*.bundle' \
  $PY tools/assets/dump_ui_prefab.py "$GAME" local_extracted/ui/prefabs "${BACKGROUNDS[@]}"
$PY tools/assets/export_ui_runtime.py "$GAME" local_extracted/ui/runtime MainMenuUi LoadingUI "${SCENES[@]}"
$PY tools/assets/catalog_audio.py "$GAME/BloonsTD6_Data/StreamingAssets/aa/catalog.bin" local_extracted/catalog_sprites.json png
$PY tools/assets/index_sprite_guids.py "$GAME" local_extracted/catalog_sprite_paths.json
$PY tools/assets/export_frontend_data.py "$GAME" local_extracted --images
$PY tools/assets/export_rank_info.py "$GAME" local_extracted
$PY -I tools/assets/sort_heroes.py local_extracted
$PY tools/assets/export_power_portraits.py "$GAME" local_extracted
$PY tools/assets/export_knowledge_menu.py local_extracted
$PY tools/assets/export_upgrade_xp.py local_extracted
$PY tools/assets/export_legends_menu.py local_extracted
$PY tools/assets/export_social_menu.py local_extracted

$PY tools/assets/dump_ui_prefab.py "$GAME" local_extracted/ui/prefabs \
  "field:StoreScreen/StoreScreen.productButtonSmallPrefab=StoreProductSmall" \
  "field:StoreScreen/StoreScreen.productButtonLargePrefab=StoreProductLarge" \
  "field:TrophyStoreScreen/TrophyStoreScreen.trophyPrefab=TrophyStoreCard" \
  "Assets/UI - New/Prefabs/Store/Loot/SmallBaseLootItem.prefab" \
  "Assets/UI - New/Prefabs/Store/Loot/SmallInstaTowerLootItem.prefab"
$PY tools/assets/export_store_menu.py "$GAME" local_extracted
$PY tools/assets/export_menu_catalogs.py "$GAME" local_extracted --images
$PY tools/assets/export_menu_chrome.py "$GAME" local_extracted
$PY tools/assets/export_model.py "$GAME" local_extracted/models/mainmenu scene ui_scenes_all_ \
  cab:CAB-613509f63417784aa3960cd4dc66a279
$PY tools/assets/export_home_animations.py "$GAME" local_extracted

$PY tools/assets/export_home_vegetation.py "$GAME" local_extracted
$PY tools/assets/export_home_interactions.py "$GAME" local_extracted
$PY tools/assets/export_home_buttons.py "$GAME" local_extracted
