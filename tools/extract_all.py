"""Extract everything the game loads from your own BTD6 install, in one command, without bash.

Runs every exporter in dependency order with this Python interpreter, from ROOT (a folder
holding tools/; the output goes to ROOT/local_extracted and ROOT/research/dumps). Finished
steps are recorded in local_extracted/.extract_state.json and skipped on the next run unless
the game build changed (everything re-runs) or the step's STEP_VERSIONS entry was raised. Output is proprietary game data: keep it local, never
redistribute.

Usage: python -I tools/extract_all.py --game "<BTD6 folder>" [--root DIR] [--only STEP ...] [--force]
"""
import argparse
import glob
import hashlib
import json
import os
import struct
import subprocess
import sys
import time
from pathlib import Path

TOWERS = ["DartMonkey", "TackShooter", "BombShooter", "SniperMonkey", "BoomerangMonkey", "IceMonkey", "GlueGunner",
          "NinjaMonkey", "WizardMonkey", "SuperMonkey", "SpikeFactory", "BananaFarm", "MonkeyVillage", "Druid",
          "Alchemist", "Desperado", "MortarMonkey", "DartlingGunner", "EngineerMonkey", "HeliPilot", "MonkeyAce"]
FRONTEND_SCENES = ["QuestBrowserUI", "TowerGiftBoxUI", "GameEventsUI", "LinkFriendsUI", "TeamsBrowserUI", "PerksUI",
                   "CoopQuickMatchUI", "CoopJoinMatchUI", "MonkeySelectUI", "HeroDetailsUI", "KnowledgeMainUI",
                   "KnowledgeSkillTreeUI", "PlaySocialUI", "PowersSelectUI", "AchievementsUI", "StoreUI",
                   "TrophyStoreUI", "MailboxUI", "MapSelectUI"]
CORE_SCENES = {
    "MainMenuUI": "Assets/UI - New/MainMenuUi.unity",
    **{name: f"Assets/UI - New/Scenes/{name}.unity" for name in
       ["DifficultySelectUI", "ModeSelectUI", "SettingsUI", "ExtraSettingsUI", "AccessibilitySettingsUI", "HotkeysUI"]},
    **{name: f"Assets/UI - New/Scenes/EndGame/{name}.unity" for name in ["VictoryUI", "DefeatUI"]},
}


class Ctx:
    def __init__(self, game, root):
        self.game, self.root = Path(game), Path(root)
        self.aa = self.game / "BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full"
        self.meta = self.game / "BloonsTD6_Data/il2cpp_data/Metadata/global-metadata.dat"
        self.out = self.root / "local_extracted"

    def py(self, script, *args, env=None):
        """Run one tools/ script with this interpreter (isolated mode), from ROOT."""
        cmd = [sys.executable, "-I", str(self.root / script), *map(str, args)]
        # unitypy_lowmem (via sitecustomize) streams large bundles through a disk cache.
        full_env = dict(os.environ, PYTHONIOENCODING="utf-8", RRBTD6_TOOLS=str(self.root / "tools"),
                        RRBTD6_BUNDLE_CACHE=str(self.out / "_bundle_cache"), **(env or {}))
        print("  $", " ".join(Path(c).name if i < 2 else c for i, c in enumerate(cmd)), flush=True)
        subprocess.run(cmd, cwd=self.root, env=full_env, check=True)

    def bundle(self, pattern):
        found = sorted(glob.glob(str(self.aa / pattern)))
        if not found:
            raise FileNotFoundError(f"no bundle matches {pattern}")
        return found[0]

    def containers(self):
        return json.loads((self.out / "all_container_paths.json").read_text(encoding="utf-8"))


# ---- steps ---------------------------------------------------------------------------------

def metadata(c):
    dumps = c.root / "research/dumps"
    dumps.mkdir(parents=True, exist_ok=True)
    assembly = c.game / "MelonLoader/Il2CppAssemblies/Assembly-CSharp.dll"
    if not assembly.exists():
        raise FileNotFoundError("MelonLoader's Il2CppAssemblies/Assembly-CSharp.dll is missing: "
                                "install MelonLoader and start BTD6 once.")
    c.py("tools/dotnet_meta/dotnet_meta.py", assembly, dumps / "Assembly-CSharp.txt", "--json", dumps / "Assembly-CSharp.jsonl")
    c.py("tools/il2cpp/il2cpp_meta.py", c.meta, c.game / "GameAssembly.dll", dumps / "methods-all.jsonl", "*")
    c.py("tools/il2cpp/field_offsets.py", c.meta, c.game / "GameAssembly.dll", dumps / "field_offsets.json")
    c.py("tools/il2cpp/setter_schema.py", c.game / "GameAssembly.dll", dumps / "methods-all.jsonl", dumps / "setter_schemas.json")


def sim_model(c):
    dumps = c.root / "research/dumps"
    gm = c.out / "game_model"
    c.py("tools/unityfs/unityfs.py", "extract", c.bundle("game_model_assets_all_*.bundle"), gm)
    import UnityPy
    env = UnityPy.load(glob.glob(str(gm / "CAB-*"))[0])
    for o in env.objects:
        if o.type.name == "TextAsset":
            t = o.read()
            data = t.m_Script.encode("utf-8", "surrogateescape") if isinstance(t.m_Script, str) else bytes(t.m_Script)
            (gm / f"{t.m_Name}.bytes").write_bytes(data)
    c.py("tools/sim_model/decode.py", c.game / "GameAssembly.dll", dumps / "methods-all.jsonl",
         dumps / "setter_schemas.json", gm / "sim_model.bytes", c.out / "decoded")
    c.py("tools/sim_model/name_records.py", c.out / "decoded", dumps / "field_offsets.json", dumps / "Assembly-CSharp.jsonl")


def cab_index(c):
    """CAB name -> bundle file (resolves cross-bundle references)."""
    sys.path.insert(0, str(c.root / "tools/unityfs"))
    import unityfs
    idx = {}
    for b in glob.glob(str(c.aa / "*.bundle")):
        with open(b, "rb") as f:
            d = f.read(4096)
            sig, p = unityfs.cstr(d, 0); p += 4; _, p = unityfs.cstr(d, p); _, p = unityfs.cstr(d, p)
            _, csz, usz, flags = struct.unpack_from(">qIII", d, p); p += 20; p = (p + 15) & ~15
            if flags & 0x80:
                f.seek(-csz, 2); ic = f.read()
            else:
                f.seek(0); ic = f.read(p + csz)[p:p + csz]
        info = unityfs.decompress(flags & 0x3F, ic, usz)
        q = 16; nb = struct.unpack_from(">I", info, q)[0]; q += 4 + 10 * nb
        nn = struct.unpack_from(">I", info, q)[0]; q += 4
        for _ in range(nn):
            q += 20; n, q = unityfs.cstr(info, q); idx[n] = os.path.basename(b)
    (c.out / "cab_index.json").write_text(json.dumps(idx), encoding="utf-8")


def container_index(c):
    """Addressables container path -> bundle file (what build_display_plan.py builds when missing)."""
    c.py("tools/assets/build_display_plan.py", c.game)


def rounds_maps(c):
    c.py("tools/assets/export_rounds.py", c.game, c.out / "data")
    keys = sorted({Path(b).name[4:].split("_scenes_all_")[0] for b in glob.glob(str(c.aa / "map_*_scenes_all_*.bundle"))} - {"baseeditormap"})
    for k in keys:
        if (c.out / f"maps/{k}/map.json").exists() and (c.out / f"models/map_{k}/meta.json").exists():
            continue
        print("== map", k, flush=True)
        c.py("tools/assets/export_map.py", c.game, k, c.out / "maps")
        c.py("tools/assets/export_model.py", c.game, c.out / f"models/map_{k}", "scene", f"map_{k}")


def catalogs(c):
    cat = c.game / "BloonsTD6_Data/StreamingAssets/aa/catalog.bin"
    c.py("tools/assets/catalog_guids.py", cat, c.out / "catalog_guids.json")
    c.py("tools/assets/catalog_audio.py", cat, c.out / "catalog_audio.json")
    c.py("tools/assets/catalog_audio.py", cat, c.out / "catalog_prefabs.json", "prefab")
    c.py("tools/assets/catalog_audio.py", cat, c.out / "catalog_sprites.json", "png")
    c.py("tools/assets/index_sprite_guids.py", c.game, c.out / "catalog_sprite_paths.json")


def sprites(c):
    c.py("tools/assets/export_sprites.py", c.game, c.out / "sprites", "Bloons", "MonkeyIcons", "UiBloonIcons", "IngameUi")
    c.py("tools/assets/export_sprites.py", c.game, c.out / "sprites")
    c.py("tools/assets/export_font.py", c.game, "localization_assets_all", "Btd6FontTitleSDF", "Luckiest Guy", c.out / "fonts/title")
    c.py("tools/assets/export_font.py", c.game, "localization_assets_all", "Btd6FontBodySDF", "Source Sans Pro", c.out / "fonts/body")
    c.py("tools/assets/export_text.py", c.game, c.out / "text", "localization_assets_all_*.bundle",
         "Assets/Data/Localization/Locs/English/English.xml")


def mapset(c):
    """MapSet (map ids, tiers, music); matches map images, so it runs after sprites."""
    c.py("tools/assets/export_mapset.py", c.game, c.out / "data/mapset.json")


def tower_models(c):
    c.py("tools/assets/export_animated.py", c.game, c.out / "models/DartMonkey", "asset_references_assets_all",
         "Assets/Monkeys/DartMonkey/Graphics/DartMonkey.prefab")
    special = {"WizardMonkey": "Wizard", "MonkeyVillage": "MonkeyVillage", "Desperado": "000-Desperado"}
    folder = {"MonkeyVillage": "graphics"}
    for t in ["WizardMonkey", "MonkeyVillage", "Desperado", "TackShooter", "BombShooter", "SniperMonkey", "BoomerangMonkey",
              "IceMonkey", "GlueGunner", "NinjaMonkey", "SuperMonkey", "SpikeFactory", "BananaFarm", "Druid", "Alchemist",
              "MortarMonkey", "DartlingGunner", "EngineerMonkey"]:
        c.py("tools/assets/export_animated.py", c.game, c.out / f"models/{t}", "generated_assets_all",
             f"Assets/Generated/Assets/Monkeys/{t}/{folder.get(t, 'Graphics')}/{special.get(t, t)}.prefab")
    c.py("tools/assets/build_display_plan.py", c.game, *TOWERS)
    c.py("tools/assets/export_tower_tiers.py", c.game, *TOWERS)
    c.py("tools/assets/export_projectiles.py", c.game)


def misc_art(c):
    c.py("tools/assets/export_textures.py", c.game, c.out / "ui", "Assets/Monkeys/Materials/Range.png",
         "Assets/Icons/PlayerIcon/HandCursorDefault.png", "Assets/Icons/PlayerIcon/HandCursorDownDefaultSprite.png")
    c.py("tools/assets/export_cursor.py", c.game)
    c.py("tools/assets/export_bloon_displays.py", c.game, c.out / "bloon_displays")
    c.py("tools/assets/export_effect_sprites.py", c.game, "Assets/Bloons/Graphics/Effects/Pop.prefab", c.out / "effects/pop")


def upgrade_art(c):
    paths = c.containers()
    for t in TOWERS:
        icon_dir = {"Druid": "DruidMonkey", "DartlingGunner": "DartlingMonkey"}.get(t, t)
        icons = [p for p in paths if p.startswith(f"Assets/ResizedImages/UI/UpgradeIcons/{icon_dir}/")]
        if icons:
            c.py("tools/assets/export_textures.py", c.game, c.out / "ui/upgrades", *icons)
        # Icons named by tier code (Desperado: 100UpgradeIcon.png) get the tower name in front.
        for f in glob.glob(str(c.out / "ui/upgrades/[0-9][0-9][0-9]UpgradeIcon.png")):
            os.replace(f, Path(f).with_name(t + Path(f).name))
        # Portrait folders and file names vary in case (Dartmonkeys/030-Dartmonkey.png).
        ps = [p for p in paths if p.startswith("Assets/ResizedImages/MonkeyPortraits/")]
        dirs = {os.path.dirname(p) for p in ps if p.lower().endswith(f"-{t}.png".lower())}
        portraits = [p for p in ps if os.path.dirname(p) in dirs]
        if portraits:
            c.py("tools/assets/export_textures.py", c.game, c.out / f"ui/portraits/{t}", *portraits)


def ingame_ui(c):
    p = "Assets/UI - New/InGameUi/Prefabs"
    c.py("tools/assets/dump_ui_prefab.py", c.game, c.out / "ui/prefabs",
         f"{p}/MainHudLeftAlign.prefab", f"{p}/MainHudRightAlign.prefab", f"{p}/RightPanel.prefab", f"{p}/TowerSelectionMenu.prefab",
         f"{p}/TowerSelectionMenuPrefabs/TSMThemeDefault.prefab", "pid:6659321379417877030=TowerPurchaseButton",
         f"{p}/TowerSelectionMenuPrefabs/TSMThemeMortarMonkey.prefab", f"{p}/TowerSelectionMenuPrefabs/TSMThemeSelectPointInput.prefab")
    themes = [x for x in c.containers() if "/TowerSelectionMenuPrefabs/TSMTheme" in x and x.endswith(".prefab")]
    c.py("tools/assets/dump_ui_prefab.py", c.game, c.out / "ui/prefabs", *themes)
    c.py("tools/assets/dump_ui_prefab.py", c.game, c.out / "ui/prefabs",
         "scene:PauseMenuPanel=PauseScreen", "scene:UpgradeScreen=UpgradeScreen",
         "scene:ForegroundScreen=CommonForeground", "scene:CommonBackground=CommonBackground",
         "field:UpgradeScreen/UpgradeScreen.upgradePrefab=UpgradeDetails",
         "field:UpgradeScreen/UpgradeScreen.upgradePopupPrefab=UpgradePopup",
         "field:UpgradeScreen/UpgradeScreen.upgradePopupLeftPrefab=UpgradePopupLeft")
    c.py("tools/assets/export_bundle_sprites.py", c.game, c.out / "sprites/_loose", "ui_*.bundle",
         "CircleUi", "ControlBtn", "GlowUi", "GlowyStar", "GodRay", "GodRaySmallUi", "LightGradientWhite", "PerksBtn",
         "ResetIcon", "SmallPanelWhite", "SpeechBubblePoint", "SpeechBubbleSquare", "TowerContainerT5Bottom",
         "TowerContainerT5Top", "YellowRibbonShop", "GiftboxIconSmall", "TowerContainerMilitaryLarge",
         "TowerContainerHeroLarge", "TowerContainerParagonLarge")
    b = "Assets/ResizedImages/UI/BigPanels"
    c.py("tools/assets/export_textures.py", c.game, c.out / "ui/panels",
         *[f"{b}/{n}.png" for n in ["TowerContainerPrimary", "TowerContainerPrimaryLarge", "TowerContainerMilitary",
                                     "TowerContainerMagic", "TowerContainerMAgicLarge", "TowerContainerSupport",
                                     "TowerContainerSupportLarge", "TowerContainerHero"]])


def game_data(c):
    root_index = struct.unpack("<i", (c.out / "game_model/sim_model.bytes").read_bytes()[-4:])[0]
    c.py("tools/assets/build_game_data.py", c.out, "tutorial", root_index)
    c.py("tools/assets/export_missing_upgrade_icons.py", c.game)


def extended_towers(c):
    # Include every current tower/hero and all paragon, air-unit and pet displays.
    # A short hand-maintained list omitted most paragons and all hero models.
    game = json.loads((c.out / "game/game.json").read_text(encoding="utf-8"))
    towers = game["tower_tiers"]
    c.py("tools/assets/export_tower_assets.py", c.game, *towers)
    heroes = [name for name, definition in towers.items() if definition.get("hero") is not None]
    if heroes:
        c.py("tools/assets/export_hero_portraits.py", c.game, *heroes)


def core_menus(c):
    prefabs = c.out / "ui/prefabs"
    c.py("tools/assets/dump_ui_prefab.py", c.game, prefabs,
         *[f"scenefile:{path}={name}" for name, path in CORE_SCENES.items()],
         "Assets/UI - New/Popups/Prefabs/CommonPopup.prefab",
         "Assets/UI - New/Popups/Prefabs/ParagonConfirmationPopup.prefab",
         "Assets/UI - New/InGameUi/Prefabs/SandboxBloonsMenu.prefab",
         "Assets/UI - New/InGameUi/Prefabs/BuffManager.prefab",
         "Assets/UI - New/InGameUi/Prefabs/GeraldoShop.prefab",
         "field:SandboxBloonsMenu/BloonMenu.spawnBloonButtonPrefab=SpawnBloonButton")
    c.py("tools/assets/export_ui_runtime.py", c.game, c.out / "ui/runtime",
         "MainMenuUi", *[name for name in CORE_SCENES if name != "MainMenuUI"])
    c.py("tools/assets/resolve_ui_sprites.py", c.game, c.out, "asset_references_assets_all_*.bundle",
         *CORE_SCENES, "CommonPopup", "ParagonConfirmationPopup", "SandboxBloonsMenu", "SpawnBloonButton", "BuffManager", "GeraldoShop")


def particles(c):
    game = json.loads((c.out / "game/game.json").read_text(encoding="utf-8"))
    names = {"UpgradeEffect", "UpgradeEffectTier5", "HeroUpgradeEffect", "HeroUpgradeEffectLv20", "SellEffect"}

    def visit(value):
        if isinstance(value, dict):
            for key, item in value.items():
                if key == "fx" or key.endswith("_fx"):
                    effect = item[0] if isinstance(item, list) and item else item
                    if isinstance(effect, str) and effect:
                        names.add(effect)
                visit(item)
        elif isinstance(value, list):
            for item in value:
                visit(item)

    visit(game)
    index = c.containers()
    paths = []
    for name in sorted(names):
        matches = [path for path in index if path.endswith(f"/{name}.prefab")]
        if len(matches) != 1:
            raise ValueError(f"Effect {name}: expected one original prefab, found {len(matches)}")
        paths.append(matches[0])
    c.py("tools/assets/export_particles.py", c.game, c.out / "effects", *paths)
    # Separate processes bound UnityPy memory across maps and tower bundles.
    for map_json in sorted((c.out / "maps").glob("*/map.json")):
        c.py("tools/assets/export_particles.py", c.game, c.out / "effects", f"scene:{map_json.parent.name}")
    for tower in game["tower_tiers"]:
        c.py("tools/assets/export_particles.py", c.game, c.out / "effects", f"tiers:{tower}")
    c.py("tools/assets/export_particles.py", c.game, c.out / "effects", "projectiles")


def audio(c):
    c.py("tools/assets/export_audio.py", c.game)
    c.py("tools/assets/export_jukebox.py", c.game)


def bloon_models(c):
    c.py("tools/assets/export_bloon_models.py", c.game)
    c.py("tools/assets/compact_anims.py", c.out / "models")


def frontend(c):
    prefabs = c.out / "ui/prefabs"
    c.py("tools/assets/dump_ui_prefab.py", c.game, prefabs,
         *[f"scenefile:Assets/UI - New/Scenes/{s}.unity={s}" for s in FRONTEND_SCENES],
         "scenefile:Assets/Main/Scenes/LoadingUI.unity=LoadingUI", "Assets/UI - New/Popups/Prefabs/BloonsMonkeyTalesPopup.prefab")
    backgrounds = [k for k in c.containers() if "/Heroes/HeroBackgrounds/" in k and k.endswith(".prefab")]
    c.py("tools/assets/dump_ui_prefab.py", c.game, prefabs, *backgrounds,
         env={"RRBTD6_UI_BUNDLES": "*_monoscripts_*.bundle;asset_references_assets_all_*.bundle"})
    c.py("tools/assets/export_ui_runtime.py", c.game, c.out / "ui/runtime", "MainMenuUi", "LoadingUI", *FRONTEND_SCENES)
    c.py("tools/assets/export_frontend_data.py", c.game, c.out, "--images")
    c.py("tools/assets/export_rank_info.py", c.game, c.out)
    c.py("tools/assets/sort_heroes.py", c.out)
    c.py("tools/assets/export_power_portraits.py", c.game, c.out)
    # No Legends menu: the Legends DLC modes aren't supported (DEVIATIONS.md 4).
    for s in ["export_knowledge_menu", "export_upgrade_xp", "export_social_menu"]:
        c.py(f"tools/assets/{s}.py", c.out)
    c.py("tools/assets/dump_ui_prefab.py", c.game, prefabs,
         "field:StoreScreen/StoreScreen.productButtonSmallPrefab=StoreProductSmall",
         "field:StoreScreen/StoreScreen.productButtonLargePrefab=StoreProductLarge",
         "field:TrophyStoreScreen/TrophyStoreScreen.trophyPrefab=TrophyStoreCard",
         "Assets/UI - New/Prefabs/Store/Loot/SmallBaseLootItem.prefab",
         "Assets/UI - New/Prefabs/Store/Loot/SmallInstaTowerLootItem.prefab")
    c.py("tools/assets/export_store_menu.py", c.game, c.out)
    c.py("tools/assets/export_menu_catalogs.py", c.game, c.out, "--images")
    c.py("tools/assets/export_menu_chrome.py", c.game, c.out)


def startup(c):
    """Splash, initial loading and title screens, account management and the version line."""
    prefabs = c.out / "ui/prefabs"
    boot = "*_monoscripts_*.bundle;initial_loading_ui_scenes_all_*.bundle;sprite_atlases_assets_all_*.bundle;localization_assets_all_*.bundle;asset_references_assets_all_*.bundle"
    c.py("tools/assets/dump_ui_prefab.py", c.game, prefabs,
         "scenefile:Assets/Main/Scenes/NinjaKiwiSplashUI.unity=NinjaKiwiSplashUI",
         "scenefile:Assets/Main/Scenes/InitialLoadingUI.unity=InitialLoadingUI", env={"RRBTD6_UI_BUNDLES": boot})
    c.py("tools/assets/dump_ui_prefab.py", c.game, prefabs,
         "scenefile:Assets/Main/Scenes/TitleScreenUI.unity=TitleScreenUI",
         "scenefile:Assets/Main/Scenes/DataConflictUI.unity=DataConflictUI",
         "scenefile:Assets/UI - New/Scenes/AccountManagementUI.unity=AccountManagementUI",
         "scenefile:Assets/UI - New/Scenes/LinkAccountUI.unity=LinkAccountUI",
         "Assets/UI - New/Popups/Prefabs/LogoutPopup.prefab")
    c.py("tools/assets/export_ui_runtime.py", c.game, c.out / "ui/runtime", "NinjaKiwiSplashUI", "InitialLoadingUI",
         "TitleScreenUI", "DataConflictUI", "AccountManagementUI", "LinkAccountUI",
         env={"RRBTD6_UI_BUNDLES": "*_monoscripts_*.bundle;initial_loading_ui_scenes_all_*.bundle;ui_scenes_all_*.bundle;asset_references_assets_all_*.bundle"})
    c.py("tools/assets/export_bundle_sprites.py", c.game, c.out / "sprites/_loose", "ui_scenes_all_*.bundle", "TitleBg")
    c.py("tools/assets/export_game_version.py", c.game, c.root / "research/dumps/methods-all.jsonl", c.out / "game_version.json")


def home(c):
    c.py("tools/assets/export_model.py", c.game, c.out / "models/mainmenu", "scene", "ui_scenes_all_",
         "cab:CAB-613509f63417784aa3960cd4dc66a279")
    for s in ["export_home_animations", "export_home_vegetation", "export_home_interactions", "export_home_buttons"]:
        c.py(f"tools/assets/{s}.py", c.game, c.out)
    c.py("tools/assets/export_home_events.py", c.game, c.out)


def social(c):
    c.py("tools/assets/export_coop_ui.py", c.game, c.out)
    c.py("tools/assets/export_event_ui.py", c.game, c.out)


def finish(c):
    c.py("tools/assets/patch_tmp_gradients.py", c.game, c.root)
    c.py("tools/assets/rebuild_shader_glsl.py", c.out)


STEPS = [metadata, sim_model, cab_index, catalogs, container_index, rounds_maps, sprites, mapset, tower_models, misc_art, upgrade_art,
         ingame_ui, core_menus, game_data, extended_towers, particles, audio, bloon_models, frontend, startup, home, social, finish]


def install_hook(root):
    """Put tools/sitecustomize.py in this interpreter's site-packages (loads unitypy_lowmem)."""
    import sysconfig
    target = Path(sysconfig.get_paths()["purelib"]) / "sitecustomize.py"
    source = (root / "tools/sitecustomize.py").read_text(encoding="utf-8")
    if not target.exists() or target.read_text(encoding="utf-8") != source:
        atomic_write(target, source)


def atomic_write(path, text):
    """Keep the previous checkpoint/hook intact until its replacement is complete."""
    pending = path.with_name(path.name + ".pending")
    with pending.open("w", encoding="utf-8") as stream:
        stream.write(text)
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(pending, path)


def load_state(path, fp):
    try:
        state = json.loads(path.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError, UnicodeDecodeError):
        state = None
    if not isinstance(state, dict) or state.get("fingerprint") != fp or not isinstance(state.get("done"), dict):
        return {"fingerprint": fp, "done": {}}
    return state


# Raise a step's number when its output must be regenerated for existing installs (a fixed or
# extended exporter). Other tools changes do not make players re-extract.
STEP_VERSIONS = {s.__name__: 1 for s in STEPS}


def fingerprint(c):
    """The game build: an update re-runs every step."""
    h = hashlib.sha256()
    for f in [c.game / "GameAssembly.dll", c.meta]:
        st = f.stat(); h.update(f"{f.name}:{st.st_size}:{int(st.st_mtime)}".encode())
    return h.hexdigest()


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--game", required=True, help="BTD6 install folder (with MelonLoader)")
    ap.add_argument("--root", default=str(Path(__file__).resolve().parents[1]), help="folder holding tools/")
    ap.add_argument("--only", nargs="*", help="run just these steps")
    ap.add_argument("--force", action="store_true", help="ignore recorded progress")
    args = ap.parse_args()
    c = Ctx(args.game, args.root)
    if not (c.game / "GameAssembly.dll").exists():
        sys.exit(f"Not a BTD6 install: {c.game}")
    c.out.mkdir(parents=True, exist_ok=True)
    install_hook(c.root)
    state_file = c.out / ".extract_state.json"
    fp = fingerprint(c)
    state = {"fingerprint": fp, "done": {}} if args.force else load_state(state_file, fp)
    names = [s.__name__ for s in STEPS]
    for n in args.only or []:
        if n not in names:
            sys.exit(f"unknown step {n}; steps: {', '.join(names)}")
    for step in STEPS:
        name = step.__name__
        if args.only and name not in args.only:
            continue
        if not args.only and state["done"].get(name) == STEP_VERSIONS[name]:
            print(f"== {name}: done", flush=True)
            continue
        print(f"== {name}", flush=True)
        began = time.time()
        step(c)
        print(f"== {name}: {time.time() - began:.0f}s", flush=True)
        state["done"][name] = STEP_VERSIONS[name]
        atomic_write(state_file, json.dumps(state))
    print("extraction complete", flush=True)


if __name__ == "__main__":
    main()
