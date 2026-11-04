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
import shutil
import struct
import subprocess
import sys
import threading
import time
from pathlib import Path
from collections import deque
sys.path.insert(0, str(Path(__file__).resolve().parent))
from extraction_progress import Progress, load_history

TOWERS = ["DartMonkey", "TackShooter", "BombShooter", "SniperMonkey", "BoomerangMonkey", "IceMonkey", "GlueGunner",
          "NinjaMonkey", "WizardMonkey", "SuperMonkey", "SpikeFactory", "BananaFarm", "MonkeyVillage", "Druid",
          "Alchemist", "Desperado", "MortarMonkey", "DartlingGunner", "EngineerMonkey", "HeliPilot", "MonkeyAce"]
FRONTEND_SCENES = ["QuestBrowserUI", "TowerGiftBoxUI", "GameEventsUI", "LinkFriendsUI", "TeamsBrowserUI", "PerksUI",
                   "CoopQuickMatchUI", "CoopJoinMatchUI", "MonkeySelectUI", "HeroDetailsUI", "KnowledgeMainUI",
                   "KnowledgeSkillTreeUI", "PlaySocialUI", "PowersSelectUI", "StoreUI",
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
        self.progress = None
        self.log_lock = threading.Lock()

    def py(self, script, *args, env=None, progress=(0., 1.)):
        """Run one tools/ script with this interpreter (isolated mode), from ROOT."""
        cmd = [sys.executable, "-I", str(self.root / script), *map(str, args)]
        # unitypy_lowmem (via sitecustomize) streams large bundles through a disk cache.
        full_env = dict(os.environ, OPENBLAS_NUM_THREADS="1", OMP_NUM_THREADS="1", MKL_NUM_THREADS="1", PYTHONIOENCODING="utf-8", RRBTD6_TOOLS=str(self.root / "tools"),
                        RRBTD6_BUNDLE_CACHE=str(self.out / "_bundle_cache"), **(env or {}))
        print("  Preparing:", Path(script).stem.replace('_', ' '), flush=True)
        began = time.time()
        try:
            self._run(cmd, full_env, progress, script)
        finally:
            # Per-exporter wall time, for finding where setup time goes.
            try:
                with open(self.out / ".extract_profile.jsonl", "a", encoding="utf-8") as log:
                    log.write(json.dumps({"script": script, "args": [str(a) for a in args][1:4],
                                          "seconds": round(time.time() - began, 2)}) + "\n")
            except OSError:
                pass  # Never let the timing log hide the exporter's own result.

    def _run(self, cmd, full_env, progress, script=""):
        if self.progress is None:
            subprocess.run(cmd, cwd=self.root, env=full_env, check=True)
            return
        tail = deque(maxlen=30)
        with subprocess.Popen(cmd, cwd=self.root, env=full_env, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, text=True, encoding="utf-8", errors="replace", bufsize=1) as child:
            for line in child.stdout:
                line = line.rstrip()
                if self.progress.child(line, progress):
                    continue
                tail.append(line)
                # Everything exporters print, for finding failures they catch and carry on from.
                try:
                    with self.log_lock, open(self.out / ".extract.log", "a", encoding="utf-8") as log:
                        log.write(f"{Path(script).stem}: {line}" + chr(10))
                except OSError:
                    pass
                if os.environ.get("RRBTD6_SETUP_VERBOSE") == "1":
                    print(line, flush=True)
            code = child.wait()
        if code:
            print("\n".join(tail), flush=True)
            raise subprocess.CalledProcessError(code, cmd)

    def parallel(self, label, jobs, per_worker):
        """Run independent, repeatable [script, *args, optional env dict] jobs on a worker pool sized by free memory
        (per_worker: measured peak per process plus headroom). A failure stops unstarted jobs;
        those and the failed ones then run again one at a time, in case it was memory (other
        programs can use it up after the pool was sized), and a second failure is raised."""
        from concurrent.futures import ThreadPoolExecutor, as_completed
        from assets.model_batch import worker_count
        if not jobs:
            return
        count = min(len(jobs), worker_count(per_worker=per_worker, max_workers=8),
                    max(1, int(os.environ.get("RRBTD6_SETUP_WORKERS", "8"))))
        print(f"{label}: {count} parallel worker(s), {len(jobs)} job(s)", flush=True)
        stopped = threading.Event()

        def call(job):
            # Each job's own progress says nothing about the group's; finished jobs move the bar.
            if isinstance(job[-1], dict):
                self.py(*job[:-1], env=job[-1], progress=(0., 0.))
            else:
                self.py(*job, progress=(0., 0.))

        def run(job):
            if stopped.is_set():
                return False
            try:
                call(job)
                return True
            except Exception:
                stopped.set()
                return False

        done = 0

        def report():
            if self.progress is not None:
                self.progress.update(done, len(jobs), f"{label}: {done}/{len(jobs)}")

        with ThreadPoolExecutor(max_workers=count) as pool:
            futures = {pool.submit(run, job): job for job in jobs}
            again = []
            for future in as_completed(futures):
                if future.result():
                    done += 1
                    report()
                else:
                    again.append(futures[future])
        if again:
            print(f"{label}: retrying {len(again)} job(s) one at a time", flush=True)
        for job in again:
            call(job)
            done += 1
            report()

    def bundle(self, pattern):
        found = sorted(glob.glob(str(self.aa / pattern)))
        if not found:
            raise FileNotFoundError(f"no bundle matches {pattern}")
        return found[0]

    def containers(self):
        return json.loads((self.out / "all_container_paths.json").read_text(encoding="utf-8"))


# ---- steps ---------------------------------------------------------------------------------

# Budget for an exporter process in a parallel group: a UI dump opening five bundles measured
# 0.77 GiB at peak. Jobs in a group write separate files; what reads them runs after the group.
UI_WORKER = 2 << 30


def metadata(c):
    dumps = c.root / "research/dumps"
    dumps.mkdir(parents=True, exist_ok=True)
    assembly = c.game / "MelonLoader/Il2CppAssemblies/Assembly-CSharp.dll"
    if not assembly.exists():
        raise FileNotFoundError("MelonLoader's Il2CppAssemblies/Assembly-CSharp.dll is missing: "
                                "install MelonLoader and start BTD6 once.")
    c.parallel("Game metadata", [
        ["tools/dotnet_meta/dotnet_meta.py", assembly, dumps / "Assembly-CSharp.txt", "--json", dumps / "Assembly-CSharp.jsonl"],
        ["tools/il2cpp/il2cpp_meta.py", c.meta, c.game / "GameAssembly.dll", dumps / "methods-all.jsonl", "*"],
        ["tools/il2cpp/field_offsets.py", c.meta, c.game / "GameAssembly.dll", dumps / "field_offsets.json"]], UI_WORKER)
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
    """CAB name (lower case) -> bundle file (resolves cross-bundle references), Full and the
    bundles only in Half: tools/assets/cab_index.py, the format its readers expect."""
    c.py("tools/assets/cab_index.py", c.game)


def container_index(c):
    """Addressables container path -> bundle file (what build_display_plan.py builds when missing)."""
    c.py("tools/assets/build_display_plan.py", c.game)


def rounds_maps(c):
    c.py("tools/assets/export_rounds.py", c.game, c.out / "data")
    keys = sorted({Path(b).name[4:].split("_scenes_all_")[0] for b in glob.glob(str(c.aa / "map_*_scenes_all_*.bundle"))} - {"baseeditormap"})
    # One map pair measured 0.16-0.26 GiB peak working set; 1 GiB each leaves ample headroom.
    c.parallel("Maps", [["tools/assets/export_map_pair.py", c.game, k, c.out / "maps", c.out / f"models/map_{k}"] for k in keys
                        if not ((c.out / f"maps/{k}/map.json").exists() and (c.out / f"models/map_{k}/meta.json").exists())], 1 << 30)


def map_mechanics(c):
    """Removeables, map events, clickable items and prop scripts (maps/<key>/mechanics.json)."""
    keys = sorted({Path(b).name[4:].split("_scenes_all_")[0] for b in glob.glob(str(c.aa / "map_*_scenes_all_*.bundle"))})
    # Only playable maps: baseeditormap has no map.json.
    keys = [k for k in keys if (c.out / f"maps/{k}/map.json").exists()]
    c.py("tools/assets/export_map_mechanics.py", c.game, c.out / "maps", *keys)
    c.py("tools/assets/export_map_clips.py", c.game, c.out / "maps", *keys)
    # The co-op divisions' placement areas, the same for every map (Game.coopLayouts).
    c.py("tools/assets/export_coop_areas.py", c.game, c.out / "game/coop_areas.json")


def catalogs(c):
    cat = c.game / "BloonsTD6_Data/StreamingAssets/aa/catalog.bin"
    c.parallel("Catalogs", [
        ["tools/assets/catalog_guids.py", cat, c.out / "catalog_guids.json"],
        ["tools/assets/catalog_audio.py", cat, c.out / "catalog_audio.json"],
        ["tools/assets/catalog_audio.py", cat, c.out / "catalog_prefabs.json", "prefab"],
        ["tools/assets/catalog_audio.py", cat, c.out / "catalog_sprites.json", "png"],
        ["tools/assets/index_sprite_guids.py", c.game, c.out / "catalog_sprite_paths.json"]], UI_WORKER)


def sprites(c):
    # The full export includes the gameplay atlases. Exporting those first decoded and
    # wrote the exact same sprites twice on every fresh installation.
    c.parallel("Sprites and fonts", [
        ["tools/assets/export_sprites.py", c.game, c.out / "sprites"],
        ["tools/assets/export_font.py", c.game, "localization_assets_all", "Btd6FontTitleSDF", "Luckiest Guy", c.out / "fonts/title"],
        ["tools/assets/export_font.py", c.game, "localization_assets_all", "Btd6FontBodySDF", "Source Sans Pro", c.out / "fonts/body"],
        ["tools/assets/export_text.py", c.game, c.out / "text", "localization_assets_all_*.bundle",
         "Assets/Data/Localization/Locs/English/English.xml"]], UI_WORKER)


def mapset(c):
    """MapSet (map ids, tiers, music); matches map images, so it runs after sprites."""
    c.py("tools/assets/export_mapset.py", c.game, c.out / "data/mapset.json")
    # Where the game's menus read it (frontend.rs, local_profile.rs, main.rs).
    (c.out / "ui").mkdir(exist_ok=True)
    shutil.copyfile(c.out / "data/mapset.json", c.out / "ui/mapset.json")


def drop_vertex_frame_models(c):
    """Once, on installs from before skin.bin: remove animated models stored as vertex frames so
    the exporters write them again as skinning data (a much smaller form). Maps are static."""
    models = c.out / "models"
    if not models.is_dir():
        return
    names = ("anim.bin", "anim.bin.z", "anim.bin.dz", "anim_q.bin", "anim_q.bin.z", "anim_q.bin.dz", "anim_q.bin.wz")
    dropped = 0
    for directory, folders, files in os.walk(models):
        if "meta.json" in files and any(n in files for n in names) and "skin.bin" not in files:
            # Only the marker the exporters check: a re-export replaces the rest of the folder.
            for name in names + ("meta.json",):
                (Path(directory) / name).unlink(missing_ok=True)
            dropped += 1
    print(f"{dropped} animated models will be stored in the smaller form", flush=True)


def tower_models(c):
    # Load shared Unity bundles once per bounded group, rather than once per model.
    jobs = [[str(c.out / "models/DartMonkey"), "asset_references_assets_all",
             "Assets/Monkeys/DartMonkey/Graphics/DartMonkey.prefab"]]
    special = {"WizardMonkey": "Wizard", "MonkeyVillage": "MonkeyVillage", "Desperado": "000-Desperado"}
    folder = {"MonkeyVillage": "graphics"}
    for t in ["WizardMonkey", "MonkeyVillage", "Desperado", "TackShooter", "BombShooter", "SniperMonkey", "BoomerangMonkey",
              "IceMonkey", "GlueGunner", "NinjaMonkey", "SuperMonkey", "SpikeFactory", "BananaFarm", "Druid", "Alchemist",
              "MortarMonkey", "DartlingGunner", "EngineerMonkey"]:
        jobs.append([str(c.out / f"models/{t}"), "generated_assets_all",
                     f"Assets/Generated/Assets/Monkeys/{t}/{folder.get(t, 'Graphics')}/{special.get(t, t)}.prefab"])
    plan = c.out / ".base_model_jobs.json"
    atomic_write(plan, json.dumps(jobs))
    try:
        c.py("tools/assets/model_batch.py", c.game, plan, progress=(0., .08))
    finally:
        plan.unlink(missing_ok=True)
    c.py("tools/assets/build_display_plan.py", c.game, *TOWERS)
    c.py("tools/assets/export_tower_tiers.py", c.game, *TOWERS, progress=(.08, .9))
    c.py("tools/assets/export_projectiles.py", c.game)


def misc_art(c):
    c.parallel("Art", [
        ["tools/assets/export_textures.py", c.game, c.out / "ui", "Assets/Monkeys/Materials/Range.png",
         "Assets/Icons/PlayerIcon/HandCursorDefault.png", "Assets/Icons/PlayerIcon/HandCursorDownDefaultSprite.png"],
        ["tools/assets/export_cursor.py", c.game],
        ["tools/assets/export_bloon_displays.py", c.game, c.out / "bloon_displays"],
        ["tools/assets/export_effect_sprites.py", c.game, "Assets/Bloons/Graphics/Effects/Pop.prefab", c.out / "effects/pop"]], UI_WORKER)


def upgrade_art(c):
    paths = c.containers()
    jobs = []
    portraits_all = [p for p in paths if p.startswith("Assets/ResizedImages/MonkeyPortraits/")]
    # Keep the original tower order for names shared by multiple towers.
    for t in TOWERS + ["MonkeySub", "MonkeyBuccaneer", "BeastHandler", "Skywarden", "Mermonkey"]:
        icon_dir = {"Druid": "DruidMonkey", "DartlingGunner": "DartlingMonkey"}.get(t, t)
        icons = [p for p in paths if p.startswith(f"Assets/ResizedImages/UI/UpgradeIcons/{icon_dir}/")]
        if icons:
            jobs.append({"out": str(c.out / "ui/upgrades"), "paths": icons, "prefix_numeric": t})
        dirs = {os.path.dirname(p) for p in portraits_all if p.lower().endswith(f"-{t}.png".lower())}
        portraits = [p for p in portraits_all if os.path.dirname(p) in dirs]
        if portraits:
            jobs.append({"out": str(c.out / f"ui/portraits/{t}"), "paths": portraits})
    plan = c.out / ".upgrade_art_jobs.json"
    atomic_write(plan, json.dumps(jobs))
    try:
        c.py("tools/assets/export_textures.py", c.game, "--jobs", plan)
    finally:
        plan.unlink(missing_ok=True)


def ingame_ui(c):
    p = "Assets/UI - New/InGameUi/Prefabs"
    themes = [x for x in c.containers() if "/TowerSelectionMenuPrefabs/TSMTheme" in x and x.endswith(".prefab")]
    b = "Assets/ResizedImages/UI/BigPanels"
    dump, prefabs = "tools/assets/dump_ui_prefab.py", c.out / "ui/prefabs"
    c.parallel("In-game UI", [
        [dump, c.game, prefabs,
         f"{p}/BossUI-RushUi.prefab", f"{p}/PowersMenu.prefab", f"{p}/MainHudLeftAlign.prefab", f"{p}/MainHudRightAlign.prefab", f"{p}/RightPanel.prefab", f"{p}/TowerSelectionMenu.prefab",
         f"{p}/TowerSelectionMenuPrefabs/TSMThemeDefault.prefab", "pid:6659321379417877030=TowerPurchaseButton",
         f"{p}/TowerSelectionMenuPrefabs/TSMThemeMortarMonkey.prefab", f"{p}/TowerSelectionMenuPrefabs/TSMThemeSelectPointInput.prefab"],
        [dump, c.game, prefabs, *themes],
        [dump, c.game, prefabs,
         "scene:PauseMenuPanel=PauseScreen", "scene:UpgradeScreen=UpgradeScreen",
         "scene:ForegroundScreen=CommonForeground", "scene:CommonBackground=CommonBackground",
         "field:UpgradeScreen/UpgradeScreen.upgradePrefab=UpgradeDetails",
         "field:UpgradeScreen/UpgradeScreen.upgradePopupPrefab=UpgradePopup",
         "field:UpgradeScreen/UpgradeScreen.upgradePopupLeftPrefab=UpgradePopupLeft"],
        # The pause screen's Animators (the jukebox EQ, the Drag & Drop radio).
        ["tools/assets/export_ui_runtime.py", c.game, c.out / "ui/runtime", "PauseScreenUI"],
        # RightPanel's CancelPurchase animators (placement cancel / Lock & Drop confirm buttons).
        ["tools/assets/export_ui_runtime.py", c.game, c.out / "ui/runtime", f"prefab:{p}/RightPanel.prefab",
         {"RRBTD6_UI_BUNDLES": "*_monoscripts_*.bundle;asset_references_assets_all_*.bundle"}],
        # The in-game Powers menu's Animators (its open / close slide).
        ["tools/assets/export_ui_runtime.py", c.game, c.out / "ui/runtime", f"prefab:{p}/PowersMenu.prefab",
         {"RRBTD6_UI_BUNDLES": "*_monoscripts_*.bundle;asset_references_assets_all_*.bundle"}],
        ["tools/assets/export_bundle_sprites.py", c.game, c.out / "sprites/_loose", "ui_*.bundle",
         "CircleUi", "ControlBtn", "GlowUi", "GlowyStar", "GodRay", "GodRaySmallUi", "LightGradientWhite", "PerksBtn",
         "ResetIcon", "SmallPanelWhite", "SpeechBubblePoint", "SpeechBubbleSquare", "TowerContainerT5Bottom",
         "TowerContainerT5Top", "YellowRibbonShop", "GiftboxIconSmall", "TowerContainerMilitaryLarge",
         "TowerContainerHeroLarge", "TowerContainerParagonLarge"],
        ["tools/assets/export_textures.py", c.game, c.out / "ui/panels",
         *[f"{b}/{n}.png" for n in ["TowerContainerPrimary", "TowerContainerPrimaryLarge", "TowerContainerMilitary",
                                     "TowerContainerMagic", "TowerContainerMAgicLarge", "TowerContainerSupport",
                                     "TowerContainerSupportLarge", "TowerContainerHero"]]]], UI_WORKER)


def game_data(c):
    root_index = struct.unpack("<i", (c.out / "game_model/sim_model.bytes").read_bytes()[-4:])[0]
    c.py("tools/assets/build_game_data.py", c.out, "tutorial", root_index)
    c.py("tools/assets/export_missing_upgrade_icons.py", c.game)


def extended_towers(c):
    # Include every current tower/hero and all paragon, air-unit and pet displays.
    # A short hand-maintained list omitted most paragons and all hero models.
    game = json.loads((c.out / "game/game.json").read_text(encoding="utf-8"))
    towers = game["tower_tiers"]
    # Installs from before the Genie's shots were planned; tower_models doesn't run again for them.
    c.py("tools/assets/build_display_plan.py", c.game, "Geraldo")
    c.py("tools/assets/export_tower_assets.py", c.game, *towers)
    heroes = [name for name, definition in towers.items() if definition.get("hero") is not None]
    if heroes:
        c.py("tools/assets/export_hero_portraits.py", c.game, *heroes)


def core_menus(c):
    prefabs = c.out / "ui/prefabs"
    c.parallel("Menus", [
        ["tools/assets/dump_ui_prefab.py", c.game, prefabs,
         *[f"scenefile:{path}={name}" for name, path in CORE_SCENES.items()],
         "Assets/UI - New/Popups/Prefabs/CommonPopup.prefab",
         "Assets/UI - New/Popups/Prefabs/ParagonConfirmationPopup.prefab",
         # PopupScreen.setValuePopup: sandbox Set Cash / Set Health (InGame.CheatSetCash).
         "Assets/UI - New/Popups/Prefabs/SetValuePopup.prefab",
         "Assets/UI - New/InGameUi/Prefabs/SandboxBloonsMenu.prefab",
         "Assets/UI - New/InGameUi/Prefabs/BuffManager.prefab",
         "Assets/UI - New/InGameUi/Prefabs/GeraldoShop.prefab",
         "field:SandboxBloonsMenu/BloonMenu.spawnBloonButtonPrefab=SpawnBloonButton",
         # DefeatScreen.bloonCauseDeathDisplayPrefab: a Bloons Leaked entry.
         "field:DefeatPanel/DefeatScreen.bloonCauseDeathDisplayPrefab=BloonCauseDeathDisplay"],
        ["tools/assets/export_ui_runtime.py", c.game, c.out / "ui/runtime",
         "MainMenuUi", *[name for name in CORE_SCENES if name != "MainMenuUI"]],
        # The sandbox bloon menu's slide in / out (SandBoxSlideOut).
        ["tools/assets/export_ui_runtime.py", c.game, c.out / "ui/runtime",
         "prefab:Assets/UI - New/InGameUi/Prefabs/SandboxBloonsMenu.prefab=SandboxBloonsMenu"]], UI_WORKER)
    c.py("tools/assets/resolve_ui_sprites.py", c.game, c.out, "asset_references_assets_all_*.bundle",
         *CORE_SCENES, "CommonPopup", "ParagonConfirmationPopup", "SetValuePopup", "SandboxBloonsMenu", "SpawnBloonButton", "BuffManager", "GeraldoShop",
         "BloonCauseDeathDisplay")


def tower_indices(c):
    """Repair indices on existing installs, reusing all completed animation bakes."""
    data = json.loads((c.out / "game/game.json").read_text(encoding="utf-8"))
    c.py("tools/assets/export_tower_tiers.py", c.game, *data["tower_tiers"])


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
    # What Geraldo's items show on the towers they buff (buffDisplayPath, a display GUID).
    buffs = [g for item in game.get("geraldo") or [] if (g := (item.get("params") or {}).get("buffDisplayPath"))]
    if buffs:
        catalog = json.loads((c.out / "catalog_prefabs.json").read_text(encoding="utf-8"))
        names.update(catalog[g].removesuffix(".prefab") for g in buffs)
    index = c.containers()
    paths = []
    for name in sorted(names):
        matches = [path for path in index if path.endswith(f"/{name}.prefab")]
        if len(matches) != 1:
            raise ValueError(f"Effect {name}: expected one original prefab, found {len(matches)}")
        paths.append(matches[0])
    script, effects = "tools/assets/export_particles.py", c.out / "effects"
    # Separate processes bound UnityPy memory across maps. A process keeps its loaded tower
    # bundle for every tower it is given, so towers go in a few groups (one load each)
    # instead of one process per tower. Every job writes its own effect folders.
    towers = list(game["tower_tiers"])
    groups = max(1, min(4, len(towers)))
    jobs = [[script, c.game, effects, *paths], [script, c.game, effects, "projectiles"]]
    jobs += [[script, c.game, effects, *(f"tiers:{t}" for t in towers[i::groups])] for i in range(groups)]
    jobs += [[script, c.game, effects, f"scene:{m.parent.name}"] for m in sorted((c.out / "maps").glob("*/map.json"))]
    # Measured 0.94-1.09 GiB peak for tier and projectile processes.
    c.parallel("Effects", jobs, UI_WORKER)


def audio(c):
    c.py("tools/assets/export_audio.py", c.game)
    c.py("tools/assets/export_jukebox.py", c.game)
    c.py("tools/assets/compress_audio.py", c.out / "audio")


def bloon_models(c):
    c.py("tools/assets/export_bloon_models.py", c.game)
    # Boss tiers' behaviours and models (game/bosses.json, models/bosses).
    c.py("tools/assets/export_bosses.py", c.game)
    c.py("tools/assets/export_powers.py")
    # The Power projectiles' displays it planned (Cash Drop's crate, the track items).
    plan = json.loads((c.out / "display_plans/Powers.json").read_text(encoding="utf-8"))
    c.py("tools/assets/export_projectiles.py", c.game, *sorted({p["name"] for p in plan["projectiles"].values()}))
    # Bloon overlays (glue, ice, bleed... on each bloon class: WhiteRegrowGlueBasic, MoabIce).
    c.py("tools/assets/plan_overlays.py")
    plan = json.loads((c.out / "display_plans/Overlays.json").read_text(encoding="utf-8"))
    c.py("tools/assets/export_projectiles.py", c.game, *sorted({p["name"] for p in plan["projectiles"].values()}))
    # The towers Powers place (game.json power_towers): display plans, models and portraits.
    game = json.loads((c.out / "game/game.json").read_text(encoding="utf-8"))
    c.py("tools/assets/export_tower_assets.py", c.game, *sorted({t["name"] for t in game.get("power_towers", {}).values()}))
    c.py("tools/assets/compact_anims.py", c.out / "models")


def frontend(c):
    prefabs = c.out / "ui/prefabs"
    backgrounds = [k for k in c.containers() if "/Heroes/HeroBackgrounds/" in k and k.endswith(".prefab")]
    c.parallel("Front-end scenes", [
        ["tools/assets/dump_ui_prefab.py", c.game, prefabs,
         *[f"scenefile:Assets/UI - New/Scenes/{s}.unity={s}" for s in FRONTEND_SCENES],
         "scenefile:Assets/Main/Scenes/LoadingUI.unity=LoadingUI", "Assets/UI - New/Popups/Prefabs/BloonsMonkeyTalesPopup.prefab"],
        ["tools/assets/dump_ui_prefab.py", c.game, prefabs, *backgrounds,
         {"RRBTD6_UI_BUNDLES": "*_monoscripts_*.bundle;asset_references_assets_all_*.bundle"}],
        ["tools/assets/export_ui_runtime.py", c.game, c.out / "ui/runtime", "MainMenuUi", "LoadingUI", *FRONTEND_SCENES, "DifficultySelectUI", "ModeSelectUI", "SettingsUI", "AccessibilitySettingsUI", "HotkeysUI"]], UI_WORKER)
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
    c.parallel("Startup screens", [
        ["tools/assets/export_startup_plane.py", c.game, c.out],
        ["tools/assets/dump_ui_prefab.py", c.game, prefabs,
         "scenefile:Assets/Main/Scenes/NinjaKiwiSplashUI.unity=NinjaKiwiSplashUI",
         "scenefile:Assets/Main/Scenes/InitialLoadingUI.unity=InitialLoadingUI", {"RRBTD6_UI_BUNDLES": boot}],
        ["tools/assets/dump_ui_prefab.py", c.game, prefabs,
         "scenefile:Assets/Main/Scenes/TitleScreenUI.unity=TitleScreenUI",
         "scenefile:Assets/Main/Scenes/DataConflictUI.unity=DataConflictUI",
         "scenefile:Assets/UI - New/Scenes/AccountManagementUI.unity=AccountManagementUI",
         "scenefile:Assets/UI - New/Scenes/LinkAccountUI.unity=LinkAccountUI",
         "Assets/UI - New/Popups/Prefabs/LogoutPopup.prefab"],
        ["tools/assets/export_ui_runtime.py", c.game, c.out / "ui/runtime", "NinjaKiwiSplashUI", "InitialLoadingUI",
         "TitleScreenUI", "DataConflictUI", "AccountManagementUI", "LinkAccountUI",
         {"RRBTD6_UI_BUNDLES": "*_monoscripts_*.bundle;initial_loading_ui_scenes_all_*.bundle;ui_scenes_all_*.bundle;asset_references_assets_all_*.bundle"}],
        # The window and taskbar icon, from the player's own BloonsTD6.exe.
        ["tools/assets/export_game_icon.py", c.game, c.out / "brand"]], UI_WORKER)
    # One after another: each updates the folder's shared borders file.
    c.py("tools/assets/export_bundle_sprites.py", c.game, c.out / "sprites/_loose", "ui_scenes_all_*.bundle", "TitleBg", "BTD6Logo")
    # The splash and initial loading screens' sprites live in their own scene bundle.
    c.py("tools/assets/export_bundle_sprites.py", c.game, c.out / "sprites/_loose", "initial_loading_ui_scenes_all_*.bundle",
         "NZoA_RGB_Game Sector Logo_WhiteText", "NkLogoKiwiIcon", "NkLogoKiwiText", "NkLogoMask", "NkLogoNinjaText",
         "NkLogoText0", "NkLogoTextA", "NkLogoTextE", "NkLogoTextM", "NkLogoTextR", "NkLogoTextS", "NkLogoTextW",
         "NkSheenSquare", "DartpointIconInitialAsset", "GreenBtnLongInitialAsset", "LoadPlane01", "LoadPlaneProp01",
         "LoadPlaneScarf01", "LoadingCloud", "LoadingCloudPuff", "LoadingSky", "MainBGPanelBlueInitialAsset",
         "SmallSquareWhiteLoadBar", "TitleBannerCommonInitialAsset", "WhiteSquareGradientLoadBar", "YellowBtnLongInitialAsset")
    c.py("tools/assets/export_game_version.py", c.game, c.root / "research/dumps/methods-all.jsonl", c.out / "game_version.json")


def home(c):
    c.py("tools/assets/export_model.py", c.game, c.out / "models/mainmenu", "scene", "ui_scenes_all_",
         "cab:CAB-613509f63417784aa3960cd4dc66a279")
    for s in ["export_home_animations", "export_home_vegetation", "export_home_interactions"]:
        c.py(f"tools/assets/{s}.py", c.game, c.out)
    c.py("tools/assets/export_home_events.py", c.game, c.out)


def social(c):
    c.parallel("Co-op and event menus", [["tools/assets/export_coop_ui.py", c.game, c.out],
                                         ["tools/assets/export_event_ui.py", c.game, c.out]], UI_WORKER)


def menu_additions(c):
    """New menu assets without invalidating already completed map/tower bakes."""
    c.parallel("Menu additions", [
        ["tools/assets/dump_ui_prefab.py", c.game, c.out / "ui/prefabs",
         "scenefile:Assets/UI - New/Scenes/DailyRewardUI.unity=DailyRewardUI",
         "scenefile:Assets/UI - New/Scenes/JukeboxUI.unity=JukeboxUI",
         "field:JukeboxScreen/JukeBoxScreen.trackItemPrefab=JukeboxTrack",
         "Assets/UI - New/Prefabs/Store/Loot/MediumBaseLootItem.prefab",
         "Assets/UI - New/Prefabs/Store/Loot/MediumInstaTowerLootItem.prefab"],
        ["tools/assets/export_ui_runtime.py", c.game, c.out / "ui/runtime", "DailyRewardUI", "JukeboxUI"]], UI_WORKER)
    c.py("tools/assets/export_menu_chrome.py", c.game, c.out)
    c.py("tools/assets/export_home_buttons.py", c.game, c.out)


def profile_menu(c):
    """The profile page opened from the main menu's avatar."""
    c.py("tools/assets/export_profile_ui.py", c.game, c.out)


def builtin_ui_sprites(c):
    """Sprites UI prefabs reference outside the bundles the dump reads: Unity's built-in UISprite,
    Background, InputFieldBackground and UIMask (scrollbars, input fields, dropdowns) and
    PowerProBundleBanner. After every UI step, so a later dump can't bring the raw ids back."""
    names = sorted(p.stem for p in (c.out / "ui/prefabs").glob("*.json"))
    for pattern in ("*_unitybuiltinassets_*.bundle", "ui_assets_all_*.bundle"):
        c.py("tools/assets/resolve_ui_sprites.py", c.game, c.out, pattern, *names)
    c.py("tools/assets/export_buff_icons.py", c.game, c.out)
    # Sprites named by a prefab dumped after the earlier chrome passes (SquareRadialShadow).
    c.py("tools/assets/export_menu_chrome.py", c.game, c.out)


def pending_models(c):
    """Models for mechanics not built yet (Paragon degree add-ons, Magus Perfectus' phoenix...)."""
    c.py("tools/assets/export_pending_models.py", c.game)


def finish(c):
    c.py("tools/assets/canonical_anims.py", c.root, c.out / "models")
    # Home/startup models are exported after bloon_models' earlier compaction pass.
    # Keep that earlier pass to bound peak disk usage; finish compresses late models too.
    c.py("tools/assets/compact_anims.py", c.out / "models")
    c.py("tools/assets/pack_anims.py", c.out / "models")
    c.py("tools/assets/delta_anims.py", c.out / "models")
    # Installs packed before exports used zlib level 6: recompress (verified, lossless).
    c.py("tools/assets/repack_anims.py", c.out / "models")
    c.py("tools/assets/word_anims.py", c.out / "models", "--root", c.root)
    c.py("tools/assets/patch_tmp_gradients.py", c.game, c.root)
    c.py("tools/assets/rebuild_shader_glsl.py", c.out)
    c.py("tools/assets/share_model_textures.py", c.out / "models")
    c.py("tools/assets/compact_json.py", c.out)


def dedupe(c):
    """Byte-identical runtime assets stored once (hard links; paths and bytes unchanged)."""
    c.py("tools/assets/link_duplicates.py", c.out)


def prune_achievements(c):
    c.py("tools/assets/prune_achievements.py", c.out)


STEPS = [prune_achievements, metadata, sim_model, cab_index, catalogs, container_index, rounds_maps, sprites, mapset, tower_models, misc_art, upgrade_art,
         ingame_ui, core_menus, game_data, extended_towers, tower_indices, particles, audio, bloon_models, frontend, startup, home, social, menu_additions, profile_menu, builtin_ui_sprites, map_mechanics, pending_models, finish, dedupe]


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
# core_menus 2: the sandbox menu's slide animation (ui/runtime/SandboxBloonsMenu.json).
STEP_VERSIONS["core_menus"] = 2
# startup 2: the splash and initial loading screens' sprites.
# startup 4: the title screen's BTD6Logo and the game's window icon.
# startup 5: its plane model as skin.bin.
STEP_VERSIONS["startup"] = 5
# finish 9: remove runtime JSON whitespace while preserving every value token.
# rounds_maps 2: renderers with a RectTransform are kept. particles 2: override controllers' clips.
STEP_VERSIONS["rounds_maps"] = 2
STEP_VERSIONS["particles"] = 3  # 3: Geraldo's buff effects and the Genie's shots.
# mapset 2: also at ui/mapset.json, where the menus read it.
STEP_VERSIONS["mapset"] = 2
# finish 10, tower_models 2 and the other model steps' raises: animated models as skin.bin.
# finish 11: packs pending_models' animations.
STEP_VERSIONS["finish"] = 11
for _step, _version in (("tower_models", 2), ("extended_towers", 2), ("bloon_models", 2), ("home", 2), ("menu_additions", 2)):
    STEP_VERSIONS[_step] = _version
# extended_towers 3: Geraldo's shop item models.
STEP_VERSIONS["extended_towers"] = 4  # 4: the Genie Bottle's genie (models/air).
# ingame_ui 2: RightPanel's animators.
STEP_VERSIONS["ingame_ui"] = 5  # 3: the boss HUD; 4: the Powers menu; 5: its Animators.
# misc_art 3, game_data 3: bloon sprites at their true size (Simple draw mode) with trim, per axis.
STEP_VERSIONS["misc_art"] = 3
STEP_VERSIONS["game_data"] = 7  # 4: power_projectiles and power_towers; 5: Desperado's panel toggles, portraits;
# 6: the lightning flag; 7: bloon overlay classes and effect overlay types.
# audio 3: the game's own Vorbis as .ogg (fsb_vorbis.py) instead of FLAC decoded by FMOD.
# audio 4: also the asset library's clips.
STEP_VERSIONS["audio"] = 4
STEP_VERSIONS["dedupe"] = 1
# bloon_models 2: boss models and data.
STEP_VERSIONS["bloon_models"] = 8  # 3: also game/powers.json; 4: keyed by the Powers' ids; 5-6: Power projectile displays; 7: Power towers' models;
# 8: bloon overlays (glue, ice...).
# map_mechanics 4: part ids, animators, clips, the spawner and logic in map.json; more triggers.
STEP_VERSIONS["map_mechanics"] = 4
# builtin_ui_sprites 3: also buff icons under the names towers ask for.
STEP_VERSIONS["builtin_ui_sprites"] = 3


def drop_old_game_exports(c):
    """After a BTD6 update: remove models, maps and audio, which exporters keep when they already
    exist, so they are made again from the new game instead of staying on the old version.
    Everything else is overwritten by its step. Refuses links, as cleanup_bundle_cache does."""
    is_link = lambda path: path.is_symlink() or getattr(path, "is_junction", lambda: False)()
    if is_link(c.out):
        return
    for name in ("models", "maps", "audio"):
        folder = c.out / name
        if folder.is_dir() and not is_link(folder):
            shutil.rmtree(folder)
            print(f"== {name}: re-exporting from the updated game", flush=True)


def cleanup_bundle_cache(c):
    """Remove only this install's regenerable decompression cache, after success.

    Preserve caches on interrupted/partial runs. Refuse links/junctions, including
    nested ones, before removing anything. Never follow a path outside the output.
    """
    cache = c.out / "_bundle_cache"
    if not cache.exists():
        return 0
    is_link = lambda path: path.is_symlink() or getattr(path, "is_junction", lambda: False)()
    base = c.out.resolve()
    if is_link(c.out) or is_link(cache) or cache.resolve().parent != base or cache.resolve().name != "_bundle_cache":
        print("== retaining bundle cache: unexpected linked cache path", flush=True)
        return 0
    try:
        size = 0
        for directory, folders, files in os.walk(cache, followlinks=False):
            for name in folders + files:
                entry = Path(directory) / name
                if is_link(entry):
                    print("== retaining bundle cache: linked cache entry", flush=True)
                    return 0
            for name in files:
                size += (Path(directory) / name).stat().st_size
        shutil.rmtree(cache)
    except OSError as error:
        # A virus scanner may temporarily hold a cached file. Extraction itself succeeded.
        print(f"== bundle cache cleanup deferred ({type(error).__name__}); next full run will retry", flush=True)
        return 0
    print(f"== removed {size / 1e9:.2f} GB of temporary bundle data", flush=True)
    return size


def fingerprint(c):
    """The game build: an update re-runs every step."""
    h = hashlib.sha256()
    for f in [c.game / "GameAssembly.dll", c.meta]:
        st = f.stat(); h.update(f"{f.name}:{st.st_size}:{int(st.st_mtime)}".encode())
    return h.hexdigest()


def asset_revision():
    """Release contract: bump a step version when its required output changes."""
    return hashlib.sha256(json.dumps(STEP_VERSIONS, sort_keys=True).encode("utf-8")).hexdigest()


def main():
    if sys.argv[1:] == ["--asset-revision"]:
        print(asset_revision())
        return
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--game", required=True, help="BTD6 install folder (with MelonLoader)")
    ap.add_argument("--root", default=str(Path(__file__).resolve().parents[1]), help="folder holding tools/")
    ap.add_argument("--only", nargs="*", help="run just these steps")
    ap.add_argument("--force", action="store_true", help="ignore recorded progress")
    ap.add_argument("--keep-bundle-cache", action="store_true", help="retain temporary decompressed bundles for exporter development")
    args = ap.parse_args()
    c = Ctx(args.game, args.root)
    if not (c.game / "GameAssembly.dll").exists():
        sys.exit(f"Not a BTD6 install: {c.game}")
    c.out.mkdir(parents=True, exist_ok=True)
    install_hook(c.root)
    state_file = c.out / ".extract_state.json"
    fp = fingerprint(c)
    state = {"fingerprint": fp, "done": {}} if args.force else load_state(state_file, fp)
    if not args.only and not args.force:
        try:
            previous = json.loads(state_file.read_text(encoding="utf-8")).get("fingerprint")
        except (OSError, ValueError, AttributeError):
            previous = None
        if previous and previous != fp:
            drop_old_game_exports(c)
    names = [s.__name__ for s in STEPS]
    for n in args.only or []:
        if n not in names:
            sys.exit(f"unknown step {n}; steps: {', '.join(names)}")
    selected = [name for name in names if not args.only or name in args.only]
    history_path = c.out / ".extract_timings.json"
    c.progress = Progress(selected, load_history(history_path))
    print("RustTD6 setup - you can follow progress here in the MelonLoader terminal.", flush=True)
    print("Time remaining is an estimate and will adjust as stages finish.", flush=True)
    print("Setup uses a lot of CPU, memory and disk. It goes faster with more free memory: close programs you aren't using, then let it finish.", flush=True)
    print(f"Install data: {c.out} | Free space: {shutil.disk_usage(c.out).free / 1e9:.1f} GB", flush=True)
    c.progress.begin()
    success = False
    try:
        split = False
        for step in STEPS:
            name = step.__name__
            if args.only and name not in args.only:
                continue
            if not args.only and state["done"].get(name) == STEP_VERSIONS[name]:
                print(f"== {name}: already prepared", flush=True)
                c.progress.complete(name, skipped=True)
                continue
            if name == "tower_models" and not args.only and state["done"].get(name, 2) < 2:
                drop_vertex_frame_models(c)
            if name != "dedupe" and not split:
                # Exporters overwrite files in place: if an earlier run linked duplicates, give them
                # their own copies first (dedupe links them again at the end).
                if state["done"].pop("dedupe", None) is not None:
                    c.py("tools/assets/link_duplicates.py", "--split", c.out)
                    atomic_write(state_file, json.dumps(state))
                split = True
            print(f"== {name}", flush=True)
            c.progress.start(name)
            began = time.time()
            step(c)
            print(f"== {name}: {time.time() - began:.0f}s", flush=True)
            c.progress.complete(name)
            state["done"][name] = STEP_VERSIONS[name]
            atomic_write(state_file, json.dumps(state))
        if not args.only and not args.keep_bundle_cache:
            cleanup_bundle_cache(c)
        if not args.only:
            atomic_write(c.out / "assets_version.json", json.dumps({"revision": asset_revision(), "source": fp}))
        success = True
        print("Asset preparation complete. Starting RustTD6...", flush=True)
    finally:
        c.progress.finish(success)
        try:
            atomic_write(history_path, json.dumps(c.progress.history))
        except OSError:
            print("Could not save setup timing estimates; completed extraction checkpoints remain valid.", flush=True)



if __name__ == "__main__":
    main()
