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
import library_bridge
import repin_bridge
from check import source_hashes, sha, require, ROM, SRAM, POLICY


def run(repo, out, rom, save, current_descriptor=None, fixed_source_repo=None,
        library_descriptor=None, predecessor_source_repo=None, repin_descriptor=None):
    current_mode = current_descriptor is not None
    library_mode = library_descriptor is not None
    repin_mode = repin_descriptor is not None
    any_mode = current_mode or library_mode or repin_mode
    require(sum((current_mode, library_mode, repin_mode)) <= 1,
            'select exactly one explicit descriptor mode')
    require((fixed_source_repo is not None) == any_mode,
            'descriptor mode and explicit historical fixed source must be supplied together')
    require((predecessor_source_repo is not None) == library_mode,
            'library descriptor and explicit predecessor source must be supplied together')
    repo, out, rom, save = [p.resolve() for p in (repo, out, rom, save)]
    descriptor = None
    mode = None
    if current_descriptor is not None:
        descriptor = bridge.verify_current(repo, fixed_source_repo, current_descriptor)
        descriptor_sha = sha(current_descriptor.read_bytes())
        mode = 'current'
    elif library_descriptor is not None:
        require(fixed_source_repo is not None,
                'library mode requires explicit historical fixed source')
        descriptor = library_bridge.verify_library_descriptor(
            repo, predecessor_source_repo, fixed_source_repo, library_descriptor)
        descriptor_sha = sha(library_descriptor.read_bytes())
        mode = 'library'
    elif repin_descriptor is not None:
        descriptor = repin_bridge.verify_repin_descriptor(repo, fixed_source_repo, repin_descriptor)
        descriptor_sha = sha(repin_descriptor.read_bytes())
        mode = 'repin'
    if descriptor is not None:
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
        if mode == 'current':
            unchanged = bridge.verify_current(repo, fixed_source_repo, current_descriptor) == descriptor
            unchanged = unchanged and sha(current_descriptor.read_bytes()) == descriptor_sha
        elif mode == 'library':
            unchanged = library_bridge.verify_library_descriptor(
                repo, predecessor_source_repo, fixed_source_repo, library_descriptor) == descriptor
            unchanged = unchanged and sha(library_descriptor.read_bytes()) == descriptor_sha
        else:
            unchanged = repin_bridge.verify_repin_descriptor(
                repo, fixed_source_repo, repin_descriptor) == descriptor
            unchanged = unchanged and sha(repin_descriptor.read_bytes()) == descriptor_sha
        require(unchanged, 'sources/descriptor changed during capture')
        provenance.update(fresh_target=True, source_repo=str(repo), rom_path=str(rom), sram_path=str(save),
                          stdout_sha256=sha(stdout), stderr_sha256=sha(stderr),
                          process=dict(run_id=str(uuid.uuid4()), pid=process.pid, exit_code=process.returncode,
                                       started_ns=started, finished_ns=finished))
        if mode == 'current':
            provenance.update(schema_version=2, epoch=bridge.EPOCH, policy=POLICY,
                              descriptor_sha256=descriptor_sha,
                              additional_source_hashes=descriptor['additional_source_hashes'])
        elif mode == 'library':
            predecessor = library_bridge.frozen_predecessor()[1]
            provenance.update(schema_version=3, kind=library_bridge.KIND,
                              epoch=library_bridge.EPOCH, policy=POLICY,
                              descriptor_sha256=descriptor_sha,
                              source_hashes=library_bridge.current_sources(descriptor, predecessor),
                              **{field: descriptor[field] for field in
                                 ('replaced_source_hashes', *library_bridge.GROUPS)})
        else:
            provenance.update(schema_version=4, kind=repin_bridge.KIND,
                              epoch=repin_bridge.EPOCH, policy=POLICY,
                              descriptor_sha256=descriptor_sha,
                              source_hashes=repin_bridge.current_sources(descriptor),
                              replaced_source_hashes=descriptor['replaced_source_hashes'])
    (out / 'producer.json').write_text(json.dumps(provenance, indent=2, sort_keys=True) + '\n')
    print(out)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('repo', 'out', 'rom', 'save'):
        parser.add_argument(name, type=Path)
    parser.add_argument('--current-descriptor', type=Path)
    parser.add_argument('--library-descriptor', type=Path)
    parser.add_argument('--fixed-source-repo', type=Path)
    parser.add_argument('--predecessor-source-repo', type=Path)
    parser.add_argument('--repin-descriptor', type=Path)
    run(**vars(parser.parse_args()))
