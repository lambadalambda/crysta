//! The Restart file select's run ([notes](../../../docs/restart-screen.md)):
//! the screen the title's Start opens, as map `$04`'s actor (`$87:8000`)
//! runs it, frame by frame from the forced blank after the title's fade.
//! It owns the SRAM while it runs (Copy and Erase write it) and ends by
//! loading a slot or starting a new game.

use crate::audio::{Audio, Cue};
use crate::records::Repeat;
use crate::sram::Sram;
use room_core::Direction;

/// The frames that differ between the ROMs, from the forced blank (the
/// doc's S+52).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Timing {
    /// Map `$04`'s song: its stop command (`$F0`).
    music: u16,
    /// The first step of the fade in, and the input loop's first frame.
    fade_in: u16,
    input: u16,
    /// A redraw's slots typed again, and the cursor's return, from the
    /// confirming press.
    slots: [u16; 3],
    cursor: u16,
}

const JAPAN: Timing = Timing {
    music: 57,
    fade_in: 161,
    input: 218,
    slots: [12, 16, 18],
    cursor: 20,
};
const EUROPE: Timing = Timing {
    music: 59,
    fade_in: 159,
    input: 216,
    slots: [12, 15, 17],
    cursor: 19,
};

/// Map `$04`'s selection (`$04B2`).
const SONG: u8 = 0x1B;
/// The fade in: a step every 4 frames up to 15.
const FADE_STEP: u16 = 4;
/// A load's fade out: a step a frame from A+4, forced blank at A+18.
const LOAD_FADE: (u16, u16) = (4, 18);
/// A new game's: a step every 4 frames from N+7, forced blank at N+64.
const NEW_FADE: (u16, u16) = (7, 64);
/// A redraw: the text goes at E+2; the page's six lines, then the slots.
const CLEAR: u16 = 2;
const LINES: [u16; 6] = [4, 5, 6, 7, 9, 11];
/// A copy's redraw starts two frames later.
const COPY_DELAY: u16 = 2;
/// The loop resumes this long after the cursor's return.
const RESUME: u16 = 2;
/// The entries: slots 1–3, New Game, Copy Data, Erase Data.
const NEW_GAME: u8 = 3;
const COPY: u8 = 4;
const LAST: u8 = 5;
/// Sounds on port 3: the cursor, a choice, a confirm, back, a refusal.
const MOVE: u8 = 0x22;
const CHOOSE: u8 = 0x21;
const CONFIRM: u8 = 0x23;
const BACK: u8 = 0x24;
const REFUSE: u8 = 0x1B;

/// How the screen ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Load this valid slot (`$0496` = slot).
    Load(u8),
    /// A new game; from an empty slot, `$0496` = that slot.
    NewGame(Option<u8>),
}

/// Where the run stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Loading map `$04` and fading in.
    Entry,
    /// The main loop.
    Main,
    /// Copy: choosing the source, then (`source`) the destination.
    Copy { source: Option<u8> },
    /// Erase: choosing the slot, then (`slot`) confirming.
    Erase { slot: Option<u8> },
    /// The page typed again after a copy, an erase or B; `delay` two
    /// frames more after a copy.
    Redraw { delay: u16 },
    /// Fading out to `outcome`, from `start`.
    Leaving { outcome: Outcome, fade: (u16, u16) },
}

/// What the host draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Black: forced blank or brightness 0.
    Blank,
    /// The screen.
    Screen(Page),
}

/// The screen's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    /// Brightness, 1–15.
    pub brightness: u8,
    /// The cursor's entry (0–5), `None` while it blinks off.
    pub cursor: Option<u8>,
    /// Copy's second cursor, on a slot.
    pub second: Option<u8>,
    /// The page's lines typed (0–6).
    pub lines: u8,
    /// Slots typed.
    pub slots: u8,
}

/// The Restart screen under way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restart {
    timing: Timing,
    sram: Sram,
    /// Whether the screen wrote the SRAM since the host last kept it.
    written: bool,
    phase: Phase,
    /// Frames since the start, and since the phase's press.
    frame: u16,
    since: u16,
    /// `$04C8`, and the entry the cursor sprite shows (a frame late).
    cursor: u8,
    shown: u8,
    /// Where the main cursor stays while Copy's second cursor moves or a
    /// choice waits, until a redraw returns it.
    anchor: Option<u8>,
    held: Repeat,
    audio: Audio,
}

impl Restart {
    /// Starts the screen with the cartridge's SRAM; the slot list repairs
    /// it as it draws (`$87:CB4B`).
    #[must_use]
    pub fn open(europe: bool, sram: Sram) -> Self {
        let before = sram.clone();
        let sram = sram.repaired();
        let cursor = sram.cursor();
        Self {
            timing: if europe { EUROPE } else { JAPAN },
            written: sram != before,
            sram,
            phase: Phase::Entry,
            frame: 0,
            since: 0,
            cursor,
            shown: cursor,
            anchor: None,
            held: Repeat::default(),
            audio: Audio::default(),
        }
    }

    /// The SRAM as the screen left it.
    #[must_use]
    pub fn sram(&self) -> &Sram {
        &self.sram
    }

    /// Whether the screen wrote the SRAM since the last call.
    pub fn take_sram_write(&mut self) -> bool {
        std::mem::take(&mut self.written)
    }

    /// Music and sound requests made since the last call.
    pub fn take_cues(&mut self) -> Vec<Cue> {
        self.audio.take()
    }

    /// One frame: `direction` held, `confirm` (A) and `cancel` (B) pressed.
    /// Returns how the screen ended, on the frame it hands over.
    pub fn step(
        &mut self,
        direction: Option<Direction>,
        confirm: bool,
        cancel: bool,
    ) -> Option<Outcome> {
        self.frame = self.frame.saturating_add(1);
        self.since = self.since.saturating_add(1);
        self.shown = self.cursor;
        // `$0454` counts on whatever the loops do.
        let moved = self.held.step(direction);
        if self.frame == self.timing.music {
            self.audio.load_map(Some(SONG));
        }
        let outcome = match self.phase {
            Phase::Entry => {
                if self.frame >= self.timing.input {
                    self.phase = Phase::Main;
                    self.input(moved, confirm, cancel);
                }
                None
            }
            Phase::Redraw { delay } => {
                if self.since == delay + self.timing.cursor {
                    self.cursor = self.sram.cursor();
                    self.anchor = None;
                } else if self.since == delay + self.timing.cursor + RESUME {
                    self.phase = Phase::Main;
                }
                None
            }
            Phase::Leaving { outcome, fade } => (self.since == fade.1).then_some(outcome),
            Phase::Main | Phase::Copy { .. } | Phase::Erase { .. } => {
                self.input(moved, confirm, cancel);
                None
            }
        };
        self.audio.end_frame();
        outcome
    }

    fn begin(&mut self, phase: Phase) {
        (self.phase, self.since) = (phase, 0);
    }

    fn sound(&mut self, sound: u8) {
        self.audio.sound_port3(sound);
    }

    /// The loops (`$87:81BE`, Copy `$87:825D`, Erase `$87:8342`): A, B,
    /// then Up and Down, which stop at the ends.
    fn input(&mut self, moved: Option<Direction>, confirm: bool, cancel: bool) {
        let last = if self.phase == Phase::Main { LAST } else { 2 };
        match self.phase {
            // Erase waits two frames after its choice (`COP C1 2`).
            Phase::Erase { slot: Some(_) } if self.since <= 2 => {}
            _ if confirm => self.confirm(),
            Phase::Copy { .. } | Phase::Erase { .. } if cancel => self.cancel(),
            // Confirming an erase takes only A or B.
            Phase::Erase { slot: Some(_) } => {}
            _ => match moved {
                Some(Direction::Up) if self.cursor > 0 => {
                    self.cursor -= 1;
                    self.sound(MOVE);
                }
                Some(Direction::Down) if self.cursor < last => {
                    self.cursor += 1;
                    self.sound(MOVE);
                }
                _ => {}
            },
        }
    }

    fn confirm(&mut self) {
        let cursor = self.cursor;
        match self.phase {
            Phase::Main if cursor <= 2 && self.sram.valid(usize::from(cursor)) => {
                self.sound(CONFIRM);
                self.leave(Outcome::Load(cursor), LOAD_FADE);
            }
            Phase::Main if cursor <= NEW_GAME => {
                self.sound(CONFIRM);
                // An empty slot's failed load costs a frame.
                let (slot, late) = if cursor < NEW_GAME {
                    (Some(cursor), 1)
                } else {
                    (None, 0)
                };
                self.leave(
                    Outcome::NewGame(slot),
                    (NEW_FADE.0 + late, NEW_FADE.1 + late),
                );
            }
            Phase::Main => {
                self.sound(CONFIRM);
                self.cursor = 0;
                self.begin(if cursor == COPY {
                    Phase::Copy { source: None }
                } else {
                    Phase::Erase { slot: None }
                });
            }
            Phase::Copy { source: None } => {
                self.sound(CHOOSE);
                self.anchor = Some(cursor);
                self.begin(Phase::Copy {
                    source: Some(cursor),
                });
            }
            Phase::Copy {
                source: Some(source),
            } => {
                let copied = !self.sram.valid(usize::from(cursor))
                    && self
                        .sram
                        .copy_slot(usize::from(source), usize::from(cursor));
                if copied {
                    self.sound(CONFIRM);
                    self.written = true;
                    self.redraw(COPY_DELAY);
                } else {
                    self.sound(REFUSE);
                }
            }
            Phase::Erase { slot: None } if self.sram.valid(usize::from(cursor)) => {
                self.sound(CHOOSE);
                self.anchor = Some(cursor);
                self.begin(Phase::Erase { slot: Some(cursor) });
            }
            Phase::Erase { slot: None } => self.sound(REFUSE),
            Phase::Erase { slot: Some(slot) } => {
                self.sound(CONFIRM);
                self.sram.erase(usize::from(slot));
                self.written = true;
                self.redraw(0);
            }
            Phase::Entry | Phase::Redraw { .. } | Phase::Leaving { .. } => {}
        }
    }

    fn cancel(&mut self) {
        self.sound(BACK);
        match self.phase {
            Phase::Copy {
                source: Some(source),
            } => {
                (self.cursor, self.anchor) = (source, None);
                self.begin(Phase::Copy { source: None });
            }
            Phase::Erase { slot: Some(_) } => {
                self.anchor = None;
                self.begin(Phase::Erase { slot: None });
            }
            _ => self.redraw(0),
        }
    }

    /// Back to the main page (`$87:8242`): the slot list repairs the SRAM
    /// again as it types the slots.
    fn redraw(&mut self, delay: u16) {
        let before = self.sram.clone();
        self.sram.repair();
        self.written |= self.sram != before;
        self.begin(Phase::Redraw { delay });
    }

    fn leave(&mut self, outcome: Outcome, fade: (u16, u16)) {
        self.begin(Phase::Leaving { outcome, fade });
    }

    /// What shows this frame.
    #[must_use]
    pub fn view(&self) -> View {
        let since = self.since;
        let page = |brightness, cursor, second, (lines, slots)| {
            View::Screen(Page {
                brightness,
                cursor,
                second,
                lines,
                slots,
            })
        };
        let full = (6, 3);
        let shown = Some(self.anchor.unwrap_or(self.shown));
        // A chosen slot's cursor blinks: off from two frames after the
        // choice, every other frame.
        let blink = |at: u8| (since < 2 || since % 2 == 1).then_some(at);
        match self.phase {
            Phase::Entry if self.frame < self.timing.fade_in => View::Blank,
            Phase::Entry => {
                let steps = (self.frame - self.timing.fade_in) / FADE_STEP + 1;
                page(level(steps), shown, None, full)
            }
            Phase::Main | Phase::Copy { source: None } | Phase::Erase { slot: None } => {
                page(15, shown, None, full)
            }
            Phase::Copy {
                source: Some(source),
            } => page(15, blink(source), (since >= 1).then_some(self.shown), full),
            Phase::Erase { slot: Some(slot) } => page(15, blink(slot), None, full),
            Phase::Redraw { delay } => {
                let typed = |at: &u16| u8::from(since >= delay + at);
                let text = if since < delay + CLEAR {
                    full
                } else {
                    (
                        LINES.iter().map(typed).sum(),
                        self.timing.slots.iter().map(typed).sum(),
                    )
                };
                page(15, shown, None, text)
            }
            Phase::Leaving { fade, outcome } if since < fade.1 => {
                let brightness = match outcome {
                    _ if since < fade.0 => 15,
                    Outcome::Load(_) => level(18 - since),
                    Outcome::NewGame(_) => level(14 - (since - fade.0) / FADE_STEP),
                };
                if brightness == 0 {
                    View::Blank
                } else {
                    page(brightness, shown, None, full)
                }
            }
            Phase::Leaving { .. } => View::Blank,
        }
    }
}

/// A brightness level, 15 at most.
fn level(steps: u16) -> u8 {
    u8::try_from(steps.min(15)).unwrap_or(15)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::SaveSlot;

    fn run(restart: &mut Restart, frames: u16) -> Option<Outcome> {
        (0..frames).find_map(|_| restart.step(None, false, false))
    }

    fn press(restart: &mut Restart, direction: Option<Direction>, confirm: bool, cancel: bool) {
        restart.step(direction, confirm, cancel);
        restart.step(None, false, false);
    }

    fn two_slots() -> Sram {
        let mut sram = Sram::default();
        sram.write_slot(0, &SaveSlot::default());
        sram.write_slot(2, &SaveSlot::default());
        sram
    }

    fn page(restart: &Restart) -> Page {
        match restart.view() {
            View::Screen(page) => page,
            View::Blank => panic!("blank"),
        }
    }

    #[test]
    fn the_screen_fades_in_and_takes_input_from_frame_218() {
        let mut restart = Restart::open(false, two_slots());
        assert_eq!(restart.cursor, 2, "`$1FFE` & 3: slot 3 was saved last");
        run(&mut restart, 160);
        assert_eq!(restart.view(), View::Blank);
        run(&mut restart, 1);
        assert_eq!(page(&restart).brightness, 1);
        run(&mut restart, 56);
        assert_eq!(page(&restart).brightness, 15, "frame 217");
        assert!(restart
            .take_cues()
            .iter()
            .any(|cue| matches!(cue, Cue::Track { .. })));
        // Down from slot 3 to New Game, then Erase Data, and no further.
        for _ in 0..4 {
            press(&mut restart, Some(Direction::Down), false, false);
        }
        assert_eq!(restart.cursor, LAST);
    }

    #[test]
    fn a_valid_slot_loads_and_an_empty_one_starts_a_new_game() {
        let mut restart = Restart::open(false, two_slots());
        run(&mut restart, 217);
        restart.step(None, true, false);
        assert_eq!(page(&restart).brightness, 15);
        assert_eq!(run(&mut restart, 17), None);
        assert_eq!(page(&restart).brightness, 1, "A+17");
        assert_eq!(run(&mut restart, 1), Some(Outcome::Load(2)), "A+18");
        let mut restart = Restart::open(false, two_slots());
        run(&mut restart, 217);
        press(&mut restart, Some(Direction::Up), false, false);
        restart.step(None, true, false);
        assert_eq!(
            run(&mut restart, 65),
            Some(Outcome::NewGame(Some(1))),
            "N+65"
        );
    }

    #[test]
    fn copy_and_erase_write_sram_and_type_the_page_again() {
        let mut restart = Restart::open(false, two_slots());
        run(&mut restart, 217);
        // Copy Data: slot 1 over slot 2.
        press(&mut restart, Some(Direction::Down), false, false);
        press(&mut restart, Some(Direction::Down), false, false);
        restart.step(None, true, false);
        assert_eq!(restart.cursor, 0);
        restart.step(None, true, false);
        restart.step(None, false, false);
        restart.step(None, false, false);
        assert_eq!(page(&restart).second, Some(0), "the second cursor");
        assert_eq!(page(&restart).cursor, None, "the source's blinks off");
        press(&mut restart, Some(Direction::Down), false, false);
        restart.step(None, true, false);
        assert!(restart.sram().valid(1));
        assert!(restart.take_sram_write());
        restart.step(None, false, false);
        assert_eq!(
            page(&restart).cursor,
            Some(0),
            "the main cursor stays on the source"
        );
        run(&mut restart, 3);
        assert_eq!(
            (page(&restart).lines, page(&restart).slots),
            (0, 0),
            "cleared"
        );
        run(&mut restart, 11);
        assert_eq!((page(&restart).lines, page(&restart).slots), (6, 1), "E+14");
        run(&mut restart, 10);
        assert_eq!(restart.phase, Phase::Main);
        // Erase Data: slot 1, confirmed.
        restart.cursor = LAST;
        restart.step(None, true, false);
        restart.step(None, true, false);
        run(&mut restart, 2);
        restart.step(None, true, false);
        assert!(!restart.sram().valid(0));
        assert!(restart.take_sram_write());
    }
}
