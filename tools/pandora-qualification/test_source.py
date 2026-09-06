"""ROM-free source-reader framing controls; no copyrighted source fixtures."""
import unittest
from source import Source


class SourceTests(unittest.TestCase):
    def source(self, values):
        return Source(bytes(0x8000) + bytes(values))

    def test_request_banked_word(self):
        self.assertEqual(self.source([2, 0x1b, 0x34, 0x92]).request(0x808000),
                         {'source': 0x808000, 'request': 0x809234})

    def test_flag_set_and_clear(self):
        for high, setting in [(0x82, True), (2, False)]:
            s = self.source([2, 7, 0x92, high])
            self.assertEqual(s.flag(0x808000), {'source': 0x808000, 'event': 0x292, 'set': setting})

    def test_palette_bank_first_not_u24(self):
        self.assertEqual(self.source([2, 0x5a, 0xaf, 0x5b, 0xe4, 24, 8]).palette(0x808000),
                         {'source': 0x808000, 'pointer': 0xafe45b, 'destination': 24, 'count': 8})

    def test_transition_framing(self):
        self.assertEqual(self.source([2, 0x14, 0x41, 0, 4, 2, 128, 0, 192, 0]).transition(0x808000),
                         {'source': 0x808000, 'map': 65, 'mode': 4, 'selector': 2,
                          'raw_position': [128, 192]})

    def test_rejects_wrong_selector_and_truncation(self):
        for values in [[], [2], [2, 0x1b, 0], [2, 0x1a, 0, 0], [3, 0x1b, 0, 0]]:
            with self.subTest(values=values), self.assertRaises(ValueError):
                self.source(values).request(0x808000)

    def test_choice_keeps_cancel_and_option_order(self):
        s = self.source([2, 0x1a, 1, 5, 0x80, 0x11, 0x91, 0x22, 0x92, 0x33, 0x93])
        self.assertEqual(s.choice(0x808000)['cancel_then_options'], [0x809111, 0x809222, 0x809333])


if __name__ == '__main__':
    unittest.main()
