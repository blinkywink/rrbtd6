"""Resolve late-bound sprites by Unity's render-data key, never by display name."""
import glob
from pathlib import Path


class SpriteImages:
    def __init__(self, bundle_dir):
        self.bundle_dir = str(bundle_dir)
        self.atlas = None
        self.sprites = {}

    def image(self, sprite):
        try:
            return sprite.image
        except ValueError:
            # A null local texture is valid when the runtime binds a SpriteAtlas.
            if sprite.m_RD.texture.m_PathID:
                raise
        return self.source(sprite).image

    def source(self, sprite):
        """The Sprite holding this one's pixels: itself, or for a late-bound sprite the atlas
        bundle's copy with the same render data key (its rect may be another resolution).
        KeyError when the atlas bundle has no copy."""
        if sprite.m_RD.texture.m_PathID:
            return sprite
        if self.atlas is None:
            import UnityPy
            path = glob.glob(str(Path(self.bundle_dir) / "sprite_atlases_assets_all_*.bundle"))[0]
            # Its decompressed files in local_extracted/_bundle_cache when there: decompressing
            # the bundle again can run out of memory.
            cached = [d for d in sorted(glob.glob(str(Path("local_extracted/_bundle_cache") / (glob.escape(Path(path).name) + "-*"))))
                      if Path(d).is_dir()]
            if cached:
                self.atlas = UnityPy.load(*[str(f) for f in sorted(Path(cached[0]).iterdir())])
            else:
                self.atlas = UnityPy.load(path)
            for obj in self.atlas.objects:
                if obj.type.name == "Sprite":
                    item = obj.read()
                    self.sprites[(str(item.m_RenderDataKey[0]), item.m_RenderDataKey[1])] = item
        key = str(sprite.m_RenderDataKey[0]), sprite.m_RenderDataKey[1]
        return self.sprites[key]
