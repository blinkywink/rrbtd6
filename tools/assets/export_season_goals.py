"""Prepare localized native Social Season goal labels and icons (local assets only)."""
import json,sys,xml.etree.ElementTree as ET
from pathlib import Path
root=Path(sys.argv[1]);loc={t.attrib['id']:''.join(t.itertext()).replace('\\n','\n') for t in ET.parse(root/'text/English.xml').iter('T') if 'id' in t.attrib}
index=json.loads((root/'all_container_paths.json').read_text(encoding='utf8'))
import re
catalog={}
for path in index:
    if '/Events/SocialSeasons/Goals/GoalIcon' not in path or not path.endswith('.png'):continue
    icon=Path(path).stem
    words=re.findall(r'[A-Z][a-z0-9]*',icon.removeprefix('GoalIcon'))
    key='_'.join(words).lower();loc_key='_'.join(words)
    catalog[key]={'label':loc.get(loc_key,' '.join(words)),'icon':icon}
out={'goals':catalog,'phase':loc.get('Phase','Phase'),'tier':loc.get('Tier','Tier'),'info':loc.get('SocialSeasonsInfo6',''),'excluded':loc.get('SocialSeasonsExcludedModes','')}
p=root/'ui/runtime/season_goals.json';p.write_text(json.dumps(out,indent=1),encoding='utf8')
print(len(catalog),'localized goal categories')
