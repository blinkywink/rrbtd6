"""Exact word prediction + Zstandard for quantized animation storage.

The matching client restores the original bytes once on its staging worker.
Never run model exporters concurrently. Original files survive failed validation.
"""
from concurrent.futures import ThreadPoolExecutor
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import time
import zlib

import numpy as np
import zstandard as zstd

sys.path.insert(0,str(Path(__file__).resolve().parent))
from delta_anims import chunks, frames, restored_frames as old_frames

MAGIC=b"RRWORD01"
HEADER=40


def signature(path):
    s=path.stat();return s.st_size,s.st_mtime_ns,s.st_ino


def filter_frame(frame,previous,subtract):
    current=np.frombuffer(frame,dtype=np.uint8).reshape(-1,10)
    prior=np.frombuffer(previous,dtype=np.uint8).reshape(-1,10)
    output=current.copy()
    words=output.view("<u2").reshape(-1,5)
    before=prior.view("<u2").reshape(-1,5)
    if subtract:
        words[:,:3]-=before[:,:3]
        output[:,6:]-=prior[:,6:]
    else:
        words[:,:3]+=before[:,:3]
        output[:,6:]+=prior[:,6:]
    return output.tobytes()


def source_frames(path,stride):
    if path.suffix==".dz":yield from old_frames(path)
    elif path.suffix==".z":yield from frames(chunks(path),stride)
    else:
        with path.open("rb") as f:
            while frame:=f.read(stride):
                if len(frame)!=stride:raise ValueError("partial source frame")
                yield frame


def restored_frames(path):
    path=Path(path)
    with path.open("rb") as f:
        header=f.read(HEADER)
        if len(header)!=HEADER or header[:8]!=MAGIC or zlib.crc32(header[:36])!=struct.unpack("<I",header[36:])[0]:
            raise ValueError("invalid word-animation header")
        stride,expected,order,packed=struct.unpack("<QQIQ",header[8:36])
        if not 0<stride<=64*1024*1024 or stride%10 or expected%stride or order not in (0,1,2) or packed!=path.stat().st_size-HEADER:
            raise ValueError("invalid word-animation dimensions")
        previous=[bytes(stride) for _ in range(max(1,order))]
        seen=0
        # Bounded reads avoid allocating another full decoded animation.
        with zstd.ZstdDecompressor(max_window_size=128*1024*1024).stream_reader(f,closefd=False) as reader:
            def decoded():
                while data:=reader.read(1024*1024):yield data
            for frame in frames(decoded(),stride):
                seen+=len(frame)
                if seen>expected:raise ValueError("word-animation exceeds header length")
                if order==0:
                    frame=(np.frombuffer(frame,dtype=np.uint8)+np.frombuffer(previous[0],dtype=np.uint8)).tobytes();previous[0]=frame
                else:
                    for i in range(order):
                        frame=filter_frame(frame,previous[i],False);previous[i]=frame
                yield frame
        if seen!=expected:raise ValueError("word-animation length mismatch")


def read_bytes(path):return b"".join(restored_frames(path))


def finalize(root,client):
    """Apply native timing decisions only while all candidate inputs are unchanged."""
    root=Path(root).resolve(strict=True);snapshots={}
    for target in root.rglob("anim_q.bin.wz"):
        sources=[target.with_name(name) for name in ("anim_q.bin","anim_q.bin.z","anim_q.bin.dz") if target.with_name(name).is_file()]
        if sources:
            paths=sources+[target,target.with_name("meta.json")]
            if any(path.is_symlink() for path in paths):raise ValueError("linked audit input")
            snapshots[str(target.parent.relative_to(root))]={path:signature(path) for path in paths}
    if not snapshots:return {"accepted":0,"retained_old":0,"saved_bytes":0,"timings":[]}
    report=None
    try:
        with tempfile.NamedTemporaryFile(suffix=".json",delete=False) as f:report=Path(f.name)
        subprocess.run([str(Path(client).resolve(strict=True)),"--audit-animation-codec",str(root),str(report)],check=True)
        decisions=json.loads(report.read_bytes())
        for decision in decisions:decision["directory"]=str(Path(decision["directory"]))
        if {d["directory"] for d in decisions}!=set(snapshots):raise RuntimeError("codec audit inputs changed")
        saved=accepted=rejected=0
        for decision in decisions:
            stamps=snapshots[decision["directory"]]
            if any(signature(p)!=stamp for p,stamp in stamps.items()):raise RuntimeError("codec inputs changed during audit")
            directory=root/decision["directory"];target=directory/"anim_q.bin.wz"
            if not directory.resolve().is_relative_to(root):raise ValueError("audit path leaves model root")
            if decision["accepted"]:
                sources=[p for p in stamps if p.name in ("anim_q.bin","anim_q.bin.z","anim_q.bin.dz")]
                saved+=sum(stamps[p][0] for p in sources)-target.stat().st_size
                for path in sources:path.unlink()
                accepted+=1
            else:
                target.unlink();rejected+=1
        return {"accepted":accepted,"retained_old":rejected,"saved_bytes":saved,"timings":decisions}
    finally:
        if report is not None:report.unlink(missing_ok=True)


def pack(directory,keep_original=False,order=2):
    directory=Path(directory)
    sources=[directory/name for name in ("anim_q.bin","anim_q.bin.z","anim_q.bin.dz") if (directory/name).is_file()]
    if not sources:return 0,0
    if any(p.is_symlink() for p in sources):raise ValueError("refusing linked source")
    stamps={p:signature(p) for p in sources};source=sources[0]
    meta_path=directory/"meta.json";meta_stamp=signature(meta_path)
    meta=json.loads(meta_path.read_bytes())
    if not meta.get("quant"):raise ValueError("word storage requires existing quantized samples")
    stride=meta["vertex_count"]*10;count=max(a["offset"]+a["frames"] for a in meta["anims"])
    if stride==0 and all(stamp[0]==0 for stamp in stamps.values()):return 0,0
    if not 0<stride<=64*1024*1024 or count<1:raise ValueError("invalid source dimensions")
    expected=stride*count;target=directory/"anim_q.bin.wz"
    if target.is_symlink():raise ValueError("refusing linked output")
    target_stamp=signature(target) if target.exists() else None
    # Resume an interrupted candidate pass without recompressing verified work.
    if keep_original and target_stamp is not None and target_stamp[0]<stamps[source][0]:
        try:
            original=hashlib.blake2b();restored=hashlib.blake2b();seen=decoded=0
            for frame in source_frames(source,stride):original.update(frame);seen+=len(frame)
            for frame in restored_frames(target):restored.update(frame);decoded+=len(frame)
            matching=seen==decoded==expected and original.digest()==restored.digest()
        except (ValueError,zstd.ZstdError,zlib.error):matching=False
        if matching:
            if signature(meta_path)!=meta_stamp or signature(target)!=target_stamp or any(signature(p)!=stamp for p,stamp in stamps.items()):
                raise RuntimeError("animation export changed during candidate verification")
            return sum(stamp[0] for stamp in stamps.values()),target_stamp[0]
    pending=None
    try:
        original=hashlib.blake2b();seen=0
        previous=[bytes(stride) for _ in range(max(1,order))]
        with tempfile.NamedTemporaryFile(dir=directory,prefix=target.name+".",suffix=".pending",delete=False) as out:
            pending=Path(out.name);out.write(bytes(HEADER))
            compressor=zstd.ZstdCompressor(level=12,write_checksum=True)
            with compressor.stream_writer(out,size=expected,closefd=False) as writer:
                for frame in source_frames(source,stride):
                    seen+=len(frame)
                    if len(frame)!=stride or seen>expected:raise ValueError("source exceeds metadata")
                    original.update(frame)
                    if order==0:
                        current=frame;frame=(np.frombuffer(frame,dtype=np.uint8)-np.frombuffer(previous[0],dtype=np.uint8)).tobytes();previous[0]=current
                    else:
                        for i in range(order):
                            current=frame;frame=filter_frame(frame,previous[i],True);previous[i]=current
                    writer.write(frame)
            if seen!=expected:raise ValueError("source length differs from metadata")
            packed=out.tell()-HEADER
            header=MAGIC+struct.pack("<QQIQ",stride,expected,order,packed)
            out.seek(0);out.write(header+struct.pack("<I",zlib.crc32(header)));out.flush();os.fsync(out.fileno())
        after=pending.stat().st_size
        before=sum(stamp[0] for stamp in stamps.values())
        if after>=stamps[source][0]:
            # A fresh small export can supersede a larger previous .wz cache.
            # Keep the preferred source, and discard only the stale cache.
            if target_stamp is not None:
                if signature(target)!=target_stamp or signature(meta_path)!=meta_stamp or any(signature(p)!=stamp for p,stamp in stamps.items()):
                    raise RuntimeError("animation export changed during conversion")
                target.unlink()
            return before,before
        restored=hashlib.blake2b();size=0
        for frame in restored_frames(pending):restored.update(frame);size+=len(frame)
        if size!=expected or restored.digest()!=original.digest():raise ValueError("word-animation verification failed")
        if signature(meta_path)!=meta_stamp or any(signature(p)!=stamp for p,stamp in stamps.items()):
            raise RuntimeError("animation export changed during conversion")
        if (signature(target) if target.exists() else None)!=target_stamp:raise RuntimeError("packed output changed during conversion")
        os.replace(pending,target);pending=None
        if not keep_original:
            for path,stamp in stamps.items():
                if signature(path)!=stamp:raise RuntimeError("source changed before removal")
                path.unlink()
        return before,after
    finally:
        if pending is not None:pending.unlink(missing_ok=True)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("models",type=Path);parser.add_argument("--keep-original",action="store_true")
    parser.add_argument("--root",type=Path,help="install directory containing the matching client")
    parser.add_argument("--report",type=Path,help="optional timing/result JSON")
    parser.add_argument("--workers",type=int,default=2);args=parser.parse_args()
    if not 1<=args.workers<=8:parser.error("workers must be between 1 and 8")
    if args.models.is_symlink() or os.path.isjunction(args.models):parser.error("refusing linked model root")
    root=args.models.resolve(strict=True);directories=[]
    install=(args.root or root.parent.parent).resolve(strict=True)
    client=next((install/name for name in ("rrbtd6.exe","BTD6.exe") if (install/name).is_file()),None)
    if not args.keep_original and client is None:parser.error("matching client required for the native acceptance gate")
    for directory,dirs,names in os.walk(root,followlinks=False):
        dirs[:]=[n for n in dirs if not Path(directory,n).is_symlink() and not os.path.isjunction(Path(directory,n))]
        if "meta.json" in names and any(n in names for n in ("anim_q.bin","anim_q.bin.z","anim_q.bin.dz")):
            path=Path(directory)
            if not path.resolve().is_relative_to(root):raise ValueError("source leaves model root")
            directories.append(path)
    before=after=0;start=time.perf_counter()
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        for i,(b,a) in enumerate(pool.map(lambda p:pack(p,True),sorted(directories)),1):
            before+=b;after+=a
            if i%50==0 or i==len(directories):
                print(f"word packed {i}/{len(directories)}: {before/1e9:.3f} -> {after/1e9:.3f} GB ({time.perf_counter()-start:.1f}s)",flush=True)
    print(f"candidates save {(before-after)/1e9:.3f} logical GB before native timing acceptance",flush=True)
    if not args.keep_original:
        result=finalize(root,client)
        if args.report:args.report.write_text(json.dumps(result,indent=2))
        print(f"accepted {result['accepted']}; retained old format for {result['retained_old']}; saved {result['saved_bytes']/1e9:.3f} logical GB",flush=True)


if __name__=="__main__":main()
