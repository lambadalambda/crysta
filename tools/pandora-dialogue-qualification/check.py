"""Independent bounded text/font reconstruction; all copyrighted output stays local."""
from hashlib import sha256
from pathlib import Path
import json
import sys

ROM_SHA = 'f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'
# Direct execution order, not numerical source order. D720 is invoked four times.
INVOCATION_SITES = (
    0x88B673, 0x88B69C,
    0x889AE9, 0x889AFB, 0x889B11, 0x889DA9,
    0x88ABBC, 0x889B9F, 0x889BC2, 0x889BEF, 0x88A241, 0x88A3D9, 0x889C10,
    0x88AD95, 0x88ADB7, 0x88AE77, 0x88AE85, 0x88AE93, 0x88AEA1,
    0x89D3E0, 0x89D3F4, 0x89D408, 0x89D41C, 0x89D430, 0x89D444, 0x89D458,
    0x89D470, 0x89D4A4, 0x89D4AE, 0x89D4C9, 0x89D4D3, 0x89D4EE, 0x89D4F8, 0x89D48B,
)
RETRY_SITE = 0x88B67F
REFUSAL_SITE = 0x88B691  # map13 result0/2, not a direct-route invocation
CHOICE_SITES = {0x88B6C7:0x88B679, 0x889EFC:0x889B17, 0x88B722:0x88B685}
DISPATCH = {0xC0:0x95BD, 0xC1:0x960D, 0xC2:0x982D, 0xC4:0x98B7, 0xC5:0x98CF,
            0xC6:0x98FE, 0xC7:0x9930, 0xC8:0x9956, 0xCA:0x99F4,
            0xCC:0x9A13, 0xCF:0x9BB3, 0xD0:0x9C68, 0xD1:0x9C71, 0xD2:0x9C7A,
            0xD3:0x9C9E, 0xD4:0x9D13, 0xD5:0x9D94, 0xDA:0x964D, 0xDC:0x969F,
            0xE3:0x96BE, 0xE4:0x9725}

def require(value, message):
    if not value:
        raise ValueError(message)

def u(data, at):
    require(at >= 0 and at + 2 <= len(data), 'short word')
    return int.from_bytes(data[at:at+2], 'little')

def glyph(data):
    require(len(data) == 64, 'font extent')
    tiles = []
    for start in range(0, 64, 16):
        tiles.append([[sum(((data[start+y*2+plane] >> (7-x)) & 1) << plane
                           for plane in range(2)) for x in range(8)] for y in range(8)])
    return [value for y in range(16) for tile in (y//8*2, y//8*2+1)
            for value in tiles[tile][y%8]]

def default_assignments(rom):
    assignments = {}
    for i, destination in enumerate((0x634,0x636,0x638,0x63A,0x63E,0x63C)):
        start = 0x5BEF7+i*6
        require(rom[start] == 0xA9 and rom[start+3] == 0x8D and u(rom,start+4) == destination,
                'default controller immediate writes')
        assignments[destination] = u(rom,start+1)
    return assignments

def reconstruct(rom, source):
    pc = source
    name = []
    for index in range(6):
        at = 0x78C99 + index*5
        require(rom[at] == 0xA9 and rom[at+2] == 0x8D and u(rom, at+3) == 0x610+index,
                'default name immediate writes')
        name.append(rom[at+1])
    stack = []
    pages = []
    placements = []
    width, height = 224, 48
    pixels = bytearray([3] * (width*height))
    x = y = 0
    kana = False
    transformed = False
    def take():
        nonlocal pc
        if 0x610 <= pc < 0x616:
            result = name[pc-0x610]
        else:
            require(0x80 <= pc >> 16 <= 0xBF and pc & 65535 >= 0x8000, 'ROM address')
            result = rom[pc & 0x3FFFFF]
        pc += 1
        return result
    for _ in range(4096):
        at = pc
        command = take()
        if command < 0xC0:
            if command < 0x80:
                address = 0xB48000 + command*64 + (0x2000 if kana else 0)
            else:
                code = (command-0x80)*256 + take()
                bank, index = divmod(code, 512)
                address = (0xB5+bank)*65536 + 0x8000 + index*64
            require(x+16 <= width and y+16 <= height, 'page geometry')
            placements.append(dict(text_source=at, font_source=address, position=[x,y]))
            decoded = glyph(rom[address & 0x3FFFFF:(address & 0x3FFFFF)+64])
            if transformed:
                decoded = [0 if value == 3 else value for value in decoded]
            for row in range(16):
                for column in range(16):
                    pixels[(y+row)*width+x+column] = decoded[row*16+column]
            x += 12
        elif command in (0xC0,0xC1,0xC2,0xDA):
            require(not placements, 'unacknowledged clear')
            if command == 0xC2:
                arguments = [take() for _ in range(4)]
                require(arguments == [6,6,24,6], 'unsupported custom window')
                width, height = arguments[2]*8, (arguments[3]//2)*16
            else:
                width, height = 224, 48
            x = y = 0
            kana = False
            pixels = bytearray([0 if transformed else 3] * (width*height))
        elif command == 0xC4:
            mode = take()
            require(mode in (0,1), 'unsupported transformed font')
            transformed = mode == 0
        elif command in (0xC5,0xC7,0xC8):
            take()
        elif command in (0xC6,0xDC):
            if command == 0xC6:
                require(take() in (0,4), 'unsupported palette')
            x = (x+7)//8*8
        elif command == 0xCA:
            require(take() == 5, 'unsupported RAM write')
            take(); take()
        elif command == 0xCF:
            x = 0
            y += 16
            kana = False
        elif command in (0xD0,0xD1):
            kana = command == 0xD0
        elif command in (0xD2,0xE4):
            index = take()
            allowed = (0,3,7) if command == 0xD2 else (6,0x25)
            require(index in allowed and len(stack) < 8, 'unsupported call')
            stack.append(pc)
            target = u(rom, (0x12C447 if command == 0xD2 else 0x12C5E7)+index*2)
            pc = target if command == 0xD2 and index == 0 and target == 0x610 else 0x920000+target
        elif command == 0xE3:
            mask = take() | take()<<8
            require(mask == 0x40 and len(stack) < 8, 'unsupported controller icon')
            assignments = default_assignments(rom)
            selected = next((i for i in range(6) if assignments[0x634+i*2] == mask), None)
            require(selected is not None, 'default controller assignment')
            stack.append(pc)
            pc = 0x850000 | u(rom,0x5970D+selected*2)
        elif command == 0xCC:
            target = take() | take()<<8 | take()<<16
            require(target in (0x88B93C,0x8AF898,0x88A195,0x8AF891) and len(stack) < 8, 'unsupported long call')
            stack.append(pc)
            pc = target
        elif command == 0xD4 and stack:
            pc = stack.pop()
        elif command in (0xD3,0xD4,0xD5):
            require(placements and not stack and len(pages) < 15, 'page boundary')
            pages.append(dict(text_source=source, index=len(pages), page_id=(source << 4) | len(pages), glyphs=placements,
                              boundary_source=at, acknowledgement={0xD3:'end',0xD4:'none',0xD5:'next'}[command],
                              background_index=0 if transformed else 3, width=width, height=height, pixels=bytes(pixels)))
            if command != 0xD5:
                return pages
            placements = []
            pixels = bytearray([0 if transformed else 3] * (width*height))
            x = y = 0
            kana = False
        else:
            raise ValueError(f'unsupported command {command:02x} at {at:06x}')
    raise ValueError('instruction budget')

def choices(rom):
    catalogs = []
    for catalog in (1,):
        base = u(rom, 0x12C259+catalog*2)
        options = []
        for index in range(2):
            at = 0x120000+base+index*10
            coordinate = u(rom, at)
            offset = (coordinate & 127)*64 + (coordinate >> 8)
            if not coordinate & 128:
                offset -= 0x504
            require(offset >= 0 and offset % 2 == 0, 'choice position')
            row, column = divmod(offset,64)
            require(row < 6 and column < 56, 'choice geometry')
            links = []
            for word in range(1,5):
                target = u(rom,at+word*2)
                require(target in (0,base,base+10), 'choice link')
                links.append(None if target == 0 else 1+(target-base)//10)
            options.append(dict(source=at|0x800000,result=index+1,position=[column*4,row*8],neighbors=links))
        catalogs.append(dict(catalog=catalog,options=options))
    return catalogs

def invocation(rom, site):
    at = site & 0x3FFFFF
    require(rom[at:at+2] == bytes([2, 0x1B]), 'COP1B request provenance')
    return dict(site=site, source=(site & 0xFF0000) | u(rom, at+2))


def requests(rom):
    records = []
    for site in (*INVOCATION_SITES, RETRY_SITE, REFUSAL_SITE):
        source = invocation(rom, site)['source']
        if source not in [r['source'] for r in records]:
            pages = reconstruct(rom, source)
            catalog = None
            if source in CHOICE_SITES:
                at = CHOICE_SITES[source] & 0x3FFFFF
                require(rom[at:at+3] == bytes([2,0x1A,1]), 'COP1A choice catalog provenance')
                require(pages[-1]['acknowledgement'] == 'none', 'retained choice tail')
                catalog = 1
            records.append(dict(source=source, choice_catalog=catalog,
                                page_ids=[p['page_id'] for p in pages]))
    require(records, 'nonempty requests')
    return records


def summary(page):
    return dict(text_source=page['text_source'], index=page['index'], page_id=page['page_id'],
                boundary_source=page['boundary_source'], acknowledgement=page['acknowledgement'],
                width=page['width'], height=page['height'], background_index=page['background_index'], glyph_count=len(page['glyphs']), sha256=sha256(page['pixels']).hexdigest())


def check(rom, root):
    require(len(rom) == 0x400000 and sha256(rom).hexdigest() == ROM_SHA, 'JP ROM identity')
    for command, target in DISPATCH.items():
        require(u(rom, 0x59198+(command-0xC0)*2) == target, 'conversation dispatch, not overlay dispatch')
    meta = json.loads((root/'export.json').read_text())
    require(meta['rom_sha256'] == ROM_SHA, 'export identity')
    require(meta['choices'] == choices(rom), 'choice catalog 1')
    invocations = [invocation(rom, site) for site in INVOCATION_SITES]
    require(meta['invocations'] == invocations, 'ordered repeated invocations')
    records = requests(rom)
    require(meta['requests'] == records, 'ordered request records')
    expected = [page for request in records for page in reconstruct(rom, request['source'])]
    require(len(expected) == len(meta['pages']), 'page count')
    for actual, page in zip(meta['pages'], expected):
        for key in ('text_source','index','page_id','background_index','glyphs','boundary_source','acknowledgement'):
            require(actual[key] == page[key], f'page {key}')
        require((actual['width'],actual['height']) == (page['width'],page['height']), 'page dimensions')
        filename = f"{page['text_source']:06x}-{page['index']}.indexed"
        require(actual['file'] == filename, 'local page filename')
        require((root/filename).read_bytes() == page['pixels'], 'independent ROM page raster')
        require(actual['sha256'] == sha256(page['pixels']).hexdigest(), 'export bitmap hash')
    report = dict(rom_sha256=ROM_SHA, invocations=invocations, requests=records, pages=[summary(p) for p in expected])
    reference = json.loads(Path(__file__).with_name('reference.json').read_text())
    require(report == reference, 'qualified source reference')
    return report


if __name__ == '__main__':
    require(len(sys.argv) == 3, 'usage: check.py ROM EXPORT')
    print(json.dumps(check(Path(sys.argv[1]).read_bytes(), Path(sys.argv[2])), indent=2))
