//! The landing on `$114` (`docs/tower-three.md` §2): a fall through
//! `$10F`'s crumbling row arrives on the `FD` record at (15,9), whose
//! script (`$90:FA53`, European `$90:F7D5`) runs only when Ark stands
//! exactly on it. It hides him and locks the pad, then shows him 256 pixels
//! up (`$0970 = $FF00`) in resource 0's `$13`, over the high tiles
//! (`+$08 |= $3000`), and lowers him 4 pixels a frame for 64 frames; there
//! (`$90:FAE3`, `COP C6`), unless the cell under him is `$13`, sound `$0F`
//! and resource 0's `$14`, then the pad again.
//!
//! In Rust: the script writes Ark's action word (`TSB $097C`), his draw
//! offset `$0970` and his `+$08`, which the native runs do not.

use super::pose::ArkPose;
use super::{Step, World, WorldError};

/// The landing's script, Japanese and European.
const SCRIPT: [u32; 2] = [0x90_FA53, 0x90_F7D5];
/// Frames hidden before the drop: the check, `COP C1 2` (resumed on its
/// fourth frame), the yield.
const HIDDEN: u16 = 5;
/// The drop: 64 frames of 4 pixels, from 256 up.
const DROP: u16 = 64;
const FALL: u16 = 4;
/// The landing's sound (port 3) and the cell that skips it.
const SOUND: u8 = 0x0F;
const EDGE: u16 = 0x13;
/// Frames of resource 0's `$14`: 6, eight of 1, 16 (`COP 8E` waits them).
const LANDED: u16 = 30;

/// The landing under way: frames into it, and the frame it ends on, set
/// when Ark lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Landing {
    frame: u16,
    end: Option<u16>,
}

impl Landing {
    /// Ark hidden, then dropping in `$13`, then landed in `$14` (none on
    /// the `$13` cell, which ends the landing at once).
    pub(super) const fn pose(self) -> Option<ArkPose> {
        if self.frame < HIDDEN {
            None
        } else if self.frame < HIDDEN + DROP {
            Some(ArkPose::new(0, 0x13, false, self.frame - HIDDEN, false))
        } else {
            Some(ArkPose::new(
                0,
                0x14,
                false,
                self.frame - HIDDEN - DROP,
                true,
            ))
        }
    }

    /// Whether Ark is hidden.
    pub(super) const fn hidden(self) -> bool {
        self.frame < HIDDEN
    }

    /// How far above his place Ark is drawn (`$0970`), in pixels.
    pub(super) const fn lift(self) -> i16 {
        if self.frame < HIDDEN || self.frame >= HIDDEN + DROP {
            0
        } else {
            -(FALL * (HIDDEN + DROP - 1 - self.frame)).cast_signed()
        }
    }
}

impl World<'_> {
    /// At a load: the landing's record starts it when Ark stands on it;
    /// else its script deletes itself. Either way the actor goes.
    pub(super) fn start_landing(&mut self) {
        self.landing = None;
        let script = SCRIPT[assets::layout::per_revision(self.image, 0, 1)];
        let at = self.position();
        let Some(index) = self
            .residents
            .iter()
            .position(|resident| resident.script == Some(script))
        else {
            return;
        };
        if self.residents[index].position == at {
            self.landing = Some(Landing {
                frame: 0,
                end: None,
            });
        }
        self.actors[index].remove();
    }

    /// A frame of the landing: the world runs on, Ark held.
    pub(super) fn landing_frame(&mut self) -> Result<Option<Step>, WorldError> {
        let Some(mut landing) = self.landing.take() else {
            return Ok(None);
        };
        landing.frame += 1;
        if landing.frame == HIDDEN + DROP {
            // `COP C6`: the cell under him (`$80:C4A9`).
            let (x, y) = self.position();
            let under = self.attribute((x.wrapping_sub(8) / 16, y.wrapping_sub(16) / 16));
            let posed = under != Some(EDGE);
            if posed {
                self.globals.audio.sound_port3(SOUND);
            }
            landing.end = Some(landing.frame + if posed { LANDED } else { 0 });
        }
        if landing.end.is_none_or(|end| landing.frame < end) {
            self.landing = Some(landing);
        }
        self.run_actors()?;
        Ok(Some(Step::Stayed))
    }

    /// How far above his place Ark is drawn: a landing's drop.
    #[must_use]
    pub fn ark_lift(&self) -> i16 {
        self.landing.map_or(0, Landing::lift) + self.jump_height()
    }

    /// Whether Ark is drawn over the high tiles (`+$08 |= $3000`): a
    /// landing's drop.
    #[must_use]
    pub fn ark_over(&self) -> bool {
        self.landing
            .is_some_and(|landing| !landing.hidden() && landing.lift() != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ark_drops_in_from_256_pixels_up_four_a_frame() {
        let at = |frame| Landing { frame, end: None };
        assert!(at(0).hidden() && at(0).pose().is_none());
        assert_eq!(at(HIDDEN).lift(), -252);
        assert_eq!(at(HIDDEN + DROP - 1).lift(), 0);
        assert_eq!(at(HIDDEN + 5).pose().map(|pose| pose.list), Some(0x13));
        assert_eq!(at(HIDDEN + DROP).pose().map(|pose| pose.list), Some(0x14));
    }
}
