"""ROM-free mutation tests for the semantic qualification checker."""
import copy
import json
import unittest
from check import ROOT, validate


class SemanticMutations(unittest.TestCase):
    def setUp(self):
        self.good = json.loads((ROOT/'reference.json').read_text())

    def reject(self, mutate):
        bad = copy.deepcopy(self.good)
        mutate(bad)
        with self.assertRaises(ValueError):
            validate(bad)

    def test_reference(self):
        validate(self.good)

    def test_early_entry_grant(self):
        self.reject(lambda r: r['checkpoints']['entry-no-ack']['events'].insert(1,38))

    def test_grant_before_acknowledgement(self):
        self.reject(lambda r: r['checkpoints']['first-no-ack']['events'].insert(1,38))

    def test_grant_delayed_until_choice_completion(self):
        self.reject(lambda r: r['checkpoints']['choice-no-confirm']['events'].remove(38))

    def test_choice_missing(self):
        self.reject(lambda r: r['checkpoints']['repeat-page'].update(phase='idle'))

    def test_cancel_changes_another_event(self):
        self.reject(lambda r: r['checkpoints']['repeat-cancel-page']['events'].append(39))

    def test_wrong_choice_branch(self):
        self.reject(lambda r: r['checkpoints']['repeat2-followup'].update(cursor=0x91eb))

    def test_b_button_incorrectly_advances_page(self):
        self.reject(lambda r: r['checkpoints']['first-complete'].update(cursor=0x9126))

    def test_missing_facing_control(self):
        self.reject(lambda r: r['checkpoints']['away-negative'].update(facing=1))

    def test_wrong_target_geometry(self):
        self.reject(lambda r: r['checkpoints']['entry-no-ack']['target']['rectangle'].__setitem__(1,15))

    def test_gate_occupancy(self):
        self.reject(lambda r: r['checkpoints']['open-D'].update(gate_cell=0x8592))

    def test_gate_membership_not_stale_slot(self):
        self.reject(lambda r: r['checkpoints']['open-D'].update(gate_continuation_present=True))

    def test_source_spawn_is_not_settled(self):
        self.reject(lambda r: r['checkpoints']['landed-A'].update(position=[504,752]))

    def test_old_map_coordinates_are_not_spawn(self):
        self.reject(lambda r: r['transition'][3].update(position=[120,738]))

    def test_wrong_walking_endpoint(self):
        self.reject(lambda r: r['checkpoints']['exterior-walk-settled'].update(position=[504,769]))

    def test_missing_checkpoint(self):
        self.reject(lambda r: r['checkpoints'].pop('first-no-ack'))


if __name__ == '__main__':
    unittest.main()
