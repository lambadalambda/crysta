# Load local SRAM in oracle sessions

## Summary

Allow ROM-backed verification scenarios to boot ares with caller-provided local SRAM without committing save data.

## Dependencies

- [Select and integrate the reference emulator](select-reference-emulator.md)

## Requirements

- Preserve the existing zero-filled SRAM behavior for `Session::new`.
- Accept exactly the cartridge's 8 KiB SRAM image before the emulated system powers on.
- Keep all caller-provided save bytes local and outside committed fixtures.

## Acceptance Criteria

- Invalid SRAM lengths fail before claiming the process-global session.
- A valid local SRAM image is visible to the game at boot.
- Existing ROM-free and ordinary oracle tests remain unchanged.

## Notes

- This enables controlled memory-symbol verification from user-supplied or externally sourced local saves; it does not make SRAM contents commit-safe.
- The qualified local scenario uses FantasyAnime Game Save #1, default slot 3 from
  <https://fantasyanime.com/legacy/terran_saves.htm>, with local SRAM SHA-256
  `709c1cb67b8aff8db49cba05959f128b1c0a1ca32184c9bb62c415d537658055`.
  The save is not bundled, and its provenance does not verify unrelated public
  memory-map meanings.

## Implementation

- Added `Session::new_with_sram`, accepting exactly 8 KiB. Length validation
  occurs before the process-global ares session is claimed.
- Valid SRAM is synchronously copied into the core before the emulated system
  powers on. Invalid lengths return a typed error without consuming the
  singleton session.
- `Session::new` preserves existing behavior by supplying zero-filled 8 KiB
  SRAM.
- The qualified integration scenario loads the optional external save from
  `local/saves/Terranigma.srm`; raw SRAM and symbol-trace bytes remain local.

## Verification

Targeted checks recorded as run:

- `cargo test -p oracle sram_length_validation_is_rom_free_and_does_not_claim`
  — invalid 0-byte, 8191-byte, and 8193-byte inputs fail without claiming the
  session.
- `cargo test -p oracle supplied_sram_is_visible_at_boot_and_default_is_zeroed`
  — caller SRAM is copied and visible at boot, while `Session::new` supplies
  zeroed SRAM.
- `cargo test -p oracle --test local_roms local_sram_trace_verifies_gameplay_symbols -- --nocapture`
  — the Japanese-ROM plus qualified local save child scenario ran and produced
  symbol-trace digest
  `6d2c6756ff22e9c41a8287f649252599a493422b54d684e886b64e9b12741595`.

- Full workspace formatting, Clippy with warnings denied, all-target tests, and
  rustdoc with warnings denied passed.
- Independent final review found no blockers or high-severity issues.
