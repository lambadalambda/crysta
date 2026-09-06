"""Run fixed input-only plans twice; do not regenerate expectations automatically."""
import json
import os
from pathlib import Path
import subprocess
import sys

from verify import verify
from test_compare import check as check_compare

rom, out = sys.argv[1], Path(sys.argv[2])
plans = json.loads(Path('tools/player-animation-qualification/plans.json').read_text())
root = Path('local/player-animation-qualification')
probe = root / 'probe/target/release/animation-probe'
compare = root / 'compare/target/release/animation-compare'
for name, plan in plans.items():
    for run in ('a', 'b'):
        path = out / f'{name}-{run}'
        env = dict(os.environ)
        env.pop('PCS', None)
        if plan.get('pcs'):
            env['PCS'] = ','.join(plan['pcs'])
        subprocess.run([str(probe), rom, str(path), str(plan['end']), *plan['inputs']], env=env, check=True)
        if plan['spans']:
            subprocess.run([str(compare), rom, str(path), *plan['spans']], check=True)
    for file in ('frames.csv', 'report.json', 'final.wram'):
        assert (out/f'{name}-a'/file).read_bytes() == (out/f'{name}-b'/file).read_bytes(), (name, file)
# Negative control: six records are necessary, not just two alternating poses.
result = subprocess.run([str(compare), rom, str(out/'house-a'), '6800:6967'],
                        env={**os.environ, 'NAIVE_TWO_FRAME': '1'}, capture_output=True, text=True)
(out/'red-two-frame.log').write_text(result.stdout + result.stderr)
assert result.returncode != 0 and 'selection frame 6820' in result.stderr
verify(Path(rom), out)

check_compare(compare, Path(rom), out / "house-a")
