"""Build game_mk.json for a set of Monkey Knowledge items (the clone's owned ones).

    python -I tools/assets/build_mk.py <local_extracted> <items.json>

items.json is a JSON list of knowledge names. Runs mk_mutate.py on <root>/decoded with those items
active (-> <root>/decoded_mk_engine), then build_game_data.py with RRBTD6_DECODED /
RRBTD6_GAME_JSON (-> <root>/game/game_mk.json), and writes <root>/game/game_mk.key: the sorted item
names, one per line, which the clone compares with the profile's before using game_mk.json. Takes
about 20 seconds; the clone plays on game.json meanwhile. Output is proprietary game data: keep it
local.
"""
import json
import os
import struct
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def main():
    root, items = sys.argv[1], sys.argv[2]
    names = sorted(set(json.load(open(items, encoding="utf-8"))))
    out = os.path.join(root, "decoded_mk_engine")
    py = sys.executable
    subprocess.run([py, "-I", os.path.join(HERE, "mk_mutate.py"), root, out, "--active", items], check=True,
                   stdout=subprocess.DEVNULL)
    tail = open(os.path.join(root, "game_model", "sim_model.bytes"), "rb").read()[-4:]
    root_idx = struct.unpack("<i", tail)[0]
    env = dict(os.environ, RRBTD6_DECODED=out, RRBTD6_GAME_JSON="game_mk.json.tmp")
    subprocess.run([py, "-I", os.path.join(HERE, "build_game_data.py"), root, "tutorial", str(root_idx)], check=True,
                   env=env, stdout=subprocess.DEVNULL)
    game = os.path.join(root, "game")
    os.replace(os.path.join(game, "game_mk.json.tmp"), os.path.join(game, "game_mk.json"))
    with open(os.path.join(game, "game_mk.key"), "w", encoding="utf-8") as f:
        f.write("\n".join(names) + "\n")
    print(f"game_mk.json for {len(names)} knowledge items")


if __name__ == "__main__":
    main()
