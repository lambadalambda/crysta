#!/usr/bin/env python3
"""A linear 65C816 disassembler for reading one handler at a time.

Usage: disasm65.py ROM START [END] [--m8] [--x8]

START and END are runtime addresses such as 80:9AC7 or normalized offsets
such as 0x9AC7. Register widths default to sixteen bits, which is how the
COP dispatcher enters its handlers; SEP and REP are tracked from there.
This is linear: it does not follow branches, so a data byte in the path
reads as an instruction. It is for looking, not for building.
"""
import sys

# (mnemonic, mode) by opcode. Modes name the operand encoding.
MODES = {
    'imp': 0, 'acc': 0, 'imm8': 1, 'immM': 1, 'immX': 1, 'dp': 1, 'dpx': 1,
    'dpy': 1, '(dp)': 1, '(dp,x)': 1, '(dp),y': 1, '[dp]': 1, '[dp],y': 1,
    'abs': 2, 'absx': 2, 'absy': 2, '(abs)': 2, '(abs,x)': 2, '[abs]': 2,
    'long': 3, 'longx': 3, 'sr': 1, '(sr),y': 1, 'rel': 1, 'rel16': 2,
    'blk': 2,
}

_ROWS = [
    "BRK imm8;ORA (dp,x);COP imm8;ORA sr;TSB dp;ORA dp;ASL dp;ORA [dp];PHP imp;ORA immM;ASL acc;PHD imp;TSB abs;ORA abs;ASL abs;ORA long",
    "BPL rel;ORA (dp),y;ORA (dp);ORA (sr),y;TRB dp;ORA dpx;ASL dpx;ORA [dp],y;CLC imp;ORA absy;INC acc;TCS imp;TRB abs;ORA absx;ASL absx;ORA longx",
    "JSR abs;AND (dp,x);JSL long;AND sr;BIT dp;AND dp;ROL dp;AND [dp];PLP imp;AND immM;ROL acc;PLD imp;BIT abs;AND abs;ROL abs;AND long",
    "BMI rel;AND (dp),y;AND (dp);AND (sr),y;BIT dpx;AND dpx;ROL dpx;AND [dp],y;SEC imp;AND absy;DEC acc;TSC imp;BIT absx;AND absx;ROL absx;AND longx",
    "RTI imp;EOR (dp,x);WDM imm8;EOR sr;MVP blk;EOR dp;LSR dp;EOR [dp];PHA imp;EOR immM;LSR acc;PHK imp;JMP abs;EOR abs;LSR abs;EOR long",
    "BVC rel;EOR (dp),y;EOR (dp);EOR (sr),y;MVN blk;EOR dpx;LSR dpx;EOR [dp],y;CLI imp;EOR absy;PHY imp;TCD imp;JML long;EOR absx;LSR absx;EOR longx",
    "RTS imp;ADC (dp,x);PER rel16;ADC sr;STZ dp;ADC dp;ROR dp;ADC [dp];PLA imp;ADC immM;ROR acc;RTL imp;JMP (abs);ADC abs;ROR abs;ADC long",
    "BVS rel;ADC (dp),y;ADC (dp);ADC (sr),y;STZ dpx;ADC dpx;ROR dpx;ADC [dp],y;SEI imp;ADC absy;PLY imp;TDC imp;JMP (abs,x);ADC absx;ROR absx;ADC longx",
    "BRA rel;STA (dp,x);BRL rel16;STA sr;STY dp;STA dp;STX dp;STA [dp];DEY imp;BIT immM;TXA imp;PHB imp;STY abs;STA abs;STX abs;STA long",
    "BCC rel;STA (dp),y;STA (dp);STA (sr),y;STY dpx;STA dpx;STX dpy;STA [dp],y;TYA imp;STA absy;TXS imp;TXY imp;STZ abs;STA absx;STZ absx;STA longx",
    "LDY immX;LDA (dp,x);LDX immX;LDA sr;LDY dp;LDA dp;LDX dp;LDA [dp];TAY imp;LDA immM;TAX imp;PLB imp;LDY abs;LDA abs;LDX abs;LDA long",
    "BCS rel;LDA (dp),y;LDA (dp);LDA (sr),y;LDY dpx;LDA dpx;LDX dpy;LDA [dp],y;CLV imp;LDA absy;TSX imp;TYX imp;LDY absx;LDA absx;LDX absy;LDA longx",
    "CPY immX;CMP (dp,x);REP imm8;CMP sr;CPY dp;CMP dp;DEC dp;CMP [dp];INY imp;CMP immM;DEX imp;WAI imp;CPY abs;CMP abs;DEC abs;CMP long",
    "BNE rel;CMP (dp),y;CMP (dp);CMP (sr),y;PEI (dp);CMP dpx;DEC dpx;CMP [dp],y;CLD imp;CMP absy;PHX imp;STP imp;JML [abs];CMP absx;DEC absx;CMP longx",
    "CPX immX;SBC (dp,x);SEP imm8;SBC sr;CPX dp;SBC dp;INC dp;SBC [dp];INX imp;SBC immM;NOP imp;XBA imp;CPX abs;SBC abs;INC abs;SBC long",
    "BEQ rel;SBC (dp),y;SBC (dp);SBC (sr),y;PEA abs;SBC dpx;INC dpx;SBC [dp],y;SED imp;SBC absy;PLX imp;XCE imp;JSR (abs,x);SBC absx;INC absx;SBC longx",
]
TABLE = [tuple(entry.split(' ')) for row in _ROWS for entry in row.split(';')]
# Operand text by mode, with `#` standing for the hex operand.
TEMPLATES = {
    'dp': '#', 'dpx': '#,X', 'dpy': '#,Y', '(dp)': '(#)', '(dp,x)': '(#,X)',
    '(dp),y': '(#),Y', '[dp]': '[#]', '[dp],y': '[#],Y', 'abs': '#',
    'absx': '#,X', 'absy': '#,Y', '(abs)': '(#)', '(abs,x)': '(#,X)',
    '[abs]': '[#]',
}
assert len(TABLE) == 256


def operand_length(mode, m16, x16):
    length = MODES[mode]
    if mode == 'immM' and m16:
        length = 2
    if mode == 'immX' and x16:
        length = 2
    return length


def runtime(offset):
    """Runtime address for a normalized HiROM offset, in the repo's convention."""
    bank, low = offset >> 16, offset & 0xFFFF
    bank = (0x80 if low >= 0x8000 else 0xC0) | bank
    return f'${bank:02X}:{low:04X}'


def normalized(text):
    """Parses 80:9AC7, $80:9AC7 or 0x9AC7 into a normalized offset."""
    text = text.lstrip('$')
    if ':' in text:
        bank, low = text.split(':')
        return ((int(bank, 16) & 0x3F) << 16) | int(low, 16)
    return int(text, 0)


def decode(image, at, m16=True, x16=True):
    """One instruction: (length, text, m16, x16) with widths after it."""
    opcode = image[at]
    mnemonic, mode = TABLE[opcode]
    length = 1 + operand_length(mode, m16, x16)
    raw = image[at:at + length]
    if len(raw) < length:
        return None
    value = int.from_bytes(raw[1:], 'little') if length > 1 else 0
    if mode in ('imp', 'acc'):
        text = mnemonic
    elif mode in ('imm8', 'immM', 'immX'):
        text = f'{mnemonic} #${value:0{2 * (length - 1)}X}'
    elif mode == 'rel':
        target = (at + 2 + (value - 256 if value >= 128 else value)) & 0xFFFFFF
        text = f'{mnemonic} {runtime(target)}'
    elif mode == 'rel16':
        target = (at + 3 + (value - 65536 if value >= 32768 else value)) & 0xFFFFFF
        text = f'{mnemonic} {runtime(target)}'
    elif mode == 'blk':
        text = f'{mnemonic} ${raw[2]:02X},${raw[1]:02X}'
    elif mode == 'long':
        text = f'{mnemonic} ${value >> 16:02X}:{value & 0xFFFF:04X}'
    elif mode == 'longx':
        text = f'{mnemonic} ${value >> 16:02X}:{value & 0xFFFF:04X},X'
    elif mode == 'sr':
        text = f'{mnemonic} ${value:02X},S'
    elif mode == '(sr),y':
        text = f'{mnemonic} (${value:02X},S),Y'
    else:
        hexed = f'${value:0{2 * (length - 1)}X}'
        text = f'{mnemonic} ' + TEMPLATES[mode].replace('#', hexed)
    if opcode == 0xE2:
        m16, x16 = m16 and not value & 0x20, x16 and not value & 0x10
    elif opcode == 0xC2:
        m16, x16 = m16 or bool(value & 0x20), x16 or bool(value & 0x10)
    return length, text, m16, x16


def listing(image, start, end, m16=True, x16=True):
    """Lines from START up to END, or until the image ends."""
    lines = []
    at = start
    while at < end:
        decoded = decode(image, at, m16, x16)
        if decoded is None:
            break
        length, text, m16, x16 = decoded
        raw = ' '.join(f'{b:02X}' for b in image[at:at + length])
        lines.append(f'{runtime(at)}  {raw:<12} {text}')
        at += length
    return lines


def main(argv):
    flags = [a for a in argv if a.startswith('--')]
    args = [a for a in argv if not a.startswith('--')]
    if len(args) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    with open(args[0], 'rb') as handle:
        image = handle.read()
    start = normalized(args[1])
    end = normalized(args[2]) if len(args) > 2 else start + 0x80
    for line in listing(image, start, end, '--m8' not in flags, '--x8' not in flags):
        print(line)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
