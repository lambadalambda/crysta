//! The native save slot ([saves](../../../docs/saves.md)): WRAM
//! `$7E:0600–07FF` and `$7F:8000–82F9`, 1274 bytes, as `$8D:A6FB` copies
//! them. The world writes what it models into a slot and reads it back on
//! a load; every other byte passes through as it was loaded, so a round trip
//! loses nothing.

use crate::inventory::{bcd, Inventory};

/// Bytes of one slot's data, before its checksum words.
pub const SLOT_BYTES: usize = 0x4FA;
/// Where each field sits in the slot (WRAM `$0600` is offset 0).
const MAP: usize = 0x00;
const FACING: usize = 0x02;
const POSITION: usize = 0x04;
const NAME: usize = 0x10;
/// The name's bytes, its `D4` end included.
const NAME_BYTES: usize = 0x0C;
const SECONDS: usize = 0x2E;
const LEVEL: usize = 0x56;
const MONEY: usize = 0x94;
const EVENTS: usize = 0xC0;
/// The event flags the slot keeps: `$7E:06C0–07FF`.
pub const EVENT_BYTES: usize = 0x200 - EVENTS;
const PRIME_BLUE: usize = 0x1ED;
const ITEMS: usize = 0x200;
/// The item slots' bytes, `$7F:8000–80FF`.
const ITEM_BYTES: usize = 0x100;

/// One native save slot's data.
#[derive(Clone, PartialEq, Eq)]
pub struct SaveSlot {
    bytes: Box<[u8; SLOT_BYTES]>,
}

impl std::fmt::Debug for SaveSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SaveSlot")
            .field("map", &self.map())
            .field("position", &self.position())
            .field("facing", &self.facing())
            .finish_non_exhaustive()
    }
}

impl Default for SaveSlot {
    fn default() -> Self {
        Self {
            bytes: Box::new([0; SLOT_BYTES]),
        }
    }
}

impl SaveSlot {
    /// A slot from its bytes, as SRAM holds them.
    #[must_use]
    pub fn from_bytes(bytes: &[u8; SLOT_BYTES]) -> Self {
        Self {
            bytes: Box::new(*bytes),
        }
    }

    /// The slot's bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8; SLOT_BYTES] {
        &self.bytes
    }

    fn word(&self, at: usize) -> u16 {
        u16::from_le_bytes([self.bytes[at], self.bytes[at + 1]])
    }

    fn set_word(&mut self, at: usize, value: u16) {
        self.bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }

    /// The map (`$047E` at the save).
    #[must_use]
    pub fn map(&self) -> u16 {
        self.word(MAP)
    }

    /// The facing (`$0956`): 0 Down, 1 Up, 2 Left, 3 Right.
    #[must_use]
    pub fn facing(&self) -> u16 {
        self.word(FACING)
    }

    /// The player's position as the load places it: the saved corner
    /// (`$0952`/`$0954`) plus (8,16).
    #[must_use]
    pub fn position(&self) -> (u16, u16) {
        (
            self.word(POSITION).wrapping_add(8),
            self.word(POSITION + 2).wrapping_add(16),
        )
    }

    /// Writes where the player stands: map, facing, position.
    pub fn set_place(&mut self, map: u16, facing: u16, (x, y): (u16, u16)) {
        self.set_word(MAP, map);
        self.set_word(FACING, facing);
        self.set_word(POSITION, x.wrapping_sub(8));
        self.set_word(POSITION + 2, y.wrapping_sub(16));
    }

    /// The name's glyph codes, up to its `D4` end.
    #[must_use]
    pub fn name(&self) -> &[u8] {
        let name = &self.bytes[NAME..NAME + NAME_BYTES];
        let end = name
            .iter()
            .position(|&code| code == NAME_END)
            .unwrap_or(NAME_BYTES);
        &name[..end]
    }

    /// The level (`$0656`).
    #[must_use]
    pub fn level(&self) -> u8 {
        self.bytes[LEVEL]
    }

    /// The play time in seconds (`$062E`).
    #[must_use]
    pub fn seconds(&self) -> u32 {
        u32::from(self.word(SECONDS)) | u32::from(self.word(SECONDS + 2)) << 16
    }

    /// The event flags `$7E:06C0–07FF`.
    #[must_use]
    pub fn events(&self) -> &[u8] {
        &self.bytes[EVENTS..EVENTS + EVENT_BYTES]
    }

    /// Writes the flags the slot keeps from a bitmap.
    pub fn set_events(&mut self, events: &[u8]) {
        let kept = events.len().min(EVENT_BYTES);
        self.bytes[EVENTS..EVENTS + kept].copy_from_slice(&events[..kept]);
    }

    /// The money: four BCD digits at `$0694`, the fifth at `$0696`.
    #[must_use]
    pub fn money(&self) -> u32 {
        decimal(self.word(MONEY)) + decimal(self.word(MONEY + 2)) * 10_000
    }

    /// Writes the money.
    pub fn set_money(&mut self, money: u32) {
        self.set_word(MONEY, bcd(money % 10_000));
        self.set_word(MONEY + 2, bcd(money / 10_000));
    }

    /// The Prime Blue word `$07ED`.
    #[must_use]
    pub fn prime_blue(&self) -> u16 {
        self.word(PRIME_BLUE)
    }

    /// Writes the Prime Blue word.
    pub fn set_prime_blue(&mut self, word: u16) {
        self.set_word(PRIME_BLUE, word);
    }

    /// The inventory the slot holds, with its money and Prime Blue.
    #[must_use]
    pub fn inventory(&self) -> Inventory {
        let mut inventory = Inventory::default();
        inventory.set_slot_bytes(&self.bytes[ITEMS..ITEMS + ITEM_BYTES]);
        inventory.add_money(self.money());
        inventory.set_prime_blue(self.prime_blue());
        inventory
    }

    /// Writes an inventory: its items, money and Prime Blue.
    pub fn set_inventory(&mut self, inventory: &Inventory) {
        self.bytes[ITEMS..ITEMS + ITEM_BYTES].copy_from_slice(&inventory.slot_bytes());
        self.set_money(inventory.money());
        self.set_prime_blue(inventory.prime_blue());
    }
}

/// The byte that ends a name.
const NAME_END: u8 = 0xD4;

/// A BCD word's value; 0 for a word that is not BCD.
fn decimal(word: u16) -> u32 {
    assets::shops::bcd(word).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fields_sit_where_the_native_slot_keeps_them() {
        let mut slot = SaveSlot::default();
        slot.set_place(0x0F, 1, (472, 176));
        assert_eq!(&slot.bytes()[..8], &[0x0F, 0, 1, 0, 0xD0, 0x01, 0xA0, 0]);
        assert_eq!(
            (slot.map(), slot.facing(), slot.position()),
            (0x0F, 1, (472, 176))
        );
        slot.set_money(12_345);
        assert_eq!(&slot.bytes()[0x94..0x98], &[0x45, 0x23, 0x01, 0]);
        assert_eq!(slot.money(), 12_345);
        let mut events = vec![0; 512];
        events[0x20 / 8] = 1;
        events[EVENT_BYTES] = 0xFF;
        slot.set_events(&events);
        assert_eq!(slot.events()[4], 1);
        assert_eq!(
            slot.events().len(),
            0x140,
            "flags past `$07FF` are not kept"
        );
    }

    #[test]
    fn the_summary_reads_name_level_and_play_time() {
        let mut bytes = [0; SLOT_BYTES];
        bytes[NAME..NAME + 4].copy_from_slice(&[0x21, 0x52, 0x4B, NAME_END]);
        bytes[LEVEL] = 7;
        bytes[SECONDS..SECONDS + 4].copy_from_slice(&[0x10, 0x27, 0x01, 0]);
        let slot = SaveSlot::from_bytes(&bytes);
        assert_eq!(slot.name(), [0x21, 0x52, 0x4B]);
        assert_eq!((slot.level(), slot.seconds()), (7, 0x1_2710));
    }

    #[test]
    fn the_inventory_round_trips_and_other_bytes_pass_through() {
        let mut bytes = [0xAB; SLOT_BYTES];
        bytes[ITEMS..ITEMS + ITEM_BYTES].fill(0);
        let mut slot = SaveSlot::from_bytes(&bytes);
        let mut inventory = Inventory::default();
        inventory.add(0x81);
        inventory.add_money(90);
        inventory.add_prime_blue(3);
        slot.set_inventory(&inventory);
        let restored = slot.inventory();
        assert_eq!(restored.items(), [0x81]);
        assert_eq!((restored.money(), restored.prime_blue()), (90, 0x0003));
        // A byte nothing models, say the level at `$0656`, is kept.
        assert_eq!(slot.bytes()[0x56], 0xAB);
    }
}
