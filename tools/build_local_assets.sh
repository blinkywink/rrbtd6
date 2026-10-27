#!/usr/bin/env bash
# Rebuild everything the playable prototype loads, from your own BTD6 install,
# into local_extracted/ (git-ignored, never redistribute). Re-run after a game update.
set -euo pipefail
cd "$(dirname "$0")/.."
GAME="${BTD6_DIR:-C:/Program Files/Epic Games/BloonsTD6}"
PY=tools/.venv/Scripts/python.exe
AA="$GAME/BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full"
META="$GAME/BloonsTD6_Data/il2cpp_data/Metadata/global-metadata.dat"
mkdir -p research/dumps local_extracted

echo "== metadata, method addresses, field offsets"
python -I tools/dotnet_meta/dotnet_meta.py "$GAME/MelonLoader/Il2CppAssemblies/Assembly-CSharp.dll" \
  research/dumps/Assembly-CSharp.txt --json research/dumps/Assembly-CSharp.jsonl
python -I tools/il2cpp/il2cpp_meta.py "$META" "$GAME/GameAssembly.dll" research/dumps/methods-all.jsonl "*"
python -I tools/il2cpp/field_offsets.py "$META" "$GAME/GameAssembly.dll" research/dumps/field_offsets.json
$PY tools/il2cpp/setter_schema.py "$GAME/GameAssembly.dll" research/dumps/methods-all.jsonl research/dumps/setter_schemas.json

echo "== sim_model (all tower/bloon/upgrade data)"
python -I tools/unityfs/unityfs.py extract "$(ls "$AA"/game_model_assets_all_*.bundle)" local_extracted/game_model
$PY - <<'EOF'
# Pull the sim_model TextAsset out of the extracted SerializedFile.
import glob, UnityPy
env = UnityPy.load(glob.glob("local_extracted/game_model/CAB-*")[0])
for o in env.objects:
    if o.type.name == "TextAsset":
        t = o.read()
        open(f"local_extracted/game_model/{t.m_Name}.bytes", "wb").write(t.m_Script.encode("utf-8", "surrogateescape") if isinstance(t.m_Script, str) else bytes(t.m_Script))
EOF
$PY tools/sim_model/decode.py "$GAME/GameAssembly.dll" research/dumps/methods-all.jsonl \
  research/dumps/setter_schemas.json local_extracted/game_model/sim_model.bytes local_extracted/decoded
python -I tools/sim_model/name_records.py local_extracted/decoded research/dumps/field_offsets.json research/dumps/Assembly-CSharp.jsonl

echo "== rounds, modes, map, sprites, 3D models"
$PY tools/assets/export_rounds.py "$GAME" local_extracted/data
$PY - "$AA" <<'EOF'
# CAB name -> bundle index (resolves cross-bundle references).
import glob, json, os, struct, sys
sys.path.insert(0, "tools/unityfs")
import unityfs
idx = {}
for b in glob.glob(os.path.join(sys.argv[1], "*.bundle")):
    d = open(b, "rb").read(4096)
    sig, p = unityfs.cstr(d, 0); p += 4; _, p = unityfs.cstr(d, p); _, p = unityfs.cstr(d, p)
    _, csz, usz, flags = struct.unpack_from(">qIII", d, p); p += 20; p = (p + 15) & ~15
    if flags & 0x80:
        f = open(b, "rb"); f.seek(-csz, 2); ic = f.read()
    else:
        ic = open(b, "rb").read(p + csz)[p:p + csz]
    info = unityfs.decompress(flags & 0x3F, ic, usz)
    q = 16; nb = struct.unpack_from(">I", info, q)[0]; q += 4 + 10 * nb
    nn = struct.unpack_from(">I", info, q)[0]; q += 4
    for _ in range(nn):
        q += 20; n, q = unityfs.cstr(info, q); idx[n] = os.path.basename(b)
json.dump(idx, open("local_extracted/cab_index.json", "w"))
EOF
# Every map (track per entrySplitter, areas, terrain) and its 3D scene: tools/assets/export_all_maps.sh.
bash tools/assets/export_all_maps.sh
$PY tools/assets/export_sprites.py "$GAME" local_extracted/sprites Bloons MonkeyIcons UiBloonIcons IngameUi
TOWERS=(DartMonkey TackShooter BombShooter SniperMonkey BoomerangMonkey IceMonkey GlueGunner NinjaMonkey WizardMonkey SuperMonkey SpikeFactory BananaFarm MonkeyVillage Druid Alchemist Desperado MortarMonkey DartlingGunner EngineerMonkey HeliPilot MonkeyAce)
$PY tools/assets/export_animated.py "$GAME" local_extracted/models/DartMonkey asset_references_assets_all   "Assets/Monkeys/DartMonkey/Graphics/DartMonkey.prefab"
$PY tools/assets/export_animated.py "$GAME" local_extracted/models/WizardMonkey generated_assets_all "Assets/Generated/Assets/Monkeys/WizardMonkey/Graphics/Wizard.prefab"
$PY tools/assets/export_animated.py "$GAME" local_extracted/models/MonkeyVillage generated_assets_all "Assets/Generated/Assets/Monkeys/MonkeyVillage/graphics/MonkeyVillage.prefab"
$PY tools/assets/export_animated.py "$GAME" local_extracted/models/Desperado generated_assets_all "Assets/Generated/Assets/Monkeys/Desperado/Graphics/000-Desperado.prefab"
for T in TackShooter BombShooter SniperMonkey BoomerangMonkey IceMonkey GlueGunner NinjaMonkey SuperMonkey SpikeFactory BananaFarm Druid Alchemist MortarMonkey DartlingGunner EngineerMonkey; do
  $PY tools/assets/export_animated.py "$GAME" "local_extracted/models/$T" generated_assets_all "Assets/Generated/Assets/Monkeys/$T/Graphics/$T.prefab"
done
python -I tools/assets/catalog_guids.py "$GAME/BloonsTD6_Data/StreamingAssets/aa/catalog.bin" local_extracted/catalog_guids.json
python -I tools/assets/catalog_audio.py "$GAME/BloonsTD6_Data/StreamingAssets/aa/catalog.bin" local_extracted/catalog_audio.json
python -I tools/assets/catalog_audio.py "$GAME/BloonsTD6_Data/StreamingAssets/aa/catalog.bin" local_extracted/catalog_prefabs.json prefab
$PY tools/assets/build_display_plan.py "$GAME" "${TOWERS[@]}"
$PY tools/assets/export_tower_tiers.py "$GAME" "${TOWERS[@]}"
$PY tools/assets/export_projectiles.py "$GAME"
$PY tools/assets/export_textures.py "$GAME" local_extracted/ui "Assets/Monkeys/Materials/Range.png" \
  "Assets/Icons/PlayerIcon/HandCursorDefault.png" "Assets/Icons/PlayerIcon/HandCursorDownDefaultSprite.png"
$PY tools/assets/export_bloon_displays.py "$GAME" local_extracted/bloon_displays
$PY tools/assets/export_effect_sprites.py "$GAME" "Assets/Bloons/Graphics/Effects/Pop.prefab" local_extracted/effects/pop

echo "== in-game UI: prefab layouts, all sprite atlases (+ 9-slice borders), TMP fonts, icons, portraits"
$PY tools/assets/export_sprites.py "$GAME" local_extracted/sprites
$PY tools/assets/export_font.py "$GAME" localization_assets_all Btd6FontTitleSDF "Luckiest Guy" local_extracted/fonts/title
$PY tools/assets/export_font.py "$GAME" localization_assets_all Btd6FontBodySDF "Source Sans Pro" local_extracted/fonts/body
for T in "${TOWERS[@]}"; do
  ICON_DIR=$T; [ "$T" = Druid ] && ICON_DIR=DruidMonkey; [ "$T" = DartlingGunner ] && ICON_DIR=DartlingMonkey
  mapfile -t ICONS < <($PY -c "import json; [print(p) for p in json.load(open('local_extracted/all_container_paths.json')) if p.startswith('Assets/ResizedImages/UI/UpgradeIcons/$ICON_DIR/')]")
  $PY tools/assets/export_textures.py "$GAME" local_extracted/ui/upgrades "${ICONS[@]}"
  # Icons named by tier code (Desperado: 100UpgradeIcon.png) get the tower name in front.
  for f in local_extracted/ui/upgrades/[0-9][0-9][0-9]UpgradeIcon.png; do
    [ -e "$f" ] && mv "$f" "local_extracted/ui/upgrades/$T$(basename "$f")"
  done
  # Portrait folders and file names vary in case (Dartmonkeys/030-Dartmonkey.png).
  mapfile -t PORTRAITS < <($PY -c "
import json, os
ps = [p for p in json.load(open('local_extracted/all_container_paths.json')) if p.startswith('Assets/ResizedImages/MonkeyPortraits/')]
dirs = {os.path.dirname(p) for p in ps if p.lower().endswith('-$T.png'.lower())}
[print(p) for p in ps if os.path.dirname(p) in dirs]")
  $PY tools/assets/export_textures.py "$GAME" "local_extracted/ui/portraits/$T" "${PORTRAITS[@]}"
done
P="Assets/UI - New/InGameUi/Prefabs"
$PY tools/assets/dump_ui_prefab.py "$GAME" local_extracted/ui/prefabs \
  "$P/MainHudLeftAlign.prefab" "$P/MainHudRightAlign.prefab" "$P/RightPanel.prefab" "$P/TowerSelectionMenu.prefab" \
  "$P/TowerSelectionMenuPrefabs/TSMThemeDefault.prefab" "pid:6659321379417877030=TowerPurchaseButton" \
  "$P/TowerSelectionMenuPrefabs/TSMThemeMortarMonkey.prefab" "$P/TowerSelectionMenuPrefabs/TSMThemeSelectPointInput.prefab"
# Every other upgrade-panel theme (TowerModel.towerSelectionMenuThemeId: Camo, AmbidextrousRangs, AmmoSwap...).
mapfile -t THEMES < <($PY -c "
import json
[print(p) for p in json.load(open('local_extracted/all_container_paths.json')) if '/TowerSelectionMenuPrefabs/TSMTheme' in p and p.endswith('.prefab')]" | tr -d '\r')
$PY tools/assets/dump_ui_prefab.py "$GAME" local_extracted/ui/prefabs "${THEMES[@]}"

echo "== menus: pause + upgrades scenes, common back button / backdrop, loose sprites, tower panels, English text"
$PY tools/assets/dump_ui_prefab.py "$GAME" local_extracted/ui/prefabs \
  "scene:PauseMenuPanel=PauseScreen" "scene:UpgradeScreen=UpgradeScreen" \
  "scene:ForegroundScreen=CommonForeground" "scene:CommonBackground=CommonBackground" \
  "field:UpgradeScreen/UpgradeScreen.upgradePrefab=UpgradeDetails" \
  "field:UpgradeScreen/UpgradeScreen.upgradePopupPrefab=UpgradePopup" \
  "field:UpgradeScreen/UpgradeScreen.upgradePopupLeftPrefab=UpgradePopupLeft"
$PY tools/assets/export_bundle_sprites.py "$GAME" local_extracted/sprites/_loose "ui_*.bundle" \
  CircleUi ControlBtn GlowUi GlowyStar GodRay GodRaySmallUi LightGradientWhite PerksBtn ResetIcon SmallPanelWhite \
  SpeechBubblePoint SpeechBubbleSquare TowerContainerT5Bottom TowerContainerT5Top YellowRibbonShop GiftboxIconSmall \
  TowerContainerMilitaryLarge TowerContainerHeroLarge TowerContainerParagonLarge
B="Assets/ResizedImages/UI/BigPanels"
$PY tools/assets/export_textures.py "$GAME" local_extracted/ui/panels \
  "$B/TowerContainerPrimary.png" "$B/TowerContainerPrimaryLarge.png" "$B/TowerContainerMilitary.png" \
  "$B/TowerContainerMagic.png" "$B/TowerContainerMAgicLarge.png" "$B/TowerContainerSupport.png" \
  "$B/TowerContainerSupportLarge.png" "$B/TowerContainerHero.png"
$PY tools/assets/export_text.py "$GAME" local_extracted/text "localization_assets_all_*.bundle" \
  "Assets/Data/Localization/Locs/English/English.xml"

ROOT=$(tail -c 4 local_extracted/game_model/sim_model.bytes | python -I -c "import sys,struct;print(struct.unpack('<i',sys.stdin.buffer.read())[0])")
python -I tools/assets/build_game_data.py local_extracted tutorial "$ROOT"
echo "== sub-tower models (Engineer sentries) and air units (Heli, Ace) named in game.json, pad / runway base models"
$PY tools/assets/export_tower_assets.py "$GAME" EngineerMonkey HeliPilot MonkeyAce BeastHandler Skywarden MonkeySub MonkeyBuccaneer Mermonkey
echo "== every upgrade / ability icon not exported yet (hero ability icons, per-tower copies of shared names)"
$PY tools/assets/export_missing_upgrade_icons.py "$GAME"
echo "== audio: every sound clip and music track, decoded to WAV; the jukebox track list"
$PY tools/assets/export_audio.py "$GAME"
$PY tools/assets/export_jukebox.py "$GAME"
echo "== MOAB-class 3D models and damage states"
$PY tools/assets/export_bloon_models.py "$GAME"
echo "== shrink baked animations (idle + attack clips, quantized)"
$PY tools/assets/compact_anims.py local_extracted/models
echo "== done. Play with: cargo run -p btd6-game --release"

# Native music browser and its external track row.
$PY tools/assets/dump_ui_prefab.py "$GAME" local_extracted/ui/prefabs \
  "scenefile:Assets/UI - New/Scenes/JukeboxUI.unity=JukeboxUI" \
  "field:JukeboxScreen/JukeBoxScreen.trackItemPrefab=JukeboxTrack"
$PY tools/assets/export_ui_runtime.py "$GAME" local_extracted/ui/runtime JukeboxUI
# Daily chest calendar and the LootThemes theme-2 reward items.
$PY tools/assets/dump_ui_prefab.py "$GAME" local_extracted/ui/prefabs \
  "scenefile:Assets/UI - New/Scenes/DailyRewardUI.unity=DailyRewardUI" \
  "Assets/UI - New/Prefabs/Store/Loot/MediumBaseLootItem.prefab" \
  "Assets/UI - New/Prefabs/Store/Loot/MediumInstaTowerLootItem.prefab"
$PY tools/assets/export_ui_runtime.py "$GAME" local_extracted/ui/runtime DailyRewardUI
$PY tools/assets/export_menu_chrome.py "$GAME" local_extracted

# Lossless storage, read by the current client (preserves every baked frame).
$PY tools/assets/pack_anims.py local_extracted/models
