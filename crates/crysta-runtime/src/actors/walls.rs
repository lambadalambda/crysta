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
}

impl Layer<'_> {
    /// Whether the cell at (column, row) stops an enemy: every attribute
    /// but 0, 1, 17 and 22 (`$80:E11C`). Attribute 2, the door gaps, blocks
    /// only with `$048A & $8000`, which the towers set; it blocks here.
    /// Cells off the map block.
    fn blocks(&self, column: i32, row: i32) -> bool {
        let (Ok(column), Ok(row)) = (u16::try_from(column), u16::try_from(row)) else {
            return true;
        };
        if column >= self.width || row >= self.height {
            return true;
        }
        let at = usize::from(row) * usize::from(self.width) + usize::from(column);
        self.cells
            .get(at)
            .is_none_or(|&word| !matches!((word >> 9) & 0x1F, 0 | 1 | 17 | 22))
    }
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
            };
            assert_eq!(step((24, 32), (-9, 0), BOX, &layer).1, (false, false));
        }
    }
}
