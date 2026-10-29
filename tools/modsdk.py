"""Author RustTD6 mods in Python; emit data consumed by the native client.

python tools/modsdk.py build path/to/mod.py --out mods/my-mod/mod.json
The author script receives Mod in its globals and must leave an instance named mod.
Author scripts run only on the author's explicit command, never in-game or via P2P.
"""
import argparse
import json
from pathlib import Path
import re
import runpy


class Projectile:
    def __init__(self, path, fields):
        self.path, self.fields = path, fields


class Tower:
    def __init__(self, mod, identifier):
        self.mod, self.id = mod, identifier

    def projectile(self, template=None, *, tier="000", attack=0, weapon=0, **fields):
        if template is not None:
            if not isinstance(template, Projectile):
                raise TypeError("Expected a projectile created by mod.projectile")
            self.mod.copy(template.path, f"/tower_tiers/{self.id}/tiers/{tier}/attacks/{attack}/weapons/{weapon}/projectile")
            fields = {**template.fields, **fields}
        for key, value in fields.items():
            self.mod.set(f"/tower_tiers/{self.id}/tiers/{tier}/attacks/{attack}/weapons/{weapon}/projectile/{key}", value)
        return self

    def weapon(self, *, tier="000", attack=0, weapon=0, **fields):
        for key, value in fields.items():
            self.mod.set(f"/tower_tiers/{self.id}/tiers/{tier}/attacks/{attack}/weapons/{weapon}/{key}", value)
        return self


class Mod:
    def __init__(self, identifier, name, version="1.0.0", description=""):
        if not re.fullmatch(r"[a-z0-9][a-z0-9._-]{0,63}", identifier):
            raise ValueError("Use a lowercase mod ID, starting with a letter or digit")
        self.id = identifier
        self.manifest = dict(api_version=1, id=identifier, name=name, version=version,
                             description=description, content=[])

    def set(self, path, value):
        self.manifest["content"].append(dict(op="set", path=path, value=value))
        return self

    def copy(self, source, target):
        self.manifest["content"].append(dict(op="copy", **{"from": source}, path=target))
        return self

    def projectile(self, name, *, base_tower="DartMonkey", base_tier="000", attack=0, weapon=0, **fields):
        # A build-time template; at runtime each weapon holds native compiled projectile data.
        return Projectile(f"/tower_tiers/{base_tower}/tiers/{base_tier}/attacks/{attack}/weapons/{weapon}/projectile", fields)

    def tower(self, name, base="DartMonkey", **patch):
        identifier = f"{self.id}-{name}"
        self.manifest["content"].append(dict(op="tower", id=identifier, base=base, patch=patch))
        return Tower(self, identifier)

    def bloon(self, name, base="Red", **patch):
        identifier = f"{self.id}-{name}"
        self.manifest["content"].append(dict(op="bloon", id=identifier, base=base, patch=patch))
        return identifier

    def mode(self, name, base="Medium", **patch):
        identifier = f"{self.id}-{name}"
        self.manifest["content"].append(dict(op="mode", id=identifier, base=base, patch=patch))
        return identifier

    def all_modes(self, **patch):
        self.manifest["content"].append(dict(op="all_modes", patch=patch))
        return self

    def menu(self, page, path, *, text=None, visible=None):
        edit = dict(op="menu", page=page, path=path)
        if text is not None:
            edit["text"] = text
        if visible is not None:
            edit["visible"] = bool(visible)
        self.manifest["content"].append(edit)
        return self

    def write(self, path):
        if len(self.manifest["content"]) > 256:
            raise ValueError("At most 256 content edits per mod")
        data = json.dumps(self.manifest, indent=2, ensure_ascii=False, allow_nan=False) + "\n"
        if len(data.encode("utf-8")) > 64 * 1024:
            raise ValueError("Manifest exceeds 64 KiB")
        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(data, encoding="utf-8")
        return path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["build"])
    parser.add_argument("script", type=Path)
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    namespace = runpy.run_path(str(args.script), init_globals={"Mod": Mod})
    mod = namespace.get("mod")
    if not isinstance(mod, Mod):
        parser.error("The script must define mod = Mod(...)")
    print(mod.write(args.out or args.script.parent / "mod.json"))


if __name__ == "__main__":
    main()
