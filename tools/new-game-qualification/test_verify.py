"""ROM-free checks of the bounded control witness, not a movement simulator."""
import copy
import unittest
from verify import verify_control


def witness():
    rows = {}
    for frame, x, y, joy, resume in [
        (6800, 304, 112, 0, 0x84A2A3),
        (6801, 304, 112, 0x100, 0x84A2A3),
        (6802, 304, 112, 0x100, 0x84A385),
        (6820, 331, 112, 0x100, 0x84A385),
        (6821, 332, 112, 0, 0x84A385),
        (6823, 332, 112, 0, 0x84A258),
        (6850, 332, 112, 0, 0x84A258),
        (6900, 332, 112, 0, 0x84A258),
        (6901, 332, 112, 0x400, 0x84A258),
        (6902, 332, 112, 0x400, 0x84A351),
        (6920, 332, 139, 0x400, 0x84A351),
        (6921, 332, 140, 0, 0x84A351),
        (6923, 332, 140, 0, 0x84A258),
        (7000, 332, 140, 0, 0x84A258),
        (7100, 332, 140, 0, 0x84A258),
    ]:
        rows[frame] = dict(map=15, x=x, y=y, joy=joy, resume=resume,
                           flags=0x414 if frame <= 6801 else 0x415,
                           flags8=0, intro=1, gate=0, state=0xAA, gate2=0, aux=0,
                           player_index=0x1000, controller_index=0x11C0)
    return rows


class ControlTest(unittest.TestCase):
    def test_admits_witness(self):
        verify_control(witness())

    def test_rejects_incomplete_or_unowned_witness(self):
        for frame, field, value in [
            (6900, 'x', 304), (7100, 'y', 112),  # no response
            (6900, 'x', 333), (7100, 'y', 141),  # fails to stop
            (6850, 'x', 333),  # intermediate drift, outside exact-sample table
            (6800, 'intro', 0), (6900, 'flags', 0x1415),
            (6802, 'resume', 0x84B975), (6900, 'map', 16),
            (6801, 'joy', 0), (6901, 'joy', 0),
            (6900, 'gate', 0x10), (6900, 'flags8', 0x100),
            (6900, 'gate2', 1), (6900, 'aux', 0x8000),
            (6900, 'controller_index', 0), (6900, 'player_index', 0),
        ]:
            rows = copy.deepcopy(witness())
            rows[frame][field] = value
            with self.subTest(frame=frame, field=field), self.assertRaises(AssertionError):
                verify_control(rows)
        rows = witness()
        del rows[6800]
        with self.assertRaises(AssertionError):
            verify_control(rows)


if __name__ == '__main__':
    unittest.main()
