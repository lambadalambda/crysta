#!/usr/bin/env python3
"""JP bounded tower approach evidence, frozen return through first map101 entry.

project(bytes) authenticates the unheadered JP ROM before decoding. The decode_*
helpers permit synthetic Sources for tests; shape checks alone do not authenticate
source or prove a native route. No script/text/world-movement/combat VM. Running
this file with an owned ROM verifies typed operands and hashes against the pin.
"""
import json
from pathlib import Path
import sys

from source_contract import DepartureSource, ROM_SHA, require, sha, window_hashes

ROOT = Path(__file__).resolve().parent
PATH = [(0x21, 0x20), (0x20, 0xe), (0xe, 0xc), (0xc, 0xd), (0xd, 0xa),
        (0xa, 3), (3, 0x100), (0x100, 0x101)]


class TowerSource(DepartureSource):
    def xor_gate(self, address, selector):
        require(selector in (9, 0x47), 'unsupported XOR service')
        p = self.cop(address, selector)
        first, last = self.u(p), self.u(p + 2)
        require(first & 0xf000 == 0x1000 and last & 0x7000 == 0, 'unsupported XOR expression')
        result = {'source': address, 'events': [first & 0xfff, last & 0xfff], 'operation': 'xor'}
        # COP47 terminates on the selected result; COP09 branches on it.
        if selector == 0x47:
            result['require_true'] = bool(last & 0x8000)
        else:
            result.update(when_true=not bool(last & 0x8000),
                          target=(address & 0xff0000) | self.u(p + 4))
        return result

    def proximity(self, address):
        p = self.cop(address, 0xd6)
        return {'source': address, 'radius': self.u(p, 1),
                'target': (address & 0xff0000) | self.u(p + 1)}

    def spawn(self, address, selector):
        p = self.cop(address, selector)
        return {'source': address, 'target': self.u(p, 3), 'parameters': [self.u(p + 3, 1), self.u(p + 4, 1)]}

    def handoff(self, address):
        p = self.cop(address, 0xcb)
        return {'source': address, 'mode': self.u(p, 1), 'target': self.u(p + 1, 3)}

    def relative(self, address):
        self.expect(address, [0x80])
        delta = int.from_bytes(self.b(address + 1, 1), 'little', signed=True)
        return {'source': address, 'target': address + 2 + delta}


def route_exit(s, map_id, destination):
    """At most 16 fixed-size records; require one exit to the named next map."""
    offset = s.u(0x818000 + 2 * map_id)
    require(offset >= 0x8000, 'invalid exit table pointer')
    start = address = 0x810000 | offset
    matches = []
    for _ in range(16):
        require(address < 0x820000, 'exit bank boundary')
        if s.u(address, 1) == 0xff:
            require(len(matches) == 1, 'missing/ambiguous route exit')
            return {'from_map': map_id, 'table': start, 'end': address + 1, 'exit': matches[0]}
        require(address + 12 <= 0x820000, 'exit record bank boundary')
        record = s.exit(address)
        require(all(record['rectangle'][2:]), 'empty exit rectangle')
        if record['map'] == destination:
            matches.append(record)
        address += 12
    raise ValueError('unbounded exit list')


def scene(s, map_id):
    low, high = s.u(0x828000 + 2 * map_id), s.u(0x838000 + 2 * map_id)
    require((low or high) >= 0x8000, 'invalid scene pointer')
    return {'map': map_id, 'bank82_word': low, 'bank83_word': high,
            'selected': (0x820000 | low) if low else (0x830000 | high)}


def decode_elder(s):
    return {
        'admission': s.xor_gate(0x888bf0, 0x47), 'callback': s.callback(0x888bfc),
        'effects': [s.flag(a) for a in [0x888bf6, 0x888c17, 0x888c33]],
        'choice': s.choice(0x888c21),
        'requests': [s.request(a) for a in [0x888c1b, 0x888c2d, 0x888c41]],
        'waits': [s.marker(a, 0x1f) for a in [0x888c1f, 0x888c31, 0x888c45]],
        'clear_callbacks': [s.callback(a) for a in [0x888c37, 0x888c47]],
        'departures': [s.pointer(a, 0xc0) for a in [0x888c3b, 0x888c4b]],
        'event21_means': 'encounter_not_acceptance',
        'retry': {
            'qualification': 'source_only_not_native_route', 'map': 0xb, 'callback': 0x888ede,
            'branches': [s.branch(a) for a in [0x888ede, 0x888ee4, 0x888eea, 0x888ef0]],
            'choice': s.choice(0x888f4f), 'effect': s.flag(0x888f61),
            'requests': [s.request(a) for a in [0x888f49, 0x888f5b, 0x888f66]],
            'waits': [s.marker(a, 0x1f) for a in [0x888f4d, 0x888f5f, 0x888f6a]],
        },
    }


def decode_town(s):
    return {
        'gate': s.xor_gate(0x8884ef, 9),
        'effects': [s.flag(a) for a in [0x88850f, 0x888538]],
        'controls': [s.control(0x888513, 0x2a), s.control(0x888534, 0x29)],
        'delays': [s.delay(a) for a in [0x888517, 0x888526, 0x888530]],
        'requests': [s.request(a) for a in [0x88851b, 0x88852a]],
        'waits': [s.marker(0x88851f, 0x1f), s.marker(0x88852e, 0x20)],
        'presentation': s.pointer(0x888521, 0xdf), 'continuation': s.pointer(0x88853c, 0xbf),
        'proves_296_geographic_prerequisite': False,
    }


def decode_world(s):
    return {
        'spawn': s.spawn(0x84def2, 0x9b), 'effect': s.flag(0x84defe),
        'waits': [s.wait_flag(a) for a in [0x84df02, 0x84e473]],
        'branches': [s.branch(a) for a in [0x84e02a, 0x84e445]],
        'player_header': 0x84dee0, 'world_actor_header': 0x84e423,
        'movement_model': 'not_modeled_distinct_world_mode',
    }


def decode_guardian(s):
    input_word = s.instruction(0x908cee, 0xad)
    input_mask = s.instruction(0x908cf1, 0x29)
    return {
        'branches': [s.branch(a) for a in [0x908bfb, 0x908c10]],
        'proximity': [s.proximity(a) for a in [0x908c03, 0x908c0a]],
        'presentation': s.pointer(0x908c16, 0xdf), 'spawn': s.spawn(0x908c1b, 0xa2),
        'wait_helper': s.wait_flag(0x908c22),
        'requests': [s.request(a) for a in
                     [0x908c26, 0x908c30, 0x908c3a, 0x908c4b, 0x908c55, 0x908c5d, 0x908c67]],
        'waits': [s.marker(a, 0x1f) for a in
                  [0x908c2a, 0x908c34, 0x908c3e, 0x908c4f, 0x908c59, 0x908c61, 0x908c6b]],
        'choice': s.choice(0x908c40),
        'effects': [s.flag(a) for a in [0x908c2c, 0x908c36, 0x908c51, 0x908c63, 0x908c6d, 0x908c75]],
        'option1_join': s.relative(0x908c5b), 'cancel_option2_fallthrough': 0x908c6d,
        'delay': s.delay(0x908c71), 'handoff': s.handoff(0x908c79),
        'both_choices_grant_115': True,
        'helper': {'entry': 0x908c9f, 'effect': s.flag(0x908ccb), 'wait': s.wait_flag(0x908cd4),
                   'branches': [s.branch(a) for a in [0x908ce2, 0x908ce8]],
                   'input': {'source': 0x908cee, 'address': input_word, 'mask': input_mask},
                   'action': s.marker(0x908cf6, 0xe4), 'teardown': 0x908cfa,
                   'delay': s.delay(0x908d07), 'end': s.marker(0x908d0b, 0xa7)},
        'intro': {'header': 0x908f23, 'membership': s.membership(0x908f28),
                  'effect': s.flag(0x908f2c), 'request': s.request(0x908f40),
                  'wait': s.marker(0x908f44, 0x1f),
                  'controls': [s.control(0x908f30, 0x2a), s.control(0x908f46, 0x29)],
                  'end': s.marker(0x908f4a, 0xa7)},
    }


def decode_equipment(s):
    """Menu category selection + equip/clear writes + zero-checked stat readers.

    These words are separate from inventory slots and the $0648 usable-item word.
    This does not establish HP/XP meanings, auto-equip absence on every code path,
    or absence of combat from snapshots. Native checker owns state observations.
    """
    s.expect(0x85ae9f, [0xb0, 0x7e])  # item >= $80 -> weapon/armor split
    s.expect(0x85af22, [0xb0, 0x32])  # item >= $A0 -> armor selection
    s.expect(0x85f4bf, [0xc2, 0x20])  # word-width stat recalculation
    result = {'category_split': [s.instruction(0x85ae9c, 0xc9), s.instruction(0x85af1f, 0xc9)],
              'item81_insertion_proves_equipped': False, 'combat_state': 'hp_xp_not_qualified'}
    for name, selection, clear, equip, stat, branch in [
        ('weapon', 0x85af24, 0x85b006, 0x85b015, 0x85f4e2, 0x1d),
        ('armor', 0x85af56, 0x85b024, 0x85b033, 0x85f504, 0x2d),
    ]:
        address = s.instruction(selection, 0xad)
        selected_item = s.instruction(selection + 3, 0xcd)
        require(s.instruction(equip, 0xad) == selected_item, 'selected equipment item')
        require(s.instruction(clear, 0xa9) == 0, 'unequip zero')
        require(all(s.instruction(a, op) == address for a, op in
                    [(clear + 3, 0x8d), (equip + 3, 0x8d), (stat, 0xad)]), 'equipment word consistency')
        s.expect(stat + 3, [0xf0, branch])
        result[name] = {'address': 0x7e0000 | address, 'width': 2, 'none': 0,
                        'selection_source': selection, 'selected_item_address': selected_item,
                        'clear_source': clear + 3, 'equip_source': equip + 3, 'stat_read_source': stat}
    require(s.instruction(0x848932, 0xad) == result['weapon']['address'] & 0xffff, 'player weapon test')
    s.expect(0x848935, [0xf0, 0xb1])
    result['weapon']['player_zero_test'] = 0x848932
    return result


def decode_records(s):
    records = []
    for a, header in [(0x838cbe, 0x888beb), (0x838902, 0x84dee0),
                      (0x8288c3, 0x84a129), (0x8288ca, 0x908bf0), (0x8288ff, 0x84a129)]:
        require(s.u(a + 4, 3) == header, f'scene record {a:06x}')
        records.append({'source': a, 'header': header,
                        'position': [s.u(a + 1, 1) * 16 + 8, s.u(a + 2, 1) * 16]})
    require(s.u(0x8288ea, 3) == 0x908f23, 'tower introduction controller')
    p = s.cop(0x908bf5, 0xb3)
    return {'actors': records, 'intro_header_reference': 0x8288ea,
            'guardian_initial_offset': {'source': 0x908bf5, 'xy': [s.u(p), s.u(p + 2)]}}


def decode_proximity_handler(s):
    s.expect(0x80b475, [0xa7, 0x36, 0xe6, 0x36, 0x29, 0xff, 0, 0x1a])
    for a, values in [(0x80b48c, [0xc5, 0, 0xb0, 0x16]), (0x80b49d, [0xc5, 0, 0xb0, 5]),
                      (0x80b488, [0x49, 0xff, 0xff, 0x1a]), (0x80b499, [0x49, 0xff, 0xff, 0x1a])]:
        s.expect(a, values)
    return {'entry': 0x80b474, 'player_reference_words': [s.instruction(a, 0xed) for a in [0x80b483, 0x80b494]],
            'condition': 'abs_dx_and_abs_dy_lte_radius', 'requires_button': False}


# Half-open selected code/header windows; request payloads are deliberately absent.
RANGES = [
    (0x888beb, 0x888c51), (0x888ede, 0x888f6d), (0x8884ef, 0x88855f),
    (0x808695, 0x808720), (0x80963a, 0x8096cb),
    (0x84dee0, 0x84df12), (0x84e02a, 0x84e06c), (0x84e423, 0x84e480),
    (0x908bf0, 0x908d44), (0x908f23, 0x908f4d), (0x90fb0b, 0x90fb36),
    (0x80b474, 0x80b4b1),
    (0x85ae9c, 0x85aea1), (0x85af1f, 0x85af2c), (0x85af56, 0x85af5e),
    (0x85b006, 0x85b040), (0x85f4bd, 0x85f536), (0x848932, 0x84893f),
    (0x838cbe, 0x838cc8), (0x8388f9, 0x838929), (0x8288c1, 0x8288fd), (0x8288fd, 0x82890c),
]


def project(data):
    require(sha(data) == ROM_SHA, 'owned Japanese ROM authentication')
    s = TowerSource(data)
    route = [route_exit(s, m, dest) for m, dest in PATH]
    scenes = [scene(s, m) for m in [0xa, 0xb, 0xd, 3, 0x100, 0x101]]
    ranges = RANGES + [(r['table'], r['end']) for r in route]
    ranges += [(0x818000 + 2 * m, 0x818002 + 2 * m) for m, _ in PATH]
    ranges += [(a, a + 2) for r in scenes for a in [0x828000 + 2 * r['map'], 0x838000 + 2 * r['map']]]
    return {'rom_sha256': ROM_SHA, 'scope': 'frozen_return_to_map101_first_interior_entry',
            'route': route, 'scenes': scenes, 'records': decode_records(s),
            'elder': decode_elder(s), 'town': decode_town(s), 'world': decode_world(s),
            'guardian': decode_guardian(s), 'proximity_handler': decode_proximity_handler(s),
            'equipment': decode_equipment(s), 'ranges': window_hashes(s, ranges)}


if __name__ == '__main__':
    require(len(sys.argv) == 2, 'usage: tower_source.py OWNED_JP_ROM')
    result = project(Path(sys.argv[1]).read_bytes())
    require(result == json.loads((ROOT / 'tower-source.json').read_text()), 'tower source metadata mismatch')
    print('Tower approach through map101 entry: bounded operands and source-window hashes verified')
