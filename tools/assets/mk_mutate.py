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
        if types is not None:
            types = set(types)
            for t in list(types):
                types |= SUBCLASSES.get(t, set())
        return self._descendants(types)

    def _descendants_into(self, types):
        """GetDescendants from a nested tower model itself (its root is the walk's start)."""
        return self._descendants(types)

    def _descendants(self, types=None):
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


# GetDescendants<T> finds subclasses too (from names and the oracle: TravelCurvy / TravelStraitSlowdown
# lifespans, DamageOverTimeCustom immunities, AttackAirUnit ranges change with their base's mods).
SUBCLASSES = {
    "TravelStraitModel": {"TravelCurvyModel", "TravelStraitSlowdownModel"},
    "SlowModel": {"SlowForBloonModel", "SlowMinusAbilityDurationModel"},
    "DamageOverTimeModel": {"DamageOverTimeCustomModel", "DamageOverTimeForTagModel"},
    "AttackModel": {"AttackAirUnitModel"},
}


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
    # ProjectileTrackTargetModModel.Mutate (VERIFIED-DECOMP @0x1812acdc0): the ProjectileModels whose
    # name HasTargetName, then every TrackTargetModel among each one's GetDescendants: turnRate x
    # multiplierTurnRate + turnRate (oracle: the Ace Paragon's RedTippedDart / Spectre darts 160 ->
    # 192 under AcceleratedAerodarts, though their TrackTargetModels aren't named after it).
    k = f32(mm.get("multiplierTurnRate", 0.0))
    done = set()
    for p in desc(tw, "ProjectileModel", target):
        for o in subtree(tw.m, p["i"], {"TrackTargetModel"}):
            if o["i"] in done:
                continue
            done.add(o["i"])
            v = fl(o, "turnRate")
            o["f"]["turnRate"] = f32(f32(v * k) + v)


def subtree(m, oid, types):
    """Model.GetDescendants from `oid` (itself included) through childDependants, not into
    TowerModels below it."""
    out, seen = [], set()

    def rec(i):
        if i in seen or i not in m.objs:
            return
        seen.add(i)
        o = m.objs[i]
        if o["type"] in types:
            out.append(o)
        if o["type"] == "TowerModel" and i != oid:
            return
        for e in m.items(o["f"].get("childDependants")):
            if isinstance(e, int):
                rec(e)
    rec(oid)
    return out


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
    # TowerExpireModModel: each CreateTowerModel's tower, its TowerExpireModels + aditionalTimeBonus
    # (into the created tower; the tower's own TowerExpireModels are left: the Sentry entry keeps 25).
    for c in tw.descendants({"CreateTowerModel"}):
        r = tw.m.ref(c["f"].get("tower"))
        if r is None:
            continue
        for o in Tower(tw.m, r, copy_tree=False)._descendants_into({"TowerExpireModel"}):
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


def m_alternate_arc(tw, mm, target):
    # AlternateArcModModel.Mutate (VERIFIED-DECOMP @0x18129d640): each WeaponModel the target string
    # ends with gets AlternateProjectileModel(the mod's name, its projectile, a clone of its
    # ArcEmissionModel with count + increase, interval shotCount, alternateAnimation -1) at the end
    # of its behaviours (4And4: Triple Shot's every 4th throw makes 4; recording dart_030_mk, 4 darts
    # at rt 163, 1862, 1974 ...). The game's all-knowledge GameModel dump doesn't have it (it is put on
    # elsewhere), so the oracle counts 15 differences for it.
    for w in tw.descendants({"WeaponModel"}):
        n = name_of(w)
        if not n or not (target or "").endswith(n):
            continue
        e = tw.m.ref(w["f"].get("emission"))
        e = tw.m.objs.get(e) if isinstance(e, int) else e
        if not e or e["type"] != "ArcEmissionModel":
            continue
        cid = clone_model(tw.m, e["i"])
        c = tw.m.objs[cid]
        c["f"]["SliceSize"] = c["f"].get("SliceSize", 0) + mm.get("increase", 0)
        fields = {"_name": mm.get("_name") or "", "projectile": w["f"].get("projectile"), "emissionModel": {"$ref": cid},
                  "interval": mm.get("shotCount", 0), "alternateAnimation": -1, "priority": 0}
        if append_behavior(tw.m, w, "behaviors", "AlternateProjectileModel", fields) is None:
            nid = tw.m.new_id()
            tw.m.objs[nid] = {"i": nid, "type": "AlternateProjectileModel", "f": fields}
            vid = tw.m.new_id()
            tw.m.values[vid] = {"_container": "array", "len": 1, "items": [nid]}
            w["f"]["behaviors"] = {"$ref": vid}


def m_paint_stripper(tw, mm, target):
    # PaintStripperModModel.Mutate (VERIFIED-DECOMP @0x1812aaef0): ProjectileModels with id
    # "CamoRemoval" or "StripExplosion": every RemoveBloonModifiersModel under them gets a new
    # bloonTagExcludeList without the comma-separated removeTagsFromExcludeList ("Ddt": the Mortar's
    # strip works on DDTs).
    rm = [t for t in (mm.get("removeTagsFromExcludeList") or "").split(",")]
    for p in tw.descendants({"ProjectileModel"}):
        if p["f"].get("id") not in ("CamoRemoval", "StripExplosion"):
            continue
        for o in subtree(tw.m, p["i"], {"RemoveBloonModifiersModel"}):
            v = o["f"].get("bloonTagExcludeList")
            cur = tw.m.values.get(v["$ref"]) if isinstance(v, dict) and "$ref" in v else None
            old = cur.get("items", []) if isinstance(cur, dict) else cur
            if not isinstance(old, list):
                continue
            new = [x for t in rm for x in old if x != t]
            cid = tw.m.new_id()
            if isinstance(cur, dict):
                c = copy.deepcopy(cur)
                c["items"] = new
                c["len"] = len(new)
            else:
                c = new
            tw.m.values[cid] = c
            o["f"]["bloonTagExcludeList"] = {"$ref": cid}


def m_cold_front(tw, mm, target):
    # ColdFrontModModel.Mutate (VERIFIED-DECOMP @0x1812a12c0): each ProjectileModel whose name ends
    # with the target (BallLightningProj) gets a clone of the mod's freeze (FreezeModel, 1.5 s, 4
    # layers) at the end of its behaviours (then UpdateCollisionPassList).
    fid = tw.m.ref(mm.get("freeze"))
    if fid is None:
        return
    for p in tw.descendants({"ProjectileModel"}):
        if not target or not name_of(p).endswith(target):
            continue
        cid = clone_model(tw.m, fid)
        v = p["f"].get("behaviors")
        c = tw.m.values.get(v["$ref"]) if isinstance(v, dict) and "$ref" in v else None
        if isinstance(c, dict) and "items" in c:
            c["items"].append(cid)
            c["len"] = len(c["items"])
        elif isinstance(c, list):
            c.append({"$ref": cid})
        add_child(tw.m, p, cid)


def m_tiny_tornadoes(tw, mm, target):
    # TinyTornadoesModModel.Mutate (VERIFIED-DECOMP @0x1812b22a0): a Druid (baseId) with top path 3+:
    # the projectile of every attack whose name contains "Tornado" gets CreateProjectileOnExpireModel and
    # CreateProjectileOnExhaustFractionModel (fraction 1, durationfraction -1, passOnCollidedWith),
    # both "TinyTornadoes", each with a clone of the mod's projectile (middle path 2+: the
    # "HeartOfOak" copy with a RemoveBloonModifiersModel cleansing regrow) and of its emission, then
    # a clone of emissionBehaviorModel (EmissionRotationOffProjectileDirectionModel -120 / 120) on
    # the projectile; here it goes into each emission's behaviours, where the exporter reads it.
    root = tw.m.objs[tw.root]["f"]
    if root.get("baseId") != "Druid":
        return
    tv = root.get("tiers")
    tiers = tw.m.values.get(tv["$ref"]) if isinstance(tv, dict) and "$ref" in tv else tv
    if isinstance(tiers, dict):
        tiers = tiers.get("items", [])
    tiers = tiers if isinstance(tiers, list) else []
    if len(tiers) < 2 or tiers[0] < 3:
        return
    pid, eid, bid = tw.m.ref(mm.get("projectileModel")), tw.m.ref(mm.get("emissionModel")), tw.m.ref(mm.get("emissionBehaviorModel"))
    if pid is None or eid is None:
        return
    if tiers[1] >= 2:
        pid = clone_model(tw.m, pid)
        po = tw.m.objs[pid]
        po["f"]["_name"] = name_of(po) + "HeartOfOak"
        rid = tw.m.new_id()
        lid, lid2 = tw.m.new_id(), tw.m.new_id()
        tw.m.values[lid], tw.m.values[lid2] = [], []
        tw.m.objs[rid] = {"i": rid, "type": "RemoveBloonModifiersModel", "f": {
            "_name": "RemoveBloonModifiers", "collisionPass": -1, "cleanseRegen": True, "cleanseCamo": False,
            "cleanseLead": False, "cleanseFortified": False, "cleanseOnlyIfDamaged": False,
            "bloonTagExcludeList": {"$ref": lid}, "bloonTagExplicitList": {"$ref": lid2}}}
        append_ids(tw.m, po, "behaviors", rid)
        add_child(tw.m, po, rid)

    def emis():
        c = clone_model(tw.m, eid)
        if bid is not None:
            append_ids(tw.m, tw.m.objs[c], "behaviors", clone_model(tw.m, bid))
        return c
    # (The check is on the AttackModel's name, AttackModel_TornadoAttack_; GetDescendant<ProjectileModel>:
    # its first weapon's projectile.)
    for at in tw.descendants({"AttackModel"}):
        if "Tornado" not in name_of(at):
            continue
        ws = [x for x in tw.m.items(at["f"].get("weapons")) if x in tw.m.objs]
        p = tw.m.ref(tw.m.objs[ws[0]]["f"].get("projectile")) if ws else None
        if p is None:
            continue
        po = tw.m.objs[p]
        for typ, extra in (("CreateProjectileOnExpireModel", {"useRotation": False, "useTargetPosition": False}),
                           ("CreateProjectileOnExhaustFractionModel", {"fraction": 1.0, "durationfraction": -1.0,
                                                                        "canCreateInBetweenRounds": False,
                                                                        "collideOnSubProjectile": False,
                                                                        "passOnCollidedWith": True})):
            nid = tw.m.new_id()
            tw.m.objs[nid] = {"i": nid, "type": typ, "f": {"_name": "TinyTornadoes", "projectile": {"$ref": clone_model(tw.m, pid)},
                                                           "emission": {"$ref": emis()}, **extra}}
            append_ids(tw.m, po, "behaviors", nid)
            add_child(tw.m, po, nid)


def append_ids(m, holder, field, nid):
    """nid at the end of holder's container field (made if missing)."""
    v = holder["f"].get(field)
    c = m.values.get(v["$ref"]) if isinstance(v, dict) and "$ref" in v else None
    if isinstance(c, dict) and "items" in c:
        c["items"].append(nid)
        c["len"] = len(c["items"])
    elif isinstance(c, list):
        c.append(nid)
    else:
        cid = m.new_id()
        m.values[cid] = {"_container": "array", "len": 1, "items": [nid]}
        holder["f"][field] = {"$ref": cid}


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
        add_child(m, p, nid)
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


def add_child(m, holder, nid):
    """Model.AddChildDependant: the new model among the holder's childDependants (GetDescendants
    then finds it)."""
    v = holder["f"].get("childDependants")
    c = m.values.get(v["$ref"]) if isinstance(v, dict) and "$ref" in v else None
    if isinstance(c, dict) and "items" in c:
        c["items"].append(nid)
        c["len"] = len(c["items"])
    else:
        cid = m.new_id()
        m.values[cid] = {"_container": "list", "len": 1, "items": [nid]}
        holder["f"]["childDependants"] = {"$ref": cid}


def m_add_ability(tw, mm, target):
    # AddAbilityToTowerModModel: a clone of abilityModel at the end of the tower's behaviours.
    src = tw.m.ref(mm.get("abilityModel"))
    t = tw.m.objs[tw.root]
    v = t["f"].get("behaviors")
    if src is None or not (isinstance(v, dict) and "$ref" in v and isinstance(tw.m.values.get(v["$ref"]), dict)):
        return
    c = tw.m.values[v["$ref"]]
    nid = clone_model(tw.m, src)
    c["items"].append(nid)
    c["len"] = len(c["items"])
    add_child(tw.m, t, nid)


def add_field(typ, field, key, sign=1.0):
    """A mutator adding mod field `key` to `field` of every `typ`."""
    def fn(tw, mm, target):
        for o in tw.descendants({typ}):
            o["f"][field] = f32(fl(o, field) + f32(sign * mm.get(key, 0.0)))
    return fn


def m_cash_increase(tw, mm, target):
    for o in tw.descendants({"CashIncreaseModel"}):
        o["f"]["increase"] = f32(fl(o, "increase") + f32(mm.get("additionalAmount", 0.0)))
        o["f"]["multiplier"] = f32(fl(o, "multiplier") + f32(mm.get("percentIncrease", 0.0)))


def m_acid_pool(tw, mm, target):
    for o in desc(tw, "AcidPoolModel", target):
        o["f"]["lifespanIfMisses"] = f32(fl(o, "lifespanIfMisses") + f32(mm.get("additionalTime", 0.0)))


def m_dmg_bloon_type(tw, mm, target):
    # DamageModifierForBloonTypeModModel: damageMultiplier x percentBonus + itself, damageAdditive +
    # additiveBonus, then percentMultiplierOverride when not 0.
    for o in tw.descendants({"DamageModifierForBloonTypeModel"}):
        v = fl(o, "damageMultiplier")
        o["f"]["damageMultiplier"] = f32(f32(v * f32(mm.get("percentBonus", 0.0))) + v)
        o["f"]["damageAdditive"] = f32(fl(o, "damageAdditive") + f32(mm.get("additiveBonus", 0.0)))
        if f32(mm.get("percentMultiplierOverride", 0.0)) != 0.0:
            o["f"]["damageMultiplier"] = f32(mm["percentMultiplierOverride"])


def m_lives(tw, mm, target):
    # LivesModModel: maximum / minimum + (int)(x percentBonus).
    p = f32(mm.get("percentBonus", 0.0))
    for o in tw.descendants({"LivesModel"}):
        for k in ("maximum", "minimum"):
            v = fl(o, k)
            o["f"][k] = f32(float(int(f32(v * p))) + v)


def m_imf(tw, mm, target):
    for o in tw.descendants({"ImfLoanModel"}):
        o["f"]["amount"] = f32(fl(o, "amount") + f32(mm.get("additionalLoanAmount", 0.0)))
        o["f"]["incomeRecoveryRate"] = f32(fl(o, "incomeRecoveryRate") - f32(mm.get("interestRateDecreaseAmount", 0.0)))
        c = tw.m.ref(o["f"].get("imfLoanCollection"))
        if c is not None:
            tw.m.objs[c]["f"]["amount"] = o["f"]["amount"]
            tw.m.objs[c]["f"]["collectionRate"] = o["f"]["incomeRecoveryRate"]


def m_ignore_immunity(tw, mm, target):
    bits = mm.get("bloonProperties", 0)
    for t in ("DamageModel", "DamageOverTimeModel"):
        for o in desc(tw, t, target):
            o["f"]["immuneBloonProperties"] = o["f"].get("immuneBloonProperties", 0) & ~bits


def set_flag(typ, field):
    def fn(tw, mm, target):
        for o in tw.descendants({typ}):
            o["f"][field] = True
    return fn


def m_support_global(tw, mm, target):
    # SupportMaxRangeModModel: support models (named with the target in them, or all) go global.
    for o in tw.descendants():
        if o["type"].endswith("SupportModel") and "isGlobal" in o["f"]:
            if not target or target in name_of(o):
                o["f"]["isGlobal"] = True


def m_turbo(tw, mm, target):
    for o in tw.descendants({"TurboModel"}):
        o["f"]["lifespan"] = f32(fl(o, "lifespan") + f32(mm.get("amount", 0.0)))



def m_activate_attack(tw, mm, target):
    # ActivateAttackModModel: under the named abilities, the timed parts + additionalTime.
    # HYPOTHESIS P-MKACTIVATE (types from the oracle: Alchemist x-4-x).
    a = f32(mm.get("additionalTime", 0.0))
    for o in tw.descendants({"ActivateAttackModel", "SwitchDisplayModel", "MorphTowerModel", "CreateEffectOnAbilityEndModel"}):
        if "lifespan" in o["f"]:
            o["f"]["lifespan"] = f32(fl(o, "lifespan") + a)


def m_bank_deposits(tw, mm, target):
    # BankDepositsModModel: towerSelectionMenuThemeId, then a BankDepositsModel at the end.
    t = tw.m.objs[tw.root]
    t["f"]["towerSelectionMenuThemeId"] = mm.get("towerSelectionMenuThemeId")
    append_behavior(tw.m, t, "behaviors", "BankDepositsModel",
                    {"_name": "BankDepositsModel_" + (mm.get("_name") or ""), "depositPercent": f32(mm.get("depositPercent", 0.0)),
                     "displayPath": mm.get("displayPath"), "displayLifetime": f32(mm.get("displayLifetime", 0.0))})


def m_healthy_bananas(tw, mm, target):
    # HealthyBananasModModel: a Banana Farm with path 3 tier 3 (marketplaceLives) or 4
    # (centralMarketLives): its BonusLivesPerRoundModel + that, or a new one.
    t = tw.m.objs[tw.root]
    if t["f"].get("baseId") != "BananaFarm":
        return
    tr = t["f"].get("tiers")
    tv = tw.m.values.get(tr["$ref"]) if isinstance(tr, dict) and "$ref" in tr else None
    if not isinstance(tv, list) or len(tv) < 3:
        return
    n = mm.get("centralMarketLives", 0) if tv[2] == 4 else mm.get("marketplaceLives", 0) if tv[2] == 3 else 0
    if n <= 0:
        return
    for i in tw.m.items(t["f"].get("behaviors")):
        o = tw.m.objs.get(i)
        if o and o["type"] == "BonusLivesPerRoundModel":
            o["f"]["amount"] = o["f"].get("amount", 0) + n
            return
    append_behavior(tw.m, t, "behaviors", "BonusLivesPerRoundModel",
                    {"_name": "BonusLivesPerRoundModel_HealthyBananas", "amount": n,
                     "lifespan": f32(mm.get("displayLifespan", 0.0)), "assetId": mm.get("displayPath")})


def m_wingmonkey(tw, mm, target):
    # WingmonkeyModModel @0x1812ba390: attacks with a CirclePatternModel (HYPOTHESIS P-MKWING: the
    # oracle's 65 Ace attacks) get wingmonkeyPatternModel as a behaviour and child; then
    # TowerModel.UpdateTargetProviders (adds the "Wingmonkey" target type; not exported).
    src = tw.m.ref(mm.get("wingmonkeyPatternModel"))
    if src is None:
        return
    for a in tw.descendants({"AttackModel"}):
        its = tw.m.items(a["f"].get("behaviors"))
        if not any(tw.m.objs.get(i, {}).get("type") == "CirclePatternModel" for i in its):
            continue
        v = a["f"]["behaviors"]
        c = tw.m.values[v["$ref"]]
        nid = clone_model(tw.m, src)
        c["items"].append(nid)
        c["len"] = len(c["items"])
        add_child(tw.m, a, nid)


def m_pineapples(tw, mm, target):
    # TargetedPineapplesModModel: WeaponModels whose name ends with the target get a clone of
    # checkAirUnitOverTrackScript among their behaviours (unless they have one).
    src = tw.m.ref(mm.get("checkAirUnitOverTrackScript"))
    if src is None:
        return
    src_t = tw.m.objs[src]["type"]
    for w in tw.descendants({"WeaponModel"}):
        if target and not name_of(w).endswith(target):
            continue
        v = w["f"].get("behaviors")
        if isinstance(v, dict) and "$ref" in v and isinstance(tw.m.values.get(v["$ref"]), dict):
            c = tw.m.values[v["$ref"]]
        else:
            cid = tw.m.new_id()
            c = tw.m.values[cid] = {"_container": "array", "len": 0, "items": []}
            w["f"]["behaviors"] = {"$ref": cid}
        if any(tw.m.objs.get(i, {}).get("type") == src_t for i in c["items"]):
            continue
        c["items"].append(clone_model(tw.m, src))
        c["len"] = len(c["items"])


def m_grant_camo(tw, mm, target):
    # GrantCamoDetectForAbilityModModel: AbilityModels whose displayName is in the target string get a
    # DetectCamoOnAbilityModel(lifespan frames x 1/60, mutatorId): frames (int)(lifespan x 60), or with
    # useAbilityDuration the ability's TurboModel lifespanFrames.
    for ab in tw.descendants({"AbilityModel"}):
        dn = ab["f"].get("displayName") or ""
        if not dn or dn not in (target or ""):
            continue
        frames = int(f32(f32(mm.get("lifespan", 0.0)) * 60.0))
        life = None
        if mm.get("useAbilityDuration"):
            for i in tw.m.items(ab["f"].get("behaviors")):
                o = tw.m.objs.get(i)
                if o and o["type"] == "TurboModel":
                    frames = int(f32(fl(o, "lifespan") * 60.0))
                elif o and o["type"] == "CreateEffectOnAbilityModel" and life is None:
                    # HYPOTHESIS P-MKCAMOLIFE: without a TurboModel, the ability effect's lifespan
                    # (oracle: Perma Charge 20 exactly, its effect 15 -> 20).
                    e = tw.m.ref(o["f"].get("effectModel"))
                    if e is not None:
                        life = fl(tw.m.objs[e], "lifespan")
            if any(tw.m.objs.get(i, {}).get("type") == "TurboModel" for i in tw.m.items(ab["f"].get("behaviors"))):
                life = None
        nid = tw.m.new_id()
        tw.m.objs[nid] = {"i": nid, "type": "DetectCamoOnAbilityModel", "f": {
            "_name": "DetectCamoOnAbilityModel_GrantCamoDetectForTurboMod",
            "lifespan": life if life is not None else f32(frames * f32(1.0 / 60.0)),
            "mutatorId": mm.get("mutatorId")}}
        v = ab["f"].get("behaviors")
        c = tw.m.values[v["$ref"]]
        c["items"].append(nid)
        c["len"] = len(c["items"])
        add_child(tw.m, ab, nid)


def m_effect_on_ability(tw, mm, target):
    # CreateEffectOnAbilityModModel: CreateEffectOnAbilityModels (named the target, or all), their
    # effectModel's lifespan + additionalLifespan.
    for o in tw.descendants({"CreateEffectOnAbilityModel"}):
        if target and name_of(o) != target:
            continue
        e = tw.m.ref(o["f"].get("effectModel"))
        if e is not None:
            tw.m.objs[e]["f"]["lifespan"] = f32(fl(tw.m.objs[e], "lifespan") + f32(mm.get("additionalLifespan", 0.0)))


def m_discount_zone(tw, mm, target):
    # DiscountZoneModModel: DiscountZoneModels (their name containing specificScriptId, when set):
    # discountMultiplier + additionalMultiplier.
    sid = mm.get("specificScriptId") or ""
    for o in tw.descendants({"DiscountZoneModel"}):
        if sid and sid not in name_of(o):
            continue
        o["f"]["discountMultiplier"] = f32(fl(o, "discountMultiplier") + f32(mm.get("additionalMultiplier", 0.0)))


def m_recurring_rangs(tw, mm, target):
    # RecurringRangsModModel: a tower whose tiers are within all three [pathNTierMin, pathNTierMax]
    # ranges: its FollowPathModel (the first one found) takes newPath / newEasePath. (Its three
    # mutators overlap at the edges: the later one wins.)
    t = tw.m.objs[tw.root]
    tr = t["f"].get("tiers")
    tv = tw.m.values.get(tr["$ref"]) if isinstance(tr, dict) and "$ref" in tr else None
    if not isinstance(tv, list) or len(tv) < 3:
        return
    for k in range(3):
        if not (mm.get(f"path{k + 1}TierMin", 0) <= tv[k] <= mm.get(f"path{k + 1}TierMax", 5)):
            return
    fp = tw.descendants({"FollowPathModel"})
    if fp:
        if mm.get("newPath"):
            fp[0]["f"]["path"] = mm["newPath"]
        if mm.get("newEasePath"):
            fp[0]["f"]["easePath"] = mm["newEasePath"]


MUTATORS = {
    "RecurringRangsModModel": m_recurring_rangs,
    "TargetedPineapplesModModel": m_pineapples,
    "GrantCamoDetectForAbilityModModel": m_grant_camo,
    "AlternateArcModModel": m_alternate_arc,
    "PaintStripperModModel": m_paint_stripper,
    "ColdFrontModModel": m_cold_front,
    "TinyTornadoesModModel": m_tiny_tornadoes,
    "CreateEffectOnAbilityModModel": m_effect_on_ability,
    "WingmonkeyModModel": m_wingmonkey,
    "BankDepositsModModel": m_bank_deposits,
    "HealthyBananasModModel": m_healthy_bananas,
    "DiscountZoneModModel": m_discount_zone,
    "CashIncreaseModModel": m_cash_increase,
    "AcidPoolModModel": m_acid_pool,
    "EatBloonModModel": add_field("EatBloonModel", "rbeCapacity", "additionalTrapAmount"),
    "DamageModifierForBloonTypeModModel": m_dmg_bloon_type,
    "BankModModel": add_field("BankModel", "capacity", "additionalCapacity"),
    "LivesModModel": m_lives,
    "ImfLoanModModel": m_imf,
    "CallToArmsModModel": add_field("CallToArmsModel", "lifespan", "additionalTime"),
    "IgnoreImmunityForBloonTypeModModel": m_ignore_immunity,
    "DoorGunnerModModel": set_flag("RedeployModel", "isDoorGunnerActive"),
    "TCBOOModModel": set_flag("MonkeyTempleModel", "checkForThereCanOnlyBeOne"),
    "SupportMaxRangeModModel": m_support_global,
    "TurboDurationIncreaseModModel": m_turbo,
    "ActivateAttackModModel": m_activate_attack,
    "AddAbilityToTowerModModel": m_add_ability,
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


# Mutators run after the tower's others (HYPOTHESIS P-MKLATE: Boomerang 0-4-0's camo detection takes
# the Turbo's 15 s from LongTurbo, listed after BionicAugmentation; appended behaviours otherwise
# follow the tower's mods list order).
DEFERRED = {"GrantCamoDetectForAbilityModModel"}
# Mutators on the GameModel / simulation (exported as mk_sim, run by the sim).
GAME_LEVEL = {"SimTowerDiscountModModel", "ComeOnEverybodyModModel", "FreeTowerModModel", "StartingCashModModel",
              "CrossTheStreamsModModel", "MonkeyMoneyModModel", "StartingShieldModModel", "ParagonOfPowerModModel",
              "HeroXPBonusModModel", "PreGamePrepModModel"}


def main():
    src, out = sys.argv[1], sys.argv[2]
    oracle = sys.argv[sys.argv.index("--oracle") + 1] if "--oracle" in sys.argv else None
    oracle_dump = sys.argv[sys.argv.index("--dump") + 1] if "--dump" in sys.argv else None
    m = Model(os.path.join(src, "decoded"))
    objs = m.objs
    mods = {name_of(o): o for o in objs.values() if o["type"] == "ModModel"}
    active = {name_of(o) for o in objs.values() if o["type"] == "KnowledgeModel"}
    # --active FILE: the knowledge items to apply (one name per line, or a JSON list); all otherwise.
    if "--active" in sys.argv:
        txt = open(sys.argv[sys.argv.index("--active") + 1], encoding="utf-8").read().strip()
        names = json.loads(txt) if txt.startswith("[") else [x.strip() for x in txt.splitlines() if x.strip()]
        active &= set(names)
    gm = next(o for o in objs.values() if o["type"] == "GameModel")
    tower_ids = [i for i in m.items(gm["f"]["towers"]) if i in objs]
    missing = collections.Counter()

    def apply(tw):
        ams = [am for am in m.items(objs[tw.root]["f"].get("mods")) if am in objs]
        deferred = []
        for am in ams:
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
                if mm["type"] in DEFERRED:
                    deferred.append((fn, mm["f"], a["f"].get("target") or ""))
                    continue
                fn(tw, mm["f"], a["f"].get("target") or "")
        for fn, f, tg in deferred:
            fn(tw, f, tg)

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
                    if r is not None and inv.get(r) in listed and inv.get(r) != tw.root and o["type"] in LINK_TYPES and k == "towerModel":
                        o["f"][k] = {"$ref": inv[r]}
    # BloonModels' own mods lists (BigBloonSabotage): BloonHealthModModel (Towers.Mods,
    # VERIFIED-DECOMP @0x181282400): a bloon that isn't a boss loses (int)(maxHealth x
    # percentageHealthReduced), at least 1 left (MOAB 200 -> 180: recording supermonkey_000_mk).
    for bid in [i for i in m.items(gm["f"].get("bloons")) if i in objs]:
        b = objs[bid]
        for am in m.items(b["f"].get("mods")):
            a = objs.get(am)
            if not a or a["f"].get("mod") not in active or a["f"]["mod"] not in mods:
                continue
            for mmid in m.items(mods[a["f"]["mod"]]["f"].get("mutatorMods")):
                mm = objs.get(mmid)
                if mm and mm["type"] == "BloonHealthModModel" and not b["f"].get("isBoss"):
                    h = b["f"].get("maxHealth", 0)
                    h = h - int(f32(float(h) * f32(mm["f"].get("multiplier", mm["f"].get("percentageHealthReduced", 0.0)))))
                    b["f"]["maxHealth"] = max(h, 1)
    # Game-level knowledge (mods no model's mods list names: ComeOnEverybody, PreGamePrep, MoreCash,
    # ManaShield, free towers, first-buy discounts...): their mutators' fields go to
    # GameModel.mkSim for the sim (build_game_data -> game.json mk_sim).
    listed = set()
    for o in objs.values():
        if o["type"] in ("TowerModel", "BloonModel", "PowerModel", "PowerDetailsModel", "PowerProModel"):
            for a in m.items(o["f"].get("mods")):
                if a in objs:
                    listed.add(objs[a]["f"].get("mod"))
    sim = []
    for k in m.items(gm["f"].get("allKnowledge")):
        n = name_of(objs[k]) if k in objs else None
        if n not in active or n not in mods:
            continue
        for mmid in m.items(mods[n]["f"].get("mutatorMods")):
            mm = objs.get(mmid)
            # (Game-level mutator types, whoever lists the mod: FlatPackBuildings is a tower sell
            # mod and a first-buy discount.)
            if mm and (n not in listed or mm["type"] in GAME_LEVEL):
                sim.append({"type": mm["type"], "knowledge": n,
                            **{f: v for f, v in mm["f"].items() if f not in ("childDependants", "runtimeTypeIndex", "isTower")}})
    gm["f"]["mkSim"] = sim
    # UpgradeCostModModel (VERIFIED-DECOMP @0x181133ae0) runs on GameModel.upgrades: the one named
    # upgradeName costs cost - (int)(cost x percentageReduced) - amountReduced. (Each active mod in
    # allKnowledge order; HYPOTHESIS P-MKUPORDER.)
    ups = {name_of(objs[i]): objs[i] for i in m.items(gm["f"].get("upgrades")) if i in objs}
    up_changes = {}
    for k in m.items(gm["f"].get("allKnowledge")):
        mo = mods.get(name_of(objs[k])) if k in objs and name_of(objs[k]) in active else None
        for mmid in m.items(mo["f"].get("mutatorMods")) if mo else []:
            mm = objs.get(mmid)
            if not mm or mm["type"] != "UpgradeCostModModel":
                continue
            u = ups.get(mm["f"].get("upgradeName"))
            if u is None:
                continue
            # (After the difficulty's scaling: the game's 7020 -> 6420 for Artillery Battery's 600 off
            # on Hard; so kept as steps for the cost code, not folded into the base cost.)
            u["f"].setdefault("mkCost", []).append([f32(mm["f"].get("percentageReduced", 0.0)), mm["f"].get("amountReduced", 0)])
            up_changes[name_of(u)] = u["f"]["mkCost"]
    if oracle_dump:
        # (All-knowledge against no-knowledge dump: the difficulty's cost scaling is in both.)
        def costs(path):
            out = {}
            for line in open(path, encoding="utf-8"):
                p = line.rstrip("\n").split("\t")
                if p[0] == "MD" and p[1] == "#upgrades" and p[2].endswith("/cost"):
                    out[p[2].split("/")[1]] = (int(float(p[3])), int(float(p[4])))
            return out
        a_c, n_c = costs(oracle_dump), costs(oracle_dump.replace("all.mkdump", "none.mkdump"))
        changed = {n: (n_c[n][1] if n in n_c else v[0], v[1]) for n, v in a_c.items() if (n_c[n][1] if n in n_c else v[0]) != v[1]}
        print(f"upgrade costs: {len(changed)} changed by knowledge in the game, {len(up_changes)} by the engine;",
              "game:", sorted(changed.items())[:8], "engine:", sorted(up_changes.items())[:8])
    print("mutators not ported (uses):", missing.most_common(80))
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
                # (The oracle builds added models from the dump: a field it lacks, or an enum by
                # name, is unknown there.)
                if vy is None and vx is not None and k not in oy["f"]:
                    continue
                if isinstance(vy, str) and isinstance(vx, (int, float)) and not isinstance(vx, bool):
                    continue
                if isinstance(vx, float) and isinstance(vy, float):
                    same = f32(vx) == f32(vy) or (vx != vx and vy != vy)
                else:
                    same = vx == vy
                if not same:
                    key = (ox["type"], k)
                    diff[key] += 1
                    ex.setdefault(key, (tname, path + "/" + k, vx, vy))
        rec(ta, tb, "", 0)
    print(f"differences from the oracle: {sum(diff.values())}")
    for key, n in diff.most_common(int(os.environ.get("MK_TOP", "40"))):
        print(f"  {n:6d} {key[0]}.{key[1]}  e.g. {ex.get(key)}")


if __name__ == "__main__":
    main()
