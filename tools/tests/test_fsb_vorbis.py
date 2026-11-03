"""fsb_vorbis.py's Ogg writer with synthetic packets and a synthetic setup header (no game data)."""
from pathlib import Path
import struct
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "assets"))
import fsb_vorbis


def bits_to_bytes(bits):
    out = bytearray((len(bits) + 7) // 8)
    for i, b in enumerate(bits):
        out[i >> 3] |= b << (i & 7)
    return bytes(out)


def setup_with_modes(flags):
    """A setup header whose tail is the Vorbis mode section: count-1, then per mode the
    block flag, 16-bit window type, 16-bit transform type, 8-bit mapping, then framing."""
    bits = [1, 0, 1, 1, 0, 0, 1, 0] * 5  # arbitrary codebook-ish filler
    n = len(flags) - 1
    bits += [(n >> k) & 1 for k in range(6)]
    for f in flags:
        bits += [f] + [0] * 16 + [0] * 16 + [0] * 8
    bits.append(1)
    return b"\x05vorbis" + bits_to_bytes(bits)


def pages(data):
    out, pos = [], 0
    while pos < len(data):
        assert data[pos:pos + 4] == b"OggS"
        flags, granule, serial, seq, crc, count = struct.unpack_from("<BqIIIB", data, pos + 5)
        lacing = data[pos + 27:pos + 27 + count]
        size = 27 + count + sum(lacing)
        page = bytearray(data[pos:pos + size])
        struct.pack_into("<I", page, 22, 0)
        out.append((flags, granule, seq, crc == fsb_vorbis._page_crc(page), list(lacing), bytes(data[pos + 27 + count:pos + size])))
        pos += size
    return out


def packets(page_list):
    out, current = [], bytearray()
    for flags, granule, seq, ok, lacing, body in page_list:
        pos = 0
        for size in lacing:
            current += body[pos:pos + size]
            pos += size
            if size < 255:
                out.append(bytes(current))
                current = bytearray()
    return out


class OggTests(unittest.TestCase):
    def test_modes_read_from_the_end(self):
        self.assertEqual(fsb_vorbis._mode_blockflags(setup_with_modes([0, 1])), [0, 1])
        self.assertEqual(fsb_vorbis._mode_blockflags(setup_with_modes([1, 0, 1, 1])), [1, 0, 1, 1])

    def test_pages_carry_every_packet_unchanged_with_valid_crcs_and_granules(self):
        setup = setup_with_modes([0, 1])
        # Mode bit (after the packet-type bit): 0 short (256), 1 long (2048).
        audio = [bytes([0b10]) + bytes(300), bytes([0b00]) + bytes(40), bytes([0b10]) + bytes(700)] * 20
        audio.append(bytes([0b10]) + bytes(600))
        data = fsb_vorbis.ogg_bytes(44100, 2, 50000, setup, audio)
        ps = pages(data)
        self.assertTrue(all(ok for *_, ok, _, _ in [(p[0], p[1], p[2], p[3], p[4], p[5]) for p in ps]))
        self.assertEqual([p[2] for p in ps], list(range(len(ps))))
        self.assertEqual(ps[0][0] & 2, 2)
        self.assertEqual(ps[-1][0] & 4, 4)
        got = packets(ps)
        self.assertEqual(got[0][:7], b"\x01vorbis")
        self.assertEqual(got[2], setup)
        self.assertEqual(got[3:], audio)
        # The last packet is alone on the last page, which ends at the sample count.
        self.assertEqual(len(ps[-1][4]), 3)  # 600 + 1 bytes: lacing 255, 255, 91
        self.assertEqual(ps[-1][1], 50000)
        granules = [p[1] for p in ps[2:-1] if p[1] != -1]
        self.assertEqual(granules, sorted(granules))

    def test_identification_header(self):
        data = fsb_vorbis.ogg_bytes(48000, 1, 10, setup_with_modes([0]), [b"\x00" * 10, b"\x00" * 10])
        ident = packets(pages(data))[0]
        self.assertEqual(struct.unpack_from("<BIB", ident, 11), (1, 48000, 0))
        self.assertEqual(ident[28], 0x8 | (0xB << 4))


if __name__ == "__main__":
    unittest.main()
