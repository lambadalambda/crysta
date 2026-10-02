//! Treasure chests (`docs/chests.md`): map cells the player opens facing Up
//! with A (`$87:C7F1`, as the wooden doors), the worker `$87:9444` (European
//! `$87:939E`): the lid opens with sound `$4A`, Ark holds the item up, the
//! text names it, and once it is read the item or the gems are his and the
//! chest's flag `$500 + n` is set. A full inventory shows "overloaded" and
//! closes the lid again. On load, `$8D:912B` draws the opened chests open.
//!
//! Not modelled: Ark's both-hands lift (resource 3, poses `$39`/`$3A`; the
//! item grant's lift stands in), the gems' and the empty chest's icons, the
//! `$0F9` chests and "I have enough" (`$04F8`). Poses tracked:
//! `meta/issues/ark-underworld-poses.md`.

use super::{Step, World, WorldError};
use crate::scene::{Presentation, Presses};
use assets::chests::{self, Chest, Contents};
use assets::layout::per_revision;
use assets::maps::scripts::EventFlags;

/// Frames from the press to the text: the patch, `COP 36`, a wait of 4 and
/// `COP C1 $16`.
const TEXT: u16 = 27;
/// The fanfare's hold (`COP C1 $168`) and its track.
const FANFARE: u16 = 360;
const FANFARE_TRACK: u8 = 0x34;
/// The lid's sound (port 3) and the gems' (port 2).
const OPEN_SOUND: u8 = 0x4A;
const GEMS_SOUND: u8 = 0x47;
/// The texts, Japanese and European: gems, an item, nothing, an item with
/// the fanfare, the fanfare's close, and a full inventory.
const TEXTS: [[u32; 6]; 2] = [
    [
        0x92_810E, 0x92_812C, 0x92_8164, 0x92_8147, 0x92_8162, 0x92_8096,
    ],
    [
        0x92_8131, 0x92_8151, 0x92_818F, 0x92_816F, 0x92_818D, 0x92_80B0,
    ],
];

/// A chest being opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Opening {
    chest: Chest,
    contents: Contents,
    /// Whether the inventory takes the item.
    room: bool,
    frame: u16,
    /// Frames of the fanfare left, once it plays.
    fanfare: Option<u16>,
}

/// What a frame of the opening does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// The lid opens.
    Open,
    /// The text by the contents (an index into [`TEXTS`]).
    Text(usize),
    /// The fanfare plays on with the window up.
    Fanfare,
    /// The text read: the contents are Ark's, or the lid closes.
    Done,
}

impl Opening {
    /// The text again next frame.
    fn retry(&mut self) {
        self.frame -= 1;
        self.fanfare = None;
    }

    /// One frame, the window `busy` or not.
    fn tick(&mut self, busy: bool) -> Option<Action> {
        self.frame = self.frame.saturating_add(1);
        if let Some(left) = &mut self.fanfare {
            *left = left.saturating_sub(1);
            return Some(if *left == 0 {
                Action::Done
            } else {
                Action::Fanfare
            });
        }
        match self.frame {
            1 => Some(Action::Open),
            TEXT => Some(Action::Text(match self.contents {
                _ if !self.room => 5,
                Contents::Gems(_) => 0,
                Contents::Empty => 2,
                Contents::Item(_) if self.chest.fanfare() => {
                    self.fanfare = Some(FANFARE);
                    3
                }
                Contents::Item(_) => 1,
            })),
            frame if frame > TEXT && !busy => Some(Action::Done),
            _ => None,
        }
    }
}

/// A chest's cell as the grid counts.
fn cell(chest: &Chest) -> (u16, u16) {
    (u16::from(chest.cell.0), u16::from(chest.cell.1))
}

/// The tile `$8D:91E4` draws an opened chest's cell with.
const fn opened(tile: u16) -> u16 {
    match tile {
        chests::CLOSED | chests::OPEN => chests::OPEN,
        _ => OTHER_OPEN,
    }
}

/// An opened chest whose cell holds neither chest tile.
const OTHER_OPEN: u16 = 0x0F9;

impl World<'_> {
    /// Draws the chests opened before open (`$8D:912B`).
    pub(super) fn open_opened_chests(&mut self) {
        let events = &self.globals.events;
        let width = usize::from(self.base.width);
        for chest in chests::chests(self.image, self.map) {
            if EventFlags::Bitmap(events).get(chest.opened) != Some(true) {
                continue;
            }
            let (column, row) = cell(&chest);
            let at = usize::from(row) * width + usize::from(column);
            if let Some(word) = self
                .base
                .room
                .cells()
                .get(at)
                .filter(|_| column < self.base.width)
            {
                self.globals
                    .patches
                    .push((column, row, opened(word & 0x1FF)));
            }
        }
    }

    /// Starts opening the chest the player faces Up at, if there is a
    /// closed one with a table entry. Returns whether it did.
    pub(super) fn open_chest(&mut self) -> bool {
        let Some(faced) = self.faced_tile(chests::CLOSED) else {
            return false;
        };
        let events = &self.globals.events;
        let set = |flag| EventFlags::Bitmap(events).get(flag) == Some(true);
        let found = chests::chests(self.image, self.map)
            .into_iter()
            .find_map(|chest| {
                (cell(&chest) == faced)
                    .then(|| chest.holds(set).map(|contents| (chest, contents)))?
            });
        let Some((chest, contents)) = found else {
            return false;
        };
        let room = match contents {
            Contents::Item(item) => self.globals.inventory.has_room(item),
            _ => true,
        };
        self.chest = Some(Opening {
            chest,
            contents,
            room,
            frame: 0,
            fanfare: None,
        });
        true
    }

    /// A frame of the chest's worker: the world runs on, the player held.
    pub(super) fn chest_frame(&mut self, presses: Presses) -> Result<Option<Step>, WorldError> {
        let Some(mut opening) = self.chest else {
            return Ok(None);
        };
        self.globals.dialogue.press(presses);
        let (column, row) = (
            u16::from(opening.chest.cell.0),
            u16::from(opening.chest.cell.1),
        );
        match opening.tick(self.globals.dialogue.busy()) {
            Some(Action::Open) => {
                self.globals.audio.sound_port3(OPEN_SOUND);
                self.globals.patches.push((column, row, chests::OPEN));
                if let (Contents::Item(item), true) = (opening.contents, opening.room) {
                    self.globals.presentation = Some(Presentation {
                        item,
                        frames: 0x7FFF,
                        age: 0,
                    });
                }
            }
            Some(Action::Text(text)) => {
                let source = TEXTS[per_revision(self.image, 0, 1)][text];
                let (item, gems) = match opening.contents {
                    Contents::Item(item) => (item, 0),
                    Contents::Gems(gems) => (0, crate::inventory::bcd(u32::from(gems))),
                    Contents::Empty => (0, 0),
                };
                let pages =
                    assets::text::HouseDialogue::decode_reading(
                        self.image,
                        source,
                        |at| match at {
                            0x09C7 => Some(item),
                            0x09CB | 0x09CC => Some(gems.to_le_bytes()[usize::from(at - 0x09CB)]),
                            0x09C8 => Some(0),
                            _ => None,
                        },
                    )
                    .unwrap_or_default();
                if !self.globals.dialogue.request(pages) {
                    // The window is another's: try again next frame.
                    opening.retry();
                } else if opening.fanfare.is_some() {
                    self.globals.audio.fanfare(FANFARE_TRACK, FANFARE);
                }
            }
            Some(Action::Done) => {
                if opening.fanfare.is_some() {
                    // `$92:8162`: the window closes.
                    self.globals.dialogue.request(Vec::new());
                }
                self.finish_chest(&opening);
                self.chest = None;
                self.run_actors()?;
                return Ok(Some(Step::Stayed));
            }
            Some(Action::Fanfare) | None => {}
        }
        self.chest = Some(opening);
        self.run_actors()?;
        Ok(Some(Step::Stayed))
    }

    /// The contents become Ark's and the flag is set; or, without room, the
    /// lid closes again.
    fn finish_chest(&mut self, opening: &Opening) {
        self.globals.presentation = None;
        let (column, row) = (
            u16::from(opening.chest.cell.0),
            u16::from(opening.chest.cell.1),
        );
        if !opening.room {
            self.globals.patches.push((column, row, chests::CLOSED));
            return;
        }
        match opening.contents {
            Contents::Item(item) => {
                self.globals.inventory.add(item);
            }
            Contents::Gems(gems) => {
                self.globals.inventory.add_money(u32::from(gems));
                self.globals.audio.sound_port2(GEMS_SOUND);
            }
            Contents::Empty => {}
        }
        self.globals.write_flag(0x8000 | opening.chest.opened);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opening(contents: Contents, flags: u8, room: bool) -> Opening {
        Opening {
            chest: Chest {
                cell: (12, 37),
                flags,
                condition: chests::Condition::None,
                contents,
                opened: 0x580,
            },
            contents,
            room,
            frame: 0,
            fanfare: None,
        }
    }

    #[test]
    fn the_lid_opens_then_the_text_shows_and_reading_it_ends() {
        let mut chest = opening(Contents::Item(0x10), 0, true);
        assert_eq!(chest.tick(false), Some(Action::Open));
        let quiet = (2..TEXT).all(|_| chest.tick(false).is_none());
        assert!(quiet);
        assert_eq!(chest.tick(false), Some(Action::Text(1)));
        assert_eq!(chest.tick(true), None);
        assert_eq!(chest.tick(false), Some(Action::Done));
    }

    #[test]
    fn the_text_follows_the_contents_and_the_room() {
        let text = |contents, flags, room| {
            let mut chest = opening(contents, flags, room);
            (1..TEXT).for_each(|_| {
                chest.tick(false);
            });
            chest.tick(false)
        };
        assert_eq!(text(Contents::Gems(30), 0, true), Some(Action::Text(0)));
        assert_eq!(text(Contents::Empty, 0, true), Some(Action::Text(2)));
        assert_eq!(text(Contents::Item(0x59), 8, true), Some(Action::Text(3)));
        assert_eq!(text(Contents::Item(0x10), 0, false), Some(Action::Text(5)));
    }

    #[test]
    fn the_fanfare_holds_360_frames_whatever_the_window() {
        let mut chest = opening(Contents::Item(0x59), 8, true);
        (1..=TEXT).for_each(|_| {
            chest.tick(false);
        });
        let held = (1..FANFARE).all(|_| chest.tick(false) == Some(Action::Fanfare));
        assert!(held);
        assert_eq!(chest.tick(true), Some(Action::Done));
    }
}
