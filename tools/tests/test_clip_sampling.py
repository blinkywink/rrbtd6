"""Exact clip sampling against the original linear-scan implementation; synthetic data only."""
from pathlib import Path
import struct
import sys
import unittest
import numpy as np
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"assets"))
from mecanim_clip import Clip

class LegacyClip(Clip):
    def values(self, t):
        out = []
        for keys in self.keys:
            v = 0.0
            for k in keys:
                if k[0] <= t:
                    dt = t - k[0]
                    v = ((k[1] * dt + k[2]) * dt + k[3]) * dt + k[4]
                else:
                    break
            out.append(v)
        if self.n_dense:
            f = (t - self.dense_begin) * self.dense_rate
            i0 = int(np.clip(np.floor(f), 0, len(self.dense) - 1))
            i1 = min(i0 + 1, len(self.dense) - 1)
            u = float(np.clip(f - i0, 0, 1))
            out.extend((self.dense[i0] * (1 - u) + self.dense[i1] * u).tolist())
        out.extend(self.const)
        return out
    def pose(self, t):
        """{path_hash: {attr: value tuple}}"""
        vals = self.values(t)
        res, ci = ({}, 0)
        for b in self.bindings:
            n = {1: 3, 2: 4, 3: 3, 4: 3}.get(b['attribute'], 1) if b['typeID'] == 4 else 1
            if b['typeID'] == 4:
                res.setdefault(b['path'], {})[b['attribute']] = tuple(vals[ci:ci + n])
            ci += n
        return res


def fixture(times, dense=True):
    data=bytearray()
    for i,t in enumerate(times):
        data.extend(struct.pack("<fI",t,3))
        for c in range(3):data.extend(struct.pack("<I4f",c,.01*(c+1),-.03*i,.25,1.+i+c))
    return {"m_Name":"fixture","m_MuscleClip":{"m_StartTime":0.,"m_StopTime":2.,"m_LoopTime":True,
        "m_Clip":{"data":{"m_StreamedClip":{"curveCount":3,"data":list(struct.unpack(f"<{len(data)//4}I",data))},
            "m_DenseClip":{"m_CurveCount":4 if dense else 0,"m_SampleArray":[i*.13 for i in range(20)] if dense else [],"m_SampleRate":12.,"m_BeginTime":.2},
            "m_ConstantClip":{"data":[1.,2.,3.,7.]}}}},
        "m_ClipBindingConstant":{"genericBindings":[{"path":1,"attribute":1,"typeID":4},
            {"path":9,"attribute":8,"typeID":3},{"path":2,"attribute":2,"typeID":4},
            {"path":3,"attribute":3,"typeID":4},{"path":1,"attribute":1,"typeID":4}]}}


class SamplingTests(unittest.TestCase):
    def test_values_and_poses_match_at_boundaries_and_for_unsorted_or_duplicate_keys(self):
        for keys in [[],[0.],[0.,0.,.5,1.],[-1.,.5,0.,1.],list(np.linspace(-1.,3.,64)),[float("nan")]]:
            for dense in [False,True]:
                original=LegacyClip(fixture(keys,dense));optimized=Clip(fixture(keys,dense))
                with np.errstate(invalid="ignore"):
                    for t in [-float("inf"),-1e300,*np.linspace(-2,4,121),0.,.2,.5,1.,float("inf")]:
                        self.assertEqual(np.asarray(original.values(t),dtype="<f8").tobytes(),np.asarray(optimized.values(t),dtype="<f8").tobytes())
                        a,b=original.pose(t),optimized.pose(t)
                        self.assertEqual(list(a),list(b))
                        for path in a:
                            self.assertEqual(list(a[path]),list(b[path]))
                            for attr in a[path]:self.assertEqual(np.asarray(a[path][attr],dtype="<f8").tobytes(),np.asarray(b[path][attr],dtype="<f8").tobytes())

    def test_nan_sample_retains_original_error_and_no_dense_behavior(self):
        for cls in [LegacyClip,Clip]:
            with self.assertRaises(ValueError):cls(fixture([0.])).values(float("nan"))
        a=LegacyClip(fixture([0.],False)).values(float("nan"))
        b=Clip(fixture([0.],False)).values(float("nan"))
        self.assertEqual(a,b)


if __name__=="__main__":unittest.main()
