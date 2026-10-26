"""Decompile IL2CPP methods with Ghidra (headless) to C.

    python -I tools/il2cpp/decompile.py <Type::Method | 0xVA> [...]

Addresses come from the installed build (tools/il2cpp/live_methods.py). `Type` may be a suffix ("Weapons.Weapon::Process"); every overload matches. Output goes to
research/dumps/decomp/<Type>/<Method>@<va>.c. Lines that use the method's `this` (param_1) get
a trailing comment naming the fields at the offsets they touch (from field_offsets.json).

The first run imports GameAssembly.dll into a Ghidra project under tools/external/ghidra_proj
(no auto-analysis; a few minutes) and labels every known method address; later runs reuse it.
"""
import glob
import json
import os
import re
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
GAME = r"C:\Program Files\Epic Games\BloonsTD6\GameAssembly.dll"
EXT = os.path.join(ROOT, "tools", "external")
PROJ = os.path.join(EXT, "ghidra_proj")
OUT = os.path.join(ROOT, "research", "dumps", "decomp")
TMP = os.path.join(EXT, "ghidra_tmp")


def headless():
    found = sorted(glob.glob(os.path.join(EXT, "ghidra", "ghidra_*", "support", "analyzeHeadless.bat")))
    if not found:
        raise SystemExit("Ghidra not found under tools/external/ghidra")
    return found[0]


def methods():
    """(va, type, method) for the installed build (live_methods.py; built on first use)."""
    live = os.path.join(TMP, "methods_live.tsv")
    if not os.path.exists(live):
        subprocess.run([sys.executable, "-I", os.path.join(ROOT, "tools", "il2cpp", "live_methods.py"), live], check=True)
    rows = []
    for line in open(live, encoding="utf-8"):
        va, _img, t, m, *_ = line.rstrip("\n").split("\t")
        rows.append((int(va, 16), t, m))
    return rows


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    rows = methods()
    want = []  # (va, type, method)
    for a in args:
        if a.lower().startswith("0x"):
            va = int(a, 16)
            hit = next((r for r in rows if r[0] == va), (va, "unknown", a))
            want.append(hit)
        else:
            t, m = a.rsplit("::", 1)
            hits = [r for r in rows if (r[1] == t or r[1].endswith("." + t)) and r[2] == m]
            if not hits:
                raise SystemExit(f"no method {a}")
            want += hits
    os.makedirs(TMP, exist_ok=True)
    vas = os.path.join(TMP, "vas.txt")
    open(vas, "w").write("\n".join(f"{va:x}" for va, _, _ in want) + "\n")
    names = os.path.join(TMP, "names.txt")
    if not os.path.exists(names):
        with open(names, "w", encoding="utf-8") as f:
            seen = set()
            for va, t, m in rows:
                if va not in seen:
                    seen.add(va)
                    f.write(f"{va:x}\t{t.split('.')[-1]}::{m}\n")
    raw = os.path.join(TMP, "out")
    os.makedirs(PROJ, exist_ok=True)
    exists = os.path.exists(os.path.join(PROJ, "btd6.gpr"))
    cmd = [headless(), PROJ, "btd6"]
    cmd += (["-process", "GameAssembly.dll"] if exists else ["-import", GAME]) + ["-noanalysis"]
    cmd += ["-scriptPath", os.path.join(ROOT, "tools", "il2cpp", "ghidra"),
            "-postScript", "DecompileVAs.java", vas, names, raw]
    # Ghidra 12 needs JDK 25+: RRBTD6_JDK, else a JetBrains IDE's bundled JBR.
    env = dict(os.environ)
    jdk = os.environ.get("RRBTD6_JDK") or next(iter(sorted(glob.glob(r"C:\Program Files\JetBrains\*\jbr"), reverse=True)), None)
    if jdk:
        env["JAVA_HOME"] = jdk
        env["PATH"] = os.path.join(jdk, "bin") + os.pathsep + env.get("PATH", "")
    r = subprocess.run(cmd, capture_output=True, text=True, env=env, stdin=subprocess.DEVNULL)
    if r.returncode != 0:
        print(r.stdout[-3000:], r.stderr[-3000:])
        raise SystemExit("ghidra failed")
    offsets = json.load(open(os.path.join(ROOT, "research", "dumps", "field_offsets.json")))
    for va, t, m in want:
        src = os.path.join(raw, f"{va:x}.c")
        if not os.path.exists(src):
            print("missing", hex(va), t, m)
            continue
        c = open(src, encoding="utf-8").read()
        fields = {o: n for n, o in offsets.get(t, [])}
        if fields:
            lines = []
            for line in c.splitlines():
                if "param_1" in line:
                    hits = [fields[int(h, 16)] for h in re.findall(r"param_1\s*\+\s*0x([0-9a-f]+)", line) if int(h, 16) in fields]
                    if hits:
                        line += "   // " + ", ".join(hits)
                lines.append(line)
            c = "\n".join(lines) + "\n"
        d = os.path.join(OUT, re.sub(r"[<>:|?*]", "_", t.split(".")[-1]))
        os.makedirs(d, exist_ok=True)
        dst = os.path.join(d, f"{m}@{va:x}.c".replace("<", "_").replace(">", "_").replace("|", "_"))
        open(dst, "w", encoding="utf-8").write(f"// {t}::{m} @ {va:#x}\n" + c)
        print(dst)


if __name__ == "__main__":
    main()
