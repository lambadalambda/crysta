#!/usr/bin/env python3
"""ROM-free controls for reproducible contiguous slope motion routes."""
import unittest

from slope_route import route


class MotionRoutes(unittest.TestCase):
    def test_motion_window_has_settled_prefix_and_no_gaps(self):
        for direction in ('Right', 'Left'):
            for vertical in (False, True):
                commands = route(direction, motion=True, vertical=vertical)
                start = next(i for i, c in enumerate(commands) if c.get('motion'))
                self.assertEqual(commands[start - 1], {
                    'label': 'slope-settle', 'frames': 12, 'buttons': []})
                window = commands[start:-1]
                self.assertTrue(all(c.get('motion') for c in window))
                self.assertTrue(all(c['frames'] > 0 for c in window))
                self.assertEqual(sum(c['frames'] for c in window), 236 if vertical else 40)
                self.assertEqual(window[0]['buttons'], [direction])
                self.assertEqual(commands[-1], {'finish': True})
                if vertical:
                    self.assertEqual({b for c in window for b in c['buttons']},
                                     {direction, 'Up', 'Down'})
                    self.assertTrue(all(c['frames'] == 12 for c in window if not c['buttons']))

    def test_incompatible_modes_are_rejected(self):
        for options in ({'trace': True, 'motion': True}, {'vertical': True}):
            with self.assertRaises(ValueError):
                route('Right', **options)

    def test_tree_routes_cover_the_other_faces_without_resets(self):
        from slope_route import tree_route
        for side, count in [('east', 714), ('bottom', 706)]:
            commands = tree_route(side)
            start = next(i for i, c in enumerate(commands) if c.get('motion'))
            self.assertEqual(commands[start - 1]['label'], 'slope-settle')
            window = commands[start:-1]
            self.assertTrue(all(c.get('motion') for c in window))
            self.assertEqual(sum(c['frames'] for c in window), count)
            self.assertEqual(commands[-1], {'finish': True})
            for before, after in zip(window[1::2], window[2::2]):
                self.assertEqual(before['buttons'], [])
                self.assertEqual(before['frames'], 12)
                self.assertEqual(len(after['buttons']), 1)
        with self.assertRaises(ValueError):
            tree_route('north')

    def test_historical_discovery_routes_are_unchanged(self):
        self.assertEqual(route('Left')[-2], {
            'label': 'slope-left', 'frames': 40, 'buttons': ['Left']})
        self.assertEqual(route('Right', trace=True)[-3], {
            'trace': 'slope-right', 'buttons': ['Right']})


if __name__ == '__main__':
    unittest.main()
