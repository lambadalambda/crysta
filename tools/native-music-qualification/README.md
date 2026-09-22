# Bounded native Crysta music source recipe

Part of [native Crysta music](../../meta/issues/native-crysta-music.md). This
qualifies **Japanese fresh-game music selection 3 only**, not every map audio
transition, sound effects, other tracks, or a complete CPU/SPC protocol. No
SPC/WRAM capture is accepted by the production extractor.

`crates/crysta-app/src/music_data.rs` is pure extraction from `rom::Rom`, with
an additional actual SHA-256/CRC check against the built-in Japanese identity
(the caller-assigned revision supported by test loaders is not authentication).
It reuses the existing packed-pointer decoder. Each block records its normalized
ROM source ranges, destination, and payload. Only the owned input supplies bytes.
This small independent test crate uses existing MIT project crates `rom` and
`assets`; it does not depend on or modify the app/audio backend.

## Evidence chain (reconstructed annotation, no extracted payloads)

- Host `$86:AA9C` selects bootstrap stream `$86:AC42`; `$86:ABB0` reads
  little-endian `(length:u16, destination:u16, payload[length])` records.
  Length zero ends the group. Initial IPL execution entry is `$0300`.
- Fresh bedroom/exterior audio's branch at `$98:83AE`, with fresh flag23 clear,
  selects subscript7 and reaches FC selection3 at `$98:83C6`. Selection3 is not
  the straight-line default5. This extractor does not evaluate runtime events.
- `$86:9047` uses music-list slot0 at `$86:959C`, whose pointer is `$98:8002`.
  `$86:8F37` scans six-byte records (opcode02, play flag, selection, packed
  three-byte pointer). Selection3 resolves to `$AA:8C82`. `$86:AC26` reads
  the low nibble of `$96:F2A3 + (selection+1)*4`: F0 parameter7.
- `$86:AAA9` uploads the inline group, then reads a count and sample IDs after
  its terminator. Its terminal address `$821C` is the sample destination base,
  **not an execution entry** for the resident receiver.
- The sample pool at `$B8:8000` is a sequence of little-endian length prefixes
  and payloads. `$86:AAEA–AB65` restarts the scan for each sample ID and skips
  lower bank halves: `$xx:FFFF -> $(xx+1):8000`. Destination advances by the
  previously uploaded sample's length. Selection3 uses eight IDs:
  `$13,$19,$1A,$1B,$0E,$1C,$1D,$1E`.
- Driver receiver at SPC `$040B–044A` accepts IPL-style transfers, but ignores
  the zero-length terminal address, resets its command state, and returns to
  the resident loop. Command `$FF` reenters it; `$F4` starts sequence playback.

All offsets below are normalized ROM offsets; lengths/destinations are hex.

| Group | Payload source | Length | SPC destination |
| --- | --- | --- | --- |
| bootstrap | 06AC46 | 0020 | 01E0 |
| bootstrap | 06AC6A | 0BF5 | 0300 |
| bootstrap | 06B863 | 006F | FF16 |
| sequence | 2A8C86 | 0030 | 1048 |
| sequence | 2A8CBA | 0018 | 0FE8 |
| sequence | 2A8CD6 | 0B72 | 76AA |
| sequence | 2A984C | 0020 | 0F30 |
| sequence | 2A9870 | 0002 | 10FC |
| sequence | 2A9876 | 0002 | 10FE |
| sequence | 2A987C | 0008 | FFAB |
| samples | 39A5FD | 032A | 821C |
| samples | 39CF4F | 027F | 8546 |
| samples | 39D1D0 | 0FA5 | 87C5 |
| samples | 39E177 | 0036 | 976A |
| samples | 38EF5A, then 398000 | 20BB total | 97A0 |
| samples | 39E1AF | 0051 | B85B |
| samples | 39E202 | 04FE | B8AC |
| samples | 39E702 | 03D5 | BDAA |

Zero-length terminal destinations: bootstrap `$0300`, sequence `$821C`, samples
`$BDAA`. The sample terminator preserves the **last start**, not the final end.

## Host protocol for the audio-only backend

Use separate input/output port latches. Bound every wait by a cycle budget;
never spin indefinitely on an incorrect backend/transfer. Port numbers below
are 0–3 (`$2140–2143` on the original CPU).

For each group:

1. Wait for output ports0/1 to equal `$AA/$BB`. First header token is `$CC`.
2. Write destination low/high to input ports2/3, input port1=1 for data (0 for
   zero-length terminal), then input port0=header token. Wait output port0=token.
3. For payload index `i`, write input port1=byte, port0=`i mod 256`, then wait
   output port0=`i mod 256`. **Run 32 extra SPC cycles after each payload ACK
   before changing any input.** Resident code echoes at `$0421` before reading
   input port1 at `$0423`. Without settling time fast hosts silently corrupt
   uploads; IPL itself reads data before ACK. The 32-cycle gap works for both.
4. Next header token is `(length - 1 + 4) mod 256`; replace zero with4. This
   reconstructs host ADC03 with carry set by the successful comparison.
5. After terminal ACK clear ports2/3. For bootstrap, entry `$0300` executes;
   the resident receiver instead returns to its command loop for later groups.

Qualified source-only startup sequence (cycle delays are conservative setup
waits corresponding approximately to the host's frame waits, not gameplay):

1. Fresh `Apu::new()` / physical IPL; upload bootstrap; run20000 SPC cycles.
2. Input port0=0, port1=`stop_parameter` (7), port0=`F0`. Run18000 cycles; wait
   output port0=0. Run36000 cycles; input port0=`FF`; run36000 cycles.
3. Upload sequence group. Input port0=`FF`; upload sample group.
4. Clear input ports1–3; run54000 cycles; input port0=`F4`.
5. Render stereo PCM at32000 Hz. Do not run/discard setup cycles during playback.

The repeat FF between groups is needed because each zero-length terminal exits
its upload session. No additional ROM data, game CPU, or emulator save state is
needed for this bounded startup.

## Verification / reproduction

```sh
cargo test --manifest-path tools/native-music-qualification/Cargo.toml
CRYSTA_JP_ROM="$PWD/local/Tenchi Souzou (Japan).sfc" \
  cargo test --manifest-path tools/native-music-qualification/Cargo.toml -- --ignored
cargo clippy --manifest-path tools/native-music-qualification/Cargo.toml \
  --all-targets -- -W clippy::pedantic -W missing_docs -D warnings
```

TDD: synthetic framing/bounds/provenance tests were written first, failed to
compile against the missing extractor, then passed. Pure disassembly research
preceded those tests. Public tests use only synthetic bytes; the ROM-backed test
is explicitly ignored unless requested and reads `CRYSTA_JP_ROM`.

An ignored standalone probe linked the parent's safe MIT LakeSnes `spc-player`
wrapper and this exact Rust extractor. All sequence/sample destination RAM
matched the descriptors byte-for-byte **before** starting playback, including
sample `$0E`'s skipped-half-bank crossing. Physical IPL initialization and the
above port sequence produced ten seconds of PCM: peak5785, 623585 nonzero stereo
samples out of640000. An earlier C audio-only probe independently produced
non-silent output. These are source-transfer/non-silence checks, not a claim of
bit-perfect whole-game audio equivalence or a native audio-device smoke test.
All probes, disassembly, raw samples, and PCM remain ignored under `local/music/`.
The parent integration owns the reproducible backend/playback smoke command.
