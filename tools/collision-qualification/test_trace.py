#!/usr/bin/env python3
"""ROM-free controls for the bounded COP CA branch witness checker."""
import copy
import unittest

from check_trace import load_witness, validate_pair
from pathlib import Path


def witness(moved):
    before = {'kind': 'frame', 'frame': 10, 'map': 15, 'position': [100, 112],
              'held': ['Right'], 'facing': 3, 'control': 160}
    after = dict(before, frame=11, position=[101 if moved else 100, 112])
    addresses = [0x848E6C, 0x80AD39, 0x80ADAD,
                 0x80ADAF if moved else 0x80ADB7, 0x848E76, 0x80D107]
    meta = {'kind': 'trace', 'trace': 'open' if moved else 'wall',
            'frames': [10, 11], 'held': ['Right'], 'stop': 'FrameLimit',
            'instructions': len(addresses)}
    return before, meta, after, addresses


class TraceControls(unittest.TestCase):
    def test_both_branches_with_observed_movement(self):
        validate_pair(witness(True), witness(False))

    def test_metadata_and_context_mutations_are_rejected(self):
        mutations = [
            (1, 'stop', 'InstructionLimit'), (1, 'frames', [10, 12]),
            (1, 'instructions', 1), (1, 'held', ['Left']),
            (2, 'frame', 12), (2, 'map', 16), (2, 'control', 0),
            (0, 'held', []), (2, 'facing', 2),
            (2, 'position', [100, 112]),
        ]
        for index, key, value in mutations:
            with self.subTest(key=key, value=value):
                changed = copy.deepcopy(witness(True))
                changed[index][key] = value
                with self.assertRaises(ValueError):
                    validate_pair(changed, witness(False))

    def test_wrong_branch_missing_call_and_absent_resolver_are_rejected(self):
        for index in range(6):
            changed = copy.deepcopy(witness(True))
            changed[3][index] = 0
            with self.subTest(index=index), self.assertRaises(ValueError):
                validate_pair(changed, witness(False))

    def test_a_moving_refusal_witness_is_rejected(self):
        changed = copy.deepcopy(witness(False))
        changed[2]['position'] = [101, 112]
        with self.assertRaises(ValueError):
            validate_pair(witness(True), changed)

    def test_non_numeric_positions_and_non_object_rows_are_rejected(self):
        changed = copy.deepcopy(witness(True))
        changed[0]['position'] = ['100', '112']
        changed[2]['position'] = ['101', '112']
        with self.assertRaises(ValueError):
            validate_pair(changed, witness(False))
        with self.assertRaises(ValueError):
            load_witness([None], Path('.'), 'right-admitted')

    def test_pair_must_share_map_and_facing(self):
        changed = copy.deepcopy(witness(False))
        changed[0]['map'] = changed[2]['map'] = 10
        with self.assertRaises(ValueError):
            validate_pair(witness(True), changed)


if __name__ == '__main__':
    unittest.main()
