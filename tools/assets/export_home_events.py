"""Export native Home event controls and resolve their sprites in separate RAM-bounded passes."""
import os
import json
from pathlib import Path
import subprocess
import sys

game,root=sys.argv[1:3]
tools=Path(__file__).resolve().parent
env=dict(os.environ,RRBTD6_UI_BUNDLES='*_monoscripts_*.bundle;ui_scenes_all_*.bundle;sprite_atlases_assets_all_*.bundle;localization_assets_all_*.bundle')
def run(script,*args):subprocess.run([sys.executable,str(tools/script),*map(str,args)],env=env,check=True)
fields={'racePrefab':'HomeRace','odysseyPrefab':'HomeOdyssey','ctPrefab':'HomeCt',
        'bossRushPrefab':'HomeBossRush','socialSeasonsPrefab':'HomeSocialSeason'}
run('dump_ui_prefab.py',game,Path(root)/'ui/prefabs',
    'scenefile:Assets/UI - New/MainMenuUi.unity=MainMenuUI',
    'scenefile:Assets/UI - New/Scenes/GameEventsUI.unity=GameEventsUI',
    *[f'field:EventButtonGroup/MainMenuEventPanel.{field}={alias}' for field,alias in fields.items()])
env['RRBTD6_UI_BUNDLES']='*_monoscripts_*.bundle;asset_references_assets_all_*.bundle;sprite_atlases_assets_all_*.bundle'
bosses=['Bloonarius','Vortex','Lych','Dreadbloon','Phayze','Blastapopoulos','Diamondback']
run('dump_ui_prefab.py',game,Path(root)/'ui/prefabs',
    *[f'Assets/UI - New/Prefabs/MainMenuEventPanel/{boss}EventPanelBtn.prefab' for boss in bosses])
run('resolve_ui_sprites.py',game,root,'asset_references_assets_all_*.bundle',
    'MainMenuUI','GameEventsUI',*fields.values(),*[f'{boss}EventPanelBtn' for boss in bosses])
run('export_menu_chrome.py',game,root)
index=json.loads((Path(root)/'all_container_paths.json').read_text(encoding='utf-8'))
paths=[path for path in index if any(path.endswith(f'/EventBannerSmall{boss}.png') for boss in bosses)]
if paths:run('export_textures.py',game,Path(root)/'sprites/MenuChrome',*paths)
