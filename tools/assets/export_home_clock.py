"""Export the home world's Town Hall clock hands (ClockTimeAnimator); local assets only.

ClockTimeAnimator.Update (VERIFIED-DECOMP @0x1804e0dc0) sets each hand's localRotation to
Euler(0, 0, -(DateTime.Now part / whole x 360)): seconds / 60, minutes / 60, (hour % 12) / 12.
Each hand keeps its local position and scale under its parent; the renderer rebuilds it.

    python -I tools/assets/export_home_clock.py <game dir> local_extracted
"""
import json
import sys
from pathlib import Path


def export_clock(af, em, scene_path, root):
    holder = None
    for o in af.objects.values():
        if o.type.name != 'MonoBehaviour':
            continue
        t = o.read_typetree()
        if 'hourHand' in t and 'minuteHand' in t:
            holder = t
            break
    assert holder is not None, 'ClockTimeAnimator not found'
    hands = []
    for role, divisor in (('hourHand', 12), ('minuteHand', 60), ('secondHand', 60)):
        ref = holder[role]['m_PathID']
        if not ref:
            continue
        go = af.objects[ref].read_typetree()
        tid = next(af.objects[c['component']['m_PathID']] for c in go['m_Component']
                   if af.objects[c['component']['m_PathID']].type.name == 'Transform').path_id
        t = af.objects[tid].read_typetree()
        hands.append({'role': role, 'divisor': divisor, 'path': scene_path(tid),
                      'position': [t['m_LocalPosition'][a] for a in 'xyz'],
                      'scale': [t['m_LocalScale'][a] for a in 'xyz'],
                      'parent': em.world_matrix(af, t['m_Father']['m_PathID']).T.flatten().tolist(),
                      'rest': em.world_matrix(af, tid).T.flatten().tolist()})
    out = Path(root) / 'models/mainmenu_anim/clock.json'
    out.write_text(json.dumps(hands, indent=1))
    return hands


if __name__ == '__main__':
    import glob
    game, root = sys.argv[1:3]
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import UnityPy
    sys.argv = [sys.argv[0], game, str(Path(root) / 'models/mainmenu'), 'scene', 'ui_scenes_all_']
    import export_model as em
    cab = 'CAB-613509f63417784aa3960cd4dc66a279'
    bundle = glob.glob(str(Path(em.AA) / 'ui_scenes_all_*.bundle'))[0]
    env = UnityPy.load(bundle)
    af = next(o.assets_file for o in env.objects if o.assets_file.name == cab)

    def scene_path(tid):
        names = []
        while tid:
            t = af.objects[tid].read_typetree()
            names.append(af.objects[t['m_GameObject']['m_PathID']].read_typetree()['m_Name'])
            tid = t['m_Father']['m_PathID']
        return '/'.join(reversed(names))

    for h in export_clock(af, em, scene_path, root):
        print(h['role'], h['path'])
