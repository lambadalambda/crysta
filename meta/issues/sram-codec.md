# Read and write native SRAM

## Summary

Encode and decode the 8 KiB native SRAM: three slots with backups, the
checksum (`$8D:A867`), recovery from a bad primary, the erase marker and the
last-slot word ([saves](../../docs/saves.md)).

## Dependencies

- [Keep the native save slot in the world](save-slot-block.md)

## Acceptance Criteria

- Local native dumps (Japanese, European, a 2008 `.srm`) decode; writing them
  back is byte-identical; a corrupt primary recovers from its backup, both
  corrupt reads as no data, and nothing is changed on a failed read.
