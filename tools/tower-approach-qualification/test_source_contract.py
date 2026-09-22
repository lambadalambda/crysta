"""ROM-free, independently authored instruction fragments; no text/ROM fixtures."""
import hashlib
import importlib.util
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

import source_contract as contract
from source_contract import DepartureSource, decode_departure, decode_return, decode_inventory


def source(parts):
    """Sparse synthetic HiROM image, populated only with selected instructions."""
    size = max((a & 0x3fffff) + len(bytes.fromhex(h)) for a, h in parts.items())
    data = bytearray(size)
    for a, h in parts.items():
        b = bytes.fromhex(h)
        at = a & 0x3fffff
        data[at:at + len(b)] = b
    return DepartureSource(bytes(data))


DEPARTURE = {
    0x89dcb5: '02 14 42 00 00 02 80 00 c0 01',
    0x89d9fe: '02 b2 f8 ff', 0x89da07: '02 48 42 82',
    0x89da17: '02 3b', 0x89da19: '02 21 56 da',
    0x89da56: '02 08 41 82 96 da', 0x89da5c: '02 08 40 82 72 da',
    0x89da62: '02 07 40 82', 0x89da66: '02 1b b4 da 02 1f',
    0x89da6c: '02 1a 01 7e da', 0x89da72: '02 1b b3 db 02 1f',
    0x89da78: '02 1a 01 7e da', 0x89da7e: '84 da 8b da 84 da',
    0x89da84: '02 1b bb db 02 1f', 0x89da8b: '02 1b ea db 02 1f',
    0x89da91: '02 07 41 82', 0x89da96: 'a9 00 01 0c 8a 04',
    0x89da9c: '02 07 42 82', 0x89daa0: '02 21 00 00',
    0x89daa4: '02 2a 50 ff', 0x89daa8: '02 1b 0e dc 02 1f',
    0x89daae: '02 c0 20 da 89', 0x89da20: '02 60 81 a4 01 34',
    0x89da26: '02 c1 e0 01', 0x89da2a: '02 1c 88 65 d1 02 1f',
    0x89da31: 'a9 00 01 1c 8a 04', 0x89da37: '02 07 01 80',
    0x89da3b: '02 c1 3c 00', 0x89da3f: '02 1b 29 dc 02 1f',
    0x89da45: '02 c1 3c 00', 0x89da49: '02 14 21 00 04 01 80 00 60 01',
}
RETURN = {
    0x88aebd: '02 48 23 80', 0x88aec1: '02 48 22 00', 0x88aec5: '02 2a 50 ff',
    0x88aec9: '02 08 44 82 e9 ae', 0x88aee9: '02 c1 20 00',
    0x88aeed: '02 07 03 80', 0x88aef1: '02 08 02 80 fe ae',
    0x88af03: '02 00 07 b5 88', 0x88af08: '02 08 0a 80 15 af',
    0x88af30: '02 df 72 af 88', 0x88af35: '02 c1 3c 00',
    0x88af39: '02 1b ce b2 02 20', 0x88af3f: '02 07 fe 80',
    0x88af43: '02 07 23 80', 0x88af47: '02 29 50 ff 02 3c 02 a7',
    0x88b2ff: '02 48 01 81', 0x88b303: '02 08 23 00 18 b3',
    0x88b321: '02 05 03 00', 0x88b32e: '02 3a 03 68 14',
    0x88b33e: '02 1b e0 b3 02 1f', 0x88b346: '02 1b 20 b4 02 1f',
    0x88b34c: '02 3a 03 68 17', 0x88b35c: '02 df 50 af 88',
    0x88b361: '02 1b 45 b4 02 20', 0x88b36c: '02 1b 80 b4 02 1f',
    0x88b372: '02 00 b0 b3 88', 0x88b377: '02 07 02 80',
    0x88b392: '02 1b 9e b1 02 1f', 0x88b398: '02 df 62 af 88',
    0x88b39d: '02 1b bc b1 02 20', 0x88b3a3: '02 07 0a 80',
    0x88b3a7: '02 05 23 00',
}
# Width and branch framing deliberately explicit: the resolver enters M8, not M16.
INVENTORY = {
    0x809a0c: '8d c7 09', 0x809a0f: '22 53 96 8d da',
    0x809a5b: '4c d6 90',
    0x8d9659: 'ad c7 09 e2 20 48 20 32 97',
    0x8d9662: 'bf 00 80 7f c3 01 f0 1c c9 00 d0 08 a5 00 d0 04 86 00 e6 00',
    0x8d9676: 'e8 e8 e4 04 90 e6 a6 00 d0 05 68 fa 28 38 6b ca',
    0x8d9686: 'bf 01 80 7f c9 09 b0 f2 68 9f 00 80 7f bf 01 80 7f 1a 9f 01 80 7f fa 28 18 6b',
    0x8d9732: 'c9 10 b0 10', 0x8d9746: 'c9 80 b0 15',
    0x8d975f: 'c9 a0 b0 15 c9 9c 90 0a',
    0x8d9771: 'a2 48 00 a9 60 80 13', 0x8d978b: '85 04 64 05 60',
}


class OperandTests(unittest.TestCase):
    def test_bank_first_request_not_little_endian_long(self):
        s = source({0x89da2a: '02 1c 88 65 d1'})
        self.assertEqual(s.request_long(0x89da2a), {'source': 0x89da2a, 'request': 0x88d165})

    def test_item_presentation_and_worker_not_quantity(self):
        s = source({0x89da20: '02 60 81 a4 01 34'})
        self.assertEqual(s.award(0x89da20), {'source': 0x89da20, 'item': 0x81,
                                         'presentation': 0x1a4, 'worker': 0x34})
        self.assertNotIn('quantity', s.award(0x89da20))

    def test_operand_mutations_are_not_hardcoded(self):
        s = source({0x89da20: '02 60 82 a5 02 35', 0x89da2a: '02 1c 87 66 d2'})
        self.assertEqual(s.award(0x89da20)['item'], 0x82)
        self.assertEqual(s.award(0x89da20)['presentation'], 0x2a5)
        self.assertEqual(s.award(0x89da20)['worker'], 0x35)
        self.assertEqual(s.request_long(0x89da2a)['request'], 0x87d266)

    def test_branch_membership_and_wait_polarities(self):
        s = source({0x808000: '02 08 41 82 96 da 02 48 42 82 02 05 03 00'})
        self.assertEqual(s.branch(0x808000), {'source': 0x808000, 'event': 0x241,
                                            'when_set': True, 'target': 0x80da96})
        self.assertEqual(s.membership(0x808006)['require_set'], False)
        self.assertEqual(s.wait_flag(0x80800a), {'source': 0x80800a, 'event': 3, 'until_set': True})

    def test_shape_and_truncation_controls(self):
        for method, fragments in [('award', ['03 60 81 a4 01 34', '02 61 81 a4 01 34', '02 60 81 a4 01']),
                                  ('request_long', ['02 1b 88 65 d1', '02 1c 88 65']),
                                  ('branch', ['02 08 41 82 96']), ('membership', ['02 48 42']),
                                  ('wait_flag', ['02 05 03'])]:
            for h in fragments:
                with self.subTest(method=method, fragment=h), self.assertRaises(ValueError):
                    getattr(source({0x808000: h}), method)(0x808000)

    def test_import_does_not_replace_parent_source(self):
        spec = importlib.util.spec_from_file_location('parent_source_control',
                    Path(__file__).resolve().parents[1] / 'pandora-qualification/source.py')
        parent = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(parent)
        with patch.dict(sys.modules, {'source': parent}):
            spec = importlib.util.spec_from_file_location('departure_import_control', contract.__file__)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            self.assertIs(sys.modules['source'], parent)
            self.assertEqual(module.DepartureSource.__mro__[1].__name__, 'Source')


class DepartureTests(unittest.TestCase):
    def test_door_choice_acceptance_and_second_interaction(self):
        d = decode_departure(source(DEPARTURE))
        self.assertEqual(d['transitions'], [
            {'source': 0x89dcb5, 'map': 0x42, 'mode': 0, 'selector': 2, 'raw_position': [128, 448]},
            {'source': 0x89da49, 'map': 0x21, 'mode': 4, 'selector': 1, 'raw_position': [128, 352]}])
        self.assertEqual(d['spark']['offset'], -8)
        self.assertEqual(d['callback'], {'source': 0x89da19, 'target': 0x89da56})
        self.assertEqual(d['clear_callback']['target'], 0)
        self.assertEqual([b['event'] for b in d['branches']], [0x241, 0x240])
        self.assertEqual([b['target'] for b in d['branches']], [0x89da96, 0x89da72])
        for choice in d['choices']:
            self.assertEqual(choice['catalog'], 1)
            self.assertEqual(choice['cancel_then_options'], [0x89da84, 0x89da8b, 0x89da84])
        self.assertEqual([f['event'] for f in d['effects']], [0x240, 0x241, 0x242, 1])
        self.assertEqual(d['collection_jump']['target'], 0x89da20)
        self.assertEqual(d['masks'], [
            {'source': 0x89da96, 'address': 0x48a, 'mask': 0x100, 'set': True},
            {'source': 0x89da31, 'address': 0x48a, 'mask': 0x100, 'set': False}])
        self.assertEqual([x['ticks'] for x in d['delays']], [480, 60, 60])
        self.assertEqual([r['request'] for r in d['requests']],
                         [0x89dab4, 0x89dbb3, 0x89dbbb, 0x89dbea, 0x89dc0e, 0x88d165, 0x89dc29])

    def test_choice_and_destination_mutations_change_projection(self):
        parts = dict(DEPARTURE)
        parts[0x89da7e] = '8b da 84 da 8b da'
        parts[0x89da49] = '02 14 21 00 00 02 80 00 60 01'
        d = decode_departure(source(parts))
        self.assertEqual(d['choices'][0]['cancel_then_options'], [0x89da8b, 0x89da84, 0x89da8b])
        self.assertEqual(d['transitions'][1]['mode'], 0)
        self.assertEqual(d['transitions'][1]['selector'], 2)

    def test_wrong_wait_selector_rejected(self):
        parts = dict(DEPARTURE)
        parts[0x89da84] = '02 1b bb db 02 20'
        with self.assertRaises(ValueError):
            decode_departure(source(parts))


class ReturnTests(unittest.TestCase):
    def test_frozen_controller_and_worker_handshake(self):
        d = decode_return(source(RETURN))
        self.assertEqual([(x['event'], x['require_set']) for x in d['membership']],
                         [(0x23, False), (0x22, True), (0x101, False)])
        self.assertEqual([(x['event'], x['when_set'], x['target']) for x in d['branches']],
                         [(0x244, True, 0x88aee9), (2, True, 0x88aefe),
                          (10, True, 0x88af15), (0x23, False, 0x88b318)])
        self.assertEqual([f['event'] for f in d['effects']], [3, 0xfe, 0x23, 2, 10])
        self.assertEqual([w['event'] for w in d['flag_waits']], [3, 0x23])
        self.assertTrue(all(w['until_set'] for w in d['flag_waits']))
        self.assertEqual([w['source'] for w in d['dialogue_waits'] if w['selector'] == 0x20],
                         [0x88af3d, 0x88b365, 0x88b3a1])
        self.assertEqual([x['target'] for x in d['calls']], [0x88b507, 0x88b3b0])
        self.assertEqual([x['target'] for x in d['presentation_calls']], [0x88af72, 0x88af50, 0x88af62])
        self.assertEqual([r['request'] for r in d['requests']],
                         [0x88b2ce, 0x88b3e0, 0x88b420, 0x88b445, 0x88b480, 0x88b19e, 0x88b1bc])

    def test_movement_operand_width_and_signed_tile(self):
        parts = dict(RETURN)
        parts[0x88b32e] = '02 3a 83 69 fe'
        move = decode_return(source(parts))['movement_waits'][0]
        self.assertEqual(move['target_tile_y'], -2)
        self.assertEqual(move['animation_control'], 0x83)
        self.assertEqual(move['motion_stream_index'], 0x69)

    def test_flag_polarity_mutation_is_visible(self):
        parts = dict(RETURN)
        parts[0x88b303] = '02 08 23 80 18 b3'
        self.assertTrue(decode_return(source(parts))['branches'][-1]['when_set'])


class InventoryTests(unittest.TestCase):
    def test_m8_region_stride_cap_and_actual_write(self):
        d = decode_inventory(source(INVENTORY))
        self.assertEqual(d['item_latch'], 0x9c7)
        self.assertEqual(d['call'], {'source': 0x809a0f, 'target': 0x8d9653})
        self.assertEqual(d['resolver_call'], {'source': 0x8d965f, 'target': 0x8d9732})
        self.assertEqual(d['region'], {'item': 0x81, 'accumulator_bits': 8, 'start': 0x48,
                                     'end_exclusive': 0x60, 'stride': 2, 'base': 0x7f8000})
        self.assertEqual(d['quantity_cap'], 9)
        self.assertEqual(d['writes'], [{'source': 0x8d968f, 'base': 0x7f8000, 'value': 'item'},
                                      {'source': 0x8d9698, 'base': 0x7f8001, 'value': 'quantity+1'}])
        self.assertEqual(d['first_empty_weapon_slot'], {'address': 0x7f8048, 'item': 0x81,
                                                      'quantity_address': 0x7f8049, 'quantity': 1})
        self.assertFalse(d['cop60_checks_failure_carry'])
        self.assertTrue(d['requires_runtime_inventory_proof'])

    def test_quantity_and_slot_operand_mutations_visible(self):
        parts = dict(INVENTORY)
        parts[0x8d9771] = 'a2 4a 00 a9 60 80 13'
        parts[0x8d9686] = INVENTORY[0x8d9686].replace('c9 09', 'c9 08')
        d = decode_inventory(source(parts))
        self.assertEqual(d['region']['start'], 0x4a)
        self.assertEqual(d['quantity_cap'], 8)
        self.assertEqual(d['first_empty_weapon_slot']['address'], 0x7f804a)

    def test_width_stride_call_and_carry_mutations_rejected(self):
        for address, old, new in [(0x8d9659, 'e2 20', 'c2 20'), (0x8d9676, 'e8 e8', 'e8 ea'),
                                  (0x8d9659, '20 32 97', '20 33 97'),
                                  (0x809a0f, 'da', 'b0'), (0x8d9732, 'b0 10', 'b0 11')]:
            parts = dict(INVENTORY)
            parts[address] = parts[address].replace(old, new)
            with self.subTest(address=hex(address)), self.assertRaises(ValueError):
                decode_inventory(source(parts))


class AuthenticationTests(unittest.TestCase):
    def test_project_authenticates_before_any_decode(self):
        with patch.object(contract, 'decode_departure', side_effect=AssertionError('decoded first')):
            for data in [b'', b'not a ROM', source(DEPARTURE).data]:
                with self.assertRaisesRegex(ValueError, 'Japanese ROM authentication'):
                    contract.project(data)

    def test_window_hashes_are_half_open_and_mutation_sensitive(self):
        s = source({0x808000: '01 02 03 04'})
        self.assertEqual(contract.window_hashes(s, [(0x808000, 0x808003)]),
                         [{'start': 0x808000, 'end': 0x808003,
                           'sha256': hashlib.sha256(bytes([1, 2, 3])).hexdigest()}])
        mutated = source({0x808000: '01 02 04 04'})
        self.assertNotEqual(contract.window_hashes(s, [(0x808000, 0x808003)]),
                            contract.window_hashes(mutated, [(0x808000, 0x808003)]))
        with self.assertRaises(ValueError):
            contract.window_hashes(s, [(0x808000, 0x808005)])


class DependencyTests(unittest.TestCase):
    def handlers(self):
        return {
            0x808ca8: '22 0e 91 85', 0x808cad: '90 0d', 0x808cb4: '3a 3a',
            0x808cbc: '9c c2 0d', 0x808c3e: 'a7 36 e6 36 29 ff 00 8d c2 0d 80 bc',
            0x808c0f: 'a7 36 e6 36 e6 36 8d c0 0d',
            0x808631: '30 07 20 a6 bb 90 07', 0x80863a: '20 a6 bb 90 0a',
            0x808641: '3a 3a', 0x8092b5: 'f0 cd',
            0x8092af: '20 2f bc', 0x8092e9: '20 db bb',
        }

    def presentation(self):
        parts = {0x88af03: '02 00 07 b5 88'}
        sites = [0x88b3b4, 0x88b3bb, 0x88b3c2, 0x88b3c9, 0x88b3d0, 0x88b3d7]
        for i, a in enumerate(sites):
            target = 0x88b573 + 26 * i
            parts[a] = '02 a2 ' + target.to_bytes(3, 'little').hex() + ' 10 10'
            parts[target] = '20 0f b6'
            parts[target + 23] = '02 a7 6b'
        return parts

    def scenes(self):
        parts = {0x838082: '27 95', 0x838084: '69 95', 0x838042: '68 92'}
        for a, header in [(0x83953a, 0x89dc9d), (0x83957c, 0x89d9f9), (0x839586, 0x89da12),
                          (0x839271, 0x88b2fa), (0x83927b, 0x88aeb8)]:
            parts[a + 4] = header.to_bytes(3, 'little').hex()
        return parts

    def test_native_wait_handler_evidence_and_mutations(self):
        d = contract.decode_handlers(source(self.handlers()))
        self.assertEqual(d['cop20']['helper'], 0x85910e)
        self.assertTrue(d['cop20']['retry_on_carry_set'])
        self.assertEqual(d['cop20']['clears'], 0xdc2)
        self.assertEqual(d['cop1c']['operand_order'], 'bank_then_word')
        self.assertEqual(d['cop1c']['word_latch'], 0xdc0)
        self.assertTrue(d['cop05']['positive_waits_until_set'])
        self.assertEqual(d['cop3a']['coordinate_helper'], 0x80bc2f)
        for a, h in [(0x808cad, 'b0 0d'), (0x808636, 'b0 07'), (0x808cbc, '9c c0 0d'),
                     (0x808c40, 'ea ea')]:
            parts = self.handlers()
            parts[a] = h
            with self.subTest(address=hex(a)), self.assertRaises(ValueError):
                contract.decode_handlers(source(parts))

    def test_scene_records_and_override_rejection(self):
        d = contract.decode_scenes(source(self.scenes()))
        self.assertEqual([r['entry'] for r in d['records']],
                         [0x89dca2, 0x89d9fe, 0x89da17, 0x88b2ff, 0x88aebd])
        for a, h in [(0x828082, '01 00'), (0x83927f, 'b9 ae 88')]:
            parts = self.scenes()
            parts[a] = h
            with self.subTest(address=hex(a)), self.assertRaises(ValueError):
                contract.decode_scenes(source(parts))

    def test_six_presentation_workers_and_fade_pointer(self):
        d = contract.decode_presentation(source(self.presentation()))
        self.assertEqual([w['target'] for w in d['spawned_workers']],
                         [0x88b573, 0x88b58d, 0x88b5a7, 0x88b5c1, 0x88b5db, 0x88b5f5])
        parts = self.presentation()
        parts[0x88af03] = '02 00 08 b5 88'
        self.assertEqual(contract.decode_presentation(source(parts))['fade_call']['target'], 0x88b508)
        parts[0x88b58a] = '02 07 03 80'
        with self.assertRaises(ValueError):
            contract.decode_presentation(source(parts))

    def test_complete_synthetic_projection_and_pinned_typed_operands(self):
        import json
        parts = {**DEPARTURE, **RETURN, **INVENTORY, **self.handlers(),
                 **self.scenes(), **self.presentation(), 0x8d978f: '60'}
        data = source(parts).data
        # Inject a trusted fixture digest, never disable authentication. Production
        # project has no bypass and always uses the real JP digest.
        with patch.object(contract, 'ROM_SHA', contract.sha(data)):
            result = contract.project(data)
            mutated = bytearray(data)
            mutated[0x9da22] ^= 1
            with self.assertRaisesRegex(ValueError, 'Japanese ROM authentication'):
                contract.project(mutated)
        pinned = json.loads((Path(contract.__file__).parent / 'source.json').read_text())
        for section in ['scenes', 'departure', 'frozen_return', 'inventory', 'handlers', 'presentation']:
            self.assertEqual(result[section], pinned[section], section)
        self.assertEqual([(r['start'], r['end']) for r in pinned['ranges']], contract.RANGES)
        self.assertTrue(all(len(r['sha256']) == 64 for r in pinned['ranges']))


if __name__ == '__main__':
    unittest.main()
