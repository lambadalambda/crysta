//! Versioned replay fixtures: per-frame controller input plus metadata,
//! reproducible across sessions.
//!
//! Fixtures are commit-safe: they contain input edges, ROM identity by
//! digest, and harness version — never ROM content or memory dumps.

use crate::Button;
use serde::{Deserialize, Serialize};

/// Harness version of the fixture format. Bump on breaking changes.
pub const FIXTURE_VERSION: u32 = 1;

/// A single frame's controller state, as button edges to apply.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameInput {
    /// Buttons newly pressed this frame (edge-triggered).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub press: Vec<ButtonDef>,
    /// Buttons released this frame.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub release: Vec<ButtonDef>,
}

/// Serializable button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonDef {
    /// SNES B.
    B,
    /// SNES Y.
    Y,
    /// Select.
    Select,
    /// Start.
    Start,
    /// D-pad up.
    Up,
    /// D-pad down.
    Down,
    /// D-pad left.
    Left,
    /// D-pad right.
    Right,
    /// SNES A.
    A,
    /// SNES X.
    X,
    /// Left shoulder.
    L,
    /// Right shoulder.
    R,
}

impl From<ButtonDef> for Button {
    fn from(def: ButtonDef) -> Self {
        match def {
            ButtonDef::B => Self::B,
            ButtonDef::Y => Self::Y,
            ButtonDef::Select => Self::Select,
            ButtonDef::Start => Self::Start,
            ButtonDef::Up => Self::Up,
            ButtonDef::Down => Self::Down,
            ButtonDef::Left => Self::Left,
            ButtonDef::Right => Self::Right,
            ButtonDef::A => Self::A,
            ButtonDef::X => Self::X,
            ButtonDef::L => Self::L,
            ButtonDef::R => Self::R,
        }
    }
}

/// Starting state for a replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum StartPoint {
    /// Boot from hard reset and discard `skip_frames` frames with no input.
    Reset {
        /// Frames to advance before the first input frame.
        skip_frames: u32,
    },
    /// Resume from a snapshot produced by this harness (opaque bytes).
    Snapshot {
        /// Harness snapshot payload (`LakeSnes` state blob), local-only when
        /// derived from a real ROM run.
        payload: Vec<u8>,
    },
}

/// A replay fixture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fixture {
    /// Fixture format version; must equal [`FIXTURE_VERSION`].
    pub version: u32,
    /// SHA-256 of the normalized ROM this fixture was recorded against.
    pub rom_sha256: [u8; 32],
    /// Harness identity that produced the fixture.
    pub harness: String,
    /// Where the replay starts.
    pub start: StartPoint,
    /// Per-frame inputs; the replay length is this vector's length.
    pub frames: Vec<FrameInput>,
}

/// Errors from loading or validating a fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureError {
    /// The fixture's version field is not the supported one.
    UnsupportedVersion {
        /// Version found in the fixture.
        found: u32,
    },
    /// The fixture's ROM digest does not match the loaded ROM.
    RomMismatch,
}

impl std::fmt::Display for FixtureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedVersion { found } => write!(
                f,
                "fixture version {found} is not supported (expected {FIXTURE_VERSION})"
            ),
            Self::RomMismatch => write!(f, "fixture was recorded against a different ROM"),
        }
    }
}

impl std::error::Error for FixtureError {}

impl Fixture {
    /// Validates version and ROM identity for replay against `rom_sha256`.
    ///
    /// # Errors
    ///
    /// Returns [`FixtureError`] on version or ROM mismatch.
    pub fn validate(&self, rom_sha256: [u8; 32]) -> Result<(), FixtureError> {
        if self.version != FIXTURE_VERSION {
            return Err(FixtureError::UnsupportedVersion {
                found: self.version,
            });
        }
        if self.rom_sha256 != rom_sha256 {
            return Err(FixtureError::RomMismatch);
        }
        Ok(())
    }
}

/// Executes a fixture against a session, returning the final frame state.
///
/// The session must have been created from the ROM named by the fixture's
/// digest; the caller validates that. Reset-based scenarios must receive a
/// freshly constructed (already hard-reset) session.
pub fn run_fixture(session: &mut crate::Session, fixture: &Fixture) -> crate::FrameState {
    match &fixture.start {
        StartPoint::Reset { skip_frames } => {
            session.run_frames(*skip_frames as usize);
        }
        StartPoint::Snapshot { payload } => {
            crate::load_state(session, payload);
        }
    }
    for frame in &fixture.frames {
        for b in &frame.press {
            session.set_button((*b).into(), true);
        }
        for b in &frame.release {
            session.set_button((*b).into(), false);
        }
        session.run_frame();
    }
    session.frame_state()
}

/// Captures a core snapshot for later [`StartPoint::Snapshot`] resumption.
///
/// # Panics
///
/// Panics if the core state exceeds `STATE_MAX` bytes (guarded allocation).
#[must_use]
pub fn save_snapshot(session: &crate::Session) -> Vec<u8> {
    crate::save_state(session)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rom_sha() -> [u8; 32] {
        [0xAA; 32]
    }

    fn fixture(version: u32) -> Fixture {
        Fixture {
            version,
            rom_sha256: rom_sha(),
            harness: "test".into(),
            start: StartPoint::Reset { skip_frames: 10 },
            frames: vec![
                FrameInput {
                    press: vec![ButtonDef::Start],
                    release: vec![],
                },
                FrameInput::default(),
            ],
        }
    }

    #[test]
    fn accepts_matching_fixture() {
        assert_eq!(fixture(FIXTURE_VERSION).validate(rom_sha()), Ok(()));
    }

    #[test]
    fn rejects_version_mismatch_with_found_version() {
        let err = fixture(FIXTURE_VERSION + 1)
            .validate(rom_sha())
            .unwrap_err();
        assert_eq!(
            err,
            FixtureError::UnsupportedVersion {
                found: FIXTURE_VERSION + 1
            }
        );
        assert!(err.to_string().contains("not supported"));
    }

    #[test]
    fn rejects_rom_mismatch() {
        let err = fixture(FIXTURE_VERSION).validate([0xBB; 32]).unwrap_err();
        assert_eq!(err, FixtureError::RomMismatch);
    }

    #[test]
    fn round_trips_through_json() {
        let f = fixture(FIXTURE_VERSION);
        let json = serde_json::to_string(&f).expect("serialize");
        let back: Fixture = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(f, back);
        // No ROM bytes: fixture stays tiny.
        assert!(json.len() < 512, "{} bytes", json.len());
    }
}
