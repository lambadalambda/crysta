"""ROM-free tests of the bounded source-derived startup operations."""
import unittest
from startup import assign_event, spawn, transition


class StartupTest(unittest.TestCase):
    def test_transition_fields_and_tag(self):
        record = bytes([2, 20, 15, 0, 1, 0, 40, 1, 96, 0])
        self.assertEqual(transition(record), (15, 1, 0, 296, 96))
        for bad in (record[:-1], b'\x00' + record[1:]):
            with self.assertRaises(ValueError):
                transition(bad)

    def test_spawn_queue_overrides_default_and_is_consumed(self):
        self.assertEqual(spawn((19, 7), (296, 96)), ((304, 112), (0, 0)))
        self.assertEqual(spawn((19, 7), (0, 0)), ((312, 112), (0, 0)))
        self.assertEqual(spawn((1, 2), (0, 96)), ((8, 112), (0, 0)))
        self.assertEqual(spawn((1, 2), (65535, 65535)), ((7, 15), (0, 0)))

    def test_event_set_clear_preserve_and_bounds(self):
        initial = bytes(64)
        default = assign_event(initial, 0x80FB)
        released = assign_event(default, 0x8020)
        self.assertEqual(initial, bytes(64))
        self.assertEqual([(i, x) for i, x in enumerate(released) if x], [(4, 1), (31, 8)])
        self.assertEqual(assign_event(released, 0x0020), default)
        self.assertEqual(assign_event(default, 0x80FB), default)
        with self.assertRaises(ValueError):
            assign_event(initial, 0x8200)


if __name__ == '__main__':
    unittest.main()
