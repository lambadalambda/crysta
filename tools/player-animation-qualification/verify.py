"""Authenticate native capture hashes and bounded source metadata; no art decoding."""
import csv
from hashlib import sha256
import json
from pathlib import Path
import sys

ROOT = Path('tools/player-animation-qualification')

def digest(data):
    return sha256(data).hexdigest()

def verify(rom_path, out):
    rom = rom_path.read_bytes()
    if len(rom) % 0x8000 == 512:
        rom = rom[512:]
    pins = json.loads((ROOT/'pins.json').read_text())
    assert digest(rom) == pins['rom_sha256']
    for source in pins['sources']:
        start, end = source['start'], source['end']
        assert digest(rom[start & 0x3fffff:end & 0x3fffff]) == source['sha256'], source['name']
    for name, expected in pins['captures'].items():
        for run in ('a', 'b'):
            path = out / f'{name}-{run}'
            report = json.loads((path/'report.json').read_text())
            assert report == expected['report'], (name, run, 'report')
            assert digest((path/'frames.csv').read_bytes()) == report['frames_sha256']
            assert digest((path/'final.wram').read_bytes()) == expected['final_wram_sha256']
            assert digest((path/'f6800.wram').read_bytes()) == pins['bootstrap_wram_sha256']
            for i, stop in enumerate(report['stops']):
                assert digest((path/f'stop-{i}.wram').read_bytes()) == stop['wram_sha256']
                # Native trace digest is structured, not the human-readable text digest.
                assert digest((path/f'stop-{i}.trace').read_bytes()) == expected['trace_text_sha256'][i]
    # Behavioral counterexamples preserved separately from the pure semantic policy.
    def rows(name):
        return {int(v['frame']): {k: int(n) for k, n in v.items()} for v in csv.DictReader((out/f'{name}-a/frames.csv').open())}
    house, map10, idle = rows('house'), rows('map10'), rows('idle')
    assert (house[6800]['facing'], house[6800]['base'], house[6800]['sequence']) == (0, 0xa5dca6, 33)
    assert house[6801]['base'] == 0xa5dca6 and house[6802]['base'] == 0x9ad064
    assert house[6855]['cursor'] == 6 and house[6856]['cursor'] == 1
    assert house[6855]['x'] == house[6856]['x']  # horizontal restart gap
    assert house[6863]['base'] == 0x9ad064 and house[6864]['base'] == 0xa4a1e4
    assert (house[6864]['cursor'], house[6865]['cursor']) == (1, 0)  # standing record persists after sentinel
    assert len({map10[f]['y'] for f in range(7090, 7202)}) == 1
    assert {map10[f]['cursor'] for f in range(7090, 7202)} == set(range(1, 7))
    assert map10[7381]['input'] == 512 and map10[7382]['input'] == 0
    assert map10[7382]['base'] == 0x9ad064 and map10[7383]['base'] == 0xa4a1e4
    assert map10[7381]['x'] == map10[7382]['x'] == map10[7383]['x']
    assert (idle[6942]['sequence'], idle[7484]['sequence']) == (35, 34)
    print('All ROM/source/capture pins and native ownership counterexamples verified.')

if __name__ == '__main__':
    verify(Path(sys.argv[1]), Path(sys.argv[2]))
