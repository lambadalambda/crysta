"""Authenticated input-only F -> 10 -> F witness; no reference payloads embedded."""
import csv
from hashlib import sha256
import json
from pathlib import Path
import sys

PINS = json.loads(Path(__file__).with_name('house-return-pins.json').read_text())

def digest(data):
    return sha256(data).hexdigest()

def verify_sources():
    rom = Path('local/Tenchi Souzou (Japan).sfc').read_bytes()
    assert digest(rom) == PINS['rom_sha256'], 'ROM identity'
    assert digest(Path('local/saves/Terranigma.srm').read_bytes()) == PINS['sram_sha256'], 'SRAM identity'
    for source in PINS['sources']:
        at, size = source['offset'], source['length']
        assert digest(rom[at:at+size]) == source['sha256'], source['meaning']

def verify(root):
    raw = (root / 'frames.csv').read_bytes()
    assert digest(raw) == PINS['csv_sha256'], 'frame stream identity'
    rows = list(csv.DictReader(raw.decode().splitlines()))
    assert [int(r['frame']) for r in rows] == list(range(1601, 1951))
    by_frame = {int(r['frame']): r for r in rows}
    for frame, pin in PINS['checkpoints'].items():
        w = (root / f'f{frame}.wram').read_bytes()
        assert len(w) == 131072 and digest(w) == pin['wram_sha256'], frame
        word = lambda at: int.from_bytes(w[at:at+2], 'little')
        fields = dict(map=word(0x47e), x=word(0x1000), y=word(0x1002),
                      flags=word(0x1004), resume=int.from_bytes(w[0x100a:0x100d], 'little'),
                      control_gate=word(0x97c), events_sha256=digest(w[0x6c0:0x700]))
        assert fields == {k: v for k, v in pin.items() if k != 'wram_sha256'}, frame
        row = by_frame[int(frame)]
        assert (int(row['map'], 16), int(row['x']), int(row['y'])) == (pin['map'], pin['x'], pin['y']), frame
    # The final walking step owns 1815; the superficially similar next delta is
    # transition-controlled. Do not feed it to the portable walking comparator.
    for frame in range(1801, 1816):
        assert int(by_frame[frame]['flags'], 16) & 0x1404 == 0x0404
    assert int(by_frame[1816]['flags'], 16) & 0x1000
    for frame in range(1872, 1951):
        row = by_frame[frame]
        assert (row['map'], row['x'], row['y'], row['input']) == ('f', '392', '191', '')
        assert int(row['flags'], 16) & 0x1404 == 0x0404
    print(f'{root}: return doorway ownership, spawn and stable recovery qualified')

if __name__ == '__main__':
    verify_sources()
    if sys.argv[1] != '--sources':
        verify(Path(sys.argv[1]))
