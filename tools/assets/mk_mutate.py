"""Monkey Knowledge from the knowledge models themselves (the distributable path).

For each TowerModel in GameModel.towers, each ApplyModModel in its `mods` whose mod is active (every
knowledge item: the owner's profile) runs that ModModel's mutatorMods on the tower, each mutator as
its Mutate does (decomp; `MUTATORS`). Each tower gets its own copies of what it changes (models are
shared between towers in the decoded set).

    python -I tools/assets/mk_mutate.py local_extracted local_extracted/decoded_mk_engine \
        [--oracle local_extracted/decoded_mk]

With --oracle (apply_mk.py's set, from the game's own modded models): every field where the two
differ, counted by (model type, field), and the first examples. Output is proprietary game data:
keep it local.
"""
import collections
import copy
import json
import os
import struct
import sys


def f32(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


class Model:
    def __init__(self, root):
        self.objs = {}
        for line in open(os.path.join(root, "named", "objects.jsonl"), encoding="utf-8"):
            r = json.loads(line)
            self.objs[r["i"]] = r
        self.values = {int(k): v for k, v in json.load(open(os.path.join(root, "values.json"))).items()}
        self.next_id = max(max(self.objs), max(self.values)) + 1

    def new_id(self):
        self.next_id += 1
        return self.next_id

    def items(self, v):
        if not (isinstance(v, dict) and "$ref" in v):
            return []
        c = self.values.get(v["$ref"])
        if isinstance(c, dict) and "items" in c:
            return c["items"]
        return []

    def ref(self, v):
        if isinstance(v, dict) and "$ref" in v and v["$ref"] in self.objs:
            return v["$ref"]
        return None


class Tower:
    """One tower's own copy of its model tree (every model and container reachable through fields,
    shared ones kept shared within the tower), so mutators write in place."""

    def __init__(self, m, tid, copy_tree=True):
        self.m = m
        self.root = tid
        if not copy_tree:
            return
        memo = {}

        def cp(oid):
            if oid in memo:
                return memo[oid]
            nid = m.new_id()
            memo[oid] = nid
            o = copy.deepcopy(m.objs[oid])
            o["i"] = nid
            m.objs[nid] = o
            for k, v in list(o["f"].items()):
                if isinstance(v, dict) and "$ref" in v:
                    r = v["$ref"]
                    if r in m.objs:
                        o["f"][k] = {"$ref": cp(r)}
                    elif isinstance(m.values.get(r), dict) and "items" in m.values[r]:
                        if ("c", r) not in memo:
                            c = copy.deepcopy(m.values[r])
                            cid = m.new_id()
                            memo[("c", r)] = cid
                            m.values[cid] = c
                            c["items"] = [cp(e) if isinstance(e, int) and e in m.objs else e for e in c["items"]]
                        o["f"][k] = {"$ref": memo[("c", r)]}
            return nid

        # (The tower object itself keeps its id: GameModel.towers points at it.)
        self.root = tid
        memo[tid] = tid
        self.memo = memo
        o = m.objs[tid]
        for k, v in list(o["f"].items()):
            if isinstance(v, dict) and "$ref" in v:
                r = v["$ref"]
                if r in m.objs:
                    o["f"][k] = {"$ref": cp(r)}
                elif isinstance(m.values.get(r), dict) and "items" in m.values[r]:
                    c = copy.deepcopy(m.values[r])
                    cid = m.new_id()
                    m.values[cid] = c
                    c["items"] = [cp(e) if isinstance(e, int) and e in m.objs else e for e in c["items"]]
                    o["f"][k] = {"$ref": cid}

    def descendants_all_holders(self):
        """Every model below the root through any field, not into nested TowerModels."""
        out, seen = [], set()

        def rec(oid):
            if oid in seen:
                return
            seen.add(oid)
            o = self.m.objs[oid]
            if o["type"] == "TowerModel" and oid != self.root:
                return
            out.append(o)
            for k, v in o["f"].items():
                if k == "childDependants":
                    continue
                r = self.m.ref(v)
                if r is not None:
                    rec(r)
                    continue
                for e in self.m.items(v):
                    if isinstance(e, int) and e in self.m.objs:
                        rec(e)
        rec(self.root)
        return out

    def descendants_all_towers(self):
        """TowerModels reachable through any model field below the root (not the root)."""
        out, seen = [], set()

        def rec(oid):
            if oid in seen:
                return
            seen.add(oid)
            o = self.m.objs[oid]
            if o["type"] == "TowerModel" and oid != self.root:
                out.append(oid)
                return
            for k, v in o["f"].items():
                if k == "childDependants":
                    continue
                r = self.m.ref(v)
                if r is not None:
                    rec(r)
                    continue
                for e in self.m.items(v):
                    if isinstance(e, int) and e in self.m.objs:
                        rec(e)
        rec(self.root)
        return out

    def descendants(self, types=None):
        """Model.GetDescendants: every model under the tower through childDependants, depth first,
        the tower itself first. HYPOTHESIS P-MKCHILDREN (oracle: the Sub's submergeAttackModel
        weapons are left alone)."""
        out, seen = [], set()

        def rec(oid):
            if oid in seen:
                return
            seen.add(oid)
            o = self.m.objs[oid]
            if types is None or o["type"] in types:
                out.append(o)
            # (Not into towers it makes: the game's mods leave a CreateTowerModel's tower as it
            # is; oracle: Buccaneer 4-0-0's plane weapon keeps its rate.)
            if o["type"] == "TowerModel" and oid != self.root:
                return
            for e in self.m.items(o["f"].get("childDependants")):
                if isinstance(e, int) and e in self.m.objs:
                    rec(e)
        rec(self.root)
        return out


def name_of(o):
    return o["f"].get("_name") or ""


def targets_of(target):
    """StringExtensions.CommaSeperatedListToStringArray(target, trim, remove empty)."""
    return [t.strip() for t in (target or "").split(",") if t.strip()]


def has_target(o, targets):
    """MutatorModModel.HasTargetName (VERIFIED-DECOMP @0x18106f540): no targets, or the model's
    name (one trailing '_' dropped) ends with one of them."""
    if not targets:
        return True
    n = name_of(o)
    if n.endswith("_"):
        if any(n.endswith(t) for t in targets):
            return True
        n = n[:-1]
    return any(n.endswith(t) for t in targets)


def fl(o, k):
    return f32(o["f"].get(k) or 0.0)


def desc(tw, typ, target=None):
    ts = targets_of(target) if target is not None else []
    return [o for o in tw.descendants({typ}) if has_target(o, ts)]


# ---- mutators: Mutate(model) per type (VERIFIED-DECOMP at each address) ------------------------


def m_weapon_reload_pct(tw, mm, target):
    # WeaponReloadPercentageModModel @0x1812b98b0: every WeaponModel, rate - rate x multiplier.
    k = f32(mm.get("multiplier", 0.0))
    for o in tw.descendants({"WeaponModel"}):
        r = fl(o, "rate")
        o["f"]["rate"] = f32(r - f32(r * k))


def m_weapon_reload_reduction(tw, mm, target):
    # WeaponReloadReductionModModel @0x1812b9d40: named WeaponModels, rate - reductionAmount.
    for o in desc(tw, "WeaponModel", target):
        o["f"]["rate"] = f32(fl(o, "rate") - f32(mm.get("reductionAmount", 0.0)))


def m_pierce(tw, mm, target):
    # ProjectilePierceModModel: named ProjectileModels, pierce + amount; with minTier, only a tower
    # with a tier at least that (HYPOTHESIS P-MKMINTIER).
    mt = mm.get("minTier", 0)
    if mt >= 1:
        tr = tw.m.objs[tw.root]["f"].get("tiers")
        tv = tw.m.values.get(tr["$ref"]) if isinstance(tr, dict) and "$ref" in tr else []
        tv = tv if isinstance(tv, list) else tw.m.items(tr)
        if not any(isinstance(v, int) and v >= mt for v in tv):
            return
    for o in desc(tw, "ProjectileModel", target):
        o["f"]["pierce"] = f32(fl(o, "pierce") + f32(mm.get("amount", 0.0)))


def m_ability_cd_pct(tw, mm, target):
    # AbilityCooldownPercentageModModel: named AbilityModels, cooldown - cooldown x reduction.
    k = f32(mm.get("cooldownPercentageReduction", 0.0))
    for o in desc(tw, "AbilityModel", target):
        c = fl(o, "cooldown")
        o["f"]["cooldown"] = f32(c - f32(c * k))


def m_ability_cd_time(tw, mm, target):
    # AbilityCooldownTimeModModel: cooldown - reduction, at least 0.05.
    r = f32(mm.get("cooldownTimeReduction", 0.0))
    for o in desc(tw, "AbilityModel", target):
        c = f32(fl(o, "cooldown") - r)
        o["f"]["cooldown"] = c if f32(0.05) <= c else f32(0.05)


def m_proj_range_pct(tw, mm, target):
    # ProjectileRangePercentageModModel @0x1812ac320: every TravelStraitModel and AgeModel,
    # lifespan x percentIncrease + lifespan.
    k = f32(mm.get("percentIncrease", 0.0))
    for t in ("TravelStraitModel", "AgeModel"):
        for o in tw.descendants({t}):
            v = fl(o, "lifespan")
            o["f"]["lifespan"] = f32(f32(v * k) + v)


def m_range(tw, mm, target):
    # RangeModModel @0x1812ad4a0: the tower's range (x multiplier when it isn't 0) + additive;
    # named AttackModels without fixedRange: range x multiplier + additive.
    k, add = f32(mm.get("multiplier", 0.0)), f32(mm.get("additive", 0.0))
    t = tw.m.objs[tw.root]
    r = fl(t, "range")
    t["f"]["range"] = f32((r if k == 0.0 else f32(r * k)) + add)
    for o in desc(tw, "AttackModel", target):
        if not o["f"].get("fixedRange"):
            o["f"]["range"] = f32(f32(fl(o, "range") * k) + add)


def m_proj_speed(tw, mm, target):
    # ProjectileSpeedModModel @0x1812ac840: named TravelStraitModels' speed and TrackTargetModels'
    # turnRate, x multiplier + itself.
    k = f32(mm.get("multiplier", 0.0))
    for o in desc(tw, "TravelStraitModel", target):
        v = fl(o, "speed")
        o["f"]["speed"] = f32(f32(v * k) + v)
    for o in desc(tw, "TrackTargetModel", target):
        v = fl(o, "turnRate")
        o["f"]["turnRate"] = f32(f32(v * k) + v)


def m_proj_radius(tw, mm, target):
    # ProjectileRadiusModModel @0x1812abd00: named ProjectileModels, radius x multiplierAmount +
    # additiveAmount.
    for o in desc(tw, "ProjectileModel", target):
        o["f"]["radius"] = f32(f32(fl(o, "radius") * f32(mm.get("multiplierAmount", 0.0))) + f32(mm.get("additiveAmount", 0.0)))


def m_damage_type(tw, mm, target):
    # DamageTypeModModel: named DamageModels' and DamageOverTimeModels' immuneBloonProperties
    # lose canHitbloonProperties.
    bits = mm.get("canHitbloonProperties", 0)
    for t in ("DamageModel", "DamageOverTimeModel"):
        for o in desc(tw, t, target):
            o["f"]["immuneBloonProperties"] = o["f"].get("immuneBloonProperties", 0) & ~bits


def m_knockback(tw, mm, target):
    a = f32(mm.get("percentMultiplierAdded", 0.0))
    for o in tw.descendants({"KnockbackModel"}):
        for k in ("lightMultiplier", "heavyMultiplier", "moabMultiplier"):
            o["f"][k] = f32(fl(o, k) + a)


def m_retarget(tw, mm, target):
    n = mm.get("amount", 0)
    for o in tw.descendants({"RetargetOnContactModel"}):
        o["f"]["distance"] = f32(f32(float(n)) + fl(o, "distance"))
        o["f"]["maxBounces"] = o["f"].get("maxBounces", 0) + n


def m_track_target(tw, mm, target):
    # ProjectileTrackTargetModModel: TrackTargetModels, turnRate x multiplierTurnRate + turnRate.
    # HYPOTHESIS P-MKTRACK (which ones the target names).
    k = f32(mm.get("multiplierTurnRate", 0.0))
    for o in desc(tw, "TrackTargetModel", target):
        v = fl(o, "turnRate")
        o["f"]["turnRate"] = f32(f32(v * k) + v)


def m_ignore_blockers(tw, mm, target):
    t = tw.m.objs[tw.root]
    t["f"]["ignoreBlockers"] = True
    for o in tw.descendants({"ProjectileModel"}):
        o["f"]["ignoreBlockers"] = True
    for o in tw.descendants({"AttackModel"}):
        o["f"]["attackThroughWalls"] = True


def m_dot(tw, mm, target):
    # DamageOverTimeModModel: interval - timeReductionAmount (at least 0.05), damage + increase.
    for o in desc(tw, "DamageOverTimeModel", target):
        i = f32(fl(o, "interval") - f32(mm.get("timeReductionAmount", 0.0)))
        o["f"]["interval"] = i if not i < f32(0.05) else f32(0.05)
        o["f"]["damage"] = f32(f32(float(mm.get("damageIncreaseAmount", 0))) + fl(o, "damage"))


def m_pushback(tw, mm, target):
    k = f32(mm.get("amount", 0.0))
    for o in tw.descendants({"PushBackModel"}):
        v = fl(o, "pushAmount")
        o["f"]["pushAmount"] = f32(f32(v * k) + v)


def m_wind(tw, mm, target):
    for o in tw.descendants({"WindModel"}):
        o["f"]["chance"] = f32(fl(o, "chance") + f32(mm.get("chanceIncrease", 0.0)))


def m_tower_expire(tw, mm, target):
    for o in tw.descendants({"TowerExpireModel"}):
        o["f"]["lifespan"] = f32(fl(o, "lifespan") + f32(mm.get("aditionalTimeBonus", 0.0)))


def m_slow(tw, mm, target):
    mid = mm.get("mutationId") or ""
    for o in tw.descendants({"SlowModel"}):
        if mid and not (o["f"].get("mutationId") or "").startswith(mid):
            continue
        o["f"]["multiplier"] = f32(fl(o, "multiplier") + f32(mm.get("slowModifier", 0.0)))
        v = fl(o, "lifespan")
        o["f"]["lifespan"] = f32(f32(v * f32(mm.get("durationModifier", 0.0))) + v)


def m_freeze(tw, mm, target):
    mid = mm.get("mutationId") or ""
    for o in tw.descendants({"FreezeModel"}):
        if mid and not (o["f"].get("mutationId") or "").startswith(mid):
            continue
        v = fl(o, "lifespan")
        v = f32(f32(v * f32(mm.get("durationModifier", 0.0))) + v)
        o["f"]["lifespan"] = f32(v + f32(mm.get("durationAdd", 0.0)))


def m_slow_tag(tw, mm, target):
    tag = mm.get("tag") or ""
    for o in tw.descendants({"SlowModifierForTagModel"}):
        if not tag or (o["f"].get("tag") or "").startswith(tag):
            o["f"]["slowMultiplier"] = f32(mm.get("slowModifier", 0.0))


def m_increase_range(tw, mm, target):
    for o in tw.descendants({"IncreaseRangeModel"}):
        o["f"]["lifespanFrames"] = o["f"].get("lifespanFrames", 0) + mm.get("additionalFrames", 0) * 60


def m_arc_count(tw, mm, target):
    # EmissionArcCountModModel: each WeaponModel (its name without spaces one of the targets, or no
    # targets): its ArcEmissionModels' count + additionalCount (decoded `SliceSize` is the count).
    ts = targets_of(target)
    for w in tw.descendants({"WeaponModel"}):
        if ts and name_of(w).replace(" ", "") not in ts:
            continue
        for o in Tower(tw.m, w["i"], copy_tree=False).descendants({"ArcEmissionModel"}):
            o["f"]["SliceSize"] = o["f"].get("SliceSize", 0) + mm.get("additionalCount", 0)


def m_dmg_tag(tw, mm, target):
    # DamageModifierForTagModModel: each named ProjectileModel; its first DamageModifierForTagModel
    # with tag bloonTag gets damageMultiplier + multiplierAdditive and damageAddative + addative;
    # otherwise a new DamageModifierForTagModel(projectile name, bloonTag, multiplierAdditive + 1,
    # addative) is added and hasDamageModifiers set.
    m = tw.m
    tag = mm.get("bloonTag") or ""
    ma, add = f32(mm.get("multiplierAdditive", 0.0)), f32(mm.get("addative", 0.0))
    for p in desc(tw, "ProjectileModel", target):
        beh = p["f"].get("behaviors")
        its = m.items(beh)
        first = next((m.objs[i] for i in its if isinstance(i, int) and i in m.objs and m.objs[i]["type"] == "DamageModifierForTagModel"), None)
        if first is not None and (first["f"].get("tag") or "") == tag:
            first["f"]["damageMultiplier"] = f32(fl(first, "damageMultiplier") + ma)
            first["f"]["damageAddative"] = f32(fl(first, "damageAddative") + add)
            continue
        if not (isinstance(beh, dict) and "$ref" in beh and isinstance(m.values.get(beh["$ref"]), dict)):
            continue
        nid = m.new_id()
        m.objs[nid] = {"i": nid, "type": "DamageModifierForTagModel", "f": {
            "_name": "DamageModifierForTagModel_" + name_of(p), "collisionPass": 0, "tag": tag,
            "damageMultiplier": f32(ma + 1.0), "damageAddative": add, "mustIncludeAllTags": False,
            "applyOverMaxDamage": False, "ignoreTag": False}}
        c = m.values[beh["$ref"]]
        c["items"].append(nid)
        c["len"] = len(c["items"])
        p["f"]["hasDamageModifiers"] = True


def append_behavior(m, holder, field, typ, fields):
    """A new model at the end of holder's container field."""
    v = holder["f"].get(field)
    if not (isinstance(v, dict) and "$ref" in v and isinstance(m.values.get(v["$ref"]), dict)):
        return None
    nid = m.new_id()
    m.objs[nid] = {"i": nid, "type": typ, "f": fields}
    c = m.values[v["$ref"]]
    c["items"].append(nid)
    c["len"] = len(c["items"])
    return m.objs[nid]


def m_tower_xp(tw, mm, target):
    # TowerXpModModel @0x1812b37b0: the tower gets a TransformTowerXpModel(multiplier) at the end of
    # its behaviours, named after the mod.
    append_behavior(tw.m, tw.m.objs[tw.root], "behaviors", "TransformTowerXpModel",
                    {"_name": "TransformTowerXpModel_" + (mm.get("_name") or ""), "multiplier": f32(mm.get("multiplier", 0.0))})


def m_tower_sell(tw, mm, target):
    # TowerSellModModel: a TowerSellModel(charges, mutltiplier, additional) at the end.
    append_behavior(tw.m, tw.m.objs[tw.root], "behaviors", "TowerSellModel",
                    {"_name": "TowerSellModel_" + (mm.get("_name") or ""), "charges": mm.get("charges", 0),
                     "multiplier": f32(mm.get("mutltiplier", 0.0)), "additional": f32(mm.get("additional", 0.0))})


def m_hero_cost(tw, mm, target):
    # HeroCostToLevelModModel: named HeroModels, costPerXpToLevel x costMultiplier.
    for o in desc(tw, "HeroModel", target):
        o["f"]["costPerXpToLevel"] = f32(fl(o, "costPerXpToLevel") * f32(mm.get("costMultiplier", 1.0)))


def clone_model(m, oid):
    """Model.Clone: a deep copy of a model tree (objects and containers)."""
    memo = {}

    def cp(x):
        if x in memo:
            return memo[x]
        nid = m.new_id()
        memo[x] = nid
        o = copy.deepcopy(m.objs[x])
        o["i"] = nid
        m.objs[nid] = o
        for k, v in list(o["f"].items()):
            if isinstance(v, dict) and "$ref" in v:
                r = v["$ref"]
                if r in m.objs:
                    o["f"][k] = {"$ref": cp(r)}
                elif isinstance(m.values.get(r), dict) and "items" in m.values[r]:
                    cid = m.new_id()
                    c = copy.deepcopy(m.values[r])
                    m.values[cid] = c
                    c["items"] = [cp(e) if isinstance(e, int) and e in m.objs else e for e in c["items"]]
                    o["f"][k] = {"$ref": cid}
        return nid
    return cp(oid)


def m_reload_prio(tw, mm, target):
    # WeaponReloadPercentageOnTargetPrioModModel @0x1812b9bb0: the tower gets a
    # WeaponReloadPercentageOnTargetPrioModel(multiplier, targetType) at the end of its behaviours.
    append_behavior(tw.m, tw.m.objs[tw.root], "behaviors", "WeaponReloadPercentageOnTargetPrioModel",
                    {"_name": "WeaponReloadPercentageOnTargetPrioModel_" + (mm.get("_name") or ""),
                     "multiplier": f32(mm.get("multiplier", 0.0)), "targetType": mm.get("targetType")})


def m_cash(tw, mm, target):
    # CashModModel: every CashModel, minimum / maximum x percentBonus + itself, bonusMultiplier +
    # bonusMultiplierBuff.
    p = f32(mm.get("percentBonus", 0.0))
    for o in tw.descendants({"CashModel"}):
        for k in ("maximum", "minimum"):
            v = fl(o, k)
            o["f"][k] = f32(f32(v * p) + v)
        o["f"]["bonusMultiplier"] = f32(fl(o, "bonusMultiplier") + f32(mm.get("bonusMultiplierBuff", 0.0)))


def m_cash_per_round(tw, mm, target):
    for o in tw.descendants({"PerRoundCashBonusTowerModel"}):
        o["f"]["cashPerRound"] = f32(fl(o, "cashPerRound") + f32(mm.get("additionalMoney", 0.0)))


def m_ceramic_shock(tw, mm, target):
    # CeramicShockModModel: every ProjectileModel gets the mod's slowModelToUse (as a behaviour,
    # the same model for all; here a clone each) and UpdateCollisionPassList.
    src = tw.m.ref(mm.get("slowModelToUse"))
    if src is None:
        return
    for p in tw.descendants({"ProjectileModel"}):
        v = p["f"].get("behaviors")
        if not (isinstance(v, dict) and "$ref" in v and isinstance(tw.m.values.get(v["$ref"]), dict)):
            continue
        c = tw.m.values[v["$ref"]]
        c["items"].append(clone_model(tw.m, src))
        c["len"] = len(c["items"])


MUTATORS = {
    "WeaponReloadPercentageOnTargetPrioModModel": m_reload_prio,
    "CashModModel": m_cash,
    "CashPerRoundModModel": m_cash_per_round,
    "CeramicShockModModel": m_ceramic_shock,
    "TowerXpModModel": m_tower_xp,
    "TowerSellModModel": m_tower_sell,
    "HeroCostToLevelModModel": m_hero_cost,
    "DamageModifierForTagModModel": m_dmg_tag,
    "WeaponReloadPercentageModModel": m_weapon_reload_pct,
    "WeaponReloadReductionModModel": m_weapon_reload_reduction,
    "ProjectilePierceModModel": m_pierce,
    "AbilityCooldownPercentageModModel": m_ability_cd_pct,
    "AbilityCooldownTimeModModel": m_ability_cd_time,
    "ProjectileRangePercentageModModel": m_proj_range_pct,
    "RangeModModel": m_range,
    "ProjectileSpeedModModel": m_proj_speed,
    "ProjectileRadiusModModel": m_proj_radius,
    "DamageTypeModModel": m_damage_type,
    "KnockbackModModel": m_knockback,
    "RetargetIncreaseModModel": m_retarget,
    "ProjectileTrackTargetModModel": m_track_target,
    "IgnoreBlockersModModel": m_ignore_blockers,
    "DamageOverTimeModModel": m_dot,
    "PushBackIncreaseModModel": m_pushback,
    "WindModModel": m_wind,
    "TowerExpireModModel": m_tower_expire,
    "SlowModifierModModel": m_slow,
    "FreezeModifierModModel": m_freeze,
    "SlowModifierForTagModModel": m_slow_tag,
    "IncreaseRangeModModel": m_increase_range,
    "EmissionArcCountModModel": m_arc_count,
}


LINK_TYPES = set(os.environ.get("MK_LINK_TYPES", "MorphTowerModel").split(","))


def main():
    src, out = sys.argv[1], sys.argv[2]
    oracle = sys.argv[sys.argv.index("--oracle") + 1] if "--oracle" in sys.argv else None
    m = Model(os.path.join(src, "decoded"))
    objs = m.objs
    mods = {name_of(o): o for o in objs.values() if o["type"] == "ModModel"}
    active = {name_of(o) for o in objs.values() if o["type"] == "KnowledgeModel"}
    gm = next(o for o in objs.values() if o["type"] == "GameModel")
    tower_ids = [i for i in m.items(gm["f"]["towers"]) if i in objs]
    missing = collections.Counter()
    def apply(tw):
        for am in m.items(objs[tw.root]["f"].get("mods")):
            a = objs.get(am)
            if not a or a["f"].get("mod") not in active or a["f"]["mod"] not in mods:
                continue
            for mmid in m.items(mods[a["f"]["mod"]]["f"].get("mutatorMods")):
                mm = objs.get(mmid)
                if not mm:
                    continue
                fn = MUTATORS.get(mm["type"])
                if fn is None:
                    missing[mm["type"]] += 1
                    continue
                fn(tw, mm["f"], a["f"].get("target") or "")

    # (Every tower copied from the base model first: a tower model is also inside others.)
    views = [Tower(m, tid) for tid in tower_ids]
    for tw in views:
        apply(tw)
    # HYPOTHESIS P-MKLINK: a tower inside another that is itself a GameModel.towers entry is that
    # entry, modded (oracle: DanD 3's MorphTowerModel tower is modded once; Buccaneer 4-0-0's own
    # plane copy is not).
    if not os.environ.get("MK_NOLINK"):
        listed = set(tower_ids)
        for tw in views:
            inv = {v: k for k, v in tw.memo.items() if isinstance(k, int)}
            for o in tw.descendants_all_holders():
                for k, v in list(o["f"].items()):
                    r = m.ref(v)
                    if r is not None and inv.get(r) in listed and inv.get(r) != tw.root and o["type"] in LINK_TYPES:
                        o["f"][k] = {"$ref": inv[r]}
    print("mutators not ported (uses):", missing.most_common(20))
    if oracle:
        compare(m, Model(oracle), tower_ids)
    os.makedirs(os.path.join(out, "named"), exist_ok=True)
    with open(os.path.join(out, "named", "objects.jsonl"), "w", encoding="utf-8") as f:
        for i in sorted(objs):
            f.write(json.dumps(objs[i]) + "\n")
    json.dump({str(k): v for k, v in m.values.items()}, open(os.path.join(out, "values.json"), "w"))


def compare(a, b, tower_ids):
    """Field differences between the engine's towers and the oracle's, by (type, field)."""
    diff = collections.Counter()
    ex = {}
    gm_b = next(o for o in b.objs.values() if o["type"] == "GameModel")
    b_ids = [i for i in b.items(gm_b["f"]["towers"]) if i in b.objs]
    for ta, tb in zip(tower_ids, b_ids):
        tname = name_of(a.objs[ta])

        def rec(x, y, path, depth):
            if depth > 30:
                return
            ox, oy = a.objs[x], b.objs[y]
            if ox["type"] != oy["type"]:
                diff[("<type>", ox["type"])] += 1
                return
            for k, vx in ox["f"].items():
                if k in ("childDependants",):
                    continue
                vy = oy["f"].get(k)
                rx, ry = a.ref(vx), b.ref(vy)
                if rx is not None and ry is not None:
                    rec(rx, ry, path + "/" + k, depth + 1)
                    continue
                ix, iy = a.items(vx), b.items(vy)
                # (Plain values behind a reference, an empty container or a list of numbers or
                # strings: by value.)
                if isinstance(vx, dict) and isinstance(vy, dict) and "$ref" in vx and "$ref" in vy and not ix and not iy:
                    cx, cy = a.values.get(vx["$ref"]), b.values.get(vy["$ref"])
                    if cx == cy or (isinstance(cx, dict) and isinstance(cy, dict) and cx.get("len") == cy.get("len") == 0):
                        continue
                    if isinstance(cx, dict) and isinstance(cy, dict) and "items" in cx and "items" in cy:
                        if [a.values.get(i, i) for i in cx["items"]] == [b.values.get(i, i) for i in cy["items"]]:
                            continue
                if ix or iy:
                    if len(ix) != len(iy):
                        key = (ox["type"], k + "[len]")
                        diff[key] += 1
                        ex.setdefault(key, (tname, path + "/" + k, len(ix), len(iy)))
                    for n, (ex_, ey) in enumerate(zip(ix, iy)):
                        if isinstance(ex_, int) and isinstance(ey, int) and ex_ in a.objs and ey in b.objs:
                            rec(ex_, ey, f"{path}/{k}[{n}]", depth + 1)
                    continue
                if isinstance(vx, float) and isinstance(vy, float):
                    same = f32(vx) == f32(vy)
                else:
                    same = vx == vy
                if not same:
                    key = (ox["type"], k)
                    diff[key] += 1
                    ex.setdefault(key, (tname, path + "/" + k, vx, vy))
        rec(ta, tb, "", 0)
    print(f"differences from the oracle: {sum(diff.values())}")
    for key, n in diff.most_common(40):
        print(f"  {n:6d} {key[0]}.{key[1]}  e.g. {ex.get(key)}")


if __name__ == "__main__":
    main()
