#!/usr/bin/env python3
"""Derive the collision reference point and solid attribute set from samples.

Input is one or more probe JSONL runs plus their retained layer dumps. This
imposes no published mask: it solves for the offset that keeps the collision
point off every attribute that demonstrably stops a sustained press, and
reports the resulting partition. Disagreement is reported, never smoothed.
"""
import json
import sys

DIRECTIONS = {'Right': (1, 0), 'Left': (-1, 0), 'Up': (0, -1), 'Down': (0, 1)}
# A press held this long with no integer movement is wall contact, not the
# sub-pixel accumulator failing to carry.
STALL_FRAMES = 20


def load(run):
    frames = []
    with open(run) as handle:
        for line in handle:
            row = json.loads(line)
            if row.get('kind') == 'frame' and row.get('label') != 'boot':
                frames.append(row)
    return frames


def attribute(layer, cx, cy):
    """Bits 9..14 of the cell word.

    Word bit 15 is excluded: docs/static-maps.md records gameplay setting it
    after initialization, so including it would split one attribute into two
    depending on whether a cell had been touched.
    """
    if not (0 <= cx < layer['width'] and 0 <= cy < layer['height']):
        return None
    return (layer['cells'][cy * layer['width'] + cx] >> 9) & 0x3F


def runs(frames):
    """Maximal runs of a single held cardinal within one command label."""
    out, cur = [], None
    for row in frames:
        held = [h for h in row['held'] if h in DIRECTIONS]
        key = held[0] if len(held) == 1 else None
        if cur and cur[0] == key and cur[1][-1]['label'] == row['label']:
            cur[1].append(row)
        else:
            if cur and cur[0]:
                out.append(cur)
            cur = (key, [row])
    if cur and cur[0]:
        out.append(cur)
    return out


def stalls(frames):
    """Positions pressed against something for STALL_FRAMES with no movement.

    The tail must also hold one `control` value throughout. A stall that spans
    a dialogue, transition or any other state where input is not admitted is
    not wall contact, and scoring it as such would invent a solid cell.
    """
    found = []
    for key, group in runs(frames):
        tail = group[-STALL_FRAMES:]
        if len(tail) != STALL_FRAMES:
            continue
        if len({tuple(t['position']) for t in tail}) != 1:
            continue
        if len({t.get('control') for t in tail}) != 1:
            continue
        x, y = tail[0]['position']
        found.append((x, y, *DIRECTIONS[key]))
    return found


def check_one_map(frames, layer):
    """Every sample must belong to the layer it is scored against."""
    ids = {r['map'] for r in frames if 'map' in r}
    if len(ids) > 1:
        raise SystemExit(f'samples span maps {sorted(ids)}; score each map separately')
    if ids and 'map' in layer and layer['map'] not in ids:
        raise SystemExit(f'samples are map {sorted(ids)}, layer is {layer["map"]}')


# The collision point is assumed to lie within the player's own sprite extent,
# one 16x16 cell wide and up to a tile-and-a-half tall above the position word.
# Without this the solver admits mirror solutions: an offset that sits
# systematically off the player can land on walls everywhere and report the
# partition inverted, which is internally consistent and physically absurd.
DX_WINDOW = range(-16, 16)
DY_WINDOW = range(-24, 8)


def solve(frames, layer, dx_window=DX_WINDOW, dy_window=DY_WINDOW):
    """Offsets consistent with every sample, within the sprite-extent window."""
    points = {tuple(r['position']) for r in frames}
    contacts = stalls(frames)
    results = []
    for dx in dx_window:
        for dy in dy_window:
            occupied = set()
            for (x, y) in points:
                a = attribute(layer, (x + dx) // 16, (y + dy) // 16)
                if a is None:
                    occupied = None
                    break
                occupied.add(a)
            if occupied is None:
                continue
            blocking = set()
            for (x, y, ux, uy) in contacts:
                a = attribute(layer, (x + dx) // 16 + ux, (y + dy) // 16 + uy)
                if a is not None:
                    blocking.add(a)
            # An offset is consistent only if nothing that blocks a press is
            # also a cell the player provably stands on.
            if occupied & blocking:
                continue
            results.append({'dx': dx, 'dy': dy, 'walkable': sorted(occupied),
                            'solid': sorted(blocking)})
    # Cells, not positions, are the unit a per-cell predicate is evidenced in.
    cells = len({((x + 0) // 16, (y + 0) // 16) for (x, y) in points})
    return results, len(points), len(contacts), cells, len(set(contacts))


def main():
    if len(sys.argv) < 3:
        sys.exit('usage: derive.py LAYER.json RUN.jsonl [RUN.jsonl ...]')
    with open(sys.argv[1]) as handle:
        layer = json.load(handle)
    frames = []
    for run in sys.argv[2:]:
        frames.extend(load(run))
    check_one_map(frames, layer)
    results, points, contacts, cells, distinct = solve(frames, layer)
    dx_edge = (DX_WINDOW[0], DX_WINDOW[-1])
    dy_edge = (DY_WINDOW[0], DY_WINDOW[-1])
    print(f'{len(frames)} frames, {points} distinct positions covering {cells} cells, '
          f'{distinct} distinct sustained contacts ({contacts} total)')
    if not results:
        print('NO CONSISTENT OFFSET: the samples contradict a single-point model')
        return
    xs = {r['dx'] for r in results}
    ys = {r['dy'] for r in results}
    print(f'{len(results)} consistent offsets: dx in {min(xs)}..{max(xs)}, dy in {min(ys)}..{max(ys)}')
    if min(xs) == dx_edge[0] or max(xs) == dx_edge[1] or \
       min(ys) == dy_edge[0] or max(ys) == dy_edge[1]:
        print('WARNING: consistent offsets touch the search window; widen it')
    # Report how many offsets vote for each set, so a value that only some
    # offsets call solid is visible as unresolved rather than averaged away.
    for field in ('walkable', 'solid'):
        tally = {}
        for r in results:
            tally[tuple(r[field])] = tally.get(tuple(r[field]), 0) + 1
        print(f'{field}:')
        for value, count in sorted(tally.items(), key=lambda kv: -kv[1]):
            print(f'  {list(value)} for {count}/{len(results)} offsets')
        agreed = set.intersection(*(set(r[field]) for r in results))
        disputed = set.union(*(set(r[field]) for r in results)) - agreed
        print(f'  agreed by every offset: {sorted(agreed)}')
        if disputed:
            print(f'  UNRESOLVED (offset-dependent): {sorted(disputed)}')


if __name__ == '__main__':
    main()
