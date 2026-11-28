"""Fill website/progress.json's sections from the recordings and the last regression run.

    python website/count_progress.py [regression.txt]

regression.txt (default website/regression_last.txt) is tools/rec/compare.sh's summary lines
("== <set>: <matching>/<total> match"). Each set's recordings are counted in
research/dumps/recordings (pending/<set>/*.rec; dart_* and boomerang_* at the top level), skipping
effect captures as compare.sh does. Recordings in the *_open, hero_new and hero_open folders are
recorded but not matching yet. Only names and counts end up on the site, never recording data.
Then run python website/build_progress.py.
"""
import datetime
import glob
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REC = os.path.join(HERE, "..", "research", "dumps", "recordings")
SKIP = re.compile(r"_fx|fxp|rot3d|hand_lvl|menu_")

TOWERS = [("Dart Monkey", ["dart"]), ("Boomerang Monkey", ["boomerang"]), ("Bomb Shooter", ["bomb"]),
          ("Tack Shooter", ["tack"]), ("Ice Monkey", ["ice"]), ("Glue Gunner", ["glue", "glue_open"]),
          ("Desperado", ["desperado"]), ("Sniper Monkey", ["sniper"]), ("Monkey Sub", ["sub"]),
          ("Monkey Buccaneer", ["bucc"]), ("Monkey Ace", ["ace"]), ("Heli Pilot", ["heli"]),
          ("Mortar Monkey", ["mortar"]), ("Dartling Gunner", ["dartling"]), ("Wizard Monkey", ["wizard"]),
          ("Super Monkey", ["super"]), ("Ninja Monkey", ["ninja"]), ("Alchemist", ["alch"]), ("Druid", ["druid"]),
          ("Mermonkey", ["mermonkey"]), ("Banana Farm", ["farm", "farm_std"]), ("Spike Factory", ["spike"]),
          ("Monkey Village", ["village", "village_std"]), ("Engineer Monkey", ["engineer", "engineer_open"]),
          ("Beast Handler", ["beast", "merge"]), ("Skywarden", ["sky", "sky_open"])]
HEROES = [("Quincy", "quincy"), ("Gwendolin", "gwen"), ("Striker Jones", "jones"), ("Obyn Greenfoot", "obyn"),
          ("Rosalia", "rosalia"), ("Captain Churchill", "churchill"), ("Benjamin", "benjamin"), ("Pat Fusty", "pat"),
          ("Ezili", "ezili"), ("Adora", "adora"), ("Etienne", "etienne"), ("Sauda", "sauda"),
          ("Admiral Brickell", "brickell"), ("Psi", "psi"), ("Geraldo", "geraldo"), ("Corvus", "corvus"),
          ("Silas", "silas"), ("DanDMonke", "dand")]
OTHER = [("Maps", [("Map spot checks", ["maps"]), ("Towers on many maps", ["maptowers"]), ("Logs (water map)", ["logs"])]),
         ("Monkey Knowledge", [("All knowledge on", ["mk"]), ("Upgraded towers", ["mk_up"]), ("Tier 4 towers", ["mk_t4"]), ("Tier 5 towers", ["mk_t5"]), ("Abilities", ["mk_ab"]), ("Combos", ["mk_combo"]), ("Heroes", ["mk_hero"])]),
         ("Paragons & combos", [("Paragons", ["paragon"]), ("Tower combos", ["combo", "combo_open"]), ("MOAB-class bloons", ["moab"])]),
         ("Powers", [("Powers", ["powers"])])]


def files(set_name):
    if set_name in ("dart", "boomerang"):
        fs = glob.glob(os.path.join(REC, f"{set_name}_*.rec"))
    else:
        fs = glob.glob(os.path.join(REC, "pending", set_name, "*.rec"))
    return [f for f in fs if not SKIP.search(os.path.basename(f))]


def main():
    reg_path = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "regression_last.txt")
    reg = {}
    for line in open(reg_path, encoding="utf-8"):
        m = re.match(r"== (\S+): (\d+)/(\d+) match", line.strip())
        if m:
            reg[m.group(1)] = int(m.group(2))

    def count(sets):
        done = total = 0
        for s in sets:
            n = len(files(s))
            total += n
            if not s.endswith("_open"):
                done += min(reg.get(s, 0), n)
        return done, total

    open_files = [os.path.basename(f) for d in ("hero_new", "hero_open") for f in files(d)]
    sections = [["Towers", [[name, *count(sets)] for name, sets in TOWERS]]]
    heroes = []
    for name, key in HEROES:
        d, t = count([key])
        extra = sum(1 for f in open_files if f.startswith(key + "_"))
        heroes.append([name, d, t + extra])
    sections.append(["Heroes", heroes])
    for title, rows in OTHER:
        sections.append([title, [[name, *count(sets)] for name, sets in rows]])
    path = os.path.join(HERE, "progress.json")
    data = json.load(open(path, encoding="utf-8"))
    data["sections"] = sections
    data.pop("towers", None)
    data["updated"] = datetime.date.today().isoformat()
    data["_how"] = ("python website/count_progress.py (recording counts + website/regression_last.txt), then "
                    "python website/build_progress.py. Sections: recorded scenarios that match the real game frame by "
                    "frame / all recorded. Areas: rough estimates out of 10.")
    json.dump(data, open(path, "w", encoding="utf-8"), indent=2)
    for title, rows in sections:
        print(title, sum(r[1] for r in rows), "/", sum(r[2] for r in rows))


if __name__ == "__main__":
    main()
