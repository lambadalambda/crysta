"""Synthetic red/green protection of the bounded priority-survival comparison."""
import unittest
from scene_order import check_tilemap

def fixture():
    w=bytearray(0x20000);v=bytearray(0x10000)
    w[0x46d:0x46f]=bytes([0x3c,0x38])
    # All map cells select metatile 0, with alternating high/low quadrant words.
    for q in range(4):w[0x2000+q*2:0x2002+q*2]=(0x2001 if q%2==0 else 1).to_bytes(2,'little')
    for y in range(32):
        for x in range(32):
            a=(0x3800+y*32+x)*2
            v[a:a+2]=(0x2001 if x%2==0 else 1).to_bytes(2,'little')
    return w,v

class SceneOrderTests(unittest.TestCase):
    def test_full_word_comparison_exercises_both_priorities(self):
        w,v=fixture()
        self.assertEqual(check_tilemap(w,v,0),{'words':672,'high':336,'low':336})
        self.assertEqual(check_tilemap(w,v,256),{'words':672,'high':336,'low':336})
    def test_cleared_priority_wrong_layer_or_vacuous_coverage_rejected(self):
        w,v=fixture();v[(0x3800+2*32+2)*2+1]^=0x20
        with self.assertRaises(ValueError):check_tilemap(w,v,0)
        w,v=fixture();w[0x46d:0x46f]=bytes([0x38,0x3c])
        with self.assertRaises(ValueError):check_tilemap(w,v,0)
        w,v=fixture();w[0x2000:0x2008]=bytes(8);v[:]=bytes(len(v))
        with self.assertRaises(ValueError):check_tilemap(w,v,0)
    def test_bad_extents_or_camera_rejected(self):
        w,v=fixture()
        for wb,vb,cy in [(w[:-1],v,0),(w,v[:-1],0),(w,v,8)]:
            with self.assertRaises(ValueError):check_tilemap(wb,vb,cy)

if __name__=='__main__':unittest.main()
