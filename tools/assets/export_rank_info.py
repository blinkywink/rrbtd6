"""Extract original player-rank thresholds for local account UI; no progress writes."""
import glob
import json
from pathlib import Path
import sys
import xml.etree.ElementTree as ET
import UnityPy

game,root=map(Path,sys.argv[1:3])
base=game/'BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full'
env=UnityPy.load(glob.glob(str(base/'game_data_assets_all_*.bundle'))[0],glob.glob(str(base/'*_monoscripts_*.bundle'))[0])
obj=env.container['Assets/Data/GameData.asset']
obj=obj.deref() if hasattr(obj,'deref') else obj
pointer=obj.read_typetree()['rankInfo']
if pointer['m_FileID']:raise ValueError('Unexpected external rank data.')
ranks=obj.assets_file.objects[pointer['m_PathID']].read_typetree()
loc={t.attrib['id']:''.join(t.itertext()) for t in ET.parse(root/'text/English.xml').iter('T') if 'id' in t.attrib}
output={'thresholds':[r['totalXpNeeded'] for r in ranks['rankInfo']],
        'veteranInterval':ranks['xpNeededPerVeteranRank'],'maxLabel':loc['Max Level'],
        # RankInfo.GetHeroUnlockLevel: the first rank (1-based) whose levelUnlockString contains the hero.
        'unlockStrings':[r['levelUnlockString'] for r in ranks['rankInfo']]}
path=root/'ui/runtime/rank_info.json'
path.parent.mkdir(parents=True,exist_ok=True)
path.write_text(json.dumps(output,indent=1),encoding='utf-8')
print('Exported rank thresholds:',len(output['thresholds']))
