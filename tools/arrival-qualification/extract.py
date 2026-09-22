#!/usr/bin/env python3
"""Extract/verify bounded native arrival metadata; Python standard library only."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

HERE = Path(__file__).resolve().parent
ROM_SHA256 = 'f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'
EDGES = {'1e': (30, 10, 5, 12914, 12950), '19': (25, 23, 14, 14498, 14576)}


class EvidenceError(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise EvidenceError(message)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read_rows(path):
    with Path(path).open() as stream:
        return [json.loads(line) for line in stream if line.strip()]


def script(words):
    return words[5] | ((words[6] & 255) << 16)


def selected(row):
    """Selected observable contract, not a dump of entity words or map cells."""
    return dict(map=row['map'], position=row['position'], control=row['control'],
                flags=row['player_words'][2], player_script=script(row['player_words']),
                controller_script=script(row['control_words']),
                controller_flags=row['control_words'][2],
                control_slot=row['control_slot'], special=row['special'],
                facing=row['facing'])


def window(rows, first, last):
    found = [r for r in rows if first <= r.get('frame', -1) <= last]
    require([r['frame'] for r in found] == list(range(first, last + 1)),
            f'window {first}..{last} must have exactly one ordered row per frame')
    require(all(r['kind'] == 'arrival' for r in found), 'arrival observations required')
    return found


def changes(values):
    return [(i, value) for i, value in enumerate(values) if i == 0 or value != values[i - 1]]


def extract_profile(rows, loaded, free):
    samples = window(rows, loaded, free)
    states = [selected(r) for r in samples]
    require(len({s['map'] for s in states}) == 1, 'map changes inside loaded-to-free window')
    require(states[0]['player_script'] == 0x84A12E, 'first sample must be initialized player')
    require(states[-1]['player_script'] == 0x84A258 and states[-1]['special'] == 0,
            'last sample must be released free-player resume')
    require(all(s['player_script'] != 0x84A258 for s in states[:-1]),
            'free boundary must be the first free-player resume')
    ownership = next((i for i, s in enumerate(states) if s['special'] & 0x8000), None)
    forced = next((i for i, s in enumerate(states)
                   if s['player_script'] in (0x84BB70, 0x84BD7E)), None)
    require(ownership is not None and forced is not None, 'missing owned forced phase')
    recovery = next((i for i in range(ownership + 1, len(states))
                     if not states[i]['special'] & 0x8000), None)
    endpoint = next(i for i, s in enumerate(states) if s['position'] == states[-1]['position'])
    require(recovery is not None and ownership <= forced <= endpoint < recovery < len(states) - 1,
            'unexpected phase ordering')
    require(all(s['position'] == states[-1]['position'] for s in states[endpoint:]),
            'position moved after endpoint')
    require(states[recovery]['player_script'] == 0x84A303, 'missing recovery script')
    # These records cover EVERY frame by carrying the last change forward.
    positions = [[i, *xy] for i, xy in changes([s['position'] for s in states])]
    state_changes = [dict(elapsed=i, **state) for i, state in changes([
        {k: v for k, v in s.items() if k != 'position'} for s in states])]
    return dict(loaded_frame=loaded, free_frame=free, advances=free - loaded,
                samples=len(samples), loaded_position=states[0]['position'],
                settled_position=states[-1]['position'], positions=positions,
                phases=dict(initialized=0, ownership=ownership, forced=forced,
                            endpoint=endpoint, recovery=recovery, free=free - loaded),
                state_changes=state_changes)


def decode_record(data, offset):
    require(len(data) == 12, 'exit record must contain 12 bytes')
    return dict(rom_offset=offset, cpu_address=0x800000 + offset,
                rectangle=list(data[:4]), destination=struct.unpack_from('<H', data, 4)[0],
                mode=data[6], selector=data[7], raw_position=list(struct.unpack_from('<HH', data, 8)))


def source_record(image, source, destination, selector):
    pointer = struct.unpack_from('<H', image, 0x18000 + source * 2)[0]
    require(pointer >= 0x88AC, 'invalid exit list pointer')
    matches = []
    offset = 0x10000 + pointer
    while offset + 12 <= 0x20000 and image[offset] != 255:
        record = decode_record(image[offset:offset + 12], offset)
        if (record['destination'], record['selector']) == (destination, selector):
            matches.append(record)
        offset += 12
    require(len(matches) == 1, 'expected one source exit record')
    return matches[0]


def compare_hostile(baseline, hostile, first, last):
    left, right = window(baseline, first, last), window(hostile, first, last)
    selected_diffs, ancillary = [], []
    for a, b in zip(left, right):
        sa, sb = selected(a), selected(b)
        fields = sorted(k for k in sa if sa[k] != sb[k])
        if fields:
            selected_diffs.append(dict(frame=a['frame'], fields=fields))
        # Only names/offsets are published; no raw actor words or layers.
        fields = sorted(k for k in set(a) | set(b)
                        if k not in ('held', 'label') and a.get(k) != b.get(k))
        if fields:
            ancillary.append(dict(frame=a['frame'], fields=fields))
    return dict(first_frame=first, last_frame=last, selected_differences=selected_diffs,
                ancillary_differences=ancillary)


def queue_observations(rows, source, destination, selector, loaded):
    candidates = [r for r in rows if r.get('kind') == 'arrival'
                  and r['frame'] < loaded and r['map'] == source
                  and r['pending_map'] == destination and r['selector'] == selector]
    require(candidates, 'missing observed queue before load')
    values = [dict(position=r['queued_position'], mode=r['request_mode']) for r in candidates]
    return [dict(frame=candidates[i]['frame'], **value) for i, value in changes(values)]


def recipe_inputs(path):
    result = {}
    frame = 6800
    rows = read_rows(path)
    require(rows[-1] == {'finish': True}, 'recipe must finish')
    for row in rows[:-1]:
        require(set(row) <= {'label', 'buttons', 'frames', 'arrival'}, 'non-fixed-input command')
        require(0 < row['frames'] <= 2000, 'invalid held duration')
        for _ in range(row['frames']):
            frame += 1
            result[frame] = row['buttons']
    return result


def verify_inputs(rows, recipe):
    inputs = recipe_inputs(recipe)
    observed = [r for r in rows if r.get('frame', 0) > 6800]
    require([r['frame'] for r in observed] == list(inputs), 'capture/recipe frame coverage differs')
    require(all(sorted(r['held']) == sorted(inputs[r['frame']]) for r in observed),
            'capture held input differs from recipe')


def build(args):
    image = Path(args.rom).read_bytes()
    require(hashlib.sha256(image).hexdigest() == ROM_SHA256, 'wrong ROM revision/hash')
    edges = {}
    for name, (source, destination, selector, loaded, free) in EDGES.items():
        path = getattr(args, 'capture' + name)
        recipe = HERE / f'return{name}-route.jsonl'
        rows = read_rows(path)
        verify_inputs(rows, recipe)
        edge = dict(source_map=source, destination_map=destination, selector=selector,
                    source_record=source_record(image, source, destination, selector),
                    capture_sha256=sha(path), recipe=recipe.name, recipe_sha256=sha(recipe),
                    queue_observations=queue_observations(rows, source, destination, selector, loaded),
                    **extract_profile(rows, loaded, free))
        hostile_path = getattr(args, 'hostile' + name)
        if hostile_path:
            hostile_recipe = HERE / f'return{name}-hostile-route.jsonl'
            hostile = read_rows(hostile_path)
            verify_inputs(hostile, hostile_recipe)
            edge['hostile'] = dict(status='captured', capture_sha256=sha(hostile_path),
                recipe=hostile_recipe.name, recipe_sha256=sha(hostile_recipe),
                inputs=[[i, held] for i, held in changes([r['held'] for r in window(hostile, loaded, free)])],
                comparison=compare_hostile(rows, hostile, loaded, free))
        else:
            edge['hostile'] = dict(status='pending')
        edges[name] = edge
    return dict(schema='crysta-return-arrivals-v1', rom_sha256=ROM_SHA256,
                cursor='completed-frame elapsed from initialized player; inclusive cursor zero',
                exit_scan_gate=dict(address=0x8D87A0, mask=0x10,
                                    clear='permits scanning', set='suppresses scanning'),
                edges=edges)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['extract', 'verify'])
    parser.add_argument('--rom', required=True)
    parser.add_argument('--capture1e', required=True)
    parser.add_argument('--capture19', required=True)
    parser.add_argument('--hostile1e', required=True)
    parser.add_argument('--hostile19', required=True)
    parser.add_argument('--evidence', type=Path, default=HERE / 'evidence.json')
    args = parser.parse_args()
    try:
        actual = build(args)
        if args.command == 'extract':
            args.evidence.write_text(json.dumps(actual, indent=2) + '\n')
        else:
            require(actual == json.loads(args.evidence.read_text()), 'evidence differs from supplied sources')
            print('verified capture/recipe hashes, inputs, source operands, and every loaded-to-free frame')
    except (EvidenceError, KeyError, IndexError, OSError, json.JSONDecodeError) as error:
        parser.exit(1, f'evidence error: {error}\n')


if __name__ == '__main__':
    main()
