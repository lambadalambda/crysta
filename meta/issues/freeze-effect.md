# Play the freeze: ice crystals, the brightening, a blue Elle

## Summary

When the people freeze, ice crystals fly toward Elle, the screen brightens and Elle turns blue; none of it shows.

## Dependencies

- [Fix the slice bugs the user reported on 2026-10-01](slice-bug-report.md)

## Requirements

- Research the effect's sprites, motion, palette changes and colour math, and play it where the script starts it.

## Acceptance Criteria

- The freeze matches native frames on both ROMs.

## Notes

- Native runs pause at the nested frame (`$80:80DF`), so the whitening plays one step a frame (98 frames white, natively 100); the crystals orbit with `COP D0`/`D1` from the `$81:F563` table; Elle turns blue with `COP BB` (`local_story.rs` frozen return). Checked by eye against `local/effects/jp/freeze`. The crystals' last frame at radius 0 is not drawn (natively it is).
