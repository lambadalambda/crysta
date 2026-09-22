"""ROM-free semantic, recipe and pipeline mutations for the first-tower segment."""
from copy import deepcopy
import json
from pathlib import Path
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

import check_tower as tower


def good():
    return json.loads((tower.ROOT / 'tower-reference.json').read_text())


class SemanticTests(unittest.TestCase):
    def test_native_contract(self):
        tower.validate(good())

    def test_mutations_without_reference_or_hash_comparison(self):
        ref = good()
        mutations = [
            ('return-stair-up-rest', 'position', [120, 383]),
            ('return-20-up-rest', 'position', [391, 864]),
            ('return-20-stair-rest', 'map', 0x20),
            ('return-E-stair-rest', 'map', 0xe),
            ('elder-talk-A', 'phase', 'text'), ('elder-request', 'position', [120, 704]),
            ('elder-near-A', 'facing', 1), ('elder-near-request', 'cursor', 0x8d48),
            ('elder-page-4', 'phase', 'ack'), ('elder-page-4', 'selection', 1),
            ('elder-accept-3', 'phase', 'idle'), ('elder-accept-4', 'phase', 'ack'),
            ('frozen-town-arrival', 'phase', 'idle'), ('frozen-town-page-2', 'position', [504, 769]),
            ('frozen-town-page-3', 'input_disable', 0), ('frozen-town-control', 'input_disable', 0xff50),
            ('underworld-arrival', 'map', 0xa), ('underworld-arrival', 'script', 0x84a258),
            ('underworld-west-rest', 'facing', 0), ('tower-align-east-rest', 'position', [184, 880]),
            ('tower-approach-arrival', 'input_disable', 0), ('tower-approach-arrival', 'map', 0x101),
            ('tower-door-up', 'position', [256, 959]), ('tower-door-wait', 'phase', 'idle'),
            ('tower-intro-1', 'cursor', 0x8f69), ('tower-guardian-request', 'phase', 'ack'),
            ('tower-guardian-approach', 'script', 0x84a258),
            ('guardian-page-1', 'script', 0x84a258), ('guardian-page-4', 'phase', 'ack'),
            ('guardian-page-4', 'selection', 1), ('guardian-response', 'cursor', 0x8dfd),
            ('guardian-response-6', 'script', 0x84a258), ('guardian-response-6', 'phase', 'ack'),
            ('guardian-complete-stable', 'script', 0x90fb23),
            ('first-tower-transition-control', 'map', 0x100),
            ('first-tower-left-rest', 'position', [128, 623]),
            ('first-tower-up-rest', 'position', [112, 623]),
            ('first-tower-neutral-stable', 'facing', 0),
            ('first-tower-neutral-stable', 'held_slot', 0x98a),
            ('first-tower-neutral-stable', 'script', 0x84a2a3),
            ('return-stair-up', 'equipped_weapon', 0x81),
            ('underworld-arrival', 'equipped_armor', 0xa1),
            ('first-tower-neutral-stable', 'equipped_weapon', 0x81),
        ]
        # Event21 starts a request, not consent; local handshakes are not final effects.
        for label in ('elder-talk-A', 'elder-near-A', 'elder-page-4', 'elder-accept-3',
                      'elder-accept-4', 'frozen-town-page-3', 'frozen-town-control',
                      'tower-approach-arrival', 'guardian-page-4', 'guardian-response-2',
                      'guardian-response-6-A', 'guardian-response-6', 'first-tower-neutral-stable'):
            for flag in (1, 2, 3, 0x21, 0x3c, 0x100, 0x115, 0x296):
                mutations.append((label, 'events', sorted(set(ref['points'][label]['events']) ^ {flag})))
        for label in ('return-stair-up', 'underworld-arrival', 'guardian-response', 'first-tower-neutral-stable'):
            for region in ([0] * 24, [0x82, 1] + [0] * 22, [0x81, 2] + [0] * 22,
                           [0x81, 1, 1] + [0] * 21, [0x81, 1] + [0] * 21 + [1]):
                mutations.append((label, 'weapon_region', region))
        for label, key, value in mutations:
            with self.subTest(label=label, key=key, value=value), self.assertRaises(ValueError):
                changed = deepcopy(ref)
                changed['points'][label][key] = value
                tower.validate(changed)

    def test_missing_extra_checkpoints_and_map_path(self):
        for extra in (False, True):
            changed = good()
            if extra:
                changed['points']['invented'] = changed['points']['guardian-page-4']
            else:
                del changed['points']['tower-door-up']
            with self.assertRaises(ValueError):
                tower.validate(changed)
        for path in ([0x21, 0x100], [0x21, 0x20, 0xe, 0xc, 0xd, 0xa, 3, 0x101, 0x100]):
            changed = good()
            changed['map_path'] = path
            with self.assertRaises(ValueError):
                tower.validate(changed)


class PipelineTests(unittest.TestCase):
    def test_recipe_exactness_and_accepted_prefix(self):
        recipe = (tower.ROOT / 'tower-route.jsonl').read_bytes()
        prefix = (tower.ROOT / 'route.jsonl').read_bytes()
        tower.validate_recipe(recipe, recipe, prefix)
        changed = recipe.replace(b'"Left"', b'"Right"', 1)
        for actual, retained in ((recipe + b'\n', recipe), (changed, changed)):
            with self.assertRaises(ValueError):
                tower.validate_recipe(actual, retained, prefix)

    def test_complete_schedule_and_mutations(self):
        commands = tower.commands(tower.ROOT / 'tower-route.jsonl')
        rows = [dict(kind='checkpoint', label='boot', frame=6800)]
        frame = 6800
        for c in commands[:-1]:
            for _ in range(c['frames']):
                frame += 1
                rows.append(dict(kind='frame', label=c['label'], frame=frame))
            rows.append(dict(kind='checkpoint', label=c['label'], frame=frame))
        self.assertEqual(tower.timeline(commands, rows), good()['final_frame'])
        at = next(i for i, r in enumerate(rows) if r['label'] == 'guardian-accept-A')
        for changed in (rows[1:], rows[:-1], rows + rows[-1:], rows[:at] + rows[at + 1:],
                        rows[:at] + rows[at:at + 2][::-1] + rows[at + 2:]):
            with self.assertRaises(ValueError):
                tower.timeline(commands, changed)
        with self.assertRaises(ValueError):
            tower.timeline(commands[:-1], rows)

    def test_inventory_projection_uses_exact_wram_slice(self):
        wram = bytearray([0x55] * 131072)
        wram[0x18048:0x18060] = bytes(range(24))
        with patch.object(tower.departure, 'checkpoint', return_value={'map': 0x101}) as shared, \
                patch.object(Path, 'read_bytes', return_value=bytes(wram)):
            point = tower.checkpoint(Path('unused'), {'label': 'edge'}, b'ROM')
        shared.assert_called_once_with(Path('unused'), {'label': 'edge'}, b'ROM')
        self.assertEqual(point, {'map': 0x101, 'weapon_region': list(range(24))})

    def test_equipment_words_are_not_inventory_or_0648(self):
        wram = bytearray(131072)
        wram[0x648:0x64e] = bytes.fromhex('810034127856')
        wram[0x18048:0x1804a] = bytes.fromhex('8101')
        with patch.object(tower, 'checkpoint', return_value={'weapon_region': tower.SPEAR}), \
                patch.object(Path, 'read_bytes', return_value=bytes(wram)):
            point = tower.current_checkpoint(Path('unused'), {'label': 'edge'}, b'ROM')
        self.assertEqual(point['equipped_weapon'], 0x1234)
        self.assertEqual(point['equipped_armor'], 0x5678)
        self.assertEqual(point['weapon_region'], tower.SPEAR)

    def test_evidence_enforces_artifact_gate_before_prefix_or_projection(self):
        # A tiny in-memory filesystem drives the real pipeline, not just its helper.
        cmds = [dict(label='edge', buttons=[], frames=1), {'finish': True}]
        recipe = b''.join(json.dumps(c).encode() + b'\n' for c in cmds)
        rows = [dict(kind='checkpoint', label='boot', frame=6800),
                dict(kind='frame', label='edge', frame=6801),
                dict(kind='checkpoint', label='edge', frame=6801)]
        raw = b''.join(json.dumps(r).encode() + b'\n' for r in rows)
        root = Path('mock-capture')
        names = [root / 'route.jsonl'] + [root / f'{label}.{ext}' for label in ('boot', 'edge') for ext in tower.SURFACES]
        def read_bytes(path):
            return raw if path == root.with_suffix('.jsonl') else recipe
        for files in (names, names[:-1], names + [root / 'unexpected.state']):
            with patch.object(Path, 'read_bytes', read_bytes), patch.object(Path, 'rglob', return_value=files), \
                    patch.object(Path, 'is_file', return_value=True), \
                    patch.object(tower, 'verify_prefix', side_effect=ValueError('prefix reached')) as prefix, \
                    patch.object(tower, 'checkpoint') as projection:
                if files == names:
                    with self.assertRaisesRegex(ValueError, 'prefix reached'):
                        tower.evidence(b'ROM', root)
                    prefix.assert_called_once()
                else:
                    with self.assertRaisesRegex(ValueError, 'missing/extra capture artifacts'):
                        tower.evidence(b'ROM', root)
                    prefix.assert_not_called()
                projection.assert_not_called()

    def test_prefix_log_gate(self):
        with self.assertRaisesRegex(ValueError, 'accepted departure prefix log changed'):
            tower.verify_prefix(b'', Path('unused'), [b'mutated\n'], [{'frame': 6800}])

    def test_prefix_reconstruction_preserves_points_sources_and_original_gates(self):
        ref = json.loads((tower.ROOT / 'reference.json').read_text())
        pandora_source = json.loads((tower.departure.PANDORA / 'source.json').read_text())
        rows = [dict(kind='checkpoint', label=label, frame=p['frame']) for label, p in ref['points'].items()]
        lines = [b'prefix'] + [b''] * (len(rows) - 1)
        real_sha = tower.sha
        for mutation in (None, 'inventory', 'surface', 'position', 'observer', 'provenance',
                         'extra-provenance', 'source', 'extra-source'):
            points = deepcopy(ref['points'])
            sources = deepcopy(ref['observer_source_hashes'])
            provenance = deepcopy(ref['provenance'])
            metadata = deepcopy(ref['source_metadata'])
            endpoint = points['frozen-return-stable']
            if mutation == 'inventory':
                endpoint['weapon_region'][1] = 2
            elif mutation == 'surface':
                endpoint['hashes']['pixels'] = 'changed'
            elif mutation == 'position':
                endpoint['position'][0] += 1
            elif mutation == 'observer':
                sources[next(iter(sources))] = 'changed'
            elif mutation == 'provenance':
                provenance['tower-approach-qualification/check_departure.py'] = 'changed'
            elif mutation == 'extra-provenance':
                provenance['extra'] = 'unexpected'
            elif mutation == 'source':
                metadata = {}
            elif mutation == 'extra-source':
                metadata['extra'] = True
            with self.subTest(mutation=mutation), \
                    patch.dict(sys.modules, source_contract=SimpleNamespace(project=lambda _: metadata)), \
                    patch.object(tower.departure, 'pandora_project', return_value=pandora_source), \
                    patch.object(tower.departure, 'observer_sources', return_value=sources), \
                    patch.object(tower, 'hashes', return_value=provenance), \
                    patch.object(tower, 'sha', side_effect=lambda b: ref['frame_log_sha256'] if b == b'prefix' else real_sha(b)), \
                    patch.object(tower, 'timeline', return_value=tower.PREFIX_END) as schedule, \
                    patch.object(tower, 'checkpoint', side_effect=lambda root, row, rom: points[row['label']]), \
                    patch.object(tower.departure, 'check_prefix') as original:
                if mutation is None:
                    self.assertEqual(tower.verify_prefix(b'ROM', Path('unused'), lines, rows), ref)
                    original.assert_called_once()
                    schedule.assert_called_once_with(tower.commands(tower.ROOT / 'route.jsonl'), rows)
                else:
                    with self.assertRaises(ValueError):
                        tower.verify_prefix(b'ROM', Path('unused'), lines, rows)

    def test_authentication_and_lazy_source_pin(self):
        with patch.object(Path, 'read_bytes', return_value=b'wrong ROM'), \
                self.assertRaisesRegex(ValueError, 'Japanese ROM authentication'):
            tower.report('unused', 'unused')
        metadata = {'window': 'retained'}
        project = Mock(return_value=metadata)
        with patch.dict(sys.modules, tower_source=SimpleNamespace(project=project)), \
                patch.object(Path, 'read_bytes', return_value=b'ROM'), \
                patch.object(Path, 'read_text', return_value=json.dumps(metadata)), \
                patch.object(tower, 'sha', return_value=tower.ROM_SHA), \
                patch.object(tower, 'evidence', return_value={'provenance': {}}) as evidence:
            result = tower.report('unused', 'unused')
            self.assertEqual(result['source_metadata'], metadata)
            project.assert_called_once_with(b'ROM')
            for name in ('check_tower.py', 'test_tower.py', 'tower_source.py', 'test_tower_source.py', 'tower-source.json', 'replay-tower.sh'):
                self.assertIn(f'tower-approach-qualification/{name}', result['provenance'])
            self.assertNotIn('tower-approach-qualification/tower-reference.json', result['provenance'])
            for changed in ({}, {'window': 'wrong'}, dict(metadata, extra=1)):
                project.return_value = changed
                evidence.reset_mock()
                with self.assertRaisesRegex(ValueError, 'tower source metadata mismatch'):
                    tower.report('unused', 'unused')
                evidence.assert_not_called()


class ExactTests(unittest.TestCase):
    def test_all_fields_surfaces_provenance_missing_and_extra(self):
        ref = good()
        paths = [(key,) for key in ref if key != 'points']
        paths += [('points', 'first-tower-neutral-stable', key) for key in ref['points']['first-tower-neutral-stable']]
        paths += [('points', 'first-tower-neutral-stable', 'hashes', ext) for ext in tower.SURFACES]
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
                        node[path[-1] if action == 'change' else 'extra'] = 'mutated'
                    tower.require_equal(changed, ref, 'retained tower evidence')


if __name__ == '__main__':
    unittest.main()
