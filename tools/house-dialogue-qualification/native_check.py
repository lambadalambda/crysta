"""Compare text/font cells with the event owner's fresh input-only journey.

This consumes captures, never boots/steps/patches the original CPU or owns a route.
Native windows, colors, wait-arrow animation and choice cursor are not page art.
"""
from pathlib import Path
from hashlib import sha256
import json
import sys
from check import check, choices, require, u

SAMPLES = [
    (0x888FDA,0,'north-door-settled'),
    (0x888FF0,0,'first-no-ack'),
    (0x888FF0,1,'choice-no-confirm'),
    (0x8890D9,0,'first-followup'),
    (0x8890D9,1,'repeat-A'),
    (0x8890D9,2,'followup-X'),
    (0x889156,0,'repeat-page'),
    (0x88918C,0,'repeat2-followup'),
    (0x88918C,1,'repeat2-page2'),
    (0x8891D6,0,'repeat-cancel-page'),
    (0x8891D6,1,'repeat-after-L'),
]

def native_pixels(wram, vram):
    require(len(wram) == 0x20000 and len(vram) == 0x10000, 'native memory extent')
    require(u(wram,0xDB6) == 0x504 and u(wram,0xDB4) == 0x6A80, 'standard text window')
    out = []
    for y in range(48):
        for x in range(224):
            word = u(wram,0x1D504+y//8*64+x//8*2)
            tx = 7-x%8 if word & 0x4000 else x%8
            ty = 7-y%8 if word & 0x8000 else y%8
            at = (0xE000+(word & 1023)*16+ty*2) & 65535
            out.append(sum(((vram[at+plane] >> (7-tx)) & 1) << plane for plane in range(2)))
    return bytes(out)

def compare(page, expected, wram, vram, cursor=None):
    actual = native_pixels(wram,vram)
    count = 0
    for placement in page['glyphs']:
        gx,gy = placement['position']
        # Native font compositor packs 12-pixel glyph cells, not 16-pixel advances.
        for y in range(gy,gy+16):
            for x in range(gx,gx+12):
                at = y*224+x
                if cursor and cursor[0] <= x < cursor[0]+8 and cursor[1] <= y < cursor[1]+16:
                    require(expected[at] == 3, 'cursor must overlay only its reserved space')
                    continue
                require(actual[at] == expected[at], f'native font cell {x},{y}')
                count += 1
    return count

def inspect(rom, export, captures):
    check(rom,export)
    meta = json.loads((export/'export.json').read_text())
    by_key = {(p['text_source'],p['index']):p for p in meta['pages']}
    catalogs = choices(rom)
    report = []
    for source,index,label in SAMPLES:
        page = by_key[source,index]
        wram = (captures/f'{label}.wram').read_bytes()
        vram = (captures/f'{label}.vram').read_bytes()
        cursor = None
        if page['acknowledgement'] == 'none':
            catalog = 0 if source == 0x888FF0 else 1
            selected = u(wram,0xDCE)
            require(selected < 2, 'native selected option')
            cursor = catalogs[catalog]['options'][selected]['position']
        count = compare(page,(export/page['file']).read_bytes(),wram,vram,cursor)
        if page['acknowledgement'] != 'none':
            require(wram[0xDC2]<<16 | u(wram,0xDC0) == page['boundary_source'], 'native ack pointer')
        report.append(dict(label=label,text_source=source,index=index,compared_pixels=count,
                           wram_sha256=sha256(wram).hexdigest(),vram_sha256=sha256(vram).hexdigest()))
    reference = json.loads(Path(__file__).with_name('native-reference.json').read_text())
    require(report == reference['samples'], 'selected fresh native reference')
    print(json.dumps(dict(samples=report,total_compared_pixels=sum(p['compared_pixels'] for p in report)),indent=2))

if __name__ == '__main__':
    inspect(Path(sys.argv[1]).read_bytes(),Path(sys.argv[2]),Path(sys.argv[3]))
