"""The pure-Python DXBC disassembler (tools/assets/dxbc_disasm.py) against Microsoft's D3DDisassemble.

    python -I tools/tests/test_dxbc_disasm.py --collect "<BTD6 folder>"   (Windows: build the corpus)
    python -I tools/tests/test_dxbc_disasm.py                             (compare)

--collect walks every bundle of the game, takes each shader's D3D11 programs (the blobs
dxbc_glsl.translate reads) and saves every unique one with d3dcompiler_47's disassembly under
research/dumps/dxbc_corpus/ (git-ignored: it's the game's code). The comparison then requires
dxbc_disasm.disassemble to give exactly the same text for every program, so dxbc_glsl works the
same on macOS and Linux, which have no d3dcompiler.
"""
import glob
import hashlib
import os
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "assets"))
CORPUS = ROOT / "research" / "dumps" / "dxbc_corpus"


def collect(game):
    import UnityPy
    import dxbc_glsl
    CORPUS.mkdir(parents=True, exist_ok=True)
    aa = os.path.join(game, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
    bundles = sorted(glob.glob(os.path.join(aa, "*.bundle")))
    seen, shaders = set(), 0
    for i, path in enumerate(bundles):
        if os.path.getsize(path) > 600 << 20:  # the huge reference bundle holds no shaders
            continue
        try:
            env = UnityPy.load(path)
            objs = [o for o in env.objects if o.type.name == "Shader"]
            for o in objs:
                shaders += 1
                for entry in dxbc_glsl.blob_entries(o.read_typetree()):
                    code = dxbc_glsl.dxbc_in(entry)
                    if not code:
                        continue
                    h = hashlib.sha1(code).hexdigest()[:16]
                    if h in seen:
                        continue
                    seen.add(h)
                    (CORPUS / f"{h}.dxbc").write_bytes(code)
                    (CORPUS / f"{h}.txt").write_text(dxbc_glsl.disassemble_d3d(code), encoding="utf-8")
        except Exception as e:  # a bundle UnityPy can't read has nothing we need
            print("skip", os.path.basename(path), type(e).__name__, e, flush=True)
        if i % 50 == 0:
            print(f"{i}/{len(bundles)} bundles, {shaders} shaders, {len(seen)} programs", flush=True)
    print(f"done: {shaders} shaders, {len(seen)} unique programs in {CORPUS}")


def compare():
    import dxbc_disasm
    files = sorted(CORPUS.glob("*.dxbc"))
    if not files:
        raise SystemExit(f"no corpus in {CORPUS}: run --collect on Windows first")
    bad = 0
    for f in files:
        want = f.with_suffix(".txt").read_text(encoding="utf-8")
        try:
            got = dxbc_disasm.disassemble(f.read_bytes())
        except Exception as e:
            got = f"<{type(e).__name__}: {e}>"
        if got != want:
            bad += 1
            if bad <= 3:
                w, g = want.splitlines(), got.splitlines()
                for k in range(max(len(w), len(g))):
                    a = w[k] if k < len(w) else "<end>"
                    b = g[k] if k < len(g) else "<end>"
                    if a != b:
                        print(f"{f.name} line {k + 1}:\n  d3d:  {a}\n  ours: {b}")
                        break
    print(f"{len(files) - bad}/{len(files)} programs identical")
    if bad:
        raise SystemExit(1)


if __name__ == "__main__":
    if sys.argv[1:2] == ["--collect"]:
        collect(sys.argv[2])
    else:
        compare()
