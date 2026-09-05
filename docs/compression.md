# Terranigma compression packets

The `assets::compression` module is an original, dependency-free implementation
of the packet format used for compressed graphics and map data. It reads
caller-owned slices and performs no file I/O. Only the observed zero-header
variant is qualified; this is not a general-purpose ROM scanner or an asset
format decoder.

## Sources and publication boundary

Research retrieved 2026-09-05:

- [Terranigma Wiki: Compression, revision 343](https://www.terranigma.be/index.php?title=Compression&oldid=343)
  describes the interleaved control stream and the original greedy compressor.
- [Brad Smith / rainwarrior's TerranigmaCompressor.py](https://gist.github.com/bbbradsmith/935c03fc31d81ad29b489a943bc79a5c),
  raw revision `705b7b41be6386bf265cb9bbac708cf09c7edf70`, provides an independent
  community decoder used locally to establish output hashes. Its downloaded
  source SHA-256 is `29050cdac65dbadbc4acd3c80aa909e0af964bfd6a8209d3fea8090abe8dd481`.
- [Wiki graphics offsets](https://www.terranigma.be/index.php/Graphics) and
  [map offsets, revision 1136](https://www.terranigma.be/index.php?title=Maps&oldid=1136)
  supplied candidate packet locations, not trusted cross-version addresses.

No compatible source-code license was found in the community Python script.
It was studied and run locally only, and is not copied, vendored, invoked by
workspace tests, or required at runtime. The Rust implementation and synthetic
fixtures are original. Community-tool downloads, decoded bytes, and compressed
packets remain under ignored `local/compression-research/`.

## Packet structure

| Byte offset | Meaning |
| --- | --- |
| 0 | Zero (unknown historical chunk/variant field; nonzero unsupported) |
| 1–2 | Nonzero decompressed size, little-endian `u16` |
| 3 | First output byte, unconditionally literal |
| 4 onward | Interleaved control bytes and token operands |

A new control byte is read **only when the next control bit is requested and no
bits remain**. Bits are consumed MSB first. Literal/copy operand bytes come from
the same input cursor but do not consume control bits. A token's control code
can straddle two control bytes; a byte boundary does not prefetch control ahead
of an operand.

Let `n` be the number of bytes already output:

| Control bits | Operand | Action |
| --- | --- | --- |
| `1` | One byte | Append literal |
| `00ab` | One byte `o` | Copy `2 + 2a + b` bytes from `n - 256 + o` |
| `01` | Big-endian word `w`, low 3 bits nonzero | Copy `2 + (w & 7)` bytes from `n - 8192 + (w >> 3)` |
| `01` | Big-endian word with low 3 bits zero; extra byte `l != 0` | Copy `l + 1` bytes from the same long-distance source |
| `01` | Word with low 3 bits zero; extra byte zero | End of packet; offset ignored |

Copies proceed byte by byte and may overlap their destination, allowing runs.
Short distances cover 1–256 bytes; long distances cover 1–8192. Short copies
cover 2–5 bytes, compact long copies 3–9, and extended long copies 2–256.
The conventional terminator uses three zero operands; the decoder also accepts
nonzero ignored offset bits and unused final control bits.

## Safe decoder contract

```rust,ignore
let packet = assets::compression::decode(compressed, output_limit)?;
// packet.data is exactly the declared output size.
// packet.consumed ends immediately after the explicit terminator.
```

The decoder checks the header and caller's output budget before allocating.
Output is at most 65,535 bytes. A zero length is rejected rather than guessed to
mean 64 KiB. It rejects truncated headers/control/operands, references before the
start of output, premature termination, and tokens exceeding the declared
length. Reaching the declared output length without a terminator is still an
error. Trailing bytes belong to the next packet or container and are not read.
There is no repair, implicit zero-filled dictionary, unchecked index, or
unbounded end-marker search. Every nonterminal token produces at least a byte,
so the declared output length bounds work as well as allocation.

This deliberately differs from the community script's permissive malformed-
input behavior (warnings, synthesized bytes, and corrected invalid references).
Only well-formed, authenticated packets were used as reference evidence.

## Qualified local packets

All offsets below are normalized/headerless. The test authenticates the entire
ROM with the existing `rom` crate first; JP and EU remain distinct revision
identities. EU is a codec/localization compatibility check, not a second gameplay
behavior reference.

| Packet | JP offset | EU offset | Compressed bytes JP / EU | Output bytes |
| --- | --- | --- | --- | --- |
| Bank-zero graphics | `$000000` | `$000000` | 5,779 / 6,196 | 8,192 |
| Intro Earth graphics | `$2D0000` | `$2F0000` | 18,112 / 18,112 | 32,768 |
| Box menu map | `$30B113` | `$32B215` | 796 / 796 | 2,048 |

The Earth and box packets have identical compressed bytes in both revisions;
their JP counterparts were found by full-packet matching, not a guessed fixed
address delta. The bank-zero packets have different content. The wiki's Crysta
candidate `$2BBAFB` did **not** validate as a zero-header compressed packet in
either owned dump and was rejected rather than corrected heuristically.

The full compressed and decoded SHA-256 values are pinned in
[`crates/assets/tests/local_roms.rs`](../crates/assets/tests/local_roms.rs).
Output hashes were established by the separately downloaded community decoder,
then matched by the Rust decoder. No decoded bytes are in the repository.

To reproduce the community comparison locally, use the pinned Python tool's
`e <headerless-rom> <hex-offset> <local-output>` command for each table row.
The JP dump is already headerless; normalize the EU dump with `rom::Rom::load`
first rather than treating its 512-byte copier header as packet data. The
community tool reports packet end and compressed size; hash the emitted bytes.
Workspace tests need only the original dumps, not the Python tool:

```sh
cargo test -p assets --test local_roms -- --nocapture
```

Missing dumps produce explicit skips; other read errors or invalid images fail.
Synthetic tests cover every token form, control refills within tokens and after
literal payloads, overlapping and maximum-distance copies, ignored terminator
bits, truncated prefixes, output budgets, and a deterministic malformed corpus.
