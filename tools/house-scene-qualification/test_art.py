"""Synthetic independent raster/relocation checks (no ROM required)."""
import json
from pathlib import Path
import tempfile
import unittest
from art_check import exported, raster, relocate, rgba, sha, tile
from check import RESIDENTS

class ArtTests(unittest.TestCase):
    def test_planar_planes(self):
        b=bytearray(32);b[0]=128;b[17]=64
        self.assertEqual(tile(b)[:3],bytes([1,8,0]))

    def test_raster_order_flip_priority_and_transparency(self):
        # Two overlapping small parts: transparent first pixels expose second.
        c=bytes([0,8,0,0]+[0]*12+[2]+[0,0,0,0,0,0,0x24]+[0,0,0,0,0,1,0x14])
        tiles=bytearray(256*64);tiles[0]=3;tiles[64:128]=bytes([2]*64)
        bounds, indexed, priorities = raster(c,tiles,False)
        self.assertEqual(bounds,[0,0,8,8]);self.assertEqual(indexed[:2],bytes([163,162]))
        self.assertEqual(priorities[:2],bytes([2,1]))
        bounds, indexed, priorities = raster(c,tiles,True)
        self.assertEqual(bounds,[-8,0,0,8]);self.assertEqual(indexed[:8],bytes([162]*7+[163]))

    def test_palette_relocation_preserves_component_attributes(self):
        c=bytes([0]*16+[1]+[0,0,0,0,0,0xff,0x62])
        moved=relocate(c,1,5)
        self.assertEqual(moved[:-2],c[:-2]);self.assertEqual(moved[-2:],bytes([255,0x6a]))

    def test_rejects_bad_extent_and_large_boundary(self):
        with self.assertRaises(ValueError):raster(bytes(17),bytes(16384),False)
        c=bytes([0]*16+[1]+[1,0,0,0,0,15,0x20])
        with self.assertRaises(ValueError):raster(c,bytes(16384),False)

    def test_art_route_only_adds_passive_capture_boundaries(self):
        def inputs(name):
            rows=[json.loads(line) for line in Path('tools/house-scene-qualification',name).read_text().splitlines()]
            self.assertEqual(rows[-1],{'finish':True})
            return [tuple(c['buttons']) for c in rows[:-1] for _ in range(c['frames'])]
        self.assertEqual(inputs('route.jsonl'),inputs('art-route.jsonl'))

    def test_export_rejects_rehashed_wrong_raster_and_bad_sources(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);meta=synthetic_export(root)
            self.assertEqual(exported(b'x',root),meta)
            a=meta['actors'][0];f=a['frames'][0];path=root/f"{a['id']:06x}"/'0-indexed'
            original=path.read_bytes();wrong=bytes([162])+original[1:]
            path.write_bytes(wrong);f['indexed_sha256']=sha(wrong)
            (root/'export.json').write_text(json.dumps(meta))
            with self.assertRaisesRegex(ValueError,'independent raster'):exported(b'x',root)
            path.write_bytes(original);f['indexed_sha256']=sha(original)
            a['sources'][0]['end']=2
            (root/'export.json').write_text(json.dumps(meta))
            with self.assertRaisesRegex(ValueError,'source extent'):exported(b'x',root)


def synthetic_export(root):
    c=bytes([0]*16+[1]+[0,0,0,0,0,0,0x24]);tiles=bytes([1])+bytes(16383);palette=bytes(32)
    bounds,indexed,priorities=raster(c,tiles,False)
    blobs={'composition':c,'source-composition':c,'indexed':indexed,'priorities':priorities,'rgba':rgba(indexed,palette,160)}
    actors=[]
    for identity in sorted({v for m in RESIDENTS.values() for v in m.values()}|{0x88d618}):
        d=root/f'{identity:06x}';d.mkdir()
        for name,b in blobs.items():(d/f'0-{name}').write_bytes(b)
        (d/'tiles').write_bytes(tiles);(d/'palette').write_bytes(palette)
        frame={'index':0,'bounds':bounds,'opaque':1,**{name.replace('-','_')+'_sha256':sha(b) for name,b in blobs.items()}}
        actors.append({'id':identity,'hflip':False,'palette_base':160,'frames':[frame],
                       'tiles_sha256':sha(tiles),'palette_sha256':sha(palette),
                       'sources':[{'start':0,'end':1,'sha256':sha(b'x')}]})
    meta={'actors':actors};(root/'export.json').write_text(json.dumps(meta));return meta

if __name__=='__main__': unittest.main()
