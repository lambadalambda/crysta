"""ROM-free selected instruction fragments: no ROM/text/disassembly fixture."""
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import tower_source as t


def source(parts):
    size = max((a & 0x3fffff) + len(bytes.fromhex(v)) for a, v in parts.items())
    data = bytearray(size)
    for a, v in parts.items():
        b = bytes.fromhex(v)
        at = a & 0x3fffff
        data[at:at + len(b)] = b
    return t.TowerSource(bytes(data))


ELDER = {
    0x888bf0: '02 47 23 10 21 80', 0x888bf6: '02 07 14 80',
    0x888bfc: '02 21 17 8c', 0x888c17: '02 07 21 80',
    0x888c1b: '02 1b 51 8c 02 1f', 0x888c21: '02 1a 01 27 8c',
    0x888c27: '41 8c 2d 8c 41 8c', 0x888c2d: '02 1b 1f 8d 02 1f',
    0x888c33: '02 07 96 82', 0x888c37: '02 21 00 00',
    0x888c3b: '02 c0 0b 8c 88', 0x888c41: '02 1b d4 8d 02 1f',
    0x888c47: '02 21 00 00', 0x888c4b: '02 c0 0b 8c 88',
    0x888ede: '02 08 09 81 be 8f', 0x888ee4: '02 08 3b 80 ea 8e',
    0x888eea: '02 08 96 82 6d 8f', 0x888ef0: '02 08 21 80 49 8f',
    0x888f49: '02 1b 41 92 02 1f', 0x888f4f: '02 1a 01 55 8f',
    0x888f55: '66 8f 5b 8f 66 8f', 0x888f5b: '02 1b 70 92 02 1f',
    0x888f61: '02 07 96 82', 0x888f66: '02 1b 9c 92 02 1f',
}
TOWN = {
    0x8884ef: '02 09 96 12 3c 00 0f 85', 0x88850f: '02 07 14 80',
    0x888513: '02 2a 50 ff', 0x888517: '02 c1 78 00',
    0x88851b: '02 1b 5f 85 02 1f', 0x888521: '02 df 41 85 88',
    0x888526: '02 c1 64 00', 0x88852a: '02 1b 7b 85 02 20',
    0x888530: '02 c1 20 00', 0x888534: '02 29 50 ff',
    0x888538: '02 07 3c 80', 0x88853c: '02 bf f7 84 88',
}
GUARDIAN = {
    0x908bfb: '02 08 96 81 1f 90', 0x908c03: '02 d6 40 97 8c',
    0x908c0a: '02 d6 40 10 8c', 0x908c10: '02 08 15 81 7f 8c',
    0x908c16: '02 df 0b fb 90', 0x908c1b: '02 a2 9f 8c 90 00 91',
    0x908c22: '02 05 01 00', 0x908c26: '02 1b 44 8d 02 1f',
    0x908c2c: '02 07 02 80', 0x908c30: '02 1b 75 8d 02 1f',
    0x908c36: '02 07 02 00', 0x908c3a: '02 1b a4 8d 02 1f',
    0x908c40: '02 1a 01 45 8c', 0x908c45: '5d 8c 4b 8c 5d 8c',
    0x908c4b: '02 1b d2 8d 02 1f', 0x908c51: '02 07 02 80',
    0x908c55: '02 1b 11 8e 02 1f', 0x908c5b: '80 10',
    0x908c5d: '02 1b 2c 8e 02 1f', 0x908c63: '02 07 02 80',
    0x908c67: '02 1b 78 8e 02 1f', 0x908c6d: '02 07 03 80',
    0x908c71: '02 c1 3c 00', 0x908c75: '02 07 15 81',
    0x908c79: '02 cb 01 c1 87 84',
    0x908ccb: '02 07 01 80', 0x908cd4: '02 05 02 00',
    0x908ce2: '02 08 03 80 fa 8c', 0x908ce8: '02 08 02 00 cf 8c',
    0x908cee: 'ad a4 0d 29 30 00', 0x908cf6: '02 e4',
    0x908d07: '02 c1 3c 00', 0x908d0b: '02 a7',
    0x908f28: '02 48 00 81', 0x908f2c: '02 07 00 81',
    0x908f30: '02 2a f0 ff', 0x908f40: '02 1b 4d 8f 02 1f',
    0x908f46: '02 29 f0 ff', 0x908f4a: '02 a7',
}
WORLD = {
    0x84def2: '02 9b 2a e0 84 00 84', 0x84defe: '02 07 f0 00',
    0x84df02: '02 05 f2 80', 0x84e02a: '02 08 f2 80 0f e5',
    0x84e445: '02 08 f2 00 73 e4', 0x84e473: '02 05 f2 00',
}
EQUIPMENT = {
    0x85ae9c: 'c9 80 00 b0 7e', 0x85af1f: 'c9 a0 00 b0 32',
    0x85af24: 'ad 4a 06 cd d0 0d', 0x85af56: 'ad 4c 06 cd d0 0d',
    0x85b006: 'a9 00 00 8d 4a 06', 0x85b015: 'ad d0 0d 8d 4a 06',
    0x85b024: 'a9 00 00 8d 4c 06', 0x85b033: 'ad d0 0d 8d 4c 06',
    0x85f4bf: 'c2 20', 0x85f4e2: 'ad 4a 06 f0 1d',
    0x85f504: 'ad 4c 06 f0 2d', 0x848932: 'ad 4a 06 f0 b1',
}


class OperandTests(unittest.TestCase):
    def test_xor_admission_differs_from_branch_polarity(self):
        s = source({**ELDER, **TOWN})
        self.assertEqual(s.xor_gate(0x888bf0, 0x47),
                         {'source': 0x888bf0, 'events': [0x23, 0x21], 'operation': 'xor', 'require_true': True})
        self.assertEqual(s.xor_gate(0x8884ef, 9),
                         {'source': 0x8884ef, 'events': [0x296, 0x3c], 'operation': 'xor',
                          'when_true': True, 'target': 0x88850f})

    def test_xor_mutations_and_unsupported_operator(self):
        s = source({0x888000: '02 09 97 12 3d 80 10 85'})
        self.assertEqual(s.xor_gate(0x888000, 9)['events'], [0x297, 0x3d])
        self.assertFalse(s.xor_gate(0x888000, 9)['when_true'])
        for fragment in ['02 09 96 22 3c 00 0f 85', '02 09 96 12 3c 00 0f', '02 48 96 12 3c 00']:
            with self.subTest(fragment=fragment), self.assertRaises(ValueError):
                source({0x888000: fragment}).xor_gate(0x888000, 9)

    def test_proximity_not_a_button_gate(self):
        gate = source({0x908c0a: '02 d6 40 10 8c'}).proximity(0x908c0a)
        self.assertEqual(gate, {'source': 0x908c0a, 'radius': 64, 'target': 0x908c10})
        with self.assertRaises(ValueError):
            source({0x908c0a: '02 d6 40 10'}).proximity(0x908c0a)


class RouteTests(unittest.TestCase):
    def test_exit_table_resolution_and_12_byte_stride(self):
        s = source({0x818006: 'c1 8c', 0x818cc1: '00 00 01 01 0a 00 00 00 00 00 00 00',
                    0x818ccd: '0d 31 01 02 00 01 00 66 f8 00 e0 03 ff'})
        r = t.route_exit(s, 3, 0x100)
        self.assertEqual(r, {'from_map': 3, 'table': 0x818cc1, 'end': 0x818cda,
                            'exit': {'source': 0x818ccd, 'rectangle': [13, 49, 1, 2], 'map': 0x100,
                                     'mode': 0, 'selector': 0x66, 'raw_position': [248, 992]}})

    def test_exit_controls_missing_duplicate_truncated_and_unbounded(self):
        record = '00 00 01 01 00 01 00 66 f8 00 e0 03 '
        for data in ['ff', record[:-4], record * 2 + 'ff', record * 16]:
            with self.subTest(data_length=len(data)), self.assertRaises(ValueError):
                t.route_exit(source({0x818006: '00 90', 0x819000: data}), 3, 0x100)
        with self.assertRaises(ValueError):
            t.route_exit(source({0x818006: 'ff 7f'}), 3, 0x100)

    def test_scene_bank82_override_and_world_bank83(self):
        s = source({0x828006: '00 00', 0x838006: 'f9 88',
                    0x828200: 'c1 88', 0x838200: '00 00'})
        self.assertEqual(t.scene(s, 3)['selected'], 0x8388f9)
        self.assertEqual(t.scene(s, 0x100)['selected'], 0x8288c1)
        changed = source({0x828006: '00 90', 0x838006: 'f9 88'})
        self.assertEqual(t.scene(changed, 3)['selected'], 0x829000)


class ScriptTests(unittest.TestCase):
    def test_elder_21_is_encounter_not_acceptance_and_retry_source_only(self):
        d = t.decode_elder(source(ELDER))
        self.assertEqual([f['event'] for f in d['effects']], [0x14, 0x21, 0x296])
        self.assertEqual(d['choice']['cancel_then_options'], [0x888c41, 0x888c2d, 0x888c41])
        self.assertEqual([r['request'] for r in d['requests']], [0x888c51, 0x888d1f, 0x888dd4])
        self.assertEqual([j['target'] for j in d['departures']], [0x888c0b, 0x888c0b])
        self.assertEqual([b['target'] for b in d['retry']['branches']][-2:], [0x888f6d, 0x888f49])
        self.assertEqual(d['retry']['choice']['cancel_then_options'], [0x888f66, 0x888f5b, 0x888f66])
        self.assertEqual(d['retry']['effect']['event'], 0x296)
        self.assertEqual(d['retry']['qualification'], 'source_only_not_native_route')

    def test_town_xor_sequence_not_geographic_prerequisite(self):
        d = t.decode_town(source(TOWN))
        self.assertEqual(d['gate']['events'], [0x296, 0x3c])
        self.assertEqual([f['event'] for f in d['effects']], [0x14, 0x3c])
        self.assertEqual([v['ticks'] for v in d['delays']], [120, 100, 32])
        self.assertEqual(d['waits'][-1], {'source': 0x88852e, 'selector': 0x20})
        self.assertEqual(d['continuation']['target'], 0x8884f7)
        self.assertFalse(d['proves_296_geographic_prerequisite'])

    def test_guardian_196_not_296_and_both_choices_join_before_115(self):
        d = t.decode_guardian(source(GUARDIAN))
        self.assertEqual([b['event'] for b in d['branches']], [0x196, 0x115])
        self.assertEqual(d['choice']['cancel_then_options'], [0x908c5d, 0x908c4b, 0x908c5d])
        self.assertEqual(d['option1_join'], {'source': 0x908c5b, 'target': 0x908c6d})
        self.assertEqual(d['cancel_option2_fallthrough'], 0x908c6d)
        self.assertEqual(d['effects'][-1], {'source': 0x908c75, 'event': 0x115, 'set': True})
        self.assertEqual(d['handoff'], {'source': 0x908c79, 'mode': 1, 'target': 0x8487c1})
        self.assertEqual(d['helper']['input'], {'source': 0x908cee, 'address': 0xda4, 'mask': 0x30})
        self.assertEqual(d['intro']['effect']['event'], 0x100)
        self.assertEqual(d['intro']['request']['request'], 0x908f4d)

    def test_guardian_mutations_are_decoded_or_rejected(self):
        parts = dict(GUARDIAN)
        parts[0x908bfb] = '02 08 96 82 1f 90'
        self.assertEqual(t.decode_guardian(source(parts))['branches'][0]['event'], 0x296)
        parts[0x908c79] = '02 cb 02 c2 87 84'
        self.assertEqual(t.decode_guardian(source(parts))['handoff']['target'], 0x8487c2)
        parts[0x908c5b] = 'ea 10'
        with self.assertRaises(ValueError):
            t.decode_guardian(source(parts))

    def test_world_startup_has_distinct_helper_and_f0_f2_handshake(self):
        d = t.decode_world(source(WORLD))
        self.assertEqual(d['spawn']['target'], 0x84e02a)
        self.assertEqual(d['effect'], {'source': 0x84defe, 'event': 0xf0, 'set': False})
        self.assertEqual(d['waits'][0]['event'], 0xf2)
        self.assertFalse(d['waits'][0]['until_set'])
        self.assertTrue(d['waits'][1]['until_set'])
        self.assertEqual(d['movement_model'], 'not_modeled_distinct_world_mode')


class EquipmentTests(unittest.TestCase):
    def test_equipped_words_not_inventory_or_0648(self):
        d = t.decode_equipment(source(EQUIPMENT))
        self.assertEqual(d['weapon']['address'], 0x7e064a)
        self.assertEqual(d['armor']['address'], 0x7e064c)
        self.assertEqual(d['weapon']['width'], 2)
        self.assertEqual(d['weapon']['none'], 0)
        self.assertEqual(d['armor']['equip_source'], 0x85b036)
        self.assertEqual(d['category_split'], [0x80, 0xa0])
        self.assertFalse(d['item81_insertion_proves_equipped'])
        self.assertEqual(d['combat_state'], 'hp_xp_not_qualified')

    def test_equipment_mutations_fail_consistency_or_change_projection(self):
        parts = dict(EQUIPMENT)
        parts[0x85b015] = 'ad d0 0d 8d 48 06'
        with self.assertRaises(ValueError):
            t.decode_equipment(source(parts))
        parts = dict(EQUIPMENT)
        parts[0x85b006] = 'a9 01 00 8d 4a 06'
        with self.assertRaises(ValueError):
            t.decode_equipment(source(parts))


class AuthenticationTests(unittest.TestCase):
    def test_authentication_precedes_projection(self):
        with patch.object(t, 'decode_elder', side_effect=AssertionError('decode before auth')):
            for b in [b'', b'not ROM', source(ELDER).data]:
                with self.assertRaisesRegex(ValueError, 'Japanese ROM authentication'):
                    t.project(b)

    def test_pinned_projection_matches_independent_synthetic_operands(self):
        pinned = json.loads((Path(t.__file__).parent / 'tower-source.json').read_text())
        for name, decoder, parts in [('elder', t.decode_elder, ELDER), ('town', t.decode_town, TOWN),
                                     ('guardian', t.decode_guardian, GUARDIAN), ('world', t.decode_world, WORLD),
                                     ('equipment', t.decode_equipment, EQUIPMENT)]:
            self.assertEqual(decoder(source(parts)), pinned[name], name)
        self.assertEqual([(r['from_map'], r['exit']['map']) for r in pinned['route']],
                         [(0x21, 0x20), (0x20, 0xe), (0xe, 0xc), (0xc, 0xd), (0xd, 0xa),
                          (0xa, 3), (3, 0x100), (0x100, 0x101)])
        self.assertEqual(pinned['route'][-1]['exit']['raw_position'], [120, 608])


if __name__ == '__main__':
    unittest.main()
