"""Recompress fast-packed animations without changing their format or payload.

Uses bounded-memory streams. Replaces a file atomically only when smaller and
after verifying every decoded byte. Existing level-6/9 streams are skipped.
"""
from concurrent.futures import ThreadPoolExecutor
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import sys
import tempfile
import time
import zlib

sys.path.insert(0, str(Path(__file__).resolve().parent))
from delta_anims import chunks, MAGIC

NAMES = {"anim.bin.z", "anim.bin.dz", "anim_q.bin.z", "anim_q.bin.dz"}


def signature(path):
    stat=path.stat()
    return stat.st_size, stat.st_mtime_ns, stat.st_ino


def repack(path):
    path=Path(path)
    if path.name not in NAMES or path.is_symlink():
        raise ValueError("expected an unlinked packed animation")
    before=signature(path)
    predicted=path.suffix==".dz"
    skip=16 if predicted else 0
    with path.open("rb") as src:
        header=src.read(skip)
        zheader=src.read(2)
    if len(zheader)!=2 or zheader[0]&15!=8 or int.from_bytes(zheader,"big")%31:
        raise ValueError("invalid zlib header")
    if predicted and (len(header)!=16 or header[:8]!=MAGIC):
        raise ValueError("invalid prediction header")
    # The zlib FLEVEL bits distinguish existing fast exports from the stronger
    # streams. New exports already use level 6, so future finishes skip these.
    if zheader[1]>>6>=2:
        return before[0],before[0]
    meta_path=path.with_name("meta.json")
    meta_stamp=signature(meta_path)
    meta=json.loads(meta_path.read_bytes())
    stride=meta["vertex_count"]*(10 if path.name.startswith("anim_q.") else 24)
    count=max(a["offset"]+a["frames"] for a in meta["anims"])
    if stride<=0 or count<=0 or (predicted and struct.unpack("<Q",header[8:])[0]!=stride):
        raise ValueError("invalid animation dimensions")
    expected=stride*count
    pending=None
    try:
        encoder=zlib.compressobj(6)
        digest=hashlib.blake2b();size=0
        with tempfile.NamedTemporaryFile(dir=path.parent,prefix=path.name+".",suffix=".pending",delete=False) as out:
            pending=Path(out.name);out.write(header)
            for data in chunks(path,skip):
                size+=len(data)
                if size>expected:raise ValueError("animation exceeds metadata")
                digest.update(data);out.write(encoder.compress(data))
            out.write(encoder.flush());out.flush();os.fsync(out.fileno())
        if size!=expected:raise ValueError("animation differs from metadata")
        after=pending.stat().st_size
        if after>=before[0]:return before[0],before[0]
        restored=hashlib.blake2b();size=0
        for data in chunks(pending,skip):
            size+=len(data)
            if size>expected:raise ValueError("repacked animation exceeds metadata")
            restored.update(data)
        if size!=expected or restored.digest()!=digest.digest():
            raise ValueError("repacked animation verification failed")
        if signature(path)!=before or signature(meta_path)!=meta_stamp:
            raise RuntimeError("animation export changed while repacking")
        os.replace(pending,path);pending=None
        return before[0],after
    finally:
        if pending is not None:pending.unlink(missing_ok=True)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("models",type=Path)
    parser.add_argument("--workers",type=int,default=2)
    args=parser.parse_args()
    if not 1<=args.workers<=16:parser.error("workers must be between 1 and 16")
    if args.models.is_symlink() or os.path.isjunction(args.models):parser.error("refusing linked model root")
    root=args.models.resolve(strict=True);paths=[]
    for directory,dirs,names in os.walk(root,followlinks=False):
        dirs[:]=[name for name in dirs if not Path(directory,name).is_symlink() and not os.path.isjunction(Path(directory,name))]
        for name in names:
            if name in NAMES:
                path=Path(directory,name)
                if not path.resolve().is_relative_to(root):raise ValueError("animation leaves model root")
                paths.append(path)
    before=after=0;start=time.perf_counter()
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        for i,(b,a) in enumerate(pool.map(repack,sorted(paths)),1):
            before+=b;after+=a
            if i%50==0 or i==len(paths):
                print(f"repacked {i}/{len(paths)}: {before/1e9:.3f} -> {after/1e9:.3f} GB ({time.perf_counter()-start:.1f}s)",flush=True)
    print(f"saved {(before-after)/1e9:.3f} GB; all replacements verified",flush=True)


if __name__=="__main__":main()
