"""Negative controls for the real comparator; never mutate the source capture."""
import csv
import io
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def check(compare: Path, rom: Path, capture: Path) -> None:
    rows = list(csv.reader((capture / 'frames.csv').read_text().splitlines()))
    env = dict(os.environ)
    env.pop('NAIVE_TWO_FRAME', None)

    def run(path, span):
        return subprocess.run([str(compare), str(rom), str(path), span],
                              env=env, capture_output=True, text=True)

    baseline = run(capture, '6800:6967')
    if baseline.returncode:
        raise AssertionError(f'baseline rejected: {baseline.stderr}')
    # Column numbers follow the probe's committed CSV header. Each initial
    # mutation runs only one step, so later movement cannot mask a false pass.
    cases = [
        ('later fidget', 6802, 9, 0xa5dca6, '6800:6802'),
        ('initial map', 6801, 1, 16, '6800:6801'),
        ('initial facing', 6801, 8, 1, '6800:6801'),
        ('initial mirror', 6801, 5, 0x4000, '6800:6801'),
        ('initial timer', 6801, 7, 139, '6800:6801'),
        ('initial selector', 6801, 10, 32, '6800:6801'),
        ('initial cursor', 6801, 11, 1, '6800:6801'),
        ('initial composition', 6801, 12, 0xf838, '6800:6801'),
        ('initial input', 6801, 17, 0, '6800:6801'),
    ]
    with tempfile.TemporaryDirectory(prefix='compare-negative-', dir=capture.parent) as temp:
        scratch = Path(temp)
        shutil.copyfile(capture / 'final.wram', scratch / 'final.wram')
        for name, frame, column, value, span in cases:
            changed = [row.copy() for row in rows]
            target = next(row for row in changed[1:] if int(row[0]) == frame)
            if target[column] == str(value):
                raise AssertionError(f'vacuous mutation: {name}')
            target[column] = str(value)
            text = io.StringIO()
            csv.writer(text, lineterminator='\n').writerows(changed)
            (scratch / 'frames.csv').write_text(text.getvalue())
            result = run(scratch, span)
            if result.returncode == 0 or f'selection frame {frame}' not in result.stderr:
                raise AssertionError(f'{name}: expected selection rejection, got {result.returncode}\n{result.stderr}')
    print(f'PASS: unchanged baseline and {len(cases)} initial/later fidget negative controls')


if __name__ == '__main__':
    check(*(Path(arg) for arg in sys.argv[1:]))
