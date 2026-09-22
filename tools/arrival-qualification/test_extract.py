"""ROM-free tests for completed-frame arrival evidence extraction."""
import unittest

from extract import extract_profile, compare_hostile, decode_record, EvidenceError


def row(frame, xy=(10, 20), special=0, script=0x84A12E):
    player = [0] * 32
    player[2] = 0x4414
    player[5], player[6] = script & 0xFFFF, script >> 16
    return dict(kind='arrival', frame=frame, position=list(xy), map=23,
                control=0, special=special, facing=0, player_words=player,
                control_words=[0] * 32, control_slot=4416, actors=[],
                held=[], service_words=[0] * 5, input_mask=0)


class ProfileTests(unittest.TestCase):
    def fixture(self):
        return [row(100), row(101, special=0x8000, script=0x84BD7E),
                row(102, (11, 21), 0x8000, 0x84BD7E),
                row(103, (11, 21), script=0x84A303),
                row(104, (11, 21), script=0x84A258)]

    def test_sparse_positions_and_inclusive_cursor_zero(self):
        p = extract_profile(self.fixture(), 100, 104)
        self.assertEqual(p['positions'], [[0, 10, 20], [2, 11, 21]])
        self.assertEqual((p['advances'], p['samples']), (4, 5))
        self.assertEqual(p['phases'], dict(initialized=0, ownership=1,
                         forced=1, endpoint=2, recovery=3, free=4))
        self.assertNotIn('player_words', str(p))
        self.assertNotIn('actors', str(p))

    def test_missing_frame_rejected(self):
        with self.assertRaises(EvidenceError):
            extract_profile(self.fixture()[1:], 100, 104)

    def test_duplicate_frame_rejected(self):
        with self.assertRaises(EvidenceError):
            extract_profile(self.fixture() + [row(102)], 100, 104)

    def test_map_change_rejected(self):
        rows = self.fixture(); rows[2]['map'] = 25
        with self.assertRaises(EvidenceError):
            extract_profile(rows, 100, 104)

    def test_nonfree_endpoint_rejected(self):
        rows = self.fixture(); rows[-1]['player_words'][5] = 0xBD7E
        with self.assertRaises(EvidenceError):
            extract_profile(rows, 100, 104)

    def test_late_free_boundary_rejected(self):
        rows = self.fixture() + [row(105, (11, 21), script=0x84A258)]
        with self.assertRaises(EvidenceError):
            extract_profile(rows, 100, 105)

    def test_initialized_boundary_rejected_if_already_free(self):
        rows = self.fixture(); rows[0]['player_words'][5] = 0xA258
        with self.assertRaises(EvidenceError):
            extract_profile(rows, 100, 104)

    def test_hostile_selected_and_ancillary_differences_separated(self):
        baseline = self.fixture(); hostile = self.fixture()
        hostile[2]['held'] = ['Left', 'A']
        hostile[2]['service_words'][0] = 0x280
        hostile[2]['player_words'][20] = 0xFFFF
        result = compare_hostile(baseline, hostile, 100, 104)
        self.assertEqual(result['selected_differences'], [])
        self.assertEqual(result['ancillary_differences'][0]['frame'], 102)
        hostile[2]['position'][0] += 1
        self.assertEqual(compare_hostile(baseline, hostile, 100, 104)
                         ['selected_differences'][0]['fields'], ['position'])

    def test_source_record_full_operands(self):
        record = bytes([58, 20, 1, 1, 23, 0, 0, 14, 192, 1, 96, 1])
        self.assertEqual(decode_record(record, 0x18F42), dict(
            rom_offset=0x18F42, cpu_address=0x818F42,
            rectangle=[58, 20, 1, 1], destination=23, mode=0,
            selector=14, raw_position=[448, 352]))

    def test_short_source_record_rejected(self):
        with self.assertRaises(EvidenceError):
            decode_record(b'\x00', 0x18F42)


if __name__ == '__main__':
    unittest.main()
