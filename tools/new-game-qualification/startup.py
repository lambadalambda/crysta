"""Source-backed, CPU-free *room-only* NewGame semantics; not an intro interpreter.

Authenticates the owned JP ROM and every annotated source extent. Reads neither
SRAM nor captures. Rendering, dialogue timing, inventory and NPC simulation are
outside this projection; intro completion is an explicit semantic operation.
"""
from hashlib import sha256
import json
from pathlib import Path
import sys


def word(data, offset=0):
    return int.from_bytes(data[offset:offset + 2], 'little')


def transition(record):
    """COP 14's eight inline bytes, per $80:8A23..8A57."""
    if len(record) != 10 or record[:2] != bytes([2, 0x14]):
        raise ValueError('expected one COP 14 map request')
    return word(record, 2), record[4], record[5], word(record, 6), word(record, 8)


def spawn(tile, queued):
    """FD default then $80:F7F3 override; return position and consumed queue."""
    x, y = queued
    position = ((x + 8) & 0xFFFF, (y + 16) & 0xFFFF) if x or y else (tile[0] * 16 + 8, tile[1] * 16)
    return position, (0, 0)


def assign_event(events, operand):
    """COP 07 -> $80:BB77: positive clears, bit15 sets, low12 select bit."""
    index = operand & 0xFFF
    offset, bit = divmod(index, 8)
    if offset >= len(events):
        raise ValueError('event outside the projected block')
    result = bytearray(events)
    mask = 1 << bit
    result[offset] = result[offset] | mask if operand & 0x8000 else result[offset] & ~mask
    return bytes(result)


def sources(rom):
    metadata = json.loads(Path(__file__).with_name('sources.json').read_text())
    if sha256(rom).hexdigest() != metadata['rom_sha256']:
        raise ValueError('expected authenticated headerless Japanese ROM')
    result = {}
    for item in metadata['ranges']:
        start = item['normalized_start']
        size = int(item['cpu_end_exclusive'], 16) - int(item['cpu_start'], 16)
        data = rom[start:start + size]
        if sha256(data).hexdigest() != item['sha256']:
            raise ValueError(f"source mismatch: {item['name']}")
        result[item['name']] = data
    return result


def semantic_new_game(rom):
    s = sources(rom)
    # The pinned reset loop clears $0600..07FF. Its pinned default table uses
    # (address,value) pairs, terminated by a negative address; none reseeds the
    # projected event block. This is reset semantics, not copied captured flags.
    defaults = s['reset-default-table']
    for offset in range(0, len(defaults) - 2, 4):
        address = word(defaults, offset)
        if address < 0x700 and address + 1 >= 0x6C0:
            raise ValueError('default table unexpectedly writes projected events')
    if defaults[-2:] != b'\xff\xff':
        raise ValueError('default table terminator changed')
    events = bytes(64)
    for name in ('new-game-default-event', 'intro-release-event'):
        record = s[name]
        if len(record) != 4 or record[:2] != bytes([2, 7]):
            raise ValueError(f'{name}: expected COP 07')
        events = assign_event(events, word(record, 2))

    map_id, mode, selector, x, y = transition(s['prologue-house-request'])
    # Authenticate both table slots, the chosen bank-$83 list, its first FD and
    # the player header. No dependence on a saved player's actor or coordinates.
    if word(s['map-f-actor-bank82']) != 0 or word(s['map-f-actor-bank83']) != 0x8D1E:
        raise ValueError('map F actor list changed')
    record = s['map-f-first-actor']
    if record[0] != 0xFD or int.from_bytes(record[4:7], 'little') != 0x84A129:
        raise ValueError('expected map F player FD header')
    position, consumed = spawn((record[1], record[2]), (x, y))
    header = s['player-header']
    return {
        'profile': 'jp-room-only-semantic-new-game-v1',
        'map': map_id, 'request_mode': mode, 'startup_selector': selector,
        'queued_position': [x, y], 'consumed_queue': list(consumed),
        'record_default_position': [record[1] * 16 + 8, record[2] * 16],
        'position': list(position), 'player_script': 0x84A129 + len(header),
        'initial_actor_flags': word(header, 1),
        'events_nonzero': [[i, b] for i, b in enumerate(events) if b],
        'event_flags_sha256': sha256(events).hexdigest(),
        'control': 'after-explicit-semantic-intro-completion',
        'omits': ['intro timing/rendering', 'NPC simulation', 'inventory/stat initialization',
                  'emulator actor/animation/scheduler state'],
    }


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit('usage: startup.py OWNED_JP_ROM')
    print(json.dumps(semantic_new_game(Path(sys.argv[1]).read_bytes()), indent=2))
