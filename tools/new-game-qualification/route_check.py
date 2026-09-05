"""Authenticate the fresh house round trip; no transition/collision simulation."""
import csv
from hashlib import sha256
import json
from pathlib import Path
import sys
from verify import require

# Reuse the independently qualified onset admission rule; do not invent another
# movement/collision model for this capture tool.
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'movement-qualification'))
from input_admission import Admission


def compare_grids(static, runtime):
    require(len(static) == len(runtime) == 4096, '32x64 word grid extent')
    differences = []
    for index in range(2048):
        a = int.from_bytes(static[2 * index:2 * index + 2], 'little')
        b = int.from_bytes(runtime[2 * index:2 * index + 2], 'little')
        require(a == b & 0x7FFF, f'low15 BG1 cell {index}')
        if a != b:
            differences.append(dict(index=index, x=index % 32, y=index // 32, static=a, runtime=b))
    return differences


def verify_route(directory):
    root = Path(directory)
    expected = json.loads(Path(__file__).with_name('route-reference.json').read_text())
    actual = json.loads((root / 'route.json').read_text())
    require(actual == expected, 'route metadata mismatch')
    for name, field in [('frames.csv', 'frames_sha256'), ('runtime-grid.bin', 'runtime_grid_sha256'),
                        ('static-bg1-grid.bin', 'static_bg1_grid_sha256')]:
        require(sha256((root / name).read_bytes()).hexdigest() == expected[field], name)
    differences = compare_grids((root / 'static-bg1-grid.bin').read_bytes(), (root / 'runtime-grid.bin').read_bytes())
    require(differences == expected['grid_differences'], 'runtime overrides')
    for c in actual['checkpoints']:
        memory = (root / f"f{c['frame']}.wram").read_bytes()
        require(sha256(memory).hexdigest() == c['wram_sha256'], 'checkpoint WRAM')
        require(sha256(memory[0xA000:0xB000]).hexdigest() == c['grid_sha256'], 'checkpoint grid')
        require(sha256(memory[0x6C0:0x700]).hexdigest() == c['event_sha256'], 'checkpoint events')
    rows = list(csv.DictReader((root / 'frames.csv').read_text().splitlines()))
    require([int(r['frame']) for r in rows] == list(range(6800, 7461)), 'contiguous 661-row route')
    require((rows[0]['input'], rows[0]['map'], rows[0]['x'], rows[0]['y']) == ('', 'f', '304', '112'), 'fresh boundary')
    admission = Admission()
    changes, previous_map, previous_input, onsets = [], None, '', []
    for row in rows[1:]:
        direction = row['input']
        admission = admission.submit(direction)
        if direction and direction != previous_input:
            onsets.append((int(row['frame']) - 1, direction))
        previous_input = direction
        if row['map'] != previous_map:
            changes.append(row['map'])
            previous_map = row['map']
    require(changes == ['f', '10', 'f'], 'only qualified house rooms')
    require(onsets == [(6800, 'Right'), (6900, 'Down'), (7100, 'Up'), (7260, 'Right'),
                       (7310, 'Left'), (7360, 'Right'), (7410, 'Left')], 'intended normal onsets')
    require(all(b[0] - a[0] >= 11 for a, b in zip(onsets, onsets[1:])), 'conservative onset spacing')
    require((rows[-1]['map'], rows[-1]['x'], rows[-1]['y']) == ('f', '392', '191'), 'returned and settled')
    print(f'{root}: 661 authenticated rows, F -> 10 -> F, 7 normal onsets; exact two-cell high-bit overlay retained')


if __name__ == '__main__':
    if len(sys.argv) < 2:
        raise SystemExit('usage: route_check.py CAPTURE_DIRECTORY [...]')
    for directory in sys.argv[1:]:
        verify_route(directory)
