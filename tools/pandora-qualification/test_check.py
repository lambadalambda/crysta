"""ROM-free red/green semantic and itinerary mutation controls."""
from copy import deepcopy
import unittest
import json
from pathlib import Path
from check import FINAL, STORY, timeline, validate, validate_discovery


def good():
    base = dict(map=0x41, position=[136, 208], events=FINAL, script=0x84a258,
                control=0, input_disable=0, phase='idle', held_slot=0)
    points = {
        '13-choice': dict(phase='choice', events=[]),
        '13-followup-4': dict(phase='ack', events=[]),
        '13-grant28': dict(events=[0x20, 0x26, 0x28, 0xfb]),
        'C-direct-ready': dict(events=STORY),
        'left-pot-held': dict(held_slot=0x98a, held_kind=0xfa, events=[]),
        'middle-held': dict(held_slot=0x98f, held_kind=0xfb, events=[]),
        'door-hit1': dict(hit_counter=0, events=[]),
        'door-real-hit1': dict(hit_counter=1, door_cells=[0x1da7, 0xb81], events=[]),
        'door-hit2': dict(hit_counter=2, events=[0x292]),
        'door-passable': dict(door_cells=[0x1cf6, 0x3acb], events=[]),
        '21-warning-complete-neutral': dict(events=[1, 2]),
        '21-opening-wait': dict(events=[0x22], input_disable=0xff50),
    }
    for label, m, pos in [('landed-E', 14, [152, 880]), ('landed-20', 32, [408, 880]),
                           ('landed-21', 33, [136, 128])]:
        points[label] = dict(map=m, position=pos, events=[])
    for label, pos in [('pandora-tour-control', [136, 208]), ('pandora-left-rest', [120, 208]),
                       ('pandora-up-rest', [120, 192]), ('pandora-neutral-stable', [120, 192])]:
        points[label] = dict(base, position=pos)
    return deepcopy(dict(points=points, tutorial_map_path=[0x41, 0x44, 0x42, 0x43, 0x41]))


class SemanticTests(unittest.TestCase):
    def test_accepts_complete_contract(self):
        validate(good())

    def test_rejects_semantic_mutations_without_hash_comparison(self):
        mutations = [
            ('13-choice', 'events', [0x28]),
            ('13-followup-4', 'events', [0x28]),
            ('13-followup-4', 'phase', 'idle'),
            ('13-grant28', 'events', [0x20, 0x26, 0xfb]),
            ('C-direct-ready', 'events', STORY + [0x2f]),
            ('left-pot-held', 'held_slot', 0),
            ('middle-held', 'held_kind', 0xfa),
            ('door-hit1', 'hit_counter', 1),
            ('door-real-hit1', 'events', [0x292]),
            ('door-real-hit1', 'door_cells', [0x1cf6, 0x3acb]),
            ('door-hit2', 'hit_counter', 1),
            ('door-hit2', 'events', []),
            ('door-passable', 'door_cells', [0x9cf6, 0xbacb]),
            ('landed-E', 'position', [136, 880]),
            ('landed-20', 'map', 0x21),
            ('21-warning-complete-neutral', 'events', [1, 2, 0x22]),
            ('21-opening-wait', 'input_disable', 0),
            ('pandora-tour-control', 'events', sorted(FINAL + [0x23, 0xfe])),
            ('pandora-tour-control', 'events', [v for v in FINAL if v != 0x244]),
            ('pandora-left-rest', 'position', [136, 208]),
            ('pandora-up-rest', 'position', [120, 208]),
            ('pandora-neutral-stable', 'position', [121, 192]),
            ('pandora-neutral-stable', 'script', 0x84a2a3),
            ('pandora-neutral-stable', 'phase', 'ack'),
            ('pandora-neutral-stable', 'input_disable', 0xff50),
            ('pandora-neutral-stable', 'held_slot', 0x98a),
        ]
        for label, key, value in mutations:
            with self.subTest(label=label, key=key), self.assertRaises(ValueError):
                report = good()
                report['points'][label][key] = value
                validate(report)

    def test_rejects_omitted_or_reordered_tutorial(self):
        for path in [[0x41], [0x41, 0x42, 0x44, 0x43, 0x41]]:
            with self.subTest(path=path), self.assertRaises(ValueError):
                report = good()
                report['tutorial_map_path'] = path
                validate(report)


class DiscoveryTests(unittest.TestCase):
    def test_controls_and_mutations(self):
        ref = json.loads(Path(__file__).with_name('discovery-reference.json').read_text())
        validate_discovery(ref)
        for label, key, value in [
            ('cellar-without28-blocked', 'events', [1, 32, 38, 40, 251]),
            ('resident13-refused', 'events', [1, 32, 38, 40, 251]),
            ('door-dash2-rest', 'hit_counter', 1),
            ('second-impact', 'hit_counter', 2),
            ('door-second-hit', 'hit_counter', 1),
            ('inside-box-up-rest', 'position', [120, 208]),
        ]:
            with self.subTest(label=label), self.assertRaises(ValueError):
                changed = deepcopy(ref)
                changed['points'][label][key] = value
                validate_discovery(changed)


class TimelineTests(unittest.TestCase):
    def fixture(self):
        commands = [dict(label='move', buttons=['Left'], frames=2), {'finish': True}]
        rows = [dict(kind='checkpoint', label='boot', frame=6800),
                dict(kind='frame', label='move', frame=6801),
                dict(kind='frame', label='move', frame=6802),
                dict(kind='checkpoint', label='move', frame=6802)]
        return commands, rows

    def test_complete(self):
        self.assertEqual(timeline(*self.fixture()), 6802)

    def test_rejects_missing_frame_sync_and_finish(self):
        commands, rows = self.fixture()
        for mutated in [rows[:-1], rows[:1] + rows[2:], rows + rows[-1:], rows[::-1]]:
            with self.subTest(rows=mutated), self.assertRaises(ValueError):
                timeline(commands, mutated)
        with self.assertRaises(ValueError):
            timeline(commands[:-1], rows)

    def test_rejects_unlabelled_extra_observation(self):
        commands, rows = self.fixture()
        rows[1]['kind'] = 'passive-save'
        with self.assertRaises(ValueError):
            timeline(commands, rows)


if __name__ == '__main__':
    unittest.main()
