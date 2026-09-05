# Read-only sprite hardware inspection

`oracle::Session::sprite_state()` returns `SpriteState` for reference research:

- `oam: [u8; 544]`: the512-byte low table followed by32 physical high-table bytes;
- `obsel: u8`: reconstructed `$2101` tile-base/name-gap/size configuration;
- `first_sprite: u8`: the current first-sprite priority index,0..127.

These are **current PPU state**, not a guarantee that they are the scanline/output
latches which produced the last completed framebuffer. Correlate them with CPU
stops, DMA and frame ownership when qualifying rendered poses. This interface
is reference-only; neither the device-free core nor the live house calls it.

The project-authored shim uses ares's pure `OAM::read` reconstruction directly,
not the side-effecting `$2138` PPU I/O register. Addresses0..511 expose low-table
object fields;512..543 pack each group's ninth-X and size bits. OBJSEL reverses
`io.cpp`'s `$2101` assignments: tile word base>>13, name gap<<3, size mode<<5.
The byte ABI fills exactly546 bytes; no C/Rust struct-layout dependency exists.

## Verification

```sh
cargo test -p oracle --test local_sprite_state -- --nocapture
```

The optional owned-JP-ROM test executes the shared fresh New Game prefix in two
independent processes, one `Session::new` per process, with explicit exit0.
At completed6800 it pins:

- Physical OAM SHA-256:
  `eb7269c9cfae9312395d4450d732e6f27d68a8bf9de453c8067d30255e0ccf5b`.
- OBJSEL **2** (tile word base`$4000`, name-gap code0, size-mode0).
- First-sprite index **0**.

Repeated inspection leaves CPU registers/frame/cycle fields and exported
WRAM, VRAM, CGRAM, APU RAM, framebuffer and audio sample digests unchanged.
This tests exported-surface invariance, not every hidden emulator latch.
Compile-time packing tests cover zero, mixed bitfields and maximum OBJSEL.
The first Rust test failed because the accessor did not exist; it passed after
implementation and the observation correction below. No raw OAM is committed.

**Important test finding:** `save_state` calls synchronized serialization
(`core.root->serialize(true)`), which is not a passive observation of a stopped
core. Comparing serialized bytes before and after a read falsely attributes
serializer synchronization to that read. The regression therefore uses the
existing read-only observation APIs, without serialization or restoration.

This API is groundwork for [Ark sprite qualification](../meta/issues/qualify-ark-sprite-assets.md),
not proof that the portable house renders Ark yet.
