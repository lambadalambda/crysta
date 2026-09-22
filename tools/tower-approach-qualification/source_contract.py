#!/usr/bin/env python3
"""Bounded JP source evidence: map41 door -> map42 spear -> map21 frozen return.

project(rom_bytes) authenticates the entire unheadered Japanese ROM before decoding.
The decode_* helpers accept synthetic/untrusted Source objects for ROM-free tests;
they check selected instruction shapes, NOT ROM authenticity or runtime success.
No event/text VM: only finite source sites, typed operands and half-open hashes.
Run this file with an owned ROM path to verify against the pinned source.json.
"""
import importlib.util
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent
# File loading avoids both importing and replacing a parent's unrelated source.py.
_spec = importlib.util.spec_from_file_location('_pandora_departure_operands',
    ROOT.parent / 'pandora-qualification/source.py')
_pandora = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_pandora)
require, sha, ROM_SHA = _pandora.require, _pandora.sha, _pandora.ROM_SHA


class DepartureSource(_pandora.Source):
    def branch(self, address):
        p = self.cop(address, 8)
        value = self.u(p)
        return {'source': address, 'event': value & 0xfff, 'when_set': bool(value & 0x8000),
                'target': (address & 0xff0000) | self.u(p + 2)}

    def membership(self, address):
        value = self.u(self.cop(address, 0x48))
        return {'source': address, 'event': value & 0xfff, 'require_set': not bool(value & 0x8000)}

    def wait_flag(self, address):
        value = self.u(self.cop(address, 5))
        return {'source': address, 'event': value & 0xfff, 'until_set': not bool(value & 0x8000)}

    def callback(self, address):
        value = self.u(self.cop(address, 0x21))
        return {'source': address, 'target': (address & 0xff0000) | value if value else 0}

    def pointer(self, address, selector):
        return {'source': address, 'target': self.u(self.cop(address, selector), 3)}

    def request_long(self, address):
        p = self.cop(address, 0x1c)
        return {'source': address, 'request': (self.u(p, 1) << 16) | self.u(p + 1)}

    def award(self, address):
        p = self.cop(address, 0x60)
        return {'source': address, 'item': self.u(p, 1),
                'presentation': self.u(p + 1), 'worker': self.u(p + 3, 1)}

    def delay(self, address):
        return {'source': address, 'ticks': self.u(self.cop(address, 0xc1))}

    def control(self, address, selector):
        return {'source': address, 'selector': selector, 'mask': self.u(self.cop(address, selector))}

    def mask(self, address, setting):
        self.expect(address, [0xa9])  # LDA #word; TSB/TRB absolute
        self.expect(address + 3, [0x0c if setting else 0x1c])
        return {'source': address, 'address': self.u(address + 4),
                'mask': self.u(address + 1), 'set': setting}

    def marker(self, address, selector):
        self.cop(address, selector)
        return {'source': address, 'selector': selector}

    def instruction(self, address, opcode, size=2):
        self.expect(address, [opcode])
        return self.u(address + 1, size)


def decode_departure(s):
    spark = s.cop(0x89d9fe, 0xb2)
    return {
        'transitions': [s.transition(a) for a in [0x89dcb5, 0x89da49]],
        'spark': {'source': 0x89d9fe, 'offset': int.from_bytes(s.b(spark, 2), 'little', signed=True),
                  'membership': s.membership(0x89da07)},
        'interaction': s.marker(0x89da17, 0x3b),
        'callback': s.callback(0x89da19), 'clear_callback': s.callback(0x89daa0),
        'branches': [s.branch(a) for a in [0x89da56, 0x89da5c]],
        'choices': [s.choice(a) for a in [0x89da6c, 0x89da78]],
        'effects': [s.flag(a) for a in [0x89da62, 0x89da91, 0x89da9c, 0x89da37]],
        'requests': [s.request(a) for a in [0x89da66, 0x89da72, 0x89da84, 0x89da8b, 0x89daa8]]
                    + [s.request_long(0x89da2a), s.request(0x89da3f)],
        'dialogue_waits': [s.marker(a, 0x1f) for a in
                           [0x89da6a, 0x89da76, 0x89da88, 0x89da8f, 0x89daac, 0x89da2f, 0x89da43]],
        'masks': [s.mask(0x89da96, True), s.mask(0x89da31, False)],
        'controls': [s.control(0x89daa4, 0x2a)],
        'collection_jump': s.pointer(0x89daae, 0xc0), 'award': s.award(0x89da20),
        'delays': [s.delay(a) for a in [0x89da26, 0x89da3b, 0x89da45]],
    }


def decode_return(s):
    return {
        'membership': [s.membership(a) for a in [0x88aebd, 0x88aec1, 0x88b2ff]],
        'branches': [s.branch(a) for a in [0x88aec9, 0x88aef1, 0x88af08, 0x88b303]],
        'effects': [s.flag(a) for a in [0x88aeed, 0x88af3f, 0x88af43, 0x88b377, 0x88b3a3]],
        'flag_waits': [s.wait_flag(a) for a in [0x88b321, 0x88b3a7]],
        'movement_waits': [
            {'source': a, 'selector': 0x3a, 'animation_control': s.u(s.cop(a, 0x3a), 1),
             'motion_stream_index': s.u(a + 3, 1),
             'target_tile_y': int.from_bytes(s.b(a + 4, 1), 'little', signed=True)}
            for a in [0x88b32e, 0x88b34c]],
        'requests': [s.request(a) for a in
                     [0x88af39, 0x88b33e, 0x88b346, 0x88b361, 0x88b36c, 0x88b392, 0x88b39d]],
        'dialogue_waits': [s.marker(a, op) for a, op in
                           [(0x88af3d, 0x20), (0x88b342, 0x1f), (0x88b34a, 0x1f),
                            (0x88b365, 0x20), (0x88b370, 0x1f), (0x88b396, 0x1f), (0x88b3a1, 0x20)]],
        'calls': [s.pointer(a, 0) for a in [0x88af03, 0x88b372]],
        'presentation_calls': [s.pointer(a, 0xdf) for a in [0x88af30, 0x88b35c, 0x88b398]],
        'delays': [s.delay(a) for a in [0x88aee9, 0x88af35]],
        'controls': [s.control(0x88aec5, 0x2a), s.control(0x88af47, 0x29)],
        'end': [s.marker(0x88af4b, 0x3c), s.marker(0x88af4d, 0xa7)],
    }


def decode_inventory(s):
    """Selected native evidence for item81; not an inventory implementation.

    Successful insertion is conditional: existing ID wins, else first empty;
    full region or quantity >= cap returns carry set. COP60 ignores that carry.
    """
    latch = s.instruction(0x809a0c, 0x8d)
    require(s.instruction(0x8d9659, 0xad) == latch, 'inventory item latch')
    call = s.instruction(0x809a0f, 0x22, 3)
    require(call == 0x8d9653, 'COP60 inventory call')
    s.expect(0x809a13, [0xda])  # PHX, not a failure-carry branch
    s.expect(0x8d965c, [0xe2, 0x20])  # M8 before region resolver; X remains 16-bit
    resolver = 0x8d0000 | s.instruction(0x8d965f, 0x20)
    require(resolver == 0x8d9732, 'inventory region resolver')
    # Only the item81 path: unsigned M8 comparisons, ending at weapon region.
    for a, values in [(0x8d9732, [0xc9, 0x10, 0xb0, 0x10]),
                      (0x8d9746, [0xc9, 0x80, 0xb0, 0x15]),
                      (0x8d975f, [0xc9, 0xa0, 0xb0, 0x15, 0xc9, 0x9c, 0x90, 0x0a]),
                      (0x8d9776, [0x80, 0x13]), (0x8d978b, [0x85, 4, 0x64, 5, 0x60]),
                      (0x8d9666, [0xc3, 1, 0xf0, 0x1c]), (0x8d966a, [0xc9, 0]),
                      (0x8d9672, [0x86, 0, 0xe6, 0]),
                      (0x8d9676, [0xe8, 0xe8, 0xe4, 4, 0x90, 0xe6]),
                      (0x8d9683, [0x38, 0x6b, 0xca]), (0x8d968c, [0xb0, 0xf2]),
                      (0x8d9697, [0x1a]), (0x8d969e, [0x18, 0x6b])]:
        s.expect(a, values)
    start = s.instruction(0x8d9771, 0xa2)
    end = s.instruction(0x8d9774, 0xa9, 1)  # M8 LDA #$60, NOT #$8060
    base = s.instruction(0x8d9662, 0xbf, 3)
    item_write = s.instruction(0x8d968f, 0x9f, 3)
    quantity_write = s.instruction(0x8d9698, 0x9f, 3)
    require(item_write == base and quantity_write == base + 1, 'inventory write layout')
    require(s.instruction(0x8d9686, 0xbf, 3) == quantity_write and
            s.instruction(0x8d9693, 0xbf, 3) == quantity_write, 'inventory quantity reads')
    return {
        'item_latch': latch, 'call': {'source': 0x809a0f, 'target': call},
        'resolver_call': {'source': 0x8d965f, 'target': resolver},
        'region': {'item': 0x81, 'accumulator_bits': 8, 'start': start,
                   'end_exclusive': end, 'stride': 2, 'base': base},
        'selection': 'existing_id_else_first_empty', 'quantity_cap': s.instruction(0x8d968a, 0xc9, 1),
        'writes': [{'source': 0x8d968f, 'base': item_write, 'value': 'item'},
                   {'source': 0x8d9698, 'base': quantity_write, 'value': 'quantity+1'}],
        'first_empty_weapon_slot': {'address': base + start, 'item': 0x81,
                                    'quantity_address': base + start + 1, 'quantity': 1},
        'failure': 'carry_set_if_full_or_at_cap', 'cop60_checks_failure_carry': False,
        'requires_runtime_inventory_proof': True,
        'presentation_tail': 0x800000 | s.instruction(0x809a5b, 0x4c),
    }


def decode_handlers(s):
    # The COP20 call may keep the request live; it is not a generic COP1F wait.
    helper = s.instruction(0x808ca8, 0x22, 3)
    s.expect(0x808cad, [0x90, 0x0d])
    s.expect(0x808cb4, [0x3a, 0x3a])
    cleared = s.instruction(0x808cbc, 0x9c)
    # COP1C consumes BANK first, then the address word via the shared tail.
    s.expect(0x808c3e, [0xa7, 0x36, 0xe6, 0x36, 0x29, 0xff, 0])
    bank_latch = s.instruction(0x808c45, 0x8d)
    s.expect(0x808c48, [0x80, 0xbc])
    s.expect(0x808c0f, [0xa7, 0x36, 0xe6, 0x36, 0xe6, 0x36])
    word_latch = s.instruction(0x808c15, 0x8d)
    require(bank_latch == cleared, 'request bank latch')
    s.expect(0x808631, [0x30, 7])
    s.expect(0x808636, [0x90, 7])
    s.expect(0x80863d, [0x90, 0x0a])
    s.expect(0x808641, [0x3a, 0x3a])
    flag_test = 0x800000 | s.instruction(0x808633, 0x20)
    require(s.instruction(0x80863a, 0x20) == flag_test & 0xffff, 'flag wait test')
    s.expect(0x8092b5, [0xf0, 0xcd])  # Reached target -> shared advancement at 9284
    return {
        'cop20': {'entry': 0x808c9a, 'call_source': 0x808ca8, 'helper': helper,
                  'retry_on_carry_set': True, 'retry_source': 0x808caf,
                  'completion_source': 0x808cbc, 'clears': cleared},
        'cop1c': {'entry': 0x808c28, 'shared_tail': 0x808c06, 'operand_order': 'bank_then_word',
                  'bank_latch': bank_latch, 'word_latch': word_latch},
        'cop05': {'entry': 0x80862e, 'flag_test': flag_test, 'positive_waits_until_set': True},
        'cop3a': {'entry': 0x80929c, 'reached_target_tail': 0x809284,
                  'coordinate_helper': 0x800000 | s.instruction(0x8092af, 0x20),
                  'motion_helper': 0x800000 | s.instruction(0x8092e9, 0x20)},
    }


def decode_scenes(s):
    maps = [0x41, 0x42, 0x21]
    for m in maps:
        require(s.u(0x828000 + 2 * m) == 0, 'nonzero bank82 scene override')
    records = []
    for a, header in [(0x83953a, 0x89dc9d), (0x83957c, 0x89d9f9), (0x839586, 0x89da12),
                      (0x839271, 0x88b2fa), (0x83927b, 0x88aeb8)]:
        require(s.u(a + 4, 3) == header, f'scene header {a:06x}')
        records.append({'source': a, 'header': header, 'entry': header + 5})
    return {'maps': [{'map': m, 'scene': 0x830000 | s.u(0x838000 + 2 * m)} for m in maps],
            'records': records}


def decode_presentation(s):
    workers = []
    for a in [0x88b3b4, 0x88b3bb, 0x88b3c2, 0x88b3c9, 0x88b3d0, 0x88b3d7]:
        p = s.cop(a, 0xa2)
        target = s.u(p, 3)
        require(s.instruction(target, 0x20) == 0xb60f, 'presentation worker helper')
        s.cop(target + 23, 0xa7)
        workers.append({'source': a, 'target': target,
                        'parameters': [s.u(p + 3, 1), s.u(p + 4, 1)]})
    return {'spawned_workers': workers, 'role': 'presentation_only_not_handshake_flag_writers',
            'shared_helper': 0x88b60f,
            'fade_call': s.pointer(0x88af03, 0)}


# All endpoints are EXCLUSIVE. No request payload/text windows are exported.
RANGES = [
    (0x89d9f9, 0x89dab4), (0x89dc9d, 0x89dcc2),
    (0x88aeb8, 0x88afa6), (0x88b2fa, 0x88b3e0), (0x88b507, 0x88b54a),
    (0x88b573, 0x88b61e),  # Six presentation workers and their shared helper
    (0x809a04, 0x809a5e), (0x8090d6, 0x809106),
    (0x8d9653, 0x8d96a0), (0x8d9732, 0x8d9790),
    (0x808c06, 0x808c4a), (0x808c9a, 0x808cc8), (0x80862e, 0x80864c),
    (0x809284, 0x809302), (0x80bba6, 0x80bbc7),
    (0x80bbd3, 0x80bc43),  # Flag masks, animation setup and signed tile-coordinate helper
    (0x85910e, 0x85922f),  # COP20 request lifecycle/dispatch, not a text projection
    (0x83953a, 0x839544), (0x83957c, 0x839590), (0x839271, 0x839285),
] + [(a, a + 2) for m in [0x41, 0x42, 0x21] for a in [0x828000 + m * 2, 0x838000 + m * 2]]


def window_hashes(s, ranges):
    return [{'start': a, 'end': z, 'sha256': sha(s.b(a, z - a))} for a, z in ranges]


def project(data):
    """Authenticate JP bytes (not a filename/caller assertion), then project evidence."""
    require(sha(data) == ROM_SHA, 'owned Japanese ROM authentication')
    s = DepartureSource(data)
    return {'rom_sha256': ROM_SHA, 'scope': 'map41_left_door_map42_spear_map21_frozen_return',
            'scenes': decode_scenes(s), 'departure': decode_departure(s),
            'frozen_return': decode_return(s), 'inventory': decode_inventory(s),
            'handlers': decode_handlers(s), 'presentation': decode_presentation(s),
            'ranges': window_hashes(s, RANGES)}


if __name__ == '__main__':
    require(len(sys.argv) == 2, 'usage: source_contract.py OWNED_JP_ROM')
    result = project(Path(sys.argv[1]).read_bytes())
    require(result == json.loads((ROOT / 'source.json').read_text()), 'source metadata mismatch')
    print('Tower approach departure/frozen-return operands and source-window hashes verified')
