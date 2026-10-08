//! Ark's statuses (`docs/darts-and-burn.md` §2): a hit's status roll sets
//! one from the table `$85:DB5F` into `$066C` and its counter at
//! `$0670 + 2e`; the runner (`$84:D04A`) counts them out; the burn's script
//! (`$84:DBF1`) holds Ark. A map's load clears them (`$85:DFA8`).
//!
//! Only the burn has its runner here: a roll for another element only ends
//! the burn and the sleep (`meta/issues/darts-and-burn.md`).

use super::{Step, World, WorldError};

/// The statuses' word and their counters.
const STATUSES: u16 = 0x066C;
const COUNTERS: u16 = 0x0670;
/// The table, Japanese and European (bank `$85` is `$98` on).
const TABLE: [usize; 2] = [0x05_DB5F, 0x05_DBF7];
/// The burn: its bit and its counter (element 11).
const BURN: u16 = 0x0020;
const BURN_COUNTER: u16 = COUNTERS + 2 * 11;
/// The bits and counters a map's load clears (`$85:DFA8`), and those it
/// clears off a tower floor too.
const LOAD_CLEARS: u16 = 0x17F8;
const LOAD_COUNTERS: std::ops::RangeInclusive<u16> = 0x067C..=0x068A;
const FLOOR_CLEARS: u16 = 0xE800;
const FLOOR_COUNTERS: std::ops::RangeInclusive<u16> = 0x0672..=0x067A;
/// The burn's script: Ark's 60 invulnerable frames (`7F:1020 = $3C`), his
/// two poses (57 and 1 frames), then up to 62 frames while it lasts.
const BURN_IMMUNE: u16 = 60;
const BURN_POSES: u16 = 58;
const BURN_HOLD: u16 = BURN_POSES + 62;
/// "...TOASTED" (`COP 1B`, Japanese and European).
const TOASTED: [u32; 2] = [0x84_DC40, 0x84_DC0D];

/// A row of the status table: the bits that block it, the mask it keeps,
/// its bit and its frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Row {
    block: u16,
    keep: u16,
    bit: u16,
    frames: u16,
}

pub(super) fn row(image: &[u8], element: u8) -> Option<Row> {
    let at = assets::layout::per_revision(image, TABLE[0], TABLE[1]) + usize::from(element) * 8;
    let bytes = image.get(at..at + 8)?;
    let word = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    Some(Row {
        block: word(0),
        keep: word(2),
        bit: word(4),
        frames: word(6),
    })
}

/// A successful roll (`$85:DAE3`): the burn and the sleep end, then the
/// element's status starts unless one of its blocking bits is set.
/// Returns the statuses and the counter to set.
fn rolled(statuses: u16, row: Row) -> (u16, Option<u16>) {
    let statuses = statuses & 0xFF9F;
    if statuses & row.block != 0 {
        (statuses, None)
    } else {
        ((statuses & row.keep) | row.bit, Some(row.frames))
    }
}

/// The burn's script under way: frames into it, or waiting for a hit's
/// push to end (`COP DF` waits out `$097C & $0810`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Burn {
    Waiting,
    Frame(u16),
}

impl Burn {
    /// Resource 5's list 4 (57 frames), then 5 held (`$84:DBF1`).
    pub(super) const fn pose(self) -> Option<super::pose::ArkPose> {
        use super::pose::ArkPose;
        match self {
            Self::Waiting => None,
            Self::Frame(frame) if frame < BURN_POSES - 1 => {
                Some(ArkPose::new(5, 4, false, frame, true))
            }
            Self::Frame(frame) => Some(ArkPose::new(5, 5, false, frame - (BURN_POSES - 1), true)),
        }
    }
}

impl World<'_> {
    /// A hit's successful status roll for `element`. Only the burn is set:
    /// no other has its runner.
    pub(super) fn take_status(&mut self, element: u8) {
        let slot = &mut self.globals.slot;
        let Some(row) = row(self.image, element) else {
            return;
        };
        slot.set_word_at(COUNTERS + 2 * 10, 0);
        slot.set_word_at(BURN_COUNTER, 0);
        let (statuses, frames) = if row.bit == BURN {
            rolled(slot.word_at(STATUSES), row)
        } else {
            (slot.word_at(STATUSES) & 0xFF9F, None)
        };
        slot.set_word_at(STATUSES, statuses);
        if let Some(frames) = frames {
            slot.set_word_at(COUNTERS + 2 * u16::from(element), frames);
        }
    }

    /// Ark's statuses as an enemy's hit reads them.
    pub(super) fn statuses(&self) -> u16 {
        self.globals.slot.word_at(STATUSES)
    }

    /// The runner's frame for the burn (`$84:D329`..`D366`): a new one
    /// starts the script; it counts up to 0 and ends; a fall or a drop
    /// ends it at once (`$097C & $C4E5`).
    pub(super) fn status_frame(&mut self) {
        let slot = &mut self.globals.slot;
        let statuses = slot.word_at(STATUSES);
        if statuses & BURN == 0 || self.down.is_some() {
            return;
        }
        if self.fall.is_some() || self.jump.is_some() {
            slot.set_word_at(STATUSES, statuses & !BURN);
            return;
        }
        let mut counter = slot.word_at(BURN_COUNTER).cast_signed();
        if counter == 0 {
            slot.set_word_at(STATUSES, statuses & !BURN);
            return;
        }
        // A new one turns its count negative and starts the script, then
        // counts on the same frame (`$84:D340`, `D355`).
        if counter > 0 {
            counter = counter.wrapping_neg();
            self.burn = Some(Burn::Waiting);
        }
        counter += 1;
        slot.set_word_at(BURN_COUNTER, counter.cast_unsigned());
        if counter == 0 {
            slot.set_word_at(STATUSES, statuses & !BURN);
        }
    }

    /// A frame of the burn's script (`$84:DBF1`): Ark held, his attacks
    /// off, 60 frames out of reach, "...TOASTED", his two poses, then
    /// held while the burn lasts, up to 62 frames.
    pub(super) fn burn_frame(&mut self) -> Result<Option<Step>, WorldError> {
        let Some(burn) = self.burn else {
            return Ok(None);
        };
        let frame = match burn {
            Burn::Waiting if self.hurt.is_some() => return Ok(None),
            Burn::Waiting => {
                self.ark_immune = BURN_IMMUNE;
                self.thrust = None;
                if !self.globals.dialogue.busy() {
                    let at = TOASTED[assets::layout::per_revision(self.image, 0, 1)];
                    let pages =
                        assets::text::HouseDialogue::decode_at(self.image, at).unwrap_or_default();
                    self.globals.dialogue.request(pages);
                }
                0
            }
            Burn::Frame(frame) => frame + 1,
        };
        // Enemies still hit him once his 60 frames are out.
        self.hurt_ark();
        let burning = self.statuses() & BURN != 0;
        self.burn =
            (frame < BURN_POSES || (burning && frame < BURN_HOLD)).then_some(Burn::Frame(frame));
        self.run_actors()?;
        Ok(Some(Step::Stayed))
    }

    /// A map's load (`$85:DFA8`): most statuses end, more off a tower
    /// floor; no life is 1.
    pub(super) fn clear_statuses(&mut self) {
        let floor = self.globals.tower_floor();
        let slot = &mut self.globals.slot;
        let bits = if floor {
            LOAD_CLEARS
        } else {
            LOAD_CLEARS | FLOOR_CLEARS
        };
        slot.set_word_at(STATUSES, slot.word_at(STATUSES) & !bits);
        let off_floor = (!floor).then_some(FLOOR_COUNTERS).into_iter().flatten();
        for address in LOAD_COUNTERS.step_by(2).chain(off_floor.step_by(2)) {
            slot.set_word_at(address, 0);
        }
        let stats = slot.stats();
        if stats.life == 0 {
            slot.set_life(1);
        }
        self.burn = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BURN_ROW: Row = Row {
        block: 0x0020,
        keep: 0xEE3F,
        bit: 0x0020,
        frames: 120,
    };

    #[test]
    fn a_roll_ends_the_burn_and_the_sleep_then_sets_its_status() {
        assert_eq!(rolled(0x0060, BURN_ROW), (0x0020, Some(120)));
        // A blocking bit stops it, after the burn and the sleep end.
        let blocked = Row {
            block: 0x0100,
            ..BURN_ROW
        };
        assert_eq!(rolled(0x0160, blocked), (0x0100, None));
        // The kept mask clears what the status replaces.
        assert_eq!(rolled(0x1100, BURN_ROW), (0x0020, Some(120)));
    }

    #[test]
    fn the_hits_status_bits_are_the_tables() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../local/Tenchi Souzou (Japan).sfc"
        );
        let Ok(image) = std::fs::read(path) else {
            return;
        };
        for element in 0..16 {
            assert_eq!(
                row(&image, element).map(|row| row.bit),
                Some(crate::combat::STATUS_BITS[usize::from(element)])
            );
        }
    }

    #[test]
    fn a_burn_holds_ark_while_it_lasts_and_says_so() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../local/Tenchi Souzou (Japan).sfc"
        );
        let Ok(image) = std::fs::read(path) else {
            return;
        };
        let mut world = World::enter(&image, 0x0101, 120, 300).unwrap();
        // The table's row 11 is the burn.
        assert_eq!(row(&image, 11), Some(BURN_ROW));
        world.take_status(11);
        let start = world.position();
        let mut held = 0;
        loop {
            world
                .update(
                    Some(room_core::Direction::Down),
                    crate::scene::Presses::default(),
                )
                .unwrap();
            if world.burn.is_none() {
                break;
            }
            assert_eq!(world.position(), start);
            held += 1;
            if held == 1 {
                assert!(world.globals.dialogue.busy(), "...TOASTED");
            }
        }
        // The status's 120 frames, its first counted on its start: the
        // script's poses, then the wait.
        assert_eq!(held, 119);
        assert_eq!(world.statuses() & BURN, 0);
        // Another element's roll only ends the burn (no runner for it).
        world.take_status(11);
        world.take_status(13);
        assert_eq!(world.statuses(), 0);
        world.take_status(10);
        assert_eq!(world.statuses(), 0, "no sleep without its runner");
    }
}
