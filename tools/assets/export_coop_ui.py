"""Prepare original base co-op scenes and artwork for the local prototype."""
import os,subprocess,sys
from pathlib import Path
game,root=map(Path,sys.argv[1:3]);tools=Path(__file__).resolve().parent
env=dict(os.environ,RRBTD6_UI_BUNDLES='*_monoscripts_*.bundle;ui_scenes_all_*.bundle;sprite_atlases_assets_all_*.bundle;localization_assets_all_*.bundle')
def run(script,*args):subprocess.run([sys.executable,str(tools/script),*map(str,args)],env=env,check=True)
scenes=['CoopMapSelectUI','CoopLobbyUI','CoopQuickMatchUI','CoopJoinMatchUI']
run('dump_ui_prefab.py',game,root/'ui/prefabs',*[f'scenefile:Assets/UI - New/Scenes/{s}.unity={s}' for s in scenes])
run('dump_ui_prefab.py',game,root/'ui/prefabs','Assets/UI - New/InGameUi/Prefabs/CoopManager.prefab')
env['RRBTD6_UI_BUNDLES']='*_monoscripts_*.bundle;ui_scenes_all_*.bundle'
run('export_ui_runtime.py',game,root/'ui/runtime',*scenes)
run('export_ui_runtime.py',game,root/'ui/runtime','prefab:Assets/UI - New/InGameUi/Prefabs/CoopManager.prefab')
run('resolve_ui_sprites.py',game,root,'asset_references_assets_all_*.bundle',*scenes)
run('export_menu_chrome.py',game,root)
run('prepare_coop_ui.py',root)
