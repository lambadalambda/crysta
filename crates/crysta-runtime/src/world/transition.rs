//! Leaving through an exit and arriving on the other side, frame by frame,
//! with the screen's fades.
//!
//! An exit's selector names the controller that walks the player out
//! (`$8D:895B`) and the adjustment its arrival starts from (`$8D:8985`,
//! added to the raw destination, then the player's (8,16)). The motions are
//! measured on the native route (`local/pandora-tower-discovery/departure`):
//! a door walks 16 pixels out and, after a pause, 17 in; stairs have their
//! own 63-frame descents and climbs. The screen fades out over the last 16
//! frames of leaving and in over the first 16 after the load (fade type 0,
//! `$8D:89F4` and `$8D:8A81`: one brightness step a frame). The frame API
//! ([`World::update`]) plays them; the stepping API loads at once, at its
//! raw or exact source-qualified placement, for route discovery.
//!
//! Not modelled: the loads' own frames (3 to 5 natively, more between the
//! house and the town, which only lengthens the dark). Script transfers
//! fade by their mode ([`super::fade`]).

use super::{Step, World, WorldError, EXIT_SOUND, STAIRS, STAIRS_UP};
use crate::{admitted, WORLD_MAPS};
use assets::maps::exits::ExitRecord;
use room_core::{AnimationState, Direction, WalkingState};

/// Per-frame moves as runs: (frames, dx, dy).
type Motion = &'static [(u8, i8, i8)];

/// Out through a door, drawn going Down: 16 pixels, one a frame.
const DOOR_LEAVING: Motion = &[(16, 0, 1)];
/// In through one, from native 6989 (`$0F` to `$10`): a pause, then 17
/// pixels.
const DOOR_ARRIVING: Motion = &[
    (8, 0, 0),
    (1, 0, 1),
    (2, 0, 0),
    (1, 0, 1),
    (1, 0, 0),
    (15, 0, 1),
    (1, 0, 0),
];
/// Down the stairs (selector 14), C to E at native 24987: 63 frames, +10, +7 in all.
const STAIRS_DOWN_LEAVING: Motion = &[
    (1, 0, -1),
    (7, 0, 0),
    (1, 1, -1),
    (7, 0, 0),
    (1, 1, -1),
    (15, 0, 0),
    (1, 1, 1),
    (3, 0, 0),
    (1, 1, 1),
    (3, 0, 0),
    (1, 1, 1),
    (3, 0, 0),
    (1, 1, 1),
    (3, 0, 0),
    (1, 1, 1),
    (3, 0, 0),
    (1, 1, 1),
    (3, 0, 0),
    (1, 1, 1),
    (1, 0, 0),
    (1, 0, 1),
    (1, 0, 0),
    (1, 1, 1),
    (1, 0, 0),
    (1, 0, 1),
];
/// Onto them below, E to `$20` from native 25598: 77 frames, +14, +23 in all.
const STAIRS_DOWN_ARRIVING: Motion = &[
    (13, 0, 0),
    (1, 2, 1),
    (3, 0, 0),
    (1, 2, 1),
    (3, 0, 0),
    (1, 2, 1),
    (4, 0, 0),
    (1, 2, 1),
    (4, 0, 0),
    (1, 2, 1),
    (3, 0, 0),
    (1, 1, 1),
    (3, 0, 0),
    (1, 1, 1),
    (3, 0, 0),
    (1, 1, 1),
    (3, 0, 0),
    (1, 0, 1),
    (3, 0, 0),
    (1, 1, 1),
    (3, 0, 0),
    (1, 0, 1),
    (3, 0, 0),
    (4, 0, 1),
    (2, 0, 0),
    (4, 0, 1),
    (2, 0, 0),
    (4, 0, 1),
    (2, 0, 0),
];
/// Up the stairs (selector 13), `$21` to `$20` at native 54017: 63 frames, -15, -14 in all.
const STAIRS_UP_LEAVING: Motion = &[
    (1, 0, -1),
    (3, 0, 0),
    (1, 0, -1),
    (3, 0, 0),
    (1, 0, -1),
    (3, 0, 0),
    (1, 0, -1),
    (3, 0, 0),
    (1, 0, -1),
    (3, 0, 0),
    (1, -1, -1),
    (4, 0, 0),
    (1, -1, -1),
    (4, 0, 0),
    (1, -1, -1),
    (4, 0, 0),
    (1, -2, -1),
    (4, 0, 0),
    (1, -2, -1),
    (4, 0, 0),
    (1, -2, -1),
    (4, 0, 0),
    (1, -2, -1),
    (4, 0, 0),
    (1, -2, -1),
    (4, 0, 0),
    (1, -2, -1),
    (2, 0, 0),
];
/// Onto them above, `$20` to E from native 54446: 76 frames, -8, +6 in all.
const STAIRS_UP_ARRIVING: Motion = &[
    (8, 0, 0),
    (1, -2, -2),
    (3, 0, 0),
    (1, 0, -1),
    (1, 0, 0),
    (1, -1, -2),
    (1, 0, 0),
    (1, 0, -1),
    (1, 0, 0),
    (1, -1, -1),
    (1, 0, 0),
    (1, 0, -1),
    (1, 0, 0),
    (1, -1, -1),
    (1, 0, 0),
    (1, 0, -1),
    (1, 0, 0),
    (1, -1, -1),
    (3, 0, 0),
    (1, 0, -1),
    (3, 0, 0),
    (1, -1, 1),
    (3, 0, 0),
    (1, 0, 1),
    (3, 0, 0),
    (1, -1, 1),
    (3, 0, 0),
    (1, 0, 1),
    (4, 0, 0),
    (1, 0, 1),
    (3, 0, 0),
    (1, 0, 1),
    (2, 0, 0),
    (4, 0, 1),
    (2, 0, 0),
    (4, 0, 1),
    (2, 0, 0),
    (4, 0, 1),
    (2, 0, 0),
];
/// Frames of a stair arrival before its list's second frame is due, less
/// the first record's 8: the list starts at frame 9 and holds 10 frames.
const STAIRS_ARRIVING_START: u16 = 12;

/// A stair motion's list in Ark's resource 1 (`$13..$16`).
fn stair_list(motion: Motion) -> Option<u8> {
    [
        (STAIRS_DOWN_LEAVING, 0x16),
        (STAIRS_UP_LEAVING, 0x14),
        (STAIRS_DOWN_ARRIVING, 0x13),
        (STAIRS_UP_ARRIVING, 0x15),
    ]
    .into_iter()
    .find_map(|(stairs, list)| (stairs == motion).then_some(list))
}

/// Leaving without a walk (world maps, selectors without one): the fade.
const STILL: Motion = &[(16, 0, 0)];
/// Frames of a fade.
const FADE: u16 = 16;

/// The dark frames of a load, from the last frame of the fade out to the
/// first of the fade in, as the native route measures them (no NMI while
/// `$86:86ED` decompresses and uploads): they grow with what the load
/// must decompress and whether the music's samples change, which the
/// cache (`$0418`..`$044E`) decides. Loads the route does not take stay
/// dark for [`DARK`] frames, its usual room-to-room load.
/// Japanese route measurements except for the European bedroom door, whose
/// PAL load has been measured at 14 rather than 17 dark frames.
const MEASURED_DARK: [((u16, u16), u16); 27] = [
    ((0x0F, 0x10), 17),
    ((0x10, 0x0C), 18),
    ((0x0C, 0x0B), 17),
    ((0x0B, 0x0C), 18),
    ((0x0C, 0x0D), 18),
    ((0x0D, 0x0A), 52),
    ((0x0A, 0x13), 48),
    ((0x13, 0x0A), 52),
    ((0x0A, 0x0D), 40),
    ((0x0D, 0x0C), 18),
    ((0x0C, 0x0E), 72),
    ((0x0E, 0x20), 17),
    ((0x20, 0x21), 23),
    ((0x21, 0x21), 23),
    ((0x21, 0x41), 69),
    ((0x41, 0x44), 14),
    ((0x44, 0x42), 15),
    ((0x42, 0x43), 14),
    ((0x43, 0x41), 24),
    ((0x41, 0x42), 15),
    ((0x42, 0x21), 83),
    ((0x21, 0x20), 18),
    ((0x20, 0x0E), 16),
    ((0x0E, 0x0C), 18),
    ((0x0A, 0x03), 96),
    ((0x03, 0x100), 84),
    ((0x100, 0x101), 67),
];
const DARK: u16 = 17;

/// The dark frames of the load from `from` to `to` on this ROM.
pub(super) fn dark_frames(image: &[u8], from: u16, to: u16) -> u16 {
    let japanese = MEASURED_DARK
        .iter()
        .find(|(maps, _)| *maps == (from, to))
        .map_or(DARK, |&(_, frames)| frames);
    if (from, to) == (0x0f, 0x10) {
        assets::layout::per_revision(image, japanese, 14)
    } else {
        japanese
    }
}

/// `$8D:8985`: each selector's arrival adjustment, as (dx, dy).
const ADJUSTMENTS: [(i16, i16); 15] = [
    (0, 0),
    (0, 0),
    (0, 0),
    (0, 0),
    (0, 0),
    (0, -16),
    (0, 16),
    (16, 0),
    (-16, 0),
    (0, 16),
    (0, 16),
    (0, -24),
    (0, -16),
    (8, -6),
    (-14, -23),
];

/// A selector's kind (`$8D:88DF`): its high nibble when set, as the
/// world map's and the towers' `$55`/`$66` have, else its low nibble.
const fn kind(selector: u8) -> u8 {
    if selector & 0xF0 == 0 {
        selector & 0x0F
    } else {
        selector >> 4
    }
}

/// A motion under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Walk {
    motion: Motion,
    /// The way a door motion goes; it is drawn going Down.
    toward: Direction,
    frame: u16,
}

impl Walk {
    const fn new(motion: Motion, toward: Direction) -> Self {
        Self {
            motion,
            toward,
            frame: 0,
        }
    }

    fn len(self) -> u16 {
        self.motion
            .iter()
            .map(|&(frames, _, _)| u16::from(frames))
            .sum()
    }

    /// The next frame's move, or `None` once the motion is over.
    fn step(&mut self) -> Option<(i16, i16)> {
        let mut at = self.frame;
        self.frame += 1;
        let &(_, dx, dy) = self.motion.iter().find(|&&(frames, _, _)| {
            let inside = at < u16::from(frames);
            at = at.saturating_sub(u16::from(frames));
            inside
        })?;
        let (dx, dy) = (i16::from(dx), i16::from(dy));
        Some(match self.toward {
            Direction::Down => (dx, dy),
            Direction::Up => (dx, -dy),
            Direction::Left => (-dy, dx),
            Direction::Right => (dy, dx),
        })
    }

    /// Where the whole motion leads from `from`.
    #[cfg(test)]
    fn end(mut self, from: (u16, u16)) -> (u16, u16) {
        let mut at = from;
        while let Some(delta) = self.step() {
            at = moved(at, delta);
        }
        at
    }
}

fn moved((x, y): (u16, u16), (dx, dy): (i16, i16)) -> (u16, u16) {
    (x.wrapping_add_signed(dx), y.wrapping_add_signed(dy))
}

/// Leaving through an exit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Leaving {
    record: ExitRecord,
    walk: Walk,
    /// An upward door walk's threshold: the top of the cell it began in.
    door_up: Option<(u16, u16)>,
}

/// Arriving after one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Arriving {
    walk: Walk,
}

impl Leaving {
    /// Leaving through `record`, facing `facing`; `plain` for a world map's
    /// side, which has no walk.
    pub(super) fn new(
        record: &ExitRecord,
        (facing, start): (Direction, (u16, u16)),
        plain: bool,
    ) -> Self {
        let motion = match record.selector() {
            _ if plain => STILL,
            STAIRS => STAIRS_DOWN_LEAVING,
            STAIRS_UP => STAIRS_UP_LEAVING,
            _ => DOOR_LEAVING,
        };
        // Only a door's walk turns with the player.
        let toward = if motion == DOOR_LEAVING {
            facing
        } else {
            Direction::Down
        };
        Self {
            record: record.clone(),
            walk: Walk::new(motion, toward),
            door_up: (motion == DOOR_LEAVING && toward == Direction::Up)
                .then_some((start.0, start.1 & !15)),
        }
    }
}

impl Arriving {
    /// The arrival `record` leads to, and where it starts: `None` for a
    /// selector without one.
    fn after(record: &ExitRecord) -> Option<(Self, (u16, u16))> {
        let selector = kind(record.selector());
        let &(dx, dy) = ADJUSTMENTS.get(usize::from(selector))?;
        let (motion, toward) = match (selector, dx.signum(), dy.signum()) {
            (STAIRS, ..) => (STAIRS_DOWN_ARRIVING, Direction::Down),
            (STAIRS_UP, ..) => (STAIRS_UP_ARRIVING, Direction::Down),
            (_, 0, 0) => return None,
            (_, 0, -1) => (DOOR_ARRIVING, Direction::Down),
            (_, 0, _) => (DOOR_ARRIVING, Direction::Up),
            (_, 1, _) => (DOOR_ARRIVING, Direction::Left),
            _ => (DOOR_ARRIVING, Direction::Right),
        };
        let (x, y) = record.destination_position();
        let start = moved((x, y), (dx + 8, dy + 16));
        Some((
            Self {
                walk: Walk::new(motion, toward),
            },
            start,
        ))
    }
}

impl World<'_> {
    /// Starts leaving through `record`, when it leads into the slice: the
    /// exit's sound plays now (`$8D:8872`). Returns whether it did.
    pub(super) fn leave(&mut self, record: &ExitRecord) -> bool {
        let Some(destination) = record
            .direct_destination()
            .ok()
            .filter(|&map| admitted(map))
        else {
            return false;
        };
        let plain = self.plane.is_some() || WORLD_MAPS.contains(&destination);
        self.leaving = Some(Leaving::new(record, (self.facing, self.position()), plain));
        self.globals.audio.sound_port3(EXIT_SOUND);
        true
    }

    /// While Ark walks up into a door, the helper's mask that hides him
    /// behind the wall above it (`$84:B988`, pose `$37`,
    /// `docs/depth-order.md`): anchored at the doorway's threshold, from the
    /// first pixel past it.
    #[must_use]
    pub fn door_mask(&self) -> Option<(u16, u16)> {
        let (x, threshold) = self.leaving.as_ref()?.door_up?;
        (self.position().1 < threshold).then_some((x, threshold))
    }

    /// Ark's stair list (resource 1) and its age while he walks stairs
    /// (`docs/ark-poses.md`): leaving from the exit frame; arriving from
    /// its first frame, the list's start (frame 9) held over the dark set-up.
    #[must_use]
    pub fn stairs_pose(&self) -> Option<(u8, u16)> {
        if let Some(leaving) = &self.leaving {
            let list = stair_list(leaving.walk.motion)?;
            // The exit frame shows the first record, one frame short.
            return Some((list, leaving.walk.frame + 1));
        }
        let walk = self.arriving.as_ref()?.walk;
        let list = stair_list(walk.motion).filter(|_| walk.frame > 0)?;
        Some((list, walk.frame.saturating_sub(STAIRS_ARRIVING_START)))
    }

    /// The brightness of an exit's fades, 0 dark to 15 full.
    pub(super) fn exit_brightness(&self) -> u8 {
        let level = self
            .leaving
            .as_ref()
            .map_or(u16::from(self.dawn), |leaving| {
                leaving.walk.len().saturating_sub(leaving.walk.frame)
            });
        u8::try_from(level.min(FADE - 1)).unwrap_or(15)
    }

    /// Whether the player is leaving or arriving.
    #[must_use]
    pub const fn in_transition(&self) -> bool {
        self.leaving.is_some() || self.arriving.is_some() || self.fading.is_some() || self.dark > 0
    }

    /// A frame of leaving or arriving, if one is under way: the player
    /// moves by the motion, the actors run, and leaving's last frame loads
    /// the destination.
    pub(super) fn transition_frame(&mut self) -> Result<Option<Step>, WorldError> {
        if let Some(mut leaving) = self.leaving.take() {
            let Some(delta) = leaving.walk.step() else {
                return self.arrive(&leaving.record).map(Some);
            };
            self.move_by(delta);
            self.leaving = Some(leaving);
            self.run_actors()?;
            return Ok(Some(Step::Walked));
        }
        let Some(mut arriving) = self.arriving.take() else {
            return Ok(None);
        };
        let step = arriving
            .walk
            .step()
            .map_or(Step::Stayed, |delta| self.move_by(delta));
        if arriving.walk.frame < arriving.walk.len() {
            self.arriving = Some(arriving);
        } else {
            // Stairs end facing Down (`$84:A303`).
            if stair_list(arriving.walk.motion).is_some() {
                self.facing = Direction::Down;
            }
            self.animation = AnimationState::standing(self.facing);
        }
        self.run_actors()?;
        Ok(Some(step))
    }

    fn move_by(&mut self, delta: (i16, i16)) -> Step {
        let before = self.position();
        let after = moved(before, delta);
        self.walking = WalkingState::new(after.0, after.1);
        // The pauses stand.
        self.animation
            .advance((after != before).then_some(self.facing));
        if after == before {
            Step::Stayed
        } else {
            Step::Walked
        }
    }

    /// Loads the exit's destination, the player at its arrival's start.
    fn arrive(&mut self, record: &ExitRecord) -> Result<Step, WorldError> {
        let Some(mut entered) = self.enter_exit(record, true)? else {
            return Ok(Step::Stayed);
        };
        entered.face(self.facing);
        entered.dawn = 0;
        if entered.arrival.is_none() && entered.plane.is_none() {
            if let Some((arriving, (x, y))) = Arriving::after(record) {
                entered.walking = WalkingState::new(x, y);
                entered.arriving = Some(arriving);
            }
        }
        let from = self.map;
        let to = entered.map;
        entered.dark = dark_frames(self.image, from, to);
        *self = entered;
        Ok(Step::Entered { from, to })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total(motion: Motion) -> (i16, i16) {
        motion.iter().fold((0, 0), |(x, y), &(frames, dx, dy)| {
            let n = i16::from(frames);
            (x + n * i16::from(dx), y + n * i16::from(dy))
        })
    }

    #[test]
    fn the_stairs_arrive_where_the_adjustment_left_them() {
        // Settled at raw + (8,16): the arrival undoes the adjustment.
        let (dx, dy) = ADJUSTMENTS[usize::from(STAIRS)];
        assert_eq!(total(STAIRS_DOWN_ARRIVING), (-dx, -dy));
        let (dx, dy) = ADJUSTMENTS[usize::from(STAIRS_UP)];
        assert_eq!(total(STAIRS_UP_ARRIVING), (-dx, -dy));
    }

    #[test]
    fn a_door_walks_out_sixteen_and_in_seventeen_along_its_way() {
        let mut out = Walk::new(DOOR_LEAVING, Direction::Left);
        assert_eq!(out.len(), 16);
        assert_eq!(out.step(), Some((-1, 0)));
        assert_eq!(out.end((100, 100)), (85, 100));
        assert_eq!(
            Walk::new(DOOR_ARRIVING, Direction::Up).end((0, 100)),
            (0, 83)
        );
    }

    #[test]
    fn the_european_bedroom_door_has_fourteen_dark_frames_only() {
        let mut europe = vec![0; 0x1_0000];
        europe[0xffc0..0xffc0 + 21].fill(b' ');
        europe[0xffc0..0xffc0 + 12].copy_from_slice(b"TERRANIGMA P");
        let japan = [0; 16]; // No revision header falls back to Japan.
        assert_eq!(dark_frames(&europe, 0x0f, 0x10), 14);
        assert_eq!(dark_frames(&japan, 0x0f, 0x10), 17);
        for image in [&europe[..], &japan[..]] {
            assert_eq!(dark_frames(image, 0x0c, 0x0d), 18);
            assert_eq!(dark_frames(image, 0x0f, 0x0b), DARK);
        }
    }
}
