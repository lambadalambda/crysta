"""Authenticate narrow pot operands and collision dispatch, without exporting ROM bytes."""
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parent / 'pandora-qualification'))
from source import Source, ROM_SHA, require, sha


def project(data):
    require(sha(data) == ROM_SHA, 'owned Japanese ROM authentication')
    s = Source(data)
    # Map-specific FA/FB records are loaded from this source table, not WRAM.
    s.expect(0x8db8c4, [0xbf, 0xbd, 0xdd, 0x96, 0x8d, 0x8a, 9])
    s.expect(0x8db8d9, [0xbf, 0xc2, 0xdd, 0x96, 0x8d, 0x8f, 9])
    require(s.u(0x96ddc5) == 12, 'map C held records')
    records = 0x960000 | s.u(0x96ddc7)
    require(records == 0x96e1a6, 'map C held record pointer')
    require(s.u(records) == s.u(records + 5) == 0, 'both replacement records select F8 fallback')
    s.expect(0x8796bc, [0x8d, 0x88, 9])
    s.expect(0x84bfe8, [0x9c, 0x88, 9])  # reservation clears on break, not release
    s.expect(0x84c4a5, [0xbd, 2, 0, 0x18, 0x69, 0xf8, 0xff, 0x9d, 2, 0])
    s.expect(0x84c4af, [2, 0xaf, 0x7c])  # Up world offset -8 then flight selector
    s.expect(0x84ace1, [2, 0xcb, 0x20, 0, 0xb5, 0x84])  # carrying control 0020
    # Typed COP checks skip operands correctly (linear CPU disassembly does not).
    carry = []
    for address, sequence in [(0x84b509, 9), (0x84b51a, 10), (0x84b52f, 11)]:
        p = s.cop(address, 0x83)
        require(s.b(p, 2) == bytes([sequence, 1]), 'carry COP83 animation descriptor')
        carry.append({'source': address, 'table': 1, 'sequence': sequence})
    # Type5 matches P16 in all tables. Type29 matches Open in Up's four;
    # Right S-first differs and is retained as a counterexample to broad admission.
    tables = []
    for base in [0x80d542, 0x80d8e8, 0x80dc60, 0x80dfdc]:
        for offset in [0, 64, 128, 192]:
            table = base + offset
            require(s.u(table + 5 * 2) == s.u(table + 16 * 2), 'type5 partial dispatch')
            require((s.u(table + 29 * 2) == s.u(table)) == (table != 0x80e09c),
                    'type29 directional dispatch, including Right S-first counterexample')
            tables.append({'source': table, 'partial5': s.u(table + 10),
                           'type29_target': s.u(table + 58), 'open0_target': s.u(table)})
    for address, kind in [(0x80e21a, 1), (0x80e21f, 9), (0x80e224, 10)]:
        s.expect(address, [0xc9, kind, 0])
    s.expect(0x80e22c, [0x4c, 0xe6, 0xe2])  # type5 falls through to the mask gate
    s.expect(0x80e2e6, [0xad, 0x80, 9, 0x89, 0x50, 0])
    s.expect(0x88ab81, [2, 0x4b, 0x80, 1, 0])
    ranges = [(0x879683, 0x879747), (0x84abcb, 0x84ae30), (0x84b4bf, 0x84b579),
              (0x84bfdf, 0x84c020), (0x84c2b8, 0x84c5cb), (0x84c697, 0x84c7ab), (0x8db8a5, 0x8db915),
              (records, records + 15), (0x88aaee, 0x88ab96), (0x80e1df, 0x80e32e)]
    return {'rom_sha256': ROM_SHA, 'held_records': {'map': 12, 'source': records,
            'slots': [0x98a, 0x98f], 'replacement_tile': 0xf8}, 'carry': carry,
            'collision_tables': tables,
            'ranges': [{'start': a, 'end': b, 'sha256': sha(s.b(a, b - a))} for a, b in ranges]}


if __name__ == '__main__':
    result = project(Path(sys.argv[1]).read_bytes())
    require(result == json.loads((ROOT / 'source.json').read_text()), 'pot source reference mismatch')
    print(json.dumps(result, indent=2))
