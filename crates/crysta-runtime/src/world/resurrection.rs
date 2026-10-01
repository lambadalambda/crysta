//! A tower's end (`docs/light-room.md`): the light room's orb writes the
//! pending map `$07` (`$90:8B0C`) and the tower's index `$04CC` from the map
//! before (`$0482`). Natively `$07` plays the ocean, the Earth, the spiral
//! and the flyover, then the parchment map with "On this day, Eurasia was
//! resurrected.", and the souls map `$3C` sets the tower's flag and leaves
//! for the underworld (`$87:F7A4`).
//!
//! A first, still version: a black screen with the parchment's text and
//! its button wait, then the flag and the transfer. Not modelled: the
//! scenes of `$07` (bespoke Mode 7 and Mode 3 code, `$86:BB4A..C585`), the
//! parchment map's picture and the souls.

use super::{Step, World};
use crate::scene::{Presses, Transfer};
use assets::layout::per_revision;

/// The parchment's text per tower, Japanese and European.
const TEXTS: [[u32; 5]; 2] = [
    [0x92_C78A, 0x92_C7B8, 0x92_C7E3, 0x92_C810, 0x92_C83B],
    [0x92_DE10, 0x92_DE40, 0x92_DE6D, 0x92_DE9C, 0x92_DEC9],
];
/// Where each tower's end leaves Ark on `$03` (`$87:F7A4..F7E8`, raw), with
/// the flag it sets (`$101 + 2 * tower`).
const LANDINGS: [(u16, u16); 5] = [(208, 816), (80, 608), (480, 160), (736, 208), (880, 688)];
const FIRST_FLAG: u16 = 0x101;
/// The underworld map, and the transfer's mode (`COP 14 03 00 01 10`).
const UNDERWORLD: u16 = 0x0003;
const MODE: u8 = 1;
/// Black frames before the text, as the load of `$07` takes.
const BLACK: u16 = 60;

/// A tower's end under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Resurrection {
    /// The tower's index, 0 to 4.
    tower: usize,
    frame: u16,
}

impl Resurrection {
    /// The end of the tower whose top is `map`, if it is one (`$90:8B12`).
    pub(super) fn after(map: u16) -> Option<Self> {
        let tower = [0x105, 0x10C, 0x113, 0x11A, 0x123]
            .iter()
            .position(|&top| top == map)?;
        Some(Self { tower, frame: 0 })
    }
}

impl World<'_> {
    /// Whether a tower's end holds the screen, dark under the text.
    #[must_use]
    pub const fn resurrecting(&self) -> bool {
        self.resurrection.is_some()
    }

    /// A frame of the tower's end: the text once the screen is black, then,
    /// once it is read, the flag and the transfer to the underworld.
    pub(super) fn resurrection_frame(&mut self, presses: Presses) -> Option<Step> {
        let mut end = self.resurrection?;
        self.globals.dialogue.press(presses);
        end.frame = end.frame.saturating_add(1);
        if end.frame == BLACK {
            let source = TEXTS[per_revision(self.image, 0, 1)][end.tower];
            let pages =
                assets::text::HouseDialogue::decode_at(self.image, source).unwrap_or_default();
            if !self.globals.dialogue.request(pages) {
                end.frame -= 1;
            }
        } else if end.frame > BLACK && !self.globals.dialogue.busy() {
            let flag = FIRST_FLAG + 2 * u16::try_from(end.tower).unwrap_or(0);
            self.globals.write_flag(0x8000 | flag);
            let (x, y) = LANDINGS[end.tower];
            // The loader places Ark at the raw position plus (8,16).
            self.globals.transfer = Some(Transfer {
                map: UNDERWORLD,
                position: (x + 8, y + 16),
                mode: MODE,
            });
            self.resurrection = None;
            return Some(Step::Stayed);
        }
        self.resurrection = Some(end);
        Some(Step::Stayed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_tower_top_names_its_end() {
        assert_eq!(Resurrection::after(0x105).map(|end| end.tower), Some(0));
        assert_eq!(Resurrection::after(0x123).map(|end| end.tower), Some(4));
        assert_eq!(Resurrection::after(0x104), None);
    }
}
