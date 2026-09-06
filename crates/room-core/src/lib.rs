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
pub mod events;
mod house;
mod room;
pub mod slice;
mod snapshot;
mod transition;

pub use animation::{AnimationFrame, AnimationSet, AnimationState};
pub use room::Room;
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

    const fn horizontal(self) -> bool {
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
    /// Horizontal 0..53 (0=setup/gap), vertical 0=setup/1=odd/2=even, idle=0.
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
    /// 1,2 without that restart. The caller selects exits after a successful step.
    ///
    /// # Errors
    /// Returns [`Unqualified`] for invalid bounds, cells, arithmetic or
    /// accelerated input triggers. Every error leaves all state fields unchanged.
    pub fn step(&mut self, room: &Room, input: FrameInput) -> Result<MovementOutput, Unqualified> {
        room.validate_position(self.x, self.y)?;
        let mut next = *self;
        next.onset_remaining = self.onset_remaining.saturating_sub(1);
        if let Some(direction) = input.direction {
            if input.direction != self.delayed {
                if self.last_activation == Some(direction) && next.onset_remaining != 0 {
                    return Err(Unqualified::AcceleratedTrigger(direction));
                }
                next.last_activation = Some(direction);
                next.onset_remaining = 11;
            }
        }
        if self.delayed == self.active {
            next.phase = match self.active {
                None => 0,
                Some(d) if d.horizontal() => (self.phase + 1) % 54,
                Some(_) => {
                    if self.phase == 1 {
                        2
                    } else {
                        1
                    }
                }
            };
        } else {
            next.active = self.delayed;
            next.phase = 0;
        }
        next.delayed = input.direction;
        let magnitude = if next.phase == 0 {
            0
        } else if next.phase % 2 == 1 {
            1
        } else {
            2
        };
        let (mut dx, mut dy) = (0_i16, 0_i16);
        if let Some(direction) = next.active {
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
        let (x, y, blocked) = room.resolve(self.x, self.y, next.active, dx, dy)?;
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
