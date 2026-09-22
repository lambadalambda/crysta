#!/usr/bin/env python3
"""ROM-free controls for bounded post-progression motion windows."""
import unittest


class MotionWindows(unittest.TestCase):
    def test_windows_start_settled_and_do_not_skip_frames(self):
        from window_route import route
        for name, label, count in [('Door5', 'door-real-hit1-closed', 207),
                                   ('Map41', 'pandora-neutral-stable', 880)]:
            commands = route(name)
            start = next(i for i, c in enumerate(commands) if c.get('motion'))
            self.assertEqual(commands[start - 1]['label'], label)
            self.assertEqual(commands[start - 1]['buttons'], [])
            self.assertGreaterEqual(commands[start - 1]['frames'], 12)
            self.assertTrue(all(c.get('motion') for c in commands[start:-1]))
            self.assertEqual(sum(c['frames'] for c in commands[start:-1]), count)
            self.assertTrue(all(set(c['buttons']) <= {'Up', 'Down', 'Left', 'Right'}
                                for c in commands[start:-1]))
            self.assertEqual(commands[-1], {'finish': True})

    def test_unknown_window_is_rejected(self):
        from window_route import route
        with self.assertRaises(ValueError):
            route('arbitrary')


if __name__ == '__main__':
    unittest.main()
