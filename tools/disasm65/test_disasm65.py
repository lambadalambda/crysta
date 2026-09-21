#!/usr/bin/env python3
"""ROM-free checks for the linear 65C816 disassembler."""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import disasm65 as D  # noqa: E402


def image(at, code):
    out = bytearray(at + len(code) + 8)
    out[at:at + len(code)] = code
    return bytes(out)


class Decode(unittest.TestCase):
    def test_immediates_follow_the_register_widths(self):
        img = image(0x8000, bytes.fromhex("A9 34 12 E2 20 A9 34 C2 20 A9 34 12"))
        lines = D.listing(img, 0x8000, 0x800C)
        self.assertEqual([l.rsplit("  ", 1)[1].strip() for l in lines],
                         ["LDA #$1234", "SEP #$20", "LDA #$34", "REP #$20", "LDA #$1234"])

    def test_index_immediates_follow_x_not_m(self):
        img = image(0x8000, bytes.fromhex("E2 10 A2 05 A9 34 12"))
        lines = D.listing(img, 0x8000, 0x8007)
        self.assertIn("LDX #$05", lines[1])
        self.assertIn("LDA #$1234", lines[2])

    def test_branches_resolve_to_runtime_addresses(self):
        img = image(0x9AC7, bytes.fromhex("D0 07 80 F7 82 FD FF"))
        lines = D.listing(img, 0x9AC7, 0x9ACE)
        self.assertIn("BNE $80:9AD0", lines[0])
        self.assertIn("BRA $80:9AC2", lines[1])
        self.assertIn("BRL $80:9ACB", lines[2])

    def test_addressing_modes_render(self):
        cases = {
            "BD 04 00": "LDA $0004,X", "A7 36": "LDA [$36]", "B7 36": "LDA [$36],Y",
            "7C B2 83": "JMP ($83B2,X)", "22 75 ED 80": "JSL $80:ED75", "83 02": "STA $02,S",
            "B3 02": "LDA ($02,S),Y", "54 7E 7F": "MVN $7F,$7E", "F4 52 C7": "PEA $C752",
            "BF 00 A0 7E": "LDA $7E:A000,X", "02 26": "COP #$26", "96 10": "STX $10,Y",
            "DC 00 80": "JML [$8000]", "D4 12": "PEI ($12)",
        }
        for raw, text in cases.items():
            img = image(0x8000, bytes.fromhex(raw))
            self.assertIn(text, D.listing(img, 0x8000, 0x8001)[0], raw)

    def test_address_forms(self):
        self.assertEqual(D.normalized("80:9AC7"), 0x9AC7)
        self.assertEqual(D.normalized("$88:8E50"), 0x08_8E50)
        self.assertEqual(D.normalized("0x9AC7"), 0x9AC7)
        self.assertEqual(D.runtime(0x08_8E50), "$88:8E50")
        self.assertEqual(D.runtime(0x00_0100), "$C0:0100")

    def test_every_opcode_has_a_length(self):
        for opcode in range(256):
            mnemonic, mode = D.TABLE[opcode]
            self.assertIn(mode, D.MODES, opcode)
            self.assertTrue(mnemonic.isupper() and len(mnemonic) == 3, opcode)


if __name__ == "__main__":
    unittest.main()
