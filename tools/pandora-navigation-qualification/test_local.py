"""Input-level native/source mutations; only temporary copies are ever modified."""
import copy
import json
from pathlib import Path
import shutil
import sys
from tempfile import TemporaryDirectory
import unittest
import check

EXPORT, CAPTURE = map(Path, sys.argv[1:3])
sys.argv = sys.argv[:1]

class NativeMutations(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.baseline = check.report(EXPORT,CAPTURE)

    def setUp(self):
        self.tmp = TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        root=Path(self.tmp.name)
        self.export=root/'export';shutil.copytree(EXPORT,self.export)
        self.capture=root/'journey';self.capture.mkdir()
        for label in check.WITNESSES.values():
            shutil.copyfile(CAPTURE/f'{label}.wram',self.capture/f'{label}.wram')
        shutil.copyfile(CAPTURE.with_suffix('.jsonl'),self.capture.with_suffix('.jsonl'))

    def report(self):return check.report(self.export,self.capture)
    def metadata(self,change):
        p=self.export/'export.json';m=json.loads(p.read_text());change(m);p.write_text(json.dumps(m))

    def test_reference_round_trip_and_nonvacuous_transfers(self):
        self.assertEqual(self.baseline,json.loads(json.dumps(self.baseline)))
        for transfers in ([],[self.baseline['transitions'][0]]):
            self.metadata(lambda m:m.update(transfers=transfers))
            with self.assertRaises(ValueError):self.report()
    def test_opening_positive_and_event_provenance(self):
        self.metadata(lambda m:m['box_opening'].update(bounds=[120,369,152,400]))
        with self.assertRaises(ValueError):self.report()
        shutil.copyfile(EXPORT/'export.json',self.export/'export.json')
        p=self.capture/'21-entry-closed.wram';w=bytearray(p.read_bytes());w[0x6c0+0x26//8]^=1<<(0x26%8);p.write_bytes(w)
        with self.assertRaisesRegex(ValueError,'event.*agreement'):self.report()
    def test_extended_event_and_single_phase_cannot_disappear(self):
        p=self.capture/'landed-E.wram';original=p.read_bytes();w=bytearray(original);w[0x6c0+0x292//8]^=1<<(0x292%8);p.write_bytes(w)
        with self.assertRaisesRegex(ValueError,'opened shared phase'):self.report()
        p.write_bytes(original)
        p=self.capture.with_suffix('.jsonl');rows=[json.loads(l) for l in p.read_text().splitlines()]
        route=[json.loads(l) for l in (check.HERE.parent/'pandora-qualification/route.jsonl').read_text().splitlines()]
        labels=[r['label'] for r in route if 'label' in r];phase=set(labels[labels.index('return-town-down'):labels.index('return-home-open-rest')+1])
        for row in rows:
            if row['kind']=='frame' and row['label'] in phase:row['script']=0x84a258
        p.write_text('\n'.join(json.dumps(r) for r in rows)+'\n')
        with self.assertRaisesRegex(ValueError,'empty declared phase'):self.report()
    def test_full_native_hash_even_outside_sample(self):
        p=self.capture/'pandora-tour-control.wram';w=bytearray(p.read_bytes());w[-1]^=1;p.write_bytes(w)
        self.assertNotEqual(self.report(),self.baseline)

    def test_sample_word_cannot_hide_behind_full_hash(self):
        p=self.capture/'13-target-ready.wram';w=bytearray(p.read_bytes());w[0xa000+(8*64+22)*2]^=1;p.write_bytes(w)
        with self.assertRaisesRegex(ValueError,'sample difference'):self.report()

    def test_coordinated_export_hash_and_grid_change_still_compares_native(self):
        p=self.export/'13.grid';g=bytearray(p.read_bytes());g[(8*64+22)*2]^=1;p.write_bytes(g)
        self.metadata(lambda m: next(x for x in m['profiles'] if x['name']=='13').update(grid_sha256=check.sha(g)))
        with self.assertRaisesRegex(ValueError,'sample difference'):self.report()

    def test_halo_is_executable_admission(self):
        self.metadata(lambda m: next(x for x in m['profiles'] if x['name']=='13').update(halo=[23,7,25,16]))
        with self.assertRaisesRegex(ValueError,'outside admission'):self.report()

    def test_missing_frame_and_early_contact_callback(self):
        p=self.capture.with_suffix('.jsonl');original=p.read_text();rows=[json.loads(l) for l in original.splitlines()]
        i=next(i for i,r in enumerate(rows) if r['kind']=='frame' and r['frame']==26805)
        changed=copy.deepcopy(rows);changed[i]['events']=[1]+changed[i]['events']
        p.write_text('\n'.join(json.dumps(r) for r in changed)+'\n')
        with self.assertRaisesRegex(ValueError,'first actual contact boundary'):self.report()
        del rows[i];p.write_text('\n'.join(json.dumps(r) for r in rows)+'\n')
        with self.assertRaises(ValueError):self.report()

    def test_transition_and_contact_bounds_are_not_decorative(self):
        self.metadata(lambda m:m['transfers'][5].update(settled=[138,857]))
        with self.assertRaisesRegex(ValueError,'settled transition'):self.report()
        shutil.copyfile(EXPORT/'export.json',self.export/'export.json')
        self.metadata(lambda m:m['first_contact'].update(bounds=[123,360,149,400]))
        with self.assertRaisesRegex(ValueError,'first actual contact'):self.report()

if __name__=='__main__':unittest.main()
