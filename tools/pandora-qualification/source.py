"""Bounded JP Pandora source operands; not an event VM or asset decoder."""
import hashlib
import json
from pathlib import Path
import sys

ROM_SHA = 'f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'
ROOT = Path(__file__).resolve().parent


def require(ok, message):
    if not ok:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


class Source:
    def __init__(self, data):
        self.data = data

    def b(self, address, size):
        at = address & 0x3fffff
        require(0 <= size and at + size <= len(self.data), 'truncated source')
        return self.data[at:at + size]

    def u(self, address, size=2):
        return int.from_bytes(self.b(address, size), 'little')

    def expect(self, address, values):
        require(self.b(address, len(values)) == bytes(values), f'source shape {address:06x}')

    def cop(self, address, selector):
        self.expect(address, [2, selector])
        return address + 2

    def request(self, address):
        return {'source': address, 'request': (address & 0xff0000) | self.u(self.cop(address, 0x1b))}

    def flag(self, address):
        value = self.u(self.cop(address, 7))
        return {'source': address, 'event': value & 0xfff, 'set': bool(value & 0x8000)}

    def choice(self, address):
        p = self.cop(address, 0x1a)
        table = (address & 0xff0000) | self.u(p + 1)
        return {'source': address, 'catalog': self.u(p, 1), 'table': table,
                'cancel_then_options': [(address & 0xff0000) | self.u(table + i * 2) for i in range(3)]}

    def palette(self, address):
        p = self.cop(address, 0x5a)
        return {'source': address, 'pointer': (self.u(p, 1) << 16) | self.u(p + 1),
                'destination': self.u(p + 3, 1), 'count': self.u(p + 4, 1)}

    def transition(self, address):
        p = self.cop(address, 0x14)
        return {'source': address, 'map': self.u(p), 'mode': self.u(p + 2, 1),
                'selector': self.u(p + 3, 1), 'raw_position': [self.u(p + 4), self.u(p + 6)]}

    def exit(self, address):
        return {'source': address, 'rectangle': list(self.b(address, 4)),
                'map': self.u(address + 4), 'mode': self.u(address + 6, 1),
                'selector': self.u(address + 7, 1),
                'raw_position': [self.u(address + 8), self.u(address + 10)]}

    def layer(self, address):
        self.expect(address, [0x10, 1])
        value = self.u(address + 2, 3)
        bank = ((address >> 16) + ((value >> 15) & 255)) & 255
        pointer = (bank << 16) | (value & 0x7fff)
        if address >> 16 < 0xc0 and bank < 0xc0:
            pointer |= 0x8000
        return {'source': address, 'pointer': pointer}


def project(data):
    s = Source(data)
    maps = [0xa, 0xb, 0xc, 0xd, 0xe, 0xf, 0x10, 0x13, 0x20, 0x21, 0x41, 0x42, 0x43, 0x44]
    for m in maps:
        require(s.u(0x828000 + m * 2) == 0, 'nonzero bank82 scene override')
    s.expect(0x838ebe, [1, 22, 8, 0])
    require(s.u(0x838ec2, 3) == 0x88b61e, 'map13 resident header')
    s.expect(0x88b63b, [2, 0x21])
    require(s.u(0x88b63d) == 0xb653, 'map13 callback registration')
    s.expect(0x8d8735, [0x20, 0xed, 0x8a])
    s.expect(0x8d8af9, [0xa2, 0, 0])
    for a, word in [(0x8d8b0b, 0x6c0), (0x8d8b0e, 0x6c2), (0x8d8b11, 0x640)]:
        s.expect(a, [0x8e])
        require(s.u(a + 1) == word, 'room-local reset destination')
    s.expect(0x8796ac, [0xa9])
    s.expect(0x8796b4, [0xa9])
    # Conditional membership, not an invented flag-to-open subscription.
    require(s.u(s.cop(0x88aaee, 0x48)) == 0x0028, 'door requires event28')
    require(s.u(s.cop(0x88aaf2, 0x48)) == 0x8292, 'door rejects opened event292')
    s.expect(0x88ab81, [2, 0x4b, 0x80])
    hit_cases = []
    for a in [0x88ab86, 0x88ab8d]:
        s.expect(a, [2, 0x4a, 0])
        hit_cases.append({'source': a, 'value': s.u(a + 3), 'target': 0x880000 | s.u(a + 5)})
    # Two collision-hit branches and four explicit cell replacements.
    patches = []
    for a in [0x88aba6, 0x88abac, 0x88abd8, 0x88abde]:
        p = s.cop(a, 0x44)
        xy = [int.from_bytes(s.b(p + i, 1), 'little', signed=True) * 16 for i in range(2)]
        word = s.u(p + 2)
        patches.append({'source': a, 'cell_relative_pixels': xy, 'tile': word & 511,
                        'secondary': bool(word & 512), 'delay': word >> 10})
    # The interior palette/graphics setup is a scene controller, NOT a map-script load.
    s.expect(0x89d254, [0xa2])
    s.expect(0x89d257, [0x86, 0x66, 0xa9])
    s.expect(0x89d25c, [0x85, 0x68])
    s.expect(0x89d267, [0x22, 0xe1, 0x84, 0x86])
    graphics = s.u(0x89d255) | (s.u(0x89d25a) << 16)
    palettes = [s.palette(a) for a in range(0x89d26c, 0x89d29d, 7)]
    requests = [0x88b673, 0x88b67f, 0x88b691, 0x88b69c,
                0x889ae9, 0x889afb, 0x889b11, 0x889b24, 0x889b3a, 0x889da9,
                0x88abbc, 0x889b9f, 0x889bc2, 0x889bef, 0x889c10, 0x88a241, 0x88a3d9,
                0x88ad95, 0x88adb7, 0x88ae77, 0x88ae85, 0x88ae93, 0x88aea1,
                0x89d3e0, 0x89d3f4, 0x89d408, 0x89d41c, 0x89d430, 0x89d444,
                0x89d458, 0x89d470, 0x89d48b, 0x89d4a4, 0x89d4ae,
                0x89d4c9, 0x89d4d3, 0x89d4ee, 0x89d4f8]
    flags = [0x88b697, 0x88b6a2, 0x889adb, 0x889daf, 0x88aba2, 0x88abc6,
             0x88abd1, 0x88abee, 0x889ba9, 0x889bcc, 0x889bf5,
             0x889d1f, 0x889d4f, 0x88a252, 0x88a3f5,
             0x88ad69, 0x88adc5, 0x88ad4f, 0x88ae7d, 0x88ae8b, 0x88ae99,
             0x89d508, 0x89d495]
    ranges = [
        (0x838eab, 0x838eea), (0x88b61e, 0x88b6c7),
        (0x838bf0, 0x838c4d), (0x889a6a, 0x889dc3), (0x88aaee, 0x88acd3),
        (0x88a1a4, 0x88a295), (0x88a321, 0x88a420),
        (0x8d8720, 0x8d8770), (0x8d8aed, 0x8d8b30),
        (0x838cfa, 0x838d1e), (0x83923a, 0x8392b4),
        (0x88acfa, 0x88adcb), (0x88ae64, 0x88afa6),
        (0x839527, 0x839637), (0x89d24e, 0x89d50f),
        (0x808669, 0x808720), (0x80963a, 0x8096e6), (0x80bb77, 0x80bbc7),
        (0x808b85, 0x808cc8), (0x859f28, 0x859fe7),
        (0x869010, 0x869164), (0x808fe6, 0x809007), (0x808a23, 0x808a59), (0x80b501, 0x80b530),
        (0x87923f, 0x87941c), (0x87c783, 0x87c8f7),
        (0x879683, 0x8797ca), (0x84b4ad, 0x84b7e3), (0x84c2b8, 0x84c5cb),
        (0x809aeb, 0x809b29), (0x809444, 0x809532), (0x809713, 0x8097c0), (0x809d25, 0x809d72),
        (0x8d8df8, 0x8d8f88), (0x8d8797, 0x8d89a9), (0x84baf9, 0x84bdab),
        (0x98831c, 0x98834e), (0x9884bf, 0x9884dc), (0x9885e9, 0x988602),
    ]
    ranges += [(a, a + n) for m in maps for a, n in
               [(0x828000 + m * 2, 2), (0x838000 + m * 2, 2), (0x86959c + m * 3, 3)]]
    ranges += [(a, a + 12) for a in [0x818d6b, 0x818d8f, 0x818eac, 0x818dcd,
                                    0x818dfe, 0x818e0a, 0x818df1, 0x818e2f, 0x818fc1]]
    ranges += [(0x988458, 0x98845d)]
    return {
        'rom_sha256': ROM_SHA,
        'maps': [{'map': m, 'scene': 0x830000 | s.u(0x838000 + m * 2),
                  'map_script': s.u(0x86959c + m * 3, 3)} for m in maps],
        'exits': [s.exit(a) for a in [0x818d6b, 0x818d8f, 0x818eac, 0x818dcd,
                                     0x818dfe, 0x818e0a, 0x818df1, 0x818e2f, 0x818fc1]],
        'layers': [s.layer(a) for a in [0x9884c6, 0x988458, 0x9885ed, 0x9885f9,
                                       0x98831c, 0x988338, 0x988340, 0x988348]],
        'room_local_reset': {'caller': 0x8d8735, 'entry': 0x8d8aed,
                             'zero_words': [0x6c0, 0x6c2, 0x640]},
        'resident13': {'source': 0x838ebe, 'position': [360, 128], 'callback': 0x88b653,
                       'descriptor': s.u(0x838ec5, 3)},
        'choices': [s.choice(a) for a in [0x88b679, 0x88b685, 0x889b17, 0x889b40]],
        'requests': [s.request(a) for a in requests],
        'effects': [s.flag(a) for a in flags],
        'door': {'source': 0x838c32, 'position': [184, 352], 'entry': 0x88aaee,
                 'hit_callback': 0x88ab81, 'counter': 0x640, 'increment_bcd': s.u(0x88ab84),
                 'hit_cases': hit_cases, 'patches': patches,
                 'admitted_lift_tiles': [0xfa, 0xfb], 'lift_entry': 0x879683,
                 'held_slots': [s.u(0x8796ad), s.u(0x8796b5)]},
        'transitions': [s.transition(a) for a in [0x88ad53, 0x88aeab, 0x89d476,
                                                 0x89d4b4, 0x89d4d9, 0x89d4fe]],
        'interior_setup': {'controller': 0x89d24e, 'graphics': graphics, 'palettes': palettes},
        'ranges': [{'start': a, 'end': z, 'sha256': sha(s.b(a, z - a))} for a, z in ranges],
    }


if __name__ == '__main__':
    data = Path(sys.argv[1]).read_bytes()
    require(sha(data) == ROM_SHA, 'owned Japanese ROM authentication')
    result = project(data)
    if len(sys.argv) == 3 and sys.argv[2] == '--record':
        (ROOT / 'source.json').write_text(json.dumps(result, indent=2) + '\n')
    else:
        require(result == json.loads((ROOT / 'source.json').read_text()), 'source metadata mismatch')
        print('Pandora bounded source operands and source-window hashes verified')
