//! Ark's jump with B from standing or walking (`$84:96DE`/`$84:9710`,
//! `docs/jump.md`): 3 frames of crouch, 24 in the air on the height stream
//! `$39`, steered a pixel then two after each pad read; from the landing
//! he walks on, and the walk's ground test runs. A pit or a lip under him
//! in the air takes nothing.

use super::pose::{by_facing, ArkPose};
use super::{facing_delta, Step, World, WorldError};
use room_core::Direction;

/// Frames of crouch before the air (`COP C1 02` and the frame after).
const CROUCH: u16 = 3;
/// The heights in the air (`$7F:668A`, stream `$39`), from `J + 3`.
const HEIGHTS: [i16; 24] = [
    -5, -10, -13, -16, -19, -21, -23, -25, -26, -27, -28, -28, -28, -27, -26, -25, -23, -21, -19,
    -16, -13, -10, -5, 0,
];
/// The frame he lands on, standing (`J + 27`).
const LANDED: u16 = CROUCH + 24;
/// Resource 3's lists by facing (Down, Up, Right), each phase.
const CROUCHING: [u8; 3] = [0, 3, 6];
const RISING: [u8; 3] = [1, 4, 7];
const FALLING: [u8; 3] = [2, 5, 8];
/// Frames in the rising list (records 8 + 4).
const RISE: u16 = 12;
/// Sounds on port 3: the leap (`J + 4`) and the landing (`J + 28`).
const LEAP_SOUND: u8 = 0x0E;
const LAND_SOUND: u8 = 0x0F;
/// Hits pass under him from this height up (`$85:F856`).
const OVER_HITS: i16 = -16;

/// A jump under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Jump {
    /// Frames since B was pressed; the crouch begins the frame after
    /// (`J`, frame 1).
    frame: u16,
    facing: Direction,
    /// The way the last pad read steers, and whether its second, two-pixel
    /// frame is due.
    steer: Option<(Direction, bool)>,
    /// B pressed again in the air: the next jump at the landing.
    again: bool,
}

impl Jump {
    pub(super) const fn new(facing: Direction) -> Self {
        Self {
            frame: 0,
            facing,
            steer: None,
            again: false,
        }
    }

    /// Frames since `J`; `None` on the press's own frame.
    const fn since(self) -> Option<u16> {
        self.frame.checked_sub(1)
    }

    /// Whether he is off the ground.
    const fn in_air(self) -> bool {
        matches!(self.since(), Some(since) if since >= CROUCH && since < LANDED)
    }

    /// How far above his place he is drawn (`$0970`, `$80:F0BA`).
    pub(super) const fn height(self) -> i16 {
        match self.since() {
            Some(since) if self.in_air() => HEIGHTS[(since - CROUCH) as usize],
            _ => 0,
        }
    }

    /// Whether the enemies' hits pass under him.
    pub(super) const fn over_hits(self) -> bool {
        self.height() <= OVER_HITS
    }

    /// Resource 3's crouch, rise or fall by facing; standing on landing.
    pub(super) const fn pose(self) -> Option<ArkPose> {
        let Some(since) = self.since() else {
            return None;
        };
        let (lists, age) = if since < CROUCH {
            (CROUCHING, since)
        } else if since < CROUCH + RISE {
            (RISING, since - CROUCH)
        } else if since < LANDED {
            (FALLING, since - CROUCH - RISE)
        } else {
            return None;
        };
        Some(by_facing(3, lists, self.facing, age))
    }

    /// This frame's move: on a pad read (`$0042` even) the way held starts
    /// a pixel, the next frame two more (`$84:98D2`).
    fn step(&mut self, held: Option<Direction>, read: bool) -> Option<(Direction, i16)> {
        if !self.in_air() {
            return None;
        }
        if read {
            self.steer = held.map(|way| (way, true));
            return held.map(|way| (way, 1));
        }
        let (way, due) = self.steer?;
        self.steer = Some((way, false));
        due.then_some((way, 2))
    }
}

impl World<'_> {
    /// B starts a jump from standing or walking: not on a rope, with a
    /// pot, in a dash, a thrust, a hit, an arrival or on a world map.
    pub(super) fn start_jump(&mut self) {
        if self.jump.is_none()
            && self.plane.is_none()
            && self.arrival.is_none()
            && self.rope.is_none()
            && self.carry().is_none()
            && self.run.is_none()
            && self.thrust.is_none()
            && self.hurt.is_none()
        {
            self.jump = Some(Jump::new(self.facing));
        }
    }

    /// A frame of the jump. Returns `None` once it is over, so the frame
    /// walks on.
    pub(super) fn jump_frame(
        &mut self,
        held: Option<Direction>,
        again: bool,
    ) -> Result<Option<Step>, WorldError> {
        let Some(mut jump) = self.jump.take() else {
            return Ok(None);
        };
        jump.frame += 1;
        let since = jump.frame - 1;
        if since >= LANDED {
            return self.jump_landed(jump, held);
        }
        // B pressed in the air; natively it must still be held at the
        // landing (`$0454`), which a press alone cannot tell.
        jump.again |= again && jump.in_air();
        if since == CROUCH + 1 {
            self.globals.audio.sound_port3(LEAP_SOUND);
        }
        let read = self.globals.frames.is_multiple_of(2);
        if let Some((way, pixels)) = jump.step(held, read) {
            let (dx, dy) = facing_delta(way);
            self.push_by((dx * pixels, dy * pixels));
        }
        if let Some(step) = self.take_exit()? {
            return Ok(Some(step));
        }
        self.jump = Some(jump);
        self.run_actors()?;
        // A hit ends the jump; high in the air they pass under him.
        if !jump.over_hits() {
            self.touch();
        }
        self.hurt_ark();
        if self.hurt.is_some() {
            self.jump = None;
        }
        Ok(Some(Step::Stayed))
    }

    /// From the landing (`J + 27`) Ark walks on and the walk's ground test
    /// runs (a pit, a lip or a rope takes him); B pressed in the air
    /// crouches again at once (`$84:98AB`). The landing's sound comes a
    /// frame later with a direction held.
    fn jump_landed(
        &mut self,
        jump: Jump,
        held: Option<Direction>,
    ) -> Result<Option<Step>, WorldError> {
        let since = jump.frame - 1;
        if since == LANDED && jump.again {
            self.jump = Some(Jump {
                frame: 1,
                ..Jump::new(self.facing)
            });
            self.run_actors()?;
            return Ok(Some(Step::Stayed));
        }
        let sound = LANDED + if held.is_some() { 2 } else { 1 };
        if since >= sound {
            self.globals.audio.sound_port3(LAND_SOUND);
        } else {
            self.jump = Some(jump);
        }
        Ok(None)
    }

    /// How far above his place Ark is drawn while he jumps.
    pub(super) fn jump_height(&self) -> i16 {
        self.jump.map_or(0, Jump::height)
    }

    /// Whether the enemies' hits pass under Ark.
    pub(super) fn over_hits(&self) -> bool {
        self.jump.is_some_and(Jump::over_hits)
    }

    /// Ark's pose while he jumps.
    pub(super) fn jump_pose(&self) -> Option<ArkPose> {
        self.jump.and_then(Jump::pose)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_jump_crouches_rises_to_28_and_lands_on_frame_27() {
        // `J` is frame 1, the frame after the press.
        let at = |since| Jump {
            frame: since + 1,
            ..Jump::new(Direction::Down)
        };
        assert_eq!(Jump::new(Direction::Down).pose(), None);
        let heights: Vec<i16> = (0..=LANDED).map(|since| at(since).height()).collect();
        assert_eq!(&heights[..4], [0, 0, 0, -5]);
        assert_eq!(heights.iter().min(), Some(&-28));
        assert_eq!(&heights[25..], [-5, 0, 0]);
        let list = |frame| at(frame).pose().map(|pose| (pose.list, pose.age));
        assert_eq!(
            [list(2), list(3), list(15), list(26)],
            [Some((0, 2)), Some((1, 0)), Some((2, 0)), Some((2, 11))]
        );
        assert_eq!(list(LANDED), None);
        assert!(!at(5).over_hits() && at(6).over_hits() && at(22).over_hits());
        assert!(!at(23).over_hits());
    }

    #[test]
    fn a_read_steers_a_pixel_then_two() {
        let mut jump = Jump {
            frame: CROUCH + 1,
            ..Jump::new(Direction::Right)
        };
        let moves: Vec<_> = [true, false, true, false, false]
            .into_iter()
            .map(|read| {
                jump.step(Some(Direction::Left), read)
                    .map(|(_, pixels)| pixels)
            })
            .collect();
        assert_eq!(moves, [Some(1), Some(2), Some(1), Some(2), None]);
        let mut crouching = Jump::new(Direction::Right);
        assert_eq!(crouching.step(Some(Direction::Left), true), None);
    }
}
