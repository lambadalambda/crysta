//! The Records screen's run ([notes](../../../docs/records-screen.md)): the
//! desk's save screen as `$87:8590` runs it, frame by frame from the press
//! that opened it. The world applies what it asks for: a save, a sound, a
//! track.

use room_core::Direction;

/// The frames that differ between the ROMs, counted from the open or the
/// save press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Timing {
    /// The screen shows (`$2100 = $0F`).
    ready: u16,
    /// The jingle's and the room track's stop commands (`$F0`).
    jingle: u16,
    room: u16,
    /// The forced blank that starts the way back.
    exit: u16,
    /// The saved page's message, and each slot typed again.
    message: u16,
    slots: [u16; 3],
}

const JAPAN: Timing = Timing {
    ready: 46,
    jingle: 19,
    room: 238,
    exit: 289,
    message: 11,
    slots: [12, 16, 19],
};
const EUROPE: Timing = Timing {
    ready: 47,
    jingle: 16,
    room: 229,
    exit: 276,
    message: 10,
    slots: [11, 14, 16],
};

/// The room's fade out: full for two frames, then a step a frame.
const FADE_START: u16 = 2;
/// The frame after the fade's last step: forced blank.
const BLANK: u16 = 16;
/// Frames from the forced blank of the way back to the room's first step
/// of brightness, then its 15 steps.
const BACK_DARK: u16 = 5;
const STEPS: u16 = 15;
/// A cancel's forced blank, after the press.
const CANCEL_EXIT: u16 = 2;
/// The saved page: the old text goes, then ` 1`–` 3` a frame apart.
const SAVED_CLEAR: u16 = 2;
const SAVED_LINES: u16 = 4;
/// Up and Down repeat once held this long, then this often (`$0454`).
const REPEAT_DELAY: u16 = 16;
const REPEAT_EVERY: u16 = 5;

/// Up and Down as the menus' input loops see them (`$0454`): the press,
/// then held [`REPEAT_DELAY`] frames, then every [`REPEAT_EVERY`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Repeat(Option<(Direction, u16)>);

impl Repeat {
    /// This frame's Up or Down, from the direction held.
    pub(crate) fn step(&mut self, direction: Option<Direction>) -> Option<Direction> {
        let direction = direction.filter(|&d| matches!(d, Direction::Up | Direction::Down));
        let held = match (self.0, direction) {
            (Some((before, frames)), Some(now)) if before == now => frames + 1,
            _ => 0,
        };
        self.0 = direction.map(|d| (d, held));
        let fires = held == 0
            || (held >= REPEAT_DELAY && (held - REPEAT_DELAY).is_multiple_of(REPEAT_EVERY));
        direction.filter(|_| fires)
    }
}
/// The cursor's sound, on port 3.
pub const CURSOR_SOUND: u8 = 0x22;
/// The jingle a save plays.
pub const JINGLE: u8 = 0x35;

/// What the world does for the screen this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Save to this slot (`$8D:A6FB`).
    Save(u8),
    /// Sound on port 3.
    Sound(u8),
    /// The save's jingle.
    Jingle,
    /// The room's track again.
    RoomTrack,
    /// The screen closed; the desk's script goes on.
    Closed,
}

/// Where the run stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Fading the room out and setting up.
    Opening,
    /// Waiting for a press.
    Choosing,
    /// Saved to `slot`.
    Saved { slot: u8 },
    /// Cancelled.
    Cancelled,
}

/// What the host draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// The room, at this brightness (0–15).
    Room(u8),
    /// Forced blank.
    Blank,
    /// The screen itself.
    Screen(Page),
}

/// The screen's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    /// The slot the cursor points at.
    pub cursor: u8,
    /// Whether the title shows.
    pub title: bool,
    /// The text on it.
    pub text: Text,
}

/// The screen's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    /// None: cleared.
    None,
    /// The page as it opens: the slots, the current game, the question.
    Entry,
    /// The saved page, typed so far: its first `lines` of ` 1`–` 3`,
    /// whether the message shows, and how many slots are typed again.
    Saved {
        /// The slot saved to.
        slot: u8,
        /// Lines of ` 1`–` 3` typed.
        lines: u8,
        /// Whether "`n` saved" shows.
        message: bool,
        /// Slots typed again.
        slots: u8,
    },
}

/// The Records screen under way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Records {
    timing: Timing,
    phase: Phase,
    /// Frames since the open, or since the press that saved or cancelled.
    frame: u16,
    cursor: u8,
    /// The cursor as the sprite shows it: it follows a frame late.
    shown: u8,
    /// Up or Down held.
    held: Repeat,
}

impl Records {
    /// Opens the screen in the frame of the press, the cursor on `cursor`
    /// (`$0496`, the slot last saved or loaded).
    #[must_use]
    pub fn open(europe: bool, cursor: u8) -> Self {
        Self {
            timing: if europe { EUROPE } else { JAPAN },
            phase: Phase::Opening,
            frame: 0,
            cursor: cursor.min(2),
            shown: cursor.min(2),
            held: Repeat::default(),
        }
    }

    /// Whether the input loop runs, which holds the play clock.
    #[must_use]
    pub fn choosing(&self) -> bool {
        self.phase == Phase::Choosing
    }

    /// One frame: `direction` held, `confirm` (A) and `cancel` (B) pressed.
    pub fn step(
        &mut self,
        direction: Option<Direction>,
        confirm: bool,
        cancel: bool,
    ) -> Option<Event> {
        self.frame = self.frame.saturating_add(1);
        self.shown = self.cursor;
        match self.phase {
            Phase::Opening => {
                if self.frame > self.timing.ready {
                    self.phase = Phase::Choosing;
                    return self.choose(direction, confirm, cancel);
                }
                None
            }
            Phase::Choosing => self.choose(direction, confirm, cancel),
            Phase::Saved { .. } => match self.frame {
                frame if frame == self.timing.jingle => Some(Event::Jingle),
                frame if frame == self.timing.room => Some(Event::RoomTrack),
                frame if frame == self.timing.exit + BACK_DARK + STEPS => Some(Event::Closed),
                _ => None,
            },
            Phase::Cancelled => {
                (self.frame == CANCEL_EXIT + BACK_DARK + STEPS).then_some(Event::Closed)
            }
        }
    }

    /// The input loop (`$87:8772`): B, then A, then Up, then Down.
    fn choose(
        &mut self,
        direction: Option<Direction>,
        confirm: bool,
        cancel: bool,
    ) -> Option<Event> {
        let repeated = self.held.step(direction);
        if cancel {
            (self.phase, self.frame) = (Phase::Cancelled, 0);
            return None;
        }
        if confirm {
            (self.phase, self.frame) = (Phase::Saved { slot: self.cursor }, 0);
            return Some(Event::Save(self.cursor));
        }
        let cursor = match repeated? {
            Direction::Up => self.cursor.checked_sub(1)?,
            Direction::Down if self.cursor < 2 => self.cursor + 1,
            _ => return None,
        };
        self.cursor = cursor;
        Some(Event::Sound(CURSOR_SOUND))
    }

    /// What shows this frame.
    #[must_use]
    pub fn view(&self) -> View {
        let frame = self.frame;
        let page = |title, text| {
            View::Screen(Page {
                cursor: self.shown,
                title,
                text,
            })
        };
        match self.phase {
            Phase::Opening if frame < FADE_START => View::Room(15),
            Phase::Opening if frame < BLANK => View::Room(level(BLANK - frame)),
            Phase::Opening if frame < self.timing.ready => View::Blank,
            Phase::Opening | Phase::Choosing => page(true, Text::Entry),
            Phase::Cancelled if frame < CANCEL_EXIT => page(false, Text::None),
            Phase::Cancelled => back(frame - CANCEL_EXIT),
            Phase::Saved { .. } if frame >= self.timing.exit => back(frame - self.timing.exit),
            Phase::Saved { slot } => page(true, self.saved_text(slot)),
        }
    }

    /// The saved page as typed `frame` frames after the press.
    fn saved_text(&self, slot: u8) -> Text {
        let frame = self.frame;
        if frame < SAVED_CLEAR {
            return Text::Entry;
        }
        let count = |at: u16| u8::from(frame >= at);
        Text::Saved {
            slot,
            lines: (0..3).map(|line| count(SAVED_LINES + line)).sum(),
            message: frame >= self.timing.message,
            slots: self.timing.slots.iter().map(|&at| count(at)).sum(),
        }
    }
}

/// A brightness level, 15 at most.
fn level(steps: u16) -> u8 {
    u8::try_from(steps.min(STEPS)).unwrap_or(15)
}

/// The way back, `frame` frames after its forced blank: dark, then the
/// room's 15 steps.
fn back(frame: u16) -> View {
    if frame < BACK_DARK {
        View::Blank
    } else {
        View::Room(level(frame - BACK_DARK + 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(records: &mut Records, frames: u16) -> Vec<Event> {
        (0..frames)
            .filter_map(|_| records.step(None, false, false))
            .collect()
    }

    #[test]
    fn the_room_fades_then_the_screen_shows_at_once() {
        let mut records = Records::open(false, 0);
        assert_eq!(records.view(), View::Room(15));
        run(&mut records, 2);
        assert_eq!(records.view(), View::Room(14));
        run(&mut records, 13);
        assert_eq!(records.view(), View::Room(1));
        run(&mut records, 1);
        assert_eq!(records.view(), View::Blank);
        run(&mut records, 29);
        assert_eq!(records.view(), View::Blank, "frame 45");
        run(&mut records, 1);
        assert!(matches!(
            records.view(),
            View::Screen(Page {
                title: true,
                text: Text::Entry,
                ..
            })
        ));
        assert!(!records.choosing(), "input from the next frame");
        run(&mut records, 1);
        assert!(records.choosing());
        // The European setup takes a frame more.
        let mut records = Records::open(true, 0);
        run(&mut records, 46);
        assert_eq!(records.view(), View::Blank);
    }

    #[test]
    fn the_cursor_stops_at_the_ends_and_repeats_when_held() {
        let mut records = Records::open(false, 1);
        run(&mut records, 46);
        let down = Some(Direction::Down);
        assert_eq!(
            records.step(down, false, false),
            Some(Event::Sound(CURSOR_SOUND))
        );
        let shown = |records: &Records| match records.view() {
            View::Screen(page) => page.cursor,
            _ => 9,
        };
        assert_eq!(shown(&records), 1, "the sprite moves a frame late");
        assert_eq!(records.cursor, 2);
        // Held on: no wrap, no sound at the end.
        let events: Vec<_> = (0..40)
            .filter_map(|_| records.step(down, false, false))
            .collect();
        assert!(events.is_empty());
        // Up held: the press, then 16 frames on, then every 5.
        let up = Some(Direction::Up);
        assert_eq!(
            records.step(up, false, false),
            Some(Event::Sound(CURSOR_SOUND))
        );
        let fired: Vec<usize> = (1..=21)
            .filter(|_| records.step(up, false, false).is_some())
            .collect();
        assert_eq!(fired, [16], "slot 0 reached; 21 would move past it");
        assert_eq!(records.cursor, 0);
    }

    #[test]
    fn a_save_types_its_page_plays_the_jingle_and_comes_back() {
        let mut records = Records::open(false, 0);
        run(&mut records, 47);
        assert_eq!(records.step(None, true, false), Some(Event::Save(0)));
        run(&mut records, 1);
        assert!(matches!(
            records.view(),
            View::Screen(Page {
                text: Text::Entry,
                ..
            })
        ));
        run(&mut records, 10);
        assert_eq!(
            records.view(),
            View::Screen(Page {
                cursor: 0,
                title: true,
                text: Text::Saved {
                    slot: 0,
                    lines: 3,
                    message: true,
                    slots: 0
                },
            }),
            "frame 11: the message"
        );
        let events = run(&mut records, 300);
        assert_eq!(events, [Event::Jingle, Event::RoomTrack, Event::Closed]);
    }

    #[test]
    fn a_cancel_clears_the_page_and_fades_the_room_back_in() {
        let mut records = Records::open(false, 0);
        run(&mut records, 47);
        assert_eq!(records.step(None, true, true), None, "B wins over A");
        assert!(matches!(
            records.view(),
            View::Screen(Page {
                title: false,
                text: Text::None,
                ..
            })
        ));
        run(&mut records, 2);
        assert_eq!(records.view(), View::Blank);
        run(&mut records, 5);
        assert_eq!(records.view(), View::Room(1), "frame 7");
        assert_eq!(run(&mut records, 15), [Event::Closed]);
        assert_eq!(records.view(), View::Room(15));
    }
}
