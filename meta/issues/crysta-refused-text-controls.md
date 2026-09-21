# Draw the resident lines the house text profile refuses

## Summary

Nine residents in the slice reach dialogue that `talk_to` reports as
`Unsupported`, and the native app shows nothing for them. The issue tracker
and the app called these choice prompts. They are not: none of the nine
sources contains an in-text choice. They stop at controls the decoder either
admits only under the Pandora profile or refuses by a fixed operand list.

## Dependencies

- [Place Crysta residents and let the player talk to them](crysta-resident-interaction.md)

## What is already known

Census under new-game flags, by the decoder's own reason:

| Sources | Stop | Meaning |
| --- | --- | --- |
| 6 | `$DA` | Window anchor, `$85964D`: bottom unless the player stands low on screen, then top. Admitted only under the Pandora profile. |
| 1 | `$D2 02` | Speaker-prefix call through the `$92C447` table; only indices 0, 1, 7 (and 3 under Pandora) are admitted. |
| 1 | `$C2 03 03 19 06` | Window layout, `$85982D`; only the tuple `(6,6,24,6)` is admitted, and only under Pandora. |
| 1 | none | Map `$1D`, `$B35F`: the bytes are native actor code, not text. The script walker handed over a wrong address. Out of scope here. |

The `$13` source `$B6C7` also uses `$CC`, the long text call, after its
`$DA`.

## Requirements

- `HouseDialogue::decode_at` admits `$DA`, `$CC` and `$C2` in every profile,
  with the semantics the Pandora qualification recorded.
- `$D2 n` admits any table index whose pointer lies in ROM; index 0's WRAM
  default name keeps its special decode. Other WRAM targets are refused.
- `$C2` admits layouts other than `(6,6,24,6)` with the same formula:
  content is `width * 8` by `height / 2 * 16` pixels.
- A page records where the native engine places its window, so a host can
  honour `$DA` and `$C2` instead of always boxing along the bottom.
- The app draws the pages and places the box per the page's placement.

## Acceptance Criteria

- The eight text-backed `Unsupported` residents become `Speaks`; the talk
  census records the new counts.
- The seven qualified house sources and the Pandora requests decode
  unchanged.
- A source that is not text still fails closed.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Parent: [Native macOS window, renderer and gamepad for the Crysta slice](crysta-native-shell.md)
- The map-`$1D` wrong address is a walker defect; it is recorded here and
  not fixed.

## Progress: admitted and drawn

`$DA`, `$C2` and `$CC` decode in every profile; `$D2` accepts any table
entry in ROM. `DialoguePage::placement` records the window: `Bottom`,
`AwayFromPlayer` or `Tile { column, row }`, and the app's `page_origin`
puts the box at the top when a `$DA` page opens with the player in the
lower half of the screen, or at a `$C2` window's tile origin.

Talk census under new-game flags: **30 speak, 42 silent, 15 unaccounted, 1
unsupported**, from 22 / 42 / 15 / 9. The one left is map `$1D`'s native
code address. The seven house sources and the Pandora requests decode as
before.

Not modelled: `$85964D` also reads a signed word at `$048A`, not identified,
before comparing the player's row against the camera.
