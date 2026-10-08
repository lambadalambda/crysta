//! Pits, ropes and falls (`docs/tower-three.md` §2, `docs/tower-four.md`
//! §2): a cell of attribute `$14` is a pit, one of `$12` a rope. Ark's
//! ground test (`$80:CC00`) samples the cells under his 16-pixel box from
//! (x - 8, y - 16). All pits: he falls (`$84:9F53`: sound `$12`, 8 frames,
//! sound `$10`, 39 frames). Pits and rope, the top-left a pit and the box's
//! top 7 to 13 pixels into its cell (`$80:CCB4`): he is on the rope; else
//! he falls. Then the exit under him decides: its conditional list, with
//! flag `$1F` set, takes him down a floor (`$10F` to `$114`); with none, he
//! loses some life and stands again where he last stood safe (`$84:9F9B`,
//! `$84:B803`).
//!
//! On the rope (`$84:9C95`) only Left and Right walk; Up or Down leans
//! (`$84:9BFE`, `9C44`): 16 frames of the lean, then 60 in which the way
//! held decides (`COP 2B`): the same way falls, the opposite recovers; at
//! the end, 4 frames of wobble (`$84:9BD9`) and the fall.
//!
//! Not modelled: the falling, rope and landing poses, the landing's drop
//! from 256 pixels up (`$90:FA4E`), and a hit's lean on the rope.
//! Tracked: `meta/issues/ark-underworld-poses.md`,
//! `meta/issues/lip-jumps.md`, `meta/issues/fall-damage-teeter.md`.

use super::{Step, World, WorldError};
use crate::scene::Transfer;
use room_core::Direction;

/// The pit and rope attributes, and the one that keeps the safe spot
/// (`$80:CCF9`).
const PIT: u16 = 0x14;
const ROPE: u16 = 0x12;
const UNSAFE: u16 = 0x13;
/// The lip (`$80:CF30`): Ark on one jumps down (`$80:CF50`, the Hole's
/// rim, `docs/underworld-end.md` §2); 2 pixels a frame for 32 frames, into
/// the Hole's exit (guess), sound `$10`.
const LIP: u16 = 0x08;
const HOLE: u16 = 0x0127;
const JUMP: u16 = 32;
const JUMP_SOUND: u8 = 0x10;
/// The rope's band: the box's top this far into its cell.
const BAND: std::ops::RangeInclusive<u16> = 7..=13;
/// Frames of the fall, and the second sound's frame.
const FALL: u16 = 47;
const SECOND_SOUND: u16 = 8;
/// The fall's sounds (port 3).
const FALL_SOUNDS: [u8; 2] = [0x12, 0x10];
/// The flag a fall into an exit sets.
const FELL: u16 = 0x1F;
/// A lean's frames: the pose, the window for the pad, the wobble.
const LEAN_POSE: u16 = 16;
const LEAN_WINDOW: u16 = LEAN_POSE + 60;
const LEAN_END: u16 = LEAN_WINDOW + 4;

/// Ark falling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Fall {
    frame: u16,
}

impl Fall {
    /// Whether the fall has ended this frame: Ark is hidden.
    pub(super) const fn landed(self) -> bool {
        self.frame >= FALL
    }
}

/// Ark jumping down from a lip: frames into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Jump {
    frame: u16,
}

/// Ark on a rope: leaning one way, and for how long.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Rope {
    lean: Option<(Direction, u16)>,
}

/// The four cells `$80:CC00` samples for Ark at `(x, y)`: the box's top
/// left, the next column when the box is off the grid across, the next row
/// when it is off the grid down, and the cell diagonal to it.
fn samples((x, y): (u16, u16)) -> [(u16, u16); 4] {
    let (left, top) = (x.wrapping_sub(8), y.wrapping_sub(16));
    let (column, row) = (left / 16, top / 16);
    let right = column + u16::from(left % 16 != 0);
    let below = row + u16::from(top % 16 != 0);
    [(column, row), (right, row), (column, below), (right, below)]
}

/// What the ground under Ark does (`$80:CC6D`..`CCF0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ground {
    Solid,
    Rope,
    Fall,
}

fn ground(attributes: [Option<u16>; 4], y: u16) -> Ground {
    let hole = |attribute| matches!(attribute, Some(PIT | ROPE));
    if attributes.iter().all(|&attribute| attribute == Some(PIT)) {
        Ground::Fall
    } else if !attributes.into_iter().all(hole) {
        Ground::Solid
    } else if attributes[0] == Some(PIT) && BAND.contains(&(y.wrapping_sub(16) % 16)) {
        Ground::Rope
    } else {
        Ground::Fall
    }
}

impl World<'_> {
    /// Whether Ark is falling.
    #[must_use]
    pub const fn falling(&self) -> bool {
        self.fall.is_some()
    }

    /// Whether Ark is on a rope.
    #[must_use]
    pub const fn on_rope(&self) -> bool {
        self.rope.is_some()
    }

    /// The cell attribute at `(column, row)` of the room as it stands.
    fn attribute(&self, (column, row): (u16, u16)) -> Option<u16> {
        let at = self.base.index(column, row)?;
        Some((self.base.room.cells()[at] >> 9) & 0x1F)
    }

    /// The ground test after Ark's step: a fall, the rope, or solid ground;
    /// a place without `$13` under it is kept as the last safe one.
    pub(super) fn ground_test(&mut self) {
        if self.fall.is_some() || self.in_transition() {
            return;
        }
        let at = self.position();
        let under = samples(at).map(|cell| self.attribute(cell));
        match ground(under, at.1) {
            Ground::Fall => return self.start_fall(),
            Ground::Rope => self.rope = Some(self.rope.unwrap_or_default()),
            Ground::Solid => self.rope = None,
        }
        if !under.contains(&Some(UNSAFE)) {
            self.safe = Some((at, self.facing));
        }
    }

    fn start_fall(&mut self) {
        self.fall = Some(Fall { frame: 0 });
        self.rope = None;
        self.thrust = None;
        self.globals.audio.sound_port3(FALL_SOUNDS[0]);
    }

    /// A step down refused by a lip (attribute 8, which the walker admits
    /// only on its directional candidate): Ark jumps from it, as `$80:CF50`
    /// does once he stands on it.
    pub(super) fn lip_test(&mut self, step: Step) {
        // Only the Hole's rim is measured (tower 4's lips are not).
        let refused = matches!(
            step,
            Step::Refused(room_core::Unqualified::UnsupportedType(8))
        );
        if !refused || self.facing != Direction::Down || self.map != HOLE {
            return;
        }
        let (x, y) = self.position();
        let below = samples((x, y + 16)).map(|cell| self.attribute(cell));
        if below[..2] == [Some(LIP); 2] {
            self.jump = Some(Jump { frame: 0 });
            self.globals.audio.sound_port3(JUMP_SOUND);
        }
    }

    /// A frame of a jump from a lip: Ark drops, the world runs on; the
    /// exit under him is taken once he stands again.
    pub(super) fn jump_frame(&mut self) -> Result<Option<Step>, WorldError> {
        let Some(mut jump) = self.jump.take() else {
            return Ok(None);
        };
        let (x, y) = self.position();
        self.walking = room_core::WalkingState::new(x, y + 2);
        jump.frame += 1;
        if jump.frame < JUMP {
            self.jump = Some(jump);
        }
        self.run_actors()?;
        Ok(Some(Step::Walked))
    }

    /// The pad on a rope: Left and Right walk; Up or Down leans. Returns
    /// the way Ark walks.
    pub(super) fn rope_step(&mut self, direction: Option<Direction>) -> Option<Direction> {
        let Some(mut rope) = self.rope else {
            return direction;
        };
        let walked = match (rope.lean, direction) {
            (Some((way, frames)), held) => {
                let window = (LEAN_POSE..LEAN_WINDOW).contains(&frames);
                if window && held == Some(way.opposite()) {
                    rope.lean = None;
                } else if (window && held == Some(way)) || frames + 1 >= LEAN_END {
                    self.start_fall();
                    return None;
                } else {
                    rope.lean = Some((way, frames + 1));
                }
                None
            }
            (None, Some(way @ (Direction::Up | Direction::Down))) => {
                rope.lean = Some((way, 0));
                None
            }
            (None, walk) => walk,
        };
        self.rope = Some(rope);
        walked
    }

    /// A frame of the fall: the world runs on, Ark held; at its end, down a
    /// floor or back to the last safe place, hidden for that frame
    /// (`$84:9FD8`, `COP BC`); the next he is back (`$84:9FE6`).
    pub(super) fn fall_frame(&mut self) -> Result<Option<Step>, WorldError> {
        let Some(mut fall) = self.fall.take() else {
            return Ok(None);
        };
        if fall.landed() {
            return Ok(None);
        }
        fall.frame += 1;
        if fall.frame == SECOND_SOUND {
            self.globals.audio.sound_port3(FALL_SOUNDS[1]);
        }
        self.fall = Some(fall);
        if fall.landed() {
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
        // Only on a tower floor (`$84:9F9B`).
        if self.globals.tower_floor() {
            self.fall_cost();
        }
        if let Some((safe, facing)) = self.safe {
            let settled = settle(safe, facing);
            let under = samples(settled).map(|cell| self.attribute(cell));
            let (x, y) = if ground(under, settled.1) == Ground::Fall {
                safe
            } else {
                settled
            };
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
    fn the_samples_cover_the_box_from_its_top_left() {
        assert_eq!(samples((24, 48)), [(1, 2); 4]);
        assert_eq!(samples((30, 48)), [(1, 2), (2, 2), (1, 2), (2, 2)]);
        assert_eq!(samples((24, 50)), [(1, 2), (1, 2), (1, 3), (1, 3)]);
    }

    #[test]
    fn a_rope_holds_ark_only_in_its_band() {
        let (pit, rope, floor) = (Some(PIT), Some(ROPE), Some(0));
        // The box's top 10 pixels into the pit row above the rope.
        assert_eq!(
            ground([pit, pit, rope, rope], 16 * 44 + 10 + 16),
            Ground::Rope
        );
        assert_eq!(
            ground([pit, pit, rope, rope], 16 * 44 + 3 + 16),
            Ground::Fall
        );
        assert_eq!(
            ground([rope, rope, pit, pit], 16 * 45 + 10 + 16),
            Ground::Fall
        );
        assert_eq!(ground([pit; 4], 0), Ground::Fall);
        assert_eq!(ground([pit, floor, pit, pit], 0), Ground::Solid);
    }
}
