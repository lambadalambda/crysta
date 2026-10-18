//! Ark's jump with B (`docs/jump.md`). From standing or walking
//! (`$84:96DE`/`$84:9710`): 3 frames of crouch, 24 in the air on the height
//! stream `$39`, steered a pixel then two after each pad read; from the
//! landing he walks on, and the walk's ground test runs. From a dash
//! (`$84:99B1`..): no crouch, the stream `$3A`, the dash runs on unsteered,
//! and it lands in the dash's release grace. With a pot (`$84:AE94`..):
//! no crouch, no sounds, steered every frame, and it lands a frame early
//! but for Left. A pit or a lip under him in the air takes nothing.

use super::pose::{by_facing, ArkPose};
use super::{facing_delta, Step, World, WorldError};
use crate::scene::Presses;
use assets::sprites::PandoraCarryMotion;
use room_core::run::Run;
use room_core::Direction;

/// Sounds on port 3: the leap and the landing.
const LEAP_SOUND: u8 = 0x0E;
const LAND_SOUND: u8 = 0x0F;
/// Hits pass under him from this height up (`$85:F856`).
const OVER_HITS: i16 = -16;
/// Frames in the rising list (records 8 + 4, or 7 + 5).
const RISE: u16 = 12;
/// Frames of heights, every kind.
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
    /// The frame of the leap's sound, if it sounds.
    leap: Option<u16>,
    /// What follows the landing.
    landing: AfterLanding,
}

/// What follows a jump's landing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AfterLanding {
    /// The landing sounds `$0F`.
    Aloud,
    /// The dash's release grace, with its pose and no sound.
    Grace,
    /// Nothing: the carry goes on.
    Silent,
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
    leap: Some(4),
    landing: AfterLanding::Aloud,
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
    leap: Some(1),
    landing: AfterLanding::Grace,
};

/// B in a carry dash (native traces `t_b_*`): the dash jump's heights, no
/// leap sound; Ark is drawn in the carry's poses ([`Jump::carry_pose`]).
const CARRY_DASH: Profile = Profile { leap: None, ..DASH };

/// The carry jump (`$84:AE94`): the stream of the ground jump without its
/// crouch, ended at -5 (Down, Up, Right land then) and 0 (Left lands a
/// frame after, `COP 02 0D` at `$84:AF55`). Ark is drawn in the carry's
/// poses ([`Jump::carry_pose`]).
const CARRY: Profile = Profile {
    crouch: 0,
    heights: [
        -5, -10, -13, -16, -19, -21, -23, -25, -26, -27, -28, -28, -28, -27, -26, -25, -23, -21,
        -19, -16, -13, -10, -5, 0,
    ],
    lists: [[0x1B, 0x1E, 0x21], [0x1B, 0x1E, 0x21], [0x1C, 0x1F, 0x22]],
    leap: None,
    landing: AfterLanding::Silent,
};
/// The carry jump's landing but for Left (`COP 02 0B` at `$84:AEE5`).
const CARRY_LANDED: u16 = 22;

/// After a dash jump's landing, the grace shows resource 1's lists (`$26`
/// Down, `$27` Up, `$28` Right) from `J + 25` to `J + 31`.
const GRACE_LISTS: [u8; 3] = [0x26, 0x27, 0x28];
const GRACE_POSE: u16 = 25;
/// The throw from the air's lists (`$2A` Down, `$2B` Up, `$2C` Right).
const THROWN_LISTS: [u8; 3] = [0x2A, 0x2B, 0x2C];
/// The frames A throws in (`$84:AF84`: `+$24` 8 to `$15`).
const THROW_WINDOW: std::ops::RangeInclusive<u16> = 6..=18;
const GRACE_END: u16 = 32;
/// A carry dash jump's landing list ends a frame sooner (`t_b_*`).
const CARRY_DASH_END: u16 = 31;
/// The frames the carry dash runs on in a jump as if its way were held.
const CARRY_DASH_HELD: u16 = 30;

/// A kind of jump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Ground,
    Dash,
    Carry,
    /// B in a carry dash: the dash runs on with the pot in hand.
    CarryDash,
    /// From the rope (`$84:9E41`): the ground jump's frames, back onto the
    /// rope with no sound.
    Rope,
}

impl Kind {
    const fn profile(self) -> &'static Profile {
        match self {
            Self::Ground | Self::Rope => &GROUND,
            Self::Dash => &DASH,
            Self::Carry => &CARRY,
            Self::CarryDash => &CARRY_DASH,
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
    /// Landed on the rope: its stand, before the lean.
    leaning: bool,
    /// With a pot: the frame A threw it from the air (`$84:AF84`).
    thrown: Option<u16>,
}

impl Jump {
    const fn new(facing: Direction, kind: Kind) -> Self {
        Self {
            frame: 0,
            facing,
            kind,
            steer: None,
            again: false,
            leaning: false,
            thrown: None,
        }
    }

    /// Frames since `J`; `None` on the press's own frame.
    const fn since(self) -> Option<u16> {
        self.frame.checked_sub(1)
    }

    /// The frame he lands on.
    const fn landed(self) -> u16 {
        if matches!(self.kind, Kind::Carry)
            && self.thrown.is_none()
            && !matches!(self.facing, Direction::Left)
        {
            CARRY_LANDED
        } else {
            self.kind.profile().crouch + AIR
        }
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
            Some(since) if since >= profile.crouch && since - profile.crouch < AIR => {
                profile.heights[(since - profile.crouch) as usize]
            }
            _ => 0,
        }
    }

    /// How high he was the frame before.
    const fn height_before(self) -> i16 {
        Self {
            frame: self.frame.saturating_sub(1),
            ..self
        }
        .height()
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
        if let (Kind::Carry, Some(at)) = (self.kind, self.thrown) {
            // The throw from the air (`COP 83 2A/2B/2C 03`) to the landing.
            return if since > at && since < self.landed() {
                Some(by_facing(3, THROWN_LISTS, self.facing, since - at - 1))
            } else {
                None
            };
        }
        if matches!(self.kind, Kind::Carry | Kind::CarryDash) {
            return None;
        }
        if self.leaning {
            // Resource 1's `$10` (`$84:A8FB`).
            return Some(ArkPose::new(
                1,
                0x10,
                matches!(self.facing, Direction::Left),
                0,
                false,
            ));
        }
        let profile = self.kind.profile();
        let crouch = profile.crouch;
        let (lists, age) = if since < crouch {
            (profile.lists[0], since)
        } else if since < crouch + RISE {
            (profile.lists[1], since - crouch)
        } else if since < self.landed()
            || (matches!(self.kind, Kind::Rope) && since == self.landed())
            || (matches!(profile.landing, AfterLanding::Grace) && since < GRACE_POSE)
        {
            (profile.lists[2], since - crouch - RISE)
        } else if matches!(profile.landing, AfterLanding::Grace) && since < GRACE_END {
            return Some(by_facing(1, GRACE_LISTS, self.facing, since - GRACE_POSE));
        } else {
            return None;
        };
        Some(by_facing(3, lists, self.facing, age))
    }

    /// A carry jump's pose and frames into it: rising, falling, then the
    /// carry's stand on the landing frame (drawn still 5 high).
    pub(super) const fn carry_pose(self) -> Option<(PandoraCarryMotion, u16)> {
        let Some(since) = self.since() else {
            return None;
        };
        if matches!(self.kind, Kind::CarryDash) {
            // The landing's list from the landing, while the grace runs.
            return Some(if since < RISE {
                (PandoraCarryMotion::DashJumpRising, since)
            } else if since < self.landed() {
                (PandoraCarryMotion::DashJumpFalling, since - RISE)
            } else if since < CARRY_DASH_END {
                (PandoraCarryMotion::DashJumpLanding, since - self.landed())
            } else {
                return None;
            });
        }
        if !matches!(self.kind, Kind::Carry) || matches!(self.thrown, Some(at) if since > at) {
            None
        } else if since < RISE {
            Some((PandoraCarryMotion::JumpRising, since))
        } else if since < self.landed() {
            Some((PandoraCarryMotion::JumpFalling, since - RISE))
        } else if since == self.landed() {
            Some((PandoraCarryMotion::Standing, 0))
        } else {
            None
        }
    }

    /// Whether the jump still shows after the landing: the dash's grace,
    /// the carry's last heights.
    const fn showing(self) -> bool {
        let end = match self.kind.profile().landing {
            _ if matches!(self.kind, Kind::CarryDash) => CARRY_DASH_END,
            AfterLanding::Grace => GRACE_END,
            // The landing's stand.
            AfterLanding::Silent => self.landed() + 1,
            AfterLanding::Aloud => self.landed(),
        };
        matches!(self.since(), Some(since) if since < end)
    }

    /// This frame's move: on a pad read (`$0042` even) the way held starts
    /// a pixel, the next frame two more (`$84:98D2`).
    fn step(&mut self, held: Option<Direction>, read: bool) -> Option<(Direction, i16)> {
        if !self.in_air() {
            return None;
        }
        // With a pot every frame is a read: a pixel, then two (`COP 61`).
        if matches!(self.kind, Kind::Carry) {
            let since = self.frame - 1;
            return held
                .filter(|_| since > 0)
                .map(|way| (way, if since % 2 == 1 { 1 } else { 2 }));
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
        let carrying = self.carry().is_some();
        let kind = match self.run {
            None if self.rope.is_some_and(|rope| !rope.leaning()) => Kind::Rope,
            None if self.rope.is_some() => return,
            None if carrying && self.holds_pot() && self.carry_dashing() => Kind::CarryDash,
            None if carrying && self.holds_pot() => Kind::Carry,
            None if !carrying => Kind::Ground,
            Some(Run::Dash { .. }) if !carrying => Kind::Dash,
            _ => return,
        };
        let free = |jump: Jump| jump.kind == Kind::Dash && !jump.in_air();
        if self.jump.is_none_or(free)
            && self.plane.is_none()
            && self.arrival.is_none()
            && self.thrust.is_none()
            && self.hurt.is_none()
        {
            // On the rope he faces across (the horizontal poses).
            let facing = match (kind, self.facing) {
                (Kind::Rope, Direction::Left) => Direction::Left,
                (Kind::Rope, _) => Direction::Right,
                (_, facing) => facing,
            };
            self.jump = Some(Jump::new(facing, kind));
            if kind == Kind::Carry {
                self.end_carry_run();
            }
        }
    }

    /// A frame of the jump. Returns `None` once he has landed, so the
    /// frame walks on.
    pub(super) fn jump_frame(
        &mut self,
        held: Option<Direction>,
        presses: Presses,
    ) -> Result<Option<Step>, WorldError> {
        let Some(mut jump) = self.jump.take() else {
            return Ok(None);
        };
        jump.frame += 1;
        let since = jump.frame - 1;
        // With a pot the dash runs on as if held through the landing's list.
        let held_on = jump.kind == Kind::CarryDash && since < CARRY_DASH_HELD;
        if since >= jump.landed() && !held_on {
            return self.jump_landed(jump, held);
        }
        // B pressed in the air; natively it must still be held at the
        // landing (`$0454`), which a press alone cannot tell.
        jump.again |= presses.cancel && jump.in_air() && jump.kind == Kind::Ground;
        self.throw_from_air(&mut jump, since, presses.confirm);
        if Some(since) == jump.kind.profile().leap {
            self.globals.audio.sound_port3(LEAP_SOUND);
        }
        if jump.kind == Kind::Dash {
            self.dash_on();
        } else if jump.kind == Kind::CarryDash {
            self.carry_dash_on()?;
        } else {
            let read = self.globals.frames.is_multiple_of(2);
            if let Some((way, pixels)) = jump.step(held, read) {
                let (dx, dy) = facing_delta(way);
                self.push_by((dx * pixels, dy * pixels));
                self.carry_to_landing(false);
            }
        }
        // An exit puts the pot down, as a carry step's does; the frame API
        // walks him out first.
        let exit = self.take_exit()?;
        if (exit.is_some() || self.in_transition())
            && matches!(jump.kind, Kind::Carry | Kind::CarryDash)
        {
            self.drop_carried_on_exit();
        }
        if let Some(step) = exit {
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
    /// does not steer. After a turn its facing counts as held, which starts
    /// the new way's stream (not traced).
    fn dash_on(&mut self) {
        let Some(Run::Dash {
            direction, cross, ..
        }) = self.run
        else {
            return;
        };
        let input = room_core::PadInput {
            direction: Some(direction),
            cross,
        };
        if self.run_step(input).is_ok() {
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
        // After a throw from the air control returns a frame after the
        // stand (`$84:AFC7`).
        if jump.thrown.is_some() && since == jump.landed() {
            self.jump = Some(jump);
            self.run_actors()?;
            return Ok(Some(Step::Stayed));
        }
        match jump.kind.profile().landing {
            AfterLanding::Grace => {
                // The grace's pose lasts while the dash runs on its way.
                let on = if jump.kind == Kind::CarryDash {
                    self.carry_dashing()
                } else {
                    matches!(self.run, Some(Run::Dash { direction, .. }) if direction == jump.facing)
                };
                self.jump = (on && jump.showing()).then_some(jump);
                if on && self.jump.is_none() {
                    // The dash's pose again: its stream starts over.
                    self.run = self.run.map(Run::restarted);
                }
                return Ok(None);
            }
            AfterLanding::Silent => {
                self.jump = jump.showing().then_some(jump);
                return Ok(None);
            }
            AfterLanding::Aloud => {}
        }
        // From the rope: the landing's ground test (the walk runs none).
        if since == jump.landed() && jump.kind == Kind::Rope {
            self.ground_test();
            if self.fall.is_some() || self.lip_drop.is_some() {
                self.run_actors()?;
                return Ok(Some(Step::Stayed));
            }
        }
        // Only the ground jump sounds; a frame later with a direction held.
        let sound = jump.landed() + if held.is_some() { 2 } else { 1 };
        if jump.kind == Kind::Ground && since == sound {
            self.globals.audio.sound_port3(LAND_SOUND);
        }
        if self.rope.is_some() {
            return self.land_on_rope(jump);
        }
        self.jump = (since < sound).then_some(jump);
        Ok(None)
    }

    /// The landing on the rope (`$0986 & $3C00`, `$84:9BF8`): from the
    /// rope the jump's last pose holds at `J + 27`; at `J + 28` the rope's
    /// stand; at `J + 29` the lean, Up on an even `$0042`, else Down.
    fn land_on_rope(&mut self, mut jump: Jump) -> Result<Option<Step>, WorldError> {
        let since = jump.frame - 1;
        if since == jump.landed() && jump.kind == Kind::Ground {
            // He walks onto it this frame.
            self.jump = Some(jump);
            return Ok(None);
        }
        if since > jump.landed() + 1 {
            let way = if self.globals.frames.is_multiple_of(2) {
                Direction::Up
            } else {
                Direction::Down
            };
            if let Some(rope) = &mut self.rope {
                rope.lean(way);
            }
        } else {
            jump.leaning = since > jump.landed();
            self.jump = Some(jump);
        }
        self.run_actors()?;
        Ok(Some(Step::Stayed))
    }

    /// With a pot, A in the air from `J + 6` to `J + 18` (`$84:AF84`)
    /// throws it the frame after, from the height Ark had at the A
    /// (`$0999`); sound `$12`.
    fn throw_from_air(&mut self, jump: &mut Jump, since: u16, a: bool) {
        if jump.kind != Kind::Carry {
            return;
        }
        if jump.thrown.is_none() && a && THROW_WINDOW.contains(&since) && self.holds_pot() {
            jump.thrown = Some(since);
        }
        if jump.thrown == Some(since.wrapping_sub(1)) {
            if self.throw_pot_from_air(jump.height_before()) {
                self.globals.audio.sound_port3(super::pots::RELEASE_SOUND);
            } else {
                jump.thrown = None;
            }
        }
    }

    /// Whether a pot thrown from the air flies on this frame: not on its
    /// release's, which takes its first step.
    pub(super) fn pot_flies_on(&self) -> bool {
        self.jump
            .is_some_and(|jump| jump.thrown.is_none_or(|at| at + 2 != jump.frame))
    }

    /// A carry jump's pose ([`Jump::carry_pose`]).
    pub(super) fn jump_carry_pose(&self) -> Option<(PandoraCarryMotion, u16)> {
        self.jump.and_then(Jump::carry_pose)
    }

    /// How far above his place Ark is drawn while he jumps.
    pub(super) fn jump_height(&self) -> i16 {
        self.jump.map_or(0, Jump::height)
    }

    /// How high Ark was the frame before, which the shadow reads.
    pub(super) fn jump_height_before(&self) -> i16 {
        self.jump.map_or(0, Jump::height_before)
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
