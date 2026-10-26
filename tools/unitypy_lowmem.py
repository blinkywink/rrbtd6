"""Bounded-memory loading of large UnityFS bundles for UnityPy 1.25.4.

UnityPy joins every decompressed block of a bundle into one bytes object and then copies each
inner file out of it again; asset_references_assets_all (844 MB, ~4.3 GB unpacked) needs ~9 GB of
RAM that way. For bundles above THRESHOLD this streams the blocks once into per-file caches under
$RRBTD6_BUNDLE_CACHE and parses each inner file from disk (UnityPy reads it lazily).

Activated in every tools/ process by sitecustomize.py when RRBTD6_BUNDLE_CACHE is set
(tools/extract_all.py sets it). The cache is game data: it lives under local_extracted/.
"""
import os
import sys

import UnityPy
import UnityPy.files
from UnityPy.enums import ArchiveFlags, ArchiveFlagsOld
from UnityPy.helpers import ArchiveStorageManager, ImportHelper
from UnityPy.streams import EndianBinaryReader

bundle_module = sys.modules["UnityPy.files.BundleFile"]
file_module = sys.modules["UnityPy.files.File"]
if UnityPy.__version__ != "1.25.4":
    raise ImportError(f"unitypy_lowmem targets UnityPy 1.25.4, found {UnityPy.__version__}")

THRESHOLD = 512 << 20
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
        # A stream reader seeks by BaseOffset; each cached inner file starts at 0.
        node_reader = EndianBinaryReader(stream)
        f = ImportHelper.parse_file(node_reader, self, node.path, is_dependency=self.is_dependency)
        if isinstance(f, (EndianBinaryReader, file_module.SerializedFile.SerializedFile)) and self.environment:
            self.environment.register_cab(node.path, f)
        f.flags = getattr(node, "flags", 0)
        self.files[node.path] = f


if CACHE:
    bundle_module.BundleFile.read_fs = read_fs
    file_module.File.read_files = read_files
