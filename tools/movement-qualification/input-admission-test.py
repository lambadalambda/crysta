"""ROM-free tests of the bounded input gate, not a dash movement implementation."""
import unittest
from input_admission import Admission, Dash, Unsupported


class AdmissionTests(unittest.TestCase):
    def feed(self, inputs):
        state = Admission()
        for direction in inputs:
            state = state.submit(direction)
        return state

    def test_quick_retap_is_not_walking(self):
        with self.assertRaises(Dash):
            self.feed(['Left', '', '', '', 'Left'])

    def test_onset_boundary_not_release_cooldown(self):
        for gap in range(2, 11):
            with self.assertRaises(Dash):
                self.feed(['Left'] + [''] * (gap - 1) + ['Left'])
        self.feed(['Left'] + [''] * 10 + ['Left'])
        self.feed(['Left'] * 20 + [''] + ['Left'])

    def test_only_most_recent_direction_matters(self):
        self.feed(['Left', 'Right', 'Left', 'Right'] * 20)
        with self.assertRaises(Dash):
            self.feed(['Left', 'Right', '', 'Right'])

    def test_neutral_does_not_forget(self):
        state = self.feed(['Up'] + [''] * 30)
        self.assertEqual((state.last, state.remaining), ('Up', 0))
        state.submit('Up')

    def test_cardinals_and_actions(self):
        for direction in ('Left', 'Right', 'Up', 'Down'):
            self.feed([direction] * 60 + [''] + [direction])
        for direction in ('Left+Up', 'A', 'Start', 'bogus'):
            with self.assertRaises(Unsupported):
                self.feed([direction])

    def test_rejection_is_transactional(self):
        state = self.feed(['Left', ''])
        before = state
        with self.assertRaises(Dash):
            state.submit('Left')
        self.assertEqual(state, before)


if __name__ == '__main__':
    unittest.main()
