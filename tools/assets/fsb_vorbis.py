"""The game's FSB5 Vorbis audio as standard Ogg Vorbis files, without re-encoding.

FSB5 keeps each clip's Vorbis audio packets but drops the three Vorbis headers: the setup
header is replaced by the CRC32 of the one FMOD's encoder used, and FMOD looks it up in a
table compiled into its library. BTD6's UnityPlayer.dll contains that table (FMOD is built
into Unity): 32-byte entries {u64 header VA, u32 size, u32 crc32, u64 base VA, u32 offset,
u32 count}. Entries without a base point at a complete header; the others are a base header
with `count` bytes replaced at `offset`, read from the entry's pointer. Every rebuilt header is
checked against its CRC.

ogg_bytes() then writes the identification and comment headers (FMOD's encoder uses 256 /
2048 sample blocks), the looked-up setup header and the original packets into Ogg pages. The
granule positions follow the Vorbis block sizes, and the last page ends at the FSB's sample
count, so decoders trim the padding FMOD trims. The audio packets are copied byte for byte.
"""
import struct
import zlib

ENTRY = struct.Struct("<QIIQII")


def _sections(dll):
    pe = struct.unpack_from("<I", dll, 0x3C)[0]
    count, optional = struct.unpack_from("<H", dll, pe + 6)[0], struct.unpack_from("<H", dll, pe + 20)[0]
    image = struct.unpack_from("<Q", dll, pe + 24 + 24)[0]
    sections = []
    for k in range(count):
        size, va, raw_size, raw = struct.unpack_from("<IIII", dll, pe + 24 + optional + 40 * k + 8)
        sections.append((va, max(size, raw_size), raw))
    return image, sections


def setup_headers(dll):
    """{crc32: setup header bytes} from FMOD's table in a game DLL (UnityPlayer.dll)."""
    image, sections = _sections(dll)
    to_va = {}

    def offset(va):
        r = va - image
        for start, size, raw in sections:
            if start <= r < start + size:
                return raw + r - start
        return None

    def header(e):
        if e < 0 or e + ENTRY.size > len(dll):
            return None
        ptr, size, crc, base, at, count = ENTRY.unpack_from(dll, e)
        if not 0 < size < 1 << 16:
            return None
        p = offset(ptr)
        if p is None:
            return None
        if base:
            b = offset(base)
            if b is None or at + count > size:
                return None
            data = dll[b:b + at] + dll[p:p + count] + dll[b + at + count:b + size]
        else:
            data = dll[p:p + size]
        return (crc, data) if data[:7] == b"\x05vorbis" and zlib.crc32(data) == crc else None

    # Anchor on a complete header: a pointer to one of the embedded "\x05vorbis" blobs.
    for i in _find_all(dll, b"\x05vorbis"):
        for start, size, raw in sections:
            if raw <= i < raw + size:
                to_va[struct.pack("<Q", image + start + i - raw)] = i
    anchor = None
    for packed in to_va:
        for e in _find_all(dll, packed):
            if header(e) is not None:
                anchor = e
                break
        if anchor is not None:
            break
    if anchor is None:
        raise ValueError("no FMOD Vorbis setup header table found")
    table = {}
    e = anchor
    while header(e - ENTRY.size) is not None:
        e -= ENTRY.size
    while (h := header(e)) is not None:
        table[h[0]] = h[1]
        e += ENTRY.size
    return table


def _find_all(data, needle):
    i = data.find(needle)
    while i >= 0:
        yield i
        i = data.find(needle, i + 1)


def fsb5_samples(data):
    """[(frequency, channels, sample count, setup crc32, [packets])] of an FSB5 Vorbis bank."""
    if data[:4] != b"FSB5":
        raise ValueError("not an FSB5 bank")
    version, count, header_size, names_size, data_size, mode = struct.unpack_from("<6I", data, 4)
    if mode != 15:
        raise ValueError(f"FSB5 mode {mode} is not Vorbis")
    pos = 0x3C if version == 1 else 0x40
    base = pos + header_size + names_size
    rates = {1: 8000, 2: 11000, 3: 11025, 4: 16000, 5: 22050, 6: 24000, 7: 32000, 8: 44100, 9: 48000}
    headers = []
    for _ in range(count):
        raw, = struct.unpack_from("<Q", data, pos)
        pos += 8
        more, chunks = raw & 1, {}
        while more:
            c, = struct.unpack_from("<I", data, pos)
            more, size, kind = c & 1, (c >> 1) & 0xFFFFFF, c >> 25
            chunks[kind] = data[pos + 4:pos + 4 + size]
            pos += 4 + size
        rate = rates[(raw >> 1) & 0xF]
        channels = ((raw >> 5) & 1) + 1
        if 2 in chunks:
            channels = chunks[2][0]
        if 4 in chunks:
            rate, = struct.unpack_from("<I", chunks[4])
        headers.append((((raw >> 6) & 0xFFFFFFF) * 32, raw >> 34, rate, channels, struct.unpack_from("<I", chunks[11])[0]))
    out = []
    for k, (start, samples, rate, channels, crc) in enumerate(headers):
        end = headers[k + 1][0] if k + 1 < len(headers) else data_size
        blob, packets, p = data[base + start:base + end], [], 0
        while p + 2 <= len(blob):
            size, = struct.unpack_from("<H", blob, p)
            if size == 0:
                break
            packets.append(blob[p + 2:p + 2 + size])
            p += 2 + size
        out.append((rate, channels, samples, crc, packets))
    return out


def _mode_blockflags(setup):
    """Each mode's block flag, read backwards from the end of the setup header (as FFmpeg's
    vorbis_parser does): modes are its last fields, then the framing bit."""
    bits = len(setup) * 8

    def read(at, width):
        value = 0
        for k in range(width):
            b = at + k
            value |= ((setup[b >> 3] >> (b & 7)) & 1) << k
        return value

    end = bits - 1
    while end >= 0 and not read(end, 1):
        end -= 1
    flags = []  # last mode first
    at = end
    while at - 41 - 6 >= 0 and len(flags) < 64:
        mapping, transform, window = read(at - 8, 8), read(at - 24, 16), read(at - 40, 16)
        if mapping > 63 or transform or window:
            break
        flags.append(read(at - 41, 1))
        at -= 41
    # The longest run preceded by a matching 6-bit mode count.
    for n in range(len(flags), 0, -1):
        start = end - 41 * n
        if start >= 6 and read(start - 6, 6) == n - 1:
            return list(reversed(flags[:n]))
    raise ValueError("could not read the Vorbis modes")


_CRC = []
for _i in range(256):
    _r = _i << 24
    for _ in range(8):
        _r = ((_r << 1) ^ 0x04C11DB7) if _r & 0x80000000 else _r << 1
    _CRC.append(_r & 0xFFFFFFFF)


def _page_crc(data):
    crc = 0
    for byte in data:
        crc = ((crc << 8) & 0xFFFFFFFF) ^ _CRC[((crc >> 24) & 0xFF) ^ byte]
    return crc


class _Writer:
    """Ogg pages: packets are laced into pages of up to ~4 KB; a page's granule position is
    that of the last packet completed on it (-1 if none completes there)."""
    def __init__(self, serial):
        self.out, self.serial, self.sequence = bytearray(), serial, 0
        self.lacing, self.body, self.granule, self.first = [], bytearray(), -1, True

    def packet(self, data, granule, flush=False, last=False):
        rest, pos = len(data), 0
        while True:
            take = min(255, rest)
            self.lacing.append(take)
            self.body += data[pos:pos + take]
            pos += take
            rest -= take
            if take < 255:
                break
            if len(self.lacing) == 255:
                self.page(continued_next=True)
        self.granule = granule
        if flush or last or len(self.body) >= 4096 or len(self.lacing) > 240:
            self.page(last=last)

    def page(self, continued_next=False, last=False):
        if not self.lacing:
            return
        flags = (2 if self.first else 0) | (4 if last else 0) | (1 if getattr(self, "continued", False) else 0)
        header = bytearray(b"OggS\x00") + bytes([flags])
        header += struct.pack("<qIII", -1 if continued_next else self.granule, self.serial, self.sequence, 0)
        header += bytes([len(self.lacing)]) + bytes(self.lacing)
        page = header + self.body
        struct.pack_into("<I", page, 22, _page_crc(page))
        self.out += page
        self.sequence += 1
        self.first, self.continued = False, continued_next
        self.lacing, self.body = [], bytearray()


def ogg_bytes(rate, channels, samples, setup, packets, serial=1):
    """A standard Ogg Vorbis file of one FSB5 sample, its audio packets unchanged."""
    ident = (b"\x01vorbis" + struct.pack("<IBIiii", 0, channels, rate, 0, 0, 0) + bytes([0x8 | (0xB << 4)]) + b"\x01")
    comment = b"\x03vorbis" + struct.pack("<I", 7) + b"RustTD6" + struct.pack("<I", 0) + b"\x01"
    flags = _mode_blockflags(setup)
    mode_bits = max(0, (len(flags) - 1).bit_length())
    w = _Writer(serial)
    # The identification header alone on the first page, the other two on the next.
    w.packet(ident, 0, flush=True)
    w.packet(comment, 0)
    w.packet(setup, 0, flush=True)
    granule, previous = 0, None
    for k, packet in enumerate(packets):
        mode = (packet[0] >> 1) & ((1 << mode_bits) - 1) if packet else 0
        size = 2048 if flags[mode] else 256
        if previous is not None:
            granule += (previous + size) // 4
        previous = size
        last = k == len(packets) - 1
        # The final packet gets a page of its own: lewton (quad-snd) trims the end padding
        # only when the last page is not also the first audio page.
        w.packet(packet, samples if last else min(granule, samples), flush=k == len(packets) - 2, last=last)
    return bytes(w.out)
