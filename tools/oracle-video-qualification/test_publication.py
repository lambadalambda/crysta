#!/usr/bin/env python3
"""ROM-free policy and actual upstream Screen completion/lifecycle regression."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
VENDOR = ROOT / 'vendor/ares'


def compiler_args():
    command = [os.environ.get('CXX', 'c++'), '-std=c++20', '-O2',
               '-fno-char8_t', '-w', '-pthread', '-DNALL_HEADER_ONLY',
               '-DSLJIT_HAVE_CONFIG_PRE=1', '-DSLJIT_HAVE_CONFIG_POST=1']
    for path in (VENDOR, VENDOR / 'ares', VENDOR / 'nall', VENDOR / 'thirdparty'):
        command += ['-I', str(path)]
    return command


class PublicationTests(unittest.TestCase):
    def test_build_and_both_translation_units_enforce_policy(self):
        build = (ROOT / 'crates/oracle/build.rs').read_text()
        self.assertIn('.define("ARES_ORACLE_SYNCHRONOUS_VIDEO", "1")', build)
        for name in ('ares-unity.cpp', 'shims.cpp'):
            self.assertIn('static_assert(!ares::Video::Threaded', (VENDOR / name).read_text())

    def test_upstream_default_stays_threaded(self):
        subprocess.run(compiler_args() + ['-x', 'c++', '-fsyntax-only', '-'],
                       input='#include <ares/ares.hpp>\nstatic_assert(ares::Video::Threaded);\n',
                       text=True, check=True, timeout=120)

    def test_actual_screen_completion_and_lifecycle(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary = str(Path(tmp) / 'screen')
            command = compiler_args() + ['-DARES_ORACLE_SYNCHRONOUS_VIDEO=1',
                       str(Path(__file__).with_name('screen.cpp')), '-o', binary]
            subprocess.run(command, check=True, timeout=120)
            subprocess.run([binary], check=True, timeout=60)


if __name__ == '__main__':
    unittest.main()
