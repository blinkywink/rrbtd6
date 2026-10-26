"""Export original frontend SkinData and tower-container sprite references (local only).
Usage: python export_frontend_data.py GAME EXTRACTED
"""
import glob
import os
os.environ["OPENBLAS_NUM_THREADS"] = "1"
import json
import sys
from pathlib import Path
import UnityPy
from UnityPy.helpers import TypeTreeHelper
TypeTreeHelper.read_typetree_boost = False
game, extracted = map(Path,sys.argv[1:3])
base = game/"BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full"
env = UnityPy.load(glob.glob(str(base/"game_data_assets_all_*.bundle"))[0],glob.glob(str(base/"*_monoscripts_*.bundle"))[0])
prefabs = json.loads((extracted/"catalog_prefabs.json").read_text())
sprite_names = json.loads((extracted/"catalog_sprites.json").read_text())
gd = env.container["Assets/Data/GameData.asset"]
gd = gd.deref() if hasattr(gd,"deref") else gd
skin_set = gd.assets_file.objects[gd.read_typetree()["skinsData"]["m_PathID"]]
ordered = [skin_set.assets_file.objects[p["m_PathID"]].read_typetree()
           for p in skin_set.read_typetree()["SkinList"]["items"]]

source_aliases = {}
path_catalog_file = extracted/"catalog_sprite_paths.json"
path_catalog = json.loads(path_catalog_file.read_text()) if path_catalog_file.exists() else {}
from collections import Counter
basename_counts = Counter(Path(p).name for p in set(path_catalog.values()))

def image(pointer):
    guid = pointer.get("guidRef", "")
    path = path_catalog.get(guid)
    if path:
        name = Path(path).name
        alias = guid[:12]+"__"+name if basename_counts[name]>1 else name
        source_aliases[alias] = path
        return alias
    return sprite_names.get(guid)

def roster_image(pointer):
    return image(pointer)

def appearance(t):
    bg = t["backgroundBanner"]["guidRef"]
    swaps = {s["assetToReplace"]["guidRef"]:s["assetReplacement"]["guidRef"] for s in t["SwapSpriteContainer"]["items"]}
    def skin_image(pointer):
        guid = pointer.get("guidRef", "")
        return image({"guidRef":swaps.get(guid,guid)})
    return {
        "id":t["m_Name"], "default":bool(t["isDefaultTowerSkin"]),
        "hidden_when_locked":bool(t["isHiddenWhenLocked"]), "mm_cost":t["mmCost"], "iap":t["iapName"],
        # Offline preview exposes all non-IAP appearances, including special unlocks.
        # Native account binding will apply hidden/owned and SKU price overrides later.
        "collection_counted":bool(t["isDefaultTowerSkin"]) or t["mmCost"] != 0,
        "short_description":t["skinName"], "description":t["description"],
        "alternate_locs":t["alternateLocsName"],
        "background":prefabs.get(bg), "background_guid":bg,
        "color":[t["backgroundColourTintOverride"][k] for k in ("r","g","b","a")],
        "font_material":t["fontMaterial"], "icon":image(t["icon"]),
        "icon_square":roster_image(t["iconSquare"]),
        "portraits":[{"level":int(p["levelTxt"]),"image":skin_image(p["asset"])}
                     for p in t["StorePortraitsContainer"]["items"] if p["levelTxt"].isdigit()],
    }

skins = {}
for t in ordered:
    if t["isDefaultTowerSkin"]:
        skin = appearance(t)
        # UpdateHeroSkinButtons uses OrderByDescending(s => s.isDefaultTowerSkin).
        skin["skins"] = [appearance(v) for v in sorted(
            (v for v in ordered if v["baseTowerName"] == t["baseTowerName"]),
            key=lambda v:not v["isDefaultTowerSkin"])]
        skins[t["baseTowerName"]] = skin
out = extracted/"ui/runtime/hero_data.json"
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(json.dumps(skins,indent=1),encoding="utf8")
print(len(skins),"heroes;",sum(len(s["skins"]) for s in skins.values()),"skins")

# Keep the original per-screen background references, tint and tiling settings.
bg_asset = gd.assets_file.objects[gd.read_typetree()["menuBackgrounds"]["m_PathID"]].read_typetree()
backgrounds = {r["screenName"]: {
    "image":image(r["bgTextureRef"]), "own":bool(r["useOwnOrNoBackground"]),
    "color":[r["color"][k] for k in ("r","g","b","a")],
    "tiles":r["tilesPerHeight"], "vignette":r["vignetteMul"],
} for r in bg_asset["ScreenList"]["items"]}
(extracted/"ui/runtime/menu_backgrounds.json").write_text(json.dumps(backgrounds,indent=1),encoding="utf8")

# Small read-only browser data: opening Heroes must not instantiate a full simulation.
game_file = extracted/"game/game.json"
if game_file.exists():
    game_ui = json.loads(game_file.read_text(encoding="utf8"))
    (extracted/"ui/runtime/hero_shop.json").write_text(json.dumps(game_ui.get("heroes",[])),encoding="utf8")
    hero_levels = {}
    for hero in skins:
        tiers = game_ui.get("tower_tiers",{}).get(hero,{})
        levels = {}
        for level in tiers.get("hero",{}).get("levels",[]):
            abilities = tiers.get("tiers",{}).get(str(level["level"])+"00",{}).get("abilities",[])
            ability = next((a for a in abilities if a.get("upgrade")==level["name"]),None)
            if ability: levels[str(level["level"])] = {"ability":ability["name"],"upgrade":ability.get("upgrade","")}
        hero_levels[hero] = levels
    (extracted/"ui/runtime/hero_levels.json").write_text(json.dumps(hero_levels),encoding="utf8")
    del game_ui

# Count the original knowledge nodes instead of displaying the scene's "99/99" placeholders.
tree_file = extracted/"ui/prefabs/KnowledgeSkillTreeUI.json"
if tree_file.exists():
    tree = json.loads(tree_file.read_text())
    def nodes(n):
        yield n
        for child in n.get("children",[]): yield from nodes(child)
    counts = {}
    for page in nodes(tree):
        if page["name"] in ["Page"+str(i) for i in range(1,7)]:
            counts[page["name"]] = sum(any(c["type"]=="KnowledgeSkillBtn" for c in n.get("comps",[])) for n in nodes(page))
    (extracted/"ui/runtime/knowledge_counts.json").write_text(json.dumps(counts),encoding="utf8")

# The Powers screen's buttons are created from PowerModels, not stored in its scene.
model_file = extracted/"decoded/named/objects.jsonl"
if model_file.exists():
    import xml.etree.ElementTree as ET
    english = {t.attrib["id"]:"".join(t.itertext()) for t in ET.parse(extracted/"text/English.xml").iter("T") if "id" in t.attrib}
    description_models = {}
    projectile_pierce = {}
    models = {}
    containers = json.loads((extracted/"decoded/named/containers.json").read_text())
    def items(ref):
        return containers.get(str(ref.get("$ref")),{}).get("items",[]) if isinstance(ref,dict) else []
    game_model = None
    for line in model_file.open(encoding="utf8"):
        item = json.loads(line)
        if item.get("type") in ("MonkeyBoostModel","ThriveModel","GlueTrapModel","CamoTrapModel") and item["f"].get("_name")==item["type"]+"_"+item["type"].removesuffix("Model"):
            description_models[item["type"]] = item["f"]
        if item.get("type") == "ProjectileModel":
            projectile_pierce[item["i"]] = item["f"].get("pierce",0)
        if item.get("type") in ("PowerModel","PowerProModel","ApplyModModel","ModModel","PowerModModel","PowerProUpgradePathModel","PowerProUpgradeModel"):
            models[item["i"]] = item
        if item.get("type") == "GameModel": game_model = item["f"]
    by_name = {m["f"]["_name"]:m["f"] for m in models.values() if m["type"] in ("ModModel","PowerProUpgradeModel")}
    discount = {m["f"]["_name"]:m["f"]["costDecrease"] for m in models.values() if m["type"]=="PowerModModel"}
    # GameData.powersProData.baseAndProIds: "BaseId,ProId" pairs (PowersProData.GetProOrBasePowerId).
    pro_data = gd.assets_file.objects[gd.read_typetree()["powersProData"]["m_PathID"]].read_typetree()
    pairs = [entry.split(",") for entry in pro_data["baseAndProIds"]]
    linked = {a:b for a,b in pairs} | {b:a for a,b in pairs}
    powers = []
    # GameModel.GetAvailablePowers: GameModel.powers in order, skipping mode-only powers.
    for index in items(game_model["powers"]):
        f = models[index]["f"]
        if f.get("availableInMode"): continue
        pro = models[index]["type"] == "PowerProModel"
        name = f["_name"]
        power_id = f["powerId"] if pro else name
        ref = f.get("icon", "")
        icon = (ref.split("[",1)[1].rstrip("]")+".png") if "[" in ref else sprite_names.get(ref)
        # GameModel.GetModdedPowers: Monkey Knowledge PowerModModels lower the cost (PowerModModel.Mutate).
        discounts = {}
        for mod_index in items(f.get("mods")):
            mod = by_name.get(models[mod_index]["f"].get("mod"))
            for mutator in items(mod.get("mutatorMods")) if mod else []:
                if models.get(mutator,{}).get("type") == "PowerModModel": discounts[models[mutator]["f"]["_name"]] = discount[models[mutator]["f"]["_name"]]
        entry = {"id":power_id,"title":english.get(name,name),"description":english.get(name+" Description",""),
                 "icon":icon,"cost":f["cost"],"order":f.get("orderPriority",0),
                 "animated_icon":prefabs.get(f.get("animatedIcon","")),
                 "pro":pro,"store":f.get("storeNavigateToId",""),"hidden":bool(f.get("isHidden")),
                 "achievement":f.get("linkedToAchievement",0),"showAsNew":bool(f.get("showAsNew")),
                 "linked":linked.get(power_id,""),"discounts":discounts}
        if pro:
            # GameModel.GetXpCostForProUpgrade / PowersProDataHelper.GetXpForPowerProCurrentUpgrade walk
            # the path-0 upgrades in PowerProModel.upgrades order.
            path = [by_name[models[u]["f"]["upgrade"]] for u in items(f.get("upgrades"))]
            entry["proXp"] = [u["xpCost"] for u in path if u["path"]==0]
        powers.append(entry)
    # PowerModel.GetDescription formats these four parameters from the same models.
    parameters = {}
    if "MonkeyBoostModel" in description_models: parameters["MonkeyBoost"] = description_models["MonkeyBoostModel"]["duration"]
    if "ThriveModel" in description_models: parameters["Thrive"] = (description_models["ThriveModel"]["cashScale"]-1)*100
    for name in ("GlueTrap","CamoTrap"):
        if name+"Model" in description_models:
            parameters[name] = projectile_pierce[description_models[name+"Model"]["projectileModel"]["$ref"]]
    for power in powers:
        if power["id"] in parameters: power["description"] = power["description"].replace("{0}",f'{parameters[power["id"]]:g}')
    # PowersSelectScreen.SetupPowerButtons: OrderBy(storeNavigateToId == "").ThenBy(!IsPowerPro), both stable.
    powers.sort(key=lambda p:(p["store"]=="",not p["pro"]))
    (extracted/"ui/runtime/powers.json").write_text(json.dumps(powers,indent=1),encoding="utf8")

# Font materials are localized assets; read their actual TMP properties separately.
del env
import gc
gc.collect()
fonts = UnityPy.load(glob.glob(str(base/"localization_assets_all_*.bundle"))[0])
materials = {}
for obj in fonts.objects:
    if obj.type.name != "Material": continue
    t = obj.read_typetree()
    name = t.get("m_Name", "")
    if not name.startswith("Btd6FontTitleSDFHero"): continue
    # All locale variants share style; prefer the same named Latin material when present.
    props = t.get("m_SavedProperties", {})
    value = {k:v for key in ("m_Floats", "m_Ints") for k,v in props.get(key, [])}
    value.update({k:[v[c] for c in ("r","g","b","a")] for k,v in props.get("m_Colors", [])})
    value["keywords"] = t.get("m_ValidKeywords", []) or t.get("m_ShaderKeywords", "").split()
    materials.setdefault(name, value)
(extracted/"ui/runtime/hero_materials.json").write_text(json.dumps(materials,indent=1),encoding="utf8")
print(len(materials),"hero title materials")

if "--images" in sys.argv:
    sys.path.insert(0,str(Path(__file__).resolve().parent))
    from export_sprites import full_rect
    from collections import defaultdict
    del fonts
    gc.collect()
    index = json.loads((extracted/"all_container_paths.json").read_text())
    by_name = defaultdict(list)
    for path in index:
        by_name[Path(path).name].append(path)
    names = {s[k] for hero in skins.values() for s in hero["skins"] for k in ("icon","icon_square") if s[k]}
    names.add("PowersPatternGreyBg.png")
    names |= {b["image"] for b in backgrounds.values() if b["image"]}
    names |= {p["image"] for hero in skins.values() for s in hero["skins"] for p in s["portraits"] if p["image"]}
    bundles = defaultdict(list)
    missing = []
    for name in sorted(names):
        paths = [source_aliases[name]] if name in source_aliases and source_aliases[name] in index else ([] if name in source_aliases else by_name.get(name, []))
        if not paths:
            missing.append(name)
            continue
        path = sorted(paths,key=lambda p:("/ResizedImages/" not in p,p))[0]
        if "--refresh-images" in sys.argv or not (extracted/"ui/hero_skins"/name).exists():
            bundles[index[path]].append((path,name))
    dest = extracted/"ui/hero_skins"
    dest.mkdir(exist_ok=True)
    for bundle, paths in bundles.items():
        images = UnityPy.load(str(base/bundle))
        for path, output_name in paths:
            source = images.container[path].read()
            image = full_rect(source) if hasattr(source,"m_Rect") else source.image
            image.save(dest/output_name)
            # UnityPy retains every decoded atlas in the serialized-file cache. Bound the
            # export's memory instead of accumulating all hero atlases at once.
            for f in images.files.values():
                for sf in getattr(f,"files",{}).values():
                    if hasattr(sf,"_cache"): sf._cache.clear()
            del image
        del images
        gc.collect()
    print(len(names)-len(missing),"skin images exported; unresolved:",missing)
