"""A models/extra display's Animator state machine (plain Animator, no MonkeyAnimationController):
its entry clip and the clip each int-parameter value jumps to (AnyState Equals), written to
local_extracted/models/extra/<name>/anim_states.json. Used for the Great White / Orca / Megalodon
grab (GreatWhiteEffect: bitingAnimationState 4 = Thrash, exitAnimationState 5 = Fall), and the Beast
Handler birds (entry Hover, 1 Fly, 3 Snatch, 4 Placement): also a states.json for each exported folder of
the display that has none.

    tools/.venv/Scripts/python.exe -I tools/assets/export_display_states.py <prefab name> [...]

Proprietary data: local use only.
"""
import os, sys, glob, json, UnityPy
paths = json.load(open("local_extracted/all_container_paths.json"))
AA = "C:/Program Files (x86)/Steam/steamapps/common/BloonsTD6/BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full"
for name in sys.argv[1:]:
    prefab = next(k for k in paths if k.endswith("/" + name + ".prefab"))
    bundle = paths[prefab]
    d = [x for x in glob.glob(f"local_extracted/_bundle_cache/{bundle}-*") if os.path.isdir(x)]
    env = UnityPy.load(*[os.path.join(d[0], f) for f in sorted(os.listdir(d[0])) if f.startswith("CAB-")]) if d else UnityPy.load(os.path.join(AA, bundle))
    go = env.container[prefab].read()

    def find(go):
        for c in go.m_Components:
            o = c.read() if hasattr(c, "read") else c.component.read()
            if o.object_reader.type.name == "Animator":
                return o
            if o.object_reader.type.name == "Transform":
                for ch in o.m_Children:
                    r = find(ch.read().m_GameObject.read())
                    if r:
                        return r
    an = find(go)
    ctl = an.m_Controller.read()
    over = {}
    if ctl.object_reader.type.name == "AnimatorOverrideController":
        over = {o.m_OriginalClip.path_id: o.m_OverrideClip for o in ctl.m_Clips if o.m_OverrideClip.path_id}
        ctl = ctl.m_Controller.read()
    ct = ctl.object_reader.read_typetree()
    sm = ct["m_Controller"]["m_StateMachineArray"][0]["data"]
    clips = ctl.m_AnimationClips
    names = []
    for sc in sm["m_StateConstantArray"]:
        st = sc["data"]
        ci = st["m_BlendTreeConstantArray"][0]["data"]["m_NodeArray"][0]["data"]["m_ClipID"] if st.get("m_BlendTreeConstantArray") else None
        names.append(over.get(clips[ci].path_id, clips[ci]).read().m_Name if ci is not None and ci < len(clips) else None)
    param = {}
    for t in sm.get("m_AnyStateTransitionConstantArray", []):
        td = t["data"]
        cs = td["m_ConditionConstantArray"]
        if len(cs) == 1 and cs[0]["data"]["m_ConditionMode"] == 6:
            param[int(cs[0]["data"]["m_EventThreshold"])] = names[td["m_DestinationState"]] if td["m_DestinationState"] < len(names) else None
    entry = sm.get("m_DefaultState", 0)
    for sel in sm.get("m_SelectorStateConstantArray", []):
        if sel["data"].get("m_IsEntry") and sel["data"]["m_TransitionConstantArray"]:
            dd = sel["data"]["m_TransitionConstantArray"][0]["data"]["m_Destination"]
            if dd < len(names):
                entry = dd
    exits = {names[i]: (s["data"]["m_TransitionConstantArray"][0]["data"]["m_DestinationState"] if s["data"]["m_TransitionConstantArray"] else None) for i, s in enumerate(sm["m_StateConstantArray"])}
    out = {"entry": names[entry], "param": {str(k): v for k, v in param.items()}, "states": names}
    print(name, json.dumps(out))
    # Every exported folder of this display (models/extra, models/air...): anim_states.json, and a
    # states.json the renderer's AnimModel reads (state 0 the entry clip, state n the clip the
    # parameter value n jumps to; a gap repeats the entry).
    top = max([0] + list(param))
    states = [out["entry"]] + [param.get(k) or out["entry"] for k in range(1, top + 1)]
    for d in glob.glob(f"local_extracted/models/*/{name}") + glob.glob(f"local_extracted/models/*/tiers/{name}"):
        if os.path.exists(os.path.join(d, "meta.json")):
            json.dump(out, open(os.path.join(d, "anim_states.json"), "w"), indent=1)
            if not os.path.exists(os.path.join(d, "states.json")):
                json.dump(states, open(os.path.join(d, "states.json"), "w"))
