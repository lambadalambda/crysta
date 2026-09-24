#!/usr/bin/env python3
"""ROM-free checks for the JP->EU address mapper."""
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import addresses  # noqa: E402
import lz  # noqa: E402
import map as eumap  # noqa: E402


def pattern(seed, length):
    """Deterministic bytes with no long repeats."""
    out, x = bytearray(), seed
    for _ in range(length):
        x = (x * 1103515245 + 12345) & 0x7FFFFFFF
        out.append((x >> 16) & 0xFF)
    return bytes(out)


class Addresses(unittest.TestCase):
    def test_runtime_and_normalized_forms_reduce_to_one_offset(self):
        self.assertEqual(addresses.classify(0x88_9AE9), 0x08_9AE9)
        self.assertEqual(addresses.classify(0xCB_7A98), 0x0B_7A98)
        self.assertEqual(addresses.classify(0x1B_8000), 0x1B_8000)

    def test_masks_and_ram_are_not_addresses(self):
        for value in (0x3F_FFFF, 0xFF_0000, 0x40_0000, 0x7E_46E6, 0x7F_201C):
            self.assertIsNone(addresses.classify(value))

    def test_scan_skips_sizes_and_marks_tests(self):
        with tempfile.TemporaryDirectory() as root:
            folder = os.path.join(root, 'crates', 'assets', 'src')
            os.makedirs(folder)
            with open(os.path.join(folder, 'a.rs'), 'w', encoding='utf-8') as handle:
                handle.write('const T: usize = 0x06_959C;\n'
                             'const B: usize = 0x03_0000;\n'
                             'let n = 0x1_0000 - (at & 0xffff);\n'
                             '#[cfg(test)]\nmod tests {\n'
                             '    let image = vec![0; 0x17_0000];\n'
                             '    let x = 0x92_C5E7;\n}\n')
            found = addresses.scan(root)
        self.assertEqual(sorted(found), [0x03_0000, 0x06_959C, 0x12_C5E7])
        self.assertFalse(found[0x06_959C][0][2])
        self.assertTrue(found[0x12_C5E7][0][2])


class Packets(unittest.TestCase):
    def test_decodes_literals_copies_and_terminator(self):
        # Header 00, length 6, first byte 'a'. Control C5 = 1,1 (two
        # literals), 0,0,0,1 (short copy of 3 at distance 256 - FE), then
        # 0,1 with word 0000 and extension 00: the terminator.
        packet = bytes([0x00, 0x06, 0x00, ord('a'), 0xC5, ord('b'), ord('c'),
                        0xFE, 0x00, 0x00, 0x00])
        data, consumed = lz.decode(packet, 0)
        self.assertEqual(data, b'abcbcb')
        self.assertEqual(consumed, len(packet))

    def test_rejects_other_headers_and_wrong_lengths(self):
        self.assertIsNone(lz.decode(bytes([0x01, 0x06, 0x00, 0x61]), 0))
        self.assertIsNone(lz.decode(bytes([0x00, 0x09, 0x00, 0x61, 0x40, 0x00, 0x00, 0x00]), 0))

    def test_find_locates_a_packet_by_its_bytes(self):
        packet = bytes([0x00, 0x06, 0x00, ord('a'), 0xC5, ord('b'), ord('c'),
                        0xFE, 0x00, 0x00, 0x00])
        image = pattern(1, 300) + packet + pattern(2, 50)
        self.assertEqual(lz.find(image, b'abcbcb'), [300])


class Alignment(unittest.TestCase):
    def setUp(self):
        head, tail = pattern(3, 400), pattern(4, 400)
        # JP: head, a 2-byte pointer, tail; EU: 64 inserted bytes, head,
        # the pointer rewritten, tail, so the rewrite keeps its length.
        self.jp = head + b'\x10\x20' + tail
        self.eu = pattern(5, 64) + head + b'\x30\x40' + tail
        self.aligner = eumap.Aligner(self.jp, self.eu)

    def test_identical_runs_map_exactly(self):
        position, method, _ = self.aligner.position(100)
        self.assertEqual((position, method), (164, 'exact'))

    def test_rewritten_bytes_between_anchors_map_by_position(self):
        position, method, _ = self.aligner.position(401)
        self.assertEqual((position, method), (465, 'aligned'))

    def test_a_gap_of_another_length_is_only_an_estimate(self):
        jp = self.jp
        eu = pattern(5, 64) + jp[:400] + b'\x30\x40\x50' + jp[402:]
        position, method, _ = eumap.Aligner(jp, eu).position(401)
        self.assertIsNone(method)


class Shapes(unittest.TestCase):
    def test_masks_absolute_operands_but_keeps_immediates(self):
        a = bytes.fromhex('A9 01 00 8D 34 12 22 00 80 88 E2 20 60')
        b = bytes.fromhex('A9 01 00 8D 78 56 22 10 90 89 E2 20 60')
        c = bytes.fromhex('A9 02 00 8D 34 12 22 00 80 88 E2 20 60')
        self.assertEqual(eumap.shape(a, 0), eumap.shape(b, 0))
        self.assertNotEqual(eumap.shape(a, 0), eumap.shape(c, 0))

    def test_cop_commands_skip_their_operands(self):
        a = bytes.fromhex('02 1B F0 8F 02 1F 02 07 26 80 02 1B 5A 90 6B')
        b = bytes.fromhex('02 1B C6 91 02 1F 02 07 26 80 02 1B 5C 92 6B')
        self.assertEqual(eumap.shape(a, 0), eumap.shape(b, 0))


class References(unittest.TestCase):
    def test_script_and_opcode_operands_are_code_the_rest_tables(self):
        jp = bytes.fromhex('00 02 1B F0 8F 00 BF 47 C4 92 00 55 F0 8F')
        pointers = eumap.Pointers.__new__(eumap.Pointers)
        pointers.jp, pointers.eu = jp, jp
        self.assertEqual(pointers.context(3, 3, eumap.WORD_OPERAND), 'code')
        self.assertEqual(pointers.context(7, 7, eumap.LONG_OPERAND), 'code')
        self.assertEqual(pointers.context(12, 12, eumap.WORD_OPERAND), 'table')

    def test_a_tie_is_no_verdict(self):
        from collections import Counter
        self.assertEqual(eumap.verdict(Counter({1: 2, 2: 2})), (None, 0))
        self.assertEqual(eumap.verdict(Counter({1: 3, 2: 2})), (1, 3))


if __name__ == '__main__':
    unittest.main()
