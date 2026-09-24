#!/usr/bin/env python3
"""Maps every ROM address the slice uses from the Japanese ROM to the European one.

Usage: map.py JP_ROM EU_ROM [--repo=DIR] [--out=FILE] [--layout]

--layout prints, per 32 KiB of the Japanese image, how much of it was
found verbatim in the European one and at which shifts.

Both images may carry a 512-byte copier header; it is stripped. Output is a
TSV: jp_address eu_address kind method confidence note. Addresses are
runtime addresses in the repo's convention ($80-$BF for the upper half of a
bank, $C0-$FF for the lower half).

Methods, tried in order, each recorded in the `method` column:
  exact      the address lies in a byte run identical in both ROMs;
  aligned    it lies between two identical anchors whose gap has the same
             length in both ROMs (operands and pointers rewritten in place);
  shape      the instruction stream from the address, operands masked,
             occurs once near the aligned estimate in the European ROM;
  pointer    references to the address (24-bit, or 16-bit from its bank)
             sit at aligned places, and the European values there agree.
  hand       overrides.tsv sets it, after the matching, with a reason.
"""
import bisect
import difflib
import json
import os
import sys
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', 'disasm65'))
import addresses  # noqa: E402
import lz  # noqa: E402
import disasm65  # noqa: E402

SEED = 12          # anchor seed length, bytes
LOCAL_BLOCK = 4    # shortest local anchor inside a gap
SHAPE_LENGTH = 10  # instructions compared by shape
SHAPE_WINDOW = 0x3000

with open(os.path.join(HERE, 'cop_lengths.json'), encoding='utf-8') as _handle:
    COP_LENGTHS = {int(k): v for k, v in json.load(_handle).items()}


def load(path):
    with open(path, 'rb') as handle:
        data = handle.read()
    return data[512:] if len(data) % 0x8000 == 512 else data


def runtime(offset):
    return disasm65.runtime(offset).replace('$', '')


# --- anchors -----------------------------------------------------------------

def anchor_runs(jp, eu):
    """Maximal identical runs (jp, eu, length), seeded by EU-rare 12-grams."""
    index = defaultdict(list)
    for at in range(len(eu) - SEED):
        index[eu[at:at + SEED]].append(at)
    runs, at = [], 0
    while at < len(jp) - SEED:
        window = jp[at:at + SEED]
        spots = index.get(window)
        if not spots or len(spots) > 4 or len(set(window)) < 3:
            at += 1
            continue
        best = None
        for spot in spots:
            a, b = at, spot
            while a > 0 and b > 0 and jp[a - 1] == eu[b - 1]:
                a, b = a - 1, b - 1
            n = 0
            while a + n < len(jp) and b + n < len(eu) and jp[a + n] == eu[b + n]:
                n += 1
            if best is None or n > best[2]:
                best = (a, b, n)
        runs.append(best)
        at = max(at + 1, best[0] + best[2])
    return consistent(sorted(set(runs)))


def consistent(runs):
    """Drops short runs whose shift disagrees with both neighbours (chance hits)."""
    kept = []
    for i, (a, b, n) in enumerate(runs):
        if n >= 24:
            kept.append((a, b, n))
            continue
        shift = b - a
        near = [r[1] - r[0] for r in runs[max(0, i - 2):i] + runs[i + 1:i + 3]]
        if any(abs(shift - other) < 0x4000 for other in near):
            kept.append((a, b, n))
    return kept


class Aligner:
    """Positional JP->EU mapping from identical anchors."""

    def __init__(self, jp, eu):
        self.jp, self.eu = jp, eu
        self.runs = anchor_runs(jp, eu)
        self.starts = [r[0] for r in self.runs]

    def around(self, o):
        """(containing, before, after) runs for JP offset o."""
        i = bisect.bisect_right(self.starts, o) - 1
        containing = None
        for r in self.runs[max(0, i - 8):i + 1]:
            # Runs overlap at their ends; the one reaching furthest past o
            # wins, as tables and packets run forward from their address.
            if r[0] <= o < r[0] + r[2] and (containing is None or r[0] + r[2] > sum(containing[::2])):
                containing = r
        before = self.runs[i] if i >= 0 else None
        after = self.runs[i + 1] if i + 1 < len(self.runs) else None
        return containing, before, after

    def position(self, o):
        """(eu, method, detail) for a positional mapping, or (estimate, None, why)."""
        containing, before, after = self.around(o)
        if containing:
            return o + containing[1] - containing[0], 'exact', f'run {containing[2]:#x} bytes'
        if not before or not after:
            return None, None, 'no anchors'
        a_end, e_end = before[0] + before[2], before[1] + before[2]
        b_start, f_start = after[0], after[1]
        estimate = o + before[1] - before[0]
        if not 0 <= f_start - e_end <= 0x2000 or b_start - a_end > 0x2000:
            return self.one_sided(o, before, after, estimate)
        return self.inside_gap(o, a_end, b_start, e_end, f_start, estimate)

    def one_sided(self, o, before, after, estimate):
        """Anchors disagree (a block moved): align from the nearer side only."""
        a_end, e_end = before[0] + before[2], before[1] + before[2]
        b_start, f_start = after[0], after[1]
        span = 0x400
        if b_start - o < o - a_end:
            lo = max(a_end, o - span)
            found = self.inside_gap(o, lo, b_start, f_start - (b_start - lo) - span // 2,
                                    f_start, None, open_left=True)
        else:
            hi = min(b_start, o + span)
            found = self.inside_gap(o, a_end, hi, e_end, e_end + (hi - a_end) + span // 2,
                                    None, open_right=True)
        if found[1]:
            return found[0], found[1], found[2] + ' (one side)'
        return estimate, None, 'anchors disagree'

    def inside_gap(self, o, a0, a1, e0, e1, estimate, open_left=False, open_right=False):
        """Refines within a gap with short local anchors (SequenceMatcher).

        An open side has no anchor at its edge, so the stretch next to it
        cannot count as a same-length gap.
        """
        left, right = self.jp[a0:a1], self.eu[max(0, e0):e1]
        blocks = [] if open_left else [(0, 0, 0)]
        if left and right:
            matcher = difflib.SequenceMatcher(None, left, right, autojunk=False)
            blocks += [(a, b, n) for a, b, n in matcher.get_matching_blocks()
                       if n >= LOCAL_BLOCK]
        if not open_right:
            blocks.append((len(left), len(right), 0))
        rel = o - a0
        for a, b, n in blocks:
            if a <= rel < a + n:
                return max(0, e0) + b + rel - a, 'exact', f'local block {n} bytes'
        e0 = max(0, e0)
        for (a, b, n), (c, d, _) in zip(blocks, blocks[1:]):
            if a <= rel < a + n:
                return e0 + b + rel - a, 'exact', f'local block {n} bytes'
            if a + n <= rel < c:
                if c - (a + n) == d - (b + n):
                    return e0 + b + rel - a, 'aligned', f'gap {c - a - n:#x} bytes, same length'
                return (e0 + b + n + min(rel - a - n, max(0, d - b - n - 1)), None,
                        f'gap {c - a - n:#x}->{d - b - n:#x} bytes')
        return estimate, None, 'gap'


# --- instruction shape -------------------------------------------------------

def shape(image, at, count=SHAPE_LENGTH):
    """Opcodes and non-address operands of the stream from `at`, or None."""
    out, m16, x16 = [], True, True
    for _ in range(count):
        if at >= len(image):
            return None
        opcode = image[at]
        if opcode == 0x02:
            service = image[at + 1]
            length = COP_LENGTHS.get(service)
            out.append(('COP', service))
            if length is None:
                break
            at += 2 + length
            continue
        decoded = disasm65.decode(image, at, m16, x16)
        if decoded is None:
            return None
        length, _, m16, x16 = decoded
        mnemonic, mode = disasm65.TABLE[opcode]
        if mode in ('imm8', 'immM', 'immX', 'dp', 'dpx', 'dpy', '(dp)', '(dp),y',
                    '[dp]', '[dp],y', '(dp,x)', 'sr', '(sr),y', 'blk'):
            out.append((opcode, bytes(image[at + 1:at + length])))
        else:
            out.append((opcode, length))
        at += length
        if mnemonic in ('RTS', 'RTL', 'RTI', 'BRA', 'BRL', 'JMP', 'JML', 'STP'):
            break
    return tuple(out) if len(out) >= 4 else None


def shape_match(jp, eu, o, estimate):
    """EU offsets near `estimate` whose stream shape equals JP's at `o`."""
    wanted = shape(jp, o)
    if wanted is None or estimate is None:
        return None, []
    low = max(0, estimate - SHAPE_WINDOW)
    high = min(len(eu), estimate + SHAPE_WINDOW)
    first = jp[o]
    hits = [at for at in range(low, high) if eu[at] == first and shape(eu, at) == wanted]
    hits.sort(key=lambda at: abs(at - estimate))
    return wanted, hits


# --- pointers ----------------------------------------------------------------

# Opcodes whose operand is commonly a pointer: loads, stores, jumps, calls.
LONG_OPERAND = {0xAF, 0xBF, 0x8F, 0x9F, 0x22, 0x5C, 0xA7, 0xB7}
WORD_OPERAND = {0xA9, 0xA2, 0xA0, 0xAD, 0xBD, 0xB9, 0xAE, 0xBE, 0xAC, 0xBC,
                0xF4, 0x20, 0x4C, 0x7C, 0xFC, 0x8D, 0x9D, 0x99}


class Pointers:
    """References to a JP offset, read back at their European twins.

    A code reference is an operand: after a pointer-taking opcode, or the
    first or second word of a `COP` script command. The same opcode or
    command must sit at the European twin. Anything else that happens to
    hold the address is a table reference, weaker evidence since any three
    bytes can look like a pointer.
    """

    def __init__(self, jp, eu, aligner):
        self.jp, self.eu, self.aligner = jp, eu, aligner
        self.long = defaultdict(list)
        for at in range(len(jp) - 2):
            bank = jp[at + 2]
            if bank >= 0x80:
                value = ((bank & 0x3F) << 16) | jp[at] | (jp[at + 1] << 8)
                self.long[value].append(at)

    def short_refs(self, o):
        """16-bit references from o's own bank, and `COP 1C` calls from anywhere."""
        bank, word = o & 0xFF0000, bytes([o & 0xFF, (o >> 8) & 0xFF])
        refs = set(self.occurrences(word, bank, bank + 0x10000))
        for bank_byte in {0x80 | (o >> 16), 0xC0 | (o >> 16)}:
            call = bytes([0x02, 0x1C, bank_byte]) + word
            refs.update(at + 3 for at in self.occurrences(call, 0, len(self.jp)))
        return sorted(refs)

    def occurrences(self, needle, start, end):
        while True:
            at = self.jp.find(needle, start, end)
            if at < 0:
                return
            yield at
            start = at + 1

    def twin(self, at):
        """The EU place of a referrer; one-sided alignments are too loose here."""
        position, method, detail = self.aligner.position(at)
        return position if method and 'one side' not in detail else None

    def context(self, at, twin, opcodes):
        """'code' when an operand of the same instruction at both ends, else 'table'.

        Script commands count when the address is the first operand of a
        known `COP`, the address after `COP 1C`'s bank byte, or the branch
        target of `COP 08` (after its flag word).
        """
        jp, eu = self.jp, self.eu
        if at >= 1 and jp[at - 1] in opcodes and eu[twin - 1] == jp[at - 1]:
            return 'code'
        for back, services in ((2, None), (3, {0x1C}), (4, {0x08})):
            if at < back or jp[at - back] != 0x02:
                continue
            service = jp[at - back + 1]
            if COP_LENGTHS.get(service) is None or (services and service not in services):
                continue
            if eu[twin - back:twin - back + 2] == jp[at - back:at - back + 2]:
                return 'code'
        return 'table'

    def votes(self, o, estimate):
        """({'code': Counter, 'table': Counter}) of EU values at the twins.

        An unchanged reference inside bytes copied as they are says nothing
        about a move, so it only counts when it agrees with `estimate`.
        """
        found = {'code': Counter(), 'table': Counter(), 'moved': Counter()}
        eu = self.eu
        for at in self.long.get(o, []):
            twin = self.twin(at)
            if twin is None or twin + 3 > len(eu) or eu[twin + 2] < 0x80:
                continue
            value = ((eu[twin + 2] & 0x3F) << 16) | eu[twin] | (eu[twin + 1] << 8)
            changed = eu[twin:twin + 3] != self.jp[at:at + 3]
            if changed or value == estimate:
                where = self.context(at, twin, LONG_OPERAND)
                found[where][value] += 1
                if changed and where == 'code':
                    found['moved'][value] += 1
        bank = (estimate if estimate is not None else o) & 0xFF0000
        if o & 0xFFFF >= 0x0100:
            for at in self.short_refs(o):
                twin = self.twin(at)
                if twin is None or twin + 2 > len(eu):
                    continue
                word_bank = bank
                if at >= 3 and self.jp[at - 3:at - 1] == b'\x02\x1c':
                    word_bank = (eu[twin - 1] & 0x3F) << 16  # COP 1C bank, address
                value = word_bank | eu[twin] | (eu[twin + 1] << 8)
                start = 1 if word_bank != bank else 0
                changed = eu[twin - start:twin + 2] != self.jp[at - start:at + 2]
                if (changed or value == estimate) and self.context(at, twin, WORD_OPERAND) == 'code':
                    found['code'][value] += 1
                    if changed:
                        found['moved'][value] += 1
        return found


def verdict(counter):
    """The value most references agree on, with its count, or (None, 0)."""
    top = counter.most_common(2)
    if not top or (len(top) > 1 and top[0][1] == top[1][1]):
        return None, 0
    return top[0]


# --- per-address resolution --------------------------------------------------

def kind_of(uses, o):
    files = ' '.join(where for _, where, _ in uses)
    if o & 0xFFFF == 0:
        return 'bank'
    if '/text' in files or 'labels.rs' in files:
        return 'text'
    if 'shop' in files:
        return 'shop'
    if 'actors' in files or 'actor_script' in files or 'residents' in files:
        return 'script'
    if 'music' in files:
        return 'music'
    if 'sprites' in files:
        return 'sprite'
    if 'maps' in files:
        return 'map'
    return 'code' if (o & 0xFFFF) >= 0x8000 and (o >> 16) < 0x18 else 'data'


def resolve(jp, eu, aligner, pointers, o):
    """(eu, method, confidence, note) for one JP offset."""
    position, method, detail = aligner.position(o)
    found = pointers.votes(o, position)
    code, code_n = verdict(found['code'])
    table, table_n = verdict(found['table'])
    refs = f"{sum(found['code'].values())} code refs, {sum(found['table'].values())} table refs"
    wanted, hits = shape_match(jp, eu, o, position)
    shaped = hits[0] if len(hits) == 1 else None
    if method:
        agree = [name for name, value in (('code', code), ('table', table), ('shape', shaped))
                 if value == position]
        long_run = detail.startswith('run') and int(detail.split()[1], 16) >= 0x100
        loose = detail.startswith('local') or 'one side' in detail
        confidence = 'high' if long_run or agree or (method == 'exact' and not loose) else 'medium'
        note = detail + ''.join(f' +{name}' for name in agree)
        moved, moved_n = verdict(found['moved'])
        if moved is not None and moved != position and 'code' not in agree and not long_run:
            # An operand the European build rewrote names the new place;
            # matching bytes nearby are a coincidence or a stale copy.
            return moved, 'pointer', 'high' if moved_n >= 2 else 'medium', (
                f'{moved_n} rewritten code refs agree ({refs}); bytes suggested {runtime(position)} ({detail})')
        if code is not None and code != position and code_n >= 2 and 'code' not in agree:
            # The code reads elsewhere: the matching bytes here are a copy it
            # no longer uses. The role follows the code.
            return code, 'pointer', 'medium', (
                f'{code_n} of {refs} agree; same bytes also at {runtime(position)} ({detail})')
        if code is not None and code != position:
            note += f'; a code ref says {runtime(code)}'
            confidence = 'medium' if confidence == 'high' and not long_run else confidence
        return position, method, confidence, note
    if shaped is not None:
        confidence = 'high' if shaped in (code, table) else 'medium'
        return shaped, 'shape', confidence, f'{len(wanted)} instructions, unique; {detail}'
    for value, count, weight in ((code, code_n, 'code'), (table, table_n, 'table')):
        if value is not None and (count >= 2 or weight == 'code'):
            confidence = 'high' if value in hits else ('medium' if count >= 2 else 'low')
            return value, 'pointer', confidence, f'{count} of {refs} agree; {detail}'
    if hits:
        return hits[0], 'shape', 'low', f'{len(hits)} shape hits, nearest taken; {detail}'
    if position is not None:
        return position, 'estimate', 'low', detail
    return None, 'none', 'none', detail


def check_packet(jp, eu, o, target, method, confidence, note):
    """Confirms a packet by its decoded bytes, or finds the EU packet that has them."""
    packet = lz.decode(jp, o)
    if not packet or len(packet[0]) < 16:
        return target, method, confidence, note
    there = lz.decode(eu, target) if target is not None else None
    if there and there[0] == packet[0]:
        return target, method, 'high', note + ' +lz'
    hits = lz.find(eu, packet[0], near=target)
    if hits:
        return hits[0], 'lz', 'high' if len(hits) == 1 else 'medium', (
            f'packet decodes the same ({len(hits)} found); {method} said '
            f'{runtime(target) if target is not None else "-"}')
    return target, method, confidence, note + '; JP packet has no EU twin with the same bytes'


def bank_rows(rows, found):
    """Bank bases take the EU bank of the addresses their own sources read there.

    A base such as `0x92_0000` has no content of its own; the bytes at the
    bank start may have moved elsewhere than the tables the code reads
    through it.
    """
    trusted = [(o, row) for o, row in rows.items()
               if row[2] != 'bank' and row[4] in ('high', 'medium') and row[1] != '-']
    for o, row in rows.items():
        if row[2] != 'bank':
            continue
        files = {where.split(':')[0] for _, where, _ in found[o]}
        banks = Counter()
        for other, twin in trusted:
            if other >> 16 != o >> 16:
                continue
            shared = files & {where.split(':')[0] for _, where, _ in found[other]}
            banks[twin[1][:2]] += 2 if shared else 1
        written = int(found[o][0][0].replace('_', ''), 16) >> 16
        label = f'{(written | 0x80) if written < 0x40 else written:02X}:0000'
        if banks:
            (bank, votes), = banks.most_common(1)
            eu_bank = (int(bank, 16) & 0x3F) | ((written | 0x80) & 0xC0 if written >= 0x40 else 0x80)
            rows[o] = (label, f'{eu_bank:02X}:0000', 'bank', 'bank', 'high' if votes >= 4 else 'medium',
                       f'bank of {sum(banks.values())} weighted addresses in it; {row[5]}')
        else:
            rows[o] = (label,) + row[1:4] + ('low', 'no resolved address in this bank; ' + row[5])


def layout(aligner):
    """Per 32 KiB of the JP image: identical bytes found, and the main shifts."""
    lines = ['jp_half\tidentical\tshifts (eu - jp: bytes)']
    for half in range(0, 0x400000, 0x8000):
        shifts = Counter()
        for a, b, n in aligner.runs:
            lo, hi = max(a, half), min(a + n, half + 0x8000)
            if lo < hi:
                shifts[b - a] += hi - lo
        total = sum(shifts.values())
        top = ', '.join(f'{d:+X}: {n:X}' for d, n in shifts.most_common(3))
        lines.append(f'{runtime(half)}\t{100 * total // 0x8000}%\t{top}')
    return lines


def load_overrides(path):
    """{jp offset: (eu runtime address, reason)} from an overrides TSV."""
    overrides = {}
    with open(path, encoding='utf-8') as handle:
        next(handle)
        for line in handle:
            if line.strip():
                jp, eu, reason = line.rstrip('\n').split('\t')
                overrides[disasm65.normalized(jp)] = (eu, reason)
    return overrides


def apply_overrides(rows, overrides):
    """Sets the overridden rows' European address, method `hand` and the
    reason; an override for an address no source names is dropped."""
    for o, (eu, reason) in overrides.items():
        if o in rows:
            jp, found, kind, _, _, note = rows[o]
            rows[o] = (jp, eu, kind, 'hand', 'high', f'{reason} (matched {found}); {note}')


def main(argv):
    args = [a for a in argv if not a.startswith('--')]
    opts = dict(a[2:].split('=', 1) for a in argv if a.startswith('--') and '=' in a)
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    jp, eu = load(args[0]), load(args[1])
    repo = opts.get('repo', os.path.join(HERE, '..', '..'))
    found = addresses.scan(repo)
    aligner = Aligner(jp, eu)
    if '--layout' in argv:
        print('\n'.join(layout(aligner)))
        return 0
    pointers = Pointers(jp, eu, aligner)
    rows = {}
    for o, uses in sorted(found.items()):
        target, method, confidence, note = check_packet(
            jp, eu, o, *resolve(jp, eu, aligner, pointers, o))
        if kind_of(uses, o) == 'text' and method in ('exact', 'aligned') and '+code' not in note \
                and not note.startswith('run'):
            # English pages were split and reordered: bytes alone do not pair them.
            confidence = 'medium' if confidence == 'high' else confidence
            note += '; text page, no script names it: confirm with the EU text decoder'
        test_only = all(t for _, _, t in uses)
        scope = 'test' if test_only else 'code'
        note = f'{note}; {scope}; {uses[0][1]}' + (f' (+{len(uses) - 1})' if len(uses) > 1 else '')
        rows[o] = (runtime(o), runtime(target) if target is not None else '-',
                   kind_of(uses, o), method, confidence, note)
    bank_rows(rows, found)
    apply_overrides(rows, load_overrides(os.path.join(HERE, 'overrides.tsv')))
    rows = [rows[o] for o in sorted(rows)]
    out = open(opts['out'], 'w', encoding='utf-8') if 'out' in opts else sys.stdout
    print('jp_address\teu_address\tkind\tmethod\tconfidence\tnote', file=out)
    for row in rows:
        print('\t'.join(row), file=out)
    tally = Counter((r[3], r[4]) for r in rows)
    print(f'{len(rows)} addresses: ' + ', '.join(f'{m}/{c} {n}' for (m, c), n in sorted(tally.items())),
          file=sys.stderr)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
