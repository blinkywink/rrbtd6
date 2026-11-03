"""Reuse immutable prefab lookups within one bounded exporter process."""
class PrefabIndex:
    def __init__(self):
        self.bundles = {}

    def root(self, env, path):
        key = id(env)
        if key not in self.bundles:
            objects = list(env.objects)
            bundle = next(o for o in objects if o.type.name == "AssetBundle").read_typetree()
            paths = {}
            for name, entry in bundle["m_Container"]:
                paths.setdefault(name, entry["asset"]["m_PathID"])
            roots = {}
            for obj in objects:
                roots.setdefault(obj.path_id, obj)
            # Keep the environment alive with the lookup so Python cannot reuse its id.
            self.bundles[key] = env, paths, roots
        _, paths, roots = self.bundles[key]
        return roots[paths[path]]


class TextureCopies:
    """Reuse exact PNG bytes, with a bounded LRU and no change to texture quality."""
    def __init__(self, limit=64 << 20):
        from collections import OrderedDict
        self.entries = OrderedDict()
        self.limit, self.size = limit, 0

    def source(self, obj):
        key = obj.assets_file.name, obj.path_id
        return None if key in self.entries else obj.read()

    def write(self, obj, output, texture=None):
        from io import BytesIO
        from pathlib import Path
        key = obj.assets_file.name, obj.path_id
        if key in self.entries:
            name, data = self.entries.pop(key)
            self.entries[key] = name, data
        else:
            texture = texture if texture is not None else obj.read()
            name = f"{texture.m_Name}_{abs(obj.path_id) % 100000}.png"
            with BytesIO() as stream:
                texture.image.save(stream, format="PNG")
                data = stream.getvalue()
            if len(data) <= self.limit:
                while self.entries and self.size + len(data) > self.limit:
                    _, (_, previous) = self.entries.popitem(last=False)
                    self.size -= len(previous)
                self.entries[key] = name, data
                self.size += len(data)
        folder = Path(output) / "tex"
        folder.mkdir(parents=True, exist_ok=True)
        (folder / name).write_bytes(data)
        return "tex/" + name


class ClipCopies:
    """Decode shared clips once; keep each model's role/weight annotations private."""
    def __init__(self, limit=32):
        from collections import OrderedDict
        self.entries = OrderedDict()
        self.limit = limit

    def get(self, obj, decoder):
        from copy import copy
        key = obj.assets_file.name, obj.path_id
        if key in self.entries:
            clip = self.entries.pop(key)
        else:
            clip = decoder(obj.read_typetree())
        self.entries[key] = clip
        while len(self.entries) > self.limit:
            self.entries.popitem(last=False)
        # Clip.pose/values only read decoded arrays. The exporter changes only
        # top-level role/weight attributes, so a shallow copy keeps those separate.
        return copy(clip)
