//! FA/FB pots: the lift, the carry, the throw and the flight by the native
//! rules (`docs/pots.md`), separate from story and door reactions.
//!
//! The compiler supplies immutable source collision and source objects; the
//! parent strikes its own actors under a flight. No captured memory or scene
//! initializer is used here. The cellar's recorded segments
//! (`docs/pandora-pots.md`) remain exact.

use crate::{
    Direction, FrameInput, MaterialAlias, MaterialRule, MovementOutput, Room, Unqualified,
    WalkingState,
};

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
    /// Admits the cellar's Up-only geometry at exact cell (11,21) for
    /// walking: closed 0B81 as Partial, opened 3ACB as traversable. It never
    /// admits a stair transition.
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

/// A flying pot's world-space ground point.
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
    /// The cellar door's callback counts a hit this frame: the frame after
    /// the flight's box overlapped the door's (x±8, y±8 each, inclusive).
    /// Parents with actors of their own scan their hittable boxes instead.
    pub door_hit: bool,
    /// Player action control projection: 0020 while moving held, else 0000.
    /// Empty walking retains the ordinary 00A0 projection.
    pub control: u16,
    /// Component throw recovery, not proof that parent story input is enabled.
    pub control_restored: bool,
    /// The pot broke this frame, at its last ground point.
    pub broke: Option<Flight>,
    /// The sound the pot makes this frame, besides the lift's.
    pub sound: Option<Sound>,
}

/// A flying pot's sounds on port 3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    /// It leaves Ark's hands (`$12`).
    Release,
    /// It breaks (`$13`): at the sample a tile stops it, or at the landing
    /// sample (natively jittering a frame later at times). It wins over a
    /// release in the same frame, as the port's latch keeps the last.
    Break,
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

/// The flight's height above its ground point (`$0999`) for samples 1..19:
/// the delta stream `$84:C1F0` that `COP AF` selects, the same in every
/// facing (`docs/pots.md`).
const HEIGHTS: [i8; 19] = [
    -33, -34, -35, -35, -35, -35, -34, -33, -32, -30, -28, -26, -23, -20, -17, -13, -9, -5, 0,
];
/// Frames of the lift presentation, of the throw until the release, and of
/// the throw until Ark's control returns (`$84:BE9D`, `$84:B558`).
const LIFT: u8 = 23;
const RELEASE: u8 = 18;
const RECOVERY: u8 = 32;
/// A flight's speed and the launch point's distance from Ark.
const SPEED: u16 = 3;
const LAUNCH: u16 = 10;
/// The cellar door actor, whose callback counts the hits (`$0640`).
const DOOR: (u16, u16) = (184, 352);
/// Collision types a flight stops on (`$80:D1FB`, pot `+$16` = 0).
const BLOCKING: u32 = 1 << 0x05 | 0xF << 0x08 | 0x7 << 0x0D | 1 << 0x17 | 1 << 0x18 | 0x3F << 0x1A;

/// A pot in flight toward `facing`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Flying {
    x: u16,
    y: u16,
    facing: Direction,
    /// Samples shown, 1..=19.
    sample: u8,
    /// A tile stopped it at this sample; it breaks next frame.
    stopped: bool,
    door: DoorContact,
}

/// The cellar door's contact with one flight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DoorContact {
    Clear,
    /// Its box overlapped the door's this frame; the door counts next frame.
    Struck,
    Counted,
}

/// Deterministic pot ownership, movement history and bounded action continuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PotState {
    walking: WalkingState,
    consumed: u64,
    facing: Direction,
    phase: Phase,
    age: u8,
    object: u8, // catalog index + 1; zero when no pot is held or flying
    /// An A press taken this frame, acting next frame: a lift of the
    /// catalog object (index + 1), or a throw (0).
    queued: Option<u8>,
    flight: Option<Flying>,
    /// The cellar door counted this throw's pot; until the throw ends.
    counted: bool,
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
    fn collision(&self, consumed: u64) -> Result<Room, Error> {
        let mut room = self.room.clone();
        for (i, object) in self.objects.iter().enumerate() {
            if consumed & (1 << i) != 0 {
                let cell = usize::from(object.cell);
                room.replace_cell(cell, object.replacement | (room.cells()[cell] & 0x8000));
            }
        }
        let admitted_cell = room.sample_halo().is_none_or(|[left, top, right, bottom]| {
            left <= 11 && top <= 21 && 11 < right && 21 < bottom
        });
        if self.cellar_up_lanes && admitted_cell && room.width() > 11 && room.height() > 21 {
            let cell = 21 * usize::from(room.width()) + 11;
            // Preserve standalone POT1's exact-word qualification, but classify
            // raw words through the same delayed-direction solver as Pandora.
            let alias = match room.cells()[cell] {
                0x0b81 => Some(MaterialAlias::ClosedDoorPartial5),
                0x3acb => Some(MaterialAlias::StairOpen29),
                _ => None, // BACB remains flagged solid; no inferred aliases.
            };
            if let Some(alias) = alias {
                let rule = MaterialRule {
                    bounds: [11, 21, 12, 22],
                    direction: Some(Direction::Up),
                    alias,
                };
                if !room.material_policy().contains(&rule) {
                    let mut rules = room.material_policy().to_vec();
                    rules.push(rule);
                    room = room
                        .with_material_policy(rules)
                        .map_err(|_| Error::Source)?;
                }
            }
        }
        Ok(room)
    }
}

impl PotState {
    pub(crate) const fn ledger(self) -> u64 {
        self.consumed
    }
    // Aggregate-only accounting for the admitted callback lane.
    pub(crate) const fn contact_reached(self) -> bool {
        self.counted
    }
    pub(crate) const fn idle_empty(self) -> bool {
        matches!(self.phase, Phase::Empty) && self.queued.is_none() && self.flight.is_none()
    }
    pub(crate) fn with_ledger(
        a: &Admission<'_>,
        walking: WalkingState,
        facing: Direction,
        consumed: u64,
    ) -> Result<Self, Error> {
        let mut state = Self::new(a, walking, facing)?;
        if a.objects.len() < 64 && consumed >> a.objects.len() != 0 {
            return Err(Error::Source);
        }
        state.consumed = consumed;
        Ok(state)
    }
    /// Hands back the parent's walking between actions, keeping the room visit's
    /// consumed-cell ledger; for parents that own ordinary walking while no pot
    /// is held. A pot may still be flying.
    ///
    /// # Errors
    /// Refuses while a pot is held or an A press is queued.
    pub fn rebase(&mut self, walking: WalkingState, facing: Direction) -> Result<(), Error> {
        if self.phase != Phase::Empty || self.queued.is_some() {
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
            queued: None,
            flight: None,
            counted: false,
        })
    }
    /// Player position; only the collision solver can change it after construction.
    #[must_use]
    pub const fn position(&self) -> (u16, u16) {
        self.walking.position()
    }
    /// Drops the pot Ark carries (`$84:C5CB`), as an exit does: it goes,
    /// unbroken, and its cell stays lifted. Returns whether he held one.
    pub fn drop_held(&mut self) -> bool {
        if self.phase != Phase::Held {
            return false;
        }
        (self.phase, self.object, self.queued) = (Phase::Empty, 0, None);
        true
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
    /// The flying pot's ground point this frame; `None` before the release
    /// and from its break. Ark may already walk again.
    #[must_use]
    pub fn flight(&self) -> Option<Flight> {
        self.flight.map(|flight| Flight {
            x: flight.x,
            y: flight.y,
        })
    }
    /// The flying pot's height above its ground point (`$0999`, negative up).
    #[must_use]
    pub fn flight_height(&self) -> Option<i16> {
        self.flight
            .map(|flight| i16::from(HEIGHTS[usize::from(flight.sample - 1)]))
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
        if self.flight.is_some() {
            return None;
        }
        self.reserved_slot_in(a)
    }
    /// Native 0988 reservation: retained during flight, cleared at the break
    /// (`$84:BFE8`).
    #[must_use]
    pub fn reserved_slot_in(&self, a: &Admission<'_>) -> Option<u16> {
        let index = usize::from(self.object.checked_sub(1)?);
        a.objects
            .get(index)
            .map(|o| if o.raw & 511 == 0xfa { 0x98a } else { 0x98f })
    }
    /// Whether the source cell has been consumed in this component's room visit.
    #[must_use]
    pub fn consumed_in(&self, a: &Admission<'_>, cell: u16) -> bool {
        a.objects
            .iter()
            .position(|o| o.cell == cell)
            .is_some_and(|i| i < 64 && self.consumed & (1 << i) != 0)
    }
    /// The unconsumed source pot A lifts from here (`$87:9254`): the cell
    /// one probe ahead, (x, y+8) Down, (x, y−24) Up, (x∓16, y−8) Left or
    /// Right, with y ≡ 0 (mod 16) facing up or down and x ≡ 8 (mod 16)
    /// facing left or right.
    fn source_object(&self, a: &Admission<'_>) -> Result<usize, Error> {
        let (x, y) = self.position();
        let (aligned, probe) = match self.facing {
            Direction::Down => (y % 16 == 0, Some((x, y + 8))),
            Direction::Up => (y % 16 == 0, y.checked_sub(24).map(|y| (x, y))),
            Direction::Left => (x % 16 == 8, x.checked_sub(16).zip(y.checked_sub(8))),
            Direction::Right => (x % 16 == 8, y.checked_sub(8).map(|y| (x + 16, y))),
        };
        let (Some((px, py)), true) = (probe, aligned) else {
            return Err(Error::NoSourceObject);
        };
        let cell = (py / 16)
            .checked_mul(a.room.width())
            .and_then(|v| v.checked_add(px / 16))
            .ok_or(Error::Source)?;
        a.objects
            .iter()
            .position(|o| o.cell == cell && !self.consumed_in(a, cell))
            .ok_or(Error::NoSourceObject)
    }

    /// An A press: a lift from Empty when a source pot is ahead, a throw
    /// with a pot in hand; ignored during the lift and the throw.
    fn admit_action(&self, a: &Admission<'_>) -> Result<Option<u8>, Error> {
        if self.queued.is_some() {
            return Ok(None);
        }
        match self.phase {
            Phase::Empty if self.flight.is_none() => {
                let index = self.source_object(a)?;
                Ok(Some(u8::try_from(index + 1).map_err(|_| Error::Source)?))
            }
            // One pot at a time: `$0988` stays set until the break.
            Phase::Empty => Err(Error::NoSourceObject),
            Phase::Held => Ok(Some(0)),
            Phase::Lifting | Phase::Throwing => Ok(None),
        }
    }

    /// Resolve one tick transactionally, including the queued A and semantic events.
    ///
    /// An A press acts next frame and stops a walk; directions and A are
    /// ignored during the lift and the throw.
    ///
    /// # Errors
    /// An A press with nothing to lift and no pot (the parent talks
    /// instead), or a collision failure, leaves all fields unchanged.
    pub fn step(&mut self, a: &Admission<'_>, input: Input) -> Result<Output, Error> {
        let ground = a.collision(self.consumed)?;
        self.step_over(a, &ground, input)
    }

    /// As [`Self::step`], a flight probing `ground` for the tiles that stop
    /// it: the map without the cells actors occupy, which a pot flies
    /// through (`docs/pots.md`), while Ark walks on the admission's grid.
    ///
    /// # Errors
    /// As [`Self::step`].
    pub fn step_over(
        &mut self,
        a: &Admission<'_>,
        ground: &Room,
        input: Input,
    ) -> Result<Output, Error> {
        a.validate()?;
        let mut next = *self;
        let mut out = Output {
            movement: None,
            consumed_cell: None,
            held_changed: None,
            flight: None,
            door_hit: false,
            control: 0,
            control_restored: false,
            broke: None,
            sound: None,
        };
        next.fly(a, ground, &mut out);
        let (x, y) = self.position();
        match (self.queued, self.phase) {
            (Some(object @ 1..), _) => {
                let i = usize::from(object - 1);
                next.object = object;
                next.consumed |= 1 << i;
                next.phase = Phase::Lifting;
                next.age = 0;
                next.walking = WalkingState::new(x, y);
                out.consumed_cell = Some(a.objects[i]);
                out.held_changed = Some(next.held_slot_in(a));
            }
            (Some(_), _) => {
                next.phase = Phase::Throwing;
                next.age = 0;
                next.walking = WalkingState::new(x, y);
            }
            (None, Phase::Lifting) => {
                next.age += 1;
                if next.age == LIFT {
                    next.phase = Phase::Held;
                    next.age = 0;
                    // Carry control starts this frame: a held direction
                    // registers now (`$84:B4FC`).
                    let collision = a.collision(self.consumed)?;
                    out.movement = Some(next.walking.step_with_cadence(
                        &collision,
                        FrameInput {
                            direction: input.direction,
                        },
                        true,
                    )?);
                }
            }
            (None, Phase::Throwing) => {
                next.age += 1;
                if next.age == RELEASE {
                    next.release(ground, &mut out);
                }
                if next.age == RECOVERY {
                    next.phase = Phase::Empty;
                    next.age = 0;
                    out.control_restored = true;
                    // Control returns this frame: a direction held through
                    // the throw is walking already, and moves from the next.
                    if let Some(direction) = input.direction {
                        next.walking = WalkingState::walking(x, y, direction, 0);
                        next.facing = direction;
                        out.control = 0xa0;
                    }
                }
            }
            (None, Phase::Empty | Phase::Held) => {
                let held = self.phase == Phase::Held;
                let collision = a.collision(self.consumed)?;
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
        // The A handler runs after the frame's move (`$87:9254`): walking
        // into a pot, the aligned frame lifts it.
        // The count lasts while the throw or its flight does.
        next.counted &= next.phase == Phase::Throwing || next.flight.is_some();
        next.queued = if input.action {
            next.admit_action(a)?
        } else {
            None
        };
        out.flight = next.flight();
        // A tile's stop sounds at its sample, a landing at the last one.
        if next
            .flight
            .is_some_and(|after| after.stopped || usize::from(after.sample) == HEIGHTS.len())
        {
            out.sound = Some(Sound::Break);
        }
        *self = next;
        Ok(out)
    }

    /// The release (`$84:C721`): the pot leaves Ark's hands at the launch
    /// point, (x, y+10) Down, (x, y−8) Up, (x∓10, y) sideways, and takes
    /// its first step.
    fn release(&mut self, ground: &Room, out: &mut Output) {
        let (x, y) = self.position();
        let (x, y) = match self.facing {
            Direction::Down => (x, y + LAUNCH),
            Direction::Up => (x, y.saturating_sub(LAUNCH - 2)),
            Direction::Left => (x.saturating_sub(LAUNCH), y),
            Direction::Right => (x + LAUNCH, y),
        };
        self.flight = Some(Flying {
            x,
            y,
            facing: self.facing,
            sample: 0,
            stopped: false,
            door: DoorContact::Clear,
        });
        out.held_changed = Some(None);
        out.sound = Some(Sound::Release);
        self.advance_flight(ground);
    }

    /// One frame of an earlier release's flight: a step, or the break after
    /// its last sample or a tile it stopped on.
    fn fly(&mut self, a: &Admission<'_>, ground: &Room, out: &mut Output) {
        let Some(flight) = self.flight else {
            return;
        };
        if flight.door == DoorContact::Struck {
            out.door_hit = a.door_hit_enabled;
            self.counted |= out.door_hit;
        }
        if flight.stopped || usize::from(flight.sample) == HEIGHTS.len() {
            self.flight = None;
            self.object = 0;
            out.broke = Some(Flight {
                x: flight.x,
                y: flight.y,
            });
            return;
        }
        self.advance_flight(ground);
    }

    /// A step of 3 pixels, the probe for a stopping tile (`$80:D1A9`: at y−8
    /// moving sideways, y+2 up, y−16 down), and the door's box.
    fn advance_flight(&mut self, ground: &Room) {
        let Some(mut flight) = self.flight else {
            return;
        };
        let step = match flight.facing {
            Direction::Down => Some((flight.x, flight.y + SPEED)),
            Direction::Up => flight.y.checked_sub(SPEED).map(|y| (flight.x, y)),
            Direction::Left => flight.x.checked_sub(SPEED).map(|x| (x, flight.y)),
            Direction::Right => Some((flight.x + SPEED, flight.y)),
        };
        flight.sample += 1;
        // The map's edge stops it where it is.
        let Some((x, y)) = step else {
            flight.stopped = true;
            self.flight = Some(flight);
            return;
        };
        (flight.x, flight.y) = (x, y);
        let probe_y = match flight.facing {
            Direction::Left | Direction::Right => flight.y.wrapping_sub(8),
            Direction::Up => flight.y + 2,
            Direction::Down => flight.y.wrapping_sub(16),
        };
        let (column, row) = (flight.x / 16, probe_y / 16);
        flight.stopped = column >= ground.width()
            || row >= ground.height()
            || BLOCKING
                & 1 << (ground.cells()
                    [usize::from(row) * usize::from(ground.width()) + usize::from(column)]
                    >> 9
                    & 0x1F)
                != 0;
        flight.door = match flight.door {
            DoorContact::Clear
                if flight.x.abs_diff(DOOR.0) <= 16 && flight.y.abs_diff(DOOR.1) <= 16 =>
            {
                DoorContact::Struck
            }
            DoorContact::Struck => DoorContact::Counted,
            door => door,
        };
        self.flight = Some(flight);
    }

    /// Canonical 40-byte encoding: POT1, embedded walking16, consumed/u64 LE,
    /// facing, phase, age, object index+1, the queued A (0 none, 1 a throw,
    /// else 1 + the lifted object index + 1), then the flight: its sample (0
    /// none) with bit 7 stopped and bits 5–6 the door contact, x, y LE, its
    /// facing, and whether the cellar door counted this throw.
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
            self.queued.map_or(0, |object| object + 1),
        ]);
        if let Some(flight) = self.flight {
            b[33] = flight.sample | u8::from(flight.stopped) << 7 | (flight.door as u8) << 5;
            b[34..36].copy_from_slice(&flight.x.to_le_bytes());
            b[36..38].copy_from_slice(&flight.y.to_le_bytes());
            b[38] = flight.facing as u8;
        }
        b[39] = u8::from(self.counted);
        b
    }
    /// Decode structural invariants against immutable source data. This does not
    /// authenticate provenance or authorize a native restore as fresh evidence.
    ///
    /// # Errors
    /// Rejects noncanonical, truncated, stale-source or impossible phase encodings.
    pub fn decode_snapshot(a: &Admission<'_>, b: &[u8]) -> Result<Self, Error> {
        a.validate()?;
        if b.len() != SNAPSHOT_SIZE || &b[..4] != b"POT1" || b[39] > 1 {
            return Err(Error::Snapshot);
        }
        let facing = |byte: u8| match byte {
            0 => Ok(Direction::Down),
            1 => Ok(Direction::Up),
            2 => Ok(Direction::Left),
            3 => Ok(Direction::Right),
            _ => Err(Error::Snapshot),
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
        let sample = b[33] & 0x1F;
        let flight = if sample == 0 {
            if b[33..39].iter().any(|&v| v != 0) {
                return Err(Error::Snapshot);
            }
            None
        } else {
            let door = match b[33] >> 5 & 3 {
                0 => DoorContact::Clear,
                1 => DoorContact::Struck,
                2 => DoorContact::Counted,
                _ => return Err(Error::Snapshot),
            };
            Some(Flying {
                x: u16::from_le_bytes([b[34], b[35]]),
                y: u16::from_le_bytes([b[36], b[37]]),
                facing: facing(b[38])?,
                sample,
                stopped: b[33] & 0x80 != 0,
                door,
            })
        };
        let state = Self {
            walking: WalkingState::decode_snapshot(a.room, &b[4..20])?,
            consumed,
            facing: facing(b[28])?,
            phase,
            age: b[30],
            object: b[31],
            queued: b[32].checked_sub(1),
            flight,
            counted: b[39] == 1,
        };
        let object_valid = state.object > 0
            && usize::from(state.object) <= a.objects.len()
            && consumed & (1 << (state.object - 1)) != 0;
        let valid = match phase {
            Phase::Empty => state.age == 0 && (state.object == 0 || state.flight.is_some()),
            Phase::Lifting => object_valid && state.age < LIFT && state.flight.is_none(),
            Phase::Held => {
                object_valid
                    && state.age == 0
                    && state.flight.is_none()
                    && state.walking.phase() <= 2
            }
            // After the break, the recovery goes on without a pot.
            Phase::Throwing => {
                state.age < RECOVERY
                    && (object_valid
                        || (state.object == 0 && state.age > RELEASE && state.flight.is_none()))
            }
        };
        let flight_valid = state
            .flight
            .is_none_or(|flight| usize::from(flight.sample) <= HEIGHTS.len() && object_valid);
        let queued_valid = match state.queued {
            None => true,
            Some(0) => phase == Phase::Held,
            Some(object) => {
                phase == Phase::Empty
                    && state.flight.is_none()
                    && usize::from(object) <= a.objects.len()
                    && consumed & (1 << (object - 1)) == 0
            }
        };
        // A released pot is in the air until it breaks; before the release,
        // it is in Ark's hands.
        let released = phase == Phase::Throwing && state.age >= RELEASE;
        let flight_age = match (phase, state.flight) {
            (Phase::Throwing, Some(flight)) => released && flight.sample == state.age - RELEASE + 1,
            (Phase::Throwing, None) => !released || state.object == 0,
            _ => true,
        };
        let counted_valid = !state.counted || phase == Phase::Throwing || state.flight.is_some();
        if !valid || !flight_valid || !queued_valid || !flight_age || !counted_valid {
            return Err(Error::Snapshot);
        }
        Ok(state)
    }
}

#[cfg(test)]
mod raw_policy_tests {
    use super::*;
    #[test]
    fn preinstalled_rules_and_outside_halo_do_not_add_duplicate_admission() {
        let mut cells = alloc::vec![0; 32 * 64];
        cells[21 * 32 + 11] = 0x3acb;
        let room = Room::new_passive(32, 64, cells).unwrap();
        let rule = MaterialRule {
            bounds: [11, 21, 12, 22],
            direction: Some(Direction::Up),
            alias: MaterialAlias::StairOpen29,
        };
        for source in [
            room.clone()
                .with_material_policy(alloc::vec![rule])
                .unwrap(),
            room.with_sample_halo([0, 0, 10, 10]).unwrap(),
        ] {
            let a = Admission {
                room: &source,
                objects: &[],
                cellar_up_lanes: true,
                door_hit_enabled: false,
            };
            assert_eq!(a.collision(0).unwrap(), source);
        }
    }
    #[test]
    fn lane_admission_classifies_without_rewriting_raw_words() {
        for raw in [0x0b81, 0x3acb, 0xbacb] {
            let mut cells = alloc::vec![0; 32 * 64];
            cells[21 * 32 + 11] = raw;
            let room = Room::new_passive(32, 64, cells).unwrap();
            let a = Admission {
                room: &room,
                objects: &[],
                cellar_up_lanes: true,
                door_hit_enabled: false,
            };
            assert_eq!(a.collision(0).unwrap().cells(), room.cells());
        }
    }
}
