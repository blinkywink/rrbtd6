"""Every AudioClip of the game into local_extracted/audio (read-only): all sound effects and
voice lines (asset_references bundle) and every music track (music_assets_* bundles). Reports
any sound name in local_extracted/game/game.json that has no clip.

The game stores its clips as FSB5 Vorbis. They become <clip>.ogg with their original audio
packets (fsb_vorbis.py: no decoding or re-encoding), 115 MB for all clips instead of 845 MB of
FLAC. A clip that can't be repackaged is decoded with UnityPy (fmod_toolkit) into <clip>.flac.
Existing .ogg/.wav files are kept; an older setup's .flac is replaced by the .ogg.

Usage: python -I export_audio.py <game install> [out_dir]
"""
import glob
import json
import os
import struct
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from compress_audio import encode_bytes
import fsb_vorbis

import UnityPy
from UnityPy.helpers.ResourceReader import get_resource_data

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


def write_ogg(clip, table):
    """<clip>.ogg from the clip's own Vorbis packets; False when it can't be repackaged."""
    r = clip.m_Resource
    data = bytes(clip.m_AudioData) if clip.m_AudioData else bytes(
        get_resource_data(r.m_Source, clip.object_reader.assets_file, r.m_Offset, r.m_Size))
    try:
        banks = fsb_vorbis.fsb5_samples(data)
    except (ValueError, KeyError, struct.error):
        return False
    if len(banks) != 1 or banks[0][3] not in table:
        return False
    rate, channels, samples, crc, packets = banks[0]
    target = Path(OUT) / (clip.m_Name + ".ogg")
    pending = target.with_name(target.name + ".pending")
    pending.write_bytes(fsb_vorbis.ogg_bytes(rate, channels, samples, table[crc], packets))
    os.replace(pending, target)
    # The game's own audio supersedes an older setup's FMOD decode.
    (Path(OUT) / (clip.m_Name + ".flac")).unlink(missing_ok=True)
    return True


def export(bundle, table, names=None):
    """Writes the bundle's clips (only `names` when given) that aren't on disk yet."""
    env = UnityPy.load(bundle)
    done = set()
    have = {os.path.splitext(f)[0] for f in os.listdir(OUT) if f.endswith((".ogg", ".wav"))}
    for o in env.objects:
        if o.type.name != "AudioClip":
            continue
        c = o.read()
        if (names is not None and c.m_Name not in names) or c.m_Name in done:
            continue
        done.add(c.m_Name)
        if c.m_Name in have:
            continue
        if write_ogg(c, table):
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
    # FMOD's Vorbis setup headers, from the game's own UnityPlayer.dll.
    try:
        table = fsb_vorbis.setup_headers(Path(GAME, "UnityPlayer.dll").read_bytes())
    except (OSError, ValueError) as e:
        print("Vorbis setup headers unavailable, decoding clips instead:", e, flush=True)
        table = {}
    got = export(glob.glob(os.path.join(AA, "asset_references_assets_all_*.bundle"))[0], table)
    # Three of Adora's BFB warnings are in the asset library instead (some builds ship it only in Half).
    library = "asset_library_assets_all_*.bundle"
    for b in (glob.glob(os.path.join(AA, library)) or glob.glob(os.path.join(os.path.dirname(AA), "Half", library)))[:1]:
        got |= export(b, table)
    print("sound clips", len(got), "missing from the game", sorted(wanted() - got - set(MUSIC)))
    for b in sorted(glob.glob(os.path.join(AA, "music_assets_*.bundle"))):
        print(os.path.basename(b).rsplit("_", 1)[0], len(export(b, table)), flush=True)


main()
