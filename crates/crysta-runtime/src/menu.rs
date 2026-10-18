//! Yomi's box, the hall ([notes](../../../docs/menu.md)): Select opens it
//! (`$85:A179`), the pad moves Yomi across nine hotspots, Select closes it.
//! Frame by frame from the press that opened it; the field waits meanwhile
//! (`$049A`). The doors' screens are not here yet.

use room_core::Direction;

/// The frames that differ between the ROMs, from the open or close press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Timing {
    /// The hall shows at once (`$85:A95A`).
    hall: u32,
    /// The field's first step of brightness on the way back.
    back: u32,
    /// The field runs again (`$049A` = 0).
    done: u32,
}

const JAPAN: Timing = Timing {
    hall: 61,
    back: 35,
    done: 57,
};
const EUROPE: Timing = Timing {
    hall: 58,
    back: 32,
    done: 54,
};

/// The field's fade out: its first step, then one a frame to black.
const FADE_START: u32 = 20;
/// The hall stays this long after the closing press, then goes black.
const CLOSE_BLANK: u32 = 11;
/// Yomi's flapping as the hall shows: one native capture per ROM pins it.
const FLAP_AT_HALL: u32 = 4;
/// The hotspot the box opens on: the item door.
pub const ITEM_DOOR: u8 = 7;
/// Frames after a move in which the label is redrawn and the pad is not
/// read: the window is empty from the second.
const REDRAW: u32 = 4;
/// A held direction moves again after this many frames, then this often.
const REPEAT_DELAY: u16 = 16;
const REPEAT_EVERY: u16 = 5;
/// The ease's end (`$87:CE36`): half a cosine; the table runs to it.
const LAST_PHASE: u16 = 0x80;

/// A hotspot (`$92:BCB9`): Yomi's place, and where Up, Down, Left and Right
/// go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hotspot {
    /// Yomi's place on the screen.
    pub at: (i16, i16),
    /// The hotspot each direction goes to: Up, Down, Left, Right.
    pub next: [Option<u8>; 4],
}

/// The hall's data from the ROM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hall {
    /// The nine hotspots.
    pub hotspots: Vec<Hotspot>,
    /// The ease's cosine bytes (`$81:F462`), phases 0 to `$80`.
    pub cosine: Vec<i8>,
}

impl Hall {
    /// Reads the hotspots (`$92:BCB9`, European `$92:BE67`) and the cosine
    /// table (`$81:F462`).
    #[must_use]
    pub fn from_rom(image: &[u8]) -> Option<Self> {
        let table = assets::layout::per_revision(image, 0x12_BCB9, 0x12_BE67);
        let word = |at: usize| Some(u16::from_le_bytes([*image.get(at)?, *image.get(at + 1)?]));
        let hotspots = (0..9)
            .map(|index| {
                let entry = table + index * 12;
                let at = (word(entry)?.cast_signed(), word(entry + 2)?.cast_signed());
                let mut next = [None; 4];
                for (side, slot) in next.iter_mut().enumerate() {
                    let pointer = usize::from(word(entry + 4 + side * 2)?);
                    let offset = pointer.checked_sub(table & 0xFFFF);
                    *slot = offset
                        .filter(|&offset| pointer != 0 && offset % 12 == 0 && offset / 12 < 9)
                        .and_then(|offset| u8::try_from(offset / 12).ok());
                }
                Some(Hotspot { at, next })
            })
            .collect::<Option<Vec<_>>>()?;
        let cosine = image
            .get(0x01_F462..=0x01_F462 + usize::from(LAST_PHASE))?
            .iter()
            .map(|&byte| byte.cast_signed())
            .collect();
        Some(Self { hotspots, cosine })
    }
}

/// Yomi's move to a hotspot (`$85:C662`, `$87:CDB5`): a half-cosine ease
/// about the midpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ease {
    middle: (i16, i16),
    half: (i16, i16),
    target: (i16, i16),
    phase: u16,
    step: u16,
}

impl Ease {
    fn new(from: (i16, i16), target: (i16, i16)) -> Self {
        // Half the distance, the midpoint from `from` (`$87:CDB5`: rounds
        // toward it).
        let middle = |from: i16, to: i16| from + (to - from).signum() * ((to - from).abs() >> 1);
        let middle = (middle(from.0, target.0), middle(from.1, target.1));
        let half = (from.0 - middle.0, from.1 - middle.1);
        // Fast for a short way (`$87:CDB5`).
        let step = if half.0.abs() + half.1.abs() < 24 {
            20
        } else {
            9
        };
        Self {
            middle,
            half,
            target,
            phase: 0,
            step,
        }
    }

    /// A frame: the next place, the target itself once the phase would
    /// reach the end, then `None`.
    fn advance(&mut self, cosine: &[i8]) -> Option<(i16, i16)> {
        if self.phase >= LAST_PHASE {
            return None;
        }
        self.phase += self.step;
        if self.phase >= LAST_PHASE {
            return Some(self.target);
        }
        let t = i32::from(cosine.get(usize::from(self.phase)).copied().unwrap_or(0));
        let at = |middle: i16, half: i16| {
            // Toward zero, as the native multiply of the magnitudes.
            let offset = i32::from(half) * t * 2 / 256;
            i16::try_from(i32::from(middle) + offset).unwrap_or(middle)
        };
        Some((
            at(self.middle.0, self.half.0),
            at(self.middle.1, self.half.1),
        ))
    }
}

/// What the host draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// The field, at this brightness (0–15).
    Field(u8),
    /// Forced blank.
    Blank,
    /// The hall.
    Hall(Page),
}

/// The hall's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    /// The hotspot (`$06A6`).
    pub hotspot: u8,
    /// Yomi's place on the screen.
    pub yomi: (i16, i16),
    /// The hotspot whose label shows; none while it is redrawn.
    pub label: Option<u8>,
    /// Yomi's flapping: frames since she was spawned while the hall
    /// loaded.
    pub flap: u32,
}

/// Where the box stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Opening,
    Hall,
    /// Closing since this frame.
    Closing(u32),
}

/// The box while it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    timing: Timing,
    hall: Hall,
    phase: Phase,
    /// Frames since the press that opened it.
    frame: u32,
    hotspot: u8,
    yomi: (i16, i16),
    ease: Option<Ease>,
    moved: Option<Move>,
}

/// The last move: its frame, the hotspot it left, its direction and how
/// long that direction has been held (`$045A`/`$045C`): it moves again only
/// after [`REPEAT_DELAY`], until released.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Move {
    at: u32,
    from: u8,
    /// The direction still held since the move, and for how long.
    held: Option<(Direction, u16)>,
}

impl Menu {
    /// Opens on the item door (`$06A6` = 7) at the Select press, with the
    /// hall's tables from `image`.
    #[must_use]
    pub fn open(image: &[u8]) -> Option<Self> {
        let hall = Hall::from_rom(image)?;
        let yomi = hall
            .hotspots
            .get(usize::from(ITEM_DOOR))
            .map_or((0, 0), |hotspot| hotspot.at);
        Some(Self {
            timing: assets::layout::per_revision(image, JAPAN, EUROPE),
            hall,
            phase: Phase::Opening,
            frame: 0,
            hotspot: ITEM_DOOR,
            yomi,
            ease: None,
            moved: None,
        })
    }

    /// A frame with the directions the pad holds (two on a diagonal) and
    /// whether Select was pressed. Returns whether the box has closed and
    /// the field runs again.
    pub fn step(&mut self, pad: [Option<Direction>; 2], select: bool) -> bool {
        self.frame += 1;
        match self.phase {
            Phase::Opening if self.frame >= self.timing.hall => self.phase = Phase::Hall,
            Phase::Opening => {}
            Phase::Hall => {
                if let Some(ease) = &mut self.ease {
                    match ease.advance(&self.hall.cosine) {
                        Some(at) => self.yomi = at,
                        None => self.ease = None,
                    }
                }
                if select && !self.redrawing() {
                    self.phase = Phase::Closing(self.frame);
                } else {
                    self.read_pad(pad);
                }
            }
            Phase::Closing(since) => return self.frame - since >= self.timing.done,
        }
        false
    }

    /// Whether the label is being redrawn after a move: the pad is not read.
    fn redrawing(&self) -> bool {
        self.moved
            .is_some_and(|moved| self.frame - moved.at < REDRAW)
    }

    /// `$85:CB2A`: Up before Down before Left before Right, of the
    /// directions `pad` holds (a diagonal holds two); the one that just
    /// moved counts as released until [`REPEAT_DELAY`] frames, then every
    /// [`REPEAT_EVERY`] (`$045A`/`$045C`).
    fn read_pad(&mut self, pad: [Option<Direction>; 2]) {
        let held = |direction| pad.contains(&Some(direction));
        // Counted while held; a release ends the wait.
        if let Some(moved) = &mut self.moved {
            moved.held = moved
                .held
                .filter(|&(direction, _)| held(direction))
                .map(|(direction, frames)| (direction, frames + 1));
        }
        if self.redrawing() {
            return;
        }
        let masked = |direction| match self.moved.and_then(|moved| moved.held) {
            Some((moved, frames)) if moved == direction => {
                frames < REPEAT_DELAY || !(frames - REPEAT_DELAY).is_multiple_of(REPEAT_EVERY)
            }
            _ => false,
        };
        let Some(direction) = [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ]
        .into_iter()
        .find(|&direction| held(direction) && !masked(direction)) else {
            return;
        };
        let held = match self.moved.and_then(|moved| moved.held) {
            Some((moved, frames)) if moved == direction => frames,
            _ => 0,
        };
        let side = match direction {
            Direction::Up => 0,
            Direction::Down => 1,
            Direction::Left => 2,
            Direction::Right => 3,
        };
        let Some(next) = self
            .hall
            .hotspots
            .get(usize::from(self.hotspot))
            .and_then(|hotspot| hotspot.next[side])
        else {
            return;
        };
        self.moved = Some(Move {
            at: self.frame,
            from: self.hotspot,
            held: Some((direction, held)),
        });
        self.hotspot = next;
        if let Some(hotspot) = self.hall.hotspots.get(usize::from(next)) {
            self.ease = Some(Ease::new(self.yomi, hotspot.at));
        }
    }

    /// What the host draws this frame.
    #[must_use]
    pub fn view(&self) -> View {
        match self.phase {
            Phase::Opening => {
                if self.frame < FADE_START {
                    View::Field(15)
                } else if self.frame < FADE_START + 15 {
                    View::Field(u8::try_from(FADE_START + 14 - self.frame).unwrap_or(0))
                } else {
                    View::Blank
                }
            }
            Phase::Hall => View::Hall(self.page()),
            Phase::Closing(since) => {
                let after = self.frame - since;
                if after < CLOSE_BLANK {
                    View::Hall(self.page())
                } else if after < self.timing.back {
                    View::Blank
                } else {
                    View::Field(u8::try_from((after - self.timing.back + 1).min(15)).unwrap_or(15))
                }
            }
        }
    }

    fn page(&self) -> Page {
        // The old label to the first frame after a move, none to the third,
        // the new one from the fourth.
        let label = match self.moved.map(|moved| (self.frame - moved.at, moved.from)) {
            Some((0..2, from)) => Some(from),
            Some((2..REDRAW, _)) => None,
            _ => Some(self.hotspot),
        };
        Page {
            hotspot: self.hotspot,
            yomi: self.yomi,
            label,
            flap: self.frame.saturating_sub(self.timing.hall) + FLAP_AT_HALL,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_midpoint_rounds_toward_the_start() {
        // `$87:CDB5`: half of |d| from the start.
        let ease = Ease::new((5, 129), (0, 104));
        assert_eq!((ease.middle, ease.half), ((3, 117), (2, 12)));
        assert_eq!(ease.step, 20, "|2| + |12| is under 24");
    }
}
