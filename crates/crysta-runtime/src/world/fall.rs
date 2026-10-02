//! Pits and falls (`docs/tower-three.md` §2): a cell of attribute `$14` is
//! a pit. When every cell under Ark's feet is one (`$80:CC6D`), he falls
//! (`$84:9F53`: sound `$12`, 8 frames, sound `$10`, 39 frames). Then the
//! exit under him decides: its conditional list, with flag `$1F` set,
//! takes him down a floor (`$10F` to `$114`); with none, he loses some life
//! and stands again where he last stood clear of the pits (`$84:9F9B`,
//! `$84:B803`).
//!
//! Not modelled: the teeter at a pit's edge (`$80:CCB4`), the falling and
//! landing poses, and the landing's drop from 256 pixels up (`$90:FA4E`).

use super::{Step, World, WorldError};
use crate::scene::Transfer;
use room_core::Direction;

/// The pit attribute.
const PIT: u16 = 0x14;
/// Frames of the fall, and the second sound's frame.
const FALL: u16 = 47;
const SECOND_SOUND: u16 = 8;
/// The fall's sounds (port 3).
const FALL_SOUNDS: [u8; 2] = [0x12, 0x10];
/// The flag a fall into an exit sets.
const FELL: u16 = 0x1F;

/// Ark falling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Fall {
    frame: u16,
}

/// The cells under Ark's feet at `(x, y)`: his ground box, x - 4..x + 3 and
/// y - 8..y - 1, over the cells it touches.
fn feet((x, y): (u16, u16)) -> Vec<(u16, u16)> {
    let (left, right) = (x.saturating_sub(4) / 16, (x + 3) / 16);
    let (top, bottom) = (y.saturating_sub(8) / 16, y.saturating_sub(1) / 16);
    (top..=bottom)
        .flat_map(|row| (left..=right).map(move |column| (column, row)))
        .collect()
}

impl World<'_> {
    /// Whether Ark is falling.
    #[must_use]
    pub const fn falling(&self) -> bool {
        self.fall.is_some()
    }

    /// The cell attribute at `(column, row)` of the room as it stands.
    fn attribute(&self, (column, row): (u16, u16)) -> Option<u16> {
        let at = self.base.index(column, row)?;
        Some((self.base.room.cells()[at] >> 9) & 0x1F)
    }

    /// The ground test after Ark's step: every cell under his feet a pit,
    /// he falls; none, the place is kept as the last safe one.
    pub(super) fn ground_test(&mut self) {
        if self.fall.is_some() || self.in_transition() {
            return;
        }
        let under: Vec<_> = feet(self.position())
            .into_iter()
            .map(|cell| self.attribute(cell))
            .collect();
        if !under.is_empty() && under.iter().all(|&attribute| attribute == Some(PIT)) {
            self.fall = Some(Fall { frame: 0 });
            self.thrust = None;
            self.globals.audio.sound_port3(FALL_SOUNDS[0]);
        } else if under.iter().all(|&attribute| attribute != Some(PIT)) {
            self.safe = Some((self.position(), self.facing));
        }
    }

    /// A frame of the fall: the world runs on, Ark held; at its end, down a
    /// floor or back to the last safe place.
    pub(super) fn fall_frame(&mut self) -> Result<Option<Step>, WorldError> {
        let Some(mut fall) = self.fall.take() else {
            return Ok(None);
        };
        fall.frame += 1;
        if fall.frame == SECOND_SOUND {
            self.globals.audio.sound_port3(FALL_SOUNDS[1]);
        }
        if fall.frame < FALL {
            self.fall = Some(fall);
        } else {
            self.land();
        }
        self.run_actors()?;
        Ok(Some(Step::Stayed))
    }

    /// The fall's end (`$8D:8756`): the exit under Ark, or his last safe
    /// place with some life lost.
    fn land(&mut self) {
        let (x, y) = self.position();
        let record = self
            .exits
            .select(x.wrapping_sub(8), y.wrapping_sub(16))
            .filter(|record| record.raw_destination() & 0x8000 != 0)
            .cloned();
        if let Some(record) = record {
            self.globals.write_flag(0x8000 | FELL);
            let events = &self.globals.events;
            let set =
                |flag| assets::maps::scripts::EventFlags::Bitmap(events).get(flag) == Some(true);
            if let Some(down) = record.conditional(self.image, set) {
                // The loader places Ark at the raw position plus (8,16).
                self.globals.transfer = Some(Transfer {
                    map: down.map,
                    position: (down.position.0 + 8, down.position.1 + 16),
                    mode: down.mode,
                });
                return;
            }
        }
        // `$84:D4F4`: a thirty-second of the most life, at least 4; Ark keeps
        // 1 (guesses). Natively only while `$048A` bit 15 is set (always here).
        let stats = self.globals.slot.stats();
        let lost = (stats.max_life / 32).max(4);
        self.globals
            .slot
            .set_life(stats.life.saturating_sub(lost).max(1));
        if let Some((safe, facing)) = self.safe {
            let settled = settle(safe, facing);
            let clear = feet(settled)
                .into_iter()
                .all(|cell| self.attribute(cell) != Some(PIT));
            let (x, y) = if clear { settled } else { safe };
            self.place(x, y);
            self.face(facing);
        }
    }
}

/// `$84:B803`: the last safe spot moved onto the cell grid, back from the
/// way Ark faced (toward the pit).
fn settle((x, y): (u16, u16), facing: Direction) -> (u16, u16) {
    let floor = |value: u16| value & !15;
    let ceil = |value: u16| (value + 15) & !15;
    match facing {
        Direction::Down => (x, floor(y - 16) + 16),
        Direction::Up => (x, ceil(y - 16) + 16),
        Direction::Left => (ceil(x - 8) + 8, y),
        Direction::Right => (floor(x - 8) + 8, y),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_safe_spot_settles_back_from_the_pit() {
        // Facing Down onto a pit at row 3: back up to the cell above.
        assert_eq!(settle((24, 58), Direction::Down), (24, 48));
        assert_eq!(settle((24, 50), Direction::Up), (24, 64));
        assert_eq!(settle((30, 48), Direction::Left), (40, 48));
        assert_eq!(settle((30, 48), Direction::Right), (24, 48));
        // On the grid already: no move.
        assert_eq!(settle((24, 48), Direction::Down), (24, 48));
    }

    #[test]
    fn the_feet_touch_one_to_four_cells() {
        assert_eq!(feet((24, 40)), [(1, 2)]);
        assert_eq!(feet((16, 40)), [(0, 2), (1, 2)]);
        assert_eq!(feet((24, 36)), [(1, 1), (1, 2)]);
    }
}
