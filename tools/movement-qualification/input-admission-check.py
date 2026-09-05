#!/usr/bin/env python3
"""Reproduce private input-admission evidence; never exports or edits ROM/SRAM.

Run --synthetic without private assets, --capture for two fresh boots per case,
or no arguments to verify existing captures. Run from repository root.
"""
import argparse
import csv
from hashlib import sha256
import json
import os
from pathlib import Path
import subprocess
import sys

from input_admission import Admission, CARDINALS, Dash
from verify import collide

ROOT = Path('local/input-admission')
TOOLS = Path('tools/movement-qualification')
ASSETS = {
    'local/Tenchi Souzou (Japan).sfc': 'f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548',
    'local/saves/Terranigma.srm': '709c1cb67b8aff8db49cba05959f128b1c0a1ca32184c9bb62c415d537658055',
}
# Raw CSV pins; no reference data is embedded here.
CSV_SHA256 = {
  "Down-1700-10": "f37cbf4f2ae5c47746edecab00cb18d11d0ca3d882c04f0e166ca22664ed99e6",
  "Down-1700-11": "ea93916037a61da16aa5025fa6b90410bb2ba987b7ffeb3d3179fc4a4611f96e",
  "Down-1701-10": "eeb997a4a06749ba9be94cd9dfd8faa74533b5168e73aa5dc0a7e5df07475263",
  "Down-1701-11": "0fa172f3a9cb2edef1ce29600715f123a97f0e1fb0873eaf302679ded195711b",
  "Down-1702-10": "49c0cf99d4231323dab5ad4bbdf14e621541d60c1f17e4410a84370cd8bb3063",
  "Down-1702-11": "c0afef2f4bba24a7a02f4d8cf76628f26ebaff6a7d4a100af78967b0349e11c2",
  "Down-1703-10": "ff56f7091080458d80884dfa294288fa8c91736d6e29e4d55a81fb97f5d71ef7",
  "Down-1703-11": "12e49521359eb8e9cfc4e8cb18cfce93e1af48acd431f4a964cf4c4f81c98336",
  "Left-1700-10": "7c0ba0f98e51f35bde324fb0fadf915039543b2f6b22a070c9fe252c509e5fba",
  "Left-1700-11": "060e86e5cfffbde2f5144bd3053e2b47fd743fac311d8d73730befc7eb295665",
  "Left-1701-10": "2cc3464558d20ce42efbe5962f214ef47d2a0e10eccf96cd0acd6823c408c15c",
  "Left-1701-11": "1d29237ad3c2245f10d50f7a41fd20b8a58e3a5e7b3166a8541ed482a8e0cd74",
  "Left-1702-10": "71c6c69c6faf38015e584021304dd957fe384a052eedcf99c294ce4acdebca59",
  "Left-1702-11": "7949fd917bf8a4f46b863c02cc239c68b73888f0edfcbd6610c45bdcb966af46",
  "Left-1703-10": "7221d753ddc2d4f096ddc1d7a91c4c33aa5424f019011a2a2c259b401cd5c01a",
  "Left-1703-11": "555d0f33ebb698094cb76ef5217828a0c68d3ef751c514e051091daf4d97770e",
  "Right-1700-10": "c9bf5b03e3d28635c09ac633b6914ad9313cc6e8b1d3d268bbabb85407738738",
  "Right-1700-11": "35f5372ed5e13375432e7e3939696f39f3f376421f0d34b0f2958649c3265c7d",
  "Right-1701-10": "2555e67d994a75cc933a5f33c4c9512c72555555c919ae38245d16a08f1e7b37",
  "Right-1701-11": "23253a8fc7f526cf201caf904e9260578c157bc6afff4fe64d16d2ee46fa9739",
  "Right-1702-10": "aece28ca329065f481a8ef1dd488cd2436b877991785b357f134003ecb520893",
  "Right-1702-11": "638d0f4266d8e03312b4fe15a3d04434d54447cecb9123741c864400e31051d7",
  "Right-1703-10": "169046f275d29bf0b2f40daf9f494f84ff371dc6aa059944b813381149a22beb",
  "Right-1703-11": "f844313ce601853555ce54071c5b9d6fda1cbc3c6648ba1d2d2840e99dd5ea9d",
  "Up-1700-10": "274c10c11dd35b8f2e9c61c757ef80d1625f610d8ae758add0bcfc204e173356",
  "Up-1700-11": "ddfe6cdf3f4b910a01d6d08c273bb49f220eb3305b671c856ce80e245beecc10",
  "Up-1701-10": "ee5ccbc40938bad92877e0253179fca15daf8ec4e90c526c26a923b379a4874e",
  "Up-1701-11": "8e1f27a381e5540c84e03714fb3024325f52fb0de3971b20151b6961fdf99df0",
  "Up-1702-10": "1d0250215ed494541c5876812cd0700e660a98449b951d212abcb1605e776488",
  "Up-1702-11": "176973cd0369add6f2cf1ff4e600496dff185325bc38ccd7aab6ba0e5a0d774d",
  "Up-1703-10": "4fcfacfb2a11c15e6b13c9ff28c2b5ba9dac875e6e7882ece7c7be4abdecae04",
  "Up-1703-11": "a84910238150ae5371c3c4935648776c798a2c9f8297b3a650bfb2dff2cac777",
  "boundary-dash": "db4bf3741449c53f7a5cdbec54b71948cd2a4cff3d3f02b4c4931d10eaba148d",
  "boundary-walk": "3a3f79a02a29f659700ba229894bd384a2db85ccee37d32c29537bfe4da5080b",
  "longhold": "f727168a1546e1915b6fd650ef8e2076982b4f3f7c2e7f8122d7a9bb2d8491a0",
  "quick": "5b3164bc2b21ea653879d2078f9d3d35f49ba336caa51e60e1d631b99f20bcb9",
  "rapid-reverse": "afe63c989e36d7790d393acceff6f2e3e4da0dc23f01352a09438c2428ea5ad0",
  "reverse": "d0bfc215f9345ec41eea5ca47889d45e2c28f851a674e88fed95005d8097ecb1",
  "route": "75bd1b5a63366d45e938cc03e0c562116bd22c00bba0b922116c10f127827bdd",
  "trace-cop61": "d44f9e8abb50721dae42923d1184aab87ae9351e349afcd9521ebb2479810275",
  "trace-dash": "123c58d6b3e54d3a9ccbdb0692c1ab9f27874d84adda4b1df27e0204ee4ab40b",
  "trace-walk": "3f5a02c0f93c60a38eea4e696f0e37d340779948841f438d87304fbd263817f2"
}

# (completed end, input intervals, optional ordered instruction stops)
CASES = {
    'quick': (1615, ['Left:1601:1602', 'Left:1605:1607'], ''),
    'boundary-dash': (1617, ['Left:1601:1602', 'Left:1611:1614'], ''),
    'boundary-walk': (1618, ['Left:1601:1602', 'Left:1612:1615'], ''),
    'longhold': (1640, ['Left:1601:1620', 'Left:1621:1625'], ''),
    'reverse': (1630, ['Left:1601:1604', 'Right:1604:1607', 'Left:1607:1610'], ''),
    'rapid-reverse': (1630, [f'{"Left" if i % 2 == 0 else "Right"}:{1601+i}:{1602+i}' for i in range(12)], ''),
    'route': (1810, ['Left:1601:1634', 'Down:1634:1646', 'Right:1646:1658',
                     'Up:1658:1670', 'Left:1670:1682', 'Right:1682:1694',
                     'Left:1710:1722', 'Left:1735:1747', 'Right:1760:1793'], ''),
    'trace-dash': (1612, ['Left:1601:1602', 'Left:1611:1615'],
                   '84ae12,84ae1f,84ae22,84ae24,849144,84a464'),
    'trace-walk': (1613, ['Left:1601:1602', 'Left:1612:1616'],
                   '84ae12,84ae1f,84ae22,84ae2b,848eb3,848eb6'),
    'trace-cop61': (1604, ['Left:1601:1620'], '809c03,809c67,809c8b'),
}
for direction in CARDINALS:
    for start in range(1700, 1704):
        for gap in (10, 11):
            CASES[f'{direction}-{start}-{gap}'] = (
                start + gap + 5,
                ['Left:1601:1634', f'{direction}:{start}:{start+1}',
                 f'{direction}:{start+gap}:{start+gap+3}'], '')


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def authenticate():
    for name, expected in ASSETS.items():
        assert digest(Path(name)) == expected, name


def capture():
    authenticate()
    project = ROOT / 'probe'
    (project / 'src').mkdir(parents=True, exist_ok=True)
    (project / 'src/main.rs').write_bytes((TOOLS / 'input-admission-probe.rs').read_bytes())
    (project / 'Cargo.toml').write_text('''[package]
name = "input-admission-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
''')
    subprocess.run(['cargo', 'build', '--release', '--manifest-path', str(project / 'Cargo.toml')], check=True)
    for run in ('first', 'second'):
        for name, (end, inputs, pcs) in CASES.items():
            out = ROOT / run / name
            out.mkdir(parents=True, exist_ok=True)
            env = dict(os.environ)
            env.pop('PCS', None)
            if pcs:
                env['PCS'] = pcs
            with (out / 'stops.txt').open('w') as log:
                subprocess.run([str(project / 'target/release/input-admission-probe'),
                                str(out), str(end), *inputs], env=env, stdout=log, check=True)
    authenticate()


def verify_case(name):
    directory = ROOT / 'first' / name
    rows = list(csv.DictReader((directory / 'frames.csv').open()))
    wram = (directory / 'f1601.wram').read_bytes()
    gate = Admission()
    delayed = active = ''
    age = 0
    steps = 0
    dash_frame = None
    dash_direction = ''
    for previous, row in zip(rows, rows[1:]):
        frame = int(row['frame'])
        if dash_frame is None:
            try:
                gate = gate.submit(row['input'])
            except Dash:
                # Submitted at frame-1: controller selects accelerated script
                # one completed frame later. Do not qualify its collisions.
                dash_frame = frame + 1
                dash_direction = row['input']
        if dash_frame is not None and frame >= dash_frame:
            resume = {'Down': '84a44b', 'Up': '84a45c', 'Left': '84a471', 'Right': '84a471'}[dash_direction]
            assert row['resume'] == resume, (name, frame, row['resume'])
            assert int(row['last'], 16) == 0, (name, frame, 'history not consumed')
            age = frame - dash_frame
            # Only an onset signature, not a model of sustained dash/recovery.
            if age <= 3:
                magnitude = (0, 3, 2, 2)[age]
                output = (magnitude * {'Left': -1, 'Right': 1}.get(dash_direction, 0),
                          magnitude * {'Up': -1, 'Down': 1}.get(dash_direction, 0))
                assert output == (int(row['outx']), int(row['outy'])), (name, frame, 'dash onset')
            continue
        if delayed != active:
            active, age = delayed, 0
        else:
            age += 1
        phase = age % 54 if active in ('Left', 'Right') else age
        magnitude = 0 if not active or phase == 0 else (1 if phase % 2 else 2)
        dx = magnitude * {'Left': -1, 'Right': 1}.get(active, 0)
        dy = magnitude * {'Up': -1, 'Down': 1}.get(active, 0)
        assert (dx, dy) == (int(row['outx']), int(row['outy'])), (name, frame, 'cadence')
        assert int(row['flags'], 16) & 0x0406 == 0x0404, (name, frame, 'mode')
        assert previous['map'] == row['map'] == 'f', (name, frame, 'map')
        predicted = collide(wram, int(previous['x']), int(previous['y']), dx, dy, flat_only=True)
        assert predicted == (int(row['x']), int(row['y'])), (name, frame, predicted)
        delayed = row['input']
        steps += 1
    return {'ordinary_steps': steps, 'dash_completed_frame': dash_frame}


def verify():
    hashes = {}
    results = {}
    for name, (end, _, pcs) in CASES.items():
        first, second = ROOT / 'first' / name, ROOT / 'second' / name
        expected = {'frames.csv', 'stops.txt', 'f1601.wram', f'f{end}.wram'}
        for index, pc in enumerate(pcs.split(',') if pcs else []):
            expected.update(f'step{index}-{pc}.{extension}' for extension in ('trace', 'wram'))
        for directory in (first, second):
            assert {p.name for p in directory.iterdir()} == expected, (name, directory, 'artifacts')
        for artifact in first.iterdir():
            assert artifact.read_bytes() == (second / artifact.name).read_bytes(), (name, artifact.name)
        hashes[name] = digest(first / 'frames.csv')
        results[name] = verify_case(name)
    # Pins are research output hashes, not extracted data.
    assert hashes == CSV_SHA256, 'CSV provenance changed'
    (ROOT / 'csv-hashes.json').write_text(json.dumps(hashes, indent=2, sort_keys=True) + '\n')
    (ROOT / 'results.json').write_text(json.dumps(results, indent=2, sort_keys=True) + '\n')
    print(f'{len(CASES)} cases reproduced byte-for-byte; '
          f'{sum(r["ordinary_steps"] for r in results.values())} ordinary steps matched')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--capture', action='store_true')
    parser.add_argument('--synthetic', action='store_true')
    args = parser.parse_args()
    subprocess.run([sys.executable, str(TOOLS / 'input-admission-test.py')], check=True)
    if not args.synthetic:
        if args.capture:
            capture()
        verify()
