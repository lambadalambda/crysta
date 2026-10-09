//! Ark's jump with B (`docs/jump.md`). From standing or walking
//! (`$84:96DE`/`$84:9710`): 3 frames of crouch, 24 in the air on the height
//! stream `$39`, steered a pixel then two after each pad read; from the
//! landing he walks on, and the walk's ground test runs. From a dash
//! (`$84:99B1`..): no crouch, the stream `$3A`, the dash runs on unsteered,
//! and it lands in the dash's release grace. A pit or a lip under him in
//! the air takes nothing.

use super::pose::{by_facing, ArkPose};
use super::{facing_delta, Step, World, WorldError};
use room_core::run::Run;
use room_core::{Direction, FrameInput};

/// Sounds on port 3: the leap and the landing.
const LEAP_SOUND: u8 = 0x0E;
const LAND_SOUND: u8 = 0x0F;
/// Hits pass under him from this height up (`$85:F856`).
const OVER_HITS: i16 = -16;
/// Frames in the rising list (records 8 + 4, or 7 + 5).
const RISE: u16 = 12;
/// Frames in the air, both kinds.
const AIR: u16 = 24;

/// What a kind of jump does, frame by frame from `J`.
struct Profile {
    /// Frames of crouch before the air.
    crouch: u16,
    /// The heights in the air.
    heights: [i16; AIR as usize],
    /// Resource 3's crouch, rise and fall lists by facing (Down, Up,
    /// Right).
    lists: [[u8; 3]; 3],
    /// The frame of the leap's sound.
    leap: u16,
    /// Whether it lands into the dash's release grace, with its pose and
    /// no sound; else it sounds `$0F`.
    graced: bool,
}

/// The ground jump (`$84:96DE`): `COP C1 02` and the frame after, then the
/// stream `$39` (`$7F:668A`).
const GROUND: Profile = Profile {
    crouch: 3,
    heights: [
        -5, -10, -13, -16, -19, -21, -23, -25, -26, -27, -28, -28, -28, -27, -26, -25, -23, -21,
        -19, -16, -13, -10, -5, 0,
    ],
    lists: [[0, 3, 6], [1, 4, 7], [2, 5, 8]],
    leap: 4,
    graced: false,
};

/// The dash jump (`$84:99B1`): the stream `$3A`, then a frame at 0.
const DASH: Profile = Profile {
    crouch: 0,
    heights: [
        -4, -8, -11, -14, -16, -18, -20, -21, -22, -23, -23, -23, -23, -22, -21, -20, -18, -16,
        -14, -11, -8, -4, 0, 0,
    ],
    // No crouch: the first row is not shown.
    lists: [[0x09, 0x0B, 0x0D], [0x09, 0x0B, 0x0D], [0x0A, 0x0C, 0x0E]],
    leap: 1,
    graced: true,
};

/// After a dash jump's landing, the grace shows resource 1's lists (`$26`
/// Down, `$27` Up, `$28` Right) from `J + 25` to `J + 31`.
const GRACE_LISTS: [u8; 3] = [0x26, 0x27, 0x28];
const GRACE_POSE: u16 = 25;
const GRACE_END: u16 = 32;

/// A kind of jump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Ground,
    Dash,
}

impl Kind {
    const fn profile(self) -> &'static Profile {
        match self {
            Self::Ground => &GROUND,
            Self::Dash => &DASH,
        }
    }
}

/// A jump under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Jump {
    /// Frames since B was pressed; `J` is frame 1.
    frame: u16,
    facing: Direction,
    kind: Kind,
    /// The way the last pad read steers, and whether its second, two-pixel
    /// frame is due.
    steer: Option<(Direction, bool)>,
    /// B pressed again in the air: the next jump at the landing.
    again: bool,
}

impl Jump {
    const fn new(facing: Direction, kind: Kind) -> Self {
        Self {
            frame: 0,
            facing,
            kind,
            steer: None,
            again: false,
        }
    }

    /// Frames since `J`; `None` on the press's own frame.
    const fn since(self) -> Option<u16> {
        self.frame.checked_sub(1)
    }

    /// The frame he lands on.
    const fn landed(self) -> u16 {
        self.kind.profile().crouch + AIR
    }

    /// Whether he is off the ground.
    const fn in_air(self) -> bool {
        let crouch = self.kind.profile().crouch;
        matches!(self.since(), Some(since) if since >= crouch && since < self.landed())
    }

    /// How far above his place he is drawn (`$0970`, `$80:F0BA`).
    pub(super) const fn height(self) -> i16 {
        let profile = self.kind.profile();
        match self.since() {
            Some(since) if self.in_air() => profile.heights[(since - profile.crouch) as usize],
            _ => 0,
        }
    }

    /// Whether the enemies' hits pass under him.
    pub(super) const fn over_hits(self) -> bool {
        self.height() <= OVER_HITS
    }

    /// Resource 3's crouch, rise or fall by facing; after a dash jump the
    /// fall's last record, then the grace's list.
    pub(super) const fn pose(self) -> Option<ArkPose> {
        let Some(since) = self.since() else {
            return None;
        };
        let profile = self.kind.profile();
        let crouch = profile.crouch;
        let (lists, age) = if since < crouch {
            (profile.lists[0], since)
        } else if since < crouch + RISE {
            (profile.lists[1], since - crouch)
        } else if since < self.landed() || (profile.graced && since < GRACE_POSE) {
            (profile.lists[2], since - crouch - RISE)
        } else if profile.graced && since < GRACE_END {
            return Some(by_facing(1, GRACE_LISTS, self.facing, since - GRACE_POSE));
        } else {
            return None;
        };
        Some(by_facing(3, lists, self.facing, age))
    }

    /// Whether the pose still shows after the landing.
    const fn showing(self) -> bool {
        let end = if self.kind.profile().graced {
            GRACE_END
        } else {
            self.landed()
        };
        matches!(self.since(), Some(since) if since < end)
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
    /// B starts a jump from standing, walking or a dash: not braking, on a
    /// rope, with a pot, in a thrust, a hit, an arrival or on a world map.
    /// A dash jump's grace takes a new one (natively B with the dash's way).
    pub(super) fn start_jump(&mut self) {
        let kind = match self.run {
            None => Kind::Ground,
            Some(Run::Dash { .. }) => Kind::Dash,
            Some(Run::Brake { .. }) => return,
        };
        let free = |jump: Jump| jump.kind == Kind::Dash && !jump.in_air();
        if self.jump.is_none_or(free)
            && self.plane.is_none()
            && self.arrival.is_none()
            && self.rope.is_none()
            && self.carry().is_none()
            && self.thrust.is_none()
            && self.hurt.is_none()
        {
            self.jump = Some(Jump::new(self.facing, kind));
        }
    }

    /// A frame of the jump. Returns `None` once he has landed, so the
    /// frame walks on.
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
        if since >= jump.landed() {
            return self.jump_landed(jump, held);
        }
        // B pressed in the air; natively it must still be held at the
        // landing (`$0454`), which a press alone cannot tell.
        jump.again |= again && jump.in_air() && jump.kind == Kind::Ground;
        if since == jump.kind.profile().leap {
            self.globals.audio.sound_port3(LEAP_SOUND);
        }
        match jump.kind {
            Kind::Ground => {
                let read = self.globals.frames.is_multiple_of(2);
                if let Some((way, pixels)) = jump.step(held, read) {
                    let (dx, dy) = facing_delta(way);
                    self.push_by((dx * pixels, dy * pixels));
                }
            }
            Kind::Dash => self.dash_on(),
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

    /// The dash runs on through the jump as if its way were held: the pad
    /// does not steer.
    fn dash_on(&mut self) {
        let Some(Run::Dash { direction, .. }) = self.run else {
            return;
        };
        let input = FrameInput {
            direction: Some(direction),
        };
        if room_core::run::step(&mut self.walking, &mut self.run, &self.room.room, input).is_ok() {
            self.ran = true;
            self.track_run();
            if let Some(actor) = &mut self.player_actor {
                actor.position = self.walking.position();
            }
        }
    }

    /// From the landing Ark walks on, or the dash's grace runs, and the
    /// walk's ground test runs (a pit, a lip or a rope takes him); B
    /// pressed in the air crouches again at once (`$84:98AB`). The ground
    /// landing's sound comes a frame later with a direction held.
    fn jump_landed(
        &mut self,
        jump: Jump,
        held: Option<Direction>,
    ) -> Result<Option<Step>, WorldError> {
        let since = jump.frame - 1;
        if since == jump.landed() && jump.again {
            self.jump = Some(Jump {
                frame: 1,
                ..Jump::new(self.facing, Kind::Ground)
            });
            self.run_actors()?;
            return Ok(Some(Step::Stayed));
        }
        if jump.kind.profile().graced {
            // The grace's pose lasts while the dash runs on its way.
            let on =
                matches!(self.run, Some(Run::Dash { direction, .. }) if direction == jump.facing);
            self.jump = (on && jump.showing()).then_some(jump);
            return Ok(None);
        }
        let sound = jump.landed() + if held.is_some() { 2 } else { 1 };
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

    /// `since` frames after `J`.
    fn at(kind: Kind, since: u16) -> Jump {
        Jump {
            frame: since + 1,
            ..Jump::new(Direction::Down, kind)
        }
    }

    #[test]
    fn the_jump_crouches_rises_to_28_and_lands_on_frame_27() {
        let at = |since| at(Kind::Ground, since);
        assert_eq!(Jump::new(Direction::Down, Kind::Ground).pose(), None);
        let heights: Vec<i16> = (0..=27).map(|since| at(since).height()).collect();
        assert_eq!(&heights[..4], [0, 0, 0, -5]);
        assert_eq!(heights.iter().min(), Some(&-28));
        assert_eq!(&heights[25..], [-5, 0, 0]);
        let list = |since| at(since).pose().map(|pose| (pose.list, pose.age));
        assert_eq!(
            [list(2), list(3), list(15), list(26)],
            [Some((0, 2)), Some((1, 0)), Some((2, 0)), Some((2, 11))]
        );
        assert_eq!(list(27), None);
        assert!(!at(5).over_hits() && at(6).over_hits() && at(22).over_hits());
        assert!(!at(23).over_hits());
    }

    #[test]
    fn the_dash_jump_leaps_at_once_and_shows_the_grace() {
        let at = |since| at(Kind::Dash, since);
        assert_eq!(at(0).height(), -4);
        assert_eq!(at(9).height(), -23);
        assert_eq!((at(23).height(), at(24).height()), (0, 0));
        let pose = |since| {
            at(since)
                .pose()
                .map(|pose| (pose.resource, pose.list, pose.age))
        };
        assert_eq!(pose(0), Some((3, 0x09, 0)));
        assert_eq!(pose(12), Some((3, 0x0A, 0)));
        assert_eq!(pose(24), Some((3, 0x0A, 12)));
        assert_eq!(pose(25), Some((1, 0x26, 0)));
        assert_eq!(pose(31), Some((1, 0x26, 6)));
        assert_eq!(pose(32), None);
    }

    #[test]
    fn a_read_steers_a_pixel_then_two() {
        let mut jump = at(Kind::Ground, GROUND.crouch);
        let moves: Vec<_> = [true, false, true, false, false]
            .into_iter()
            .map(|read| {
                jump.step(Some(Direction::Left), read)
                    .map(|(_, pixels)| pixels)
            })
            .collect();
        assert_eq!(moves, [Some(1), Some(2), Some(1), Some(2), None]);
        let mut crouching = Jump::new(Direction::Right, Kind::Ground);
        assert_eq!(crouching.step(Some(Direction::Left), true), None);
    }
}
