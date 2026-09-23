//! Wooden doors the player opens by hand (`$87:97CA`).
//!
//! Facing Up with A, the player's tile interaction (`$87:C7F1`) recognises a
//! door's lower cell by its low nine bits `$F3`, sampled at the player's
//! position less 24 pixels (`docs/house-navigation.md`). The door script then
//! waits 2 and 10 frames, patches the door half open (`$F5` below, `$F4`
//! above), waits 8, patches it open (`$F7`, `$F6`), and releases the player
//! once the last patch's 8-frame delay has passed (`TRB $097C`).
//!
//! The sound (`COP 37 1A`, `$87:97EC`) goes with the first patch.
//!
//! Not modelled: the player's poses (`COP CB`), the `$7F:1020` writes, and
//! `$87:C7F1`'s other branch, which with `$04F6` nonzero wants the raw word
//! `$00F3` (every door found is `$1CF3`; what sets `$04F6` is not traced).

use super::{World, WorldError};
use room_core::Direction;

/// A closed door's lower tile.
pub(super) const CLOSED_LOWER: u16 = 0xF3;

/// Frames after the press at which the door script patches: (frame, lower
/// tile, upper tile).
const PATCHES: [(u16, u16, u16); 2] = [(12, 0xF5, 0xF4), (20, 0xF7, 0xF6)];
/// Port 2's sound of a door opening.
const OPEN_SOUND: u8 = 0x1A;
/// The frame the player is released on.
const RELEASE: u16 = 28;

/// A door being opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Opening {
    /// The door's lower cell.
    pub(super) lower: (u16, u16),
    /// Frames since the press.
    frame: u16,
}

impl Opening {
    /// The door whose lower cell is `lower`, just pressed.
    pub(super) const fn new(lower: (u16, u16)) -> Self {
        Self { lower, frame: 0 }
    }

    /// One frame: the patches it makes, as (column, row, tile), and whether
    /// the player is free again.
    pub(super) fn tick(&mut self) -> (Vec<(u16, u16, u16)>, bool) {
        self.frame += 1;
        let (column, row) = self.lower;
        let patches = PATCHES
            .iter()
            .filter(|&&(frame, _, _)| frame == self.frame)
            .flat_map(|&(_, lower, upper)| [(column, row, lower), (column, row - 1, upper)])
            .collect();
        (patches, self.frame >= RELEASE)
    }
}

/// The cell a player facing Up at `(x, y)` samples for a door.
pub(super) const fn sample((x, y): (u16, u16)) -> (u16, u16) {
    (x / 16, y.saturating_sub(24) / 16)
}

impl World<'_> {
    /// Starts opening the wooden door the player faces Up at, if there is
    /// one. Returns whether it did.
    pub(super) fn open_door(&mut self) -> bool {
        // `$87:C7F1` acts only on a sample aligned to a cell's top:
        // `(sample_y - 8) & $F == 0`, the player's y a multiple of 16.
        if self.facing != Direction::Up
            || self.arrival.is_some()
            || !self.position().1.is_multiple_of(16)
        {
            return false;
        }
        let (column, row) = sample(self.position());
        let width = usize::from(self.base.width);
        let closed = row > 0
            && column < self.base.width
            && self
                .base
                .room
                .cells()
                .get(usize::from(row) * width + usize::from(column))
                .is_some_and(|word| word & 0x1FF == CLOSED_LOWER);
        if closed {
            self.opening = Some(Opening::new((column, row)));
        }
        closed
    }

    /// A frame of the door script: its patches, the player held, and the
    /// world running on.
    pub(super) fn door_frame(&mut self) -> Result<(), WorldError> {
        let Some(mut opening) = self.opening else {
            return Ok(());
        };
        let (patches, free) = opening.tick();
        if patches
            .first()
            .is_some_and(|&(_, _, tile)| tile == PATCHES[0].1)
        {
            self.globals.audio.sound_port2(OPEN_SOUND);
        }
        self.globals.patches.extend(patches);
        self.opening = (!free).then_some(opening);
        self.apply_patches()?;
        self.run_actors()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_door_opens_halfway_then_fully_and_frees_the_player_after() {
        let mut opening = Opening::new((4, 3));
        let mut log = Vec::new();
        for frame in 1..=30 {
            let (patches, free) = opening.tick();
            if !patches.is_empty() {
                log.push((frame, patches));
            }
            assert_eq!(free, frame >= 28, "frame {frame}");
        }
        assert_eq!(
            log,
            [
                (12, vec![(4, 3, 0xF5), (4, 2, 0xF4)]),
                (20, vec![(4, 3, 0xF7), (4, 2, 0xF6)]),
            ]
        );
    }

    #[test]
    fn the_sample_is_the_cell_24_pixels_above_the_feet() {
        assert_eq!(sample((72, 80)), (4, 3));
        assert_eq!(sample((136, 352)), (8, 20), "C's door from its anchor");
    }
}
