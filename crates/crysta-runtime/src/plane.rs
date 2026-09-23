//! Walking on a world map's plane (`$84:DEE0`): the underworld `$03`.
//!
//! Not the room walker. The player steps from cell to cell, 16 pixels at
//! 2 pixels a frame; a step starts only from a cell-aligned position
//! (x = 8, y = 0 modulo 16) and finishes after the pad is released. The cell
//! a step enters is probed (`$80:C164`/`C21D`/`C2CE`/`C387`): a byte of `$A0`
//! or more blocks (`$80:C469`), and a blocked step slides sideways when
//! exactly one side is open, beside and diagonally ahead (`$80:C203`/`C2B4`
//! count them). Positions wrap at the plane's edges (`$8D:8D6D`). Measured
//! on the native route: 60 of 60 step starts. Own-cell tiles `$88..$8F`
//! (`$80:C440`) are not modelled; `$03` has none.

use room_core::Direction;

/// The first byte that blocks a step.
const BLOCKING: u8 = 0xA0;
/// Pixels a step covers, and a frame of it.
const STEP: u16 = 16;
const SPEED: u16 = 2;
/// Frames of the arrival's walk down from the exit's anchor.
const ARRIVAL: u16 = 16;

/// A player on the plane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plane {
    cells: Vec<u8>,
    width: u16,
    height: u16,
    /// The step under way and the pixels it has left.
    step: Option<(Direction, u16)>,
    /// Frames of the arrival walk left.
    arriving: u16,
}

impl Plane {
    /// A plane of `width` x `height` cells, the player arriving.
    #[must_use]
    pub fn new(cells: Vec<u8>, width: u16, height: u16) -> Self {
        Self {
            cells,
            width,
            height,
            step: None,
            arriving: ARRIVAL,
        }
    }

    /// Whether the player cannot be steered: arriving or mid-step.
    #[must_use]
    pub const fn busy(&self) -> bool {
        self.arriving > 0 || self.step.is_some()
    }

    /// The cell's byte; the plane repeats.
    fn cell(&self, column: i32, row: i32) -> u8 {
        let (width, height) = (i32::from(self.width), i32::from(self.height));
        let index = row.rem_euclid(height) * width + column.rem_euclid(width);
        usize::try_from(index)
            .ok()
            .and_then(|index| self.cells.get(index).copied())
            .unwrap_or(BLOCKING)
    }

    fn open(&self, (column, row): (i32, i32)) -> bool {
        self.cell(column, row) < BLOCKING
    }

    /// One frame from `position` with the pad's direction. Returns the new
    /// position.
    pub fn tick(&mut self, (x, y): (u16, u16), input: Option<Direction>) -> (u16, u16) {
        if self.arriving > 0 {
            self.arriving -= 1;
            return (x, y + 1);
        }
        if self.step.is_none() {
            self.step = input
                .filter(|_| x % STEP == 8 && y.is_multiple_of(STEP))
                .and_then(|direction| self.choose((x, y), direction))
                .map(|direction| (direction, STEP));
        }
        let Some((direction, left)) = self.step else {
            return (x, y);
        };
        self.step = (left > SPEED).then_some((direction, left - SPEED));
        let (dx, dy) = delta(direction);
        let wrap = |at: u16, by: i16, cells: u16| {
            let extent = i32::from(cells) * i32::from(STEP);
            let moved = (i32::from(at) + i32::from(by) * i32::from(SPEED)).rem_euclid(extent);
            u16::try_from(moved).unwrap_or(0)
        };
        (wrap(x, dx, self.width), wrap(y, dy, self.height))
    }

    /// The step taken for `direction`: it, or a slide around a blocked cell.
    fn choose(&self, (x, y): (u16, u16), direction: Direction) -> Option<Direction> {
        let own = (
            (i32::from(x) - 8).div_euclid(i32::from(STEP)),
            (i32::from(y) - 16).div_euclid(i32::from(STEP)),
        );
        let (dx, dy) = delta(direction);
        let ahead = (own.0 + i32::from(dx), own.1 + i32::from(dy));
        if self.open(ahead) {
            return Some(direction);
        }
        let sides = if dx == 0 {
            [Direction::Left, Direction::Right]
        } else {
            [Direction::Up, Direction::Down]
        };
        let open: Vec<Direction> = sides
            .into_iter()
            .filter(|&side| {
                let (sx, sy) = delta(side);
                let (sx, sy) = (i32::from(sx), i32::from(sy));
                self.open((own.0 + sx, own.1 + sy)) && self.open((ahead.0 + sx, ahead.1 + sy))
            })
            .collect();
        // Both open, or neither: no slide.
        match open[..] {
            [side] => Some(side),
            _ => None,
        }
    }
}

const fn delta(direction: Direction) -> (i16, i16) {
    match direction {
        Direction::Up => (0, -1),
        Direction::Down => (0, 1),
        Direction::Left => (-1, 0),
        Direction::Right => (1, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An open 8x8 plane with a wall at (4,3) and at (3,4), (4,4) below it.
    fn plane() -> Plane {
        let mut cells = vec![0; 64];
        for (column, row) in [(4, 3), (3, 4), (4, 4)] {
            cells[row * 8 + column] = 0xC1;
        }
        let mut plane = Plane::new(cells, 8, 8);
        plane.arriving = 0;
        plane
    }

    #[test]
    fn a_step_covers_a_cell_at_two_pixels_a_frame_and_finishes_after_release() {
        let mut plane = plane();
        let mut at = (24, 32);
        at = plane.tick(at, Some(Direction::Right));
        assert_eq!(at, (26, 32));
        for _ in 0..7 {
            at = plane.tick(at, None);
        }
        assert_eq!(at, (40, 32), "one cell");
        assert_eq!(plane.tick(at, None), (40, 32), "then stands");
    }

    #[test]
    fn a_blocked_step_slides_only_toward_the_one_open_side() {
        // From (3,3), Right into the wall at (4,3) is blocked; above is
        // open, below walled, so it slides Up.
        let mut walled = plane();
        let at = (3 * 16 + 8, 3 * 16 + 16);
        assert_eq!(walled.tick(at, Some(Direction::Right)), (56, 62));
        // With both sides open it stands.
        let mut open = plane();
        open.cells[4 * 8 + 3] = 0;
        open.cells[4 * 8 + 4] = 0;
        assert_eq!(open.tick(at, Some(Direction::Right)), at);
    }

    #[test]
    fn positions_wrap_at_the_planes_edges() {
        let mut plane = Plane::new(vec![0; 64], 8, 8);
        plane.arriving = 0;
        let at = plane.tick((8, 0), Some(Direction::Up));
        assert_eq!(at, (8, 126), "8 cells of 16 pixels");
    }

    #[test]
    fn the_arrival_walks_down_sixteen_pixels_first() {
        let mut plane = Plane::new(vec![0; 64], 8, 8);
        let mut at = (40, 32);
        for _ in 0..16 {
            at = plane.tick(at, Some(Direction::Left));
        }
        assert_eq!(at, (40, 48));
        assert!(!plane.busy());
    }
}
