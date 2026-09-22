"""ROM-free semantic mutations; exact evidence is tested separately."""
from copy import deepcopy
import json
from pathlib import Path
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

import check_departure as departure

ROOT = Path(__file__).parent


def good():
    return json.loads((ROOT / 'reference.json').read_text())


class SemanticTests(unittest.TestCase):
    def test_capture_contract(self):
        departure.validate(good())

    def test_semantic_mutations_without_hash_equality(self):
        mutations = [
            ('weapon-lane-up3-rest', 'position', [120, 192]),
            ('weapon-door-wait', 'map', 0x41),
            ('spear-request', 'phase', 'ack'),
            ('spear-talk', 'events', departure.FINAL),
            ('spear-page-5', 'phase', 'ack'),
            ('spear-refusal', 'cursor', 0xdc0d),
            ('spear-refusal-1', 'phase', 'ack'),
            ('spear-retry-choice', 'phase', 'choice'),
            ('spear-repeat-2', 'phase', 'ack'),
            ('spear-repeat-2', 'selection', 1),
            ('spear-accept-request', 'cursor', 0xdbe9),
            ('spear-accept-1', 'events', sorted(departure.FINAL + [0x240, 0x242])),
            ('spear-accepted-neutral', 'facing', 1),
            ('spear-collect-A', 'phase', 'text'),
            ('spear-reapproach-rest', 'facing', 0),
            ('spear-take-A', 'phase', 'idle'),
            ('spear-take-request', 'input_disable', 0),
            ('spear-take-1', 'script', 0x84a258),
            ('spear-presentation-wait', 'cursor', 0xdc29),
            ('return-sequence-2', 'map', 0xa),
            ('return-sequence-6', 'facing', 0),
            ('return-sequence-10', 'phase', 'ack'),
            ('return-sequence-12', 'input_disable', 0),
            ('return-sequence-19', 'cursor', 0xb2cd),
            ('return-sequence-19', 'script', 0x84a258),
            ('return-sequence-20', 'input_disable', 0xff50),
            ('frozen-return-control', 'map', 0xa),
            ('frozen-return-left-rest', 'position', [136, 464]),
            ('frozen-return-up-rest', 'position', [120, 464]),
            ('frozen-return-stable', 'facing', 0),
            ('frozen-return-stable', 'script', 0x84a2a3),
            ('frozen-return-stable', 'held_slot', 0x98a),
        ]
        ref = good()
        for label in ['spear-refusal-1', 'spear-accept-request', 'spear-accepted-neutral',
                      'spear-collect-request'] + [f'spear-grant-{i}' for i in range(1, 10)]:
            mutations += [(label, 'weapon_region', [0x81, 1] + [0] * 22),
                          (label, 'events', sorted(set(ref['points'][label]['events']) | {0x242}))]
        for label in ['spear-take-A', 'spear-take-request', 'frozen-return-stable']:
            mutations.append((label, 'events', [v for v in ref['points'][label]['events'] if v != 0x242]))
        mutations.append(('spear-take-A', 'weapon_region', [0x81, 1] + [0] * 22))
        for label in ['spear-take-request', 'spear-take-1', 'frozen-return-stable']:
            for region in [[0] * 24, [0x82, 1] + [0] * 22, [0x81, 2] + [0] * 22,
                           [0x81, 1, 1] + [0] * 21, [0x81, 1] + [0] * 21 + [1]]:
                mutations.append((label, 'weapon_region', region))
        for label in ['spear-refusal', 'return-sequence-2', 'return-sequence-19', 'return-sequence-20-A']:
            for flag in (0xfe, 0x23):
                mutations.append((label, 'events', sorted(ref['points'][label]['events'] + [flag])))
        for flag in (2, 3, 0xa, 0xfe, 0x23, 0x240, 0x241, 0x243, 0x244):
            mutations.append(('frozen-return-stable', 'events',
                              [v for v in ref['points']['frozen-return-stable']['events'] if v != flag]))
        for label, key, value in mutations:
            with self.subTest(label=label, key=key, value=value), self.assertRaises(ValueError):
                changed = deepcopy(ref)
                changed['points'][label][key] = value
                departure.validate(changed)

    def test_missing_extra_semantic_checkpoint(self):
        for extra in (False, True):
            changed = good()
            if extra:
                changed['points']['invented'] = deepcopy(changed['points']['spear-take-A'])
            else:
                del changed['points']['spear-grant-9']
            with self.assertRaises(ValueError):
                departure.validate(changed)


class EvidenceTests(unittest.TestCase):
    def test_recipe_and_accepted_prefix(self):
        recipe = (ROOT / 'route.jsonl').read_bytes()
        prefix = (departure.PANDORA / 'route.jsonl').read_bytes()
        departure.validate_recipe(recipe, recipe, prefix)
        for actual, retained in [(recipe + b'\n', recipe),
                                 (recipe.replace(b'"Left"', b'"Right"', 1), recipe.replace(b'"Left"', b'"Right"', 1))]:
            with self.assertRaises(ValueError):
                departure.validate_recipe(actual, retained, prefix)

    def test_entire_timeline_and_every_checkpoint(self):
        commands = [json.loads(line) for line in (ROOT / 'route.jsonl').read_text().splitlines()]
        rows = [dict(kind='checkpoint', label='boot', frame=6800)]
        frame = 6800
        for c in commands[:-1]:
            for _ in range(c['frames']):
                frame += 1
                rows.append(dict(kind='frame', label=c['label'], frame=frame))
            rows.append(dict(kind='checkpoint', label=c['label'], frame=frame))
        self.assertEqual(departure.timeline(commands, rows), good()['final_frame'])
        edge = next(i for i, row in enumerate(rows) if row['label'] == 'spear-accept-A')
        for mutated in [rows[1:], rows[:edge] + rows[edge + 1:], rows[:-1], rows + rows[-1:],
                        rows[:edge] + rows[edge:edge + 2][::-1] + rows[edge + 2:]]:
            with self.assertRaises(ValueError):
                departure.timeline(commands, mutated)
        with self.assertRaises(ValueError):
            departure.timeline(commands[:-1], rows)

    def test_strict_all_surfaces_metadata_and_provenance(self):
        ref = good()
        paths = [(key,) for key in ref if key != 'points']
        paths += [('points', 'spear-take-request', key) for key in ref['points']['spear-take-request']]
        paths += [('points', 'spear-take-request', 'hashes', ext) for ext in departure.SURFACES]
        paths += [(category, key) for category in ('provenance', 'observer_source_hashes') for key in ref[category]]
        for path in paths:
            for action in ('change', 'delete', 'extra'):
                with self.subTest(path=path, action=action), self.assertRaises(ValueError):
                    changed = deepcopy(ref)
                    node = changed
                    for key in path[:-1]:
                        node = node[key]
                    if action == 'delete':
                        del node[path[-1]]
                    else:
                        node[path[-1] if action == 'change' else 'unexpected'] = 'mutated'
                    departure.require_equal(changed, ref, 'retained evidence')

    def test_rom_authentication_precedes_source_and_captures(self):
        with patch.object(Path, 'read_bytes', return_value=b'not a JP ROM'), \
                patch.object(departure, 'evidence') as captures, \
                self.assertRaisesRegex(ValueError, 'Japanese ROM authentication'):
            departure.report('fake-ROM', 'unused')
        captures.assert_not_called()

    def test_report_source_pin_and_provenance_wiring(self):
        expected = {'window': 'right'}
        project = Mock(return_value=expected)
        # No helper installation or ROM required: exercise the real lazy import and report gates.
        with patch.dict(sys.modules, source_contract=SimpleNamespace(project=project)), \
                patch.object(Path, 'read_bytes', return_value=b'fake'), \
                patch.object(Path, 'read_text', return_value=json.dumps(expected)), \
                patch.object(departure, 'sha', return_value=departure.ROM_SHA), \
                patch.object(departure, 'pandora_project', return_value=expected), \
                patch.object(departure, 'evidence', return_value={'provenance': {}}) as captures:
            result = departure.report('fake-ROM', 'unused')
            project.assert_called_once_with(b'fake')
            captures.assert_called_once_with(b'fake', 'unused')
            self.assertEqual(result['source_metadata'], expected)
            for name in ('check_departure.py', 'test_departure.py', 'source_contract.py',
                         'test_source_contract.py', 'source.json', 'replay.sh'):
                self.assertIn(f'tower-approach-qualification/{name}', result['provenance'])
            self.assertNotIn('tower-approach-qualification/reference.json', result['provenance'])
            for changed in ({'window': 'wrong'}, {}, dict(expected, extra=1)):
                captures.reset_mock()
                project.return_value = changed
                with self.assertRaisesRegex(ValueError, 'source metadata mismatch'):
                    departure.report('fake-ROM', 'unused')
                captures.assert_not_called()

    def test_prefix_log_mutation_rejected_before_capture_reads(self):
        with patch.object(departure, 'checkpoint') as captures, \
                self.assertRaisesRegex(ValueError, 'accepted prefix log changed'):
            departure.check_prefix(b'', Path('unused'), [b'mutated\n'], [{'frame': 6800}], {}, {})
        captures.assert_not_called()

    def test_preserved_prefix_semantics_and_exact_comparison(self):
        ref = json.loads((departure.PANDORA / 'reference.json').read_text())
        departure.accepted.validate(ref)
        for label, key, value in [('pandora-neutral-stable', 'position', [121, 192]),
                                  ('13-grant28', 'events', []), ('door-hit2', 'hit_counter', 1)]:
            changed = deepcopy(ref)
            changed['points'][label][key] = value
            with self.assertRaises(ValueError):
                departure.accepted.validate(changed)
            with self.assertRaises(ValueError):
                departure.require_equal(changed, ref, 'accepted Pandora prefix changed')


if __name__ == '__main__':
    unittest.main()
