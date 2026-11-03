"""Mecanim generic AnimationClip decoding (streamed Hermite curves, dense samples, constants),
shared by export_animated.py (tower models) and export_particles.py (effect prefabs' Animators).
"""
import struct
import math
from bisect import bisect_right

import numpy as np


class Clip:
    def __init__(self, t):
        self.name = t["m_Name"]
        mc = t["m_MuscleClip"]
        c = mc["m_Clip"]["data"]
        self.start, self.stop = mc["m_StartTime"], mc["m_StopTime"]
        self.loop = bool(mc["m_LoopTime"])
        self.events = [(e["time"], e["functionName"]) for e in t.get("m_Events", [])]
        sc = c["m_StreamedClip"]
        self.n_streamed = sc["curveCount"]
        raw = struct.pack(f"<{len(sc['data'])}I", *sc["data"])
        self.keys = [[] for _ in range(self.n_streamed)]
        p = 0
        while p < len(raw):
            time, n = struct.unpack_from("<fI", raw, p)
            p += 8
            for _ in range(n):
                idx, a, b, cc, d = struct.unpack_from("<I4f", raw, p)
                p += 20
                if idx < self.n_streamed:
                    self.keys[idx].append((time, a, b, cc, d))
        dc = c["m_DenseClip"]
        self.n_dense = dc["m_CurveCount"]
        self.dense = np.array(dc["m_SampleArray"], dtype=np.float64).reshape(-1, self.n_dense) if self.n_dense else None
        self.dense_rate, self.dense_begin = dc["m_SampleRate"], dc["m_BeginTime"]
        self.const = c["m_ConstantClip"]["data"]
        self.bindings = t["m_ClipBindingConstant"]["genericBindings"]
        self.key_times = [tuple(k[0] for k in keys) if all(k[0] == k[0] for k in keys) and all(a[0] <= b[0] for a, b in zip(keys, keys[1:])) else None
                          for keys in self.keys]
        self.pose_layout = []
        ci = 0
        for b in self.bindings:
            n = {1: 3, 2: 4, 3: 3, 4: 3}.get(b["attribute"], 1) if b["typeID"] == 4 else 1
            if b["typeID"] == 4:
                self.pose_layout.append((b["path"], b["attribute"], ci, n))
            ci += n

    def values(self, t):
        out = []
        for keys, times in zip(self.keys, self.key_times):
            v = 0.0
            if times is not None and t == t:
                index = bisect_right(times, t) - 1
                if index >= 0:
                    k = keys[index]
                    dt = t - k[0]
                    v = ((k[1] * dt + k[2]) * dt + k[3]) * dt + k[4]
            else:
                # Preserve the original first-greater-key behavior for nonmonotonic data.
                for k in keys:
                    if k[0] <= t:
                        dt = t - k[0]
                        v = ((k[1] * dt + k[2]) * dt + k[3]) * dt + k[4]
                    else:
                        break
            out.append(v)
        if self.n_dense:
            f = (t - self.dense_begin) * self.dense_rate
            last = len(self.dense) - 1
            i0 = 0 if f <= 0 else last if f >= last else math.floor(f)
            i1 = min(i0 + 1, len(self.dense) - 1)
            u = float(min(1, max(0, f - i0)))
            out.extend((self.dense[i0] * (1 - u) + self.dense[i1] * u).tolist())
        out.extend(self.const)
        return out

    def pose(self, t):
        """{path_hash: {attr: value tuple}}"""
        vals = self.values(t)
        res = {}
        for path, attribute, ci, n in self.pose_layout:
            res.setdefault(path, {})[attribute] = tuple(vals[ci:ci + n])
        return res
