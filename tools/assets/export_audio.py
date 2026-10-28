"""Decode every AudioClip of the game into local_extracted/audio/<clip>.flac (read-only): all
sound effects and voice lines (asset_references bundle) and every music track (music_assets_*
bundles). Reports any sound name in local_extracted/game/game.json that has no clip.
FSB5 clips are decoded with UnityPy (fmod_toolkit). Existing files are kept.

Usage: python -I export_audio.py <game install> [out_dir]
"""
import glob
import json
import os
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from compress_audio import encode_bytes

import UnityPy

UnityPy.config.FALLBACK_UNITY_VERSION = "2022.3.0f1"

GAME = sys.argv[1]
OUT = sys.argv[2] if len(sys.argv) > 2 else "local_extracted/audio"
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")

EXTRA = ["UIClick01", "UIClick03", "VictoryCampaign"]
# STAND-IN playlist: the game's track selection logic isn't reversed; these are music_assets bundles.
MUSIC = ["MusicTitle", "MusicUpbeat1A", "MusicUpbeat2A", "MusicUpbeat3A", "MusicCityA", "MusicBTD5JazzA",
         "MusicSailsAgain"]


def wanted():
    g = json.load(open("local_extracted/game/game.json"))
    names = set(EXTRA)

    def walk(o, key=""):
        if isinstance(o, dict):
            for k, v in o.items():
                walk(v, k)
        elif isinstance(o, list):
            for v in o:
                if isinstance(v, str) and (key.endswith("sounds") or key in ("names", "place", "upgrade", "sell")):
                    names.add(v)
                else:
                    walk(v, key)
    walk(g)
    return names


def export(bundle, names=None):
    """Writes the bundle's clips (only `names` when given) that aren't on disk yet."""
    env = UnityPy.load(bundle)
    done = set()
    have = {os.path.splitext(f)[0] for f in os.listdir(OUT)}
    for o in env.objects:
        if o.type.name != "AudioClip":
            continue
        c = o.read()
        if (names is not None and c.m_Name not in names) or c.m_Name in done:
            continue
        done.add(c.m_Name)
        if c.m_Name in have:
            continue
        try:
            samples = c.samples
            if not samples:
                raise ValueError("AudioClip decoded no samples")
            for _, data in samples.items():
                encode_bytes(data, Path(OUT) / (c.m_Name + ".flac"))
                break
        except Exception as e:
            print("decode failed", c.m_Name, type(e).__name__, e, flush=True)
            raise
    return done


def main():
    os.makedirs(OUT, exist_ok=True)
    got = export(glob.glob(os.path.join(AA, "asset_references_assets_all_*.bundle"))[0])
    print("sound clips", len(got), "missing from the game", sorted(wanted() - got - set(MUSIC)))
    for b in sorted(glob.glob(os.path.join(AA, "music_assets_*.bundle"))):
        print(os.path.basename(b).rsplit("_", 1)[0], len(export(b)), flush=True)


main()
