//! Ordinary Ark render selection, independent of movement cadence and collision.
//!
//! Call only after a successful walking step, with its **active** (already delayed)
//! direction. No input admission, scheduler, asset decoding or clock lives here.
//! Idle fidgets and doorway animation timing are deliberately semantic policies:
//! neutral holds one ordinary standing pose indefinitely. See `docs/ark-animation.md`.
use crate::Direction;

/// Source-backed ordinary animation table families (not ROM data in the core).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationSet {
    /// Native table base `$A4:A1E4`: one standing record per sequence.
    Standing,
    /// Native table base `$9A:D064`: six walking records per sequence.
    Walking,
}

/// Asset lookup key. A record is zero-based, unlike the native post-load cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationFrame {
    /// Standing or walking table family.
    pub set: AnimationSet,
    /// Down = 0, Up = 1, horizontal = 2.
    pub sequence: u8,
    /// Standing = 0; walking = 0..5, nine ticks per record.
    pub record: u8,
    /// Left shares the right sequence with native horizontal mirroring.
    pub mirror_x: bool,
}

/// Small canonical snapshot state. Not a native actor VM or idle timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationState {
    facing: Direction,
    walking: bool,
    phase: u8,
}

impl AnimationState {
    /// Selects the ordinary standing policy; fresh new-game facing is Down.
    /// This does not reproduce frame 6800's native idle/fidget pose.
    #[must_use]
    pub const fn standing(facing: Direction) -> Self {
        Self {
            facing,
            walking: false,
            phase: 0,
        }
    }

    /// Validates canonical snapshot parts. Idle must have phase zero.
    #[must_use]
    pub const fn from_parts(facing: Direction, walking: bool, phase: u8) -> Option<Self> {
        if phase >= 54 || (!walking && phase != 0) {
            return None;
        }
        Some(Self {
            facing,
            walking,
            phase,
        })
    }

    /// Retained direction, including while standing.
    #[must_use]
    pub const fn facing(self) -> Direction {
        self.facing
    }

    /// Whether the last successful walking step had an active direction.
    #[must_use]
    pub const fn is_walking(self) -> bool {
        self.walking
    }

    /// Animation phase 0..53 in every direction; not `WalkingState`'s speed phase.
    #[must_use]
    pub const fn phase(self) -> u8 {
        self.phase
    }

    /// Current ordinary asset key, with no access to ROM or rendering devices.
    #[must_use]
    pub const fn frame(self) -> AnimationFrame {
        AnimationFrame {
            set: if self.walking {
                AnimationSet::Walking
            } else {
                AnimationSet::Standing
            },
            sequence: match self.facing {
                Direction::Down => 0,
                Direction::Up => 1,
                Direction::Left | Direction::Right => 2,
            },
            record: self.phase / 9,
            mirror_x: matches!(self.facing, Direction::Left),
        }
    }

    /// Advances once per successful walking tick, even when position is blocked.
    /// A changed active direction resets to record zero on its setup tick.
    /// Neutral selects standing immediately; the caller already owns input delay.
    pub fn advance(&mut self, active: Option<Direction>) -> AnimationFrame {
        if let Some(direction) = active {
            self.phase = if self.walking && self.facing == direction {
                (self.phase + 1) % 54
            } else {
                0
            };
            self.facing = direction;
            self.walking = true;
        } else {
            self.walking = false;
            self.phase = 0;
        }
        self.frame()
    }
}
