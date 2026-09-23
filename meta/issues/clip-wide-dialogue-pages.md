# Clip dialogue pages to their box

## Summary

`draw_page` caps the box at the classic 256 pixels but draws the page's
content at full width. A page wider than 240 pixels (or taller than 208)
would draw past its box, and in the wide view outside the classic area.

## Dependencies

- [Add a 16:9 view to the native Crysta app](native-crysta-widescreen.md)

## Requirements

- Draw only the part of the page that fits inside the box's margins.
- Pages that fit already draw exactly as before.

## Acceptance Criteria

- A pure test with an oversized page fails before and passes after.
- Existing frame tests and app Clippy pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Found in review of the 16:9 view. Real pages are about 224 pixels wide or
  less, so no current page is affected.
