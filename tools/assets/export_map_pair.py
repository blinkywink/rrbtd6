"""Export map logic and geometry in one process, reusing the loaded scene bundle."""
from pathlib import Path
import sys


def main():
    game, key, maps, model = sys.argv[1:5]
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import export_map
    sys.argv = ["export_map.py", game, key, maps]
    scene = export_map.main()
    sys.argv = ["export_model.py", game, model, "scene", "map_" + key]
    import export_model
    export_model.main(scene)


if __name__ == "__main__":
    main()
