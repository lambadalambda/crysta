#!/usr/bin/env python3
"""Mutation controls for the standalone Pandora background comparator."""
import copy
from pathlib import Path
import tempfile
import unittest
import check

class Checks(unittest.TestCase):
    def test_tilewords_include_flip_palette_priority_and_dynamic_differences(self):
        definitions = bytes([1, 0x28, 2, 0, 3, 0, 4, 0]) * 512
        cells = [0] * 256
        self.assertEqual(check.tileword(definitions, cells, 16, 0, 0), 0x2801)
        self.assertEqual(check.tileword(definitions, cells, 16, 1, 1), 4)
        for data in [b'', definitions[:-1]]:
            with self.assertRaises(ValueError): check.tileword(data, cells, 16, 0, 0)
        for x, y in [(-1, 0), (32, 0), (0, 32)]:
            with self.assertRaises(ValueError): check.tileword(definitions, cells, 16, x, y)

    def test_indexed_sampling_preserves_priority_and_transparency(self):
        graphics = bytearray(64)
        graphics[32] = 128
        self.assertEqual(check.sample(graphics, 0x2801, 0, 0), (33, True))
        self.assertEqual(check.sample(graphics, 0x6801, 7, 0), (33, True))
        self.assertEqual(check.sample(graphics, 0xa801, 0, 7), (33, True))
        self.assertEqual(check.sample(graphics, 0x2801, 1, 0), (0, False))
        self.assertNotEqual(check.sample(graphics, 0x0801, 0, 0), (33, True))
        with self.assertRaises(ValueError): check.sample(graphics[:32], 1, 0, 0)

    def test_full_grid_differences_never_mask_bit15_or_changed_tile(self):
        self.assertEqual(check.differences([1, 2, 3], [0x8001, 4, 3]), [[0, 1, 0x8001], [1, 2, 4]])
        with self.assertRaises(ValueError): check.differences([1], [])
        self.assertEqual(check.attributed([0, 1, 511], bytes([0xff]) * 512), [0xfe00, 0xfe01, 0xffff])
        with self.assertRaises(ValueError): check.attributed([512], bytes(512))

    def test_settled_camera_rejects_extent_and_clamps(self):
        self.assertEqual(check.origin([364, 815], [0, 0, 1024, 1024]), [236, 703])
        self.assertEqual(check.origin([0, 0], [0, 768, 256, 1024]), [0, 768])
        self.assertEqual(check.origin([136, 359], [0, 0, 256, 512]), [0, 247])
        with self.assertRaises(ValueError): check.origin([0, 0], [0, 0, 0, 0])

    def test_comparison_gate_rejects_word_index_priority_camera_mutations(self):
        good = {'tileword_errors': [], 'unexpected_pixels': [], 'camera': [0, 0], 'source_camera': [0, 0], 'visible_phase_changed_cells': 0}
        check.verify_point(good)
        for field, value in [('tileword_errors', [[0, 1, 2]]), ('unexpected_pixels', [[0, 1, 2]]), ('camera', [1, 0])]:
            bad = dict(good, **{field: value})
            with self.assertRaises(ValueError): check.verify_point(bad)

class Pipeline(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name); self.export = self.root/'21'; self.export.mkdir()
        self.rom = bytearray(0x200000)
        self.camera = {'record_offset':0x16be72,'record':[0x10,0x20],'scene_offset':0x39268,
            'display_offset':0x16bcda,'display':[0x15,0,0x20,0xb3,0x64,0,9,0,0],
            'bounds':[0,0,256,512],'vertical_extent':256,'hardware_background':1,'ring_word_base':0x3800,'bgmode':9}
        self.rom[0x38042:0x38044] = bytes([0x68,0x92])
        self.rom[0x39268:0x3926a] = bytes([0,0x1b])
        self.rom[0x16bb9a:0x16bb9c] = bytes([0xda,0xbc])
        self.rom[0x16bcda:0x16bce3] = bytes(self.camera['display'])
        self.rom[0x16be72:0x16be74] = bytes(self.camera['record'])
        self.w = bytearray(131072); self.v = bytearray(65536); self.c = bytes(512)
        def put(at, value): self.w[at:at+2] = value.to_bytes(2,'little')
        for at,value in [(0x47e,33),(0x1000,128),(0x1002,112),(0x866,256),
                (0x878,256),(0x87a,512),(0x826,256),(0x82a,512)]: put(at,value)
        self.w[0x46d:0x46f] = bytes([0x38,0x3c])
        graphics = bytearray(64)
        for y in range(8):
            for plane in range(4): graphics[32+(plane//2)*16+y*2+plane%2] = 1 << (7-plane)
        definitions = bytes([1,0x28])*2048
        self.w[0x2000:0x3000] = definitions
        self.v[:64] = graphics; self.v[0x7000:0x7800] = bytes([1,0x28])*1024
        indices = bytearray(); priorities = bytearray()
        for y in range(512):
            for x in range(256):
                i,p = check.sample(graphics,0x2801,x%8,y%8)
                indices.append(i); priorities.append(p)
        planes = {'cells':bytes(1024),'static-grid':bytes(1024),'attributes':bytes(512),
            'definitions':definitions,'graphics':graphics,'palette':bytes(256),'indices':indices,'priorities':priorities}
        self.profile = {'map':33,'width':256,'height':512,'camera':self.camera,'sources':[],
            'files':{n:check.sha(b) for n,b in planes.items()}}
        for name,data in planes.items(): (self.export/name).write_bytes(data)
        self.save()

    def save(self):
        for suffix,data in [('wram',self.w),('vram',self.v),('cgram',self.c)]:
            (self.root/f'point.{suffix}').write_bytes(data)
    def inspect(self): return check.inspect(self.rom,self.root,self.export,self.profile,'point')

    def test_positive_real_pipeline_and_all_planes(self):
        result = self.inspect()
        self.assertEqual(result['tile_words'],896)
        self.assertEqual(result['source_phase_equal_pixels'],57344)
        for plane in range(4):
            self.assertEqual(check.sample((self.export/'graphics').read_bytes(),0x2801,plane,0),(32+(1<<plane),True))

    def test_border_tileword_id_palette_priority_and_flip_mutations(self):
        original = bytes(self.v)
        for offset,bit in [(0x7000,1),(0x7001,4),(0x7001,0x20),(0x7001,0x40),(0x7001,0x80),(0x76ff,0x20)]:
            self.v[:] = original; self.v[offset] ^= bit; self.save()
            with self.assertRaises(ValueError): self.inspect()

    def test_coordinated_visible_grid_and_ring_mutation_is_not_qualified_phase(self):
        self.w[0xa000] = 1
        # Both definitions are identical, so native ring remains internally consistent.
        self.save()
        with self.assertRaises(ValueError): self.inspect()

    def test_camera_transport_and_pairs_are_independently_checked(self):
        original = copy.deepcopy(self.camera)
        for name,value in [('record_offset',0x16be74),('scene_offset',0x3926a),('ring_word_base',0x3c00),('bgmode',1),('record',[1,2]),('hardware_background',2)]:
            self.camera.clear(); self.camera.update(copy.deepcopy(original)); self.camera[name]=value
            with self.assertRaises(ValueError): self.inspect()
        self.camera.clear(); self.camera.update(original)
        for at in [0x80e,0x81e,0x874,0x866]:
            self.w[at] ^= 1; self.save()
            with self.assertRaises(ValueError): self.inspect()
            self.w[at] ^= 1

    def test_fine_scroll_nonuniform_ring_alias_and_partial_edges(self):
        self.profile['width'] = 512
        self.camera['record'] = [0x20,0x20]; self.camera['bounds'] = [0,0,512,512]
        self.rom[0x16be72] = 0x20
        for at,value in [(0x1000,132),(0x1002,115),(0x80e,4),(0x81e,4),(0x812,3),(0x822,3),(0x878,512),(0x826,512)]:
            self.w[at:at+2] = value.to_bytes(2,'little')
        cells = [1 if x==16 else 0 for y in range(32) for x in range(32)]
        grid = b''.join(c.to_bytes(2,'little') for c in cells)
        definitions = bytearray((self.export/'definitions').read_bytes()); definitions[8:16] = bytes([1,0x2c])*4
        self.w[0x2000:0x3000] = definitions; self.w[0xa000:0xa800] = grid
        graphics = (self.export/'graphics').read_bytes(); indices=bytearray(); priorities=bytearray()
        for y in range(512):
            for x in range(512):
                word = check.tileword(definitions,cells,32,x//8,y//8)
                i,p = check.sample(graphics,word,x%8,y%8); indices.append(i); priorities.append(p)
        for name,data in [('cells',grid),('static-grid',grid),('definitions',definitions),('indices',indices),('priorities',priorities)]:
            (self.export/name).write_bytes(data); self.profile['files'][name] = check.sha(data)
        self.save(); result = self.inspect()
        self.assertEqual(result['tile_words'],957)
        self.assertEqual(result['source_phase_equal_pixels'],57344)
        self.assertEqual(result['ring_alias_different_words'],29)
        self.assertEqual(result['ring_alias_different_pixels'],896)
        self.v[0x7001] ^= 0x20; self.save()
        with self.assertRaises(ValueError): self.inspect()

    def test_one_sided_capture_and_missing_point_fail(self):
        parent = self.root/'parent'; parent.mkdir()
        for suffix in ('wram','vram','cgram'):
            (parent/f'point.{suffix}').write_bytes((self.root/f'point.{suffix}').read_bytes())
        profiles = [self.profile]
        check.compare_journeys(self.rom, self.root, profiles, self.root, parent, {'point':33})
        changed = bytearray(self.w); changed[0xa001] ^= 0x80
        (parent/'point.wram').write_bytes(changed)
        with self.assertRaises(ValueError):
            check.compare_journeys(self.rom, self.root, profiles, self.root, parent, {'point':33})
        (parent/'point.wram').unlink()
        with self.assertRaises(FileNotFoundError):
            check.compare_journeys(self.rom, self.root, profiles, self.root, parent, {'point':33})

class Animations(unittest.TestCase):
    def test_source_payload_membership_and_destinations_not_change_masks(self):
        rom = bytearray(0x200000); base=bytes(0x6000); native=bytearray(base)
        for scene,selector,bank,table,destinations,size in check.ANIMATIONS[0x13]:
            rom[scene:scene+5] = bytes([0xfb,selector,0xe8,0x98,0x87])
            rom[bank+selector*2:bank+selector*2+2] = (table-bank).to_bytes(2,'little')
            source = bank+0x7000; destination=destinations[0]
            rom[table:table+8] = bytes([1])+ (source-bank).to_bytes(2,'little')+(destination//2).to_bytes(2,'little')+size.to_bytes(2,'little')+bytes([5])
            rom[source:source+size] = bytes([42])*size
            native[destination:destination+size] = rom[source:source+size]
        reconstructed,witnesses = check.reconstruct_graphics(rom,base,native,0x13)
        self.assertEqual(reconstructed,native); self.assertEqual(len(witnesses),1)
        for at in [0,destination]:
            native[at] ^= 1
            with self.assertRaises(ValueError): check.reconstruct_graphics(rom,base,native,0x13)
            native[at] ^= 1
        rom[table+3] ^= 1
        with self.assertRaises(ValueError): check.reconstruct_graphics(rom,base,native,0x13)

if __name__ == '__main__': unittest.main()
