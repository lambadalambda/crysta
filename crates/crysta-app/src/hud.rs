//! The towers' HUD (`docs/combat.md` §7, `docs/combat-graphics.md` §3): BG3
//! cells over the view, with the level, the life and the gems.

use crate::frame::{rgb, signed, Canvas, CLASSIC_WIDTH};
use assets::shop_display::ShopArt;

/// One BG3 cell: column, row, tile, palette.
pub type Cell = (u8, u8, u8, u8);

/// Labels and the frame: palette 2.
const LABELS: u8 = 2;
/// Digit tops and the gem icon: palette 3; digit bottoms: palette 4.
const TOPS: u8 = 3;
const BOTTOMS: u8 = 4;
/// Rows the screen shows (28 of 8 pixels).
const ROWS: u8 = 28;

/// The values the HUD shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shown {
    /// Ark's level.
    pub level: u8,
    /// His life and its most.
    pub life: (u16, u16),
    /// The gems he has.
    pub gems: u32,
}

/// A number's digits, most significant first, without leading zeros.
fn digits(value: u32) -> Vec<u8> {
    value
        .to_string()
        .bytes()
        .map(|digit| digit - b'0')
        .collect()
}

/// An 8x16 digit at a cell: its top over its bottom (`$85:ECB7`).
fn digit(cells: &mut Vec<Cell>, (column, row): (u8, u8), value: u8) {
    cells.push((column, row, ShopArt::TOP + value, TOPS));
    cells.push((column, row + 1, ShopArt::BOTTOM + value, BOTTOMS));
}

/// The HUD's cells for `shown`.
#[must_use]
pub fn cells(shown: Shown) -> Vec<Cell> {
    let mut cells = Vec::new();
    for row in 0..ROWS {
        cells.push((0, row, 0x20, LABELS));
        cells.push((31, row, 0x20, LABELS));
    }
    // "LEVEL", the box edge, "ITEM"; "LIFE".
    for (column, tile) in [
        (4, 0x3D),
        (5, 0x3E),
        (6, 0x2E),
        (7, 0x0C),
        (8, 0x0D),
        (24, 0x02),
        (25, 0x03),
    ] {
        cells.push((column, 1, tile, LABELS));
    }
    // Right-aligned level and life, a slash, the most life left-aligned.
    let right = |cells: &mut Vec<Cell>, value: u32, last: u8| {
        let shown = digits(value);
        for (index, &value) in shown.iter().enumerate() {
            let column = last + 1 + u8::try_from(index).unwrap_or(0)
                - u8::try_from(shown.len()).unwrap_or(0);
            digit(cells, (column, 2), value);
        }
    };
    right(&mut cells, u32::from(shown.level), 5);
    right(&mut cells, u32::from(shown.life.0), 24);
    cells.push((25, 2, 0x2F, TOPS));
    cells.push((25, 3, 0x3F, BOTTOMS));
    for (index, value) in digits(u32::from(shown.life.1)).into_iter().enumerate() {
        digit(
            &mut cells,
            (26 + u8::try_from(index).unwrap_or(0), 2),
            value,
        );
    }
    // The gem icon, then the gems: up to three end at column 28.
    for (column, row, tile) in [
        (24, 25, 0x40),
        (25, 25, 0x41),
        (24, 26, 0x50),
        (25, 26, 0x51),
    ] {
        cells.push((column, row, tile, TOPS));
    }
    let gems = digits(shown.gems.min(99_999));
    let start = if gems.len() <= 3 {
        29 - u8::try_from(gems.len()).unwrap_or(0)
    } else {
        26
    };
    for (index, value) in gems.into_iter().enumerate() {
        digit(
            &mut cells,
            (start + u8::try_from(index).unwrap_or(0), 25),
            value,
        );
    }
    cells
}

/// Draws the cells on the classic view's columns, centred in a wide one.
pub fn draw(canvas: &mut Canvas, art: &ShopArt, shown: Shown) {
    let left = (signed(canvas.width) - signed(CLASSIC_WIDTH)) / 2;
    for (column, row, tile, palette) in cells(shown) {
        let Some(pixels) = art.panel.get(usize::from(tile)) else {
            continue;
        };
        for (at, &index) in pixels.iter().enumerate() {
            if index == 0 {
                continue;
            }
            let colour = art.panel_palette[usize::from(palette) * 4 + usize::from(index)];
            let x = left + i32::from(column) * 8 + signed(at % 8);
            let y = i32::from(row) * 8 + signed(at / 8);
            canvas.set((x, y), rgb(colour));
        }
    }
}

/// Draws the damage digits floating over the bodies hit: 8x8 OBJ tiles
/// `$40 + d`, 7 pixels apart, centred on the body (`$85:E55C`). The
/// critical hit's and Ark's colours (palettes 5 and 3) are not modelled;
/// all use palette 4.
pub fn draw_digits(
    canvas: &mut Canvas,
    art: &ShopArt,
    camera: (i32, i32),
    digits: &[crysta_runtime::scene::Digits],
) {
    for shown in digits {
        let values = self::digits(u32::from(shown.amount));
        let count = i32::try_from(values.len()).unwrap_or(0);
        let x = i32::from(shown.at.0) - camera.0 - 3 * count;
        let y = i32::from(shown.at.1) - camera.1 - 1 + i32::from(shown.rise());
        for (index, value) in (0_i32..).zip(values) {
            let Some(tile) = art.sprites.get(usize::from(value)) else {
                continue;
            };
            for (at, &colour) in tile.pixels().iter().enumerate() {
                if colour != 0 {
                    let pixel = rgb(art.count_palette[usize::from(colour)]);
                    canvas.set((x + index * 7 + signed(at % 8), y + signed(at / 8)), pixel);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_life_and_gems_take_their_native_cells() {
        let cells = cells(Shown {
            level: 1,
            life: (8, 28),
            gems: 24,
        });
        let at = |column, row| {
            cells
                .iter()
                .find(|cell| (cell.0, cell.1) == (column, row))
                .map(|cell| cell.2)
        };
        // Level 1 at column 5; life " 8" ends at column 24; "/"; "28".
        assert_eq!(
            (at(5, 2), at(5, 3), at(4, 2)),
            (Some(0x22), Some(0x32), None)
        );
        assert_eq!((at(24, 2), at(23, 2)), (Some(0x29), None));
        assert_eq!(
            (at(25, 2), at(26, 2), at(27, 2)),
            (Some(0x2F), Some(0x23), Some(0x29))
        );
        // 24 gems end at column 28.
        assert_eq!((at(27, 25), at(28, 25)), (Some(0x23), Some(0x25)));
        assert_eq!((at(0, 10), at(31, 27)), (Some(0x20), Some(0x20)));
    }
}
