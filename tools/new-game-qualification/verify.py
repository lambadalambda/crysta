"""Check this exact fresh-boot witness; no claim of general movement support."""
import csv
import hashlib
import json
from pathlib import Path
import sys


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def verify_control(rows):
    # Completed frames. Release has one final delayed movement step.
    expected = {
        6800: (304, 112, 0, 0x84A2A3),
        6801: (304, 112, 0x100, 0x84A2A3),
        6802: (304, 112, 0x100, 0x84A385),
        6820: (331, 112, 0x100, 0x84A385),
        6821: (332, 112, 0, 0x84A385),
        6823: (332, 112, 0, 0x84A258),
        6900: (332, 112, 0, 0x84A258),
        6901: (332, 112, 0x400, 0x84A258),
        6902: (332, 112, 0x400, 0x84A351),
        6920: (332, 139, 0x400, 0x84A351),
        6921: (332, 140, 0, 0x84A351),
        6923: (332, 140, 0, 0x84A258),
        7000: (332, 140, 0, 0x84A258),
        7100: (332, 140, 0, 0x84A258),
    }
    for frame, values in expected.items():
        require(frame in rows, f'missing control sample {frame}')
        actual = tuple(rows[frame][key] for key in ('x', 'y', 'joy', 'resume'))
        require(actual == values, f'control sample {frame}: {actual} != {values}')
    for frame, row in rows.items():
        if frame < 6800:
            continue
        for field, value in dict(map=15, state=0xAA, flags8=0, intro=1, gate=0,
                                 gate2=0, aux=0, player_index=0x1000, controller_index=0x11C0,
                                 flags=0x414 if frame <= 6801 else 0x415).items():
            require(row[field] == value, f'control ownership {frame}: {field}')
        if 6823 <= frame <= 6900 or 6923 <= frame <= 7100:
            y = 112 if frame <= 6900 else 140
            require((row['x'], row['y'], row['joy'], row['resume']) ==
                    (332, y, 0, 0x84A258), f'neutral stability {frame}')


def verify(directory):
    directory = Path(directory)
    reference = json.loads(Path(__file__).with_name('checkpoints.json').read_text())
    actual = json.loads((directory / 'checkpoints.json').read_text())
    require(actual == reference, f'{directory}: checkpoint metadata/hash mismatch')
    raw = (directory / 'frames.csv').read_bytes()
    require(hashlib.sha256(raw).hexdigest() == reference['frames_sha256'],
            f'{directory}: per-frame stream hash mismatch')
    rows = {}
    for row in csv.DictReader(raw.decode().splitlines()):
        frame = int(row.pop('frame'))
        require(frame == len(rows) + 1, 'missing, reordered or repeated frame')
        rows[frame] = {key: int(value) for key, value in row.items()}
    require(len(rows) == 7100, 'incomplete fresh startup')
    verify_control(rows)
    print(f'{directory}: fresh startup + Right/Down/release control witness qualified')


if __name__ == '__main__':
    for directory in sys.argv[1:]:
        verify(directory)
    require(len(sys.argv) > 1, 'usage: verify.py CAPTURE_DIRECTORY [...]')
