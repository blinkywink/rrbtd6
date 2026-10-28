"""Map every tier of the given towers to its tower and projectile display prefabs.

Usage:
    tools/.venv/Scripts/python.exe tools/assets/build_display_plan.py <game install> <TowerId>...

Needs local_extracted/catalog_guids.json (catalog_guids.py) and the decoded sim_model
(local_extracted/decoded). Writes:
  - local_extracted/all_container_paths.json {container path: bundle file} (scanned once)
  - local_extracted/display_plans/<TowerId>.json
        {"towers": {tier: {prefab, bundle}}, "projectiles": {display guid: {name, prefab}}}
"""
import collections
import glob
import json
import os
import sys

import UnityPy
from UnityPy.helpers import TypeTreeHelper

TypeTreeHelper.read_typetree_boost = False

GAME = sys.argv[1]
TOWERS = sys.argv[2:]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
PATHS = "local_extracted/all_container_paths.json"

if os.path.exists(PATHS):
    paths = json.load(open(PATHS))
else:
    paths = {}
    for f in glob.glob(os.path.join(AA, "*.bundle")):
        if "map_" in os.path.basename(f) or "music" in f or "videos" in f:
            continue
        for o in UnityPy.load(f).objects:
            if o.type.name == "AssetBundle":
                for k, _ in o.read_typetree()["m_Container"]:
                    paths.setdefault(k, os.path.basename(f))
    json.dump(paths, open(PATHS, "w"))

guids = json.load(open("local_extracted/catalog_guids.json"))
CATALOG_PREFABS = "local_extracted/catalog_prefabs.json"
catalog_prefabs = json.load(open(CATALOG_PREFABS)) if os.path.exists(CATALOG_PREFABS) else {}
by_base = collections.defaultdict(list)
for p, b in paths.items():
    by_base[os.path.basename(p)].append((p, b))
objs = {}
vals = {int(k): v for k, v in json.load(open("local_extracted/decoded/values.json")).items()}
for line in open("local_extracted/decoded/named/objects.jsonl", encoding="utf-8"):
    r = json.loads(line)
    objs[r["i"]] = r


def deref(v):
    return (objs.get(v["$ref"]) or vals.get(v["$ref"])) if isinstance(v, dict) and "$ref" in v else v


def items(v):
    c = deref(v)
    if isinstance(c, list):
        return c
    if isinstance(c, dict) and "items" in c:
        return [objs.get(i) or vals.get(i) for i in c["items"] if i]
    return []


# INFERRED: the catalog stores no file name next to projectile display GUIDs. Each GUID is used
# by exactly one tier family, matched here to the one projectile prefab named for that family.
# None = an empty prefab (nothing drawn). Particle-only prefabs (Wall of Fire, Phoenix fireballs,
# undead bloons) resolve through the catalog like the rest: export_projectiles.py gives them a
# stand-in sprite and export_particles.py their particle effect, which the renderer plays instead.
PROJECTILE_BY_GUID = {
    "bdbeaa256e6c63b45829535831843376": "Assets/Monkeys/DartMonkey/Graphics/Projectiles/Dart.prefab",
    "69bf8d5932f2bea4f9ce36f861240d2e": "Assets/Monkeys/DartMonkey/Graphics/Projectiles/TripleDart.prefab",
    "0ddd8752be0d3554cb0db6abe6686e8e": "Assets/Monkeys/DartMonkey/Graphics/Projectiles/Arrow.prefab",
    "9717eb2059b0d6f4e9a5b500bd67a45e": "Assets/Monkeys/DartMonkey/Graphics/Projectiles/MasterArrow.prefab",
    "fcddee8a92f5d2e4d8605a8924566620": "Assets/Monkeys/DartMonkey/Graphics/Projectiles/SpikeOPultProjectile.prefab",
    "ee74983d627954e4e9765d86e05b4500": "Assets/Monkeys/DartMonkey/Graphics/Projectiles/JuggernautProjectile.prefab",
    "c4b8e7aa3e07d764fb9c3c773ceec2ab": "Assets/Monkeys/DartMonkey/Graphics/Projectiles/UltraJuggernautProjectile.prefab",
    "26e9c99e89180fd468cb47d76c7536b6": "Assets/Monkeys/TackShooter/Graphics/Projectiles/Tack.prefab",
    "c184360c85b9d70499bb2fff7c77ecb2": "Assets/Monkeys/TackShooter/Graphics/Projectiles/HotTack.prefab",
    "73c06263b85d1354da4cf582782051b1": "Assets/Monkeys/TackShooter/Graphics/Projectiles/Blade.prefab",
    "c840e245a0b1deb4284cfc3f953e16cf": "Assets/Monkeys/TackShooter/Graphics/InfernoRingMeteor.prefab",
    "4bce3e766a25dc74085e2427d1db6160": "Assets/Monkeys/BombShooter/Graphics/Projectiles/Bomb.prefab",
    "112a976e8d890c348a1b6b0531d2addd": "Assets/Monkeys/BombShooter/Graphics/Projectiles/BigBomb.prefab",
    "97d3634339d18c443853aecd4acbd157": "Assets/Monkeys/BombShooter/Graphics/Projectiles/ReallyBigBomb.prefab",
    "25806b6be5f85c54a84fefe2e95acfbc": "Assets/Monkeys/BombShooter/Graphics/Projectiles/Missile.prefab",
    "476543fcd521dc14abbaed958b033d2c": "Assets/Monkeys/BombShooter/Graphics/Projectiles/MissileHeavyBomb.prefab",
    "f48705ab78b809748b9c0593c1099c06": "Assets/Monkeys/BombShooter/Graphics/Projectiles/MissileHeavyBombCannon.prefab",
    "99a115167e2bb4d48946a4a439210acf": "Assets/Monkeys/BombShooter/Graphics/Projectiles/MissileCannon.prefab",
    "62e990209b10d374d89f70c6f578def0": "Assets/Monkeys/BombShooter/Graphics/Projectiles/RecursiveMIssile.prefab",
    "2e83ac1acdf198945a446f0066ddabd3": "Assets/Monkeys/BombShooter/Graphics/Projectiles/MoabMauler.prefab",
    "bec5a0304f1197143a998b22cc60f3e8": "Assets/Monkeys/BombShooter/Graphics/Projectiles/MoabAssassin.prefab",
    "e5edd901992846e409326a506d272633": "Assets/Monkeys/BombShooter/Graphics/Projectiles/MoabEliminator.prefab",
    "d15d481d5956b28449b8c87a578d8f07": "Assets/Monkeys/BombShooter/Graphics/Projectiles/Frag.prefab",
    "1d69785d9cd205b43b7457ae7276e52d": "Assets/Monkeys/BombShooter/Graphics/Projectiles/ClusterBombSmall.prefab",
    "4f037aa86b2789a448901265ade0bff4": "Assets/Monkeys/Projectile/Shrapnel.prefab",
    "7367b996679da4f4a900902bf8d2a76f": "Assets/Monkeys/WizardMonkey/Graphics/Projectiles/ArcaneShot.prefab",
    "2a3f218e6e1d23344bd4478294864ebb": "Assets/Monkeys/WizardMonkey/Graphics/Projectiles/IntenseMagic.prefab",
    "54bcc286971344146a1cec38858b6b16": "Assets/Monkeys/WizardMonkey/Graphics/Projectiles/ArcaneBlast.prefab",
    "5f1f09b0c6b6a3447b9e70e14d68869a": "Assets/Monkeys/WizardMonkey/Graphics/Projectiles/IntenseMagicBlast.prefab",
    "125ac5d653097974b9707f641258f71b": "Assets/Monkeys/WizardMonkey/Graphics/Projectiles/Fireball.prefab",
    "01dfdf7fe33be28409a9c2e1db9bbec0": "Assets/Monkeys/WizardMonkey/Graphics/Projectiles/DragonsBreath.prefab",
    "e79b286d1972c1d4aa342ebc9b7e135b": "Assets/Monkeys/WizardMonkey/Graphics/Projectiles/Shimmer.prefab",
    "7c001a7fcad3c6d48b14aacba03a98bc": "Assets/Monkeys/SuperMonkey/Graphics/Projectiles/LaserBlast.prefab",
    "904432f1c20b4b145998e26a6e56cbb1": "Assets/Monkeys/SuperMonkey/Graphics/Projectiles/LaserBlastSingle.prefab",
    "2f62958cc36daf74d88833447725e357": "Assets/Monkeys/SuperMonkey/Graphics/Projectiles/PlasmaBlast.prefab",
    "5737e26c93d5fc149ade2f7df1156c74": "Assets/Monkeys/SuperMonkey/Graphics/Projectiles/BlastSunAvatar.prefab",
    "95e1b845816b6f748af84449fe6b7a59": "Assets/Monkeys/SuperMonkey/Graphics/Projectiles/SunTempleBlast.prefab",
    "dcd6cd8511c9a03458a32f42f860882c": "Assets/Monkeys/SuperMonkey/Graphics/Projectiles/TrueSunGodBlast.prefab",
    "0b7323ccbaace054ea5a4a579e24b473": "Assets/Monkeys/SuperMonkey/Graphics/Projectiles/PlasmaBlastTechTerror.prefab",
    "187bc7112ccbf6445afc2ef9173b4568": "Assets/Monkeys/SuperMonkey/Graphics/Projectiles/PlasmaBlastAntiBloon.prefab",
    "10a699233c3aca84f96feefa84806515": "Assets/Monkeys/SuperMonkey/Graphics/Projectiles/BoomerangDarkKnight.prefab",
    "68323af29dd7d4347b0a6004b7077c90": "Assets/Monkeys/SuperMonkey/Graphics/Projectiles/BoomerangDarkChampion.prefab",
    "1de5b2336a21cb24085c4282e0c0dc7a": "Assets/Monkeys/SuperMonkey/Graphics/Projectiles/LaserBoomerangDarkKnight.prefab",
    "5d88a6eeaf733324ea8fcfc9d19013b3": "Assets/Monkeys/SuperMonkey/Graphics/Projectiles/LaserBoomerangDarkChampion.prefab",
    "ae8cebf807b15984daf0219b66f42897": "Assets/Monkeys/SuperMonkey/Graphics/Projectiles/PlasmaBoomerangDarkKnight.prefab",
    "98010195051b16341bab67a674472835": "Assets/Monkeys/SuperMonkey/Graphics/Projectiles/PlasmaBoomerangDarkChampion.prefab",
    "a6d991e94ab3af1438e751afde2d12fa": "Assets/Monkeys/NinjaMonkey/Graphics/Projectiles/Shuriken.prefab",
    "bd9321403a5a6894c8cd923490317562": "Assets/Monkeys/NinjaMonkey/Graphics/Projectiles/SharpShuriken.prefab",
    "8ab0e3fbb093a554d84a85e18fe2acac": "Assets/Monkeys/NinjaMonkey/Graphics/Projectiles/Caltrop.prefab",
    "30884a46e9ec2c1449ebe063c5ddd01b": "Assets/Monkeys/NinjaMonkey/Graphics/Projectiles/StickyBomb.prefab",
    "bc94dd64030ba3b42a74e1197bddc901": "Assets/Monkeys/NinjaMonkey/Graphics/Projectiles/MasterStickyBomb.prefab",
    "e67d0eb1f64b594439d3e95ac5194cad": "Assets/Monkeys/GlueGunner/Graphics/Projectiles/GlueYellow.prefab",
    "97f2427a81f436547b0a59f37fb689da": "Assets/Monkeys/GlueGunner/Graphics/Projectiles/GlueGreen.prefab",
    "cd1d520bc58a70c4eba5464b0076dfb9": "Assets/Monkeys/GlueGunner/Graphics/Projectiles/GluePink.prefab",
    "c73fd08146403e14fbcebd3cbf600b88": "Assets/Monkeys/IceMonkey/Graphics/Projectiles/FreezeBomb.prefab",
    "ac08d93cfcd9d144189a14d22329c953": "Assets/Monkeys/IceMonkey/Graphics/Projectiles/ImpaleIcicle.prefab",
    "842be402795e7334cbc77d33b6746bff": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/Boomerang.prefab",
    "141984ad69ba88d4ab1806f9e31142d6": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/RedHotRang.prefab",
    "d2da044ddee17434995a137ce3384d50": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/KylieBoomerang.prefab",
    "c24ac3c7fcda4b8449eecf13be451029": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/MOABPressRang.prefab",
    "10b4e5dbc54fd9948a81dd537efc1521": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/KylieBoomerangMoabDom.prefab",
    "e5d8297178c8dee41b7e41083b19f914": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/TurboRang.prefab",
    "94cf50f570f687e48b98988b547eb9f0": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/PermaChargeRang.prefab",
    "a30cf1f2609e0a94dbdd39f4452ab918": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/HotPermaChargeRang.prefab",
    "3377716d4ebee174fa81f523b48272e2": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/Glaive.prefab",
    "33d6310eb4321284e869c1c289be7a39": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/HotGlaive.prefab",
    "592238314bcbac84095110bd8fdff922": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/KylieGlaiveRang.prefab",
    "26eb9a483ec69734687b61623776f9ba": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/KylieGlaiveRangMoabDom.prefab",
    "23ee303384f0ac543ace7a301829ea88": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/PermaChargeGlaive.prefab",
    "8e2e9ecfa47d2874db4b2c3e3b7a3eb7": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/GlaiveLordGlaive.prefab",
    "87629b86f2051a64bbd04c9509437f11": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/HotGlaiveLordGlaive.prefab",
    "e23d594d3bf5af44c8b1e2445fe10a9e": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/GlaiveLordGlaiveOrbit.prefab",
    "8dd1a46e669f8194aa4b595d265251a9": "Assets/Monkeys/BoomerangMonkey/Graphics/Projectiles/HotGlaiveLordGlaiveOrbit.prefab",
    "6f709716c169ccf4b8e4169bb868d190": None,  # Inferno Ring burst
    "595c7fb1d3705aa478d0d5171b74fb57": "Assets/Monkeys/BananaFarm/Graphics/Projectiles/Banana.prefab",
    "5ea56473fe0c6a441b0ab5891b3aa2ba": "Assets/Monkeys/BananaFarm/Graphics/Projectiles/LongLifeBanana.prefab",
    "88442e0b3684e3446aaa70a036da69c9": "Assets/Monkeys/BananaFarm/Graphics/Projectiles/BananaCrate.prefab",
    "ea93504240deadd41aed8f22737322c9": None,  # Empty.prefab: Marketplace cash goes straight to the player
    "0d60d713eef3d3043915b89b35b04670": "Assets/Monkeys/BananaFarm/Graphics/Projectiles/GreenBananaCrate.prefab",
    "2ed0c946c149d5549b05a957fc2d3fcc": "Assets/Monkeys/MonkeyVillage/graphics/Projectiles/Shuriken.prefab",
    # Druid: matched by the crosspaths each guid is used on (thorn colour follows Heart of
    # Thunder = purple, Heart of Oak = green, Poplust / Avatar of Wrath at bottom 4 / 5).
    "294129f6c175f2b4c90199546271dc16": "Assets/Monkeys/Druid/Graphics/Projectiles/Thorn.prefab",
    "ec1db85d6dcc90f40bc2196456efba73": "Assets/Monkeys/Druid/Graphics/Projectiles/GreenThorn.prefab",
    "ea59545b04899d940a945f5adc1f1df4": "Assets/Monkeys/Druid/Graphics/Projectiles/PurpleThorn.prefab",
    "77ed95a7695ee2146bca1345c62c2c3c": "Assets/Monkeys/Druid/Graphics/Projectiles/PurpleGreenThorn.prefab",
    "0df8048a3f0163841bda4750bab438d2": "Assets/Monkeys/Druid/Graphics/Projectiles/PoplustThorn.prefab",
    "dac321e299fa10c468d66187d3e0e34c": "Assets/Monkeys/Druid/Graphics/Projectiles/GreenPoplustThorn.prefab",
    "25bc9e98539ed084ca99da5981760981": "Assets/Monkeys/Druid/Graphics/Projectiles/PurplePoplustThorn.prefab",
    "5a7af3162d20c8745a946b6c24681248": "Assets/Monkeys/Druid/Graphics/Projectiles/AvatarOfWrathBolt.prefab",
    "efc680bebf80d1e4584e9548ddfbff34": "Assets/Monkeys/Druid/Graphics/Effects/DruidStormTornado.prefab",
    "2a1d60690a632f543b80280219205247": "Assets/Monkeys/Druid/Graphics/Effects/SuperStormTornado.prefab",
    "88399aeca4ae48a44aee5b08eb16cc61": "Assets/Monkeys/Druid/Graphics/Projectiles/BallLightning.prefab",
    "06482eb308b5a00438e2b41deaaef68e": "Assets/Monkeys/Druid/Graphics/Projectiles/JungleVineRoot.prefab",
    "d29df05c9a3c78d4683be69327f2a8e1": "Assets/Monkeys/Druid/Graphics/Projectiles/HotJungleVineRoot.prefab",
    # Alchemist, matched the same way (Larger Potions top, Stronger Acid purple, Perishing green).
    "f28edb7bbc570c14fa286e923fbc508e": "Assets/Monkeys/Alchemist/Graphics/Projectiles/RedPotion.prefab",
    "2d2e501175f7847419565f8802024910": "Assets/Monkeys/Alchemist/Graphics/Projectiles/LargerPotion.prefab",
    "9660dfdc86013eb4a8395299a2a89827": "Assets/Monkeys/Alchemist/Graphics/Projectiles/StrongerAcidPotion.prefab",
    "9dff5a1553071044eb7d80b21b914805": "Assets/Monkeys/Alchemist/Graphics/Projectiles/LargerPotionPurple.prefab",
    "2ae4eb6e05394b54580b7c64e39ecac9": "Assets/Monkeys/Alchemist/Graphics/Projectiles/PerishingPotion.prefab",
    "d82820b12ceaf0a4ead3dc66d7e545d0": "Assets/Monkeys/Alchemist/Graphics/Projectiles/PerishingLarger.prefab",
    "e745ba1bd52a1254ebbab8c47ef17c27": "Assets/Monkeys/Alchemist/Graphics/Projectiles/UnstableConcoctionPotion.prefab",
    "648119cdb392ef843b78ea806a986ab0": "Assets/Monkeys/Alchemist/Graphics/Projectiles/RubbertoGoldPotion.prefab",
    "a07b6cec1ba58cf4290e285cc35cab18": "Assets/Monkeys/Alchemist/Graphics/Projectiles/BloonMasterPotion.prefab",
    "7e672209db39b9e4db63c13dbe11cad5": "Assets/Monkeys/Alchemist/Graphics/Projectiles/PlasmaBlueBlasts.prefab",
    "50d818a6ef055384ba9eff777d58e0da": "Assets/Monkeys/Alchemist/Graphics/Projectiles/AcidicMixtureDip.prefab",
    "5d67b291141a7da4baad684559b74206": "Assets/Monkeys/Alchemist/Graphics/Projectiles/BerserkerBrewPotion.prefab",
    "ee94628d235bbf84ea54c91d4a022be5": "Assets/Monkeys/Alchemist/Graphics/Projectiles/StrongerStimulantPotion.prefab",
    "cf3c8662b1f8b764e90be242182f2076": "Assets/Monkeys/Alchemist/Graphics/Projectiles/PermanentBrew.prefab",
    "2d6f7419734e4d84da41c0fb8a0fbece": "Assets/Monkeys/Alchemist/Graphics/Effects/RedAcidPuddle.prefab",
    "fb9bb5edadd74aa42bbe0ef2d5eb8b60": "Assets/Monkeys/Alchemist/Graphics/Effects/PurpleAcidPuddle.prefab",
    # Desperado's shotgun pellets (x-x-3+; the 4-tier crosspath variant uses the same art here).
    "f84944ef749de5f4c819cd67537d804c": "Assets/Monkeys/Desperado/Graphics/Effects/ShotgunProjectile.prefab",
    "16b6c4224d027c0458d937e69cdc8b6f": "Assets/Monkeys/Desperado/Graphics/Effects/ShotgunProjectile.prefab",
}


# SetSpriteFromPierceModel.sprites lists (spike piles that shrink as their pierce is used), keyed by
# their first GUID. INFERRED like the table above: each list belongs to one tier family, and entry i
# is <family>_<i+1>.prefab in that folder (a one-entry list is the family prefab itself).
SPRITE_LISTS = {
    "ff7ada25698ff484f87767a8c4b01865": "Assets/Monkeys/SpikeFactory/Graphics/projectiles/SpikePile",
    "9dccc16d26c1c8a45b129e2a8cbd17ba": "Assets/Monkeys/SpikeFactory/Graphics/projectiles/HotSpikePile",
    "af2f6ef7fc0b7d24ea1346f571d508f4": "Assets/Monkeys/SpikeFactory/Graphics/projectiles/SpikedBall",
    "2f7dc9e2f77bec043aa0e7b7c4c2ee38": "Assets/Monkeys/SpikeFactory/Graphics/projectiles/SpikedMines",
    "8fd6d597625658647a18e761864a14d3": "Assets/Monkeys/SpikeFactory/Graphics/projectiles/SuperMines",
    "5a356f52744fe444b9436a2be1fddfd3": "Assets/Monkeys/SpikeFactory/Graphics/projectiles/LongLifeSpike",
    "6d356a7488c2ca7488ac88f24fdbb136": "Assets/Monkeys/SpikeFactory/Graphics/projectiles/DeadlySpike",
    "fe29f18e0420294488591156005e3eb5": "Assets/Monkeys/SpikeFactory/Graphics/projectiles/PermaSpike",
    "a94b853ef9ca42d4a894526d37231110": "Assets/Monkeys/SpikeFactory/Graphics/projectiles/HotLongLifeSpike",
    "8df085d0bd121e2458b2387fc159fc5e": "Assets/Monkeys/SpikeFactory/Graphics/projectiles/HotDeadlySpike",
    "d1126e17df528ec45971cbb62e331e16": "Assets/Monkeys/SpikeFactory/Graphics/projectiles/HotPermaSpike",
}


def projectiles(proj, depth=0):
    """Display GUIDs of a projectile and its on-exhaust / on-contact sub-projectiles."""
    out = []
    for b in items(proj["f"]["behaviors"]):
        if not b:
            continue
        if b["type"] == "DisplayModel" and b["f"].get("display"):
            out.append(b["f"]["display"])
        if b["type"] == "SetSpriteFromPierceModel":
            sprites = [s for s in (deref(b["f"]["sprites"]) or []) if isinstance(s, str)]
            family = SPRITE_LISTS.get(sprites[0]) if sprites else None
            for i, s in enumerate(sprites):
                if family and s not in PROJECTILE_BY_GUID:
                    PROJECTILE_BY_GUID[s] = family + (f"_{i + 1}" if len(sprites) > 1 else "") + ".prefab"
            out += sprites
        if depth < 3 and b["type"] in ("CreateProjectileOnExhaustFractionModel", "CreateProjectileOnContactModel",
                                       "EmitOnDamageModel"):
            sub = deref(b["f"].get("projectile"))
            if sub:
                out += projectiles(sub, depth + 1)
        # Sub-towers it places (Engineer sentries): their own shots.
        if depth < 3 and b["type"] in ("CreateTowerModel", "CreateTypedTowerModel"):
            for k in ("tower", "crushingTower", "boomTower", "coldTower", "energyTower"):
                t = deref(b["f"].get(k))
                for a in items(t["f"]["behaviors"]) if t else []:
                    if a and a["type"] == "AttackModel":
                        for w in items(a["f"]["weapons"]):
                            sub = w and deref(w["f"].get("projectile"))
                            if sub:
                                out += projectiles(sub, depth + 1)
    return out


os.makedirs("local_extracted/display_plans", exist_ok=True)
for tower in TOWERS:
    plan = {"towers": {}, "projectiles": {}}
    missing, unknown = [], set()
    for r in objs.values():
        if r["type"] != "TowerModel" or r["f"]["baseId"] != tower:
            continue
        # Only the tower's own tiers ("Alchemist", "Alchemist-200", its Paragon): other models
        # sharing the baseId (TransformedMonkey...) would overwrite their tier keys.
        name = r["f"]["_name"]
        if not (name == tower or name.startswith((tower + "-", tower + " ")) or "Paragon" in name):
            continue
        # A Paragon's model is exported separately (export_tower_assets.py); its projectiles
        # still go in the plan.
        paragon = "Paragon" in r["f"]["_name"]
        key = "paragon" if paragon else "".join(str(x) for x in items(r["f"]["tiers"]))
        # Prefer the tower's own graphics over same-named shop-shelf or effect prefabs.
        # catalog_prefabs.json only for GUIDs catalog_guids.json misses (Heli pads).
        d = r["f"]["display"]
        cand = sorted(by_base.get(guids.get(d) or (catalog_prefabs.get(d) if d not in guids else None), []),
                      key=lambda c: (f"/Monkeys/{tower}/" not in c[0], "/Effects/" in c[0], c[0]))
        if paragon:
            pass
        elif cand:
            plan["towers"][key] = {"prefab": cand[0][0], "bundle": cand[0][1]}
            # Attack displays: models an AttackModel adds on the tower (a Dartling's gun and its
            # gunner, Super Monkey's throwing arm, Navarch guns, Mermonkey tentacles). They turn
            # with the attack's aim. Unkeyed GUIDs: catalog_prefabs.json is right for those.
            attack = []
            ai = -1  # which attack (in behaviour order: game.json's attacks list)
            for b in items(r["f"]["behaviors"]):
                if not b or b["type"] not in ("AttackModel", "AttackAirUnitModel"):
                    continue
                ai += 1
                for x in items(b["f"].get("behaviors")):
                    if not x or x["type"] != "DisplayModel":
                        continue
                    g = x["f"].get("display")
                    nm = guids.get(g) or catalog_prefabs.get(g)
                    if not nm or nm == "EmptyPrefab.prefab":
                        continue
                    c2 = sorted(by_base.get(nm, []), key=lambda c: (f"/Monkeys/{tower}/" not in c[0], "/Effects/" in c[0], c[0]))
                    if not c2:
                        continue
                    off = deref(x["f"].get("positionOffset")) or {}
                    off = off.get("f", off) if isinstance(off, dict) else {}
                    attack.append({"prefab": c2[0][0], "bundle": c2[0][1], "attack": ai,
                                   "offset": [off.get("x", 0.0), off.get("y", 0.0), off.get("z", 0.0)],
                                   "scale": x["f"].get("scale", 1.0)})
            if attack:
                plan["towers"][key]["attack_displays"] = attack
        else:
            missing.append(key)
        for b in items(r["f"]["behaviors"]):
            projs = []
            if b and b["type"] in ("AttackModel", "AttackAirUnitModel"):
                projs = [deref(w["f"]["projectile"]) for w in items(b["f"]["weapons"])]
            if b and b["type"] == "OrbitModel":
                projs = [deref(b["f"]["projectile"])]
            # Sun Temple / True Sun God: the attacks sacrifices add (AddAttackTowerMutatorModel).
            if b and b["type"].startswith("TempleTowerMutatorGroupTier"):
                for m in items(b["f"].get("mutators")):
                    am = m and m["type"] == "AddAttackTowerMutatorModel" and deref(m["f"].get("attackModel"))
                    if am:
                        projs += [deref(w["f"]["projectile"]) for w in items(am["f"]["weapons"])]
            # Display GUIDs a behaviour swaps in (DruidVengeanceEffectModel's weak / medium / epic
            # Avatar of Wrath bolts: *ProjectilePath fields).
            swapped = [v for k, v in (b["f"].items() if b else []) if k.endswith("ProjectilePath") and isinstance(v, str) and v]
            # Crit shots' own look (CritMultiplierModel.displayModel: Crossbow Master's crit arrow).
            if b and b["type"] in ("AttackModel", "AttackAirUnitModel"):
                for w in items(b["f"]["weapons"]):
                    for x in items(w["f"].get("behaviors")) if w else []:
                        dm = x and x["type"] == "CritMultiplierModel" and deref(x["f"].get("displayModel"))
                        if dm and dm["f"].get("display"):
                            swapped.append(dm["f"]["display"])
            for proj in projs + [None]:
                    for g in (projectiles(proj) if proj is not None else swapped):
                        # Only for GUIDs catalog_guids.py misses: catalog_prefabs.json is one entry
                        # off for the keys it has.
                        if g not in PROJECTILE_BY_GUID and g not in guids and g in catalog_prefabs:
                            # The catalog's own GUID -> prefab name (catalog_audio.py ... prefab).
                            cand = sorted(by_base.get(catalog_prefabs[g], []),
                                          key=lambda c: (f"/Monkeys/{tower}/" not in c[0], "/Generated/" in c[0], c[0]))
                            if cand:
                                PROJECTILE_BY_GUID[g] = cand[0][0]
                        if g not in PROJECTILE_BY_GUID:
                            unknown.add(g)
                        elif g not in plan["projectiles"]:
                            prefab = PROJECTILE_BY_GUID[g]
                            plan["projectiles"][g] = {
                                "name": os.path.splitext(os.path.basename(prefab))[0] if prefab else None,
                                "prefab": prefab, "mapping": "inferred-by-tier-family"}
    # Geraldo's shop items that drop projectiles on the track (GeraldoCreateProjectileModel),
    # and the shots of the towers they make (Hot Sauce creature, Wererabbit, turrets).
    if tower == "Geraldo":
        shots = [p for r in objs.values() if r["type"] == "GeraldoCreateProjectileModel"
                 for p in [deref(r["f"].get("projectileModel"))] + list(items(r["f"].get("projectileModelUpgrades")))]
        shots += [deref(w["f"].get("projectile")) for r in objs.values()
                  if r["type"] == "TowerModel" and r["f"].get("geraldoItemName")
                  for a in items(r["f"]["behaviors"]) if a and a["type"] == "AttackModel"
                  for w in items(a["f"]["weapons"]) if w]
        for proj in shots:
            for g in (projectiles(proj) if proj else []):
                nm = guids.get(g) or catalog_prefabs.get(g)
                cand = sorted(by_base.get(nm, []), key=lambda c: ("/Monkeys/Geraldo/" not in c[0], c[0])) if nm else []
                if cand and g not in plan["projectiles"]:
                    plan["projectiles"][g] = {"name": os.path.splitext(os.path.basename(cand[0][0]))[0],
                                              "prefab": cand[0][0], "mapping": "geraldo-item"}
    json.dump(plan, open(f"local_extracted/display_plans/{tower}.json", "w"), indent=1)
    print(tower, len(plan["towers"]), "tower tiers;", len(plan["projectiles"]), "projectile displays;",
          "missing towers", missing, "unknown projectile guids", sorted(unknown))
