"""The profile page: its scene and item prefabs, animations, rank data and avatar/banner art.

Reads only the local BTD6 install; never contacts an account or online service.

Usage: python -I export_profile_ui.py <game install> <local_extracted>
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys


def export(game, root, all_art=True):
    tools = Path(__file__).resolve().parent
    env = dict(os.environ, RRBTD6_UI_BUNDLES='*_monoscripts_*.bundle;ui_scenes_all_*.bundle;sprite_atlases_assets_all_*.bundle;localization_assets_all_*.bundle;asset_references_assets_all_*.bundle')

    def run(script, *arguments):
        subprocess.run([sys.executable, '-I', str(tools / script), *map(str, arguments)], env=env, check=True)
    run('dump_ui_prefab.py', game, root / 'ui/prefabs',
        'scenefile:Assets/UI - New/Scenes/TrophyStoreUI.unity=TrophyStoreUI',
        'field:TrophyStoreScreen/TrophyStoreScreen.trophyPrefab=TrophyStoreCard',
        'scenefile:Assets/UI - New/Scenes/PlayerStatsUI.unity=PlayerStatsUI',
        'field:PlayerStatsScreen/PlayerStatsScreen.topTowerPrefab=ProfileTopTower',
        'field:PlayerStatsScreen/PlayerStatsScreen.prefabStat=ProfileStat',
        'field:PlayerStatsScreen/PlayerStatsScreen.medalPrefab=ProfileMedal',
        'field:PlayerStatsScreen/PlayerStatsScreen.avatarItemPrefab=ProfileAvatarItem',
        'field:PlayerStatsScreen/PlayerStatsScreen.bannerItemPrefab=ProfileBannerItem')
    env['RRBTD6_UI_BUNDLES'] = '*_monoscripts_*.bundle;ui_scenes_all_*.bundle'
    run('export_ui_runtime.py', game, root / 'ui/runtime', 'PlayerStatsUI')
    run('resolve_ui_sprites.py', game, root, 'asset_references_assets_all_*.bundle',
        'TrophyStoreUI', 'TrophyStoreCard', 'PlayerStatsUI', 'ProfileStat', 'ProfileTopTower', 'ProfileMedal', 'ProfileAvatarItem', 'ProfileBannerItem')
    run('export_rank_info.py', game, root)
    run('export_profile_cosmetics.py', game, root)
    index = json.loads((root / 'all_container_paths.json').read_text(encoding='utf-8'))
    paths = [path for path in index if path.startswith('Assets/ResizedImages/UI/') and path.endswith('.png') and (
        Path(path).stem == 'ProfileAvatar01' or ('/BossBloons/' in path and path.endswith('Badge.png')) or
        (all_art and path.startswith(('Assets/ResizedImages/UI/ProfileAvatars/', 'Assets/ResizedImages/UI/ProfileBanners/'))))]
    if all_art:
        cosmetics = json.loads((root / 'ui/runtime/profile_cosmetics.json').read_text(encoding='utf-8'))
        paths = list(dict.fromkeys(paths + [item[key] for items in cosmetics.values() for item in items
                                            for key in ['source', 'sourceSmall'] if item.get(key)]))
    if paths:
        run('export_textures.py', game, root / 'sprites/Account', *paths)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('game', type=Path)
    parser.add_argument('root', type=Path)
    args = parser.parse_args()
    export(args.game, args.root)
