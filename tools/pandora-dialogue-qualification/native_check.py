"""Selected Pandora native font cells, not a route/provenance acceptance gate.

Consumes retained input-only captures only; never navigates or starts a core.
Original and parentfresh are explicit separate reference sets, never alternatives
chosen by a matching hash. Window colors, wait-arrow timing and cursor art are out
of scope. Every selected comparison must include foreground ink.
"""
from pathlib import Path
from hashlib import sha256
import json
import sys
from check import check, choices, default_assignments, require, u


def native_pixels(wram, vram, width=224, height=48):
    require(len(wram) == 0x20000 and len(vram) == 0x10000, 'native memory extent')
    anchor = (u(wram, 0xDB4), u(wram, 0xDB6))
    anchors = {(0x6A80,0x504), (0x6880,0x104)} if width == 224 else {(0x68C0,0x18C)}
    require(width in (192,224) and height == 48 and anchor in anchors, 'source-proven text window')
    out = []
    for y in range(height):
        for x in range(width):
            word = u(wram, 0x1D000+anchor[1]+y//8*64+x//8*2)
            tx = 7-x%8 if word & 0x4000 else x%8
            ty = 7-y%8 if word & 0x8000 else y%8
            at = (0xE000+(word & 1023)*16+ty*2) & 65535
            out.append(sum(((vram[at+plane] >> (7-tx)) & 1) << plane for plane in range(2)))
    return bytes(out)


def compare(page, expected, wram, vram, cursor=None):
    width, height = page['width'], page['height']
    require(len(expected) == width*height and page['glyphs'], 'nonempty page extent')
    actual = native_pixels(wram, vram, width, height)
    cells = set()
    for placement in page['glyphs']:
        gx, gy = placement['position']
        require(0 <= gx <= width-16 and 0 <= gy <= height-16, 'glyph geometry')
        # The native compositor advances 12 pixels although the font record is 16 wide.
        cells.update((x, y) for y in range(gy, gy+16) for x in range(gx, gx+12))
    count = ink = 0
    for x, y in sorted(cells, key=lambda xy: (xy[1], xy[0])):
        at = y*width+x
        if cursor and cursor[0] <= x < cursor[0]+8 and cursor[1] <= y < cursor[1]+16:
            require(expected[at] == page['background_index'], 'cursor must overlay only its reserved space')
            continue
        require(actual[at] == expected[at], f'native font cell {x},{y}')
        count += 1
        ink += expected[at] != page['background_index']
    require(count > 0 and ink > 0, 'nonvacuous native font comparison')
    return count, ink


def compare_capture(page, expected, wram, vram, catalog):
    require(len(wram) == 0x20000 and len(vram) == 0x10000, 'native memory extent')
    cursor = None
    if page['acknowledgement'] == 'none':
        option = u(wram, 0xDCE)
        require(option < 2, 'native selected option')
        cursor = catalog['options'][option]['position']
    else:
        require(wram[0xDC2]<<16 | u(wram, 0xDC0) == page['boundary_source'], 'native ack pointer')
    return compare(page, expected, wram, vram, cursor)


def inspect(rom, export, captures, capture_set):
    check(rom, export)
    meta = json.loads((export/'export.json').read_text())
    reference = json.loads(Path(__file__).with_name('native-reference.json').read_text())
    require(capture_set in reference['capture_sets'], 'explicit native capture set')
    selected = reference['capture_sets'][capture_set]
    covered = {s['page_id'] for data in reference['capture_sets'].values() for s in data['samples']}
    require([p['page_id'] for p in meta['pages'] if p['page_id'] not in covered] == reference['source_only_page_ids'],
            'aggregate source-only set')
    by_id = {p['page_id']: p for p in meta['pages']}
    catalog = choices(rom)[0]
    report = []
    for sample in selected['samples']:
        page = by_id[sample['page_id']]
        label = sample['label']
        require(Path(label).name == label and label not in ('.', '..'), 'capture label')
        wram = (captures/f'{label}.wram').read_bytes()
        vram = (captures/f'{label}.vram').read_bytes()
        require(all(u(wram, address) == value for address, value in default_assignments(rom).items()),
                'native default controller assignments')
        count, ink = compare_capture(page, (export/page['file']).read_bytes(), wram, vram, catalog)
        report.append(dict(label=label, page_id=page['page_id'], compared_pixels=count, foreground_pixels=ink,
                           wram_sha256=sha256(wram).hexdigest(), vram_sha256=sha256(vram).hexdigest()))
    require(report and report == selected['samples'], f'{capture_set} selected native reference')
    source_only = [p['page_id'] for p in meta['pages'] if p['page_id'] not in {r['page_id'] for r in report}]
    require(source_only == selected['source_only_page_ids'], 'explicit source-only set')
    result = dict(capture_set=capture_set, samples=report, source_only_page_ids=source_only,
                  total_compared_pixels=sum(p['compared_pixels'] for p in report),
                  total_foreground_pixels=sum(p['foreground_pixels'] for p in report))
    return result


if __name__ == '__main__':
    require(len(sys.argv) == 5, 'usage: native_check.py ROM EXPORT CAPTURES original|parentfresh|discovery')
    print(json.dumps(inspect(Path(sys.argv[1]).read_bytes(), Path(sys.argv[2]), Path(sys.argv[3]), sys.argv[4]), indent=2))
