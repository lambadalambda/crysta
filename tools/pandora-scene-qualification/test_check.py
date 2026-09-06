"""Nonvacuous exact raster, malformed source and native-piece mutation controls."""
from pathlib import Path
import importlib.util
import unittest
import tempfile

spec=importlib.util.spec_from_file_location('pandora_check',Path(__file__).with_name('check.py'))
check=importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)

class Checks(unittest.TestCase):
    def test_exact_transparency_mirror_and_palette(self):
        frame=bytes([8,8,16,0]+[0]*12+[1,0,0,8,0,0,0,0x24])
        tiles=bytes([1]+[0]*63)
        for flip in (False,True):
            bounds,indexed,priority=check.raster(frame,tiles,flip)
            self.assertEqual(bounds,[0,-16,8,-8] if flip else [-8,-16,0,-8])
            expected=bytearray(64);expected[7 if flip else 0]=161
            self.assertEqual(indexed,expected)
            expected_p=bytearray([255]*64);expected_p[7 if flip else 0]=2
            self.assertEqual(priority,expected_p)
            palette=bytes([0,0,31,0]+[0]*28)
            self.assertEqual(check.rgba(indexed,palette,160).count(b'\xff\x00\x00\xff'),1)
        with self.assertRaises(ValueError):check.raster(frame[:-1],tiles,False)
        with self.assertRaises(ValueError):check.raster(frame,b'',False)

    def test_checkpoint_hashes_cover_unselected_hardware(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            for suffix,size in [('wram',131072),('vram',65536),('cgram',512),('oam',544),('obj',2)]:
                (root/f'phase.{suffix}').write_bytes(bytes([2,0]) if suffix=='obj' else bytes(size))
            before=check.checkpoint_hashes(root,'phase')
            self.assertEqual(set(before),{'wram','vram','cgram','oam','obj'})
            # A byte outside selected OBJ tiles must still change strict evidence.
            changed=bytearray(65536);changed[42]=1
            (root/'phase.vram').write_bytes(changed)
            after=check.checkpoint_hashes(root,'phase')
            self.assertNotEqual(before['vram'],after['vram'])
            self.assertEqual({k:v for k,v in before.items() if k!='vram'},
                             {k:v for k,v in after.items() if k!='vram'})
            (root/'phase.vram').write_bytes(changed[:-1])
            with self.assertRaises(ValueError):check.checkpoint_hashes(root,'phase')

    def test_native_piece_mutations(self):
        # One asymmetric tile, independently arranged into hardware OAM/VRAM.
        frame=bytes([8,8,16,0]+[0]*12+[1,0,0,8,0,0,0,0x24])
        tiles=bytes([1]+[0]*63)
        palette=bytes([0,0,31,0]+[0]*28)
        w=bytearray(131072);v=bytearray(65536);cg=bytearray(512);o=bytearray([224]*512+[0]*32)
        v[0x8000]=128;cg[322:324]=bytes([31,0]);o[:4]=bytes([24,15,0,0x24])
        w[0x1040+24:0x1040+28]=bytes([8,0,16,0])
        e={'slot':0x1040,'position':[32,32],'flags':[0,0,0]}
        got=check.native_piece(frame,tiles,palette,160,e,w,v,cg,o)
        self.assertEqual(got['opaque'],1)
        self.assertEqual(got['first_oam'],0)
        for name,at in [('oam',0),('vram',0x8000),('cgram',322),('wram',0x1058)]:
            inputs={'wram':w[:],'vram':v[:],'cgram':cg[:],'oam':o[:]}
            inputs[name][at]^=1
            with self.assertRaises(ValueError,msg=name):
                check.native_piece(frame,tiles,palette,160,e,inputs['wram'],inputs['vram'],inputs['cgram'],inputs['oam'])
        high=o[:];high[512]=2
        with self.assertRaises(ValueError):check.native_piece(frame,tiles,palette,160,e,w,v,cg,high)

if __name__=='__main__':unittest.main()
