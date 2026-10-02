//! The Magirock pickups (`docs/chests.md` §5): an actor (`$84:DD7E`) whose
//! interaction (`$84:DDC8`, European `$84:DD82`) Ark lifts the stone with:
//! 14 frames of the lift, the stone over his head, 4 more, the text "Ark
//! obtained Magirock!", then one more Magirock (`$07ED`, BCD), its flag
//! `$900 + n` and the stone gone. The script's own code is native; this
//! models its timing.
//!
//! Not modelled: Ark's lift poses by facing (`$84:BF67..BF83`, poses
//! `$18..$1A`; the pot lift stands in;
//! `meta/issues/ark-underworld-poses.md`).

use super::{Step, World, WorldError};
use crate::scene::Presses;
use assets::layout::per_revision;

/// The interaction callback, Japanese and European.
const TALK: [usize; 2] = [0x04_DDC8, 0x04_DD82];
/// Its text, Japanese and European.
const TEXT: [u32; 2] = [0x84_DEC9, 0x84_DE83];
/// Frames of the lift before the stone goes over Ark's head (`COP C1 $0E`),
/// and of the hold before the text (`COP C1 4`).
const LIFT: u16 = 15;
const HOLD: u16 = 20;
/// The stone sits 18 pixels above Ark's probe (`$84:DE7B`).
const ABOVE: u16 = 8 + 18;

/// A Magirock being taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Pickup {
    /// The actor's index.
    actor: usize,
    frame: u16,
}

impl Pickup {
    /// Frames since the press, for Ark's lift.
    pub(super) const fn frame(self) -> u16 {
        self.frame
    }
}

impl World<'_> {
    /// Starts taking the Magirock actor `index`, if it is one. Returns
    /// whether it did.
    pub(super) fn pick_up(&mut self, index: usize) -> bool {
        let talk = TALK[per_revision(self.image, 0, 1)];
        if self.actors[index].callback_at() != Some(talk) {
            return false;
        }
        self.pickup = Some(Pickup {
            actor: index,
            frame: 0,
        });
        true
    }

    /// A frame of the pickup: the world runs on, the player held.
    pub(super) fn pickup_frame(&mut self, presses: Presses) -> Result<Option<Step>, WorldError> {
        let Some(mut pickup) = self.pickup else {
            return Ok(None);
        };
        self.globals.dialogue.press(presses);
        pickup.frame += 1;
        let (x, y) = self.position();
        let actor = &mut self.actors[pickup.actor];
        match pickup.frame {
            LIFT => {
                actor.position = (x, y.wrapping_sub(ABOVE));
                actor.priority = 3;
            }
            HOLD => {
                let source = TEXT[per_revision(self.image, 0, 1)];
                let pages =
                    assets::text::HouseDialogue::decode_at(self.image, source).unwrap_or_default();
                if !self.globals.dialogue.request(pages) {
                    pickup.frame -= 1;
                }
            }
            frame if frame > HOLD && !self.globals.dialogue.busy() => {
                self.globals.inventory.add_prime_blue(1);
                let number = u16::from(actor.parameter);
                self.globals.write_flag(0x8000 | (0x900 + number));
                actor.remove();
                self.pickup = None;
                self.run_actors()?;
                return Ok(Some(Step::Stayed));
            }
            _ => {}
        }
        self.pickup = Some(pickup);
        self.run_actors()?;
        Ok(Some(Step::Stayed))
    }
}
