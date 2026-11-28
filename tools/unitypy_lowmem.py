"""Bounded-memory loading of large UnityFS bundles for UnityPy 1.25.4.

UnityPy joins every decompressed block of a bundle into one bytes object and then copies each
inner file out of it again; asset_references_assets_all (844 MB, ~4.3 GB unpacked) needs ~9 GB of
RAM that way. For bundles above THRESHOLD this streams the blocks once into per-file caches under
$RRBTD6_BUNDLE_CACHE and parses each inner file from disk (UnityPy reads it lazily).

Activated in every tools/ process by sitecustomize.py when RRBTD6_BUNDLE_CACHE is set
(tools/extract_all.py sets it). The cache is game data: it lives under local_extracted/.
"""
import mmap
import os
import sys

from bundle_lock import cache_lock

import UnityPy
import UnityPy.files
from UnityPy.enums import ArchiveFlags, ArchiveFlagsOld
from UnityPy.helpers import ArchiveStorageManager, ImportHelper
from UnityPy.streams import EndianBinaryReader

bundle_module = sys.modules["UnityPy.files.BundleFile"]
file_module = sys.modules["UnityPy.files.File"]
if UnityPy.__version__ != "1.25.4":
    raise ImportError(f"unitypy_lowmem targets UnityPy 1.25.4, found {UnityPy.__version__}")

# Bundles above this are unpacked to disk once and every later load reads them from there instead
# of decompressing them again in each process (64 MB: a Mac setup 881 s -> 825 s, the effects step
# 124 s -> 89 s; RRBTD6_CACHE_THRESHOLD_MB overrides it).
THRESHOLD = int(os.environ.get("RRBTD6_CACHE_THRESHOLD_MB", "64")) << 20
CACHE = os.environ.get("RRBTD6_BUNDLE_CACHE")
BlockInfo = bundle_module.BlockInfo
DirectoryInfoFS = bundle_module.DirectoryInfoFS


class CachedNodes:
    """Stands in for the in-memory blocks reader: inner files already written to disk."""
    def __init__(self, folder, base_offset):
        self.folder, self.BaseOffset = folder, base_offset


def _cache_folder(reader, size):
    name = os.path.basename(getattr(getattr(reader, "stream", None), "name", "") or "")
    return os.path.join(CACHE, f"{name or 'bundle'}-{size}") if name else None


def read_fs(self, reader):
    size = reader.read_long()
    compressedSize = reader.read_u_int()
    uncompressedSize = reader.read_u_int()
    dataflagsValue = reader.read_u_int()
    if self.signature != "UnityFS":
        reader.read_byte()
    version = self.parse_version()
    if (version < (2020,) or (version[0] == 2020 and version < (2020, 3, 34))
            or (version[0] == 2021 and version < (2021, 3, 2)) or (version[0] == 2022 and version < (2022, 1, 1))):
        self.dataflags = ArchiveFlagsOld(dataflagsValue)
    else:
        self.dataflags = ArchiveFlags(dataflagsValue)
    if self.dataflags & self.dataflags.UsesAssetBundleEncryption:
        self.decryptor = ArchiveStorageManager.ArchiveStorageDecryptor(reader)
    if self.version >= 7 or (version[0] == 2019 and version >= (2019, 4, 15)):
        reader.align_stream(16)
        self._uses_block_alignment = True
    start = reader.Position
    if self.dataflags & ArchiveFlags.BlocksInfoAtTheEnd:
        reader.Position = reader.Length - compressedSize
        blocksInfoBytes = reader.read_bytes(compressedSize)
        reader.Position = start
    else:
        blocksInfoBytes = reader.read_bytes(compressedSize)
    blocksInfoBytes = self.decompress_data(blocksInfoBytes, uncompressedSize, self.dataflags)
    blocksInfoReader = EndianBinaryReader(blocksInfoBytes, offset=start)
    blocksInfoReader.read_bytes(16)
    m_BlocksInfo = [BlockInfo(blocksInfoReader.read_u_int(), blocksInfoReader.read_u_int(), blocksInfoReader.read_u_short())
                    for _ in range(blocksInfoReader.read_int())]
    m_DirectoryInfo = [DirectoryInfoFS(blocksInfoReader.read_long(), blocksInfoReader.read_long(),
                                       blocksInfoReader.read_u_int(), blocksInfoReader.read_string_to_null())
                       for _ in range(blocksInfoReader.read_int())]
    if m_BlocksInfo:
        self._block_info_flags = m_BlocksInfo[0].flags
    if isinstance(self.dataflags, ArchiveFlags) and self.dataflags & ArchiveFlags.BlockInfoNeedPaddingAtStart:
        reader.align_stream(16)

    total = sum(b.uncompressedSize for b in m_BlocksInfo)
    folder = _cache_folder(reader, size) if CACHE and total > THRESHOLD else None
    if folder is None:
        return m_DirectoryInfo, EndianBinaryReader(
            b"".join(self.decompress_data(reader.read_bytes(b.compressedSize), b.uncompressedSize, b.flags, i)
                     for i, b in enumerate(m_BlocksInfo)), offset=blocksInfoReader.real_offset())

    done = os.path.join(folder, ".complete")
    # A second exporter must never read or overwrite a partially prepared cache.
    with cache_lock(folder + ".lock"):
        if not os.path.exists(done):
            os.makedirs(folder, exist_ok=True)
            nodes = sorted(m_DirectoryInfo, key=lambda n: n.offset)
            outs = [open(os.path.join(folder, _safe(n.path)), "wb") for n in nodes]
            pos = 0
            for i, b in enumerate(m_BlocksInfo):
                data = self.decompress_data(reader.read_bytes(b.compressedSize), b.uncompressedSize, b.flags, i)
                end = pos + len(data)
                # Copy the slice of this block that belongs to each inner file.
                for n, out in zip(nodes, outs):
                    lo, hi = max(pos, n.offset), min(end, n.offset + n.size)
                    if lo < hi:
                        out.write(data[lo - pos:hi - pos])
                pos = end
            for out in outs:
                out.close()
            open(done, "w").close()
    return m_DirectoryInfo, CachedNodes(folder, blocksInfoReader.real_offset())


def _safe(path):
    return path.replace("/", "_").replace("\\", "_")


_original_read_files = file_module.File.read_files


def read_files(self, reader, files):
    if not isinstance(reader, CachedNodes):
        return _original_read_files(self, reader, files)
    for node in files:
        stream = open(os.path.join(reader.folder, _safe(node.path)), "rb")
        # Mapped, not streamed: parsing the 733,581 object table of asset_references takes a quarter
        # less time through the memory reader, pages load on demand and a file-backed
        # mapping uses no commit. Each cached inner file starts at 0.
        if os.fstat(stream.fileno()).st_size:
            node_reader = EndianBinaryReader(memoryview(mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ)))
            stream.close()
        else:
            node_reader = EndianBinaryReader(stream)
        f = ImportHelper.parse_file(node_reader, self, node.path, is_dependency=self.is_dependency)
        if isinstance(f, (EndianBinaryReader, file_module.SerializedFile.SerializedFile)) and self.environment:
            self.environment.register_cab(node.path, f)
        f.flags = getattr(node, "flags", 0)
        self.files[node.path] = f


if CACHE:
    bundle_module.BundleFile.read_fs = read_fs
    file_module.File.read_files = read_files


# ---- object tables in one read ---------------------------------------------------------------
# SerializedFile.__init__ reads its object table one field at a time: about ten reader calls and an
# enum lookup per object, ~8 us each. asset_references holds ~880k objects, so every process that
# opens it spent ~7 s there (2/3 of a typical effects job). Format 22+ records (no big ids) are a
# fixed 24 bytes after one alignment: read them in one go. This is UnityPy 1.25.4's constructor
# (checked above) with only that loop replaced; the objects are the same ObjectReaders.
import struct as _struct  # noqa: E402

serialized_module = sys.modules["UnityPy.files.SerializedFile"]
from UnityPy.files.ObjectReader import ObjectReader as _ObjectReader  # noqa: E402
from UnityPy.enums import ClassIDType as _ClassIDType  # noqa: E402


def _read_objects(self, reader):
    header = self.header
    object_count = reader.read_int()
    objects = {}
    if header.version >= 22 and not self.big_id_enabled and object_count:
        reader.align_stream()
        types = self.types
        kinds = {}
        offset = header.data_offset
        new = _ObjectReader
        for path_id, byte_start, byte_size, type_id in _struct.iter_unpack(
                header.endian + "qqIi", reader.read_bytes(24 * object_count)):
            typ = types[type_id]
            kind = kinds.get(type_id)
            if kind is None:
                kind = kinds[type_id] = _ClassIDType(typ.class_id)
            objects[path_id] = new(self, reader, path_id, type_id, typ, typ.class_id, kind,
                                   byte_start + offset, byte_size, None, None)
    else:
        for _ in range(object_count):
            obj = _ObjectReader.from_reader(self, reader)
            objects[obj.path_id] = obj
    return objects


def _serialized_init(self, reader, parent=None, name=None, **kwargs):
    m = serialized_module
    file_module.File.__init__(self, parent=parent, name=name, **kwargs)
    self.reader = reader
    self.unity_version = "2.5.0f5"
    self.target_platform = m.BuildTarget.UnknownPlatform
    self._enable_type_tree = True
    self.types = []
    self.script_types = []
    self.externals = []
    self.objects = {}
    self._cache = {}
    self.unknown = 0
    header = m.SerializedFileHeader(reader)
    self.header = header
    if header.version >= 9:
        header.endian = ">" if reader.read_boolean() else "<"
        header.reserved = reader.read_bytes(3)
        if header.version >= 22:
            header.metadata_size = reader.read_u_int()
            header.file_size = reader.read_long()
            header.data_offset = reader.read_long()
            self.unknown = reader.read_long()
    else:
        reader.Position = header.file_size - header.metadata_size
        header.endian = ">" if reader.read_boolean() else "<"
    reader.endian = header.endian
    if header.version >= 7:
        self.set_version(reader.read_string_to_null())
    if header.version >= 8:
        self._m_target_platform = reader.read_int()
        self.target_platform = m.BuildTarget(self._m_target_platform)
    if header.version >= 13:
        self._enable_type_tree = reader.read_boolean()
    type_count = reader.read_int()
    self.types = [m.SerializedType(reader, self, False) for _ in range(type_count)]
    self.big_id_enabled = 0
    if 7 <= header.version < 14:
        self.big_id_enabled = reader.read_int()
    self.objects = _read_objects(self, reader)
    if header.version >= 11:
        script_count = reader.read_int()
        self.script_types = [m.LocalSerializedObjectIdentifier(header, reader) for _ in range(script_count)]
    externals_count = reader.read_int()
    self.externals = [m.FileIdentifier(header, reader) for _ in range(externals_count)]
    if header.version >= 20:
        ref_type_count = reader.read_int()
        self.ref_types = [m.SerializedType(reader, self, True) for _ in range(ref_type_count)]
    if m.config.SERIALIZED_FILE_PARSE_TYPETREE is False:
        self._enable_type_tree = False
    if header.version >= 5:
        self.userInformation = reader.read_string_to_null()
    for obj in self.objects.values():
        if obj.type == m.ClassIDType.AssetBundle:
            self.assetbundle = obj.parse_as_object()
            self._container = m.ContainerHelper(self.assetbundle)
            break
    else:
        self.assetbundle = None
        self._container = m.ContainerHelper([])


_original_serialized_init = serialized_module.SerializedFile.__init__
serialized_module.SerializedFile.__init__ = _serialized_init


# ---- one parse per distinct type tree --------------------------------------------------------
# Every serialized file carries its own type trees, and a scene bundle repeats the same few hundred
# (ui_scenes_all: 7469 trees in 340 files, ~1M nodes, 2/3 of its load time). Parsed trees are only
# read afterwards (UnityPy and these tools never modify them), so identical blobs share one parse.
_tree_module = sys.modules["UnityPy.helpers.TypeTreeNode"]
_TypeTreeNode = _tree_module.TypeTreeNode
_original_parse_blob = _TypeTreeNode.parse_blob.__func__
_parsed_trees = {}


def _parse_blob(cls, reader, version):
    start = reader.Position
    head = 8 if version >= 23 else 0
    reader.Position = start + head
    node_count = reader.read_int()
    strings = reader.read_int()
    node_struct, _ = _tree_module._get_blob_node_struct(reader.endian, version)
    reader.Position = start
    key = (cls, reader.endian, version, reader.read_bytes(head + 8 + node_struct.size * node_count + strings))
    tree = _parsed_trees.get(key)
    if tree is None:
        reader.Position = start
        tree = _parsed_trees[key] = _original_parse_blob(cls, reader, version)
    return tree


_TypeTreeNode.parse_blob = classmethod(_parse_blob)
