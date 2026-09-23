//! Script-driven dialogue: the text request, its pages, and the choice that
//! may follow on the same window.
//!
//! This is the state `$0DC2` names natively: nonzero while a request or a
//! choice is active (`$FFFF` for a choice). Scripts publish a request with
//! `COP 1B`, show it with `COP 1F` (blocking) or `COP 20` (cooperative), and
//! ask with `COP 1A`. Input reaches it as button presses, not held state.

use assets::text::{Acknowledgement, DialogueChoice, DialoguePage};

/// Buttons pressed this frame, as edges.
#[allow(clippy::struct_excessive_bools)] // four independent buttons, not a state
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Presses {
    /// A or L: acknowledges a page, confirms a choice.
    pub confirm: bool,
    /// B: cancels a choice with result 0; does not acknowledge a page.
    pub cancel: bool,
    /// Up on the pad, which moves a choice cursor.
    pub up: bool,
    /// Down on the pad.
    pub down: bool,
}

/// What scripts read and write beyond their own actor.
#[derive(Debug, Clone)]
pub struct Globals {
    /// The `$7E:06C0` event-flag bitmap.
    pub events: Vec<u8>,
    /// The dialogue window.
    pub dialogue: Dialogue,
    /// `$045E`: pad buttons scripts have locked (`COP 2A`/`29`), as the SNES
    /// pad word. `$FF50` leaves A and L free to acknowledge text.
    pub input_mask: u16,
    /// Items scripts have given (`COP 54`), in order. The inventory's slots
    /// and limits (`$7F:8000`) are not modelled.
    pub items: Vec<u8>,
    /// `$0640..$06BF`: map-local counters `COP 4B` keeps, such as the blue
    /// door's hit count. Cleared on every map load (`$8D:8AED`).
    pub counters: Vec<u8>,
}

impl Default for Globals {
    fn default() -> Self {
        Self::with_events(Vec::new())
    }
}

impl Globals {
    /// Scripts' view with these flags and nothing else in force.
    #[must_use]
    pub fn with_events(events: Vec<u8>) -> Self {
        Self {
            events,
            dialogue: Dialogue::default(),
            input_mask: 0,
            items: Vec::new(),
            counters: vec![0; 0x80],
        }
    }

    /// `COP 4B`: stores a word at `$0640 + op`, or with bit 7 adds it in BCD,
    /// capped at 9999. Bit 6's subtraction is not modelled; returns false.
    pub fn count(&mut self, op: u8, word: u16) -> bool {
        let at = usize::from(op & 0x3F);
        let Some(slot) = self.counters.get_mut(at..at + 2) else {
            return false;
        };
        let value = match op & 0xC0 {
            0 => word,
            0x80 => bcd_add(u16::from_le_bytes([slot[0], slot[1]]), word),
            _ => return false,
        };
        slot.copy_from_slice(&value.to_le_bytes());
        true
    }

    /// A counter's word, as [`Self::count`] keeps it.
    #[must_use]
    pub fn counter(&self, op: u8) -> u16 {
        let at = usize::from(op & 0x3F);
        self.counters
            .get(at..at + 2)
            .map_or(0, |slot| u16::from_le_bytes([slot[0], slot[1]]))
    }

    /// `COP 07`'s write: bit 15 set sets flag `word & $0FFF`, clear clears it.
    pub fn write_flag(&mut self, word: u16) {
        let index = usize::from(word & 0x0FFF);
        if let Some(byte) = self.events.get_mut(index / 8) {
            let bit = 1 << (index % 8);
            if word & 0x8000 == 0 {
                *byte &= !bit;
            } else {
                *byte |= bit;
            }
        }
    }
}

/// Four-digit BCD addition, capped at 9999.
fn bcd_add(a: u16, b: u16) -> u16 {
    let decimal = |bcd: u16| {
        (0..4)
            .rev()
            .fold(0u32, |n, i| n * 10 + u32::from((bcd >> (i * 4)) & 0xF))
    };
    let sum = (decimal(a) + decimal(b)).min(9999);
    (0..4).fold(0, |bcd, i| {
        bcd | (u16::try_from(sum / 10u32.pow(i) % 10).unwrap_or(0) << (i * 4))
    })
}

/// The pad's direction bits in the SNES word: Up, Down, Left, Right.
pub const PAD_DIRECTIONS: u16 = 0x0F00;

/// A page's boundary, which is all the state machine needs to know of it.
pub trait Page {
    /// What ends this page.
    fn acknowledgement(&self) -> Acknowledgement;
}

impl Page for DialoguePage {
    fn acknowledgement(&self) -> Acknowledgement {
        DialoguePage::acknowledgement(self)
    }
}

/// The dialogue window and whatever owns it.
#[derive(Debug, Clone)]
pub struct Dialogue<P = DialoguePage> {
    /// The active request's pages and the one showing.
    text: Option<(Vec<P>, usize)>,
    /// A page a request returned from without an acknowledgement (`$D4`),
    /// still on screen for the choice that follows.
    retained: Option<P>,
    /// The open choice: catalog, cursor as a native result (1 or 2).
    choice: Option<(DialogueChoice, u8)>,
}

impl<P> Default for Dialogue<P> {
    fn default() -> Self {
        Self {
            text: None,
            retained: None,
            choice: None,
        }
    }
}

/// What the window shows.
pub struct View<'a, P> {
    /// The page on screen.
    pub page: &'a P,
    /// Where the choice cursor sits, relative to the page, when one is open.
    pub cursor: Option<[u16; 2]>,
}

impl<P: Page> Dialogue<P> {
    /// Whether a request or choice owns the window (`$0DC2 != 0`).
    #[must_use]
    pub const fn busy(&self) -> bool {
        self.text.is_some() || self.choice.is_some()
    }

    /// `COP 1B`: publishes a request. Refused while the window is busy; the
    /// script retries. A request whose first page returns at once ends here.
    pub fn request(&mut self, pages: Vec<P>) -> bool {
        if self.busy() {
            return false;
        }
        self.retained = None;
        if !pages.is_empty() {
            self.text = Some((pages, 0));
            self.settle();
        }
        true
    }

    /// Ends the request when the page showing returns without acknowledgement.
    fn settle(&mut self) {
        if let Some((pages, index)) = &mut self.text {
            if pages[*index].acknowledgement() == Acknowledgement::None {
                let (mut pages, index) = self.text.take().expect("just matched");
                self.retained = Some(pages.swap_remove(index));
            }
        }
    }

    /// `COP 1A`: opens a choice on the page still showing. Refused while a
    /// request is active.
    pub fn ask(&mut self, choice: DialogueChoice) -> bool {
        if self.busy() {
            return false;
        }
        self.choice = Some((choice, 1));
        true
    }

    /// Applies one frame's presses. Returns the choice result (0 cancel, 1
    /// or 2) when a choice closes this frame.
    pub fn press(&mut self, presses: Presses) -> Option<u8> {
        if let Some((choice, cursor)) = &mut self.choice {
            if presses.cancel {
                self.close_choice();
                return Some(0);
            }
            if presses.confirm {
                let result = *cursor;
                self.close_choice();
                return Some(result);
            }
            // Up, Down follow the catalog's neighbour links.
            let option = &choice.options[usize::from(*cursor - 1)];
            let link = if presses.up {
                option.neighbors[0]
            } else if presses.down {
                option.neighbors[1]
            } else {
                None
            };
            if let Some(next) = link {
                *cursor = next;
            }
            return None;
        }
        if !presses.confirm {
            return None;
        }
        if let Some((pages, index)) = &mut self.text {
            match pages[*index].acknowledgement() {
                Acknowledgement::Next if *index + 1 < pages.len() => {
                    *index += 1;
                    self.settle();
                }
                _ => self.text = None,
            }
        }
        None
    }

    fn close_choice(&mut self) {
        self.choice = None;
        self.retained = None;
    }

    /// The page on screen, if any, and the choice cursor.
    #[must_use]
    pub fn view(&self) -> Option<View<'_, P>> {
        if let Some((pages, index)) = &self.text {
            return Some(View {
                page: &pages[*index],
                cursor: None,
            });
        }
        let page = self.retained.as_ref()?;
        let cursor = self
            .choice
            .as_ref()
            .map(|(choice, cursor)| choice.options[usize::from(*cursor - 1)].position);
        Some(View { page, cursor })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assets::text::DialogueOption;

    #[derive(Debug, Clone, PartialEq)]
    struct Stub(Acknowledgement, u8);

    impl Page for Stub {
        fn acknowledgement(&self) -> Acknowledgement {
            self.0
        }
    }

    fn pages(kinds: &[Acknowledgement]) -> Vec<Stub> {
        (0u8..).zip(kinds).map(|(i, kind)| Stub(*kind, i)).collect()
    }

    const A: Presses = Presses {
        confirm: true,
        cancel: false,
        up: false,
        down: false,
    };
    const B: Presses = Presses {
        confirm: false,
        cancel: true,
        up: false,
        down: false,
    };

    fn choice() -> DialogueChoice {
        let option = |result, neighbors| DialogueOption {
            source: 0,
            result,
            position: [8 * u16::from(result), 16],
            neighbors,
        };
        DialogueChoice {
            catalog: 0,
            options: [
                option(1, [Some(2), Some(2), None, None]),
                option(2, [Some(1), Some(1), None, None]),
            ],
        }
    }

    #[test]
    fn pages_advance_on_confirm_and_the_end_page_closes() {
        use Acknowledgement::{End, Next};
        let mut dialogue = Dialogue::default();
        assert!(dialogue.request(pages(&[Next, Next, End])));
        for shown in 0..3 {
            assert_eq!(dialogue.view().unwrap().page.1, shown);
            // B and no press do not acknowledge a page.
            assert_eq!(dialogue.press(B), None);
            assert_eq!(dialogue.press(Presses::default()), None);
            assert!(dialogue.busy());
            dialogue.press(A);
        }
        assert!(!dialogue.busy());
        assert!(dialogue.view().is_none());
    }

    #[test]
    fn a_flag_write_sets_on_bit_fifteen_and_clears_without_it() {
        let mut globals = Globals::with_events(vec![0; 8]);
        globals.write_flag(0x8026);
        assert_eq!(globals.events[4], 0x40);
        globals.write_flag(0x0026);
        assert_eq!(globals.events[4], 0);
        // Beyond the bitmap: ignored rather than a panic.
        globals.write_flag(0x8FFF);
    }

    #[test]
    fn a_busy_window_refuses_a_second_request() {
        let mut dialogue = Dialogue::default();
        assert!(dialogue.request(pages(&[Acknowledgement::End])));
        assert!(!dialogue.request(pages(&[Acknowledgement::End])));
        assert!(!dialogue.ask(choice()));
    }

    #[test]
    fn a_page_that_returns_at_once_stays_up_for_the_choice() {
        let mut dialogue = Dialogue::default();
        dialogue.request(pages(&[Acknowledgement::Next, Acknowledgement::None]));
        dialogue.press(A);
        // The request is over without another press, the page still showing.
        assert!(!dialogue.busy());
        assert_eq!(dialogue.view().unwrap().page.1, 1);
        assert!(dialogue.ask(choice()));
        let view = dialogue.view().unwrap();
        assert_eq!((view.page.1, view.cursor), (1, Some([8, 16])));
        // Down moves to option 2, Up back; A confirms the one under the cursor.
        let down = Presses {
            down: true,
            ..Presses::default()
        };
        assert_eq!(dialogue.press(down), None);
        assert_eq!(dialogue.view().unwrap().cursor, Some([16, 16]));
        assert_eq!(dialogue.press(A), Some(2));
        assert!(dialogue.view().is_none());
    }

    #[test]
    fn cancel_answers_zero_and_an_immediate_first_page_ends_the_request() {
        let mut dialogue = Dialogue::default();
        dialogue.request(pages(&[Acknowledgement::None]));
        assert!(!dialogue.busy());
        dialogue.ask(choice());
        assert_eq!(dialogue.press(B), Some(0));
        assert!(!dialogue.busy());
        // An empty request is accepted and shows nothing.
        assert!(dialogue.request(Vec::new()));
        assert!(!dialogue.busy());
    }
}
