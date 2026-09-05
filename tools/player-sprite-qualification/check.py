"""Authenticate the selected hardware/source evidence, not a full scene latch."""
import json, sys
from pathlib import Path
from evidence import digest, inspect
HERE=Path(__file__).parent

def check(rom, capture):
    r=Path(rom).read_bytes(); out=Path(capture)
    sources=json.loads((HERE/'sources.json').read_text())
    assert digest(r)==sources['rom_sha256']
    for extent in sources['sources']+sources['consumers']:
        assert digest(r[extent['start']:extent['end']])==extent['sha256'],extent
    reference=json.loads((HERE/'reference.json').read_text())
    for run in ('a','b'):
        report=[inspect(r,out/run,p['frame']) for p in reference]
        assert report==reference, f'{run}: changed selected pose/hardware evidence'
    exported=json.loads((out/'export/export.json').read_text())
    assert exported=={k:v for k,v in sources.items() if k!='consumers'}
    for f in exported['frames']:
        name=f"{f['id']:06x}-{'left' if f['mirror'] else 'normal'}"
        for ext,key,stride in [('rgba','rgba_sha256',4),('priority','priority_sha256',1)]:
            b=(out/'export'/f'{name}.{ext}').read_bytes()
            assert len(b)==f['width']*f['height']*stride
            assert digest(b)==f[key]
    print('Qualified: matching selected OAM/VRAM/CGRAM witnesses and 28 ROM-only exports.')

if __name__=='__main__':check(*sys.argv[1:])
