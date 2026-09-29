//! The Records screen ([notes](../../../docs/records-screen.md)): the save
//! screen the bedroom desk opens, its text requests and its art.

use crate::layout::Address;
use crate::text::{DialoguePage, HouseDialogue, Request, TextError};

/// The page: ` 1`–` 3`, the current game's header and the question.
const PAGE: Address = Address::both(0x92_CB4D, 0x92_E1F3);
/// The page after a save: ` 1`–` 3` and "`n` saved".
const SAVED: Address = Address::both(0x92_CB77, 0x92_E21E);
/// Slot `n`'s name (`+12n`), from `$061C`.
const FILLED: Address = Address::both(0x92_CBA0, 0x92_E244);
/// Slot `n`'s "No Data" (`+12n`).
const EMPTY: Address = Address::both(0x92_CBC4, 0x92_E268);
/// The current game's name, from `$0610`.
const CURRENT: Address = Address::both(0x92_CBE8, 0x92_E28C);
/// Where `$87:CB4B` copies a slot's name, and the current name.
const SLOT_NAME: u16 = 0x061C;
const CURRENT_NAME: u16 = 0x0610;
/// The saved page's number: the slot plus 1.
const SAVED_NUMBER: u16 = 0x04C6;
/// Bytes a name may take, its `D4` end included.
const NAME_BYTES: u16 = 12;
/// The byte that ends a name.
const NAME_END: u8 = 0xD4;

/// What the screen shows of a valid slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot<'a> {
    /// The name's glyph codes, without its `D4` end.
    pub name: &'a [u8],
}

fn address(image: &[u8], address: Address) -> Result<u32, TextError> {
    address.of(image).ok_or(TextError {
        source: 0,
        reason: "no Records text in this revision",
    })
}

/// A name at `base` in WRAM, as the engine reads it: its codes, then `D4`.
fn name_at(base: u16, name: &[u8]) -> impl Fn(u16) -> Option<u8> + '_ {
    move |at| {
        let index = usize::from(at.checked_sub(base).filter(|&index| index < NAME_BYTES)?);
        Some(name.get(index).copied().unwrap_or(NAME_END))
    }
}

/// The slot list's requests (`$87:CB4B`): each slot's name, or "No Data",
/// with the readers of the names (`$061C`).
fn slot_requests<'a>(
    image: &[u8],
    slots: &[Option<Slot<'_>>; 3],
    readers: &'a [impl Fn(u16) -> Option<u8>; 3],
) -> Result<Vec<Request<'a>>, TextError> {
    let (filled, empty) = (address(image, FILLED)?, address(image, EMPTY)?);
    Ok(slots
        .iter()
        .zip(readers)
        .zip(0u32..)
        .map(|((slot, read), n)| {
            let base = if slot.is_some() { filled } else { empty };
            (base + 12 * n, read as &dyn Fn(u16) -> Option<u8>)
        })
        .collect())
}

/// A slot's name as `$87:CB4B` copies it; an empty slot's script reads none.
fn slot_names<'a>(slots: &[Option<Slot<'a>>; 3]) -> [impl Fn(u16) -> Option<u8> + 'a; 3] {
    slots.map(|slot| name_at(SLOT_NAME, slot.map_or(&[][..], |slot| slot.name)))
}

/// The screen as it opens: the page, the three slots, the current game
/// named `current`; one page per request, positions relative to the text
/// area at (24, 88).
///
/// # Errors
/// A script that does not decode.
pub fn entry_text(
    image: &[u8],
    slots: &[Option<Slot<'_>>; 3],
    current: &[u8],
) -> Result<Vec<DialoguePage>, TextError> {
    let none = |_| None;
    let current_name = name_at(CURRENT_NAME, current);
    let readers = slot_names(slots);
    let mut sources: Vec<Request<'_>> = vec![(address(image, PAGE)?, &none)];
    sources.extend(slot_requests(image, slots, &readers)?);
    sources.push((address(image, CURRENT)?, &current_name));
    HouseDialogue::decode_requests(image, &sources)
}

/// The screen after saving to slot `saved` (0–2): the saved page, then the
/// slots again.
///
/// # Errors
/// A script that does not decode.
pub fn saved_text(
    image: &[u8],
    saved: u8,
    slots: &[Option<Slot<'_>>; 3],
) -> Result<Vec<DialoguePage>, TextError> {
    let number = move |at: u16| match at {
        SAVED_NUMBER => Some(saved.min(2) + 1),
        _ if at == SAVED_NUMBER + 1 => Some(0),
        _ => None,
    };
    let readers = slot_names(slots);
    let mut sources: Vec<Request<'_>> = vec![(address(image, SAVED)?, &number)];
    sources.extend(slot_requests(image, slots, &readers)?);
    HouseDialogue::decode_requests(image, &sources)
}
