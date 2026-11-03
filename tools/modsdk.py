"""Author RustTD6 mods in Python; emit data consumed by the native client.

python tools/modsdk.py build path/to/mod.py --out mods/my-mod/mod.json
The author script receives Mod in its globals and must leave an instance named mod.
Python author scripts run only on explicit build. Optional embedded Rhai event scripts run in the client.
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

    def shop(self, *, before=None, after=None, index=None, visible=None, category=None):
        """Position this tower anywhere in the native shop, hide it, or change its category."""
        if sum(v is not None for v in (before,after,index))>1:
            raise ValueError("Choose before, after, or index")
        categories={"primary":1,"military":2,"magic":4,"support":8}
        if isinstance(category,str):category=categories[category.lower()]
        edit=dict(op="shop",tower=self.id)
        for key,value in [("before",before),("after",after),("index",index),("visible",visible),("category",category)]:
            if value is not None:edit[key]=value.id if isinstance(value,Tower) else value
        self.mod.manifest["content"].append(edit)
        return self

    def stats(self, **fields):
        for key,value in fields.items():self.mod.set(f"/towers/{self.id}/{key}",value)
        return self

    def model(self, asset, *, tiers=None):
        """Use a baked model and its Muzzle/release timing for the selected tiers."""
        token = self.mod.model(asset)
        meta = json.loads((self.mod.directory / "assets" / asset / "meta.json").read_text(encoding="utf-8"))
        socket = meta.get("mod_socket", {})
        muzzle, release = socket.get("muzzle", [0, 8, 5]), socket.get("release", .15)
        import math
        if len(muzzle) != 3 or not all(isinstance(v, (int, float)) and math.isfinite(v) for v in muzzle) or not isinstance(release, (int, float)) or not math.isfinite(release) or not 0 <= release <= 5:
            raise ValueError("Invalid exported muzzle or release timing")
        if tiers is None:
            self.mod.manifest["content"].append(dict(op="tower_model", tower=self.id, asset=token, muzzle=muzzle, release=release))
            return self
        self.mod.set(f"/towers/{self.id}/model", token)
        for tier in tiers:
            if not re.fullmatch(r"[0-5]{3}", tier):
                raise ValueError("Use three-digit tier keys such as 000 or 100")
            self.weapon(tier=tier, eject=[-muzzle[0], muzzle[2], muzzle[1]], animation_offset=release)
        return self

    def upgrade(self, path, tier, *, cost=None, stats=None, weapon=None, projectile=None, model=None):
        """Edit an upgrade and all later/crosspath states. Paths 0–2, tiers 1–5.
        Values are absolute replacements; later calls override earlier calls.
        """
        if not isinstance(path, int) or not 0 <= path < 3 or not isinstance(tier, int) or not 1 <= tier <= 5:
            raise ValueError("The current engine supports paths 0–2, tiers 1–5; extra paths are not implemented yet")
        edit=dict(op="upgrade_patch", tower=self.id, path=path, tier=tier)
        if isinstance(projectile, Projectile):
            edit["projectile_from"]=projectile.path
            projectile=projectile.fields
        for key,value in [("cost",cost),("stats",stats),("weapon",weapon),("projectile",projectile)]:
            if value is not None:edit[key]=value
        if model is not None:
            edit["model"]=self.mod.model(model)
            socket=json.loads((self.mod.directory/"assets"/model/"meta.json").read_text(encoding="utf-8")).get("mod_socket",{})
            edit["muzzle"]=socket.get("muzzle",[0,8,5]);edit["release"]=socket.get("release",.15)
        self.mod.manifest["content"].append(edit)
        return self

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
    author_directory = Path(".")
    def __init__(self, identifier, name, version="1.0.0", description=""):
        if not re.fullmatch(r"[a-z0-9][a-z0-9._-]{0,63}", identifier):
            raise ValueError("Use a lowercase mod ID, starting with a letter or digit")
        self.id = identifier
        self.directory = self.author_directory
        self.manifest = dict(api_version=1, id=identifier, name=name, version=version,
                             description=description, content=[])

    def model(self, asset):
        if not re.fullmatch(r"[A-Za-z0-9_-]{1,64}", asset) or not re.fullmatch(r"[A-Za-z0-9_-]{1,64}", self.id):
            raise ValueError("Model and mod folder IDs use letters, digits, underscores and hyphens")
        if self.directory.name != self.id:
            raise ValueError("Put model mods in a folder matching the mod ID")
        if not (self.directory / "assets" / asset / "meta.json").is_file():
            raise ValueError(f"Bake assets/{asset} first with tools/mod_model.py")
        return f"mod:{self.id}/{asset}"

    def set(self, path, value):
        self.manifest["content"].append(dict(op="set", path=path, value=value))
        return self

    def insert(self, path, index, value):
        """Insert a native attack, weapon, ability, round group, or another array entry."""
        self.manifest["content"].append(dict(op="insert",path=path,index=index,value=value))
        return self

    def remove(self, path, index):
        self.manifest["content"].append(dict(op="remove",path=path,index=index))
        return self

    def copy(self, source, target):
        self.manifest["content"].append(dict(op="copy", **{"from": source}, path=target))
        return self

    def projectile(self, name, *, base_tower="DartMonkey", base_tier="000", attack=0, weapon=0, **fields):
        # A build-time template; at runtime each weapon holds native compiled projectile data.
        return Projectile(f"/tower_tiers/{base_tower}/tiers/{base_tier}/attacks/{attack}/weapons/{weapon}/projectile", fields)

    def existing_tower(self, identifier):
        """Get an editable handle without cloning a tower."""
        return Tower(self,identifier)

    def script(self, name, source=None, *, file=None, tick_interval=60):
        """Embed Rhai event code; the client compiles it once when a match loads."""
        if (source is None)==(file is None):raise ValueError("Supply source or file")
        if file is not None:source=(self.directory/file).read_text(encoding="utf-8")
        if not isinstance(source,str) or len(source.encode("utf-8"))>32768:raise ValueError("Script exceeds 32 KiB")
        if not isinstance(tick_interval,int) or not 1<=tick_interval<=3600:raise ValueError("tick_interval must be 1–3600 simulation ticks")
        self.manifest["content"].append(dict(op="script",id=name,source=source,tick_interval=tick_interval))
        return self

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
    Mod.author_directory = args.script.resolve().parent
    namespace = runpy.run_path(str(args.script), init_globals={"Mod": Mod})
    mod = namespace.get("mod")
    if not isinstance(mod, Mod):
        parser.error("The script must define mod = Mod(...)")
    print(mod.write(args.out or args.script.parent / "mod.json"))


if __name__ == "__main__":
    main()
