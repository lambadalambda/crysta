"""ROM-free negative tests for the bounded native evidence checker."""
import unittest
from check import door_delta, selected


def fixture():
    before = bytearray(131072)
    def put(at, value): before[at:at+2] = value.to_bytes(2, 'little')
    put(0x47e, 12); put(0x1000, 136); put(0x1002, 352)
    put(0xa4d0, 0x1cf2); put(0xa510, 0x1cf3)
    before[0x6c0+4] = 1
    before[0x6c0+31] = 8
    after = bytearray(before)
    after[0xa4d0:0xa4d2] = bytes.fromhex('f61c')
    after[0xa510:0xa512] = bytes.fromhex('f700')
    return before, after


class DoorEvidence(unittest.TestCase):
    def test_first_coarse_failure_does_not_fall_through(self):
        first={'rectangle':[1,1,1,2]}
        second={'rectangle':[1,1,2,2]}
        self.assertIsNone(selected([first,second],(25,32)))
        self.assertIs(selected([first,second],(24,32)),first)

    def test_exact_final_patch(self):
        self.assertEqual(door_delta(*fixture()), [[8, 19, 0x1cf2, 0x1cf6], [8, 20, 0x1cf3, 0xf7]])

    def test_reject_missing_upper_visual_selector(self):
        a,b=fixture(); b[0xa4d0:0xa4d2]=a[0xa4d0:0xa4d2]
        with self.assertRaises(ValueError): door_delta(a,b)

    def test_reject_unrelated_collision_change(self):
        a,b=fixture(); b[0xa800]^=1
        with self.assertRaises(ValueError): door_delta(a,b)

    def test_no_conversation_progression_grant(self):
        a,b=fixture(); b[0x6c0+4]|=64  # event0026
        with self.assertRaises(ValueError): door_delta(a,b)

    def test_reject_truncation_and_wrong_target(self):
        a,b=fixture()
        with self.assertRaises(ValueError): door_delta(a,b[:-1])
        b[0x1000]=135
        with self.assertRaises(ValueError): door_delta(a,b)


if __name__ == '__main__': unittest.main()
