"""Bounded JP semantic source projection, not a general actor or text interpreter."""
import hashlib
from pathlib import Path

ROM_SHA = 'f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'

def require(ok, message):
    if not ok:
        raise ValueError(message)

def sha(data):
    return hashlib.sha256(data).hexdigest()

def u(data, at, size=2):
    require(0 <= at and at + size <= len(data), 'truncated source')
    return int.from_bytes(data[at:at + size], 'little')

def project(rom):
    def b(address, count):
        at = address & 0x3fffff
        require(at + count <= len(rom), 'truncated source')
        return rom[at:at + count]
    def expect(address, data):
        require(b(address, len(data)) == data, f'changed source at {address:06x}')
    def request(address):
        expect(address, bytes([2, 0x1b]))
        return (address & 0xff0000) | int.from_bytes(b(address + 2, 2), 'little')
    def choice(address, count):
        expect(address, bytes([2, 0x1a]))
        catalog = b(address + 2, 1)[0]
        table = (address & 0xff0000) | int.from_bytes(b(address + 3, 2), 'little')
        targets = [(address & 0xff0000) | int.from_bytes(b(table + i * 2, 2), 'little') for i in range(count + 1)]
        return {'source': address, 'catalog': catalog, 'table': table, 'cancel_then_options': targets}
    expect(0x838b96, bytes([1, 7, 7, 0, 0x4b, 0x8e, 0x88]))
    expect(0x888e5c, bytes([2, 0x21, 0xde, 0x8e]))
    expect(0x888f06, bytes([2, 0x1f, 2, 7, 0x26, 0x80]))
    expect(0x888efc, bytes([2, 8, 0x26, 0x80, 0x23, 0x8f]))
    expect(0x88a9b4, bytes([2, 0x48, 0x26, 0x80, 2, 0x3b, 2, 0xbc, 0x6b]))
    expect(0x838cc8, bytes([0xfd, 7, 0x2d, 0, 0xaf, 0xa9, 0x88]))
    exit_at = 0x818dfe
    raw = b(exit_at, 12)
    require(raw[:4] == bytes([7, 44, 1, 4]), 'changed D exit rectangle')
    ranges = [
        (0x838b96, 0x838ba0), (0x888e4b, 0x888eda),
        (0x888ede, 0x888f42), (0x838cc8, 0x838ccf), (0x88a9af, 0x88a9bd),
        (0x808b85, 0x808cc8), (0x808cc8, 0x808cd8),
        (0x8096cb, 0x8096e6), (0x80aaa5, 0x80aab3),
        (0x80906f, 0x8090ad), (0x87923f, 0x87941c), (0x87c783, 0x87c7f1),
        (0x859f28, 0x859fe7), (0x92c259, 0x92c25d), (0x92c271, 0x92c299),
        (0x818dfe, 0x818e0a), (0x8389b8, 0x8389bf),
        (0x8d8797, 0x8d884c), (0x8d88ad, 0x8d89a9), (0x84b94d, 0x84b9a5),
    ]
    return {
        'rom_sha256': ROM_SHA,
        'resident': {'source': 0x838b96, 'position': [120, 112], 'callback': 0x888ede,
                     'entry_request': request(0x888e6e)},
        'first': {'request': request(0x888f02), 'set_flag_source': 0x888f08, 'flag': 0x26,
                  'choice': choice(0x888f0c, 2),
                  'first_option_followup': request(0x888e8a),
                  'second_or_cancel_followup': request(0x888e9a)},
        'repeat': {'request': request(0x888f23), 'choice': choice(0x888f29, 2),
                   'first_option_followup': request(0x888f34),
                   'second_or_cancel_followup': request(0x888f3b)},
        'gate': {'source': 0x838cc8, 'position': [120, 720], 'entry': 0x88a9b4,
                 'flag': 0x26, 'occupancy_cell': 1415, 'saved_continuation': 0x88a9bc,
                 'policy': 'reject-if-set-at-load; otherwise stamp-and-return-forever'},
        'exit': {'source': exit_at, 'rectangle': list(raw[:4]), 'destination': u(raw, 4),
                 'mode': raw[6], 'selector': raw[7], 'raw_position': [u(raw, 8), u(raw, 10)],
                 'destination_player_record': 0x8389b8},
        'ranges': [{'start': a, 'end': z, 'sha256': sha(b(a, z - a))} for a, z in ranges],
    }

if __name__ == '__main__':
    import json
    import sys
    data = Path(sys.argv[1]).read_bytes()
    require(sha(data) == ROM_SHA, 'owned Japanese ROM authentication')
    print(json.dumps(project(data), indent=2))
