# Cover the native routines the underworld scripts call

## Summary

`docs/cop-inventory.md` lists 16 routines that the underworld scripts call
with `JSL`. The runtime's native model runs three itself; the others run
only where a Rust model replaces the code around the call.

## Dependencies

- [Draw the continents' door parchment](continent-door-parchment.md)

## Requirements

- Each call site is covered by a model, or by the issue that will model
  it.

## Acceptance Criteria

- The table below has no call site without a model or an issue.

## Notes

| Routine | Sites | Covered by |
|---|---|---|
| `$80:BBC7` flag test, `$80:BBCD` flag write, `$80:ED75` pose step | all | the native model (`actors/native.rs`, `CALLS`) |
| `$86:8236` (random), `$8D:8C7E`..`8D55` (cells) | `$85:E2F9`, `$85:E4D8`..`E513` | the death script `$85:E27B`, which `actors/foe.rs` models (`die`) |
| `$86:81C7` (divide) | `$93:E5A8`, `E5BB` | `$93:E5A3`, `arc_step` in `actors/shadowkeeper.rs` |
| `$86:81B0` (multiply) | `$93:E787`..`E8A0` | the sine placement in `actors/shadowkeeper.rs` |
| `$8F:8129` (random) | `$8F:80F9` | `WISP_STREAM` in `actors/routines.rs` |
| `$8D:A889`, `$86:83BE`, `$86:E834`, `$86:E894` | `$90:A4D7`..`A530` | not modelled: the continents' door parchment, skipped (`PARCHMENT`) |
| `$80:ED75` | `$90:A6A3`, `A6E0`, `A71D`, `A75A` | not run: the parchment's 4 actors wait for flag `$001`, which only the skipped set-up sets; forced, they freeze at `LDA $081E` |

## Progress

- Done: the inventory's Runtime column (`actors::models_call`).
