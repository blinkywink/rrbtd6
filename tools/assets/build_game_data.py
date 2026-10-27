"""Assemble the prototype's game.json from decoded game data.

Inputs (local, git-ignored): local_extracted/decoded/named/objects.jsonl,
local_extracted/decoded/values.json, local_extracted/data/round_sets.json,
local_extracted/data/bloon_mutator_data.json (optional),
local_extracted/maps/<key>/map.json, local_extracted/sprites/.

Usage: python -I tools/assets/build_game_data.py <local_extracted dir> <map key> <root GameModel index>
Writes <local_extracted>/game/game.json.
"""
import json
import re
import struct
import math
import os
import sys


# Towers whose attacks the prototype's rules cover (straight-travel projectiles, arc/parallel
# emissions, stationary rings, on-contact explosions and frags, instant shots, bounces, follow paths).
PLAYABLE = ["DartMonkey", "TackShooter", "BombShooter", "SniperMonkey", "BoomerangMonkey", "IceMonkey", "GlueGunner", "NinjaMonkey",
            "WizardMonkey", "SuperMonkey", "SpikeFactory", "BananaFarm", "MonkeyVillage", "Druid", "Alchemist", "Desperado",
            "MortarMonkey", "DartlingGunner", "EngineerMonkey", "HeliPilot", "MonkeyAce", "BeastHandler", "Skywarden", "MonkeySub", "MonkeyBuccaneer", "Mermonkey"]
# Heroes: TowerModels "<id>", "<id> 2" ... "<id> 20" with tiers [level, 0, 0]; levels come from XP.
HEROES = ["Quincy", "Gwendolin", "StrikerJones", "ObynGreenfoot", "CaptainChurchill", "Benjamin", "Ezili", "PatFusty",
          "Adora", "AdmiralBrickell", "Etienne", "Sauda", "Psi", "Geraldo", "Corvus", "Rosalia", "Silas", "DanDMonke"]
PLAYABLE += HEROES


def main():
    root, map_key, root_idx = sys.argv[1], sys.argv[2], int(sys.argv[3])
    objs = {}
    for line in open(os.path.join(root, "decoded", "named", "objects.jsonl"), encoding="utf-8"):
        r = json.loads(line)
        objs[r["i"]] = r
    values = {int(k): v for k, v in json.load(open(os.path.join(root, "decoded", "values.json"))).items()}

    def deref(v):
        if isinstance(v, dict) and "$ref" in v:
            i = v["$ref"]
            if i in objs:
                return objs[i]
            return values.get(i)
        return v

    def items(v):
        """Elements of a referenced array/list: objects or plain values."""
        c = deref(v)
        if c is None:
            return []
        if isinstance(c, list):
            return c
        if isinstance(c, dict) and "items" in c:
            return [objs.get(i) if i in objs else values.get(i) for i in c["items"] if i]
        return []

    # AudioClip GUID -> clip name (catalog_audio.py); SoundModel.assetId and the bloon sound ids use it.
    audio_path = os.path.join(root, "catalog_audio.json")
    audio = json.load(open(audio_path)) if os.path.exists(audio_path) else {}

    def snd(v):
        o = deref(v)
        g = o["f"].get("assetId") if isinstance(o, dict) and o.get("type") == "SoundModel" else o
        n = audio.get(g) if isinstance(g, str) else None
        return os.path.splitext(n)[0] if n else None

    def sounds(b, keys):
        """Clip names in fields `keys` of behaviour `b` (SoundModel refs or GUID strings / lists)."""
        out = []
        for k in keys:
            v = b["f"].get(k)
            dv = deref(v)
            for x in (items(v) if isinstance(dv, list) or (isinstance(dv, dict) and "items" in dv) else [v]):
                s = snd(x)
                if s:
                    out.append(s)
        return out

    def behavior_sounds(behaviors, kind, keys):
        return [s for b in behaviors if b and b["type"] == kind for s in sounds(b, keys)]

    SOUND_KEYS = ["sound", "sound1", "sound2", "sound3", "sound4", "sound5"]

    # Display GUID -> prefab (catalog_audio.py ... prefab): MOAB-class bloons are 3D prefabs.
    prefab_path = os.path.join(root, "catalog_prefabs.json")
    prefabs = json.load(open(prefab_path)) if os.path.exists(prefab_path) else {}

    # Keyed GUIDs (catalog_guids.py) first: catalog_prefabs.json is one entry off for those
    # (INFERRED from tower displays); it is right for the keys catalog_guids.py misses.
    guid_path = os.path.join(root, "catalog_guids.json")
    keyed = json.load(open(guid_path)) if os.path.exists(guid_path) else {}

    # HYPOTHESIS P-CATSHIFT: where catalog_prefabs.json is shifted (the next GUID's keyed name
    # disagrees with its catalog name), a GUID's prefab is the next entry's catalog name.
    plist = list(prefabs.items())
    shifted = {}
    for (g, _), (g2, n2) in zip(plist, plist[1:]):
        if g not in keyed and g2 in keyed and keyed[g2] != n2:
            shifted[g] = n2

    def prefab_name(guid):
        n = keyed.get(guid or "") or shifted.get(guid or "") or prefabs.get(guid or "") or ""
        return n[:-len(".prefab")] if n.endswith(".prefab") else None

    def model3d(guid):
        n = prefab_name(guid) or ""
        return n if "3D" in n else None

    sprites = set(os.path.splitext(f)[0] for f in os.listdir(os.path.join(root, "sprites", "Bloons")))
    icons = set(os.path.splitext(f)[0] for f in os.listdir(os.path.join(root, "sprites", "UiBloonIcons")))

    # GameModel.bloons order: ProgressiveDifficultyManager.ChangeCeramicsAndChildren walks it.
    game_model = next(r for r in objs.values() if r["type"] == "GameModel")
    model_index = {b["f"]["id"]: k for k, b in enumerate(items(game_model["f"].get("bloons"))) if b}
    bloons = {}
    for r in objs.values():
        if r["type"] != "BloonModel":
            continue
        f = r["f"]
        children = []
        for b in items(f.get("behaviors")):
            if b and b.get("type") == "SpawnChildrenModel":
                children = items(b["f"].get("children"))
        sprite = f["id"] if f["id"] in sprites else None
        icon = f.get("icon") or ""
        icon_name = icon.split("[")[-1].rstrip("]") if "[" in icon else None
        bbs = items(f.get("behaviors"))
        pe = next((b["f"] for b in bbs if b and b["type"] == "PopEffectModel"), None)
        pop_sound = None
        if pe:
            names = [s for s in (snd(pe.get(f"soundEffect{k}Id")) for k in range(1, 5)) if s]
            if names:
                pop_sound = {"names": names, "group": pe.get("soundEffectGroupID") or "", "limit": pe.get("soundEffectGroupLimit", 1),
                             "frames": pe.get("soundEffectTimeDelay", 0), "frames_ff": pe.get("soundEffectTimeDelayFF", 0)}
        bloons[f["id"]] = {
            "model": model3d(f.get("display")),
            "damage_models": [{"model": model3d(x["f"].get("displayPath")), "health": x["f"]["healthPercent"]}
                              for x in items(f.get("damageDisplayStates")) if x and model3d(x["f"].get("displayPath"))],
            "pop_sound": pop_sound,
            "hit_sounds": [s for b in bbs if b and b["type"] == "CreateSoundOnDamageBloonModel"
                           for s in sounds(b, ["sounds"])],
            "speed": f["speed"], "maxHealth": f["maxHealth"], "radius": f["radius"],
            # BloonModel.danger: what Strong targeting ranks by (Path.FindBestBloon).
            "danger": f.get("danger", 0.0),
            "isMoab": f["isMoab"], "isCamo": f["isCamo"], "leakDamage": f["leakDamage"],
            "bloonProperties": f.get("bloonProperties", 0), "tags": items(f.get("tags")),
            "layerNumber": f["layerNumber"], "children": children,
            "index": model_index.get(f["id"], len(model_index)),
            "isFixedMaxHealth": bool(f.get("isFixedMaxHealth", False)),
            # BadImmunityModel (BAD): no PushBack, and BloonMutatorData's mutators are kept off.
            "badImmunity": any(b and b["type"] == "BadImmunityModel" for b in items(f.get("behaviors"))),
            # CollisionGroupModel.collisionGroup (MOAB-class): circles (offset x, offset y, radius)
            # in the bloon's own frame, offset y along its heading (Targetting.CanHitMoab).
            "circles": [[c["f"].get("?None@0", 0.0), c["f"].get("offset", 0.0), c["f"].get("radius", 0.0)]
                        for c in items((deref(f.get("collisionGroup")) or {}).get("f", {}).get("collisionGroup"))
                        if c and c.get("type") == "CollisionGroupData"],
            # BloonModel.rotateToFollowPath: Bloon.UpdatePositionAlongTrack turns it towards the
            # path's heading (Math.LerpAngle by TotalSpeedFrames x 0.5) as it moves.
            "rotateToFollowPath": bool(f.get("rotateToFollowPath", False)),
            # BloonModel.distributeDamageToChildren: damage past its layer may go on to its children.
            "distributeDamageToChildren": bool(f.get("distributeDamageToChildren", False)),
            # GrowModel (regrow bloons): becomes growToId every `rate` seconds, up to its ceiling.
            "grow": next(({"to": b["f"].get("growToId") or "", "rate": b["f"].get("rate", 3.0)}
                          for b in bbs if b and b["type"] == "GrowModel"), None),
            "sprite": ("Bloons/" + sprite) if sprite else
                      (("UiBloonIcons/" + icon_name) if icon_name in icons else None),
        }

    # FreeplayBloonGroupModel (rounds past the round set, FreeplayRoundManager.GetRoundEmissions):
    # each group's bloons spread over its BloonGroupModel start..end (frames -> seconds here),
    # the rounds it may appear in, and its score (FreeplayBloonGroupModel.CalculateScore,
    # VERIFIED-DISASM @0x10d7110: a stored score if set, else the sum of each emission's
    # BloonModel.GetTotalLeakDamage x (1 + 0.1 per "Camo" / "Grow" tag), times 0.8 / 1 / 1.1 /
    # 1.4 / 1.8 by how tightly spaced it is).
    raw_bloons = {r["f"]["id"]: r["f"] for r in objs.values() if r["type"] == "BloonModel"}
    leak_cache = {}

    def total_leak(bid):
        # BloonModel.GetTotalLeakDamage (VERIFIED-DISASM @0x1154820): leakDamageSet if above 0,
        # else leakDamage + the children's totals; x leakDamageMultiplier, rounded half to even.
        if bid not in leak_cache:
            f = raw_bloons.get(bid) or {}
            t = f.get("leakDamageSet", 0.0)
            if not t > 0:
                t = f.get("leakDamage", 0.0) + sum(total_leak(c) for c in bloons.get(bid, {}).get("children", []))
            leak_cache[bid] = float(round(t * f.get("leakDamageMultiplier", 1.0)))
        return leak_cache[bid]

    def freeplay_groups():
        out = []
        # GameModel.freeplayGroups order: the seeded shuffle starts from it.
        gm = next(r for r in objs.values() if r["type"] == "GameModel")
        for r in items(gm["f"].get("freeplayGroups")):
            if not r or r["type"] != "FreeplayBloonGroupModel":
                continue
            f = r["f"]
            g = deref(f.get("group"))
            if not g:
                continue
            g = g["f"]
            n, start, end = int(g["count"]), g["start"], g["end"]
            bounds = [list(b) for b in (deref(f.get("bounds")) or []) if isinstance(b, list)]
            score = f.get("score", 0.0)
            if not score > 0:
                times = [start + (end - start) * 0.5] if n == 1 else [start + (end - start) / (n - 1) * i for i in range(n)]
                tags = items(raw_bloons.get(g["bloon"], {}).get("tags"))
                mult = 1.0 + 0.1 * sum(1 for t in tags if t in ("Grow", "Camo"))
                score = total_leak(g["bloon"]) * mult * n
                if n > 1:
                    d = max(times) / 60.0 / n
                    score *= 0.8 if d >= 1.0 else 1.0 if d >= 0.5 else 1.1 if d >= 0.1 else 1.4 if d >= 0.08 else 1.8
            out.append({"bloon": g["bloon"], "count": n, "start": start / 60.0, "duration": (end - start) / 60.0,
                        "bounds": bounds, "score": score, "name": f.get("_name", "")})
        return out

    def tag_mod(b):
        return {"tag": b["f"]["tag"], "mult": b["f"]["damageMultiplier"], "add": b["f"]["damageAddative"],
                "all": bool(b["f"].get("mustIncludeAllTags"))}

    def brew(pobj, extra_ignore):
        """AddAcidicMixtureToProjectileModel / AddBerserkerBrewToProjectileModel on a brew shot."""
        for x in items(pobj["f"]["behaviors"]):
            if not x or x["type"] not in ("AddAcidicMixtureToProjectileModel", "AddBerserkerBrewToProjectileModel"):
                continue
            bf = x["f"]
            acid = x["type"] == "AddAcidicMixtureToProjectileModel"
            ignore = [s.strip() for s in ((bf.get("ignoreList") or "") + "," + extra_ignore).split(",") if s.strip()]
            return {"kind": "acid" if acid else "berserk", "id": bf.get("mutationId") or x["type"],
                    "replaces": [s.strip() for s in (bf.get("mutatorsToRemove") or "").split(",") if s.strip()],
                    "shots": int(min(bf.get("cap", 1), 2**31)), "lifespan": 0.0 if acid else bf.get("lifespan", -1.0),
                    "damage": 0.0 if acid else bf.get("damageUp", 0.0), "pierce": 0.0 if acid else bf.get("pierceUp", 0.0),
                    "rate": 0.0 if acid else bf.get("rateUp", 0.0), "range": 0.0 if acid else bf.get("rangeUp", 0.0),
                    "tag_mods": [tag_mod(m) for m in items(deref(bf.get("projBehaviors"))) if m and m["type"] == "DamageModifierForTagModel"]
                    if acid else [],
                    "ignore": ignore}
        return None

    def projectile(pobj, depth=0):
        """Generic projectile description (recursive for sub-projectiles)."""
        f = pobj["f"]
        pb = {}
        for b in items(f["behaviors"]):
            if b:
                pb.setdefault(b["type"], []).append(b)
        dmg = (pb.get("DamageModel") or [{"f": {}}])[0]["f"]
        ts = (pb.get("TravelStraitModel") or [{"f": {}}])[0]["f"]
        age = (pb.get("AgeModel") or [{"f": {}}])[0]["f"]
        if not age and pb.get("AgeRandomModel"):
            # STAND-IN: AgeRandomModel's random lifespan in [min, max] as the middle value.
            ar = pb["AgeRandomModel"][0]["f"]
            age = dict(ar, lifespan=(ar.get("minLifespan", 0.0) + ar.get("maxLifespan", 0.0)) / 2)
        disp = (pb.get("DisplayModel") or [{"f": {}}])[0]["f"]
        if not ts and pb.get("TravelStraitSlowdownModel"):
            # HYPOTHESIS P-SLOWDOWN: the slowdown is ignored; speed * lifespan ~ maxDistance.
            sd = pb["TravelStraitSlowdownModel"][0]["f"]
            ts = {"speed": sd["speed"], "lifespan": sd.get("lifespan") or sd["maxDistance"] / sd["speed"]}
        # Hero homing shots (AdoraTrackTargetModel, SilasTrackTargetModel) accelerate from
        # minimumSpeed to maximumSpeed while turning `rotation` degrees per frame. STAND-IN
        # P-HEROTRACK: constant maximumSpeed, seeking like TrackTargetModel.
        hero_track = next((pb[k][0]["f"] for k in ("AdoraTrackTargetModel", "SilasTrackTargetModel") if pb.get(k)), None)
        if not ts and hero_track:
            ts = {"speed": hero_track.get("maximumSpeed", 300.0),
                  "lifespan": hero_track.get("lifespan") or age.get("lifespan") or 3.0}
        if not ts and pb.get("RandomRangeTravelStraitModel"):
            # HYPOTHESIS P-RANDRANGE: the random travel range is taken at its midpoint.
            rr = pb["RandomRangeTravelStraitModel"][0]["f"]
            ts = {"speed": rr["speed"], "lifespan": (rr["minRange"] + rr["maxRange"]) / 2 / rr["speed"]}
        filters, exclude_bloons, exclude_tags = [], [], []
        glue_filter, skip_frozen, cant_hit = 0, False, 0
        out_require_effects = None
        for fm in pb.get("ProjectileFilterModel", []):
            for x in items(fm["f"]["filters"]):
                if x:
                    filters.append(x["type"])
                    # FilterBloonIfDamageTypeModel: passes through bloons with these properties
                    # that its damage can't hurt.
                    if x["type"] == "FilterBloonIfDamageTypeModel":
                        cant_hit |= x["f"].get("ifCantHitBloonProperties", 0)
                    if x["type"] == "FilterOutBloonModel":
                        exclude_bloons.append(x["f"]["bloonId"])
                    if x["type"] == "FilterOutTagModel":
                        exclude_tags.append(x["f"]["tag"])
                    if x["type"] == "FilterGlueLevelModel":
                        glue_filter = x["f"]["glueLevel"]
                    if x["type"] == "FilterFrozenBloonsModel":
                        skip_frozen = True
                    # FilterOveridingMutatedTargetModel: [mutationOverideOrder, highest, default]
                    # (FilterOveridingMutatedTarget.FilterBloon).
                    if x["type"] == "FilterOveridingMutatedTargetModel":
                        out_require_effects = [[m for m in items(x["f"].get("mutationOverideOrder")) if isinstance(m, str)],
                                               x["f"].get("highestPriorityMutationId") or "", x["f"].get("defaultMutationId") or ""]
        # Bloon effects (slow, freeze, damage over time, brittle) and their per-tag modifiers.
        effects, effect_mods = [], []
        for s in pb.get("SlowModel", []) + pb.get("SlowForBloonModel", []):
            sf = s["f"]
            tags = []
            bloon_id, bloon_ids = "", []
            if s["type"] == "SlowForBloonModel":
                # SlowForBloon.Collide (VERIFIED-DISASM @0xcfbf30): the bloon's id must contain
                # bloonId (one id) or be contained in one of bloonIds (several), and it must have
                # bloonTag; excluding turns each test round.
                neg = "!" if sf.get("excluding") else ""
                tags = [neg + t for t in ([sf["bloonTag"]] if sf.get("bloonTag") else [])]
                bloon_id = sf.get("bloonId") or ""
                bloon_ids = [t for t in items(sf.get("bloonIds")) if isinstance(t, str)]
            effects.append({"kind": "slow", "id": sf.get("mutationId") or "", "mult": sf.get("multiplier", 1.0),
                            "lifespan": sf.get("lifespan", 0.0), "layers": sf.get("layers", 1),
                            "glue": sf.get("glueLevel", 0), "refresh": not sf.get("dontRefreshDuration", False),
                            "tags": tags, "keep_on_degrade": bool(sf.get("dontRemoveOnBloonDegrade", False)),
                            "bloon_id": bloon_id, "bloon_ids": bloon_ids, "excluding": bool(sf.get("excluding")),
                            # Slow.Collide (VERIFIED-DISASM @0xcfce70): removeMutatorIfNotMatching
                            # takes off a same-id mutator another SlowModel put on.
                            "replace": bool(sf.get("removeMutatorIfNotMatching", False))})
        for s in pb.get("FreezeModel", []):
            sf = s["f"]
            effects.append({"kind": "freeze", "id": sf.get("mutationId") or "", "mult": sf.get("speed", 0.0),
                            "lifespan": sf.get("lifespan", 0.0), "layers": sf.get("layers", 1),
                            "moabs": sf.get("canFreezeMoabs", False), "boss_immunity": bool(sf.get("bossImmunity", False)),
                            "after_damage": bool(sf.get("applyAfterDamage", False))})
        # PsiEffectModel (Psi): the hit bloon is held and takes damagePerTick every tickRate
        # seconds for up to maxDuration. HYPOTHESIS P-PSIHOLD: held = speed 0 meanwhile.
        for s in pb.get("PsiEffectModel", []):
            sf = s["f"]
            effects.append({"kind": "dot", "id": "Psi", "lifespan": sf.get("maxDuration", 15.0), "layers": 9999,
                            "damage": sf.get("damagePerTick", 1), "interval": sf.get("tickRate", 1.0), "immune": 0,
                            "only_if_damaged": False, "moabs_only": False})
            effects.append({"kind": "slow", "id": "PsiHold", "mult": 0.0, "lifespan": sf.get("maxDuration", 15.0),
                            "layers": 9999, "glue": 0, "refresh": True, "tags": []})
        for s in pb.get("AddBehaviorToBloonModel", []):
            sf = s["f"]
            flt = deref(sf.get("filter"))
            n0 = len(effects)
            for d in items(sf.get("behaviors")):
                if d and d["type"] == "DamageOverTimeModel":
                    df = d["f"]
                    effects.append({"kind": "dot", "id": sf.get("mutationId") or "", "lifespan": sf.get("lifespan", 0.0),
                                    "layers": sf.get("layers", 1), "damage": df["damage"], "interval": df["interval"],
                                    "immune": df.get("immuneBloonProperties", 0),
                                    "trigger_immediate": bool(df.get("triggerImmediate", False)),
                                    "tag_mods": [tag_mod(m) for m in items(df.get("damageModifierModels"))
                                                 if m and m["type"] == "DamageModifierForTagModel"],
                                    "initial_delay": df.get("initialDelay", 0.0),
                                    # Mutable.AddMutator: an isUnique mutator the bloon already has
                                    # is left as it is (no new timer, damage or tick).
                                    "refresh": not sf.get("isUnique", False),
                                    "distribute": bool(df.get("distributeToChildren", False)),
                                    "distribute_override": bool(df.get("overrideDistributionBlocker", False)),
                                    "only_if_damaged": sf.get("applyOnlyIfDamaged", False),
                                    "parent": ({"damage": pd["f"].get("damage", 0.0), "immune": pd["f"].get("immuneBloonProperties", 0)}
                                               if (pd := deref(sf.get("parentDamageModel"))) else None),
                                    "moabs_only": bool(flt and flt["type"] == "FilterMoabModel" and not flt["f"].get("flip")),
                                    # FilterWithTag.FilterBloon (VERIFIED-DISASM @0xc5fbc0): the
                                    # tag (moabTag: MOAB-class) passes, or with inclusive fails.
                                    "tags": ([("!" if flt["f"].get("inclusive") else "") + ("Moabs" if flt["f"].get("moabTag") else flt["f"].get("tag"))]
                                             if flt and flt["type"] == "FilterWithTagModel" and (flt["f"].get("moabTag") or flt["f"].get("tag")) else []),
                                    # BehaviorMutator.dontRemoveOnBloonDegrade: kept when a MOAB-class
                                    # bloon pops (Bloon.RemoveAllMutators).
                                    "keep_on_degrade": bool(sf.get("dontRemoveOnBloonDegrade", False))})
                # DesperadoMarkModel (Desperado 0-4-0+): marked bloons take desperadoDamageMultiplier
                # damage from Desperados and pay popCashMultiplier per pop.
                if d and d["type"] == "DesperadoMarkModel":
                    df = d["f"]
                    effects.append({"kind": "mark", "id": sf.get("mutationId") or "DesperadoMark", "lifespan": sf.get("lifespan", 0.0),
                                    "layers": sf.get("layers", 1), "mult": df.get("desperadoDamageMultiplier", 1.0),
                                    "damage": df.get("popCashMultiplier", 1.0),
                                    "keep_on_degrade": bool(sf.get("dontRemoveOnBloonDegrade", False))})
                # PsiBloonBehaviorModel (Psi): the bloon is held and takes damagePerTick every
                # tickRate seconds. HYPOTHESIS P-PSIHOLD: held = speed 0 while it lasts.
                if d and d["type"] == "PsiBloonBehaviorModel":
                    df = d["f"]
                    effects.append({"kind": "dot", "id": sf.get("mutationId") or "Psi", "lifespan": sf.get("lifespan", 0.0),
                                    "layers": sf.get("layers", 1), "damage": df.get("damagePerTick", 1),
                                    "interval": df.get("tickRate", 1.0), "immune": 0, "only_if_damaged": False,
                                    "moabs_only": False})
                    effects.append({"kind": "slow", "id": "PsiHold", "mult": 0.0, "lifespan": sf.get("lifespan", 0.0),
                                    "layers": sf.get("layers", 1), "glue": 0, "refresh": True, "tags": []})
                # EmitOnPopModel (Ice shards, glue splat): when the bloon pops, its projectile
                # is fired where it popped.
                if d and d["type"] == "EmitOnPopModel" and depth < 3 and deref(d["f"].get("projectile")):
                    df = d["f"]
                    effects.append({"kind": "pop_emit", "id": sf.get("mutationId") or "", "lifespan": sf.get("lifespan", 0.0),
                                    "layers": sf.get("layers", 1),
                                    "projectile": projectile(deref(df["projectile"]), depth + 1),
                                    "emission": emission(deref(df.get("emission"))),
                                    "ignore_same_frame": bool(df.get("ignoreSameFrameDegrade", False))})
                # EmitOnDestroyModel on a bloon (Glue Liquefier's linger): when the bloon is
                # destroyed its projectile comes out there. dontCopy: degrading, only the bloon
                # itself (the first child) keeps it.
                if d and d["type"] == "EmitOnDestroyModel" and depth < 3 and deref(d["f"].get("projectile")):
                    df = d["f"]
                    effects.append({"kind": "destroy_emit", "id": sf.get("mutationId") or "", "lifespan": sf.get("lifespan", 0.0),
                                    "layers": sf.get("layers", 1),
                                    "projectile": projectile(deref(df["projectile"]), depth + 1),
                                    "emission": emission(deref(df.get("emission"))),
                                    "dont_copy": bool(sf.get("dontCopy", False))})
                # CarryProjectileModel (Ice Icicles): the bloon carries its projectile, which
                # sits where the bloon is and goes with the mutator.
                if d and d["type"] == "CarryProjectileModel" and depth < 3 and deref(d["f"].get("projectile")):
                    df = d["f"]
                    effects.append({"kind": "carry", "id": sf.get("mutationId") or "", "lifespan": sf.get("lifespan", 0.0),
                                    "layers": sf.get("layers", 1),
                                    "projectile": projectile(deref(df["projectile"]), depth + 1),
                                    "emission": emission(deref(df.get("emission")))})
                if d and d["type"] == "ProjectileOverTimeModel" and depth < 3:
                    df = d["f"]
                    effects.append({"kind": "emit", "id": sf.get("mutationId") or "", "lifespan": sf.get("lifespan", 0.0),
                                    "layers": sf.get("layers", 1), "interval": df["interval"],
                                    "projectile": projectile(deref(df["projectileModel"]), depth + 1)})
            # Projectile.CollideBloon runs the behaviours by collisionPass (the DamageModel's is
            # 0): a later pass goes on after the damage, on what the bloon is then and the
            # children it made (recording glue_005: a Super Glue hit that pops a Zebra leaves no
            # splat, the splat going on the Black and White after; INFERRED: the children get it
            # whatever cascadeMutators says, a Rainbow's second Zebra's White splats later).
            if sf.get("collisionPass", -1) >= 1:
                for e in effects[n0:]:
                    e["after_damage"] = True
        # RemoveDamageTypeModifierModel (Embrittlement, Relentless Glue): the hit bloon (and
        # `layers` of its children) loses these bloonProperties (Lead 1, Frozen 16) for
        # `lifespan`: sharp shots can pop it.
        for s in pb.get("RemoveDamageTypeModifierModel", []):
            sf = s["f"]
            if sf.get("bloonPropertiesToRemove"):
                effects.append({"kind": "unarmor", "id": sf.get("mutationId") or "RemoveDamageType", "lifespan": sf.get("lifespan", 0.0),
                                "layers": sf.get("layers", 1), "immune": sf["bloonPropertiesToRemove"]})
        # StripChildrenModel (Bloon Trojan): the hit bloon releases no children when it pops.
        if pb.get("StripChildrenModel"):
            effects.append({"kind": "strip", "id": "StripChildren", "lifespan": 9999999.0, "layers": 1})
        # IncreaseBloonWorthWithTierModel (Bloon Trojan): pops pay cashPerTier x its layer.
        for s in pb.get("IncreaseBloonWorthWithTierModel", []):
            effects.append({"kind": "worth", "id": s["f"].get("mutatorId") or "Worth", "lifespan": 9999999.0, "layers": 1,
                            "damage": s["f"].get("cashPerTier", 0.0)})
        for s in pb.get("AddBonusDamagePerHitToBloonModel", []):
            sf = s["f"]
            effects.append({"kind": "brittle", "id": sf.get("mutationId") or "", "lifespan": sf.get("lifespan", 0.0),
                            "layers": sf.get("layers", 1), "damage": sf.get("perHitDamageAddition", 0.0)})
        # TranceBloonModel (Mermonkey x-x-3+): the bloon leaves the track and circles the tower.
        for s in pb.get("TranceBloonModel", []):
            sf = s["f"]
            fake = (deref(sf.get("fakeDamage")) or {}).get("f") or {}
            effects.append({"kind": "trance", "id": "Trance", "lifespan": sf.get("duration", 0.0), "layers": 1,
                            "mult": sf.get("speedMultiplier", 1.0), "moab_mult": sf.get("moabOrbitSpeed", 1.0),
                            "radius": sf.get("orbitRadius", 30.0), "cooldown": sf.get("cooldown", 0.0),
                            "immune": fake.get("immuneBloonProperties", 0), "refresh": False})
        for s in pb.get("SlowMaimMoabModel", []):
            sf = s["f"]
            by_class = {k.upper(): sf.get(k + "Duration", 0.0) for k in ("moab", "bfb", "zomg", "ddt", "bad")}
            effects.append({"kind": "slow", "id": "SlowMaimMoab", "mult": sf.get("multiplier", 0.0), "lifespan": 0.0,
                            "layers": 1, "moabs_only": True, "by_class": by_class})
            if sf.get("bloonPerHitDamageAddition"):
                effects.append({"kind": "brittle", "id": "SlowMaimMoabDamage", "lifespan": 0.0, "layers": 1,
                                "damage": sf["bloonPerHitDamageAddition"], "moabs_only": True, "by_class": by_class})
        for s in pb.get("SlowModifierForTagModel", []):
            sf = s["f"]
            effect_mods.append({"tags": [sf["tag"]], "id": sf.get("slowId") or "", "mult": sf.get("slowMultiplier", 1.0),
                                "reset": sf.get("resetToUnmodified", False), "prevent": sf.get("preventMutation", False),
                                "lifespan": sf.get("lifespanOverride", 0.0), "time_mult": 1.0,
                                "not_tag": bool(sf.get("makeNotTag", False))})
        for s in pb.get("FreezeModifierForTagsModel", []):
            sf = s["f"]
            effect_mods.append({"tags": [t for t in items(sf.get("tags")) if isinstance(t, str)], "id": sf.get("freezeId") or "",
                                "mult": 1.0, "reset": False, "prevent": sf.get("preventMutation", False), "lifespan": 0.0,
                                "time_mult": sf.get("freezeTimeMultiplier", 1.0), "freeze": True})
        path = None
        if pb.get("FollowPathModel"):
            fp = pb["FollowPathModel"][0]["f"]
            path = {"points": [[q[0], q[1]] for q in deref(fp["path"])], "speed": fp["speed"],
                    "destroy_at_end": fp.get("destroyAtEndOfPath", True),
                    # FollowPathModel.easePath: walked instead; its x over its largest x picks the
                    # fraction of the path (Kylie Boomerang: out and back).
                    "ease": [[q[0], q[1]] for q in deref(fp["easePath"])] if deref(fp.get("easePath")) else None}
            ts = {"speed": fp["speed"]}
        arrive = None
        if pb.get("ArriveAtTargetModel"):
            af = pb["ArriveAtTargetModel"][0]["f"]
            arrive = {"time": af["timeToTake"], "expire": af.get("expireOnArrival", True),
                      "filter_moving": af.get("filterCollisionWhileMoving", False), "alt_speed": af.get("altSpeed", 0.0),
                      # ArriveAtTargetModel.curveSamples: Curve.Evaluate's samples (the share of the way).
                      "samples": [float(x) for x in (deref(af.get("curveSamples")) or [])]}
        cash = None
        if pb.get("CashModel"):
            cf = pb["CashModel"][0]["f"]
            cash = {"min": cf["minimum"], "max": cf["maximum"], "bonus": cf.get("bonusMultiplier", 0.0),
                    "pickup": (pb.get("PickupModel") or [{"f": {}}])[0]["f"].get("collectRadius"),
                    "bonus_shared": cf.get("distributeBonusIncome", False)}
        push = None
        if pb.get("PushBackModel"):
            pf = pb["PushBackModel"][0]["f"]
            push = {"amount": pf["pushAmount"], "tag": pf.get("tag") or "",
                    "only_if_damaged": bool(pf.get("onlyIfDamaged", False)),
                    "mult": {k: pf.get("multiplier" + k, 1.0) for k in ("MOAB", "BFB", "DDT", "ZOMG")}}
        out = {
            "pierce": f["pierce"], "max_pierce": f.get("maxPierce", 0.0), "radius": f["radius"],
            # ProjectileModel ctor: checkCollisionIntervalFrames = (int)(interval x 60), truncated.
            "check_frames": int(struct.unpack("<f", struct.pack("<f", float(f.get("checkCollisionInterval") or 0.0) * 60.0))[0]),
            "vs_blocker_radius": f.get("vsBlockerRadius", 0.0), "ignore_blockers": f.get("ignoreBlockers", False),
            # ProjectileModel.canCollisionBeBlockedByMapLos: a hit needs a clear line to the bloon.
            "los_blocked": bool(f.get("canCollisionBeBlockedByMapLos", False)),
            "damage": dmg.get("damage", 0.0), "immune": dmg.get("immuneBloonProperties", 0),
            # DamageModel.distributeToChildren / overrideDistributeBlocker.
            "distribute": bool(dmg.get("distributeToChildren", True)),
            "distribute_override": bool(dmg.get("overrideDistributeBlocker", False)),
            # DamageModel.ignoreImmunityDestroy: false = a hit on a bloon it can't damage destroys it.
            "immune_keep": bool(dmg.get("ignoreImmunityDestroy", False)),
            # Whether it has a DamageModel at all (AddBehaviorToBloon's applyOnlyIfDamaged asks it).
            "damage_model": bool(pb.get("DamageModel")),
            "speed": ts.get("speed", 0.0), "lifespan": ts.get("lifespan") or age.get("lifespan", 0.0),
            # Its AgeModel's lifespan (Projectile.lifespan for the exhaust duration fraction).
            "age_lifespan": pb["AgeModel"][0]["f"].get("lifespan") if pb.get("AgeModel") else None,
            "display": disp.get("display"),
            "tag_mods": [tag_mod(b) for b in pb.get("DamageModifierForTagModel", [])],
            # DamageModifierWrathModel (Avatar of Wrath): + total RBE on screen / rbeThreshold x
            # damage, between 0 and maxDamageBoost (DamageModifierWrath.GetDamageAdditive).
            "wrath": next(({"threshold": b["f"]["rbeThreshold"], "damage": b["f"]["damage"], "max": b["f"]["maxDamageBoost"]}
                           for b in pb.get("DamageModifierWrathModel", [])), None),
            "type_mods": [{"id": b["f"]["bloonId"], "mult": b["f"].get("damageMultiplier", 1.0),
                           "add": b["f"].get("damageAdditive", 0.0), "children": b["f"].get("includeChildren", False)}
                          for b in pb.get("DamageModifierForBloonTypeModel", [])],
            "pierce_sprites": [s for s in (deref((pb.get("SetSpriteFromPierceModel") or [{"f": {}}])[0]["f"].get("sprites")) or [])
                               if isinstance(s, str)],
            "round_time": bool(age.get("useRoundTime")),
            "rounds": age.get("rounds", 0),
            "rebound": bool(pb.get("MapBorderReboundModel")),
            # ProjectileBlockerCollisionReboundModel: bounces off blocker areas' edges.
            "blocker_rebound": next(({"clear": b["f"].get("clearCollidedWith", False)} for b in pb.get("ProjectileBlockerCollisionReboundModel", [])), None),
            "knockback": (pb.get("KnockbackModel") or [{"f": None}])[0]["f"],
            "filters": filters,
            "no_hit": "FilterAllModel" in filters,
            # FilterAllExceptTargetModel: hits only the bloon it was aimed at.
            # CollideOnlyWithTargetModel (Sniper, Boomerang glaives...): only its target.
            "only_target": "FilterAllExceptTargetModel" in filters or bool(pb.get("CollideOnlyWithTargetModel"))
                           or any(x and x["type"] == "FilterAllExceptTargetModel" for x in items(f.get("filters"))),
            "exclude_bloons": exclude_bloons,
            "exclude_tags": exclude_tags,
            "glue_filter": glue_filter,
            "skip_frozen": skip_frozen,
            "cant_hit": cant_hit,
            "override_filter": out_require_effects,
            "effects": effects,
            "effect_mods": effect_mods,
            "path": path,
            "arrive": arrive,
            "cash": cash,
            "push": push,
            "clear_hits": (pb.get("ClearHitBloonsModel") or [{"f": {}}])[0]["f"].get("interval"),
            # ClearHitBloonsWhenNoLongerCollidingModel (Trample, sword charges, banana peels): a
            # bloon can be hit again once it has stopped touching the projectile.
            "clear_off": bool(pb.get("ClearHitBloonsWhenNoLongerCollidingModel")),
            "spin": (pb.get("RotateModel") or [{"f": {}}])[0]["f"].get("angle", 0.0),
            "on_exhaust": [],
            "on_contact": [],
            "instant": bool(pb.get("InstantModel")),
            "retarget": None,
            "hit_sounds": behavior_sounds(items(f["behaviors"]), "CreateSoundOnProjectileCollisionModel", SOUND_KEYS),
            "expire_sounds": behavior_sounds(items(f["behaviors"]), "CreateSoundOnProjectileExpireModel", SOUND_KEYS),
            "exhaust_sounds": behavior_sounds(items(f["behaviors"]), "CreateSoundOnProjectileExhaustModel", SOUND_KEYS),
            # CreateSoundOnPickupModel (bananas, crates): played when the cash is collected.
            "pickup_sounds": behavior_sounds(items(f["behaviors"]), "CreateSoundOnPickupModel", SOUND_KEYS),
            # ShowTextOnHitModel (Crossbow Master's "Crit!"), ExpireProjectileAtScreenEdgeModel.
            "hit_text": bool(pb.get("ShowTextOnHitModel")),
            "expire_at_edge": bool(pb.get("ExpireProjectileAtScreenEdgeModel")),
        }
        for k in ("TrackTargetModel", "TrackTargetWithinTimeModel"):
            if pb.get(k):
                tf = pb[k][0]["f"]
                out["seek"] = {"distance": tf.get("distance", 0.0), "turn_rate": tf.get("turnRate", 360.0),
                               "max_angle": 360.0 if tf.get("ignoreSeekAngle") else tf.get("maxSeekAngle", 360.0),
                               "new_targets": tf.get("trackNewTargets", True)}
        if hero_track and "seek" not in out:
            out["seek"] = {"distance": 9999.0, "turn_rate": hero_track.get("rotation", 9.0) * 60.0,
                           "max_angle": 360.0, "new_targets": True}
        if pb.get("WindModel"):
            wf = pb["WindModel"][0]["f"]
            out["wind"] = {"min": wf["distanceMin"], "max": wf["distanceMax"], "chance": wf.get("chance", 1.0),
                           "moabs": wf.get("affectMoab", False), "tag_scale": wf.get("distanceScaleForTags", 0.0),
                           "tags": [t for t in (wf.get("distanceScaleForTagsTags") or "").split(",") if t]
                           + [t for t in items(wf.get("distanceScaleForTagsTagsList")) if isinstance(t, str)]}
        # Skywarden: WindlashModel blows bloons back distanceMin..Max (like WindModel);
        # ChilledModel slows non-MOABs to `multiplier` for slowDuration. STAND-IN P-SKYSTATES:
        # the stages, vacuum pull, shrapnel and Galvanized / Heat states aren't modelled.
        if pb.get("WindlashModel") and "wind" not in out:
            wf = pb["WindlashModel"][0]["f"]
            out["wind"] = {"min": wf.get("distanceMin", 0.0), "max": wf.get("distanceMax", 0.0), "chance": 1.0,
                           "moabs": bool(wf.get("affectMoab")), "tag_scale": 0.0, "tags": []}
        for c in pb.get("ChilledModel", []):
            cf = c["f"]
            effects.append({"kind": "slow", "id": "Skywarden:Chilled", "mult": cf.get("multiplier", 1.0),
                            "lifespan": cf.get("slowDuration", 0.0), "layers": 1, "glue": 0, "refresh": True,
                            "tags": [] if cf.get("affectMoab") else ["!Moabs"]})
        if pb.get("RemoveBloonModifiersModel"):
            rf = pb["RemoveBloonModifiersModel"][0]["f"]
            out["decamo"] = bool(rf.get("cleanseCamo"))
            out["decamo_needs_damage"] = bool(rf.get("cleanseOnlyIfDamaged", True))
        # TravelAlongPathModel (Sword Charge): moves along the track at `speed` for `lifespan`,
        # from its end back when `reverse`.
        if pb.get("TravelAlongPathModel"):
            tf = pb["TravelAlongPathModel"][0]["f"]
            out["track_travel"] = {"speed": tf.get("speed", 0.0), "lifespan": tf.get("lifespan", 0.0), "reverse": bool(tf.get("reverse"))}
        # ScaleDamageWithTimeModel (Mermonkey): damage grows with the time flown.
        if pb.get("ScaleDamageWithTimeModel"):
            sf = pb["ScaleDamageWithTimeModel"][0]["f"]
            out["scale_damage"] = [sf.get("baseDamage", 1.0), sf.get("scalePerSecond", 0.0), sf.get("maxDamage", 1.0)]
        out["state_mods"] = [{"states": ([sf["bloonState"]] if sf.get("bloonState") else [])
                              + [t for t in items(sf.get("bloonStates")) if isinstance(t, str)],
                              "mult": sf.get("damageMultiplier", 1.0), "add": sf.get("damageAdditive", 0.0)}
                             for sf in (s["f"] for s in pb.get("DamageModifierForBloonStateModel", []))]
        # SilasApplyRimeModel (Silas): hit bloons get Rime, a lasting slow (speedModifier, MOABs
        # moabSpeedModifier), and IncreaseDamageFromTypeModel adds `amount` damage to hits on
        # rimed bloons. STAND-IN P-RIME: Rime lasts until popped; its freeze / shatter explosion
        # isn't modelled.
        for s2 in pb.get("SilasApplyRimeModel", []):
            sf = s2["f"]
            rid = sf.get("rimeMutationId") or "Silas:Rime"
            effects.append({"kind": "slow", "id": rid, "mult": sf.get("speedModifier", 1.0), "lifespan": 9999.0,
                            "layers": 1, "glue": 0, "refresh": True, "tags": []})
            for x in items(sf.get("behaviors")):
                if x and x["type"] == "IncreaseDamageFromTypeModel":
                    out["state_mods"].append({"states": [rid], "mult": 1.0, "add": x["f"].get("amount", 0.0)})
        if pb.get("RetargetOnContactModel"):
            rf = pb["RetargetOnContactModel"][0]["f"]
            out["retarget"] = {"distance": rf["distance"], "max_bounces": rf["maxBounces"], "delay": rf["delay"],
                               "expire_if_none": rf.get("expireIfNoTargetFound", True)}
        if pb.get("LightningModel"):
            # HYPOTHESIS P-LIGHTNING: an instant shot that chains to the closest unhit bloon within
            # splitRange every `delay` until its pierce is used (the split into `splits` branches
            # isn't modelled).
            lf = pb["LightningModel"][0]["f"]
            out["instant"] = True
            out["retarget"] = {"distance": lf["splitRange"], "max_bounces": int(f["pierce"]), "delay": lf["delay"],
                               "expire_if_none": True}
            out["lifespan"] = max(out["lifespan"] or 0.0, lf["delay"] * (f["pierce"] + 1) + 0.1)
        if pb.get("AcidPoolModel"):
            ap = pb["AcidPoolModel"][0]["f"]
            out["lifespan"] = ap["lifespan"]
            if ap.get("lifespanIfMisses", 0.0) > 0:
                out["acid_pool"] = {"lifespan": ap["lifespanIfMisses"], "radius": ap["radiusIfMisses"], "pierce": ap["pierce"]}
        for s in pb.get("AddBehaviorToBloonModel", []):
            sf = s["f"]
            subs = [x for x in items(sf.get("behaviors")) + items(sf.get("filters")) if x]
            gold = next((x["f"] for x in subs if x["type"] == "GiveCashOnPopModel" and x["f"].get("flatCashAmount", 0) > 0), None)
            tag = next((x["f"]["tag"] for x in subs if x["type"] == "FilterWithTagModel"), None)
            if gold and tag:
                out["cash_on_pop"] = {"amount": gold["flatCashAmount"], "tag": tag}
        if pb.get("RefreshPierceModel"):
            out["refresh_pierce"] = pb["RefreshPierceModel"][0]["f"]["interval"]
        # RemoveMutatorsFromBloonModel: by key (contained in the id), and by mutatorIds ("=id",
        # the whole id: Icicles, the Bloon Solver).
        out["remove_effects"] = [k.strip() for r in pb.get("RemoveMutatorsFromBloonModel", [])
                                 for k in (r["f"].get("key") or "").split(",") if k.strip()] +                                 ["=" + k.strip() for r in pb.get("RemoveMutatorsFromBloonModel", [])
                                 for k in (r["f"].get("mutatorIds") or "").split(",") if k.strip()]
        if depth < 3 and pb.get("CreateProjectileOnIntervalModel"):
            cf = pb["CreateProjectileOnIntervalModel"][0]["f"]
            out["on_interval"] = {"frames": cf["intervalFrames"], "range": cf.get("range", 0.0),
                                  "only_if_target": cf.get("onlyIfHasTarget", True),
                                  # onlyEmitOnce (DanDMonke's net): one shot, then no more.
                                  "once": bool(cf.get("onlyEmitOnce")),
                                  "projectile": projectile(deref(cf["projectile"]), depth + 1),
                                  "emission": emission(deref(cf.get("emission")))}
        if depth < 3:
            for cc in pb.get("CreateProjectileOnContactModel", []):
                cf = cc["f"]
                out["on_contact"].append({"projectile": projectile(deref(cf["projectile"]), depth + 1),
                                          "emission": emission(deref(cf["emission"])),
                                          "pass_on_collided": cf.get("passOnCollidedWith", False),
                                          # dontCreateAtBloon: made where the shot is, not on the bloon.
                                          "at_shot": bool(cf.get("dontCreateAtBloon", False))})
            for cc in pb.get("EmitOnDamageModel", []):
                cf = cc["f"]
                out["on_contact"].append({"projectile": projectile(deref(cf["projectile"]), depth + 1),
                                          "emission": emission(deref(cf["emission"])), "on_damage": True})
            # EmitOnPopModel (Ice shards): only on hits that pop the bloon.
            for cc in pb.get("EmitOnPopModel", []):
                cf = cc["f"]
                if deref(cf.get("projectile")):
                    out["on_contact"].append({"projectile": projectile(deref(cf["projectile"]), depth + 1),
                                              "emission": emission(deref(cf.get("emission"))), "on_pop": True})
        if depth < 3:
            # Every CreateProjectileOnExhaustFractionModel (a Mortar shell has its explosion, burn
            # and camo / fortified strip as separate ones).
            out["on_exhaust"] = [{"fraction": ce["f"]["fraction"], "duration_fraction": ce["f"]["durationfraction"],
                                  "between_rounds": bool(ce["f"].get("canCreateInBetweenRounds")),
                                  "projectile": projectile(deref(ce["f"]["projectile"]), depth + 1),
                                  "emission": emission(deref(ce["f"]["emission"]))}
                                 for ce in pb.get("CreateProjectileOnExhaustFractionModel", [])]
            # CreateDistanceProjectileOnExhaustFractionModel (Skywarden arcs): the projectile gets
            # +piercePerInterval and +radiusPerInterval per distanceInterval flown.
            out["on_exhaust"] += [{"fraction": ce["f"]["fraction"], "duration_fraction": ce["f"]["durationfraction"],
                                   "between_rounds": bool(ce["f"].get("canCreateInBetweenRounds")),
                                   "projectile": projectile(deref(ce["f"]["projectile"]), depth + 1),
                                   "emission": emission(deref(ce["f"]["emission"])),
                                   "per_distance": [ce["f"].get("distanceInterval", 0.0), ce["f"].get("piercePerInterval", 0.0),
                                                    ce["f"].get("radiusPerInterval", 0.0)]}
                                  for ce in pb.get("CreateDistanceProjectileOnExhaustFractionModel", [])]
            # CreateProjectileOnExpireModel (First Strike's strike): spawned where it expires; a
            # target-only one inherits the expiring projectile's target.
            out["on_exhaust"] += [{"projectile": projectile(deref(ce["f"]["projectile"]), depth + 1),
                                   "emission": emission(deref(ce["f"].get("emission")))}
                                  for ce in pb.get("CreateProjectileOnExpireModel", []) if deref(ce["f"].get("projectile"))]
            # DestroyWhenCollidingWithModel (DanDMonke's net trap): after collisionCount contacts
            # it goes, firing subProjectile (as pierce running out).
            for ce in pb.get("DestroyWhenCollidingWithModel", []):
                cf = ce["f"]
                if cf.get("collisionCount"):
                    out["pierce"] = min(out.get("pierce") or cf["collisionCount"], cf["collisionCount"])
                if deref(cf.get("subProjectile")):
                    out["on_exhaust"] += [{"projectile": projectile(deref(cf["subProjectile"]), depth + 1),
                                           "emission": emission(deref(cf.get("projectileEmission")))}]
            # EmitOnDestroyModel (Glue linger, Desperado): spawned whenever the projectile goes.
            out["on_exhaust"] += [{"projectile": projectile(deref(ce["f"]["projectile"]), depth + 1),
                                   "emission": emission(deref(ce["f"].get("emission")))}
                                  for ce in pb.get("EmitOnDestroyModel", []) if deref(ce["f"].get("projectile"))]
            # HYPOTHESIS P-EXHAUSTPIERCE: CreateProjectileOnExhaustPierceModel with pierceInterval 1
            # (Rocket Storm missiles) spawns its projectile at every hit, like an on-contact one.
            for ce in pb.get("CreateProjectileOnExhaustPierceModel", []):
                cf = ce["f"]
                if deref(cf.get("projectile")):
                    out["on_contact"].append({"projectile": projectile(deref(cf["projectile"]), depth + 1),
                                              "emission": emission(deref(cf.get("emission")))})
        # CreateTowerModel / CreateTypedTowerModel (Engineer sentries): a sub-tower where it lands.
        subs = []
        for ct in pb.get("CreateTowerModel", []):
            subs.append(sub_tower(deref(ct["f"].get("tower")), None))
        for ct in pb.get("CreateTypedTowerModel", []):
            for kind in ("crushing", "boom", "cold", "energy"):
                subs.append(sub_tower(deref(ct["f"].get(kind + "Tower")), ct["f"].get(kind + "Display")))
        out["create_tower"] = [s for s in subs if s]
        # EatBloonModel (Bloon Trap): swallows bloons up to rbeCapacity, then pays out.
        eat = (pb.get("EatBloonModel") or [None])[0]
        out["eat"] = {"capacity": eat["f"]["rbeCapacity"], "cash_mult": eat["f"]["rbeCashMultiplier"]} if eat else None
        # CollideExtraPierceReductionModel: each hit uses this much more pierce.
        out["pierce_cost"] = 1.0 + sum(c["f"].get("extraAmount", 0) for c in pb.get("CollideExtraPierceReductionModel", []))
        # DamageModifierForCashAmountModel (VERIFIED-DISASM IsValid @0xcd2670): with at least
        # cashThreshold cash, damage x damageMultiplier + damageAdditive.
        cm = (pb.get("DamageModifierForCashAmountModel") or [None])[0]
        out["cash_damage"] = [cm["f"].get("damageMultiplier", 1.0), cm["f"].get("damageAdditive", 0.0),
                              cm["f"].get("cashThreshold", 0.0)] if cm else None
        # IgnoreDmgImmunityModel (Ice Paragon...): its damage ignores these bloon properties.
        for m in pb.get("IgnoreDmgImmunityModel", []):
            if m["f"].get("chance", 1.0) >= 1.0:
                out["immune"] &= ~m["f"].get("removeBloonProperties", 0)
        # AccelerateModel: speeds up by `acceleration` per second up to maxSpeed.
        acc = (pb.get("AccelerateModel") or [None])[0]
        out["accel"] = [acc["f"].get("acceleration", 0.0), acc["f"].get("maxSpeed", 0.0)] if acc and not acc["f"].get("decelerate") else None
        return out

    def emission(e):
        if e is None:
            return {"kind": "Single", "count": 1}
        f = e["f"]
        if e["type"] == "ArcEmissionModel":
            return {"kind": "Arc", "count": f.get("SliceSize", 1), "angle": f.get("angle", 0.0), "offset": f.get("offset", 0.0)}
        if e["type"] == "LineProjectileEmissionModel":
            return {"kind": "Line", "count": 1}
        # STAND-IN P-WERERABBIT: the Wererabbit leaps onto its target; its blast lands there.
        if e["type"] in ("InstantDamageEmissionModel", "WererabbitEmissionModel"):
            return {"kind": "AtTarget", "count": 1}
        if e["type"] == "RandomEmissionModel":
            return {"kind": "Random", "count": f.get("count", 1), "angle": f.get("angle", 0.0),
                    "offset_start": f.get("startOffset", 0.0)}
        if e["type"] == "ParallelEmissionModel":
            return {"kind": "Parallel", "count": f.get("count", 1), "spread": f.get("spreadLength", 0.0),
                    "offset_start": f.get("offsetStart", 0.0),
                    # (Emission's per-kind fields: offset = yOffset, random_angle = linear.)
                    "offset": f.get("yOffset", 0.0), "random_angle": 1.0 if f.get("linear") else 0.0}
        if e["type"] == "RandomTargetSpreadModel":
            return {"kind": "Single", "count": 1, "target_spread": f.get("spread", 0.0)}
        # MultiEmissionModel (Super Brittle): the normal shot plus secondaryProjectileModel
        # from the tower (a map-wide pulse on brittle bloons).
        if e["type"] == "MultiEmissionModel" and deref(f.get("secondaryProjectileModel")):
            return {"kind": "Single", "count": 1, "secondary": projectile(deref(f["secondaryProjectileModel"]), 1)}
        # RandomArcEmissionModel: SliceSize shots, each turned by up to randomAngle at random.
        if e["type"] == "RandomArcEmissionModel":
            return {"kind": "Single", "count": f.get("SliceSize", 1), "random_angle": f.get("randomAngle", 0.0)}
        # EmissionWithOffsetsModel (Dartling / Sub / Buccaneer barrels...): projectileCount shots
        # from its ThrowMarkerOffsetModels, each shot from the next one (spreadProjectilesAcrossMarkers).
        if e["type"] == "EmissionWithOffsetsModel":
            ms = [m["f"] for m in items(f.get("throwMarkerOffsetModels")) if m]
            return {"kind": "Single", "count": f.get("projectileCount", 1),
                    "markers": [[m.get("ejectX", 0.0), m.get("ejectY", 0.0), m.get("ejectZ", 0.0)] for m in ms]}
        return {"kind": "Single", "count": 1}

    def tier(tobj):
        f = tobj["f"]
        attacks = []
        abilities = []
        tower_behaviors = {b["type"] for b in items(f["behaviors"]) if b}

        def parse_attack(b):
            af = b["f"]
            camo = True
            rotate = any(x and x["type"] in ("RotateToTargetModel", "RotateToPointerModel", "RotateToTargetAirUnitModel") for x in items(af["behaviors"]))
            # RotateToTargetModel with onlyRotateDuringThrow and rotateOnlyOnThrow off: it turns to
            # its target every frame (RotateToTarget.Process), not only during a throw.
            rotate_always = any(x and x["type"] == "RotateToTargetModel" and not x["f"].get("onlyRotateDuringThrow", True)
                                and not x["f"].get("rotateOnlyOnThrow", False) for x in items(af["behaviors"]))
            # RotateToTargetModel.rotateTower = False (Carrier Flagship): it aims but the tower
            # itself keeps its facing.
            turn_body = not any(x and x["type"] == "RotateToTargetModel" and x["f"].get("rotateTower") is False for x in items(af["behaviors"]))
            # RotateToMiddleOfTargetsModel (Robo Monkey / Tech Terror / Anti-Bloon): the body faces
            # the midpoint of all its attacks' targets while each arm aims at its own.
            face_middle = any(x and x["type"] == "RotateToMiddleOfTargetsModel" for x in items(af["behaviors"]))
            # RotateToPointerModel.rate (degrees per second; Dartling 180, Mortar 9999999).
            turn_rate = next((x["f"].get("rate") for x in items(af["behaviors"]) if x and x["type"] == "RotateToPointerModel"), None)
            # Target*Model behaviours, in order: the modes the targeting buttons cycle through.
            # TargetPointerModel = "Pointer" (aims at the cursor), TargetSelectedPointModel = "Point".
            targets = []
            for x in items(af["behaviors"]):
                if not x:
                    continue
                for m in ("First", "Last", "Close", "Strong"):
                    if x["type"].replace("TargetDesperado", "Target").startswith("Target" + m) and m not in targets:
                        targets.append(m)
                for t, m in (("TargetPointerModel", "Pointer"), ("TargetSelectedPointModel", "Point"),
                             ("TargetEliteTargettingModel", "Elite")):
                    if x["type"] == t and m not in targets:
                        targets.append(m)
            # Spike Factory track modes (Smart Spikes and up): Normal / Close / Smart / Automatic,
            # with the set-point mode kept in its place among them.
            track_modes = {"TargetTrackModel": "Track", "CloseTargetTrackModel": "CloseTrack",
                           "SmartTargetTrackModel": "SmartTrack", "AutoTargetTrackModel": "AutoTrack",
                           "TargetSelectedPointModel": "Point"}
            tm = [track_modes[x["type"]] for x in items(af["behaviors"]) if x and x["type"] in track_modes]
            if "Track" in tm and len(tm) > 1:
                targets = tm
            require_tags, exclude_tags, skip_effects = [], [], []
            # TargetMoabModel (Bomb Shooter's "Boom" attack): MOAB-class bloons first.
            prio_moab = any(x and x["type"] == "TargetMoabModel" for x in items(af["behaviors"]))
            glue_filter, skip_frozen, cant_hit = 0, False, 0
            for x in items(af["behaviors"]):
                if x and x["type"] == "AttackFilterModel":
                    for flt in items(x["f"]["filters"]):
                        # FilterBloonIfDamageTypeModel: won't target bloons its shots can't hurt.
                        if flt and flt["type"] == "FilterBloonIfDamageTypeModel":
                            cant_hit |= flt["f"].get("ifCantHitBloonProperties", 0)
                        if flt and flt["type"] == "FilterInvisibleModel" and flt["f"].get("isActive", True):
                            camo = False
                        # INFERRED: inclusive=True filters the tag out (Alluring Melody's attack
                        # "entrances non MOAB-Class Bloons"; its projectile filters Moabs out too).
                        if flt and flt["type"] == "FilterWithTagModel":
                            (exclude_tags if flt["f"].get("inclusive") else require_tags).append(flt["f"]["tag"])
                        if flt and flt["type"] == "FilterOutTagModel":
                            exclude_tags.append(flt["f"]["tag"])
                        if flt and flt["type"] == "FilterGlueLevelModel":
                            glue_filter = flt["f"]["glueLevel"]
                        if flt and flt["type"] == "FilterFrozenBloonsModel":
                            skip_frozen = True
                        if flt and flt["type"] == "FilterMoabModel":
                            (exclude_tags if flt["f"].get("flip") else require_tags).append("Moabs")
                        if flt and flt["type"] == "FilterMutatedTargetModel" and not flt["f"].get("inverse"):
                            skip_effects.append(flt["f"]["mutationId"])
            track_target = next((x["f"].get("maxOffset", 0.0) for x in items(af["behaviors"])
                                 if x and x["type"] == "TargetTrackModel"), None)
            track_closest = False
            for x in items(af["behaviors"]):
                if x and x["type"] == "TargetTrackOrDefaultModel" and x["f"].get("forceTargetTrack"):
                    track_target = x["f"].get("maxTrackOffset", 0.0)
                    track_closest = bool(x["f"].get("useClosestTrack"))
                # CloseTargetTrackModel (Cleansing Foam, Bloon Trap): the track spot nearest the
                # tower; it is the attack's default, so the selectable point mode is left out
                # (STAND-IN: one target mode per attack).
                if x and x["type"] == "CloseTargetTrackModel" and "Track" not in targets:
                    track_target, track_closest = x["f"].get("maxOffset", 0.0), True
                    targets = [m for m in targets if m != "Point"]
            weapons = []
            for w in items(af["weapons"]):
                if not w:
                    continue
                crit, alternate, spread, per_round, burst = None, None, 0.0, None, None
                # EmissionOverTimeModel (Monkey Ace pineapples): `count` shots `timeBetween` apart.
                # STAND-IN P-OVERTIME: initialFireDelay isn't modelled.
                eot = deref(w["f"].get("emission"))
                if eot and eot["type"] == "EmissionOverTimeModel":
                    burst = {"count": eot["f"].get("count", 1), "frames": eot["f"].get("timeBetween", 0.0) * 60.0}
                for x in items(w["f"].get("behaviors")):
                    if x and x["type"] == "BurstWeaponBehaviorModel":
                        burst = {"count": x["f"]["count"], "frames": x["f"].get("intervalFrames", 0)}
                    if x and x["type"] == "EmissionsPerRoundFilterModel":
                        per_round = x["f"]["count"]
                    if x and x["type"] == "CritMultiplierModel":
                        crit = {"damage": x["f"]["damage"], "lower": x["f"]["lower"], "upper": x["f"]["upper"],
                                # The crit shot's own display (displayModel).
                                "display": (deref(x["f"].get("displayModel")) or {}).get("f", {}).get("display")}
                    if x and x["type"] == "AlternateProjectileModel":
                        alternate = {"interval": x["f"]["interval"], "projectile": projectile(deref(x["f"]["projectile"])),
                                     "emission": emission(deref(x["f"].get("emissionModel")))}
                    if x and x["type"] == "RandomAngleOffsetModel":
                        spread = max(abs(x["f"].get("minOffset", 0)), abs(x["f"].get("maxOffset", 0)))
                # LimitProjectileModel (Bloon Trap): at most `limit` live traps that aren't full.
                limit = next((x["f"]["limit"] for x in items(w["f"].get("behaviors")) + items(af["behaviors"])
                              if x and x["type"] == "LimitProjectileModel"), None)
                # Broadside guns (Buccaneer): Emission*RotationOff{Tower,Display}*Model fire at the
                # tower's facing + offsetRotation; FireWhenAlternateWeaponIsReadyModel fires only
                # with the main gun; FilterTargetAngleFilterModel needs a bloon in its arc.
                em = deref(w["f"]["emission"])
                aim_offset = next((x["f"]["offsetRotation"] for x in items((em or {}).get("f", {}).get("behaviors"))
                                   if x and x["type"].startswith(("EmissionRotationOff", "EmissionArcRotationOff"))
                                   and "offsetRotation" in x["f"]), None)
                with_main = next((x["f"].get("weaponId") for x in items(w["f"].get("behaviors"))
                                  if x and x["type"] == "FireWhenAlternateWeaponIsReadyModel"), None)
                alt_id = next((x["f"].get("weaponId") for x in items(w["f"].get("behaviors"))
                               if x and x["type"] == "FireAlternateWeaponModel"), None)
                fov = next(([x["f"]["fieldOfView"], x["f"].get("baseTowerRotationOffset", 0.0)] for x in items(w["f"].get("behaviors"))
                            if x and x["type"] == "FilterTargetAngleFilterModel"), None)
                max_subs = next(({"ids": [s for s in (deref(x["f"].get("baseSubTowerIds")) or []) if isinstance(s, str)]
                                          or [x["f"].get("baseSubTowerId", "")],
                                  "max": int(x["f"].get("maxNumberOfSubTowers", 0))}
                                 for x in items(w["f"].get("behaviors")) if x and x["type"] == "SubTowerFilterModel"), None)
                weapons.append({"name": (w["f"].get("_name") or "").removeprefix("WeaponModel_"), "rate": w["f"]["rate"],
                                # WeaponRateMinModel: buffs can't take the interval below this.
                                "rate_min": next((x["f"].get("min", 0.0) for x in items(w["f"].get("behaviors"))
                                                  if x and x["type"] == "WeaponRateMinModel"), 0.0),
                                # BloonDistanceRateBonusModel (Desperado): faster against farther
                                # targets (rateBonusPerRange, rangeMax).
                                "dist_rate": next(([x["f"].get("rateBonusPerRange", 0.0), x["f"].get("rangeMax", 0)]
                                                   for x in items(w["f"].get("behaviors")) if x and x["type"] == "BloonDistanceRateBonusModel"), None),
                                # LeakDangerAttackSpeedModel: up to maxRateIncreasePercent faster as bloons near the exit.
                                "leak_danger": next((x["f"].get("maxRateIncreasePercent", 0.0) for x in items(w["f"].get("behaviors"))
                                                     if x and x["type"] == "LeakDangerAttackSpeedModel"), 0.0), "animation_offset": w["f"].get("animationOffset", 0.0),
                                # fireBetweenRounds (weapon clock is Simulation.time, not roundTime),
                                # startInCooldown / customStartCooldown (Weapon.ApplyStartingCooldown).
                                "between_rounds": bool(w["f"].get("fireBetweenRounds")),
                                # FlipFollowPathModel (Boomerang hands): flipTowerDisplayX / Y, the eject
                                # axes mirrored when thrown with the left hand.
                                "flip_eject": next(([bool(x["f"].get("flipTowerDisplayX")), bool(x["f"].get("flipTowerDisplayY"))]
                                                    for x in items(w["f"].get("behaviors")) if x and x["type"] == "FlipFollowPathModel"), None),
                                "start_cooldown": bool(w["f"].get("startInCooldown")),
                                "custom_start": w["f"].get("customStartCooldown", 0.0),
                                "max_subs": max_subs,
                                "eject": [w["f"].get("ejectX", 0.0), w["f"].get("ejectY", 0.0), w["f"].get("ejectZ", 0.0)],
                                "emission": emission(deref(w["f"]["emission"])),
                                "projectile": projectile(deref(w["f"]["projectile"])), "crit": crit,
                                "alternate": alternate, "spread": spread, "per_round": per_round, "burst": burst, "limit": limit,
                                "aim_offset": aim_offset, "with_main": with_main, "alt_id": alt_id, "fov": fov,
                                "fire_sounds": behavior_sounds(items(w["f"].get("behaviors")),
                                                               "CreateSoundOnProjectileCreatedModel", SOUND_KEYS),
                                "to_bank": (deref(w["f"]["emission"]) or {}).get("type") == "SendToBankModel"})
            # LinkProjectileRadiusToTowerRangeModel: only its projectileModel (by reference; a
            # model without one links every weapon, as before).
            link_models = [b["f"] for b in items(f["behaviors"])
                           if b and b["type"] == "LinkProjectileRadiusToTowerRangeModel"]
            links = [l.get("projectileModel") for l in link_models]
            if links:
                refs = {l.get("$ref") for l in links if isinstance(l, dict)}
                for w, wobj in zip(weapons, [w for w in items(af["weapons"]) if w]):
                    pr = wobj["f"]["projectile"]
                    if not refs or (isinstance(pr, dict) and pr.get("$ref") in refs):
                        w["projectile"]["link_radius"] = True
                        # radius = the tower's range + projectileRadiusOffset.
                        lm = next((l for l in link_models if not refs or (l.get("projectileModel") or {}).get("$ref") == pr.get("$ref")), link_models[0])
                        w["projectile"]["link_offset"] = lm.get("projectileRadiusOffset", 0.0)
            prio_camo = any(x and "PrioCamo" in x["type"] for x in items(af["behaviors"]))
            random_position = next(([x["f"]["minRadius"], x["f"]["maxRadius"]] for x in items(af["behaviors"])
                                    if x and x["type"] == "RandomPositionBasicModel"), None)
            rpm = next((x["f"] for x in items(af["behaviors"]) if x and x["type"] == "RandomPositionModel"), None)
            random_land = None
            if rpm and random_position is None:
                random_position = [rpm["minDistance"], rpm["maxDistance"]]
                random_land = rpm.get("targetRadius", 0.0) if rpm.get("areaType") == "Land" else None
            friendly_ignore = ",".join(x["f"].get("ignoreList") or "" for x in items(af["behaviors"])
                                       if x and x["type"] == "TargetFriendlyModel")
            attack_brew = next((br for w in items(af["weapons"]) if w
                                for br in [brew(deref(w["f"]["projectile"]), friendly_ignore)] if br), None)
            pool_fallback = any(x and x["type"] == "TargetTrackOrDefaultAcidPoolModel" for x in items(af["behaviors"]))
            min_range = next((x["f"]["minimumRange"] for x in items(af["behaviors"])
                              if x and x["type"] == "AttackMinimumRangeModel"), 0.0)
            # Target*SharedRangeModel.isSharedRangeEnabled (Advanced Intel): also targets bloons
            # in range of any other tower.
            shared_range = any(x and x["type"].endswith("SharedRangeModel") and x["f"].get("isSharedRangeEnabled")
                               for x in items(af["behaviors"]))
            # RotateToTargetModel.additionalRotation: the tower turns this far past its target
            # (Buccaneer: -90, broadside on).
            extra_rotation = next((x["f"].get("additionalRotation", 0) for x in items(af["behaviors"])
                                   if x and x["type"] == "RotateToTargetModel"), 0)
            # TargetLeftHandModel / TargetRightHandModel (Robo Monkey's arms): the attack targets
            # with that hand's own mode (the Super Monkey panel's two target rows).
            hand = {"TargetLeftHandModel": "left", "TargetRightHandModel": "right"}.get(
                (deref(af.get("targetProvider")) or {}).get("type"))
            # NecromancerEmissionModel (Necromancer, Prince of Darkness): the weapon spends the
            # tower's graveyard (RBE popped in range) on undead bloons walking back up the track.
            necro = None
            for w in items(af["weapons"]):
                em = w and deref(w["f"].get("emission"))
                if em and em["type"] == "NecromancerEmissionModel":
                    ef = em["f"]
                    necro = {k: ef.get(k, 0) for k in ("damageRbePerStack", "damageStackMax", "maxBloonsSpawnedPerWave",
                                                       "minBloonsSpawnedPerWave", "maxPiercePerBloon", "maxRbeSpawnedPerSecond",
                                                       "maxRbeStored", "rateRbePerStack", "rateStackMax", "roundsBeforeDecay",
                                                       "maxPathRandomRange")}
                if em and em["type"] == "PrinceOfDarknessEmissionModel" and deref(em["f"].get("alternateProjectile")):
                    necro = dict(necro or {}, prince=projectile(deref(em["f"]["alternateProjectile"])),
                                 princePierce=em["f"].get("minPiercePerBloon", 0))
            return {"hand": hand, "necro": necro, "brew": attack_brew, "pool_fallback": pool_fallback, "min_range": min_range, "extra_rotation": extra_rotation, "shared_range": shared_range, "turn_body": turn_body, "rotate_always": rotate_always, "face_middle": face_middle,
                    # AttackModel name: Tower.UpdatedModel keeps the Attack of the same name.
                    "model_name": af.get("_name") or "",
                    "range": af["range"], "camo": camo, "rotate": rotate, "turn_rate": turn_rate,
                    "targets": targets, "weapons": weapons,
                    "prio_camo": prio_camo, "skip_effects": skip_effects, "track_target": track_target, "track_closest": track_closest,
                    "require_tags": require_tags, "exclude_tags": exclude_tags,
                    "glue_filter": glue_filter, "skip_frozen": skip_frozen, "cant_hit": cant_hit, "prio_moab": prio_moab, "random_position": random_position, "random_land": random_land,
                    "fire_without_target": af.get("fireWithoutTarget", False),
                    # AttackModel.attackThroughWalls: no Map.IsTargetLineClear check.
                    "through_walls": bool(af.get("attackThroughWalls", False))}

        def ability(b):
            """AbilityModel: ActivateAttackModel attacks (on for `lifespan` seconds, or one volley
            when isOneShot) and TurboModel (attack rate x multiplier for `lifespan`)."""
            bf = b["f"]
            out = {"name": bf["displayName"], "cooldown": bf.get("cooldown") or 0.0,
                   "description": bf.get("description"), "upgrade": bf.get("addedViaUpgrade"),
                   "ready": bool(bf.get("startOffCooldown")), "activate": [], "turbo": None,
                   "supports": [], "loan": None,
                   "sounds": behavior_sounds(items(bf.get("behaviors")), "CreateSoundOnAbilityModel", SOUND_KEYS)}
            for x in items(bf.get("behaviors")):
                if not x:
                    continue
                xf = x["f"]
                # MOAB / ZOMG Takedown (Buccaneer x-4-x / x-5-x): a grapple attack whose
                # projectile has a MoabTakedownModel and whose target provider is
                # TargetGrapplableModel; handled as its own behaviour, not as an attack.
                takedown = None
                if x["type"] == "ActivateAttackModel":
                    for a in items(xf.get("attacks")):
                        tg = next((y["f"] for y in items(a["f"].get("behaviors")) if y and y["type"] == "TargetGrapplableModel"), None)
                        mt = next((y["f"] for w in items(a["f"].get("weapons")) if w
                                   for y in items((deref(w["f"]["projectile"]) or {"f": {}})["f"].get("behaviors"))
                                   if y and y["type"] == "MoabTakedownModel"), None)
                        if tg and mt:
                            excl = [t for y in items(a["f"].get("behaviors")) if y and y["type"] == "AttackFilterModel"
                                    for flt in items(y["f"]["filters"]) if flt and flt["type"] == "FilterWithTagsModel" and flt["f"].get("inclusive")
                                    for t in (deref(flt["f"].get("tags")) or []) if isinstance(t, str)]
                            takedown = {"hooks": tg.get("hooks", 1), "zomg": tg.get("zomgHooksRequired", 9999999),
                                        "bad": tg.get("badHooksRequired", 9999999), "mult": mt.get("multiplier", 1.0),
                                        "add": mt.get("additive", 0.0), "exclude": excl}
                    if takedown:
                        out["takedown"] = takedown
                if x["type"] == "ActivateAttackModel" and not takedown:
                    out["activate"].append({"lifespan": xf.get("lifespan", 0.0), "one_shot": bool(xf.get("isOneShot")),
                                            "process_on_activate": bool(xf.get("processOnActivate")),
                                            "turn_off_existing": bool(xf.get("turnOffExisting")),
                                            "attacks": [parse_attack(a) for a in items(xf.get("attacks")) if a]})
                # MarkedToPopModel (Desperado): after markingTimeFrames the execution attack fires
                # for executionTimeMaxFrames. STAND-IN P-MARK: the marking itself (x2 damage and
                # cash on marked bloons) isn't modelled; the execution hits any bloon in range.
                if x["type"] == "MarkedToPopModel" and deref(xf.get("executionAttackModel")):
                    out["activate"].append({"lifespan": xf.get("executionTimeMaxFrames", 60) / 60.0, "one_shot": False,
                                            "turn_off_existing": False, "delay": xf.get("markingTimeFrames", 0) / 60.0,
                                            "attacks": [parse_attack(deref(xf["executionAttackModel"]))]})
                if x["type"] == "TurboModel":
                    out["turbo"] = {"lifespan": xf.get("lifespan", 0.0), "mult": xf.get("multiplier", 1.0),
                                    "extra_damage": xf.get("extraDamage", 0)}
                if x["type"] == "DamageUpModel":
                    out["turbo"] = {"lifespan": xf.get("lifespanFrames", 0) / 60.0, "mult": 1.0,
                                    "extra_damage": xf.get("additionalDamage", 0)}
                if x["type"] == "ImfLoanModel":
                    out["loan"] = {"amount": xf.get("amount", 0.0), "rate": xf.get("incomeRecoveryRate", 0.5)}
                # OverclockModel (Engineer): boosts one chosen tower; OverclockPermanentModel
                # (Ultraboost) also leaves a permanent stack on it.
                if x["type"] == "OverclockModel":
                    out.setdefault("overclock", {})
                    out["overclock"].update({"frames": xf.get("lifespanFrames", 0), "rate": xf.get("rateModifier", 1.0),
                                             "village_range": xf.get("villageRangeModifier", 1.0),
                                             "tier_mult": [v for v in (deref(xf.get("tierBasedDurationMultiplier")) or [])
                                                           if isinstance(v, (int, float))]})
                # ChangeDamageTypeModel / MutateProjectileOnAbilityModel (Armor Piercing Shells,
                # Frostbite): for lifespanFrames the tower's projectiles get these immune
                # properties / +damageIncrease and the added DamageModifierForTagModel.
                if x["type"] == "ChangeDamageTypeModel":
                    out.setdefault("boosts", []).append({"frames": xf.get("lifespanFrames", 0), "immune": xf.get("immuneBloonProperties", 0)})
                if x["type"] == "MutateProjectileOnAbilityModel":
                    pbm = deref(xf.get("projectileBehaviorModel"))
                    out.setdefault("boosts", []).append({
                        "frames": xf.get("lifespanFrames", 0), "damage": xf.get("damageIncrease", 0),
                        "tag_mods": [tag_mod(pbm)] if pbm and pbm["type"] == "DamageModifierForTagModel" else []})
                # BiohackModel (Benjamin): the affectedCount nearest towers deal +damageIncrease
                # for `lifespan` (the shutdown after it isn't modelled).
                if x["type"] == "BiohackModel":
                    out["biohack"] = {"count": xf.get("affectedCount", 0), "damage": xf.get("damageIncrease", 0.0),
                                      "lifespan": xf.get("lifespan", 0.0)}
                # LongArmOfLightModel (Adora): range x multiplier, projectile radius x
                # projectileRadiusMultiplier, these immune properties, +damageIncrease.
                if x["type"] == "LongArmOfLightModel":
                    frames = xf.get("lifespan", 0.0) * 60
                    out["range_up"] = {"frames": frames, "mult": xf.get("multiplier", 1.0), "add": 0.0}
                    out.setdefault("boosts", []).append({"frames": frames, "immune": xf.get("immuneBloonProperties", 0),
                                                         "damage": xf.get("damageIncrease", 0),
                                                         "radius_mult": xf.get("projectileRadiusMultiplier", 1.0)})
                # MoabBarrageModel (Churchill): the `targets` strongest MOAB-class bloons take
                # damage every interval for the ability's duration.
                if x["type"] == "MoabBarrageModel":
                    dm = (deref(xf.get("damageOverTimeModel")) or {"f": {}})["f"]
                    out["barrage"] = {"targets": xf.get("targets", 0), "damage": dm.get("damage", 0),
                                      "interval": dm.get("interval", 1.0), "lifespan": xf.get("durationFrames", 0) / 60.0,
                                      "immune": dm.get("immuneBloonProperties", 0)}
                # PlaceProjectileAtModel (Mega Mine): the projectile at a picked point after `delay`.
                if x["type"] == "PlaceProjectileAtModel" and deref(xf.get("projectileModel")):
                    out["place_at"] = {"projectile": projectile(deref(xf["projectileModel"])), "delay": xf.get("delay", 0.0)}
                # BrickellFreezeMinesAbilityBuffModel: her attacks x multiplier interval for
                # `lifespan` (STAND-IN: every attack, not only the mines; projectile speed not applied).
                if x["type"] == "BrickellFreezeMinesAbilityBuffModel":
                    out["turbo"] = {"lifespan": xf.get("lifespan", 0.0), "mult": xf.get("multiplier", 1.0), "extra_damage": 0}
                # SwordChargeModel (Sauda): its projectile runs along the track; LeapingSwordModel:
                # the impact lands after landDelay (P-LEAP).
                if x["type"] == "SwordChargeModel" and deref(xf.get("projectileModel")):
                    out["track_charge"] = projectile(deref(xf["projectileModel"]))
                if x["type"] == "LeapingSwordModel" and deref(xf.get("impactProjectileModel")):
                    out["leap"] = {"projectile": projectile(deref(xf["impactProjectileModel"])), "delay": xf.get("landDelay", 0.0)}
                # SyphonFundingModel (Benjamin): for `lifespan`, see P-SYPHON.
                if x["type"] == "SyphonFundingModel":
                    out["syphon"] = xf.get("lifespan", 0.0)
                # AbilityCancelLeakModel (Quick Thinking): leaks cost nothing for durationFrames.
                if x["type"] == "AbilityCancelLeakModel":
                    out["immunity"] = xf.get("durationFrames", 0) / 60.0
                # ImmunityModel (Legend of the Night): leaks cost no lives for `lifespan` seconds.
                if x["type"] == "ImmunityModel":
                    out["immunity"] = xf.get("lifespan", 0.0)
                # RedeployModel (Heli Pilot): the pad moves to any picked spot.
                if x["type"] == "RedeployModel":
                    out["darkshift"] = {"radius": 0.0, "restrict": False}
                # DarkshiftModel (Dark Knight): the tower teleports to a picked spot within radius.
                if x["type"] == "DarkshiftModel":
                    out["darkshift"] = {"radius": xf.get("placementZoneAssetRadius", 0.0),
                                        "restrict": bool(xf.get("restrictToTowerRadius"))}
                # TakeAimModel (Desperado): range x rangeModifier on a picked tower.
                if x["type"] == "TakeAimModel":
                    out["overclock"] = {"frames": xf.get("lifespanFrames", 0), "rate": 1.0, "village_range": 1.0,
                                        "range": xf.get("rangeModifier", 1.0), "tier_mult": []}
                # IncreaseRangeModel (Transforming Tonic): the tower's own range for a while.
                if x["type"] == "IncreaseRangeModel":
                    out["range_up"] = {"frames": xf.get("lifespanFrames", 0), "mult": xf.get("multiplier", 1.0),
                                       "add": xf.get("addative", 0.0)}
                if x["type"] == "OverclockPermanentModel":
                    out.setdefault("overclock", {})
                    out["overclock"]["permanent"] = {"rate": xf.get("rateModifier", 1.0),
                                                     "village_range": xf.get("villageRangeModifier", 1.0),
                                                     "max_stacks": xf.get("maxStacks", 1)}
                # DroneSwarmModel (Etienne): `count` more drones for `duration`; UCAVModel: the
                # UCAV for `duration`. Temporary sub-towers like Summon Phoenix.
                if x["type"] == "DroneSwarmModel":
                    ds = (deref(xf.get("droneSupportModel")) or {"f": {}})["f"]
                    sp = sub_tower(deref(ds.get("droneModel")), None)
                    if sp:
                        out.setdefault("create_tower", []).extend([dict(sp, lifespan=xf.get("duration", 0.0))] * int(ds.get("count", 1)))
                if x["type"] == "UCAVModel":
                    sp = sub_tower(deref(xf.get("ucavTowerModel")), None)
                    if sp:
                        out.setdefault("create_tower", []).append(dict(sp, lifespan=xf.get("duration", 0.0)))
                # AbilityCreateTowerModel (Summon Phoenix): a temporary sub-tower.
                if x["type"] == "AbilityCreateTowerModel":
                    sp = sub_tower(deref(xf.get("towerModel")), None)
                    if sp:
                        out.setdefault("create_tower", []).append(sp)
                # SwitchDisplayModel (Lord Phoenix, Transforming Tonic): another model for `lifespan`.
                if x["type"] == "SwitchDisplayModel":
                    out["switch"] = {"display": prefab_name(xf.get("display")), "lifespan": xf.get("lifespan", 0.0)}
                # MutateRemoveAllAttacksOnAbilityActivateModel: the tower's own attacks stop meanwhile.
                if x["type"] == "MutateRemoveAllAttacksOnAbilityActivateModel":
                    out["mute"] = xf.get("lifespan", 0.0)
                if x["type"] == "MonkeyFanClubModel":
                    out["fan_club"] = {"lifespan": xf.get("lifespan", 0.0), "max_tier": xf.get("maxTier", 2),
                                       "count": xf.get("towerCount", 10), "range": xf.get("range", 0.0),
                                       "rate": xf.get("reloadModifier", 1.0), "immune": xf.get("immuneBloonProperties", 0),
                                       "pierce": xf.get("bonusPierce", 0.0), "damage": xf.get("bonusDamage", 0),
                                       "projectile_radius": xf.get("projectileRadius", 0.0),
                                       "eject": [xf.get("ejectX", 0.0), xf.get("ejectY", 0.0), xf.get("ejectZ", 0.0)],
                                       "display": prefab_name(xf.get("towerDisplay"))}
                # CreateProjectileOnAbilityActivateModel: a projectile fired from the tower on use.
                if x["type"] == "CreateProjectileOnAbilityActivateModel" and deref(xf.get("projectile")) and xf.get("chance", 1.0) >= 1.0:
                    out.setdefault("on_activate", []).append({"projectile": projectile(deref(xf["projectile"])),
                                                              "emission": emission(deref(xf.get("emission")))})
                # ActivateAbilitiesOnAbilityModel (Support Drop -> Supply Drop): uses that one too.
                if x["type"] == "ActivateAbilitiesOnAbilityModel" and xf.get("abilityToFind"):
                    out["also"] = xf["abilityToFind"]
                # AbilityReduceCooldownOnLivesLostModel: lives lost take reductionSeconds off the
                # cooldown, at most once per delaySeconds.
                if x["type"] == "AbilityReduceCooldownOnLivesLostModel":
                    out["leak_cdr"] = [xf.get("reductionSeconds", 0.0), xf.get("delaySeconds", 0.0)]
                # MorphTowerModel with mutateAll (Total Transformation): up to maxTowers towers on
                # affectList with no path above maxTier become secondaryTowerModel for lifespan.
                if x["type"] == "MorphTowerModel" and xf.get("mutateAll") and deref(xf.get("secondaryTowerModel")):
                    st = deref(xf["secondaryTowerModel"])
                    out["morph"] = {"max": xf.get("maxTowers", 1), "max_tier": xf.get("maxTier", 5),
                                    "affect": [s.strip() for s in (xf.get("affectList") or "").split(",") if s.strip()],
                                    "ignore": [s.strip() for s in (xf.get("ignoreWithMutators") or "").split(",") if s.strip()],
                                    "lifespan": xf.get("lifespan", 0.0), "tier": tier(st),
                                    "display": prefab_name(st["f"].get("display"))}
                if x["type"] == "AbilityDamageAllModel":
                    out.setdefault("damage_all", []).append(
                        {"damage": xf.get("damage", 0), "tags": [s for s in (deref(xf.get("tags")) or []) if isinstance(s, str)],
                         "inclusive": bool(xf.get("inclusive"))})
                if x["type"] == "CashPerTowerInRangeModel":
                    out["bounty"] = {"towers": [s for s in (deref(xf.get("towerIds")) or []) if isinstance(s, str)],
                                     "base": xf.get("baseCash", 0.0), "per_tier": deref(xf.get("extraCashPerTier")) or [],
                                     "max": xf.get("maxCashGeneration", 0.0), "range_add": xf.get("rangeIncrease", 0.0)}
                if x["type"] == "BonusLivesOnAbilityModel":
                    out["lives"] = xf.get("amount", 0.0)
                if x["type"] == "SniperAmmoSwapBehaviorModel" and xf.get("appliesToSelf"):
                    out["ammo"] = {"delay": xf.get("initialDelay", 0.0), "shots": xf.get("amtOfAttacksBeforeExpire", 1),
                                   "pierce": xf.get("pierceIncrease", 0), "damage_mult": xf.get("damageMultiplier", 1.0),
                                   "lifespan_mult": xf.get("lifeSpanMultiplier", 1.0),
                                   "immune": xf.get("newImmuneBloonProperties", 0) if xf.get("overrideDamageType") else None}
                # Timed support zones, in the same shape as the tower's own supports.
                kind = {"ActivateRateSupportZoneModel": "rate", "CallToArmsModel": "rate",
                        "ActivateRangeSupportZoneModel": "range", "ActivatePierceSupportZoneModel": "pierce",
                        "ActivateVisibilitySupportZoneModel": "camo",
                        "ActivateProjectileSpeedSupportZoneModel": "speed",
                        "ActivateDamageBypassSupportZoneModel": "immune",
                        "ActivateTowerDamageSupportZoneModel": "damage"}.get(x["type"])
                if kind:
                    mult, add = 0.0, 0.0
                    if x["type"] == "ActivateRateSupportZoneModel":
                        mult = xf.get("rateModifier", 1.0)
                    if x["type"] == "CallToArmsModel":
                        # STAND-IN: only the attack-speed half (interval / multiplier); the pierce
                        # multiplier isn't applied.
                        mult = 1.0 / (xf.get("multiplier") or 1.0)
                    if kind == "range":
                        mult, add = xf.get("multiplier", 0.0), xf.get("additive", 0.0)
                    if kind == "pierce":
                        add = xf.get("pierceIncrease", 0.0)
                    # HYPOTHESIS P-SPEEDPCT: speedIncrease is a percentage (200 = +200%).
                    if kind == "speed":
                        mult = xf.get("speedIncrease", 0.0) / 100.0
                    # DamageBypass: projectiles may damage these BloonProperties (a mask of the
                    # 16 low property bits that stay immune).
                    if kind == "immune":
                        bits = {"Lead": 1, "Black": 2, "White": 4, "Purple": 8, "Frozen": 16}
                        bypass = sum(v for k2, v in bits.items() if k2 in str(xf.get("properties", "")))
                        add = float(0xFFFF & ~bypass)
                    if kind == "damage":
                        add = xf.get("damageIncrease", 0.0)
                    sets, bases = [], []
                    for flt in items(xf.get("filters")):
                        if flt and flt["type"] == "FilterInSetModel":
                            sets.append(flt["f"]["towerSet"])
                        if flt and flt["type"] == "FilterInBaseTowerIdModel":
                            bases += [s for s in (deref(flt["f"].get("baseIds")) or []) if isinstance(s, str)]
                    rng = xf.get("range") or 0.0
                    glob = bool(xf.get("isGlobal")) or rng >= 999999
                    use_tower = xf.get("useTowerRange") or rng <= 0 or (x["type"] == "CallToArmsModel" and not xf.get("useRadius"))
                    out["supports"].append({"kind": kind, "mult": mult, "add": add,
                                            "id": xf.get("mutatorId") or x["type"], "global": glob,
                                            "radius": None if glob or use_tower else rng,
                                            "self": bool(xf.get("canEffectThisTower", xf.get("canAffectThisTower", True))),
                                            "stacks": 1, "sets": sets, "bases": bases, "tier_cap": 5,
                                            "lifespan": xf.get("lifespan", 0.0),
                                            "icon": xf.get("buffIconName") if xf.get("showBuffIcon") else None})
            return out

        for b in items(f["behaviors"]):
            if not b:
                continue
            if b["type"] == "AbilityModel":
                abilities.append(ability(b))
            if b["type"] in ("AttackModel", "AttackAirUnitModel"):
                attacks.append(parse_attack(b))
        zones = []
        for b in items(f["behaviors"]):
            if b and b["type"] == "SlowBloonsZoneModel":
                zf = b["f"]
                zones.append({"radius": zf["zoneRadius"], "bind": zf.get("bindRadiusToTowerRange", False),
                              "offset": zf.get("radiusOffset", 0.0), "scale": zf.get("speedScale", 1.0),
                              "id": zf.get("mutationId") or "",
                              "tag": zf.get("bloonTag") or "",
                              "tags": [t for t in items(zf.get("bloonTags")) if isinstance(t, str)],
                              "inclusive": zf.get("inclusive", False),
                              "exclude_tags": [x["f"]["tag"] for x in items(zf.get("filters"))
                                               if x and x["type"] == "FilterOutTagModel"]
                              + ["Moabs" for x in items(zf.get("filters")) if x and x["type"] == "FilterMoabModel" and x["f"].get("flip")],
                              "camo": not any(x and x["type"] == "FilterInvisibleModel" and x["f"].get("isActive", True)
                                              for x in items(zf.get("filters")))})
        orbit = None
        for b in items(f["behaviors"]):
            if b and b["type"] == "OrbitModel":
                op = deref(b["f"]["projectile"])
                disp = next((x["f"].get("display") for x in items(op["f"]["behaviors"]) if x and x["type"] == "DisplayModel"),
                            op["f"].get("display"))
                orbit = {"count": b["f"]["count"], "range": b["f"]["range"], "display": disp}
        round_start_rate = next(({"mult": b["f"]["modifier"], "duration": b["f"]["duration"]} for b in items(f["behaviors"])
                                 if b and b["type"] == "StartOfRoundRateBuffModel"), None)
        bank = next(({"capacity": b["f"]["capacity"], "interest": b["f"]["interest"], "auto": b["f"].get("autoCollect", False)}
                     for b in items(f["behaviors"]) if b and b["type"] == "BankModel"), None)
        # Buffs this tower gives others in range (Village, Shinobi Tactics).
        support_kinds = {"RangeSupportModel": "range", "RateSupportModel": "rate", "PierceSupportModel": "pierce",
                         "ProjectileSpeedSupportModel": "speed", "VisibilitySupportModel": "camo",
                         "DamageTypeSupportModel": "immune", "SupportShinobiTacticsModel": "rate",
                         "DiscountZoneModel": "discount", "DamageSupportModel": "damage",
                         # AbilityCooldownScaleSupportModel: abilities in range recharge faster;
                         # MonkeyCityIncomeSupportModel (Monkey City): income x incomeModifier.
                         "AbilityCooldownScaleSupportModel": "ability_cd", "MonkeyCityIncomeSupportModel": "income",
                         # SupportRemoveFilterOutTagModel (MIB): towers in range ignore their
                         # FilterOutTag filters.
                         "SupportRemoveFilterOutTagModel": "unfilter",
                         # FreeUpgradeSupportModel (Primary Mentoring / Expertise): upgrades up to
                         # `upgrade` free for towers in range: a 100% discount up to that tier.
                         "FreeUpgradeSupportModel": "discount",
                         # BananaCentralBuffModel: Banana Farms' income x multiplier (global);
                         # CentralMarketBuffModel: Merchantmen's income x multiplier per Central
                         # Market, up to maxStackCount (global).
                         "BananaCentralBuffModel": "income", "CentralMarketBuffModel": "income"}

        def supports_of(behaviors):
          supports = []
          for b in behaviors:
              if not b:
                  continue
              sf, kind = b["f"], support_kinds.get(b["type"])
              mult, add = sf.get("multiplier", 0.0), sf.get("additive", 0.0)
              if b["type"] == "PierceSupportModel":
                  add = sf.get("pierce", 0.0)
              if b["type"] == "DamageTypeSupportModel":
                  add = sf.get("immuneBloonProperties", 0)
              if b["type"] == "DiscountZoneModel":
                  mult = sf.get("discountMultiplier", 0.0)
              if b["type"] == "FreeUpgradeSupportModel":
                  mult = 1.0
              base_filter = {"BananaCentralBuffModel": ["BananaFarm"], "CentralMarketBuffModel": ["MonkeyBuccaneer"]}.get(b["type"], [])
              if b["type"] == "DamageSupportModel":
                  add = sf.get("increase", 0.0)
              if b["type"] == "AbilityCooldownScaleSupportModel":
                  mult = sf.get("abilityCooldownSpeedScale", 1.0)
              if b["type"] == "MonkeyCityIncomeSupportModel":
                  mult = sf.get("incomeModifier", 1.0)
              if b["type"] == "AddBehaviorToTowerSupportModel":
                  for x in items(sf.get("behaviors")):
                      if x and x["type"] == "CashIncreaseModel":
                          kind, mult, add = "cash", x["f"].get("multiplier", 1.0), x["f"].get("increase", 0.0)
              # SubCommanderSupportModel: +pierceIncrease pierce and x damageScale damage (+
              # damageIncrease) to Subs in range, itself included; unique (no stacking).
              extra = []
              if b["type"] == "SubCommanderSupportModel":
                  kind, add = "pierce", sf.get("pierceIncrease", 0)
                  extra = [("damage_mult", sf.get("damageScale", 1.0), sf.get("damageIncrease", 0))]
              if not kind:
                  continue
              sets, bases = [], []
              for x in items(sf.get("filters")):
                  if x and x["type"] == "FilterInSetModel":
                      sets.append(x["f"]["towerSet"])
                  if x and x["type"] == "FilterInBaseTowerIdModel":
                      bases += [s for s in (deref(x["f"].get("baseIds")) or []) if isinstance(s, str)]
              bases += [s for s in (deref(sf.get("towerBaseIdList")) or []) if isinstance(s, str)] + base_filter
              supports.append({"kind": kind, "mult": mult, "add": add,
                               "id": sf.get("mutatorId") or sf.get("stackName") or b["type"],
                               "global": bool(sf.get("isGlobal") or sf.get("isGlobalRange")),
                               "radius": sf["customRadius"] if sf.get("isCustomRadius") else None,
                               "self": bool(sf.get("appliesToOwningTower") or b["type"] == "SupportShinobiTacticsModel"),
                               "stacks": max(sf.get("maxStacks") or 0, sf.get("stackLimit") or 0, sf.get("maxStackCount") or 0, 1),
                               "sets": sets, "bases": bases, "tier_cap": sf["upgrade"] if b["type"] == "FreeUpgradeSupportModel" else sf.get("tierCap", 5),
                               # BuffIndicatorUi: the icon shown over towers this buffs.
                               "icon": sf.get("buffIconName") if sf.get("showBuffIcon") else None})
              for k2, m2, a2 in extra:
                  supports.append(dict(supports[-1], kind=k2, mult=m2, add=a2, id=supports[-1]["id"] + "_" + k2))
          return supports
        supports = supports_of(items(f["behaviors"]))
        tiers = items(f["tiers"])
        # AirUnitModel (Heli Pilot, Monkey Ace): the flying unit the attacks fire from. A Heli
        # (HeliMovementModel) steers; its modes are the main attack's *SettingModels. An Ace
        # (PathMovementModel) flies the first air attack's selectable *PatternModels.
        air, air_modes = None, []
        for b in items(f["behaviors"]):
            if b and b["type"] == "AirUnitModel":
                bs = [x for x in items(b["f"].get("behaviors")) if x]
                hm = next((x["f"] for x in bs if x["type"] == "HeliMovementModel"), None)
                pm = next((x["f"] for x in bs if x["type"] == "PathMovementModel"), None)
                fm = next((x["f"] for x in bs if x["type"] == "FighterMovementModel"), None)
                if fm:
                    # STAND-IN P-FIGHTER: FighterMovementModel (Aircraft Carrier planes) steers
                    # at maxSpeed with no slow-down, turning at most turningSpeed rad per frame
                    # (sideways acceleration speed x turningSpeed x 60), so it overshoots and
                    # comes round again like the game's flyovers. Rolls and loops aren't drawn.
                    air = {"display": prefab_name(b["f"].get("display")), "speed": fm["maxSpeed"],
                           "accel": fm["maxSpeed"] * fm.get("turningSpeed", 0.035) * 60.0, "slowdown": 1.0}
                elif hm:
                    air = {"display": prefab_name(b["f"].get("display")), "speed": hm["maxSpeed"],
                           "accel": hm["movementForceStart"], "slowdown": hm["slowdownRadiusMax"]}
                elif pm:
                    air = {"display": prefab_name(b["f"].get("display")), "speed": pm["speed"],
                           "catch_up": pm.get("catchUpSpeed", 1.0), "patterns": [],
                           # PathMovement.Process: turn rate (deg/s) off the path and banking.
                           "turn": pm.get("rotation", 360.0), "bank": pm.get("bankRotation", 0.0),
                           "bank_mul": pm.get("bankRotationMul", 0.0),
                           # PathMovement.TakeOffAnimation: scale and pitch as it takes off.
                           "takeoff": {"time": pm.get("takeOffTime", 0.0), "exp": pm.get("takeOffExponent", 1.0),
                                       "anim": pm.get("takeOffAnimTime", 0.0), "scale": pm.get("takeOffScale", 1.0),
                                       "scale_exp": pm.get("takeOffScaleExponent", 1.0), "pitch": pm.get("takeOffPitch", 0.0),
                                       "pitch_exp": pm.get("takeOffPitchExponent", 1.0)} if pm.get("takeOffAnimTime") else None}
        # SpiritTowerMovementModel (Corvus's Spirit): the tower itself flies. STAND-IN P-SPIRIT:
        # steered like a drone at maxSpeed, accelerationPerFrame per frame, slowing within
        # `tolerance`; its leash to Corvus (`distance`) isn't modelled.
        sm = next((b["f"] for b in items(f["behaviors"]) if b and b["type"] == "SpiritTowerMovementModel"), None)
        if sm and not air:
            air = {"display": prefab_name(f.get("display")), "speed": sm.get("maxSpeed", 100.0),
                   "accel": sm.get("accelerationPerFrame", 0.1667) * 3600.0, "slowdown": max(sm.get("tolerance", 1.0), 1.0)}
        if air:
            for b in items(f["behaviors"]):
                if b and b["type"] in ("AttackModel", "AttackAirUnitModel"):
                    for x in items(b["f"]["behaviors"]):
                        m = {"FollowTouchSettingModel": "Pointer", "LockInPlaceSettingModel": "Point",
                             "PursuitSettingModel": "Pursuit"}.get(x["type"]) if x else None
                        if m and m not in air_modes:
                            air_modes.append(m)
                        if "patterns" not in air or not x or not x["f"].get("isSelectable"):
                            continue
                        xf = x["f"]
                        p = None
                        if x["type"] == "CirclePatternModel":
                            p = {"mode": "Circle", "rx": xf["radius"], "ry": xf["radius"]}
                        elif x["type"] == "FigureEightPatternModel":
                            p = {"mode": "Infinite" if xf.get("rotated") else "FigureEight",
                                 "rx": xf["radius"], "ry": xf["radius"]}
                        elif x["type"] == "CenterElipsePatternModel":
                            p = {"mode": "Centered", "rx": xf["widthRadius"], "ry": xf["heightRadius"]}
                        if p and p["mode"] not in air_modes:
                            air["patterns"].append(p)
                            air_modes.append(p["mode"])
        # BeastHandlerLeashModel: the pet(s) this tier leashes (towerModel / towerModelSecond),
        # spawned as permanent sub-towers inside the handler's range (HYPOTHESIS P-PET).
        pets = []
        for b in items(tobj["f"]["behaviors"]):
            # TowerCreateTowerModel (Corvus's Spirit, Lord Phoenix's PermaPhoenix): a permanent
            # sub-tower that exists while the tower does, kept like pets.
            if b and b["type"] == "TowerCreateTowerModel":
                sp = sub_tower(deref(b["f"].get("towerModel")), None)
                if sp:
                    pets.append(sp)
            # DroneSupportModel (Etienne): `count` drones (droneModel), kept like pets.
            if b and b["type"] == "DroneSupportModel":
                sp = sub_tower(deref(b["f"].get("droneModel")), None)
                if sp:
                    pets += [sp] * int(b["f"].get("count", 1))
            if b and b["type"] == "BeastHandlerLeashModel":
                for key in ("towerModel", "towerModelSecond"):
                    sp = sub_tower(deref(b["f"].get(key)), None)
                    if sp:
                        pets.append(sp)
        # Skywarden: ToggleFocusStanceModel (Focus: range / interval / lifespan multipliers) and
        # DamageBasedAttackSpeedModel (faster per `threshold` damage dealt, reset when idle).
        stance, damage_speed = None, None
        for b in items(tobj["f"]["behaviors"]):
            if b and b["type"] == "ToggleFocusStanceModel":
                bf = b["f"]
                stance = {"range": bf.get("rangeMultiplier", 1.0), "rate": bf.get("rateMultiplier", 1.0),
                          "lifespan": bf.get("projectilelifespanMultiplier", 1.0)}
            if b and b["type"] == "DamageBasedAttackSpeedModel":
                bf = b["f"]
                damage_speed = {"threshold": bf.get("damageThreshold", 1.0), "increase": bf.get("increasePerThreshold", 0.0),
                                "max_stacks": bf.get("maxStacks", 0), "idle_frames": bf.get("maxTimeInFramesWithoutDamage", 0)}
        # GyrfalconPatternModel (Beast Handler birds): dive, grab bloons, grapple them.
        grapple = None
        for b in items(tobj["f"]["behaviors"]):
            if not (b and b["type"] == "AttackAirUnitModel"):
                continue
            for x in items(b["f"].get("behaviors")):
                if not (x and x["type"] == "GyrfalconPatternModel"):
                    continue
                xf = x["f"]
                dmg = lambda k: ((deref(xf.get(k)) or {}).get("f") or {})
                grapple = {"speed": xf.get("maxSpeed", 80.0), "dive_mult": xf.get("bloonTargetSpeedMultiplier", 1.0),
                           "cooldown": xf.get("cooldown", 1.0), "rate": xf.get("grappleDamageRate", 1.0),
                           "initial": dmg("initialDamageModel").get("damage", 0.0),
                           "grapple": dmg("grapplingDamageModel").get("damage", 0.0),
                           "initial_moab": dmg("initialDamageMoabModifierModel").get("damageAddative", 0.0),
                           "grapple_moab": dmg("grapplingDamageMoabModifierModel").get("damageAddative", 0.0),
                           "regrow_add": dmg("regrowDamageModifierModel").get("damageAddative", 0.0),
                           "amount": xf.get("grabAmount", 1), "radius": xf.get("grabRadius", 10.0),
                           "keep": bool(xf.get("keepPickingUpBloons")), "keep_range": xf.get("keepPickingBloonsRange", 0.0),
                           "penalty": {"Lead": xf.get("leadPiercePenalty", 0), "Ceramic": xf.get("ceramicPiercePenalty", 0),
                                       "Moab": xf.get("moabPiercePenalty", 0), "Bfb": xf.get("bfbPiercePenalty", 0),
                                       "Zomg": xf.get("zomgPiercePenalty", 0), "Ddt": xf.get("ddtPiercePenalty", 0)}}
        # SubmergeModel (Sub x-3-x+): the "Submerge" target mode swaps the attacks for
        # submergeAttackModel (camo-stripping / Bloontonium pulses); towers in range get their
        # ability cooldowns sped up and heroes more XP.
        # DruidVengeanceEffectModel (Avatar of Wrath): glow effects and bolt displays by how many
        # damage "stacks" Wrath adds (smallGlowEffectStacks / medium / epic).
        vengeance = None
        for b in items(tobj["f"]["behaviors"]):
            if b and b["type"] == "DruidVengeanceEffectModel":
                vf = b["f"]
                fx = [prefab_name((deref(vf.get(k)) or {}).get("f", {}).get("assetId")) for k in
                      ("smallGlowEffectModel", "mediumGlowEffectModel", "epicGlowEffectModel")]
                vengeance = {"fx": fx, "stacks": [vf.get("smallGlowEffectStacks", 0), vf.get("mediumGlowEffectStacks", 0),
                                                   vf.get("epicGlowEffectStacks", 0)],
                             "displays": [vf.get("defaultProjectilePath"), vf.get("weakProjectilePath"),
                                          vf.get("mediumProjectilePath"), vf.get("epicProjectilePath")]}
        # MonkeyTempleModel (Sun Temple / True Sun God): upgrading to it sacrifices the towers in
        # range; each TempleTowerMutatorGroup (Tier One: Sun Temple's, Tier Two: True Sun God's)
        # whose tower set's sacrificed worth reaches `cost` applies its mutators
        # (MonkeyTemple.ActOnSacrifice, VERIFIED-DISASM @0xd5cd30: worth >= cost).
        temple = None
        for b in items(tobj["f"]["behaviors"]):
            if b and b["type"] == "MonkeyTempleModel":
                temple = {"groups": [], "delay": b["f"].get("weaponDelay", 0.0),
                          "fx": prefab_name((deref(b["f"].get("effectModel")) or {}).get("f", {}).get("assetId"))}
        for b in items(tobj["f"]["behaviors"]):
            if not (temple is not None and b and b["type"].startswith("TempleTowerMutatorGroupTier")):
                continue
            g = {"set": b["f"]["towerSet"], "cost": b["f"]["cost"], "two": b["type"].endswith("TwoModel"),
                 "attacks": [], "supports": [], "round_cash": 0.0, "damage": 0.0, "pierce": 0.0, "speed": 0.0,
                 "size": 0.0, "range": 0.0, "reload": 0.0, "wind": 0.0}
            for m in items(b["f"]["mutators"]):
                if not m:
                    continue
                mf = m["f"]
                if m["type"] == "AddAttackTowerMutatorModel" and deref(mf.get("attackModel")):
                    am = deref(mf["attackModel"])
                    # The attack's family: its name without the version (TrueGoldenGlaivesV2 ->
                    # TrueGoldenGlaives); a later version replaces an earlier one.
                    fam = re.sub(r"V\d+$", "", (am["f"].get("_name") or "").removeprefix("AttackModel_").rstrip("_"))
                    g["attacks"].append([fam, parse_attack(am)])
                elif m["type"] == "AddBehaviorToTowerMutatorModel":
                    bs = [x for x in items(mf.get("behaviors")) if x]
                    g["supports"] += supports_of(bs)
                    g["round_cash"] += sum(x["f"].get("cashPerRound", 0.0) for x in bs if x["type"] == "PerRoundCashBonusTowerModel")
                elif m["type"] == "DamageTowerMutatorModel":
                    g["damage"] += mf.get("damage", 0.0)
                elif m["type"] == "PierceTowerMutatorModel":
                    g["pierce"] += mf.get("pierce", 0.0)
                elif m["type"] == "ProjectileSpeedTowerMutatorModel":
                    g["speed"] += mf.get("speedModifier", 0.0)
                elif m["type"] == "ProjectileSizeTowerMutatorModel":
                    g["size"] += mf.get("sizeModifier", 0.0)
                elif m["type"] == "RangeTowerMutatorModel":
                    g["range"] += mf.get("rangeIncrease", 0.0)
                elif m["type"] == "ReloadTimeTowerMutatorModel":
                    g["reload"] += mf.get("multiplier", 0.0)
                elif m["type"] == "WindChanceTowerMutatorModel":
                    g["wind"] += mf.get("windChance", 0.0)
            temple["groups"].append(g)
        submerge = None
        for b in items(tobj["f"]["behaviors"]):
            if b and b["type"] == "SubmergeModel" and deref(b["f"].get("submergeAttackModel")):
                bf = b["f"]
                # The effect each pulse shows: SubmergeEffectModel's (Bloontonium scanners) or
                # SubmergeModel.attackDisplayPath (SubmergeEffect).
                sub_fx = next((prefab_name(deref(x["f"].get("effectModel"))["f"].get("assetId"))
                               for x in items(tobj["f"]["behaviors"])
                               if x and x["type"] == "SubmergeEffectModel" and deref(x["f"].get("effectModel"))), None)                     or prefab_name(bf.get("attackDisplayPath"))
                sub_attack = parse_attack(deref(bf["submergeAttackModel"]))
                sub_attack["fire_fx"] = sub_fx
                submerge = {"attack": sub_attack,
                            "cooldown_scale": bf.get("abilityCooldownSpeedScale", 1.0),
                            "cooldown_scale_global": bf.get("abilityCooldownSpeedScaleGlobal", 1.0),
                            "hero_xp": bf.get("heroXpScale", 1.0), "depth": bf.get("submergeDepth", 0.0)}
        return {"name": f["_name"], "tiers": tiers, "range": f["range"], "global": f.get("isGlobalRange", False),
                "air": air, "submerge": submerge, "air_modes": air_modes, "grapple": grapple, "stance": stance, "damage_speed": damage_speed,
                "attacks": attacks, "pets": pets, "abilities": abilities, "orbit": orbit, "zones": zones, "vengeance": vengeance, "temple": temple,
                "round_start_rate": round_start_rate, "bank": bank, "supports": supports,
                # PerRoundCashBonusTowerModel (Merchantman, Favored Trades...): cash each round end.
                # SpiritOfTheForestModel (Druid x-x-4+): vines along the track; each zone's
                # DamageOverTimeCustomModel by distance from the Druid (close / middle / far).
                "spirit": next(({"close": b["f"].get("closeRange", 0.0), "middle": b["f"].get("middleRange", 0.0),
                                 "zones": [(lambda d: {"damage": d.get("damage", 0.0), "interval": d.get("interval", 1.0),
                                                       "immune": d.get("immuneBloonProperties", 0),
                                                       "tags": [t for t in (deref(d.get("bloonTagsList")) or []) if isinstance(t, str)],
                                                       "mult": d.get("multiplier", 1.0), "add": d.get("additive", 0.0)})(
                                               ((deref((deref(b["f"].get(k)) or {}).get("f", {}).get("behaviorModel")) or {}).get("f") or {}))
                                           for k in ("damageOverTimeZoneModelClose", "damageOverTimeZoneModelMiddle", "damageOverTimeZoneModelFar")]}
                                for b in items(tobj["f"]["behaviors"]) if b and b["type"] == "SpiritOfTheForestModel"), None),
                # CollectCashZoneModel (Banana Farm x-x-5): bananas landed within its range (or
                # attractRange) are collected for you.
                "collect_zone": next(((0.0 if b["f"].get("useTowerRange") else b["f"].get("attractRange", 0.0))
                                      for b in items(tobj["f"]["behaviors"]) if b and b["type"] == "CollectCashZoneModel"), None),
                # ActivateAbilityAfterIntervalModel (Carpet of Spikes): the tower's ability goes
                # off by itself every `interval` seconds.
                "auto_ability": next((b["f"].get("interval", 0.0) for b in items(tobj["f"]["behaviors"])
                                      if b and b["type"] == "ActivateAbilityAfterIntervalModel"), None),
                # AddBehaviorToBloonInZoneModel with a GrowBlockModel (Grow Blocker, Druid Paragon):
                # regrow bloons in the zone don't regrow (zoneRadius; 0 = bound to the range).
                "grow_block": next(((0.0 if b["f"].get("bindRadiusToTowerRange") else b["f"].get("zoneRadius", 0.0))
                                    for b in items(tobj["f"]["behaviors"]) if b and b["type"] == "AddBehaviorToBloonInZoneModel"
                                    and any(x and x["type"] == "GrowBlockModel" for x in items(b["f"].get("behaviors")))), None),
                # FreezeNearbyWaterModel (Ice Monkey Arctic Wind): water in range freezes so land
                # towers can stand on it (radius 0: the tower's range).
                "freeze_water": next((b["f"].get("radius", 0.0) for b in items(tobj["f"]["behaviors"])
                                      if b and b["type"] == "FreezeNearbyWaterModel"), None),
                # BonusLivesPerRoundModel: livesRestored each round end while lives are under
                # maxLivesAboveStartingLives x the starting lives.
                "bonus_lives": next(([b["f"].get("livesRestored", 0), b["f"].get("maxLivesAboveStartingLives", 1.0)]
                                     for b in items(tobj["f"]["behaviors"]) if b and b["type"] == "BonusLivesPerRoundModel"), None),
                "round_cash": sum(b["f"].get("cashPerRound", 0.0) for b in items(tobj["f"]["behaviors"])
                                  if b and b["type"] == "PerRoundCashBonusTowerModel"),
                "round_cash_shared": sum(b["f"].get("cashPerRound", 0.0) for b in items(tobj["f"]["behaviors"])
                                          if b and b["type"] == "PerRoundCashBonusTowerModel" and b["f"].get("distributeCash", False)),
                # Buccaneer economy: MerchantShipModel marks a merchant ship, TradeEmpireBuffModel
                # pays per merchant ship and buffs damage, CashbackZoneModel raises sell value in
                # range, FlagshipAttackSpeedIncreaseModel speeds up attacks.
                "merchant": any(b and b["type"] == "MerchantShipModel" for b in items(tobj["f"]["behaviors"])),
                "trade_empire": next(({"per_ship": b["f"].get("cashPerRoundPerMechantship", 0.0),
                                       "per_favoured": b["f"].get("cashPerRoundPerFavouredTrades", 0.0),
                                       "cap": b["f"].get("maxMerchantmanCapBonus", 0),
                                       "damage": b["f"].get("damageBuff", 0), "ceramic": b["f"].get("ceramicDamageBuff", 0),
                                       "moab": b["f"].get("moabDamageBuff", 0)}
                                      for b in items(tobj["f"]["behaviors"]) if b and b["type"] == "TradeEmpireBuffModel"), None),
                "cashback": next(({"mult": b["f"].get("cashbackZoneMultiplier", 0.0), "max": b["f"].get("cashbackMaxPercent", 0.95),
                                   "stacks": b["f"].get("maxStacks", 1)}
                                  for b in items(tobj["f"]["behaviors"]) if b and b["type"] == "CashbackZoneModel"), None),
                "flagship": next((b["f"].get("attackSpeedIncrease", 1.0) for b in items(tobj["f"]["behaviors"])
                                  if b and b["type"] == "FlagshipAttackSpeedIncreaseModel"), None),
                # AddMakeshiftAreaModel (Carrier Flagship): deck areas, in the tower's local
                # (x right, y forward) units, that become placeable land at height z.
                "decks": [{"points": [[q[0], q[1]] for q in (deref(b["f"].get("points")) or []) if isinstance(q, list)],
                           "height": next((q[2] for q in (deref(b["f"].get("points")) or []) if isinstance(q, list) and len(q) > 2), 0.0),
                           "exclude": [x for x in (deref(b["f"].get("filterOutSpecificTowers")) or []) if isinstance(x, str)]}
                          for b in items(tobj["f"]["behaviors"]) if b and b["type"] == "AddMakeshiftAreaModel"],
                # PreEmptiveStrikeLauncherModel (Sub x-5-x): a strike at each MOAB-class bloon.
                "preempt": next((projectile(deref(b["f"]["projectileModel"])) for b in items(tobj["f"]["behaviors"])
                                 if b and b["type"] == "PreEmptiveStrikeLauncherModel" and deref(b["f"].get("projectileModel"))), None),
                # PlacementAreaTypeRangeBuffModel (Mermonkey): range x this when placed on water.
                # ChurchillBaseRotationModel (Apex Plasma Master): the base turns to the turret.
                "base_rotation": any(b and b["type"] == "ChurchillBaseRotationModel" for b in items(tobj["f"]["behaviors"])),
                "water_range": next((b["f"].get("rangeMultiplier", 1.0) for b in items(tobj["f"]["behaviors"])
                                     if b and b["type"] == "PlacementAreaTypeRangeBuffModel"), 1.0),
                "set": f.get("towerSet", 0),
                # PrinceOfDarknessZombieBuffModel: every undead bloon (from any Necromancer) gets
                # +damageIncrease damage and travels distanceMultiplier as far.
                "zombie_buff": next(([b["f"].get("damageIncrease", 0.0), b["f"].get("distanceMultiplier", 1.0)]
                                     for b in items(f["behaviors"]) if b and b["type"] == "PrinceOfDarknessZombieBuffModel"), None),
                # MonkeyCityModel (Village x-x-4+): placing a Dart Monkey costs x multiplier once
                # every roundsTillMultiplier rounds (a free Dart Monkey a round).
                "free_tower": next(({"tower": b["f"].get("towerId"), "mult": b["f"].get("multiplier", 1.0)}
                                    for b in items(f["behaviors"]) if b and b["type"] == "MonkeyCityModel"), None),
                # ComancheDefenceModel (Heli x-x-4 / x-x-5): mini Comanches (towerModel) called in.
                "comanche": next(({"cooldown": b["f"].get("cooldownFrames", 0), "duration": b["f"].get("durationFrames", 0),
                                   "per_round": b["f"].get("maxActivationsPerRound", 0), "count": b["f"].get("reinforcementCount", 0),
                                   "immediate": bool(b["f"].get("immediate")), "tower": sub_tower(deref(b["f"].get("towerModel")), None)}
                                  for b in items(f["behaviors"]) if b and b["type"] == "ComancheDefenceModel"
                                  and sub_tower(deref(b["f"].get("towerModel")), None)), None),
                # MoabShoveZoneModel (Heli x-x-3+): MOAB-class bloons within `range` of the Heli move
                # at most this fraction of their speed (negative: pushed back). FilterOutTagModel
                # leaves ZOMGs out ("zomg": null), FilterInvisibleModel camo ones.
                "moab_shove": next(({"range": b["f"].get("range", 0.0), "moab": b["f"].get("moabPushSpeedScaleCap", 1.0),
                                     "bfb": b["f"].get("bfbPushSpeedScaleCap", 1.0),
                                     "zomg": None if any(x and x["type"] == "FilterOutTagModel" and x["f"].get("tag") == "Zomg"
                                                         for x in items(b["f"].get("filters"))) else b["f"].get("zomgPushSpeedScaleCap", 1.0),
                                     "camo": not any(x and x["type"] == "FilterInvisibleModel" and x["f"].get("isActive", True)
                                                     for x in items(b["f"].get("filters")))}
                                    for b in items(f["behaviors"]) if b and b["type"] == "MoabShoveZoneModel"), None),
                # SwitchTargetSupplierOnUpgradeModel: buying this tier sets the target mode.
                "switch_target": next((b["f"].get("targetSupplierName") for b in items(f["behaviors"])
                                       if b and b["type"] == "SwitchTargetSupplierOnUpgradeModel"), None),
                # TowerModel.towerSelectionMenuThemeId: the upgrade panel's theme (Camo, MonkeySub...).
                "tsm_theme": f.get("towerSelectionMenuThemeId") or "Default",
                "sounds": {"place": behavior_sounds(items(f["behaviors"]), "CreateSoundOnTowerPlaceModel", ["sound1", "sound2"]),
                           "upgrade": behavior_sounds(items(f["behaviors"]), "CreateSoundOnUpgradeModel", ["sound"]),
                           "sell": behavior_sounds(items(f["behaviors"]), "CreateSoundOnSellModel", ["sound"]),
                           # Hero voice lines: CreateSoundOnTowerPlace.heroSound*, CreateSoundOnUpgrade
                           # sound1-8 (PlayHeroUpgradeSound: Random.Range(0, 8)), CreateSoundOnSelected,
                           # CreateSoundOnBloonLeak, CreateSoundOnBloonDestroyed (Moabs) and
                           # CreateSoundOnBloonEnterTrack (a warning per MOAB class).
                           # CreateSoundOnAttached (the tier's own upgrade fanfare: Sun God, Tech
                           # Terror, Dark Knight) and CreateSoundOnEndOfRound (Rare Quincy figure).
                           "attached": behavior_sounds(items(f["behaviors"]), "CreateSoundOnAttachedModel", ["sound"]),
                           "round_end": behavior_sounds(items(f["behaviors"]), "CreateSoundOnEndOfRoundModel", ["sound"]),
                           "hero_place": behavior_sounds(items(f["behaviors"]), "CreateSoundOnTowerPlaceModel", ["heroSound1", "heroSound2"]),
                           "hero_upgrade": [next(iter(sounds(b, [k])), "") for b in items(f["behaviors"])
                                            if b and b["type"] == "CreateSoundOnUpgradeModel" for k in [f"sound{i}" for i in range(1, 9)]],
                           "select": behavior_sounds(items(f["behaviors"]), "CreateSoundOnSelectedModel", [f"sound{i}" for i in range(1, 7)]),
                           "select_alt": behavior_sounds(items(f["behaviors"]), "CreateSoundOnSelectedModel", ["altSound1", "altSound2"]),
                           "leak": behavior_sounds(items(f["behaviors"]), "CreateSoundOnBloonLeakModel", ["sound1", "sound2"]),
                           "moab_pop": behavior_sounds(items(f["behaviors"]), "CreateSoundOnBloonDestroyedModel", ["sound1", "sound2"]),
                           "enter": {k[:-5]: v[0] for b in items(f["behaviors"]) if b and b["type"] == "CreateSoundOnBloonEnterTrackModel"
                                     for k in ("moabSound", "bfbSound", "zomgSound", "ddtSound", "badSound") for v in [sounds(b, [k])] if v}}}

    sub_cache, sub_busy = {}, set()

    def sub_tower(tobj, display):
        """A sub-tower spawned by a projectile: its tier data, lifespan and display prefab."""
        if not tobj or tobj.get("type") != "TowerModel":
            return None
        key = (tobj["i"], display)
        if key in sub_cache:
            return sub_cache[key]
        if tobj["i"] in sub_busy:
            return None
        sub_busy.add(tobj["i"])
        f = tobj["f"]
        bs = [b for b in items(f["behaviors"]) if b]
        expire = next((b["f"].get("lifespan", 0.0) for b in bs if b["type"] == "TowerExpireModel"), 0.0)
        round_end = any(b["f"].get("expireOnRoundComplete") for b in bs if b["type"] == "TowerExpireModel")
        foot = next((b["f"] for b in bs if b["type"] == "CircleFootprintModel"), {})
        # The tower display is the stand (SentryGunLegs); the attack's DisplayModel the turret.
        turret = next((x["f"].get("display") for b in bs if b["type"] == "AttackModel"
                       for x in items(b["f"].get("behaviors")) if x and x.get("type") == "DisplayModel"), None)
        rounds = next((b["f"].get("rounds", 0) for b in bs if b["type"] == "TowerExpireModel"), 0)
        spec = {"name": f["_name"], "tier": tier(tobj), "lifespan": expire, "round_end": round_end, "rounds": rounds, "radius": foot.get("radius", f.get("radius", 0.0)),
                "areas": [a for a in (deref(f.get("areaTypes")) or []) if isinstance(a, int)],
                "base": prefab_name(f.get("display")), "model": prefab_name(turret or display),
                "credit_parent": any(b["type"] == "CreditPopsToParentTowerModel" for b in bs),
                "id": f.get("baseId") or f["_name"],
                "with_parent": any(b["type"] == "TowerExpireOnParentDestroyedModel" for b in bs),
                # CreateEffectOnPlaceModel / CreateEffectOnExpireModel (mini Sun Avatar): (effect, scale).
                "place_fx": next(([prefab_name((deref(b["f"].get("effectModel")) or {}).get("f", {}).get("assetId")),
                                   (deref(b["f"].get("effectModel")) or {}).get("f", {}).get("scale", 1.0)]
                                  for b in bs if b["type"] == "CreateEffectOnPlaceModel"), None),
                "expire_fx": next(([prefab_name((deref(b["f"].get("effectModel")) or {}).get("f", {}).get("assetId")),
                                    (deref(b["f"].get("effectModel")) or {}).get("f", {}).get("scale", 1.0)]
                                   for b in bs if b["type"].endswith("CreateEffectOnExpireModel")), None),
                # OrbitingTowerModel (Adora's Ball of Light, Dark Phoenix): circles its parent.
                "orbit_tower": next(({"radius": b["f"].get("radius", 0.0), "rate": b["f"].get("rotationDegreesPerFrame", 0.0)}
                                     for b in bs if b["type"] == "OrbitingTowerModel"), None),
                # PathMovementFromScreenCenterModel (Phoenix): circles the screen centre at `speed`,
                # radius from its attack's PathMovementFromScreenCenterPatternModel.
                "circle": next(({"speed": b["f"].get("speed", 0.0),
                                 "radius": next((x["f"].get("radius", 0.0) for a in bs if a["type"] == "AttackModel"
                                                 for x in items(a["f"].get("behaviors"))
                                                 if x and x["type"] == "PathMovementFromScreenCenterPatternModel"), 0.0)}
                                for b in bs if b["type"] == "PathMovementFromScreenCenterModel"), None)}
        sub_busy.discard(tobj["i"])
        sub_cache[key] = spec
        return spec

    def tower(name):
        t = next(r for r in objs.values() if r["type"] == "TowerModel" and r["f"]["_name"] == name)
        f = t["f"]
        weapon = next((w for b in items(f["behaviors"]) if b and b["type"] == "AttackModel"
                       for w in items(b["f"]["weapons"]) if w), None)
        # Support towers (Village) have no weapon: a stand-in so the shop entry still loads.
        weapon = weapon or {"f": {"rate": 0.0}, "i": None}
        proj = deref(weapon["f"].get("projectile")) or {"f": {"pierce": 0.0, "radius": 0.0, "behaviors": None}, "i": None}
        pb = {b["type"]: b["f"] for b in items(proj["f"]["behaviors"]) if b}
        return {
            "name": name, "cost": f["cost"], "range": f["range"], "radius": f["radius"],
            "global": f.get("isGlobalRange", False),
            "areas": [a for a in (deref(f.get("areaTypes")) or []) if isinstance(a, int)],
            "icon": f["icon"].split("[")[-1].rstrip("]") if "[" in f["icon"] else name + "Icon",
            "rate": weapon["f"]["rate"],
            "projectile": {
                "pierce": proj["f"]["pierce"], "radius": proj["f"]["radius"],
                "damage": pb.get("DamageModel", {}).get("damage"),
                "speed": pb.get("TravelStraitModel", {}).get("speed"),
                "lifespan": pb.get("TravelStraitModel", {}).get("lifespan"),
            },
            "sources": {"tower": t["i"], "weapon": weapon["i"], "projectile": proj["i"]},
            # CreateEffectOnPlaceModel: the effect played where it's placed (PlacementEffect,
            # HeroPlacementEffect...) and its EffectModel.scale.
            "place_fx": next(([prefab_name(e["f"].get("assetId")), e["f"].get("scale", 1.0)]
                              for b in items(f["behaviors"]) if b and b["type"] == "CreateEffectOnPlaceModel"
                              for e in [deref(b["f"].get("effectModel"))] if e), None),
        }

    def tower_tiers(name):
        # Only "<name>" and "<name>-xyz": other models share the baseId (TransformedMonkey).
        sep = " " if name in HEROES else "-"
        models = [r for r in objs.values() if r["type"] == "TowerModel" and r["f"]["baseId"] == name
                  and (r["f"]["_name"] == name or r["f"]["_name"].startswith(name + sep))
                  and "Paragon" not in r["f"]["_name"]]
        names = {u["f"]["upgrade"] for r in models for u in items(r["f"].get("upgrades")) if u}
        hero = None
        if name in HEROES:
            # HeroModel (xpScale) and the level UpgradeModels' xpCost: Hero.CheckForLevelUp levels up
            # while xp >= next.xpCost, subtracting it (research: Hero.AddXp / CheckForLevelUp).
            base = next(r for r in models if r["f"]["_name"] == name)
            hm = next((b["f"] for b in items(base["f"]["behaviors"]) if b and b["type"] == "HeroModel"), {})
            ups = sorted((r["f"] for r in objs.values() if r["type"] == "UpgradeModel" and r["f"]["_name"] in names),
                         key=lambda u: u["tier"])
            hero = {"xp_scale": hm.get("xpScale", 1.0), "cost_per_xp": hm.get("costPerXpToLevel", 1.0),
                    "levels": [{"level": u["tier"] + 1, "name": u["_name"], "xp": u["xpCost"]} for u in ups]}
        # The Paragon (TowerModel isParagon) and its UpgradeModel "<name> Paragon".
        pt = next((r for r in objs.values() if r["type"] == "TowerModel" and r["f"]["baseId"] == name and r["f"].get("isParagon")), None)
        pu = next((r["f"] for r in objs.values() if r["type"] == "UpgradeModel" and r["f"]["_name"] == name + " Paragon"), None)
        # ParagonTowerModel.displayDegreePaths: the attack display drawn on the tower display
        # (Lvl1..Lvl5 by display degree; the first is used).
        ptm = next((b["f"] for b in items(pt["f"]["behaviors"]) if b and b["type"] == "ParagonTowerModel"), {}) if pt else {}
        dpaths = [(x["f"].get("assetPath") if isinstance(x, dict) and "f" in x else None) for x in items(ptm.get("displayDegreePaths"))]
        top = prefab_name(dpaths[0]) if dpaths and dpaths[0] and ptm.get("changeAttackDisplay") else None
        tops = [prefab_name(g) for g in dpaths if g] if ptm.get("changeAttackDisplay") else []
        # Without changeAttackDisplay the degree displays are the tower's own.
        bases = [prefab_name(g) for g in dpaths if g] if not ptm.get("changeAttackDisplay") else []
        paragon = ({"tier": tier(pt), "cost": pu["cost"], "name": pu["_name"], "display": prefab_name(pt["f"].get("display")), "top": top,
                    "tops": tops, "bases": bases, "levels": len(dpaths),
                    "base_display": pt["f"].get("display"),
                    # ParagonTowerModel.inactiveDurationFrames: attacks paused while the sacrifice plays.
                    "inactive_frames": int(round(ptm.get("inactiveDuration", 0.0) * 60))}
                   if pt and pu else None)
        return {
            "hero": hero,
            "paragon": paragon,
            "tiers": {"".join(str(x) for x in items(r["f"]["tiers"])): tier(r) for r in models},
            "upgrades": sorted([{"name": r["f"]["_name"], "cost": r["f"]["cost"], "path": r["f"]["path"], "tier": r["f"]["tier"]}
                                for r in objs.values() if r["type"] == "UpgradeModel" and r["f"]["_name"] in names],
                               key=lambda u: (u["path"], u["tier"])),
        }

    gm = objs[root_idx]
    assert gm["type"] == "GameModel", gm["type"]
    # ParagonDegreeDataModel (GameModel.paragonDegreeDataModel): power thresholds per degree and
    # the per-degree stat steps (research: ParagonTower.GetTowerInvestment / CombineInvestments /
    # GetDegreeFromInvestment / GetDegreeMutator, PowerDegreeMutator.MutateTower).
    pdm = deref(gm["f"].get("paragonDegreeDataModel"))
    paragon_degree = None
    if pdm:
        pf = pdm["f"]
        paragon_degree = {k: pf.get(k) for k in ("attackCooldownReductionX", "piercePercentPerDegree", "pierceIncreasePerDegree",
                                               "damagePercentPerDegree", "damageIncreasePerDegree", "damageIncreaseForDegrees",
                                               "degreeCount", "maxPowerFromPops", "maxPowerFromMoneySpent", "maxPowerFromNonTier5Count",
                                               "maxPowerFromTier5Count", "popsOverX", "moneySpentOverX", "nonTier5TowersMultByX",
                                               "tier5TowersMultByX")}
        paragon_degree["requirements"] = [v for v in items(pf.get("powerDegreeRequirements")) if isinstance(v, (int, float))]             or [v for v in (deref(pf.get("powerDegreeRequirements")) or []) if isinstance(v, (int, float))]
    medium = {m["class"]: m for m in json.load(open(os.path.join(root, "data", "modes.json")))["Medium"]}
    round_set = json.load(open(os.path.join(root, "data", "round_sets.json")))["DefaultRoundSet"]
    rounds = round_set["rounds"]
    displays = json.load(open(os.path.join(root, "bloon_displays", "displays.json")))
    for bid, b in bloons.items():
        layers = displays.get(bid)
        if layers:
            b["display"] = [{"png": "bloon_displays/" + l["png"], "w": l["w"], "h": l["h"],
                             "offset": l["offset"]} for l in layers]
    game_map = json.load(open(os.path.join(root, "maps", map_key, "map.json")))

    # Tower shop (ShopTowerDetailsModel order; towerCount 0 = not in the standard shop), with the
    # default hero (HeroDetailsModel towerIndex 0) first like the in-game shop.
    def shop_entry(tid, hero):
        t = next(r for r in objs.values() if r["type"] == "TowerModel" and r["f"]["_name"] == tid)
        icon = t["f"].get("icon") or ""
        return {"id": tid, "cost": t["f"]["cost"], "set": t["f"].get("towerSet", 1), "hero": hero,
                "icon": icon.split("[")[-1].rstrip("]") if "[" in icon else None}
    heroes = sorted((r["f"] for r in objs.values() if r["type"] == "HeroDetailsModel"), key=lambda f: f["towerIndex"])
    shop = [shop_entry(heroes[0]["towerId"], True)] if heroes else []
    shop += [shop_entry(f["towerId"], False) for f in sorted(
        (r["f"] for r in objs.values() if r["type"] == "ShopTowerDetailsModel" and r["f"]["towerCount"] != 0),
        key=lambda f: f["towerIndex"])]
    # Difficulties (data/modes.json: each mode's ModModels). "Clicks" is CHIMPS. Defaults where a
    # mode has no such mod: $650, 150 lives, 70% sell, x1 cost and speed, rounds 1..all, $100 +
    # round per round (CHIMPS: no round bonus).
    modes = {}
    all_modes = json.load(open(os.path.join(root, "data", "modes.json")))
    # The mode-select screen's choices per difficulty (ModeSelectScreen); a game's mods are the
    # difficulty's, then the sub-mode's on top.
    choices = {"Easy": ["PrimaryOnly", "Deflation", "Sandbox"],
               "Medium": ["MilitaryOnly", "Apopalypse", "Reverse", "Sandbox"],
               "Hard": ["MagicOnly", "DoubleMoabHealth", "HalfCash", "AlternateBloonsRounds", "Impoppable", "Clicks", "Sandbox"]}
    combos = [(d, None) for d in choices] + [(d, sub) for d, subs in choices.items() for sub in subs]
    for diff, sub in combos:
        if sub is None:
            mname, mods, sources = diff, all_modes[diff], [all_modes[diff]]
        else:
            # Impoppable / CHIMPS keep their names; other sub-modes are "<Difficulty> <Mode>".
            mname = sub if sub in ("Impoppable", "Clicks") else f"{diff} {sub}"
            extra = all_modes[sub]
            # HYPOTHESIS H-MODE-2: the difficulty's mods apply first and the mode's on top.
            # GlobalCostModModel.Mutate adds (mult - 1) x cost, so cost multipliers add up
            # (Hard 1.08 + Impoppable 1.12 -> 1.2); LockTowerSet mods add up; other mods replace.
            stack = {"GlobalCostModModel", "LockTowerSetModModel", "LockTowerModModel"}
            sources = [all_modes[diff], extra]
            keep = {y["class"] for y in extra} - stack
            mods = [x for x in all_modes[diff] if x["class"] not in keep] + extra
        cost_mult = 1.0 + sum(x["multiplier"] - 1.0 for src in sources for x in src if x["class"] == "GlobalCostModModel")
        m = {x["class"]: x for x in mods}
        g = lambda c, k, d: m.get(c, {}).get(k, d)
        chimps = "ChimpsModModel" in m
        # StartingCashModModel: changeBase replaces the cash (0 keeps it), then x multiplier (0:
        # unchanged) + addition, in mod order (Half Cash: 650 -> 325).
        cash = 650.0
        for x in mods:
            if x["class"] == "StartingCashModModel":
                cash = x["changeBase"] or cash
                cash = cash * (x["multiplier"] or 1.0) + x.get("addition", 0.0)
        lives = 150.0
        for x in mods:
            if x["class"] == "StartingHealthModModel":
                lives = x["addition"] if not x["multiplier"] else lives * x["multiplier"] + x["addition"]
        all_cash = 1.0
        for x in mods:
            if x["class"] == "ModifyAllCashModModel":
                all_cash *= x["multiplier"]
        moab_hp = 1.0
        for x in mods:
            if x["class"] == "BloonHealthModel" and x.get("bloonTag") == "Moabs":
                moab_hp *= x["healthMod"]
        modes["CHIMPS" if mname == "Clicks" else mname] = {
            "difficulty": diff, "sub": sub,
            "locked_sets": [x["towerSetToLock"] for x in mods if x["class"] == "LockTowerSetModModel"],
            "cash_mult": all_cash, "moab_health": moab_hp,
            "round_set": g("RoundSetModModel", "roundsetName", None),
            "reverse": "ReverseModel" in m, "sandbox": sub == "Sandbox", "apopalypse": sub == "Apopalypse",
            "deflation": "DeflationModel" in m,
            "cash": cash,
            "lives": g("MaxHealthModModel", "set", None) or lives,
            "sell": g("SellMultiplierModModel", "addition", 0.7),
            "cost_mult": cost_mult,
            "speed_mult": g("GlobalSpeedModModel", "multiplier", 1.0), "speed_add": g("GlobalSpeedModModel", "addition", 0.0),
            "start_round": g("StartingRoundModModel", "round", 1), "end_round": g("EndRoundModModel", "round", len(rounds)),
            "bonus": None if chimps else [g("BonusCashPerRoundModModel", "baseCash", 100.0), g("BonusCashPerRoundModModel", "roundMultiple", 1.0)],
            "locked": [x["towerToLock"] for x in mods if x["class"] == "LockTowerModModel"],
            "no_sell": "DisableSellTowerModModel" in m,
            # SetHealthForBloonModModel (Easy: round 40's MOAB at 2/3 health).
            "health": [{"round": x["round"], "bloon": x["bloonId"], "mult": x["healthMultiplier"]}
                       for x in mods if x["class"] == "SetHealthForBloonModModel"],
        }
    def geraldo_items():
        """GeraldoItemModel (Geraldo's shop): stock, unlock level, price, and what using the
        item does: place a tower (towerId, swapped for GeraldoUpgradeTowerModel's at a level),
        put a projectile on the track (GeraldoCreateProjectileModel, with level upgrades), or a
        tower / player effect (the *BehaviorModel parameters)."""
        towers_by_name = {r["f"]["_name"]: r for r in objs.values() if r["type"] == "TowerModel"}
        out = []
        for r in objs.values():
            if r["type"] != "GeraldoItemModel":
                continue
            f = r["f"]
            item = {"name": f["_name"], "loc": f.get("locsId") or f["_name"], "cost": f.get("cost", 0),
                    "start": f.get("startingQuantity", 0), "max": f.get("maxQuantity", 0),
                    "every": f.get("roundsToReplenish", 1), "amount": f.get("amountToReplenish", 1),
                    "unlock": f.get("levelUnlockedAt", 0), "only_if_none": bool(f.get("onlyReplenishIfNonePlaced")),
                    "banned": [x.strip() for x in (f.get("bannedForModes") or "").split(",") if x.strip()],
                    "block_if_placed": f.get("blockPurchaseIfTowerPlaced") or "",
                    "kind": "self", "towers": [], "projectiles": [], "params": {}}
            if f.get("towerId") and f["towerId"] in towers_by_name:
                item["kind"] = "tower"
                sp = sub_tower(towers_by_name[f["towerId"]], None)
                if sp:
                    item["towers"].append([0, sp])
            for b in items(f.get("behaviorModels")):
                if not b:
                    continue
                bf = b["f"]
                if b["type"] == "GeraldoUpgradeTowerModel" and bf.get("towerId") in towers_by_name:
                    sp = sub_tower(towers_by_name[bf["towerId"]], None)
                    if sp:
                        item["towers"].append([bf.get("level", 0), sp])
                elif b["type"] == "GeraldoCreateProjectileModel" and deref(bf.get("projectileModel")):
                    item["kind"] = "track"
                    item["projectiles"].append([0, projectile(deref(bf["projectileModel"]))])
                    levels = [v for v in (deref(bf.get("projectileModelUpgradeAtLevels")) or []) if isinstance(v, int)]
                    for lv, pm in zip(levels, items(bf.get("projectileModelUpgrades"))):
                        if pm:
                            item["projectiles"].append([lv, projectile(pm)])
                    item["params"]["random_rotation"] = bool(bf.get("randomRotation"))
                elif b["type"] in ("JarOfPicklesBehaviorModel", "SharpeningStoneBehaviorModel", "SeeInvisibilityPotionBehaviorModel",
                                   "FertilizerBehaviorModel", "BottleHotSauceBehaviorModel", "PetRabbitBehaviorModel",
                                   "GeraldoChangeTowerBehaviorModel"):
                    item["kind"] = {"JarOfPicklesBehaviorModel": "pickles", "SharpeningStoneBehaviorModel": "stone",
                                    "SeeInvisibilityPotionBehaviorModel": "potion", "FertilizerBehaviorModel": "fertilizer",
                                    "BottleHotSauceBehaviorModel": "sauce", "PetRabbitBehaviorModel": "rabbit",
                                    "GeraldoChangeTowerBehaviorModel": "cape"}[b["type"]]
                    item["params"] = {k: v for k, v in bf.items() if isinstance(v, (int, float, str, bool)) and k not in ("_name", "runtimeTypeIndex", "isTower")}
                    # BottleHotSauceBehavior.Activate: a fire creature orbiting the tower
                    # (HotSauceCreatureTower, its V2 from level v2At).
                    if b["type"] == "BottleHotSauceBehaviorModel":
                        for lv, nm in ((0, "HotSauceCreatureTower"), (bf.get("v2At", 0), "HotSauceCreatureTowerV2")):
                            sp = sub_tower(towers_by_name.get(nm), None)
                            if sp:
                                item["towers"].append([lv, sp])
                    # PetRabbitBehavior: a pet rabbit (petRabbitBaseTowerModel); its
                    # PetRabbitMergeModel turns mergeAmountRequired of them into a Wererabbit.
                    if b["type"] == "PetRabbitBehaviorModel" and deref(bf.get("petRabbitBaseTowerModel")):
                        base = deref(bf["petRabbitBaseTowerModel"])
                        sp = sub_tower(base, None)
                        # The rabbit itself is its PetModelSynced's display.
                        pet = next((prefab_name(m["f"].get("display")) for m in items(base["f"].get("behaviors"))
                                    if m and m["type"] == "PetModelSynced"), None)
                        if sp:
                            item["towers"].append([0, dict(sp, base=pet) if pet and not sp.get("base") else sp])
                        for m in items(base["f"].get("behaviors")):
                            if m and m["type"] == "PetRabbitMergeModel":
                                wsp = sub_tower(towers_by_name.get(m["f"].get("wererabbitTowerId")), None)
                                if wsp:
                                    item["merge"] = [m["f"].get("mergeAmountRequired", 4), wsp]
                elif b["type"] == "RejuvPotionBehaviorModel":
                    item["kind"] = "rejuv"
                    item["params"] = {"lives": bf.get("livesGained", 0)}
            out.append(item)
        return out

    out = {
        "geraldo": geraldo_items(),
        "modes": modes,
        "start": {"mode": "Medium", "cash": medium["StartingCashModModel"]["changeBase"],
                  "lives": medium["StartingHealthModModel"]["addition"], "mods": list(medium.values())},
        "bloons": bloons,
        "towers": {name: tower(name) for name in PLAYABLE},
        "tower_tiers": {name: tower_tiers(name) for name in PLAYABLE},
        "rounds": [r["bloonGroups"] for r in rounds],
        # RoundSetModModel sets (Alternate Bloons Rounds).
        "round_sets": {k: [r["bloonGroups"] for r in v["rounds"]] for k, v in
                       json.load(open(os.path.join(root, "data", "round_sets.json"))).items() if k == "AlternateRoundSet"},
        "end_round": medium.get("EndRoundModModel", {}).get("round", len(rounds)),
        "freeplay": freeplay_groups(),
        # GameData.bloonMutatorData (export_rounds.py): BadImmunity.FilterMutation's lists.
        "bloon_mutator_data": (json.load(open(md)) if os.path.exists(md := os.path.join(root, "data", "bloon_mutator_data.json")) else {}),
        "shop": shop,
        # Every hero's shop entry (HeroDetailsModel order): the player picks which one the
        # shop's hero slot holds.
        "heroes": [shop_entry(f["towerId"], True) for f in heroes if f["towerId"] in HEROES],
        "round_scale": round_set.get("scale", 1.0),
        "paragon_degree": paragon_degree,
        "map": game_map,
        "notes": [
            "Emission spacing inside a bloon group is a HYPOTHESIS (evenly spaced over duration).",
            "Path curve sampling is an APPROXIMATION (H-PATH-2).",
        ],
    }
    os.makedirs(os.path.join(root, "game"), exist_ok=True)
    # Some hero projectiles divide by a zero speed (NaN lifespan); JSON has no NaN / Infinity.
    def finite(v):
        if isinstance(v, float) and not math.isfinite(v):
            return 0.0
        if isinstance(v, dict):
            return {k: finite(x) for k, x in v.items()}
        if isinstance(v, list):
            return [finite(x) for x in v]
        return v

    # Kept from the previous game.json: what tools/mod/import_paths.py adds from the real game's
    # map dump (CollisionGrid.orderedNeighborOffsets).
    gp = os.path.join(root, "game", "game.json")
    if os.path.exists(gp):
        prev = json.load(open(gp, encoding="utf-8"))
        for k in ("grid_offsets",):
            if k in prev and k not in out:
                out[k] = prev[k]
    json.dump(finite(out), open(gp, "w"), indent=1, allow_nan=False)
    print("bloons", len(bloons), "rounds", len(out["rounds"]), "start", out["start"])
    print("DartMonkey", json.dumps(out["towers"]["DartMonkey"]))
    for b in ("Red", "Blue", "Green", "Yellow", "Pink", "Black", "White", "Zebra", "Lead", "Rainbow",
              "Ceramic", "Moab"):
        print(b, json.dumps(bloons.get(b)))


if __name__ == "__main__":
    main()
