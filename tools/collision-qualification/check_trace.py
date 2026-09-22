#!/usr/bin/env python3
"""Validate two one-frame COP CA witnesses; NOT a walking-passability gate.

The Right controller can skip an accelerated-action branch and continue COP61
on either path. This checker proves the recorded call/branch and observations,
not causation, complete machine-state equality, or the downstream resolver.
Usage: check_trace.py RUN.jsonl TRACE_DIRECTORY
"""
import hashlib
import json
from pathlib import Path
import struct
import sys

from derive import require, uint, word_list


def validate_pair(admitted, refused):
    """Check actual displacement, complete frames, context and branch successors."""
    for moved, witness in ((True, admitted), (False, refused)):
        before, meta, after, addresses = witness
        require(all(isinstance(row, dict) for row in (before, meta, after)),
                'trace records must be objects')
        for row in (before, after):
            require(word_list(row.get('position'), 2), 'position must be a u16 pair')
            require(uint(row.get('frame'), 0xFFFFFFFF) and uint(row.get('map')),
                    'invalid frame or map')
        require(meta['stop'] == 'FrameLimit', 'trace did not finish a frame')
        require(meta['frames'] == [before['frame'], after['frame']]
                and after['frame'] == before['frame'] + 1, 'noncontiguous frame')
        require(meta['instructions'] == len(addresses) and addresses,
                'instruction count mismatch')
        require(before['map'] == after['map'], 'map changed during trace')
        require(before['control'] == after['control'] == 160, 'not ordinary control')
        require(before['facing'] == after['facing'] == 3, 'not facing Right')
        require(before['held'] == meta['held'] == after['held'] == ['Right'],
                'Right must remain held')
        bx, by = before['position']
        ax, ay = after['position']
        require(ay == by and (ax > bx if moved else ax == bx),
                'label disagrees with observed displacement')
        for address in (0x848E6C, 0x80AD39, 0x848E76, 0x80D107):
            require(address in addresses, f'missing call/continuation {address:06x}')
        successors = [addresses[i + 1] for i, address in enumerate(addresses[:-1])
                      if address == 0x80ADAD]
        require(successors == [0x80ADAF if moved else 0x80ADB7],
                f'unexpected COP CA branch successors: {successors}')
    require(admitted[0]['map'] == refused[0]['map'], 'witnesses use different maps')


def load_witness(rows, directory, name):
    require(all(isinstance(row, dict) for row in rows), 'trace records must be objects')
    matches = [i for i, row in enumerate(rows)
               if row.get('kind') == 'trace' and row.get('trace') == name]
    require(len(matches) == 1, f'expected one trace named {name}')
    i = matches[0]
    require(0 < i < len(rows) - 1, 'trace missing frame context')
    before, meta, after = rows[i - 1:i + 2]
    require(before.get('kind') == after.get('kind') == 'frame',
            'trace must have adjacent frame samples')
    raw = (directory / f'{name}.trace').read_bytes()
    require(len(raw) > 0 and len(raw) % 8 == 0, 'invalid trace length')
    entries = list(struct.iter_unpack('<IBBBB', raw))
    require(all(address <= 0xFFFFFF and emu in (0, 1) and reserved == 0
                for address, _, emu, _, reserved in entries), 'invalid trace entry')
    print(f'{name}: sha256={hashlib.sha256(raw).hexdigest()}')
    return before, meta, after, [entry[0] for entry in entries]


def main(argv):
    if len(argv) != 2:
        raise ValueError(__doc__)
    rows = [json.loads(line) for line in Path(argv[0]).read_text().splitlines()]
    directory = Path(argv[1])
    admitted = load_witness(rows, directory, 'right-admitted')
    refused = load_witness(rows, directory, 'right-refused')
    validate_pair(admitted, refused)
    for witness in (admitted, refused):
        before, meta, after, _ = witness
        print(f'{meta["trace"]}: frames {meta["frames"]}, '
              f'{before["position"]} -> {after["position"]}')
    print('COP CA branch witnesses passed; ordinary collision remains a separate claim')


if __name__ == '__main__':
    try:
        main(sys.argv[1:])
    except (ValueError, KeyError, OSError, struct.error) as error:
        sys.exit(f'trace check FAILED: {error}')
