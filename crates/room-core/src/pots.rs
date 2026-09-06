//! Bounded FA/FB cellar pots, separate from story/door reactions.
//!
//! This is a narrow admission, not combat or a general projectile engine. The
//! compiler supplies immutable source collision (including direct-C residents),
//! source objects and lane admission. No captured memory or scene initializer is
//! used here. See `docs/pandora-pots.md` for the native/source proof boundary.

use crate::{Direction, FrameInput, MovementOutput, Room, Unqualified, WalkingState};

/// A source pot cell and its fully decoded replacement collision word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceObject {
    /// Row-major index in the immutable source grid.
    pub cell: u16,
    /// Full original collision word; low nine bits must be FA or FB.
    pub raw: u16,
    /// Full replacement word, reconstructed from held-record tile metadata.
    pub replacement: u16,
}

/// Immutable compiler-owned data and parent-owned hit admission.
///
/// Objects must be sorted by cell, unique, and number at most 64. The room retains
/// the original pot cells; the component overlays consumed cells on a private
/// copy for collision. Never supply the refusal branch's cleared occupancy.
pub struct Admission<'a> {
    /// Source collision with the qualified sample halo and passive solid policy.
    pub room: &'a Room,
    /// Canonical source object catalog. Identity must remain stable on restore.
    pub objects: &'a [SourceObject],
    /// Assert the two native Up lanes, including intervening geometry, are intact.
    /// This admits only (136,368) and (184,368), not nearby launch positions.
    /// Also admits Up-only geometry at exact cell (11,21): closed 0B81 as
    /// Partial, opened 3ACB as traversable. It never admits a stair transition.
    pub cellar_up_lanes: bool,
    /// Parent has admitted the source door callback (28 set, 292 clear).
    /// Disabling suppresses hit events; it does not create a new flight model.
    pub door_hit_enabled: bool,
}

/// Only neutral, one cardinal, or a one-frame A pulse is admitted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    /// Cardinal held this frame (one-frame latency).
    pub direction: Option<Direction>,
    /// A action pulse, also delayed one frame; held/repeated A fails closed.
    pub action: bool,
}

/// Pot control phase. Door requests and reactions are deliberately absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    /// No owned pot; ordinary walking or a qualified source lift is possible.
    Empty = 0,
    /// Source cell consumed; lift presentation has not returned to carry control.
    Lifting = 1,
    /// Held pot; COP83 cardinal carrying is admitted.
    Held = 2,
    /// Throw presentation, explicit release/flight, then control recovery.
    Throwing = 3,
}

/// A sampled world-space pot position during the admitted Up flight only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flight {
    /// World X (not a screen coordinate).
    pub x: u16,
    /// World Y; visual elevation/fragment animation belong to the sprite owner.
    pub y: u16,
}

/// Semantic output of one committed component tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Output {
    /// Collision-resolved walking/carrying, absent during lift/throw presentation.
    pub movement: Option<MovementOutput>,
    /// Exactly-once source removal event with the replacement word.
    pub consumed_cell: Option<SourceObject>,
    /// Ownership change: Some(Some(slot)) acquires; Some(None) releases.
    pub held_changed: Option<Option<u16>>,
    /// Explicit admitted flight sample; no arbitrary trajectories are synthesized.
    pub flight: Option<Flight>,
    /// Exactly-once native-qualified contact at the door callback boundary.
    /// The parent increments its counter and owns all reactions/requests.
    pub door_hit: bool,
    /// Player action control projection: 0020 while moving held, else 0000.
    /// Empty walking retains the ordinary 00A0 projection.
    pub control: u16,
    /// Component throw recovery, not proof that parent story input is enabled.
    pub control_restored: bool,
}

/// Fail-closed admission error; the complete component is unchanged on error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Collision/coordinate/onset failure from the shared solver.
    Walking(Unqualified),
    /// Invalid source catalog, raw membership or replacement data.
    Source,
    /// No unconsumed admitted source pot at this facing/sample.
    NoSourceObject,
    /// Combined/repeated actions, moving action, or input during presentation.
    Input,
    /// Launch position/direction or immutable lane is outside the native proof.
    ThrowLane,
    /// Malformed or noncanonical snapshot/component invariants.
    Snapshot,
}
impl From<Unqualified> for Error {
    fn from(value: Unqualified) -> Self {
        Self::Walking(value)
    }
}
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "unqualified cellar pot: {self:?}")
    }
}
impl core::error::Error for Error {}

/// Local component snapshot size. No game/asset/profile schema is changed.
pub const SNAPSHOT_SIZE: usize = 40;

/// Deterministic pot ownership, movement history and bounded action continuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PotState {
    walking: WalkingState,
    consumed: u64,
    facing: Direction,
    phase: Phase,
    age: u8,
    object: u8, // catalog index + 1; zero only in Empty
    delayed_action: bool,
}

impl Admission<'_> {
    fn validate(&self) -> Result<(), Error> {
        if self.objects.len() > 64 {
            return Err(Error::Source);
        }
        for (i, object) in self.objects.iter().enumerate() {
            if !matches!(object.raw, 0x18fa | 0x18fb)
                || object.replacement != 0x00f8
                || self.room.cells().get(usize::from(object.cell)) != Some(&object.raw)
                || (i > 0 && self.objects[i - 1].cell >= object.cell)
            {
                return Err(Error::Source);
            }
        }
        Ok(())
    }
    fn collision(&self, consumed: u64, up: bool) -> Room {
        let mut room = self.room.clone();
        for (i, object) in self.objects.iter().enumerate() {
            if consumed & (1 << i) != 0 {
                room.replace_cell(usize::from(object.cell), object.replacement);
            }
        }
        if self.cellar_up_lanes && up && room.width() > 11 && room.height() > 21 {
            let cell = 21 * usize::from(room.width()) + 11;
            // Private collision classifiers, NEVER source words/patch events.
            // Type5 is P16 here. Type29 is Open only for admitted Up: Right
            // S-first's type29 entry differs, so a global alias would be wrong.
            match room.cells()[cell] {
                0x0b81 => room.replace_cell(cell, 16 << 9),
                0x3acb => room.replace_cell(cell, 0),
                _ => {} // BACB remains flagged solid; unknown words fail closed.
            }
        }
        room
    }
}

impl PotState {
    // Aggregate-only qualified forced motion: preserve the visit ledger, never a new pot.
    pub(crate) fn rebase(&mut self, walking: WalkingState, facing: Direction) -> Result<(), Error> {
        if self.phase != Phase::Empty || self.delayed_action {
            return Err(Error::Input);
        }
        self.walking = walking;
        self.facing = facing;
        Ok(())
    }

    /// Enter from a parent-owned ordinary walking state, without a WRAM initializer.
    /// The parent must retain this component/ledger across actions in this room.
    ///
    /// # Errors
    /// Rejects invalid source data or out-of-room player bounds.
    pub fn new(a: &Admission<'_>, walking: WalkingState, facing: Direction) -> Result<Self, Error> {
        a.validate()?;
        a.room.validate_position(walking.x(), walking.y())?;
        Ok(Self {
            walking,
            consumed: 0,
            facing,
            phase: Phase::Empty,
            age: 0,
            object: 0,
            delayed_action: false,
        })
    }
    /// Player position; only the collision solver can change it after construction.
    #[must_use]
    pub const fn position(&self) -> (u16, u16) {
        self.walking.position()
    }
    /// Current facing, including delayed turns.
    #[must_use]
    pub const fn facing(&self) -> Direction {
        self.facing
    }
    /// Current control phase.
    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }
    /// Elapsed lift/throw presentation ticks; zero in Empty/Held.
    #[must_use]
    pub const fn phase_tick(&self) -> u8 {
        self.age
    }
    /// Read-only movement continuation for parent animation and eventual handoff.
    /// Current qualified pre-fragment world sample, independent of a renderer.
    /// None before release and at/after flight break; recovery may still own control.
    #[must_use]
    pub fn flight(&self) -> Option<Flight> {
        if self.phase != Phase::Throwing || self.age < 18 || self.age - 18 >= self.flight_length() {
            return None;
        }
        Some(Flight {
            x: self.position().0,
            y: 357 - 3 * u16::from(self.age - 18),
        })
    }
    /// The enclosing pot snapshot, not this walker alone, owns the carry cadence.
    #[must_use]
    pub const fn walking(&self) -> &WalkingState {
        &self.walking
    }
    /// Slot of the object still in Ark's hands. FA=098A, FB=098F.
    /// After release this is None even though native 0988 remains reserved until break.
    #[must_use]
    pub fn held_slot_in(&self, a: &Admission<'_>) -> Option<u16> {
        if self.phase == Phase::Throwing && self.age >= 18 {
            return None;
        }
        self.reserved_slot_in(a)
    }
    /// Native 0988 reservation: retained during flight, cleared at source 84BFE8
    /// on break (throw age22 at the door, age27 in the miss lane).
    #[must_use]
    pub fn reserved_slot_in(&self, a: &Admission<'_>) -> Option<u16> {
        let end = 18 + self.flight_length();
        if self.object == 0 || (self.phase == Phase::Throwing && self.age >= end) {
            return None;
        }
        a.objects.get(usize::from(self.object - 1)).map(|o| {
            if o.raw & 511 == 0xfa {
                0x98a
            } else {
                0x98f
            }
        })
    }
    /// Whether the source cell has been consumed in this component's room visit.
    #[must_use]
    pub fn consumed_in(&self, a: &Admission<'_>, cell: u16) -> bool {
        a.objects
            .iter()
            .position(|o| o.cell == cell)
            .is_some_and(|i| i < 64 && self.consumed & (1 << i) != 0)
    }
    fn flight_length(&self) -> u8 {
        if self.position().0 == 184 {
            4
        } else {
            9
        }
    }
    fn stationary(&self) -> bool {
        self.walking.active_direction().is_none()
            && self.walking.delayed_direction().is_none()
            && self.walking.onset_remaining() == 0
    }
    fn source_object(&self, a: &Admission<'_>) -> Result<usize, Error> {
        // Only these source/native lift poses are admitted; not arbitrary grabs.
        let (x, y) = self.position();
        let sx = match (x, y, self.facing) {
            (104 | 88, 352, Direction::Left) => x - 16,
            (40, 352, Direction::Right) => x + 16,
            _ => return Err(Error::NoSourceObject),
        };
        let cell = ((y - 8) / 16)
            .checked_mul(a.room.width())
            .and_then(|v| v.checked_add(sx / 16))
            .ok_or(Error::Source)?;
        a.objects
            .iter()
            .position(|o| o.cell == cell && !self.consumed_in(a, cell))
            .ok_or(Error::NoSourceObject)
    }
    fn lane(&self, a: &Admission<'_>) -> Result<(), Error> {
        if !a.cellar_up_lanes
            || self.facing != Direction::Up
            || !matches!(self.position(), (136 | 184, 368))
            || a.room.width() <= 11
            || a.room.height() <= 21
        {
            return Err(Error::ThrowLane);
        }
        let width = usize::from(a.room.width());
        if a.room.cells()[21 * width + 11] != 0x0b81
            || !matches!(a.room.cells()[20 * width + 11], 0x1d80 | 0x1da7)
        {
            return Err(Error::ThrowLane);
        }
        Ok(())
    }

    fn admit_action(&self, a: &Admission<'_>) -> Result<(), Error> {
        match self.phase {
            Phase::Empty => self.source_object(a).map(|_| ()),
            Phase::Held => self.lane(a),
            _ => Err(Error::Input),
        }
    }

    /// Resolve one tick transactionally, including delayed A and semantic events.
    ///
    /// Actions require settled neutral movement (including expired onset window).
    /// Only neutral input is admitted during lift/throw presentation. A parent must
    /// stop/handoff for story reactions rather than interpret recovery as permission.
    ///
    /// # Errors
    /// Any unsupported input, collision, source or lane leaves all fields unchanged.
    pub fn step(&mut self, a: &Admission<'_>, input: Input) -> Result<Output, Error> {
        a.validate()?;
        if (input.action
            && (input.direction.is_some() || self.delayed_action || !self.stationary()))
            || (matches!(self.phase, Phase::Lifting | Phase::Throwing)
                && (input.action || input.direction.is_some()))
            || (self.delayed_action && input.direction.is_some())
        {
            return Err(Error::Input);
        }
        // Reject an unadmitted sample now, rather than leaving a poisoned delayed
        // action in an otherwise successfully committed state.
        if input.action {
            self.admit_action(a)?;
        }
        let mut next = *self;
        let mut out = Output {
            movement: None,
            consumed_cell: None,
            held_changed: None,
            flight: None,
            door_hit: false,
            control: 0,
            control_restored: false,
        };
        if self.delayed_action {
            match self.phase {
                Phase::Empty => {
                    let i = self.source_object(a)?;
                    next.object = u8::try_from(i + 1).map_err(|_| Error::Source)?;
                    next.consumed |= 1 << i;
                    next.phase = Phase::Lifting;
                    out.consumed_cell = Some(a.objects[i]);
                    out.held_changed = Some(next.held_slot_in(a));
                }
                Phase::Held => {
                    self.lane(a)?;
                    next.phase = Phase::Throwing;
                }
                _ => return Err(Error::Input),
            }
            next.age = 0;
        } else {
            match self.phase {
                Phase::Lifting => {
                    next.age += 1;
                    if next.age == 23 {
                        next.phase = Phase::Held;
                        next.age = 0;
                    }
                }
                Phase::Throwing => {
                    self.lane(a)?;
                    next.age += 1;
                    if next.age == 18 {
                        out.held_changed = Some(None);
                    }
                    if next.age == 32 {
                        next.phase = Phase::Empty;
                        next.age = 0;
                        next.object = 0;
                        out.control_restored = true;
                    }
                }
                Phase::Empty | Phase::Held => {
                    let held = self.phase == Phase::Held;
                    let collision = a.collision(
                        self.consumed,
                        self.walking.delayed_direction() == Some(Direction::Up),
                    );
                    out.movement = Some(next.walking.step_with_cadence(
                        &collision,
                        FrameInput {
                            direction: input.direction,
                        },
                        held,
                    )?);
                    if let Some(d) = next.walking.active_direction() {
                        next.facing = d;
                        out.control = if held { 0x20 } else { 0xa0 };
                    }
                }
            }
        }
        if next.phase == Phase::Throwing && next.age >= 18 {
            let flight_age = next.age - 18;
            out.flight = next.flight();
            // Exact admitted flight contact, not player proximity or consumption.
            // The generic COP65 collision dispatcher is outside this component.
            out.door_hit = a.door_hit_enabled
                && next.facing == Direction::Up
                && out.flight == Some(Flight { x: 184, y: 354 })
                && flight_age == 1;
        }
        next.delayed_action = input.action;
        *self = next;
        Ok(out)
    }

    /// Canonical 40-byte encoding: POT1, embedded walking16, consumed/u64 LE,
    /// facing, phase, age, object index+1, delayed A, seven reserved zeros.
    /// Parent snapshot binds room/catalog/lane and story identity, not this format.
    #[must_use]
    pub fn encode_snapshot(&self) -> [u8; SNAPSHOT_SIZE] {
        let mut b = [0; SNAPSHOT_SIZE];
        b[..4].copy_from_slice(b"POT1");
        b[4..20].copy_from_slice(&self.walking.encode_snapshot());
        b[20..28].copy_from_slice(&self.consumed.to_le_bytes());
        b[28..33].copy_from_slice(&[
            self.facing as u8,
            self.phase as u8,
            self.age,
            self.object,
            u8::from(self.delayed_action),
        ]);
        b
    }
    /// Decode structural invariants against immutable source data. This does not
    /// authenticate provenance or authorize a native restore as fresh evidence.
    ///
    /// # Errors
    /// Rejects noncanonical, truncated, stale-source or impossible phase encodings.
    pub fn decode_snapshot(a: &Admission<'_>, b: &[u8]) -> Result<Self, Error> {
        a.validate()?;
        if b.len() != SNAPSHOT_SIZE
            || &b[..4] != b"POT1"
            || b[33..].iter().any(|&v| v != 0)
            || b[32] > 1
        {
            return Err(Error::Snapshot);
        }
        let facing = match b[28] {
            0 => Direction::Down,
            1 => Direction::Up,
            2 => Direction::Left,
            3 => Direction::Right,
            _ => return Err(Error::Snapshot),
        };
        let phase = match b[29] {
            0 => Phase::Empty,
            1 => Phase::Lifting,
            2 => Phase::Held,
            3 => Phase::Throwing,
            _ => return Err(Error::Snapshot),
        };
        let consumed = u64::from_le_bytes(b[20..28].try_into().map_err(|_| Error::Snapshot)?);
        if a.objects.len() < 64 && consumed >> a.objects.len() != 0 {
            return Err(Error::Snapshot);
        }
        let state = Self {
            walking: WalkingState::decode_snapshot(a.room, &b[4..20])?,
            consumed,
            facing,
            phase,
            age: b[30],
            object: b[31],
            delayed_action: b[32] != 0,
        };
        let object_valid = state.object > 0
            && usize::from(state.object) <= a.objects.len()
            && consumed & (1 << (state.object - 1)) != 0;
        let valid = match phase {
            Phase::Empty => state.age == 0 && state.object == 0,
            Phase::Lifting => {
                object_valid && state.age < 23 && state.stationary() && !state.delayed_action
            }
            Phase::Held => object_valid && state.age == 0 && state.walking.phase() <= 2,
            Phase::Throwing => {
                object_valid
                    && state.age < 32
                    && state.stationary()
                    && !state.delayed_action
                    && state.lane(a).is_ok()
            }
        };
        let queued_valid = !state.delayed_action || state.admit_action(a).is_ok();
        if !valid || !queued_valid || (state.delayed_action && !state.stationary()) {
            return Err(Error::Snapshot);
        }
        Ok(state)
    }
}
