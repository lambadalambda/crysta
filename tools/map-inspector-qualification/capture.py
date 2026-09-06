#!/usr/bin/env python3
"""Retain an unmodified map-inspector capture and its local producer identity."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

from check import source_hashes, sha


def run(repo, out, rom, save):
    repo, out, rom, save = [p.resolve() for p in (repo, out, rom, save)]
    out.mkdir(parents=True, exist_ok=False)
    command = ['cargo', 'build', '--locked', '-p', 'map-inspector']
    target = repo / 'local/map-inspector-build'
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    with (out / 'build.log').open('wb') as log:
        subprocess.run(command, cwd=repo, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    binary = target / 'debug/map-inspector'
    shutil.copy2(binary, out / 'map-inspector')
    invocation = [str(out / 'map-inspector'), 'capture', str(rom), str(save)]
    result = subprocess.run(invocation, cwd=repo, capture_output=True, check=True)
    (out / 'stdout.txt').write_bytes(result.stdout)
    (out / 'stderr.txt').write_bytes(result.stderr)
    directory = Path(result.stdout.decode().strip().removeprefix('Local map viewer: ')).parent
    shutil.copytree(directory, out / 'capture')
    provenance = {
        'schema_version': 1,
        'commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo).decode().strip(),
        'source_hashes': source_hashes(repo),
        'binary_sha256': sha(binary.read_bytes()),
        'rom_sha256': sha(rom.read_bytes()), 'sram_sha256': sha(save.read_bytes()),
        'build_command': command, 'target_dir': str(target), 'invocation': invocation,
        'rustc': subprocess.check_output(['rustc', '-Vv']).decode(),
        'cargo': subprocess.check_output(['cargo', '-V']).decode(),
        'cxx': subprocess.check_output([os.environ.get('CXX', 'c++'), '--version']).decode(),
        'build_env': {k: v for k, v in env.items() if k.startswith(('CARGO_', 'RUST', 'CXX', 'CC', 'CFLAGS', 'CPPFLAGS', 'SDKROOT', 'MACOSX_DEPLOYMENT_TARGET'))},
        'build_log_sha256': sha((out / 'build.log').read_bytes()),
    }
    (out / 'producer.json').write_text(json.dumps(provenance, indent=2, sort_keys=True) + '\n')
    print(out)


if __name__ == '__main__':
    if len(sys.argv) != 5:
        sys.exit('usage: capture.py REPO NEW-OUTPUT-DIRECTORY ROM SRAM')
    run(*map(Path, sys.argv[1:]))
