//! Yomi's box ([notes](../../../docs/menu.md)): Select opens it
//! (`$85:A179`) on the hall, the pad moves Yomi across its hotspots, the
//! weapon and armor doors lead to their rooms, whose pedestals equip,
//! Select closes it. Frame by frame from the press that opened it; the
//! field waits meanwhile (`$049A`). The throw and the other screens are not
//! here yet.

use crate::scene::{Dialogue, Presses};
use assets::text::{DialoguePage, HouseDialogue};
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

/// A door's frames from the A press (`docs/menu.md`, "Transition").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DoorTiming {
    /// The first and last black frames.
    dark: (u32, u32),
    /// Yomi at the target; the window starts opening.
    yomi: u32,
    /// The label in a room.
    label: u32,
    /// The label back in the hall (European not measured: 8 frames sooner,
    /// as its room's).
    hall_label: u32,
}

const JAPAN_DOOR: DoorTiming = DoorTiming {
    dark: (62, 114),
    yomi: 99,
    label: 171,
    hall_label: 162,
};
const EUROPE_DOOR: DoorTiming = DoorTiming {
    dark: (61, 108),
    yomi: 93,
    label: 163,
    hall_label: 154,
};

/// The window closes on the door from this frame over [`WIPE`] frames.
const WIPE_START: u32 = 2;
const WIPE: u32 = 64;
/// The new screen loads after the dark begins: the room's slots are
/// sorted then (P+68 in Japanese; European not measured, the same
/// distance assumed).
const LOAD: u32 = 6;

/// The field's fade out: its first step, then one a frame to black.
const FADE_START: u32 = 20;
/// The hall stays this long after the closing press, then goes black.
const CLOSE_BLANK: u32 = 11;
/// Yomi's flapping as the hall shows, and as a door spawns her again
/// (P+99): the hall's captures (JP, EU) and the JP weapon room's pin it.
const FLAP_AT_SPAWN: u32 = 4;
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

/// A screen of the box (`$06BC`): its index in the position tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Screen {
    /// The hall, nine hotspots.
    Hall,
    /// The armor room, twenty positions.
    Armor,
    /// The weapon room, twenty positions.
    Weapon,
}

impl Screen {
    /// `$06BC`, the screen's position count, and its place in [`Hall`].
    const fn table(self) -> (usize, usize, usize) {
        match self {
            Self::Hall => (0, 9, 0),
            Self::Armor => (3, 20, 1),
            Self::Weapon => (4, 20, 2),
        }
    }

    /// A room's sixteen slots, as indices of the inventory's (`$7F:8040`,
    /// `$7F:8060`): four fixed ones, then the twelve shared.
    #[must_use]
    pub const fn slots(self) -> Option<std::ops::Range<usize>> {
        match self {
            Self::Hall => None,
            Self::Weapon => Some(0x20..0x30),
            Self::Armor => Some(0x30..0x40),
        }
    }

    /// Where slot `k`'s icon shows (`$85:CE9E`): the layer-2 cell (column,
    /// row) of metatile `$1E0 + k`, on the hall's map.
    #[must_use]
    pub fn icon_cell(self, slot: usize) -> Option<(u8, u8)> {
        const WEAPON: [(u8, u8); 16] = [
            (7, 23),
            (9, 23),
            (7, 25),
            (9, 25),
            (4, 22),
            (6, 22),
            (10, 22),
            (12, 22),
            (3, 23),
            (13, 23),
            (3, 25),
            (13, 25),
            (4, 26),
            (6, 26),
            (10, 26),
            (12, 26),
        ];
        const ARMOR: [(u8, u8); 16] = [
            (22, 7),
            (26, 7),
            (22, 9),
            (26, 9),
            (20, 6),
            (23, 6),
            (25, 6),
            (28, 6),
            (19, 7),
            (29, 7),
            (19, 9),
            (29, 9),
            (20, 10),
            (23, 10),
            (25, 10),
            (28, 10),
        ];
        match self {
            Self::Hall => None,
            Self::Weapon => WEAPON.get(slot).copied(),
            Self::Armor => ARMOR.get(slot).copied(),
        }
    }

    /// The slots entering sorts: the twelve shared ones (a lone weapon in
    /// `$7F:8048` stays there, natively).
    #[must_use]
    pub const fn sorted(self) -> Option<std::ops::Range<usize>> {
        match self {
            Self::Hall => None,
            Self::Weapon => Some(0x24..0x30),
            Self::Armor => Some(0x34..0x40),
        }
    }

    /// Where A on position `at` leads: the screen and its cursor
    /// (`$85:CD04`, `$85:CC81`, `$85:CC49`, `$85:CC65`). The item room is
    /// not here yet.
    const fn door(self, at: u8) -> Option<(Self, u8)> {
        match (self, at) {
            (Self::Hall, 6) => Some((Self::Weapon, 0)),
            (Self::Hall, 8) => Some((Self::Armor, 2)),
            (Self::Weapon, 0) => Some((Self::Hall, 6)),
            (Self::Weapon, 1) => Some((Self::Armor, 1)),
            (Self::Armor, 1) => Some((Self::Weapon, 1)),
            (Self::Armor, 2) => Some((Self::Hall, 8)),
            _ => None,
        }
    }

    /// The camera on the hall's 512×512 map (`docs/menu.md`).
    #[must_use]
    pub const fn camera(self) -> (i16, i16) {
        match self {
            Self::Hall => (0, 0),
            Self::Armor => (256, 0),
            Self::Weapon => (0, 256),
        }
    }

    /// The room's door B moves Yomi to: the hall's.
    const fn hall_door(self) -> Option<u8> {
        match self {
            Self::Hall => None,
            Self::Weapon => Some(0),
            Self::Armor => Some(2),
        }
    }
}

/// What the world does for the box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The box has closed; the field runs again.
    Closed,
    /// Entering the room: sort its shared slots by power, highest first
    /// (`$85:A6FD`, `$85:A663`).
    Sort(Screen),
    /// The room's equipment is now `item`, or none (`$064A`, `$064C`).
    Equip(Screen, Option<u8>),
    /// The stats follow the equipment (`$85:F4BD`): the powers.
    Stats,
}

/// What the box reads of the world: a room's slots and the equipment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Held<'s> {
    /// The inventory's (item, count) slots from `$7F:8000`, as bytes.
    pub slots: &'s [u8],
    /// The equipped weapon.
    pub weapon: Option<u8>,
    /// The equipped armor.
    pub armor: Option<u8>,
}

impl Held<'_> {
    /// The item and count on a room's position `at` (slot `at - 4`).
    fn on(&self, screen: Screen, at: u8) -> Option<(u8, u8)> {
        let slot = screen.slots()?.start + usize::from(at.checked_sub(4)?);
        let pair = self.slots.get(slot * 2..slot * 2 + 2)?;
        (pair[0] != 0).then_some((pair[0], pair[1]))
    }

    /// The room's equipment.
    const fn equipped(&self, screen: Screen) -> Option<u8> {
        match screen {
            Screen::Weapon => self.weapon,
            Screen::Armor => self.armor,
            Screen::Hall => None,
        }
    }
}

/// The texts of a pedestal's A (`docs/menu.md`, "Labels and texts"),
/// Japanese and European.
const EMPTY: (u32, u32) = (0x92_B6C5, 0x92_B8BC);
const EQUIP_ASK: (u32, u32) = (0x92_AB47, 0x92_ADE5);
const EQUIPPED: (u32, u32) = (0x92_AB84, 0x92_AE22);
const UNEQUIP_ASK: (u32, u32) = (0x92_AB1D, 0x92_ADC5);
const UNEQUIPPED: (u32, u32) = (0x92_AB6D, 0x92_AE04);
/// The question's choice (`COP 1A 0A`): the first answer equips or
/// unequips, B and the second close the window.
const ANSWERS: u8 = 0x0A;
/// A text shows from the press's second frame (`COP 00`, then `COP 1C`).
const TEXT_AT: u32 = 2;
/// The empty pedestal's text goes at the press's 21st frame, its label at
/// the 22nd.
const EMPTY_GONE: u32 = 21;
/// After a question's window closes: an empty label window, then the
/// label at the fourth frame.
const LABEL_AFTER: u32 = 4;

/// A pedestal's A under way (`$85:AE8E`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Task {
    /// Nothing on it: its text, from this press.
    Empty(u32),
    /// The question about `item` (`count` of it): to equip it, or to take
    /// it off.
    Asking {
        since: u32,
        item: (u8, u8),
        unequip: bool,
    },
    /// The first answer from this frame: the equipment changes on the
    /// next, the answer's text on the one after; typed out, the window
    /// closes itself and empties a frame later.
    Answering {
        since: u32,
        item: (u8, u8),
        unequip: bool,
    },
    /// The window emptied: the stats.
    Closed,
}

/// A hotspot (`$92:BCA3[$06BC]`): Yomi's place, and where Up, Down, Left
/// and Right go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hotspot {
    /// Yomi's place on the hall's map.
    pub at: (i16, i16),
    /// The hotspot each direction goes to: Up, Down, Left, Right.
    pub next: [Option<u8>; 4],
}

/// The box's tables from the ROM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hall {
    /// The hall's hotspots, then the armor room's and the weapon room's
    /// positions.
    screens: [Vec<Hotspot>; 3],
    /// The ease's cosine bytes (`$81:F462`), phases 0 to `$80`.
    pub cosine: Vec<i8>,
}

impl Hall {
    /// Reads the positions (`$92:BCA3`, European `$92:BE51`, a table per
    /// screen) and the cosine table (`$81:F462`).
    #[must_use]
    pub fn from_rom(image: &[u8]) -> Option<Self> {
        let tables = assets::layout::per_revision(image, 0x12_BCA3, 0x12_BE51);
        let word = |at: usize| Some(u16::from_le_bytes([*image.get(at)?, *image.get(at + 1)?]));
        let screen = |screen: Screen| {
            let (index, count, _) = screen.table();
            let table = (tables & 0xFF_0000) | usize::from(word(tables + index * 2)?);
            (0..count)
                .map(|entry| {
                    let entry = table + entry * 12;
                    let at = (word(entry)?.cast_signed(), word(entry + 2)?.cast_signed());
                    let mut next = [None; 4];
                    for (side, slot) in next.iter_mut().enumerate() {
                        let pointer = usize::from(word(entry + 4 + side * 2)?);
                        let offset = pointer.checked_sub(table & 0xFFFF);
                        *slot = offset
                            .filter(|&offset| {
                                pointer != 0 && offset % 12 == 0 && offset / 12 < count
                            })
                            .and_then(|offset| u8::try_from(offset / 12).ok());
                    }
                    Some(Hotspot { at, next })
                })
                .collect::<Option<Vec<_>>>()
        };
        let screens = [
            screen(Screen::Hall)?,
            screen(Screen::Armor)?,
            screen(Screen::Weapon)?,
        ];
        let cosine = image
            .get(0x01_F462..=0x01_F462 + usize::from(LAST_PHASE))?
            .iter()
            .map(|&byte| byte.cast_signed())
            .collect();
        Some(Self { screens, cosine })
    }

    /// A screen's hotspots or positions.
    #[must_use]
    pub fn hotspots(&self, screen: Screen) -> &[Hotspot] {
        &self.screens[screen.table().2]
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
    /// A screen of the box.
    Screen(Page),
}

/// A screen's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    /// The screen shown.
    pub screen: Screen,
    /// The hotspot or position (`$06A6`, `$06AA`, `$06AC`).
    pub hotspot: u8,
    /// Yomi's place on the screen.
    pub yomi: (i16, i16),
    /// The hotspot whose label shows; none while it is redrawn.
    pub label: Option<u8>,
    /// Yomi's flapping: frames since she was spawned while the hall
    /// loaded.
    pub flap: u32,
    /// A door's window closing on the door or opening on the new screen,
    /// and its frame.
    pub wipe: Option<(Wipe, u32)>,
}

/// A door's rotating window (`$85:C725`, `$85:C7F3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wipe {
    /// Closing on the door it was opened by (0, 1 or 2).
    Closing(u8),
    /// Opening from that door's place on the new screen.
    Opening(u8),
}

/// Where the box stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Opening,
    Screen,
    /// Through a door since this frame.
    Door(Door),
    /// Closing since this frame.
    Closing(u32),
}

/// A door under way: to which screen and cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Door {
    since: u32,
    /// The door's place: 0, 1 or 2 (`$7F:201C`).
    place: u8,
    to: Screen,
    cursor: u8,
}

/// The box while it runs.
#[derive(Debug, Clone)]
pub struct Menu<'a> {
    image: &'a [u8],
    timing: Timing,
    doors: DoorTiming,
    hall: Hall,
    screen: Screen,
    phase: Phase,
    /// Frames since the press that opened it.
    frame: u32,
    hotspot: u8,
    yomi: (i16, i16),
    ease: Option<Ease>,
    moved: Option<Move>,
    /// A pedestal's A under way, and its window.
    task: Option<Task>,
    dialogue: Dialogue,
    /// After a window closed, the label and the pad wait for this frame.
    label_at: u32,
    /// Yomi's flapping starts here: she is spawned as the hall loads, and
    /// again through a door.
    spawned: u32,
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

impl<'a> Menu<'a> {
    /// Opens on the item door (`$06A6` = 7) at the Select press, with the
    /// hall's tables and texts from `image`.
    #[must_use]
    pub fn open(image: &'a [u8]) -> Option<Self> {
        let hall = Hall::from_rom(image)?;
        let yomi = hall
            .hotspots(Screen::Hall)
            .get(usize::from(ITEM_DOOR))
            .map_or((0, 0), |hotspot| hotspot.at);
        let timing = assets::layout::per_revision(image, JAPAN, EUROPE);
        Some(Self {
            image,
            timing,
            doors: assets::layout::per_revision(image, JAPAN_DOOR, EUROPE_DOOR),
            hall,
            screen: Screen::Hall,
            phase: Phase::Opening,
            frame: 0,
            hotspot: ITEM_DOOR,
            yomi,
            ease: None,
            moved: None,
            task: None,
            dialogue: Dialogue::default(),
            label_at: 0,
            // Spawned while the hall loads.
            spawned: timing.hall - FLAP_AT_SPAWN,
        })
    }

    /// A frame with the directions the pad holds (two on a diagonal), the
    /// buttons pressed and what the box reads of the world: A takes a door
    /// or asks about a pedestal, B goes to a room's hall door, Select
    /// closes the box. Returns what the world does.
    pub fn step(
        &mut self,
        pad: [Option<Direction>; 2],
        presses: Presses,
        held: &Held<'_>,
    ) -> Option<Event> {
        self.frame += 1;
        match self.phase {
            Phase::Opening if self.frame >= self.timing.hall => self.phase = Phase::Screen,
            Phase::Opening => {}
            Phase::Screen => {
                self.ease_yomi();
                if let Some(task) = self.task {
                    return self.task_frame(task, presses);
                }
                if self.frame < self.label_at {
                    return None;
                }
                if self.redrawing() {
                    self.read_pad(pad);
                } else if presses.select {
                    self.phase = Phase::Closing(self.frame);
                } else if presses.confirm && self.hotspot >= 4 && self.screen != Screen::Hall {
                    self.ask(held);
                } else if presses.confirm {
                    self.take_door();
                } else if let Some(door) = presses
                    .cancel
                    .then(|| self.screen.hall_door())
                    .flatten()
                    .filter(|&door| door != self.hotspot)
                {
                    self.go(door, None);
                } else {
                    self.read_pad(pad);
                }
            }
            Phase::Door(door) => return self.door_frame(door),
            Phase::Closing(since) => {
                return (self.frame - since >= self.timing.done).then_some(Event::Closed);
            }
        }
        None
    }

    /// A on a pedestal (`$85:AE8E`): empty, or the question about its item.
    fn ask(&mut self, held: &Held<'_>) {
        self.task = Some(match held.on(self.screen, self.hotspot) {
            None => Task::Empty(self.frame),
            Some(item) => Task::Asking {
                since: self.frame,
                item,
                unequip: held.equipped(self.screen) == Some(item.0),
            },
        });
    }

    /// A frame of a pedestal's A: its texts in the window, the choice, the
    /// equipment changed; the stats when its window closes (`$85:F4BD`,
    /// after every A on a pedestal).
    fn task_frame(&mut self, task: Task, presses: Presses) -> Option<Event> {
        let before = self.dialogue.busy();
        self.dialogue.tick();
        let chosen = self.dialogue.press(presses);
        match task {
            Task::Empty(since) => match self.frame - since {
                TEXT_AT if !self.request(EMPTY, (0, 0)) => Some(self.close_window(1)),
                EMPTY_GONE => Some(self.close_window(1)),
                _ => None,
            },
            Task::Asking {
                since,
                item,
                unequip,
            } => {
                let after = self.frame - since;
                let question = if unequip { UNEQUIP_ASK } else { EQUIP_ASK };
                if after == TEXT_AT && !self.request(question, item) {
                    return Some(self.close_window(LABEL_AFTER));
                }
                if after > TEXT_AT && !self.dialogue.busy() && chosen.is_none() {
                    // The question typed out: its answers (`COP 1A 0A`).
                    let Ok(choice) = HouseDialogue::choice_at(self.image, ANSWERS) else {
                        return Some(self.close_window(LABEL_AFTER));
                    };
                    self.dialogue.ask(choice);
                }
                match chosen {
                    Some(1) => {
                        self.task = Some(Task::Answering {
                            since: self.frame,
                            item,
                            unequip,
                        });
                        None
                    }
                    Some(_) => Some(self.close_window(LABEL_AFTER)),
                    None => None,
                }
            }
            Task::Answering {
                since,
                item,
                unequip,
            } => match self.frame - since {
                1 => Some(Event::Equip(self.screen, (!unequip).then_some(item.0))),
                TEXT_AT => {
                    let answer = if unequip { UNEQUIPPED } else { EQUIPPED };
                    if !self.request(answer, item) {
                        self.task = Some(Task::Closed);
                    }
                    None
                }
                // The text closes its window when typed out (`$D7`); the
                // stats a frame later.
                _ => {
                    if before && !self.dialogue.busy() {
                        self.task = Some(Task::Closed);
                    }
                    None
                }
            },
            Task::Closed => Some(self.close_window(LABEL_AFTER)),
        }
    }

    /// Publishes the text at `source` (Japanese, European) about `item`
    /// and its count; whether it decoded.
    fn request(&mut self, source: (u32, u32), (item, count): (u8, u8)) -> bool {
        let source = assets::layout::per_revision(self.image, source.0, source.1);
        let read = |address| match address {
            0x0DD0 => Some(item),
            0x0DD2 => Some(count),
            0x0DD1 | 0x0DD3 => Some(0),
            _ => None,
        };
        HouseDialogue::decode_reading(self.image, source, read)
            .map(|pages| self.dialogue.request(pages))
            .is_ok()
    }

    /// The window closed: the label comes back `after` frames on; the stats
    /// follow the equipment.
    fn close_window(&mut self, after: u32) -> Event {
        self.task = None;
        self.dialogue = Dialogue::default();
        self.label_at = self.frame + after;
        Event::Stats
    }

    /// The box's text window, while a pedestal's A shows one.
    #[must_use]
    pub fn dialogue(&self) -> Option<crate::scene::View<'_, DialoguePage>> {
        self.dialogue.view()
    }

    /// A on a door: the window closes on it (`$85:CD43`).
    fn take_door(&mut self) {
        let Some((to, cursor)) = self.screen.door(self.hotspot) else {
            return;
        };
        // The door's place: the hall's are its hotspots 6..8, a room's
        // its positions 0..2.
        let place = if self.screen == Screen::Hall {
            self.hotspot - 6
        } else {
            self.hotspot
        };
        self.moved = None;
        self.phase = Phase::Door(Door {
            since: self.frame,
            place,
            to,
            cursor,
        });
    }

    /// A frame of a door: the new screen loads in the dark, Yomi is placed,
    /// the label shows; then the pad again.
    fn door_frame(&mut self, door: Door) -> Option<Event> {
        let after = self.frame - door.since;
        let load = self.doors.dark.0 + LOAD;
        // A move under way goes on to the load.
        self.ease_yomi();
        if after == self.doors.yomi {
            self.spawned = self.frame - FLAP_AT_SPAWN;
        }
        if after == load {
            self.ease = None;
            self.screen = door.to;
            self.hotspot = door.cursor;
            self.yomi = self
                .hall
                .hotspots(door.to)
                .get(usize::from(door.cursor))
                .map_or(self.yomi, |hotspot| hotspot.at);
        }
        if after >= self.label_frame(door) {
            self.phase = Phase::Screen;
        }
        (after == load && door.to != Screen::Hall).then_some(Event::Sort(door.to))
    }

    /// The frame a door's label shows, from its press.
    const fn label_frame(&self, door: Door) -> u32 {
        if matches!(door.to, Screen::Hall) {
            self.doors.hall_label
        } else {
            self.doors.label
        }
    }

    /// Yomi's ease, a frame.
    fn ease_yomi(&mut self) {
        if let Some(ease) = &mut self.ease {
            match ease.advance(&self.hall.cosine) {
                Some(at) => self.yomi = at,
                None => self.ease = None,
            }
        }
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
            .hotspots(self.screen)
            .get(usize::from(self.hotspot))
            .and_then(|hotspot| hotspot.next[side])
        else {
            return;
        };
        self.go(next, Some((direction, held)));
    }

    /// Yomi to `next` (`$85:CC08`), the label redrawn.
    fn go(&mut self, next: u8, held: Option<(Direction, u16)>) {
        self.moved = Some(Move {
            at: self.frame,
            from: self.hotspot,
            held,
        });
        self.hotspot = next;
        if let Some(hotspot) = self.hall.hotspots(self.screen).get(usize::from(next)) {
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
            Phase::Screen => View::Screen(self.page()),
            Phase::Door(door) => self.door_view(door),
            Phase::Closing(since) => {
                let after = self.frame - since;
                if after < CLOSE_BLANK {
                    View::Screen(self.page())
                } else if after < self.timing.back {
                    View::Blank
                } else {
                    View::Field(u8::try_from((after - self.timing.back + 1).min(15)).unwrap_or(15))
                }
            }
        }
    }

    /// A door: the window closing on the old screen, the dark, the window
    /// opening on the new one with Yomi from her frame, the label after.
    fn door_view(&self, door: Door) -> View {
        let after = self.frame - door.since;
        let (first, last) = self.doors.dark;
        if (first..=last).contains(&after) {
            return View::Blank;
        }
        let page = self.page();
        // The old screen until the dark, the new one after it; the window
        // done (JP P+165) is not separate from the label yet.
        let wipe = if after < first {
            let closing = after.saturating_sub(WIPE_START).min(WIPE);
            (Wipe::Closing(door.place), closing)
        } else {
            (Wipe::Opening(door.place), after - self.doors.yomi)
        };
        // The label goes at the press's second frame.
        let label = (after < WIPE_START).then_some(page.label).flatten();
        View::Screen(Page {
            label,
            wipe: Some(wipe),
            ..page
        })
    }

    fn page(&self) -> Page {
        let camera = self.screen.camera();
        // The old label to the first frame after a move, none to the third,
        // the new one from the fourth.
        let label = match self.moved.map(|moved| (self.frame - moved.at, moved.from)) {
            _ if self.task.is_some() || self.frame < self.label_at => None,
            Some((0..2, from)) => Some(from),
            Some((2..REDRAW, _)) => None,
            _ => Some(self.hotspot),
        };
        Page {
            screen: self.screen,
            hotspot: self.hotspot,
            yomi: (self.yomi.0 - camera.0, self.yomi.1 - camera.1),
            label,
            flap: self.frame.saturating_sub(self.spawned),
            wipe: None,
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
