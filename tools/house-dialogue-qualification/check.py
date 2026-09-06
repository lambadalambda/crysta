"""Independent bounded text/font reconstruction; all copyrighted output stays local."""
from hashlib import sha256
from pathlib import Path
import json
import sys

ROM_SHA = 'f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'
TEXTS = (0x888FDA, 0x888FF0, 0x88905A, 0x8890D9, 0x889156, 0x88918C, 0x8891D6)
DISPATCH = {0xC0:0x95BD, 0xC1:0x960D, 0xC4:0x98B7, 0xC5:0x98CF,
            0xC6:0x98FE, 0xC7:0x9930, 0xC8:0x9956, 0xCA:0x99F4,
            0xCF:0x9BB3, 0xD0:0x9C68, 0xD1:0x9C71, 0xD2:0x9C7A,
            0xD3:0x9C9E, 0xD4:0x9D13, 0xD5:0x9D94, 0xDC:0x969F}

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
    pixels = bytearray([3] * (224*48))
    x = y = 0
    kana = False
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
            require(x+16 <= 224 and y+16 <= 48, 'page geometry')
            placements.append(dict(text_source=at, font_source=address, position=[x,y]))
            decoded = glyph(rom[address & 0x3FFFFF:(address & 0x3FFFFF)+64])
            for row in range(16):
                for column in range(16):
                    pixels[(y+row)*224+x+column] = decoded[row*16+column]
            x += 12
        elif command in (0xC0,0xC1):
            require(not placements, 'unacknowledged clear')
            x = y = 0
            kana = False
        elif command == 0xC4:
            require(take() == 1, 'unsupported transformed font')
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
        elif command == 0xD2:
            index = take()
            require(index in (0,1,7) and len(stack) < 8, 'unsupported call')
            stack.append(pc)
            target = u(rom, 0x12C447+index*2)
            pc = target if index == 0 and target == 0x610 else 0x920000+target
        elif command == 0xD4 and stack:
            pc = stack.pop()
        elif command in (0xD3,0xD4,0xD5):
            require(placements and not stack and len(pages) < 16, 'page boundary')
            pages.append(dict(text_source=source, index=len(pages), glyphs=placements,
                              boundary_source=at, acknowledgement={0xD3:'end',0xD4:'none',0xD5:'next'}[command],
                              pixels=bytes(pixels)))
            if command != 0xD5:
                return pages
            placements = []
            pixels = bytearray([3] * (224*48))
            x = y = 0
            kana = False
        else:
            raise ValueError(f'unsupported command {command:02x} at {at:06x}')
    raise ValueError('instruction budget')

def choices(rom):
    catalogs = []
    for catalog in range(2):
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

def check(rom, root):
    require(len(rom) == 0x400000 and sha256(rom).hexdigest() == ROM_SHA, 'JP ROM identity')
    for command, target in DISPATCH.items():
        require(u(rom, 0x59198+(command-0xC0)*2) == target, 'conversation dispatch, not overlay dispatch')
    meta = json.loads((root/'export.json').read_text())
    require(meta['rom_sha256'] == ROM_SHA, 'export identity')
    require(meta['choices'] == choices(rom), 'choice catalogs')
    expected = [page for source in TEXTS for page in reconstruct(rom, source)]
    require(len(expected) == len(meta['pages']), 'page count')
    summaries = []
    for actual, page in zip(meta['pages'], expected):
        for key in ('text_source','index','glyphs','boundary_source','acknowledgement'):
            require(actual[key] == page[key], f'page {key}')
        require((actual['width'],actual['height']) == (224,48), 'page dimensions')
        filename = f"{page['text_source']:06x}-{page['index']}.indexed"
        require(actual['file'] == filename, 'local page filename')
        require((root/filename).read_bytes() == page['pixels'], 'independent ROM page raster')
        digest = sha256(page['pixels']).hexdigest()
        require(actual['sha256'] == digest, 'export bitmap hash')
        summaries.append(dict(text_source=page['text_source'],index=page['index'],
                              boundary_source=page['boundary_source'],acknowledgement=page['acknowledgement'],
                              glyph_count=len(page['glyphs']),sha256=digest))
    reference = json.loads(Path(__file__).with_name('reference.json').read_text())
    require(summaries == reference['pages'], 'qualified page reference')
    print(json.dumps(dict(rom_sha256=ROM_SHA,pages=summaries),indent=2))

if __name__ == '__main__':
    check(Path(sys.argv[1]).read_bytes(), Path(sys.argv[2]))
