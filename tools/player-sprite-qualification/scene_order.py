"""Bounded house scene-order evidence; no production compositor."""
import hashlib
import json
from pathlib import Path
import sys

HERE = Path(__file__).parent
ROOT = HERE.parent.parent


def word(data, offset):
    return int.from_bytes(data[offset:offset + 2], 'little')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def check_tilemap(wram, vram, camera_y):
    """Full-word comparison in a fixed interior rectangle of the house ring buffer."""
    require(len(wram) == 0x20000 and len(vram) == 0x10000, 'memory extent')
    require(camera_y in (0, 256), 'unqualified camera')
    require(wram[0x46d:0x46f] == bytes([0x3c, 0x38]), 'house hardware BG assignment')
    high = 0
    for y in range(camera_y // 8 + 2, camera_y // 8 + 26):
        for x in range(34, 62):
            cell = word(wram, 0xa000 + 2 * ((y // 2) * 32 + x // 2)) & 511
            expected = word(wram, 0x2000 + cell * 8 + ((y % 2) * 2 + x % 2) * 2)
            address = 0x3800 + (y % 32) * 32 + x % 32
            require(word(vram, address * 2) == expected, f'tilemap word at {x},{y}')
            high += bool(expected & 0x2000)
    require(0 < high < 672, 'both priorities must be exercised')
    return {'words': 672, 'high': high, 'low': 672 - high}


def check(rom_path, traces, captures):
    """Authenticate native writer stops and already-qualified fresh pose captures."""
    reference = json.loads((HERE / 'scene-order-reference.json').read_text())
    rom = Path(rom_path).read_bytes()
    require(digest(rom) == reference['rom_sha256'], 'ROM identity')
    for source in reference['rom_sources']:
        require(digest(rom[source['start']:source['end']]) == source['sha256'], 'ROM source pin')
    for source in reference['ares_sources']:
        require(digest((ROOT / source['path']).read_bytes()) == source['sha256'], 'ares source pin')
    for name, expected in reference['native'].items():
        directory = Path(traces) / name
        report = json.loads((directory / 'stops.json').read_text())
        require(report == expected, 'native stop metadata')
        require(digest((directory / 'definitions.bin').read_bytes()) == reference['definitions_sha256'], 'ROM-decoded definitions')
        for stop in report['stops']:
            prefix = directory / f"{stop['pc']:06x}"
            wram = prefix.with_suffix('.wram').read_bytes()
            require(digest(wram) == stop['wram_sha256'], 'native WRAM pin')
            require(digest(prefix.with_suffix('.trace-pcs').read_bytes()) == stop['trace_pcs_sha256'], 'native trace pin')
            require(word(wram, 0x47e) == stop['map'], 'native map')
            require(stop['p'] & 0x20 and stop['db'] == 0x81 and stop['x'] == 0xb9, '8-bit absolute writer context')
            require(rom[0x16bb6a + stop['x']] == 9, 'BGMODE source value')
            if stop['pc'] in (0x868d6f, 0x868d72):
                require(stop['a'] & 255 == 9, 'BGMODE write value')
    for run in ('a', 'b'):
        for checkpoint in reference['tilemaps']:
            prefix = Path(captures) / run / f"f{checkpoint['frame']}"
            wram = prefix.with_suffix('.wram').read_bytes()
            vram = prefix.with_suffix('.vram').read_bytes()
            require(digest(wram) == checkpoint['wram_sha256'], 'checkpoint WRAM pin')
            require(digest(vram) == checkpoint['vram_sha256'], 'checkpoint VRAM pin')
            require(digest(wram[0x2000:0x3000]) == reference['definitions_sha256'], 'ROM definitions survive to WRAM')
            require(word(wram, 0x47e) == checkpoint['map'], 'checkpoint map')
            require(wram[0x468] == 0x17, 'BG1/BG2/OBJ main-screen enable shadow')
            require(check_tilemap(wram, vram, checkpoint['camera_y']) == checkpoint['comparison'], 'priority coverage')
    print('Qualified house mode $09; first background is hardware BG2; low < OBJ2 < high.')


if __name__ == '__main__':
    check(*sys.argv[1:])
