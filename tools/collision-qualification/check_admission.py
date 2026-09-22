#!/usr/bin/env python3
"""Diagnostic comparison of a ROM table with ordinary walking observations.

The right-edge probe gates Right accelerated-action setup (COP2B), NOT ordinary
walking (COP61) or general directional admission. Its table is also reused by
slope helpers. This diagnostic projects the table onto a fixed point and
cardinal leading edges to expose disagreements with a passability hypothesis;
it does not claim those frames executed the probe. Slopes, actors and in-run
layer drift remain outside this static-layer comparison.
"""
import hashlib
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from derive import (load, stalls, require, uint, validate_frame, validate_layer,
                    WALKING_CONTROL)  # noqa: E402

TABLE = 0xE85C
ROM_SHA = 'f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'


def read_rom(path):
    """Authenticate the Japanese image after conventional copier-header removal."""
    rom = Path(path).read_bytes()
    if len(rom) % 0x8000 == 512:
        rom = rom[512:]
    require(hashlib.sha256(rom).hexdigest() == ROM_SHA, 'Japanese ROM identity mismatch')
    return rom


def read_table(path):
    rom = read_rom(path)
    require(len(rom) >= TABLE + 32, 'truncated ROM admission table')
    return list(rom[TABLE:TABLE + 32])


def probe_attribute(word):
    """AD99..ADA5: bit7 high-byte override to 6, THEN LSR and AND #$001F."""
    return 3 if word & 0x8000 else (word >> 9) & 0x1F


def label(word):
    """Keep flagged cells distinct in reports, despite their entry-3 lookup."""
    return 'dyn' if word & 0x8000 else probe_attribute(word)


def cell(layer, cx, cy):
    require(0 <= cx < layer['width'] and 0 <= cy < layer['height'],
            f'layer coverage missing at cell ({cx}, {cy})')
    return layer['cells'][cy * layer['width'] + cx]


def score(table, layer, frames):
    require(isinstance(table, list) and len(table) == 32 and all(uint(v, 255) for v in table),
            'table must contain 32 bytes')
    validate_layer(layer)
    require(bool(frames), 'no walking-frame evidence')
    occupied, contacted = {}, {}
    for row in frames:
        validate_frame(row, strict=True)
        require(row['map'] == layer['map'],
                f'sample map {row["map"]:04x} differs from layer {layer["map"]:04x}')
        require(row['control'] == WALKING_CONTROL and row['label'] != 'boot',
                'sample is not ordinary walking evidence')
        x, y = row['position']
        word = cell(layer, (x - 8) // 16, (y - 16) // 16)
        a = label(word)
        occupied[a] = occupied.get(a, 0) + 1
    for (x, y, ux, uy) in stalls(frames):
        left, top = x - 8, y - 16
        if ux > 0:
            edge = [(left + 16, top), (left + 16, top + 15)]
        elif ux < 0:
            edge = [(left - 1, top), (left - 1, top + 15)]
        elif uy > 0:
            edge = [(left, top + 16), (left + 15, top + 16)]
        else:
            edge = [(left, top - 1), (left + 15, top - 1)]
        attrs = tuple(sorted({probe_attribute(cell(layer, px // 16, py // 16))
                              for px, py in edge}))
        contacted[attrs] = contacted.get(attrs, 0) + 1
    return occupied, contacted


def compare(table, layer, frames):
    occupied, contacted = score(table, layer, frames)
    bad_occ = {a: n for a, n in occupied.items() if table[3 if a == 'dyn' else a]}
    bad_con = {a: n for a, n in contacted.items() if all(table[b] == 0 for b in a)}
    return {'map': layer['map'], 'frames': len(frames), 'occupied': occupied,
            'contacts': contacted, 'stood_on_refused': bad_occ,
            'stalled_against_admitted': bad_con,
            'disagreements': sum(bad_occ.values()) + sum(bad_con.values())}


def check_run(table, run):
    run = Path(run)
    frames = load(run, strict=True)
    require(bool(frames), f'{run}: no walking-frame evidence')
    reports = []
    for map_id in sorted({r['map'] for r in frames}):
        path = run.with_suffix('') / f'layer-{map_id:04x}.json'
        with path.open() as handle:
            layer = json.load(handle)
        try:
            reports.append(compare(table, layer, [r for r in frames if r['map'] == map_id]))
        except ValueError as error:
            raise ValueError(f'{run} map {map_id:04x} ({path}): {error}') from error
    require(any(r['contacts'] for r in reports), f'{run}: no sustained terrain-contact evidence')
    return reports


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    if len(argv) < 2:
        print('usage: check_admission.py ROM RUN.jsonl [RUN.jsonl ...]', file=sys.stderr)
        return 2
    try:
        table = read_table(argv[0])
        # Validate all requested inputs before printing a potentially reassuring result.
        reports = [(run, check_run(table, run)) for run in argv[1:]]
        print('right-edge table-precheck: Right accelerated-action setup, not ordinary walking')
        print('diagnostic table-vs-walking projection only; final movement NOT qualified')
        print('static entry layers: in-run drift unverified; no executed-precheck coverage claimed')
        print('admitted:', [a for a in range(32) if table[a] == 0])
        print('refused: ', {a: f'{table[a]:02x}' for a in range(32) if table[a]})
        disagreements = 0
        for run, maps in reports:
            for report in maps:
                print(f'{run} map {report["map"]:04x}: {report["frames"]} frames, '
                      f'occupied {report["occupied"]}, contacts {report["contacts"]}')
                print(f'  stood on table-refused attributes: {report["stood_on_refused"]}')
                print(f'  stalled against table-admitted cells: {report["stalled_against_admitted"]}')
                disagreements += report['disagreements']
        print('projection disagreements (occupancy frames + sustained contacts):', disagreements)
        return 1 if disagreements else 0
    except (OSError, ValueError) as error:
        print(f'invalid evidence: {error}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
