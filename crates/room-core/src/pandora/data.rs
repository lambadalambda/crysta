//! Immutable qualified geometry; no executable source or runtime destinations.
use super::graph::Cue;
use super::{Invocation, PandoraText};
use crate::pots::SourceObject;
use crate::slice::SliceError;
use crate::{Direction, Room};
use alloc::vec::Vec;

/// Finite source actor phase, selected by core output, not host flag logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ScenePhase {
    /// Existing house actor presentation.
    House,
    /// Asset phase `town-source`.
    TownSource,
    /// Asset phase `resident-13`.
    Resident13,
    /// Asset phase `resident-13-request`.
    Resident13Request,
    /// Asset phase `c-entry`.
    CEntry,
    /// Asset phase `c-choice`.
    CChoice,
    /// Asset phase `c-direct`.
    CDirect,
    /// Asset phase `c-first-hit`.
    CFirstHit,
    /// Asset phase `c-second-hit`.
    CSecondHit,
    /// Asset phase `c-fourth-down`.
    CFourthDown,
    /// Asset phase `c-fourth-up`.
    CFourthUp,
    /// Asset phase `c-color-math`.
    CColorMath,
    /// Asset phase `c-reaction-speaker`.
    CReactionSpeaker,
    /// Asset phase `c-reaction-right`.
    CReactionRight,
    /// Asset phase `c-reaction-left`.
    CReactionLeft,
    /// Asset phase `c-reaction-final`.
    CReactionFinal,
    /// Asset phase `c-departed`.
    CDeparted,
    /// Asset phase `cellar-e`.
    CellarE,
    /// Asset phase `cellar-20`.
    Cellar20,
    /// Asset phase `box-contact`.
    BoxContact,
    /// Asset phase `box-opening`.
    BoxOpening,
    /// Asset phase `box-opening-cue`.
    BoxOpeningCue,
    /// Asset phase `tour-41-0`.
    Tour410,
    /// Asset phase `tour-41-1`.
    Tour411,
    /// Asset phase `tour-41-2`.
    Tour412,
    /// Asset phase `tour-41-3`.
    Tour413,
    /// Asset phase `tour-41-4`.
    Tour414,
    /// Asset phase `tour-41-5`.
    Tour415,
    /// Asset phase `tour-41-6`.
    Tour416,
    /// Asset phase `tour-41-7`.
    Tour417,
    /// Asset phase `tour-44`.
    Tour44,
    /// Asset phase `tour-42`.
    Tour42,
    /// Asset phase `tour-43`.
    Tour43,
    /// Asset phase `tour-control`.
    TourControl,
}
impl ScenePhase {
    /// Exact assets phase ID; None delegates existing house art.
    #[must_use]
    pub const fn key(self) -> Option<&'static str> {
        match self {
            Self::House => None,
            Self::TownSource => Some("town-source"),
            Self::Resident13 => Some("resident-13"),
            Self::Resident13Request => Some("resident-13-request"),
            Self::CEntry => Some("c-entry"),
            Self::CChoice => Some("c-choice"),
            Self::CDirect => Some("c-direct"),
            Self::CFirstHit => Some("c-first-hit"),
            Self::CSecondHit => Some("c-second-hit"),
            Self::CFourthDown => Some("c-fourth-down"),
            Self::CFourthUp => Some("c-fourth-up"),
            Self::CColorMath => Some("c-color-math"),
            Self::CReactionSpeaker => Some("c-reaction-speaker"),
            Self::CReactionRight => Some("c-reaction-right"),
            Self::CReactionLeft => Some("c-reaction-left"),
            Self::CReactionFinal => Some("c-reaction-final"),
            Self::CDeparted => Some("c-departed"),
            Self::CellarE => Some("cellar-e"),
            Self::Cellar20 => Some("cellar-20"),
            Self::BoxContact => Some("box-contact"),
            Self::BoxOpening => Some("box-opening"),
            Self::BoxOpeningCue => Some("box-opening-cue"),
            Self::Tour410 => Some("tour-41-0"),
            Self::Tour411 => Some("tour-41-1"),
            Self::Tour412 => Some("tour-41-2"),
            Self::Tour413 => Some("tour-41-3"),
            Self::Tour414 => Some("tour-41-4"),
            Self::Tour415 => Some("tour-41-5"),
            Self::Tour416 => Some("tour-41-6"),
            Self::Tour417 => Some("tour-41-7"),
            Self::Tour44 => Some("tour-44"),
            Self::Tour42 => Some("tour-42"),
            Self::Tour43 => Some("tour-43"),
            Self::TourControl => Some("tour-control"),
        }
    }
    pub(crate) const fn map(self) -> Option<u16> {
        match self {
            Self::House => None,
            Self::TownSource => Some(10),
            Self::CEntry
            | Self::CChoice
            | Self::CDirect
            | Self::CFirstHit
            | Self::CSecondHit
            | Self::CFourthDown
            | Self::CFourthUp
            | Self::CColorMath
            | Self::CReactionSpeaker
            | Self::CReactionRight
            | Self::CReactionLeft
            | Self::CReactionFinal
            | Self::CDeparted => Some(12),
            Self::CellarE => Some(14),
            Self::Resident13 | Self::Resident13Request => Some(19),
            Self::Cellar20 => Some(32),
            Self::BoxContact | Self::BoxOpening | Self::BoxOpeningCue => Some(33),
            Self::Tour410
            | Self::Tour411
            | Self::Tour412
            | Self::Tour413
            | Self::Tour414
            | Self::Tour415
            | Self::Tour416
            | Self::Tour417
            | Self::TourControl => Some(65),
            Self::Tour42 => Some(66),
            Self::Tour43 => Some(67),
            Self::Tour44 => Some(68),
        }
    }
}

/// Fixed load-time collision profile; source occupancy and sample halo are required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CollisionKey {
    /// Wider A corridor.
    Town,
    /// Map13 resident area.
    Resident,
    /// Direct C before a hit, also entry/choice.
    CClosed,
    /// First real-hit patch.
    CDamaged,
    /// Second-hit temporary occupancy and resident reaction.
    CReaction,
    /// Door opened and residents departed.
    COpen,
    /// Cellar E.
    CellarE,
    /// Cellar20.
    Cellar20,
    /// Contact box before opening.
    Box,
    /// Same-map opening reload.
    BoxOpened,
    /// Tour41.
    Tour41,
    /// Tour44.
    Tour44,
    /// Tour42.
    Tour42,
    /// Tour43.
    Tour43,
}
impl CollisionKey {
    /// Required compiler order.
    pub const ALL: [Self; 14] = [
        Self::Town,
        Self::Resident,
        Self::CClosed,
        Self::CDamaged,
        Self::CReaction,
        Self::COpen,
        Self::CellarE,
        Self::Cellar20,
        Self::Box,
        Self::BoxOpened,
        Self::Tour41,
        Self::Tour44,
        Self::Tour42,
        Self::Tour43,
    ];
    /// Source layer dimensions in cells (not navigation/pacing constants).
    #[must_use]
    pub const fn dimensions(self) -> (u16, u16) {
        match self {
            Self::Town => (64, 80),
            Self::Resident => (64, 32),
            Self::Box | Self::BoxOpened => (16, 32),
            Self::Tour41 | Self::Tour44 | Self::Tour42 | Self::Tour43 => (32, 32),
            _ => (32, 64),
        }
    }
    /// Source map membership.
    #[must_use]
    pub const fn map(self) -> u16 {
        match self {
            Self::Town => 0xa,
            Self::Resident => 0x13,
            Self::CClosed | Self::CDamaged | Self::CReaction | Self::COpen => 0xc,
            Self::CellarE => 0xe,
            Self::Cellar20 => 0x20,
            Self::Box | Self::BoxOpened => 0x21,
            Self::Tour41 => 0x41,
            Self::Tour44 => 0x44,
            Self::Tour42 => 0x42,
            Self::Tour43 => 0x43,
        }
    }
}
/// One immutable admitted collision grid.
#[derive(Debug)]
pub struct ProfileRoom {
    /// Finite load-time profile.
    pub key: CollisionKey,
    /// Source-compiled grid and qualified halo, never a captured initializer.
    pub room: Room,
}
/// Only new ordinary routes; forced transitions belong to the graph's cues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Travel {
    /// A to13.
    TownToResident,
    /// 13 to A.
    ResidentToTown,
    /// A to old D.
    TownToHouse,
    /// C to E cellar.
    CToCellar,
    /// E to20.
    ETo20,
    /// 20 to21.
    TwentyToBox,
}
impl Travel {
    pub(crate) const fn maps(self) -> (u16, u16) {
        match self {
            Self::TownToResident => (0xa, 0x13),
            Self::ResidentToTown => (0x13, 0xa),
            Self::TownToHouse => (0xa, 0xd),
            Self::CToCellar => (0xc, 0xe),
            Self::ETo20 => (0xe, 0x20),
            Self::TwentyToBox => (0x20, 0x21),
        }
    }
}
/// One fixed qualified motion, never a caller-defined branch program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionKey {
    /// Ordinary exit, requiring player ownership and story admission.
    Travel(Travel),
    /// Currently owned continuation only. `BoxAcquireControl` specifically certifies
    /// successful COPDF, not a timer-based assumption that proximity grants22.
    Cue(Cue),
}
/// Exact qualified player anchor and facing/delayed travel direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    /// Player coordinates, not an actor origin.
    pub position: (u16, u16),
    /// Required facing/direction.
    pub facing: Direction,
}
/// Player pose operation, never an actor's presentation coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionPose {
    /// Source-qualified absolute player placement.
    Absolute(Anchor),
    /// Keep the active player coordinates; optionally select a qualified standing facing.
    /// Admitted only on non-reloading cue samples.
    Preserve {
        /// None retains facing; Some selects a source-qualified stationary direction.
        facing: Option<Direction>,
    },
}
impl MotionPose {
    pub(crate) fn apply(self, current: Anchor) -> Anchor {
        match self {
            Self::Absolute(anchor) => anchor,
            Self::Preserve { facing } => Anchor {
                position: current.position,
                facing: facing.unwrap_or(current.facing),
            },
        }
    }
}
/// One logical source-qualified presentation sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionFrame {
    /// Fixed source/destination map for this motion.
    pub map_id: u16,
    /// Source-qualified player operation, never a runtime action argument.
    pub pose: MotionPose,
    /// Explicit reconstruction edge, including same-map reload.
    pub reload: bool,
    /// In-progress actor cue. The terminal sample is a completion boundary: its
    /// pose/reload apply, then the graph's next scene wins atomically. Its `scene`
    /// is therefore not a separately displayed frame. No trailing wait is invented.
    pub scene: ScenePhase,
}
/// Immutable ordered logical samples. Missing motions fail closed; no default timers.
#[derive(Debug)]
pub struct MotionSpec {
    /// Fixed route/cue identity.
    pub key: MotionKey,
    /// Exact origin; mandatory for ordinary exits, optional for forced cues.
    pub trigger: Option<Anchor>,
    /// At most4096 samples; load motions have exactly one reload marker.
    pub frames: Vec<MotionFrame>,
}
/// Finite callback kind, not an unrestricted proximity target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactKind {
    /// Ordinary map13 resident interaction.
    Resident,
    /// First new Down contact, never Interact.
    BoxWarning,
}
/// Source-qualified callback/bump admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContactSpec {
    /// Callback kind.
    pub kind: ContactKind,
    /// Exact collision-resolved contact/interaction anchor.
    pub trigger: Anchor,
    /// Source bump/settled pose.
    pub result: Anchor,
}
/// Inclusive raw Ark coordinate predicate, independently qualified from first contact.
/// It does not impose facing, button edges or movement admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxOpeningGate {
    /// Inclusive [left, top, right, bottom]; source gate is [120,368,152,400].
    pub raw_bounds: [u16; 4],
}
impl BoxOpeningGate {
    pub(crate) fn contains(self, (x, y): (u16, u16)) -> bool {
        let [left, top, right, bottom] = self.raw_bounds;
        (left..=right).contains(&x) && (top..=bottom).contains(&y)
    }
}
/// Opt-in source contract, bound by the enclosing aggregate data identity.
#[derive(Debug)]
pub struct PandoraData {
    pub(crate) text: PandoraText,
    pub(crate) rooms: Vec<ProfileRoom>,
    pub(crate) motions: Vec<MotionSpec>,
    pub(crate) contacts: [ContactSpec; 2],
    pub(crate) opening_gate: BoxOpeningGate,
    pub(crate) objects: Vec<SourceObject>,
    pub(crate) cellar_up_lanes: bool,
    pub(crate) navigation: Option<super::NavigationSpec>,
}
impl MotionKey {
    pub(crate) const fn maps(self) -> (u16, u16, bool) {
        use Invocation::{
            BoxWarning, OpeningFirst, OpeningFourth, OpeningSecond, OpeningThird, Tour42, Tour43,
            Tour44, TourFive, TourFour, TourIntro, TourLeave41, TourLeave42, TourLeave43,
            TourLeave44, TourOne, TourSix, TourThree, TourTwo,
        };
        match self {
            Self::Travel(t) => {
                let (a, b) = t.maps();
                (a, b, true)
            }
            Self::Cue(Cue::BoxReload) => (0x21, 0x21, true),
            Self::Cue(Cue::Returned(OpeningFourth)) => (0x21, 0x41, true),
            Self::Cue(Cue::Returned(TourLeave41)) => (0x41, 0x44, true),
            Self::Cue(Cue::Returned(TourLeave44)) => (0x44, 0x42, true),
            Self::Cue(Cue::Returned(TourLeave42)) => (0x42, 0x43, true),
            Self::Cue(Cue::Returned(TourLeave43)) => (0x43, 0x41, true),
            Self::Cue(
                Cue::BoxAcquireControl
                | Cue::Returned(BoxWarning | OpeningFirst | OpeningSecond | OpeningThird),
            ) => (0x21, 0x21, false),
            Self::Cue(Cue::Returned(
                TourIntro | TourOne | TourTwo | TourThree | TourFour | TourFive | TourSix,
            )) => (0x41, 0x41, false),
            Self::Cue(Cue::Returned(Tour44)) => (0x44, 0x44, false),
            Self::Cue(Cue::Returned(Tour42)) => (0x42, 0x42, false),
            Self::Cue(Cue::Returned(Tour43)) => (0x43, 0x43, false),
            Self::Cue(_) => (0xc, 0xc, false),
        }
    }
}
impl PandoraData {
    /// Validate bounded source structure; authentication and geometry qualification belong
    /// to the compiler, which hashes every field together with all existing house data.
    /// # Errors
    /// Rejects profile order/halos, pot catalog/door words, duplicate motions, wrong
    /// maps/reload markers, absent exit anchors or unbounded frame sequences.
    /// `cellar_up_lanes` is the compiler assertion of intact source lanes and direct
    /// occupancy, as documented by the pot component; false leaves throws unadmitted.
    #[allow(clippy::too_many_lines)] // Keep the coupled immutable contract admission together.
    pub fn new(
        text: PandoraText,
        rooms: Vec<ProfileRoom>,
        motions: Vec<MotionSpec>,
        contacts: [ContactSpec; 2],
        opening_gate: BoxOpeningGate,
        objects: Vec<SourceObject>,
        cellar_up_lanes: bool,
    ) -> Result<Self, SliceError> {
        if rooms.len() != 14 || motions.len() > 48 || objects.len() > 64 {
            return Err(SliceError::Data);
        }
        for (profile, key) in rooms.iter().zip(CollisionKey::ALL) {
            if profile.key != key
                || (profile.room.width(), profile.room.height()) != key.dimensions()
                || profile.room.sample_halo().is_none()
                || !valid_material_membership(key, &profile.room)
            {
                return Err(SliceError::Data);
            }
        }
        if contacts
            .iter()
            .map(|c| c.kind)
            .ne([ContactKind::Resident, ContactKind::BoxWarning])
        {
            return Err(SliceError::Data);
        }
        let [left, top, right, bottom] = opening_gate.raw_bounds;
        if left > right || top > bottom {
            return Err(SliceError::Data);
        }
        for (x, y) in [(left, top), (right, bottom)] {
            rooms[CollisionKey::Box as usize]
                .room
                .validate_position(x, y)
                .map_err(|_| SliceError::Data)?;
        }
        for contact in &contacts {
            let key = if contact.kind == ContactKind::Resident {
                CollisionKey::Resident
            } else {
                CollisionKey::Box
            };
            let room = &rooms[key as usize].room;
            for a in [contact.trigger, contact.result] {
                room.validate_position(a.position.0, a.position.1)
                    .map_err(|_| SliceError::Data)?;
            }
            if contact.kind != ContactKind::Resident
                && (contact.trigger.facing != Direction::Down
                    || contact.result.facing != Direction::Down)
            {
                return Err(SliceError::Data);
            }
        }
        for (n, motion) in motions.iter().enumerate() {
            if matches!(
                motion.key,
                MotionKey::Cue(Cue::Returned(
                    Invocation::ResidentFirst
                        | Invocation::ResidentRetry
                        | Invocation::ResidentRefusal
                        | Invocation::ResidentGrant
                        | Invocation::CChoice
                        | Invocation::CDirect
                        | Invocation::BoxEntry
                        | Invocation::TourFinal
                ))
            ) {
                return Err(SliceError::Data);
            }
            let (source, destination, reload) = motion.key.maps();
            if matches!(motion.key, MotionKey::Travel(_))
                && motions[..n].iter().any(|other| {
                    matches!(other.key, MotionKey::Travel(_))
                        && other.key.maps().0 == source
                        && other.trigger == motion.trigger
                })
            {
                return Err(SliceError::Data);
            }
            if motions[..n].iter().any(|m| m.key == motion.key)
                || motion.frames.is_empty()
                || motion.frames.len() > 4096
                || (matches!(motion.key, MotionKey::Travel(_)) && motion.trigger.is_none())
            {
                return Err(SliceError::Data);
            }
            let mut loaded = false;
            for frame in &motion.frames {
                if frame.reload {
                    if loaded || !reload {
                        return Err(SliceError::Data);
                    }
                    loaded = true;
                }
                let expected = if loaded { destination } else { source };
                if frame.map_id != expected
                    || frame.scene.map().is_some_and(|m| m != expected)
                    || (frame.scene == ScenePhase::House && expected != 0xd)
                {
                    return Err(SliceError::Data);
                }
                let MotionPose::Absolute(anchor) = frame.pose else {
                    if frame.reload || !matches!(motion.key, MotionKey::Cue(_)) {
                        return Err(SliceError::Data);
                    }
                    continue;
                };
                let (x, y) = anchor.position;
                if expected != 0xd {
                    rooms
                        .iter()
                        .find(|r| r.key.map() == expected)
                        .ok_or(SliceError::Data)?
                        .room
                        .validate_position(x, y)
                        .map_err(|_| SliceError::Data)?;
                }
            }
            if loaded != reload {
                return Err(SliceError::Data);
            }
            if let Some(a) = motion.trigger {
                rooms
                    .iter()
                    .find(|r| r.key.map() == source)
                    .ok_or(SliceError::Data)?
                    .room
                    .validate_position(a.position.0, a.position.1)
                    .map_err(|_| SliceError::Data)?;
            }
        }
        let data = Self {
            text,
            rooms,
            motions,
            contacts,
            opening_gate,
            objects,
            cellar_up_lanes,
            navigation: None,
        };
        for key in [
            CollisionKey::CClosed,
            CollisionKey::CDamaged,
            CollisionKey::CReaction,
            CollisionKey::COpen,
        ] {
            let room = data.room(key);
            if room.width() != 32 || room.height() != 64 {
                return Err(SliceError::Data);
            }
            for (n, object) in data.objects.iter().enumerate() {
                if !matches!(object.raw, 0x18fa | 0x18fb)
                    || object.replacement != 0x00f8
                    || room.cells().get(usize::from(object.cell)) != Some(&object.raw)
                    || (n > 0 && data.objects[n - 1].cell >= object.cell)
                {
                    return Err(SliceError::Data);
                }
            }
            if room.cells()[19 * 32 + 8] != 0x1cf2 || room.cells()[20 * 32 + 8] != 0x1cf3 {
                return Err(SliceError::Data);
            }
            let door = match key {
                CollisionKey::CClosed => [0x1d80, 0x0b81],
                CollisionKey::CDamaged => [0x1da7, 0x0b81],
                CollisionKey::CReaction => [0x9cf6, 0xbacb],
                _ => [0x1cf6, 0x3acb],
            };
            if room.cells()[20 * 32 + 11] != door[0] || room.cells()[21 * 32 + 11] != door[1] {
                return Err(SliceError::Data);
            }
        }
        Ok(data)
    }
    pub(crate) fn room(&self, key: CollisionKey) -> &Room {
        &self.rooms[key as usize].room
    }
    pub(crate) fn motion(&self, key: MotionKey) -> Option<(usize, &MotionSpec)> {
        self.motions.iter().enumerate().find(|(_, m)| m.key == key)
    }
}

impl MotionSpec {
    // Resolve only immutable constraints. A preserve-only prefix uses the existing
    // frozen owner coordinates and animation facing, not a duplicate runtime pose.
    pub(crate) fn pose_at(&self, cursor: u16) -> (Option<(u16, u16)>, Option<Direction>) {
        let mut position = self.trigger.map(|a| a.position);
        let mut facing = self.trigger.map(|a| a.facing);
        for frame in self.frames.iter().take(usize::from(cursor)) {
            match frame.pose {
                MotionPose::Absolute(a) => {
                    position = Some(a.position);
                    facing = Some(a.facing);
                }
                MotionPose::Preserve { facing: Some(f) } => facing = Some(f),
                MotionPose::Preserve { facing: None } => {}
            }
        }
        (position, facing)
    }
}

fn valid_material_membership(key: CollisionKey, room: &Room) -> bool {
    use crate::MaterialAlias;
    room.material_policy().iter().all(|rule| match rule.alias {
        MaterialAlias::TownSolid25 => key == CollisionKey::Town,
        MaterialAlias::ClosedDoorPartial5 => key.map() == 0xc && rule.bounds == [11, 21, 12, 22],
        MaterialAlias::StairOpen29 => match key {
            CollisionKey::CClosed
            | CollisionKey::CDamaged
            | CollisionKey::CReaction
            | CollisionKey::COpen => rule.bounds == [11, 21, 12, 22],
            CollisionKey::CellarE => rule.bounds == [6, 53, 7, 54],
            CollisionKey::Cellar20 => rule.bounds == [22, 53, 23, 54],
            _ => false,
        },
    })
}
