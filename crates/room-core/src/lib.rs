#![no_std]
//! Deterministic, reference-qualified cardinal walking over immutable collision cells.
//!
//! Only ordinary walking with fixed (-8,-16), 16×16 bounds is modeled. Unknown
//! materials, flagged cells (unless explicitly passive) and accelerated input triggers fail closed. Qualified
//! open/solid corners retain the native perpendicular nudge.
//! The caller owns mode admission, room identity, actors, exits and transitions;
//! in particular it must hand off after the qualified doorway movement step.
//! There are no devices, clocks, filesystem access, original CPU or dependencies.

extern crate alloc;

mod animation;
pub mod arrival;
pub mod conversation;
pub mod events;
mod house;
mod pandora;
pub mod pots;
mod room;
pub mod run;
pub mod slice;
mod snapshot;
mod transition;

pub use animation::{AnimationFrame, AnimationSet, AnimationState};
pub use room::{MaterialAlias, MaterialPolicyError, MaterialRule, Room};
pub use snapshot::{SNAPSHOT_SIZE, SNAPSHOT_VERSION};

/// Cardinal directions with stable discriminants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Direction {
    /// Positive Y, mask bit 0.
    Down = 0,
    /// Negative Y, mask bit 1.
    Up = 1,
    /// Negative X, mask bit 2.
    Left = 2,
    /// Positive X, mask bit 3.
    Right = 3,
}

impl Direction {
    /// Bit corresponding to this cardinal for caller-defined direction sets.
    #[must_use]
    pub const fn mask(self) -> u8 {
        1 << self as u8
    }

    /// The direction the other way.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    /// Whether it is Left or Right.
    #[must_use]
    pub const fn horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    const fn negative(self) -> bool {
        matches!(self, Self::Left | Self::Up)
    }
}

/// One submitted frame of input; neutral is the default. Diagonals/actions are not representable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameInput {
    /// Held cardinal direction, or neutral.
    pub direction: Option<Direction>,
}

/// One frame of the pad's directions, which can hold a diagonal; a
/// [`FrameInput`] is one without.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadInput {
    /// Held cardinal direction, or neutral.
    pub direction: Option<Direction>,
    /// The perpendicular direction held with it: a diagonal. Which of the
    /// two leads is the walk's to decide ([`WalkingState::step`]).
    pub cross: Option<Direction>,
}

impl From<FrameInput> for PadInput {
    fn from(input: FrameInput) -> Self {
        Self {
            direction: input.direction,
            cross: None,
        }
    }
}

impl PadInput {
    /// The two directions of a diagonal, the walk going on in `walking` if
    /// it is one of them, else the vertical one leading; one direction
    /// alone, or neutral, as given. Returns the lead and the cross.
    #[must_use]
    pub fn lead(self, walking: Option<Direction>) -> (Option<Direction>, Option<Direction>) {
        let (Some(first), Some(second)) = (self.direction, self.cross) else {
            return (self.direction.or(self.cross), None);
        };
        if first.horizontal() == second.horizontal() {
            return (Some(first), None);
        }
        let lead = if walking == Some(second) || (walking != Some(first) && first.horizontal()) {
            second
        } else {
            first
        };
        let cross = if lead == first { second } else { first };
        (Some(lead), Some(cross))
    }
}

/// An unsupported operation. A failed walking step never modifies state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unqualified {
    /// Dimensions must be nonzero and their pixel extents fit u16 (at most 4095 cells).
    RoomDimensions,
    /// Row-major cell count differs from width times height.
    CellCount,
    /// The complete fixed player bounds do not fit inside the supplied room.
    PositionOutOfBounds,
    /// A collision sample is outside the grid; map-edge wrapping is unqualified.
    SampleOutOfBounds,
    /// A collision sample is outside the room's admitted half-open cell halo.
    SampleOutsideAdmission,
    /// Coordinate arithmetic would overflow or underflow u16.
    ArithmeticOverflow,
    /// The raw cell has the special dispatch override bit set.
    FlaggedCell(u16),
    /// A stored material type outside 0, 2, 22, 12 and 14.
    UnsupportedType(u8),
    /// Same-direction reactivation inside the measured onset window would accelerate.
    AcceleratedTrigger(Direction),
    /// Snapshot length, version, encoding or state invariants are invalid.
    Snapshot,
}

impl core::fmt::Display for Unqualified {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "unqualified room walking: {self:?}")
    }
}
impl core::error::Error for Unqualified {}

/// Resolved frame result. Attempts advance even when a wall blocks movement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MovementOutput {
    /// Resulting player X coordinate, in pixels.
    pub x: u16,
    /// Resulting player Y coordinate, in pixels.
    pub y: u16,
    /// Actual signed X displacement after collision.
    pub dx: i16,
    /// Actual signed Y displacement after collision.
    pub dy: i16,
    /// Signed stream output before collision.
    pub attempted_dx: i16,
    /// Signed stream output before collision.
    pub attempted_dy: i16,
    /// A solid new-edge sample invoked blocking correction.
    pub blocked: bool,
}

/// Small walking component, not a top-level game state.
///
/// Ordinary directions may reactivate after the 11-tick onset window or an
/// intervening direction. Accelerated triggers fail closed; dash is not modeled.
/// Room and mode identity belong to the caller, including after snapshot restore.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalkingState {
    x: u16,
    y: u16,
    active: Option<Direction>,
    delayed: Option<Direction>,
    phase: u8,
    last_activation: Option<Direction>,
    onset_remaining: u8,
    /// The perpendicular direction walked with `active`: a diagonal.
    cross: Option<Direction>,
    /// The cross direction submitted, governing the next step.
    delayed_cross: Option<Direction>,
    /// The parity of the frames into the lead's stream (1 odd), which a
    /// diagonal's end restarts while the walk's list (`phase`) runs on.
    stream: u8,
}

impl WalkingState {
    /// Constructs an ordinary idle checkpoint with empty input history.
    ///
    /// This does not validate or establish gameplay mode. Position is validated
    /// against the room on every step; callers may not reset history to bypass
    /// unsupported actions in an ongoing reference replay.
    #[must_use]
    pub const fn new(x: u16, y: u16) -> Self {
        Self {
            x,
            y,
            active: None,
            delayed: None,
            phase: 0,
            last_activation: None,
            onset_remaining: 0,
            cross: None,
            delayed_cross: None,
            stream: 0,
        }
    }

    /// Walking in `direction` already, as after a dash stopped at a wall,
    /// with `onset` ticks left of the onset window.
    pub(crate) const fn walking(x: u16, y: u16, direction: Direction, onset: u8) -> Self {
        Self {
            x,
            y,
            active: Some(direction),
            delayed: Some(direction),
            phase: 0,
            last_activation: Some(direction),
            onset_remaining: onset,
            cross: None,
            delayed_cross: None,
            stream: 0,
        }
    }

    /// Player X coordinate.
    #[must_use]
    pub const fn x(&self) -> u16 {
        self.x
    }
    /// Player Y coordinate.
    #[must_use]
    pub const fn y(&self) -> u16 {
        self.y
    }
    /// Player position in pixels.
    #[must_use]
    pub const fn position(&self) -> (u16, u16) {
        (self.x, self.y)
    }
    /// Direction governing this state's most recently resolved movement.
    #[must_use]
    pub const fn active_direction(&self) -> Option<Direction> {
        self.active
    }
    /// Most recently submitted input, which will govern the next step.
    #[must_use]
    pub const fn delayed_direction(&self) -> Option<Direction> {
        self.delayed
    }
    /// Whether `input` is a double tap, which [`Self::step`] refuses as
    /// the trigger of a dash.
    #[must_use]
    pub fn double_taps(&self, input: PadInput) -> bool {
        let (lead, _) = self.lead(input);
        lead != self.delayed && self.taps_again(lead)
    }

    /// Whether `lead` presses the direction last pressed again inside the
    /// onset window.
    fn taps_again(&self, lead: Option<Direction>) -> bool {
        lead.is_some() && self.last_activation == lead && self.onset_remaining > 1
    }

    /// The lead and the cross of `input` ([`PadInput::lead`]) for this
    /// walk: the walk going on, else, in the window a double tap dashes in,
    /// the direction last pressed (a tap then a diagonal dashes the tap's
    /// way, the native trace `q_right_then_ur`).
    #[must_use]
    pub fn lead(&self, input: PadInput) -> (Option<Direction>, Option<Direction>) {
        let tapped = self.last_activation.filter(|_| self.onset_remaining > 1);
        input.lead(self.delayed.or(tapped))
    }

    /// The walk list's counter: horizontal 0..53 (0=setup/gap), vertical
    /// 0=setup/1=odd/2=even, idle=0. After a diagonal the stream's own
    /// parity sets the next strides.
    #[must_use]
    pub const fn phase(&self) -> u8 {
        self.phase
    }
    /// Most recently activated direction; neutral and timer expiry retain it.
    #[must_use]
    pub const fn last_activation_direction(&self) -> Option<Direction> {
        self.last_activation
    }
    /// Remaining onset-window ticks (0..11), decremented before each input test.
    #[must_use]
    pub const fn onset_remaining(&self) -> u8 {
        self.onset_remaining
    }

    /// Resolves one input/frame atomically, with measured latency and qualified collision.
    ///
    /// New input takes effect after one old-input step and one zero/setup step.
    /// Horizontal walking repeats a 54-phase cycle; vertical walking alternates
    /// 1,2 without that restart. A diagonal (`PadInput::cross`) moves 1 on
    /// each axis, from the next frame when added to a walk; its end restarts
    /// the lead's 1,2 stream. The caller selects exits after a successful step.
    ///
    /// # Errors
    /// Returns [`Unqualified`] for invalid bounds, cells, arithmetic or
    /// accelerated input triggers. Every error leaves all state fields unchanged.
    pub fn step(
        &mut self,
        room: &Room,
        input: impl Into<PadInput>,
    ) -> Result<MovementOutput, Unqualified> {
        self.step_with_cadence(room, input, false)
    }

    // COP83 carrying does not reload the COP84 horizontal null stream. Collision,
    // latency and fail-closed onset handling remain shared; pots owns mode admission.
    pub(crate) fn step_with_cadence(
        &mut self,
        room: &Room,
        input: impl Into<PadInput>,
        continuous_horizontal: bool,
    ) -> Result<MovementOutput, Unqualified> {
        let input = input.into();
        room.validate_position(self.x, self.y)?;
        let mut next = *self;
        let (lead, cross) = self.lead(input);
        next.onset_remaining = self.onset_remaining.saturating_sub(1);
        if let Some(direction) = lead {
            if lead != self.delayed {
                if self.taps_again(lead) {
                    return Err(Unqualified::AcceleratedTrigger(direction));
                }
                next.last_activation = Some(direction);
                next.onset_remaining = 11;
            }
        }
        if self.delayed == self.active {
            next.phase = match self.active {
                None => 0,
                Some(d) if d.horizontal() && !continuous_horizontal => (self.phase + 1) % 54,
                Some(_) => {
                    if self.phase == 1 {
                        2
                    } else {
                        1
                    }
                }
            };
            // The lead's stream runs on; it starts over where the list does
            // and where a diagonal ends (native traces `k_*`).
            next.stream =
                if next.phase == 0 || (self.cross.is_some() && self.delayed_cross.is_none()) {
                    u8::from(next.phase != 0)
                } else {
                    (self.stream + 1) & 1
                };
        } else {
            next.active = self.delayed;
            next.phase = 0;
            next.stream = 0;
        }
        next.cross = self.delayed_cross.filter(|_| next.active.is_some());
        next.delayed = lead;
        next.delayed_cross = cross;
        // A diagonal moves one on each axis; the lead alone 1, 2 by its stream.
        let magnitude = if next.phase == 0 {
            0
        } else if next.cross.is_some() || next.stream % 2 == 1 {
            1
        } else {
            2
        };
        let (mut dx, mut dy) = (0_i16, 0_i16);
        for direction in next.active.into_iter().chain(next.cross) {
            let delta = if direction.negative() {
                -magnitude
            } else {
                magnitude
            };
            if direction.horizontal() {
                dx = delta;
            } else {
                dy = delta;
            }
        }
        let (x, y, blocked) = match (next.active, next.cross) {
            // A diagonal: each axis against the walls in turn, x first.
            (Some(lead), Some(cross)) => {
                let (horizontal, vertical) = if lead.horizontal() {
                    (lead, cross)
                } else {
                    (cross, lead)
                };
                let (x, y, across) = room.resolve(self.x, self.y, Some(horizontal), dx, 0)?;
                let (x, y, along) = room.resolve(x, y, Some(vertical), 0, dy)?;
                (x, y, across || along)
            }
            (active, _) => room.resolve(self.x, self.y, active, dx, dy)?,
        };
        next.x = x;
        next.y = y;
        let output = MovementOutput {
            x,
            y,
            dx: i16::try_from(i32::from(x) - i32::from(self.x))
                .map_err(|_| Unqualified::ArithmeticOverflow)?,
            dy: i16::try_from(i32::from(y) - i32::from(self.y))
                .map_err(|_| Unqualified::ArithmeticOverflow)?,
            attempted_dx: dx,
            attempted_dy: dy,
            blocked,
        };
        *self = next;
        Ok(output)
    }
}
