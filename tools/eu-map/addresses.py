"""Collects the ROM addresses the slice names in its Rust sources.

Runtime addresses ($80-$FF banks) and normalized offsets (below $40:0000)
both reduce to a normalized offset. Masks, image sizes and RAM addresses
are left out.
"""
import os
import re

SCOPE = ('crates/assets/src', 'crates/crysta-runtime/src', 'crates/crysta-app/src',
         'crates/room-core/src')
HEX = re.compile(r'\b0x([0-9A-Fa-f_]{5,9})\b')
# Masks, sizes and bank-OR constants: never an address.
NOT_ADDRESSES = {0x3F_FFFF, 0xFF_0000, 0x40_0000, 0x80_0000, 0xFF_FFFF}
# Round sizes, unless written as a bank (`0x03_0000`).
SIZES = {0x1_0000, 0x2_0000, 0x3_0000, 0x4_0000, 0x1_FFFF}
BANK_FORM = re.compile(r'0x[0-9A-Fa-f]{2}_0000')
SIZE_CONTEXT = re.compile(r'vec!\[[^\]]*;\s*$|\.len\(\)\s*!=\s*$|for len in')


def classify(value):
    """Normalized offset for a constant, or None when it is not a ROM address."""
    if value in NOT_ADDRESSES or value > 0xFF_FFFF:
        return None
    bank = value >> 16
    if bank in (0x7E, 0x7F) or 0x40 <= bank < 0x80:
        return None
    if bank >= 0x80:
        return value & 0x3F_FFFF
    return value


def test_start(name, lines):
    """First line number that belongs to tests (whole file for test files)."""
    if name == 'tests.rs' or name.endswith('_tests.rs') or name == 'fixtures.rs':
        return 0
    for number, line in enumerate(lines, 1):
        following = lines[number].strip() if number < len(lines) else ''
        if line.strip() == '#[cfg(test)]' and following.endswith('{'):
            return number
    return len(lines) + 1


def scan(root):
    """{normalized: [(written, file:line, in_test), ...]} over the slice sources."""
    found = {}
    for base in SCOPE:
        for folder, _, files in os.walk(os.path.join(root, base)):
            for name in sorted(files):
                if not name.endswith('.rs'):
                    continue
                path = os.path.join(folder, name)
                with open(path, encoding='utf-8') as handle:
                    lines = handle.readlines()
                test_from = test_start(name, lines)
                for number, line in enumerate(lines, 1):
                    if True:
                        code = line.split('//')[0]
                        for match in HEX.finditer(code):
                            digits = match.group(1).replace('_', '')
                            if not 5 <= len(digits) <= 6:
                                continue
                            if SIZE_CONTEXT.search(code[:match.start()]):
                                continue
                            value = int(digits, 16)
                            if value in SIZES and not BANK_FORM.fullmatch(match.group(0)):
                                continue
                            offset = classify(value)
                            if offset is None:
                                continue
                            where = f'{os.path.relpath(path, root)}:{number}'
                            found.setdefault(offset, []).append(
                                (match.group(0), where, number >= test_from))
    return found


if __name__ == '__main__':
    import sys
    root = sys.argv[1] if len(sys.argv) > 1 else '.'
    for offset, uses in sorted(scan(root).items()):
        print(f'{offset:06X}\t{len(uses)}\t{uses[0][0]}\t{uses[0][1]}')
