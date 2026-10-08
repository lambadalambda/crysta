//! Enemies against walls (`$80:D101`, `docs/enemy-scripts.md` §6): an
//! entity with `+$04 & $0006 == $0004` probes the leading edge of its box
//! in the map's first layer, 16-pixel cells, and a blocked axis is clamped
//! to the cell edge.

/// The map's first layer: native cell words, `width * height`.
#[derive(Debug, Clone, Copy)]
pub(super) struct Layer<'a> {
    pub(super) cells: &'a [u16],
    pub(super) width: u16,
    pub(super) height: u16,
    /// Whether the door gaps block: `$048A & $8000`, a tower floor.
    pub(super) gaps: bool,
}

impl Layer<'_> {
    /// Whether the cell at (column, row) stops an enemy: every attribute
    /// but 0, 1, 17 and 22 (`$80:E11C`). Attribute 2, the door gaps, blocks
    /// only with `$048A & $8000`, which the tower floors set. Cells off the
    /// map block.
    pub(super) fn blocks(&self, column: i32, row: i32) -> bool {
        self.cell(column, row)
            .is_none_or(|word| match (word >> 9) & 0x1F {
                0 | 1 | 17 | 22 => false,
                2 => self.gaps,
                _ => true,
            })
    }

    /// The word of the cell at (column, row); `None` off the map.
    pub(super) fn cell(&self, column: i32, row: i32) -> Option<u16> {
        let (column, row) = (u16::try_from(column).ok()?, u16::try_from(row).ok()?);
        if column >= self.width || row >= self.height {
            return None;
        }
        self.cells
            .get(usize::from(row) * usize::from(self.width) + usize::from(column))
            .copied()
    }
}

/// The cells a blocked test (`COP C2`..`C5`, `$80:AB2E`) probes, Up, Down,
/// Left, Right, for the box (x offset, width, y offset, height) at `at`: a
/// cell beyond the box's edge, from its top or left, one per 16 pixels of
/// its width or height rounded up (`$80:C092`, `C0AD`, `C0FB`, `C116`).
pub(super) fn beyond(direction: u8, at: (u16, u16), [ox, w, oy, h]: [i8; 4]) -> Vec<(i32, i32)> {
    let [ox, w, oy, h] = [ox, w, oy, h].map(i32::from);
    let (left, top) = (i32::from(at.0) + ox, i32::from(at.1) + oy);
    let (across, down) = ((w + 15) >> 4, (h + 15) >> 4);
    match direction {
        0 | 1 => {
            let row = (if direction == 0 { top - 16 } else { top + h }) >> 4;
            (0..across).map(|n| ((left >> 4) + n, row)).collect()
        }
        _ => {
            let column = (if direction == 2 { left - 16 } else { left + w }) >> 4;
            (0..down).map(|n| (column, (top >> 4) + n)).collect()
        }
    }
}

/// Whether a cell word stops a blocked test (`$80:C0E2`): its high byte
/// halved, the mark of bit 15 with it, passes below 2 and at 22.
pub(super) const fn stops_a_test(word: u16) -> bool {
    !matches!(word >> 9, 0 | 1 | 22)
}

/// One frame's move of `delta` from `at` for the box (x offset, width,
/// y offset, height): x first, then y from the new x. Returns the new
/// position and which axes were blocked (their streams stop).
pub(super) fn step(
    at: (u16, u16),
    delta: (i16, i16),
    [ox, w, oy, h]: [i8; 4],
    layer: &Layer<'_>,
) -> ((u16, u16), (bool, bool)) {
    let [ox, w, oy, h] = [ox, w, oy, h].map(i32::from);
    let (mut x, mut y) = (i32::from(at.0), i32::from(at.1));
    let (dx, dy) = (i32::from(delta.0), i32::from(delta.1));
    let mut blocked = (false, false);
    if dx != 0 {
        x += dx;
        let (left, top) = (x + ox, y + oy);
        let column = if dx > 0 { left + w - 1 } else { left } >> 4;
        let cells = (h >> 4) + i32::from(top & 15 != 0);
        if (0..cells).any(|i| layer.blocks(column, (top >> 4) + i)) {
            x = if dx > 0 {
                ((left + w) & !15) - ox - w
            } else {
                (left & !15) + 16 - ox
            };
            blocked.0 = true;
        }
    }
    if dy != 0 {
        y += dy;
        let (left, top) = (x + ox, y + oy);
        let row = if dy > 0 { top + h - 1 } else { top } >> 4;
        let cells = (w >> 4) + i32::from(left & 15 != 0);
        if (0..cells).any(|i| layer.blocks((left >> 4) + i, row)) {
            y = if dy > 0 {
                ((top + h) & !15) - oy - h
            } else {
                (top & !15) + 16 - oy
            };
            blocked.1 = true;
        }
    }
    let wrap = |value: i32| u16::try_from(value.rem_euclid(0x1_0000)).unwrap_or(0);
    ((wrap(x), wrap(y)), blocked)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 4x4 room: a wall (attribute 3) round a 2x2 floor (attribute 0).
    fn room() -> Vec<u16> {
        (0..16)
            .map(|at| {
                let (column, row) = (at % 4, at / 4);
                if (1..3).contains(&column) && (1..3).contains(&row) {
                    0
                } else {
                    3 << 9
                }
            })
            .collect()
    }

    const BOX: [i8; 4] = [-8, 16, -16, 16];

    #[test]
    fn a_free_move_goes_on_and_a_wall_clamps_its_axis() {
        let cells = room();
        let layer = Layer {
            cells: &cells,
            width: 4,
            height: 4,
            gaps: true,
        };
        // The box spans x 16..32, y 16..32 at (24, 32): free inside.
        assert_eq!(
            step((24, 32), (0, 0), BOX, &layer),
            ((24, 32), (false, false))
        );
        assert_eq!(
            step((26, 34), (1, 1), BOX, &layer),
            ((27, 35), (false, false))
        );
        // Left into column 0: clamped to x 24, y goes on.
        assert_eq!(
            step((24, 40), (-1, 1), BOX, &layer),
            ((24, 41), (true, false))
        );
        // Right into column 3: the box's right edge stops at 48.
        assert_eq!(
            step((40, 40), (1, 0), BOX, &layer),
            ((40, 40), (true, false))
        );
        // Down into row 3: the box's bottom stops at 48.
        assert_eq!(
            step((30, 48), (0, 2), BOX, &layer),
            ((30, 48), (false, true))
        );
        // Up into row 0: the box's top stops at 16.
        assert_eq!(
            step((30, 32), (0, -1), BOX, &layer),
            ((30, 32), (false, true))
        );
    }

    #[test]
    fn passable_attributes_and_body_marks_do_not_block() {
        // Attributes 1, 17 and 22 pass; bit 15 (a body's mark) is not one.
        for word in [1 << 9, 17 << 9, 22 << 9, 0x8000] {
            let cells = vec![word; 16];
            let layer = Layer {
                cells: &cells,
                width: 4,
                height: 4,
                gaps: true,
            };
            assert_eq!(step((24, 32), (-9, 0), BOX, &layer).1, (false, false));
        }
    }

    #[test]
    fn the_blocked_test_probes_a_cell_beyond_the_box_from_its_edge() {
        // Box (-8, 16, -16, 16) at (28, 40): left 20, top 24, 16 across.
        // Up: y 24 - 16 = 8, one cell from x 20 (`$80:C092`).
        assert_eq!(beyond(0, (28, 40), BOX), vec![(1, 0)]);
        // Down: y 24 + 16 = 40 (`$80:C0AD`).
        assert_eq!(beyond(1, (28, 40), BOX), vec![(1, 2)]);
        // Left: x 20 - 16 = 4, one cell down from y 24 (`$80:C0FB`).
        assert_eq!(beyond(2, (28, 40), BOX), vec![(0, 1)]);
        // Right: x 20 + 16 = 36 (`$80:C116`).
        assert_eq!(beyond(3, (28, 40), BOX), vec![(2, 1)]);
        // A 32-pixel box takes two cells, a 17-pixel one two as well.
        assert_eq!(beyond(0, (28, 40), [-8, 32, -16, 16]).len(), 2);
        assert_eq!(beyond(2, (28, 40), [-8, 16, -16, 17]).len(), 2);
    }

    #[test]
    fn the_blocked_test_passes_only_attributes_0_1_and_22() {
        // `$80:C0E2`: the word's high byte halved, below 2 or 22.
        for (word, solid) in [
            (0, false),
            (1 << 9, false),
            (22 << 9, false),
            (2 << 9, true),
            (17 << 9, true),
            (0x8000, true),
        ] {
            assert_eq!(stops_a_test(word), solid, "{word:#06x}");
        }
    }

    #[test]
    fn door_gaps_block_only_on_a_tower_floor() {
        // Attribute 2 (`$80:E11C`: `$8000`) blocks with `$048A & $8000`.
        let cells = vec![2 << 9; 16];
        for gaps in [false, true] {
            let layer = Layer {
                cells: &cells,
                width: 4,
                height: 4,
                gaps,
            };
            assert_eq!(step((24, 32), (-9, 0), BOX, &layer).1, (gaps, false));
        }
    }
}
