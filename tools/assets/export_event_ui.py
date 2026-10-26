"""Prepare original event layouts/art for local builds; live rules use the CDN at runtime."""
import os
from pathlib import Path
import subprocess
import sys

game,root=map(Path,sys.argv[1:3]);tools=Path(__file__).resolve().parent
env=dict(os.environ,RRBTD6_UI_BUNDLES='*_monoscripts_*.bundle;ui_scenes_all_*.bundle;sprite_atlases_assets_all_*.bundle;localization_assets_all_*.bundle')
def run(script,*args):subprocess.run([sys.executable,str(tools/script),*map(str,args)],env=env,check=True)
scenes=['DailyChallengeUI','RaceEventUI','ChallengeRulesUI','BossEventUI','BossEventPlayUI','OdysseyEventUI','CollectionEventUI','SocialSeasonsEventUI','SocialSeasonsGoalsUI','BossRushMapUI']
run('dump_ui_prefab.py',game,root/'ui/prefabs',*[f'scenefile:Assets/UI - New/Scenes/{scene}.unity={scene}' for scene in scenes])
env['RRBTD6_UI_BUNDLES']='*_monoscripts_*.bundle;ui_scenes_all_*.bundle'
run('export_ui_runtime.py',game,root/'ui/runtime',*scenes)
env['RRBTD6_UI_BUNDLES']='*_monoscripts_*.bundle;asset_references_assets_all_*.bundle;sprite_atlases_assets_all_*.bundle;localization_assets_all_*.bundle'
run('dump_ui_prefab.py',game,root/'ui/prefabs','Assets/UI - New/Prefabs/Store/Loot/SmallBaseLootItem.prefab','Assets/UI - New/Prefabs/Store/Loot/SmallTrophyLootItem.prefab','Assets/UI - New/InGameUi/Prefabs/RaceUi.prefab')
run('resolve_ui_sprites.py',game,root,'asset_references_assets_all_*.bundle',*scenes,'SmallBaseLootItem','SmallTrophyLootItem','RaceUi')
run('export_menu_chrome.py',game,root)

run('export_social_season_art.py',game,root)
run('export_season_goals.py',root)
