"""Synthetic tests for census traversal; no ROM or native assets required."""
import unittest
from check import hardware_census
from census import exit_records, native_entities, require, oam_ownership, verify_auxiliary, verify_ordinary_order

class CensusTests(unittest.TestCase):
    def test_exit_list_exact_boundaries_and_source_addresses(self):
        rom=bytearray(0x20000)
        rom[0x18020:0x18022]=bytes([0,0x90])
        rom[0x19000:0x1900d]=bytes([17,25,1,2,12,0,0,7,208,0,160,1,255])
        records,extent=exit_records(rom,16)
        self.assertEqual(extent,[0x19000,0x1900d])
        self.assertEqual(records,[{'source':0x819000,'rectangle':[17,25,1,2],'destination':12,'mode':0,'selector':7,'position':[208,416]}])
        rom[0x1900c]=0
        with self.assertRaises(ValueError): exit_records(rom,16)
    def test_rejects_bank_crossing_and_truncated_exit(self):
        rom=bytearray(0x20000);rom[0x18020:0x18022]=bytes([250,255])
        with self.assertRaises(ValueError):exit_records(rom,16)
        with self.assertRaises(ValueError):exit_records(rom[:0x18021],16)
    def test_linked_membership_is_not_nonzero_slot_guess(self):
        w=bytearray(131072)
        def word(p,v):w[p:p+2]=v.to_bytes(2,'little')
        word(0xdfc,0x1080);word(0x10ac,0x1000);word(0x1000,300);word(0x1040,999)
        entities,unlinked=native_entities(w)
        self.assertEqual([e['slot'] for e in entities],[0x1080,0x1000])
        self.assertEqual([e['slot'] for e in unlinked],[0x1040])
        word(0x102c,0x1080)
        with self.assertRaises(ValueError):native_entities(w)
        with self.assertRaises(ValueError):native_entities(w[:100])
    def test_oam_coverage_separates_auxiliary_overlay(self):
        w=bytearray(131072);r=bytearray(0x250000);o=bytearray([224]*512+[0]*32)
        def word(p,v):w[p:p+2]=v.to_bytes(2,'little')
        word(0xdfc,0x1000);word(0x1000,16);word(0x1002,16)
        word(0x1010,0x8000);w[0x1012]=0xa4;word(0x1100a,0x8010)
        r[0x24801c]=1;r[0x24801d:0x248024]=bytes([0,0,0,0,0,1,0x20])
        o[:4]=bytes([16,15,0,32]);w[0xa00:0xa04]=o[:4]
        o[4:8]=bytes([20,20,176,52]);o[512]=8
        entities,_=native_entities(w)
        owners,aux=oam_ownership(r,w,o,entities)
        self.assertEqual([(v['entity'],v['first_oam']) for v in owners],[(0x1000,0)])
        self.assertEqual([(v['slot'],v['priority'],v['size']) for v in aux],[(1,3,16)])
        o[3]=48
        with self.assertRaises(ValueError):oam_ownership(r,w,o,entities)
        o[3]=32;o[512]|=1
        with self.assertRaises(ValueError):oam_ownership(r,w,o,entities)
        o[512]=8;w[0xa00]^=1
        with self.assertRaises(ValueError):oam_ownership(r,w,o,entities)
    def test_ordinary_depth_tie_uses_link_order_and_excludes_special_paths(self):
        def e(slot,y,flags=0,override=0):return {'slot':slot,'position':[0,y],'flags':[0,flags,0],'draw_override':override}
        def o(slot,index):return {'entity':slot,'first_oam':index}
        entities=[e(0x1080,100),e(0x1040,110)]
        owners=[o(0x1040,0),o(0x1080,1)]
        verify_ordinary_order(entities,owners,0)
        entities[1]['position'][1]=100
        verify_ordinary_order(entities,owners,0) # lower slot wins only because later linked
        with self.assertRaises(ValueError):verify_ordinary_order(list(reversed(entities)),owners,0)
        with self.assertRaises(ValueError):verify_ordinary_order(entities,list(reversed([o(0x1040,1),o(0x1080,0)])),0)
        entities.extend([e(0x10c0,255,0x2001),e(0x1100,255,override=1)])
        verify_ordinary_order(entities,owners+[o(0x10c0,3),o(0x1100,4)],0)
        entities[0]['position'][1]=256
        with self.assertRaises(ValueError):verify_ordinary_order(entities,owners,0)
    def test_auxiliary_requires_bounded_text_producer_not_arbitrary_remainder(self):
        r=bytearray(0x400000);w=bytearray(131072);o=bytearray(544)
        r[0x38cda:0x38ce2]=bytes([0x80,0xa2,0x82,0x56,0x53,0x80,0x47,0xd4])
        w[0x47e]=13;w[0x10806:0x10809]=bytes([0xda,0x8c,0x83]);w[0x10810]=24
        aux=[]
        for i in range(4):
            x=104+i*12;tile=176+i*2
            w[0x10992+i*6:0x10998+i*6]=bytes([x,0,48,0,tile,52])
            o[i*4:i*4+4]=bytes([x,48,tile,52])
            o[512]|=2<<(i*2)
            aux.append({'slot':i,'x':x,'y':48,'tile':tile,'palette':2,'priority':3,'size':16})
        verify_auxiliary(r,w,o,aux)
        w[0x10810]=0
        with self.assertRaises(ValueError):verify_auxiliary(r,w,o,aux)
        w[0x10810]=24;w[0x10992]+=1
        with self.assertRaises(ValueError):verify_auxiliary(r,w,o,aux)
    def test_hardware_fingerprint_excludes_only_framebuffer_diagnostic(self):
        a={'entities':[1], 'hashes':{'pixels':'a','wram':'same','oam':'same'}}
        b={'entities':[1], 'hashes':{'pixels':'b','wram':'same','oam':'same'}}
        self.assertEqual(hardware_census(a),hardware_census(b))
        b['hashes']['oam']='tampered'
        self.assertNotEqual(hardware_census(a),hardware_census(b))
        self.assertIn('pixels',a['hashes'])
    def test_checks_do_not_depend_on_assert(self):
        with self.assertRaises(ValueError):require(False,'tampered')

if __name__=='__main__':unittest.main()
