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


# The control word while ordinary walking is admitted. Frames outside it are
# scripted: arrivals and transitions move the player through cells walking
# cannot reach, so scoring them as occupancy invents walkable terrain.
WALKING_CONTROL = 160


def require(ok, message):
    if not ok:
        raise ValueError(message)


def uint(value, maximum=0xFFFF):
    return type(value) is int and 0 <= value <= maximum


def word_list(value, size):
    return isinstance(value, list) and len(value) == size and all(uint(v) for v in value)


def actor_positions(frame):
    """Accept both live producers and historical coordinate-pair captures."""
    actors = frame.get('actors', [])
    require(isinstance(actors, list), 'actors must be a list')
    positions, slots = [], set()
    for actor in actors:
        if isinstance(actor, dict):
            require(set(actor) in ({'slot', 'position', 'script'}, {'slot', 'words'}),
                    'invalid actor schema')
            slot = actor['slot']
            require(uint(slot) and 0x1040 <= slot < 0x2000 and slot % 0x40 == 0
                    and slot not in slots, 'invalid or duplicate actor slot')
            slots.add(slot)
            if 'words' in actor:
                require(word_list(actor['words'], 16), 'actor words must be sixteen u16s')
                require(actor['words'][5] != 0, 'actor is not live')
                position = actor['words'][:2]
            else:
                require(uint(actor['script'], 0xFFFFFF) and actor['script'] & 0xFFFF != 0,
                        'invalid live actor script')
                position = actor['position']
        else:
            position = actor
        require(word_list(position, 2), 'actor position must be a u16 pair')
        positions.append(position)
    return positions


def validate_frame(row, strict=False):
    require(isinstance(row, dict) and row.get('kind') == 'frame', 'invalid frame record')
    require(isinstance(row.get('label'), str) and bool(row['label']), 'invalid frame label')
    require(word_list(row.get('position'), 2), 'position must be a u16 pair')
    held = row.get('held')
    buttons = set(DIRECTIONS) | {'A', 'B', 'X', 'Y', 'L', 'R', 'Start', 'Select'}
    require(isinstance(held, list) and all(isinstance(h, str) and h in buttons for h in held)
            and len(set(held)) == len(held), 'invalid held buttons')
    for field in ('map', 'control', 'frame'):
        require(not strict or field in row, f'missing {field} evidence')
        if field in row:
            require(uint(row[field], 0xFFFFFFFF if field == 'frame' else 0xFFFF),
                    f'invalid {field}')
    require(not strict or 'actors' in row, 'missing actors evidence')
    actor_positions(row)


def validate_layer(layer):
    require(isinstance(layer, dict) and uint(layer.get('map')), 'invalid layer map')
    for field in ('width', 'height'):
        require(uint(layer.get(field)) and layer[field] > 0, f'invalid layer {field}')
    require(word_list(layer.get('cells'), layer['width'] * layer['height']),
            'layer cells must match dimensions and contain u16 words')
    # The old producer dumped before the attribute pass. Zero-only layers cannot
    # establish a partition; reject as unverified, not as proof all cells admit.
    require(any(w >> 9 for w in layer['cells']), 'unverified zero-attribute layer (possibly stale)')


def load(run, map_id=None, strict=False):
    frames, previous = [], None
    with open(run) as handle:
        for number, line in enumerate(handle, 1):
            try:
                row = json.loads(line)
                require(isinstance(row, dict) and isinstance(row.get('kind'), str),
                        'invalid record kind')
                if row['kind'] != 'frame':
                    continue
                validate_frame(row, strict)
                if strict and previous is not None:
                    require(row['frame'] > previous, 'frame numbers must increase')
                previous = row.get('frame')
                if row['label'] == 'boot' or row.get('control') not in (None, WALKING_CONTROL):
                    continue
                if map_id is not None and row.get('map') not in (None, map_id):
                    continue
                frames.append(row)
            except ValueError as error:
                raise ValueError(f'{run}:{number}: {error}') from error
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
        previous = cur[1][-1] if cur else None
        continuous = previous is not None and all(previous.get(f) == row.get(f)
                                                  for f in ('label', 'map', 'control'))
        if previous is not None and ('frame' in previous or 'frame' in row):
            continuous = continuous and 'frame' in previous and 'frame' in row \
                and row['frame'] == previous['frame'] + 1
        if cur and cur[0] == key and continuous:
            cur[1].append(row)
        else:
            if cur and cur[0]:
                out.append(cur)
            cur = (key, [row])
    if cur and cur[0]:
        out.append(cur)
    return out


# How close a live actor must be, in the pressed direction, to make a stall
# ambiguous. One cell plus half a cell of slack: deliberately conservative,
# because a missed terrain contact only costs coverage while an invented one
# corrupts the decode.
ACTOR_CLEARANCE = 24


def blocked_by_actor(frame, x, y, ux, uy):
    """Whether a live actor could be what stopped the player.

    Terrain is not the only thing that stops a player. A resident standing in
    a doorway produces exactly the same sustained stall as a wall, and reading
    that as solid terrain would invent a collision the map does not have.

    Actors at the player's exact position are ignored: the slot table contains
    Ark's own shadow, which tracks the player and would otherwise disqualify
    every contact ever measured.
    """
    for ax, ay in actor_positions(frame):
        if (ax, ay) == (x, y):
            continue
        ahead = (ax - x) * ux + (ay - y) * uy
        lateral = abs((ax - x) * uy) + abs((ay - y) * ux)
        if 0 < ahead <= ACTOR_CLEARANCE and lateral <= ACTOR_CLEARANCE:
            return True
    return False


def stalls(frames):
    """Positions pressed against something for STALL_FRAMES with no movement.

    The tail must hold one `control` value throughout: a stall spanning a
    dialogue, transition or any other state where input is not admitted is not
    wall contact. Stalls with an actor in or beside the target cell are
    discarded for the same reason.
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
        ux, uy = DIRECTIONS[key]
        if any(blocked_by_actor(t, x, y, ux, uy) for t in tail):
            continue
        found.append((x, y, ux, uy))
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
    validate_layer(layer)
    require(bool(frames), 'no frame evidence')
    for row in frames:
        validate_frame(row)
    check_one_map(frames, layer)
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
    validate_layer(layer)
    frames = []
    for run in sys.argv[2:]:
        frames.extend(load(run, layer.get('map')))
    check_one_map(frames, layer)
    results, points, contacts, cells, distinct = solve(frames, layer)
    dx_edge = (DX_WINDOW[0], DX_WINDOW[-1])
    dy_edge = (DY_WINDOW[0], DY_WINDOW[-1])
    print(f'{len(frames)} frames, {points} distinct positions covering {cells} cells, '
          f'{distinct} distinct sustained contacts ({contacts} total)')
    if not results:
        print('NO CONSISTENT OFFSET: the samples contradict a single-point model')
        return 1
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
    try:
        sys.exit(main())
    except (OSError, ValueError) as error:
        sys.exit(f'invalid evidence: {error}')
