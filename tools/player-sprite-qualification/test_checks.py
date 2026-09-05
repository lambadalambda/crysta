"""Fail-closed qualification tests; optional local altered-export regression."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent

class CheckTests(unittest.TestCase):
    def test_optimized_imports_fail_closed(self):
        for module in ('check', 'evidence'):
            result = subprocess.run(
                [sys.executable, '-B', '-O', '-c', f'import {module}'],
                cwd=HERE, capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0, module)
            self.assertIn('optimization disables qualification', result.stderr)

    def test_altered_local_export_cannot_qualify(self):
        rom = ROOT / 'local/Tenchi Souzou (Japan).sfc'
        captures = sorted((ROOT / 'local/player-sprite-qualification').glob('replay-*'))
        captures = [p for p in captures if (p / 'export/export.json').exists()]
        if not rom.exists() or not captures:
            self.skipTest('optional local ROM and replay absent')
        with tempfile.TemporaryDirectory(dir=ROOT / 'local') as tmp:
            out = Path(tmp)
            for run in ('a', 'b'):
                (out / run).symlink_to((captures[-1] / run).resolve(), target_is_directory=True)
            shutil.copytree(captures[-1] / 'export', out / 'export')
            target = next((out / 'export').glob('*.rgba'))
            data = bytearray(target.read_bytes()); data[0] ^= 1; target.write_bytes(data)
            for optimize in ('0', '1'):
                env = dict(os.environ, PYTHONOPTIMIZE=optimize)
                result = subprocess.run(
                    [sys.executable, '-B', str(HERE / 'check.py'), str(rom), str(out)],
                    cwd=ROOT, env=env, capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0, optimize)
                self.assertNotIn('Qualified:', result.stdout)

if __name__ == '__main__': unittest.main()
