"""Bind original Legends slideshow references and unresolved localization. Local data only."""
import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

root = Path(sys.argv[1])
scene = json.loads((root/'ui/prefabs/LegendsSelectUI.json').read_text(encoding='utf8'))
loc = {t.attrib['id']: ''.join(t.itertext()).replace('\\n', '\n')
       for t in ET.parse(root/'text/English.xml').iter('T') if 'id' in t.attrib}
nodes = []
def walk(node, path=''):
    path = path+'/'+node['name'] if path else node['name']
    nodes.append((path.removeprefix('LegendsSelectUI/'), node))
    for child in node.get('children', []):
        walk(child, path)
walk(scene)
refs = {str(c['id']): path for path, n in nodes for c in n.get('comps', []) if 'id' in c}
refs.update({str(n['id']): path for path, n in nodes})
def resolve(value):
    return refs[value.removeprefix('->')]
panel = next(c for _, n in nodes for c in n.get('comps', []) if c['type']=='LegendsSelectPanel')
slides = [path for path,n in nodes if any(c['type']=='LegendsSelectSubPanel' for c in n.get('comps', []))]
text = [{'path': path, 'text': loc[c['m_text']]} for path,n in nodes for c in n.get('comps', [])
        if c.get('m_text') in loc]
panels = [{key: resolve(c[key]) for key in ('mainPanel','purchasePanel')}
          for _,n in nodes for c in n.get('comps', [])
          if c['type']=='LegendsSelectSubPanel' and 'mainPanel' in c]
result = {'slides': slides, 'text': text, 'panels': panels,
          **{key: resolve(panel[key]) for key in ('infoButton','infoClose','infoPanel','prevBtn','nextBtn')}}
(root/'ui/runtime/legends_menu.json').write_text(json.dumps(result,ensure_ascii=False,indent=1),encoding='utf8')
print(f'Legends: {len(slides)} source panels, {len(text)} localized text bindings')
