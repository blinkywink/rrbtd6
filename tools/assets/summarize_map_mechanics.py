"""Readable overview of every map's mechanics.json: what a map needs from the game, per map.

    python -I tools/assets/summarize_map_mechanics.py [local_extracted/maps] > map-mechanics.md
"""
import collections
import json
import sys
from pathlib import Path

MAPS = Path(sys.argv[1] if len(sys.argv) > 1 else "local_extracted/maps")
CLICK = ("PlayInteractableClip", "PlayInteractableEffect", "PlayInteractableAudio", "PlayAnimationOnClick",
         "PlayEffectOnClick", "RotateObjectOnClick", "PlaySoundOnMapInteractableClick", "InteractableClick",
         "InteractableClickSequence")
ROUTES = ("MoabOnlySplitter", "BloonTagOnlySplitter", "AlternateRoundSplitter", "AlternateRoundMultiPathSplitter",
          "RuleBossBloon", "LeakPoint", "PathSpawner")
PROPS = ("CustomRotationAnimator", "CustomRotationSimple", "CustomMoveAnimator", "CustomScaleAnimator",
         "CustomColorAnimator", "PlayableAnimatorSimple", "ScrollTexture", "PropAnimationController")
BUFFS = ("Buff", "ApplyTowerFreeze")


def short(t):
    return t.rsplit(".", 1)[-1]


def refs(field, by):
    return [short(by[r["comp"]]["type"]) for r in field or [] if r and r.get("comp") in by]


def main():
    rows = []
    for f in sorted(MAPS.glob("*/mechanics.json")):
        m = json.loads(f.read_text(encoding="utf-8"))
        by = {c["id"]: c for c in m["components"]}
        count = collections.Counter(short(c["type"]) for c in m["components"])
        lines = []
        for c in m["components"]:
            s = short(c["type"])
            if s in ("Removeable", "RegenRemovable", "CustomMushroomGrottoRemoveable"):
                a = c["fields"]
                cost = a.get("removealCost", "formula")
                state = "" if a.get("isActive") else " (starts inactive)"
                lines.append(f"- {s} `{m['nodes'][str(c['go'])]['name']}`: ${cost}{state}")
        for c in m["components"]:
            if short(c["type"]) == "MapEvent":
                a = c["fields"]
                actions = collections.Counter(refs(a.get("actions"), by))
                acts = ", ".join(f"{n}×{k}" if k > 1 else n for n, k in actions.items()) or "none"
                once = " once" if a.get("oneTimeEvent") else ""
                off = " inactive" if a.get("inactiveByDefault") else ""
                lines.append(f"- Event `{m['nodes'][str(c['go'])]['name']}`{once}{off}: "
                             f"{' + '.join(refs(a.get('triggers'), by)) or 'no trigger'} -> {acts}")
        extra = {
            "Clickable": sum(count[k] for k in CLICK),
            "Route rules": ", ".join(f"{k}×{count[k]}" for k in ROUTES if count[k]),
            "Prop animation": sum(count[k] for k in PROPS),
            "Area buffs": ", ".join(f"{k}×{n}" for k, n in count.items() if k.endswith(BUFFS)),
            "Special": ", ".join(k for k in count if k.startswith(("Workshop", "TrickyTracks", "DarkDungeons", "CandyFalls", "MapMods"))),
        }
        rows.append((m["key"], lines, extra, count))
    total = collections.Counter()
    for _, _, _, c in rows:
        total.update(c)
    with_gameplay = [r for r in rows if r[1]]
    print(f"# Map mechanics\n\nGenerated from maps/*/mechanics.json. {len(rows)} maps, {len(with_gameplay)} with "
          f"removeables or events; {total['Removeable']} removeables, {total['MapEvent']} events.\n")
    for key, lines, extra, _ in rows:
        print(f"## {key}")
        for k, v in extra.items():
            if v:
                print(f"{k}: {v}  ")
        print("\n".join(lines))
        print()


main()
