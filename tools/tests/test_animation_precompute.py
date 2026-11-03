"""Bit-exact animation-export math against the pre-optimization reference; no game assets."""
import ast
from pathlib import Path
import unittest
import numpy as np

REFERENCE = "def world_mats_fast(pose, time=0.0):\n    W = {}\n    for tid in order:\n        n = T[tid]\n        p, q, s = (n['p'], n['q'], n['s'])\n        if pose is not None and tid in hash_of:\n            h = pose.get(hash_of[tid], {})\n            p = h.get(1, p)\n            q = h.get(2, q)\n            if 4 in h:\n                q = euler_quat(h[4])\n            s = h.get(3, s)\n        if tid in spins:\n            axis, speed = spins[tid]\n            angle = time * speed * np.pi\n            q = [*axis * np.sin(angle), np.cos(angle)]\n        m = trs_mat(p, q, s)\n        W[tid] = parent_world @ m if n['parent'] is None else W[n['parent']] @ m\n    return W\n\ndef pose_vertices(W, r):\n    vh = np.c_[r['v'], np.ones(len(r['v']))]\n    nh = np.c_[r['n'], np.zeros(len(r['n']))]\n    if r['skin'] is not None:\n        bones, bind, w, bi = r['skin']\n        mats = np.stack([(W[b] if b in W else np.eye(4)) @ (bind[i] if i < len(bind) else np.eye(4)) for i, b in enumerate(bones)])\n        pos = np.zeros((len(vh), 4))\n        nor = np.zeros((len(vh), 4))\n        for k in range(w.shape[1]):\n            ms = mats[bi[:, k]]\n            pos += w[:, k:k + 1] * np.einsum('nij,nj->ni', ms, vh)\n            nor += w[:, k:k + 1] * np.einsum('nij,nj->ni', ms, nh)\n    else:\n        m = W[r['tid']]\n        pos, nor = (vh @ m.T, nh @ m.T)\n    nn = nor[:, :3]\n    nn = nn / np.maximum(np.linalg.norm(nn, axis=1, keepdims=True), 1e-09)\n    return (pos[:, :3], nn)"

SOURCE=ast.parse((Path(__file__).resolve().parents[1]/"assets/export_animated.py").read_text(encoding="utf-8"))
MAIN=next(n for n in SOURCE.body if isinstance(n,ast.FunctionDef) and n.name=="main")


def compile_nodes(nodes, namespace):
    exec(compile(ast.Module(body=nodes,type_ignores=[]),"export-math","exec"),namespace)


class PoseCacheTests(unittest.TestCase):
    def fixtures(self):
        rng=np.random.default_rng(73)
        T={}
        for i in range(12):
            T[i]={"p":rng.normal(size=3),"q":rng.normal(size=4),"s":rng.uniform(-2,2,size=3),"parent":i-1 if i else None}
        common={"np":np,"T":T,"order":list(T),"hash_of":{i:100+i for i in T},"spins":{},"parent_world":np.eye(4)}
        compile_nodes([n for n in SOURCE.body if isinstance(n,ast.FunctionDef) and n.name in ("quat_mat","euler_quat","trs_mat")],common)
        new=dict(common);old=dict(common)
        compile_nodes([n for n in MAIN.body if isinstance(n,ast.FunctionDef) and n.name in ("world_mats_fast","pose_vertices")],new)
        compile_nodes([n for n in MAIN.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=="rest_mats" for t in n.targets)],new)
        exec(REFERENCE,old)
        return rng,old,new

    def test_transform_bytes_match_for_static_posed_and_rotating_bones(self):
        rng,old,new=self.fixtures()
        poses=[None,{}, {100:{1:rng.normal(size=3)},103:{2:rng.normal(size=4)},108:{3:rng.uniform(.1,2,size=3)}},
            {101:{4:rng.normal(size=3)*120},110:{1:rng.normal(size=3),4:rng.normal(size=3)*90}}]
        for spins in [{},{2:(np.array([0.,1.,0.]),.75),11:(np.array([1.,0.,0.]),-1.25)}]:
            old["spins"]=new["spins"]=spins
            for pose in poses:
                for time in [0.,1/60,.23,1.5]:
                    expected=old["world_mats_fast"](pose,time);actual=new["world_mats_fast"](pose,time)
                    for bone in expected:self.assertEqual(actual[bone].tobytes(),expected[bone].tobytes())

    def test_vertex_bytes_match_for_static_and_weighted_meshes(self):
        rng,old,new=self.fixtures();W=old["world_mats_fast"]({101:{4:(30,90,-10)}})
        for skinned in [False,True]:
            r={"v":rng.normal(size=(31,3)),"n":rng.normal(size=(31,3)),"tid":2,"skin":None}
            if skinned:
                weights=rng.uniform(size=(31,4));weights/=weights.sum(axis=1,keepdims=True)
                r["skin"]=(list(range(4)),[np.eye(4) for _ in range(4)],weights,rng.integers(0,4,size=(31,4)))
            new["renderers"]=[r]
            compile_nodes([n for n in MAIN.body if isinstance(n,ast.For) and 'r["vh"]' in ast.unparse(n).replace("'",'"')],new)
            expected=old["pose_vertices"](W,r);actual=new["pose_vertices"](W,r)
            for a,b in zip(actual,expected):self.assertEqual(a.tobytes(),b.tobytes())


if __name__=="__main__":unittest.main()
