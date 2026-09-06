#!/usr/bin/env python3
"""Retain an unmodified map-inspector capture and its local producer identity."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
import uuid

import bridge
from check import source_hashes, sha, require, ROM, SRAM, POLICY


def run(repo, out, rom, save, current_descriptor=None, fixed_source_repo=None):
    require((current_descriptor is None) == (fixed_source_repo is None),
            'current descriptor and explicit historical fixed source must be supplied together')
    repo, out, rom, save = [p.resolve() for p in (repo, out, rom, save)]
    descriptor = None
    if current_descriptor is not None:
        descriptor = bridge.verify_current(repo, fixed_source_repo, current_descriptor)
        descriptor_sha = sha(current_descriptor.read_bytes())
        require(sha(rom.read_bytes()) == ROM and sha(save.read_bytes()) == SRAM and save.stat().st_size == 8192,
                'owned ROM/SRAM mismatch')
    out.mkdir(parents=True, exist_ok=False)
    command = ['cargo', 'build', '--locked', '-p', 'map-inspector']
    # Historical mode retains its original API/target. Current mode always starts
    # a clean target inside a refused-if-existing output root, even for run B.
    target = out / 'target' if descriptor else repo / 'local/map-inspector-build'
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    with (out / 'build.log').open('wb') as log:
        subprocess.run(command, cwd=repo, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    binary = target / 'debug/map-inspector'
    shutil.copy2(binary, out / 'map-inspector')
    invocation = [str(out / 'map-inspector'), 'capture', str(rom), str(save)]
    started = time.time_ns()
    with subprocess.Popen(invocation, cwd=repo, stdout=subprocess.PIPE, stderr=subprocess.PIPE) as process:
        try:
            stdout, stderr = process.communicate(timeout=600)
        except subprocess.TimeoutExpired:
            process.kill()
            stdout, stderr = process.communicate()
    finished = time.time_ns()
    (out / 'stdout.txt').write_bytes(stdout)
    (out / 'stderr.txt').write_bytes(stderr)
    require(process.returncode == 0, 'capture process failed; see retained stdout/stderr')
    require(stdout.startswith(b'Local map viewer: '), 'unexpected capture stdout')
    directory = Path(stdout.decode().strip().removeprefix('Local map viewer: ')).parent
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
    if descriptor:
        require(bridge.verify_current(repo, fixed_source_repo, current_descriptor) == descriptor and
                sha(current_descriptor.read_bytes()) == descriptor_sha, 'sources/descriptor changed during capture')
        provenance.update(schema_version=2, epoch=bridge.EPOCH, policy=POLICY,
                          descriptor_sha256=descriptor_sha,
                          additional_source_hashes=descriptor['additional_source_hashes'],
                          fresh_target=True, source_repo=str(repo), rom_path=str(rom), sram_path=str(save),
                          stdout_sha256=sha(stdout), stderr_sha256=sha(stderr),
                          process=dict(run_id=str(uuid.uuid4()), pid=process.pid, exit_code=process.returncode,
                                       started_ns=started, finished_ns=finished))
    (out / 'producer.json').write_text(json.dumps(provenance, indent=2, sort_keys=True) + '\n')
    print(out)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('repo', 'out', 'rom', 'save'):
        parser.add_argument(name, type=Path)
    parser.add_argument('--current-descriptor', type=Path)
    parser.add_argument('--fixed-source-repo', type=Path)
    run(**vars(parser.parse_args()))
