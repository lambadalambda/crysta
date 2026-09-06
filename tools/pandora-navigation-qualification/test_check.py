"""Mutation controls use explicit exceptions (also run with python -O)."""
import copy
import unittest
import check

class Samples(unittest.TestCase):
    def setUp(self):
        self.profile={'name':'a','map':10,'width':2,'halo':[0,0,2,2],
                      'actors':[{'source':0x838a19,'position':[8,16]}]}
    def test_real_edge_geometry(self):
        self.assertEqual(check.samples((360,144),'Up'),[(22,8)])
        self.assertEqual(check.samples((361,145),'Right'),[(23,8),(23,9)])
        with self.assertRaises(ValueError):check.samples((7,16),'Up')
    def test_exact_words_and_narrow_source_freeze_only(self):
        p=self.profile
        self.assertFalse(check.compare_sample(p,[3]*4,[3]*4,(0,0)))
        self.assertTrue(check.compare_sample(p,[0x8003]*4,[3]*4,(0,0)))
        for cell,source,native in [((1,0),0x8003,3),((0,0),0x8004,3),((0,0),3,0x8003),((0,0),3,4),((2,0),3,3)]:
            with self.subTest(cell=cell,source=source,native=native),self.assertRaises(ValueError):
                check.compare_sample(p,[source]*4,[native]*4,cell)
        p=copy.deepcopy(p);p['actors'][0]['source']=0x8389bf
        with self.assertRaises(ValueError):check.compare_sample(p,[0x8003]*4,[3]*4,(0,0))
    def test_materials_are_direction_cell_and_mode_scoped(self):
        self.assertEqual(check.classify(10,(21,20),25<<9,'Left',False,0),'solid')
        self.assertEqual(check.classify(12,(11,21),0x0b81,'Up',True,0x20),'partial')
        self.assertEqual(check.classify(14,(6,53),0x3acb,'Up',False,0),'open')
        for m,c,r,d,old,control in [(14,(6,53),0x3acb,'Right',False,0),(14,(7,53),0x3acb,'Up',False,0),
                (12,(11,21),0x0b81,'Left',False,0),(19,(22,8),25<<9,'Up',False,0),
                (10,(21,20),0,'Up',False,0x50),(10,(21,20),0x8c00,'Up',True,0)]:
            with self.assertRaises(ValueError):check.classify(m,c,r,d,old,control)
    def test_transition_is_settled_not_early_map_id(self):
        s={'destination':14,'settled':[152,880]}
        r={'map':14,'position':[152,880],'script':0x84a258,'control':0}
        check.check_transition(r,s)
        for key,value in [('map',12),('position',[138,857]),('script',0x84bb36),('control',0x20)]:
            with self.subTest(key=key),self.assertRaises(ValueError):
                check.check_transition(dict(r,**{key:value}),s)
    def test_optimized_python_still_checks(self):
        with self.assertRaises(ValueError):check.require(False,'mutation')

if __name__=='__main__':unittest.main()
