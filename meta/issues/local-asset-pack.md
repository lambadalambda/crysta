# Build the reproducible local asset pack

## Summary

Package verified ROM-derived data for portable execution without requiring the ROM after extraction.

## Dependencies

- [Implement and verify the compression codec](compression-codec.md)
- [Decode map, metadata, and collision formats](decode-map-collision-formats.md)
- [Decode graphics, palettes, sprites, and animation](decode-graphics-animation.md)
- [Decode text and gameplay data tables](decode-text-gameplay-data.md)
- [Reverse the event script bytecode](reverse-event-bytecode.md)
- [Reverse the CPU-to-SPC audio protocol](reverse-audio-protocol.md)
- [Define the ROM revision and version-support model](define-version-support-model.md)
- [Define the oracle artifact and publication policy](define-oracle-artifact-policy.md)

## Requirements

- Define a versioned manifest with source ROM hash and codec versions.
- Package lossless maps, graphics, tables, scripts, and audio inputs needed by the current milestone.
- Validate integrity and reject incompatible packs.
- Keep output ignored and document browser-compatible generation options.

## Acceptance Criteria

- Two runs from the same normalized ROM produce identical asset-pack hashes.
- The pack contains no executable 65C816 code unless explicitly justified and documented.
- A clean public checkout cannot accidentally package or track generated content.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- Distribution policy must be reviewed before any generated pack is shared.
