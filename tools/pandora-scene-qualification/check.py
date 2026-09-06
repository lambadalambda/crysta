"""Strict source/OAM-piece qualification. Never reads native .pixels or starts a Session."""
from pathlib import Path
import hashlib
import json
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'house-scene-qualification'))
from census import native_entities, require, sha, u

ROM_SHA = 'f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'


def tile(data):
    require(len(data) == 32, 'planar tile extent')
    return bytes(sum(((data[(p//2)*16+y*2+p%2] >> (7-x)) & 1) << p
                     for p in range(4)) for y in range(8) for x in range(8))


def components(data):
    require(len(data) >= 17 and 0 < data[16] <= 128 and len(data) == 17+data[16]*7,
            'composition extent')
    parts = [data[at:at+7] for at in range(17, len(data), 7)]
    require(all(p[0] in (0, 1) for p in parts), 'component size')
    return parts


def raster(data, tiles, flip):
    require(len(tiles) % 64 == 0 and len(tiles) > 0, 'tile plane extent')
    h = int(flip)
    ax = int.from_bytes(data[h:h+1], 'little', signed=True)
    ay = data[2]
    pixels = {}
    rectangles = []
    for part in components(data):
        size = 16 if part[0] else 8
        x, y = part[1+h]-ax, part[3]-ay
        rectangles.append((x, y, x+size, y+size))
        word = u(part, 5)
        for dy in range(size):
            for dx in range(size):
                tx = size-1-dx if bool(word & 0x4000) ^ flip else dx
                ty = size-1-dy if word & 0x8000 else dy
                tid = (word & 511) + tx//8 + 16*(ty//8)
                require(tid*64+63 < len(tiles), 'missing source tile')
                color = tiles[tid*64+(ty % 8)*8+tx % 8]
                if color:
                    pixels.setdefault((x+dx, y+dy), (128+(word >> 9 & 7)*16+color, word >> 12 & 3))
    bounds = [min(r[0] for r in rectangles), min(r[1] for r in rectangles),
              max(r[2] for r in rectangles), max(r[3] for r in rectangles)]
    l, t, r, b = bounds
    values = [pixels.get((x, y), (0, 255)) for y in range(t, b) for x in range(l, r)]
    return bounds, bytes(p[0] for p in values), bytes(p[1] for p in values)


def rgba(indexed, palette, base):
    require(len(palette) == 32, 'palette extent')
    out = bytearray()
    for i in indexed:
        if not i:
            out.extend(bytes(4))
        else:
            require(base < i < base+16, 'palette index')
            word = u(palette, (i-base)*2)
            out.extend(((word >> s & 31) << 3) | ((word >> s & 31) >> 2) for s in (0, 5, 10))
            out.append(255)
    return bytes(out)


def exported(rom, root):
    meta = json.loads((root/'export.json').read_text())
    require(meta['rom_sha256'] == sha(rom) == ROM_SHA, 'Japanese ROM authentication')
    require(len(meta['art']) == 16 and len({a['id'] for a in meta['art']}) == 16, 'unique complete art')
    frame_count = 0
    for extent in meta['sources']:
        a, b = extent['start'], extent['end']
        require(0 <= a < b <= len(rom), 'source range')
        require(sha(rom[a:b]) == extent['sha256'], 'source hash')
    for art in meta['art']:
        d = root/f"{art['id']:06x}"
        tiles, palette = [(d/name).read_bytes() for name in ('tiles', 'palette')]
        for name, data in [('tiles', tiles), ('palette', palette)]:
            require(sha(data) == art[name+'_sha256'], 'resource hash')
        for seq in art['lists']:
            require(seq['frames'], 'empty list')
            for frame in seq['frames']:
                frame_count += 1
                name = f"{seq['selector']}-{frame['index']}"
                c, raw = [(d/f'{name}-{suffix}').read_bytes() for suffix in ('composition', 'source')]
                require(sha(c) == frame['composition_sha256'] and sha(raw) == frame['source_sha256'], 'composition hash')
                adjusted = bytearray(raw)
                for i, part in enumerate(components(raw)):
                    adjusted[22+i*7:24+i*7] = ((u(part, 5) & ~0x0e00) | ((art['palette_base']-128)//16 << 9)).to_bytes(2, 'little')
                require(bytes(adjusted) == c, 'palette-only source relocation')
                require([m['flip'] for m in frame['mirrors']] == [False, True], 'both mirror rasters')
                for mirror in frame['mirrors']:
                    bounds, indexed, priority = raster(c, tiles, mirror['flip'])
                    opaque = sum(i != 0 for i in indexed)
                    require(bounds == mirror['bounds'] and opaque == mirror['opaque'] and opaque > 0, 'nonempty source bounds/raster')
                    products = [('indexed', indexed), ('priority', priority), ('rgba', rgba(indexed, palette, art['palette_base']))]
                    for suffix, data in products:
                        actual = (d/f"{name}-{int(mirror['flip'])}-{suffix}").read_bytes()
                        require(actual == data and sha(data) == mirror[suffix+'_sha256'], 'pixel-exact independent '+suffix)
    require(frame_count > 150, 'nonvacuous complete list coverage')
    return meta


def native_piece(frame, tiles, palette, base, entity, wram, vram, cgram, oam):
    require([len(b) for b in (wram, vram, cgram, oam)] == [131072, 65536, 512, 544], 'native extents')
    p = entity['slot']
    flags = entity['flags'][2]
    require(not flags & 0x8000, 'unqualified native V flip')
    flip = bool(flags & 0x4000)
    ax = int.from_bytes(frame[int(flip):int(flip)+1], 'little', signed=True)
    ay = frame[2]
    require([u(wram, p+24), u(wram, p+26)] == [ax & 65535, ay], 'source/native anchors')
    pattern = bytearray()
    highs = []
    seen_tiles = []
    checked_tiles = set()
    for c in components(frame):
        word = u(c, 5)
        source_tile = word & 511
        if p == 0x1000 or entity['flags'][1] & 0x8000:
            if source_tile not in seen_tiles:
                seen_tiles.append(source_tile)
            j = seen_tiles.index(source_tile)
            native_tile = j*2+(16 if j >= 8 else 0)+(0 if p == 0x1000 else 0xe0)
        else:
            native_tile = (source_tile + (256 if flags & 256 else 0)) & 511
        x = (entity['position'][0]+c[1+int(flip)]-ax-u(wram, 0x81e)) & 511
        y = (entity['position'][1]+c[3]-ay-u(wram, 0x822)-1) & 255
        # Native per-actor priority/palette override is retained as evidence. This
        # does not bake a lifting/scripted priority into immutable source rasters.
        attr = ((word & 0xfe00) ^ (flags & 0xc000)) | (flags & 0x3e00)
        pattern.extend([x & 255, y, native_tile & 255, (attr | native_tile) >> 8])
        highs.append((x >> 8) | (c[0] << 1))
        for delta in ([0, 1, 16, 17] if c[0] else [0]):
            tid, native = source_tile+delta, native_tile+delta
            expected = tiles[tid*64:(tid+1)*64]
            require(len(expected) == 64, 'source selected tile extent')
            observed = tile(vram[0x8000+native*32:0x8000+(native+1)*32])
            require(expected == observed, f'native selected tile {tid:03x}/{native:03x}')
            checked_tiles.add(tid)
    matches = [at for at in range(0, 512-len(pattern)+1, 4) if oam[at:at+len(pattern)] == pattern]
    require(len(matches) == 1, 'native contiguous source OAM composition')
    first = matches[0]//4
    for i, high in enumerate(highs):
        slot = first+i
        require(oam[512+slot//4] >> (slot % 4*2) & 3 == high, 'native OAM ninth-X/size')
    bounds, indexed, _ = raster(frame, tiles, flip)
    used = set(indexed)-{0}
    require(used, 'nonvacuous selected native raster')
    for color in used:
        require(u(cgram, color*2) == u(palette, (color-base)*2), f'native used source palette {color}')
    return {'first_oam': first, 'components': len(highs), 'selected_tiles': sorted(checked_tiles),
            'opaque': sum(i != 0 for i in indexed), 'bounds': bounds,
            'indexed_sha256': sha(indexed), 'actor_override': flags & 0x3e00}


# Each witness is explicit; no best-match capture search, no native RGB dependency.
WITNESSES = [
    ('town-gap-up-rest', 0x1040, 0x83ebe4),
    ('town-door-ready', 0x1200, 0x83ec7e),
    ('town-gap-up-rest', 0x1280, 0x83ed37),
    ('13-admit', 0x1040, 0x83ecc6),
    ('C-direct-ready', 0x1040, 0x83ed5a),
    ('C-direct-ready', 0x10c0, 0x83ed74),
    ('C-direct-ready', 0x1100, 0x83edf8),
    ('21-box-contact-rest', 0x1100, 0x83f984),
    ('21-opening', 0x1080, 0x83f8c0),
    ('tutorial-002', 0x1080, 0x83f8c0),
    ('tutorial-016', 0x1040, 0x83f8a8),
    ('tutorial-040', 0x1080, 0x89d9f9),
    ('left-pot-held', 0x12c0, 0x96e1a6),
    ('middle-held', 0x12c0, 0x96e1ab),
    ('left-pot-held', 0x1000, 0x80a24f),
    ('left-carry-down', 0x1000, 0x80a255),
    ('left-carry-north', 0x1000, 0x80a255),
    ('left-carry-east', 0x1000, 0x80a255),
]


def inspect(rom, capture, export, meta, witnesses=WITNESSES):
    reports = []
    for label, slot, art_id in witnesses:
        w, v, cg, o = [(capture/f'{label}.{suffix}').read_bytes() for suffix in ('wram', 'vram', 'cgram', 'oam')]
        obj = (capture/f'{label}.obj').read_bytes()
        require(obj == bytes([2,0]), 'qualified OBJSEL/first sprite')
        entities, _ = native_entities(w)
        entity = next(e for e in entities if e['slot'] == slot)
        require(not entity['flags'][0] & 0x8000, 'selected actor is visible')
        art = next(a for a in meta['art'] if a['id'] == art_id)
        seq = next(s for s in art['lists'] if s['selector'] == entity['selector'])
        d = export/f'{art_id:06x}'
        native_pointer = entity['composition']
        bank = native_pointer >> 16
        at = native_pointer & (65535 if bank == 0x7e else 0x3fffff)
        memory = w if bank == 0x7e else rom
        native_frame = memory[at-4:at+13+memory[at+12]*7]
        # Direct pot frames receive a draw-time palette override, not WRAM relocation.
        field = 'composition_sha256' if bank == 0x7e else 'source_sha256'
        matches = [f for f in seq['frames'] if f[field] == sha(native_frame)]
        require(matches, 'native exact source list membership')
        frame = matches[0]
        if bank == 0x7e:
            require(native_pointer-entity['animation_base'] == frame['key']['offset'], 'native composition packet offset')
        else:
            require(native_pointer == frame['key']['direct'], 'native direct source pointer')
        c = (d/f"{seq['selector']}-{frame['index']}-composition").read_bytes()
        palette = (d/'palette').read_bytes()
        palette_policy = 'source-natural'
        if label == '21-opening' and art_id == 0x83f8c0:
            # Retain the mismatch as an explicit native effect witness, not a
            # source palette replacement or a production initialization.
            palette = bytes([255, 127])*16
            palette_policy = 'native-white-effect-source-program-unqualified'
        try:
            result = native_piece(c, (d/'tiles').read_bytes(), palette, art['palette_base'], entity, w, v, cg, o)
        except ValueError as error:
            raise ValueError(f'{label}/{art_id:06x}: {error}') from error
        reports.append({'label': label, 'art': art_id, 'slot': slot, 'map': u(w, 0x47e),
                        'palette_policy': palette_policy, 'selector': seq['selector'], 'record_indices': [f['index'] for f in matches],
                        'position': entity['position'], 'piece': result,
                        'hashes': {name: sha(data) for name, data in [('wram', w), ('vram', v), ('cgram', cg), ('oam', o), ('obj', obj)]}})
    require(len(reports) == len(witnesses) and reports, 'complete native witnesses')
    return reports


PHASE_WITNESSES = [
    ('C-direct-ready', 'c-direct'), ('cellar-sequence-002', 'c-color-math'),
    ('cellar-sequence-005', 'c-reaction-speaker'), ('cellar-sequence-006', 'c-reaction-right'),
    ('cellar-sequence-007', 'c-reaction-left'), ('cellar-sequence-008', 'c-reaction-final'),
    ('door-passable', 'c-departed'), ('landed-E', 'cellar-e'), ('landed-20', 'cellar-20'),
    ('21-box-contact-rest', 'box-contact'), ('21-opening', 'box-opening'),
    ('tutorial-002', 'box-opening-cue'), ('tutorial-014', 'tour-41-0'),
    ('tutorial-016', 'tour-41-1'), ('tutorial-020', 'tour-41-2'),
    ('tutorial-022', 'tour-41-3'), ('tutorial-024', 'tour-41-4'),
    ('tutorial-026', 'tour-41-5'), ('tutorial-028', 'tour-41-6'),
    ('tutorial-030', 'tour-41-7'), ('tutorial-031', 'tour-44'),
    ('tutorial-040', 'tour-42'), ('tutorial-044', 'tour-43'),
    ('pandora-tour-control', 'tour-control'),
]


def checkpoint_hashes(capture,label):
    result={}
    for suffix,size in [('wram',131072),('vram',65536),('cgram',512),('oam',544),('obj',2)]:
        data=(capture/f'{label}.{suffix}').read_bytes()
        require(len(data)==size,'phase hardware extent '+suffix)
        if suffix=='obj':require(data==bytes([2,0]),'phase OBJSEL/first sprite')
        result[suffix]=sha(data)
    return result


def phase_witnesses(rom, capture, export, meta):
    reports = []
    for label, phase_id in PHASE_WITNESSES:
        phase = next(p for p in meta['phases'] if p['id'] == phase_id)
        w = (capture/f'{label}.wram').read_bytes()
        entities, _ = native_entities(w)
        require(u(w, 0x47e) == phase['map'], 'phase map')
        if phase['map'] == 12:
            slots = {0x838c0a: 0x1040, 0x838c14: 0x1080, 0x838c1e: 0x10c0, 0x838c28: 0x1100}
        elif phase['map'] == 33:
            slots = {0x83928f: 0x1100, 0x83927b: 0x1080}
        else:
            slots = {a['source']: 0x1080 if a['art'] == 0x89d9f9 else 0x1040 for a in phase['actors']}
        visible = {e['slot']: e for e in entities if not e['flags'][0] & 0x8000}
        if phase['map'] in (0xe,0x20):
            require(not any(e['animation_base'] >> 16 == 0x7e for e in visible.values()), 'empty cellar visible resident projection')
        # These are evidence slot-to-source associations, NOT portable initialization.
        require(set(slots.values()) & set(visible) == {slots[a['source']] for a in phase['actors']}, 'explicit phase membership')
        pieces = []
        for actor in phase['actors']:
            entity = visible[slots[actor['source']]]
            require(entity['position'] == actor['position'], 'source phase position')
            require(entity['selector'] == actor['selector'], 'source phase selector')
            require(bool(entity['flags'][2] & 0x4000) == actor['hflip'], 'phase mirror')
            require(actor['priority_override'] is None or entity['flags'][2] >> 12 & 3 == actor['priority_override'], 'source phase priority override')
            piece = inspect(rom, capture, export, meta, [(label, entity['slot'], actor['art'])])[0]
            pieces.append((actor, piece))
        for a, pa in pieces:
            for b, pb in pieces:
                if a['position'][1] == b['position'][1] and a['tie_rank'] > b['tie_rank']:
                    require(pa['piece']['first_oam'] < pb['piece']['first_oam'], 'qualified equal-Y painter/OAM tie')
        reports.append({'label': label, 'phase': phase_id, 'hashes': checkpoint_hashes(capture,label),
                        'actors': [{'source': a['source'], 'record_indices': p['record_indices'],
                                    'first_oam': p['piece']['first_oam'], 'opaque': p['piece']['opaque']} for a, p in pieces]})
    return reports


def main():
    require(len(sys.argv) in (4, 5), 'check.py ROM local/EXPORT CAPTURE [--record]')
    rom, export, capture = Path(sys.argv[1]).read_bytes(), Path(sys.argv[2]), Path(sys.argv[3])
    meta = exported(rom, export)
    report = inspect(rom, capture, export, meta)
    record = {'rom_sha256': meta['rom_sha256'],
              'export_metadata_sha256': sha(json.dumps(meta, sort_keys=True, separators=(',', ':')).encode()),
              'art': [{'source': a['id'], 'lists': [[s['selector'], len(s['frames'])] for s in a['lists']]} for a in meta['art']],
              'phases': meta['phases'], 'motions': meta['motions'],
              'native': report, 'phase_witnesses': phase_witnesses(rom,capture,export,meta), 'evidence': 'original journey, hardware surfaces only; no pixels'}
    reference = HERE/'reference.json'
    if len(sys.argv) == 5:
        require(sys.argv[4] == '--record', 'unknown option')
        reference.write_text(json.dumps(record, indent=2)+'\n')
    else:
        require(record == json.loads(reference.read_text()), 'strict source/native metadata reference')
    print(f'Qualified {len(meta["art"])} source resources and {len(report)} selected native OAM pieces; no native RGB claim.')

if __name__ == '__main__':
    main()
