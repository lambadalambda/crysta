//! The player's money, Prime Blue and items ([shops](../../../docs/shops.md)).
//!
//! Items sit in (item, count) slots at `$7F:8000`; `$8D:9732` names the
//! slots an item may take: magic (`$01..$0F`) and the fixed items
//! (`$7A..$7F`, `$9C..$9F`, `$BC..`) one slot each, the rest a shared
//! range. A slot holds 9 at most. Money (`$0694`/`$0696`) and Prime Blue
//! (`$07ED`) are BCD natively, kept here as their values.

use std::ops::Range;

/// Item slots: the `$100` bytes from `$7F:8000` the ranges reach.
const SLOTS: usize = 0x100 / 2;
/// The magic's slots, by item `$01..$0F` (`$8D:9790`), as byte offsets.
const MAGIC: [u8; 15] = [
    0x80, 0x8A, 0x82, 0x8C, 0x84, 0x8E, 0x86, 0x90, 0x88, 0x92, 0x94, 0x96, 0x98, 0x9A, 0x9C,
];
/// The most one slot holds.
pub const MOST: u8 = 9;
/// Money's cap (`$8D:95DB`).
const MONEY: u32 = 99_999;
/// `$8D:95B3` compares the BCD sum with `$03E8` as binary, so any 400 or
/// more becomes the raw word `$03E7`, not BCD.
const PRIME_BLUE_CAP: (u16, u16) = (0x03E8, 0x03E7);

/// What the player carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    slots: [(u8, u8); SLOTS],
    money: u32,
    /// `$07ED` as the engine keeps it: a BCD word, or the cap's raw word.
    prime_blue: u16,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: [(0, 0); SLOTS],
            money: 0,
            prime_blue: 0,
        }
    }
}

/// The slots `item` may take (`$8D:9732`), as slot indices.
fn range(item: u8) -> Range<usize> {
    let single = |offset: u8| usize::from(offset / 2)..usize::from(offset / 2) + 1;
    match item {
        0 => 0..0,
        0x01..=0x0F => single(MAGIC[usize::from(item - 1)]),
        0x10..=0x79 => 0..0x36 / 2,
        0x7A..=0x7F => single(0x36 + (item - 0x7A) * 2),
        0x80..=0x9B => 0x48 / 2..0x60 / 2,
        0x9C..=0x9F => single(0x40 + (item - 0x9C) * 2),
        0xA0..=0xBB => 0x68 / 2..0x80 / 2,
        _ => single(0x60 + (item - 0xBC) * 2),
    }
}

impl Inventory {
    /// The slot `item` would go into: its own, else the first empty one;
    /// `None` when neither is there or the slot is full (`$8D:96ED`).
    fn slot(&self, item: u8) -> Option<usize> {
        let range = range(item);
        let own = range.clone().find(|&slot| self.slots[slot].0 == item);
        let slot = own.or_else(|| range.clone().find(|&slot| self.slots[slot].0 == 0))?;
        (self.slots[slot].1 < MOST).then_some(slot)
    }

    /// Whether one more `item` fits.
    #[must_use]
    pub fn has_room(&self, item: u8) -> bool {
        self.slot(item).is_some()
    }

    /// Adds one `item` (`$8D:9653`); `false` when it does not fit.
    pub fn add(&mut self, item: u8) -> bool {
        let Some(slot) = self.slot(item) else {
            return false;
        };
        self.slots[slot] = (item, self.slots[slot].1 + 1);
        true
    }

    /// Takes one `item` (`$8D:96A0`); `false` when none is held. Item 0
    /// takes nothing and succeeds. The last one empties its slot; a count
    /// of 0 wraps, as the engine's `DEC` does. Unequipping (`$0648`) is
    /// not modelled.
    pub fn remove(&mut self, item: u8) -> bool {
        if item == 0 {
            return true;
        }
        let Some(slot) = range(item).find(|&slot| self.slots[slot].0 == item) else {
            return false;
        };
        let count = self.slots[slot].1.wrapping_sub(1);
        self.slots[slot] = if count == 0 { (0, 0) } else { (item, count) };
        true
    }

    /// How many `item` are held (`$8D:9628`).
    #[must_use]
    pub fn count(&self, item: u8) -> u8 {
        range(item)
            .find(|&slot| self.slots[slot].0 == item)
            .map_or(0, |slot| self.slots[slot].1)
    }

    /// The items held, in slot order.
    #[must_use]
    pub fn items(&self) -> Vec<u8> {
        self.slots
            .iter()
            .filter(|slot| slot.0 != 0)
            .map(|slot| slot.0)
            .collect()
    }

    /// The money.
    #[must_use]
    pub const fn money(&self) -> u32 {
        self.money
    }

    /// Adds money, up to 99,999.
    pub fn add_money(&mut self, amount: u32) {
        self.money = self.money.saturating_add(amount).min(MONEY);
    }

    /// Takes money; `false`, and nothing taken, when there is not enough.
    pub fn take_money(&mut self, amount: u32) -> bool {
        take(&mut self.money, amount)
    }

    /// The Prime Blue word `$07ED`: BCD, or the cap's raw `$03E7`.
    #[must_use]
    pub const fn prime_blue(&self) -> u16 {
        self.prime_blue
    }

    /// Adds Prime Blue (`$8D:95A8`), with the engine's cap.
    pub fn add_prime_blue(&mut self, amount: u32) {
        let sum = bcd(value(self.prime_blue).saturating_add(amount));
        self.prime_blue = if sum >= PRIME_BLUE_CAP.0 {
            PRIME_BLUE_CAP.1
        } else {
            sum
        };
    }

    /// Takes Prime Blue (`$8D:95C0`); `false`, and nothing taken, when
    /// there is not enough.
    pub fn take_prime_blue(&mut self, amount: u32) -> bool {
        let mut left = value(self.prime_blue);
        let taken = take(&mut left, amount);
        if taken {
            self.prime_blue = bcd(left);
        }
        taken
    }
}

/// A value's BCD word, as the engine keeps counts (up to 9999).
#[must_use]
pub fn bcd(value: u32) -> u16 {
    (0..4).fold(0, |word, digit| {
        let nibble = value / 10u32.pow(digit) % 10;
        word | u16::try_from(nibble).unwrap_or(0) << (digit * 4)
    })
}

/// A BCD word's value, reading each nibble as a digit (the cap's `$E`
/// included, as decimal arithmetic on it roughly does).
fn value(word: u16) -> u32 {
    (0..4).fold(0, |value, digit| {
        value * 10 + u32::from(word >> (12 - digit * 4) & 0xF)
    })
}

fn take(from: &mut u32, amount: u32) -> bool {
    from.checked_sub(amount).map(|left| *from = left).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_fill_their_ranges_nine_to_a_slot() {
        let mut inventory = Inventory::default();
        for _ in 0..MOST {
            assert!(inventory.add(0x10));
        }
        assert!(!inventory.add(0x10), "a slot holds nine");
        assert_eq!(inventory.count(0x10), 9);
        // Another shared item takes the next slot.
        assert!(inventory.add(0x11));
        assert_eq!(inventory.items(), [0x10, 0x11]);
        // A fixed item has one slot; the magic too, past the shared range.
        assert!(inventory.add(0x7A) && inventory.add(0x01));
        assert_eq!(inventory.items(), [0x10, 0x11, 0x7A, 0x01]);
    }

    #[test]
    fn a_full_shared_range_refuses_a_new_item_but_not_a_held_one() {
        let mut inventory = Inventory::default();
        for item in 0x10..0x10 + 27 {
            assert!(inventory.add(item));
        }
        assert!(!inventory.has_room(0x40), "27 shared slots");
        assert!(inventory.has_room(0x10));
    }

    #[test]
    fn removing_the_last_one_empties_the_slot() {
        let mut inventory = Inventory::default();
        inventory.add(0xA1);
        inventory.add(0xA1);
        assert!(inventory.remove(0xA1));
        assert_eq!(inventory.count(0xA1), 1);
        assert!(inventory.remove(0xA1));
        assert!(inventory.items().is_empty());
        assert!(!inventory.remove(0xA1));
        assert!(inventory.remove(0), "item 0 succeeds");
    }

    #[test]
    fn values_encode_as_bcd_words() {
        assert_eq!(bcd(0), 0);
        assert_eq!(bcd(999), 0x0999);
        assert_eq!(bcd(1_234), 0x1234);
    }

    #[test]
    fn money_caps_and_a_short_purse_takes_nothing() {
        let mut inventory = Inventory::default();
        inventory.add_money(99_990);
        inventory.add_money(20);
        assert_eq!(inventory.money(), 99_999);
        assert!(!inventory.take_money(100_000));
        assert_eq!(inventory.money(), 99_999);
        assert!(inventory.take_money(999));
        assert_eq!(inventory.money(), 99_000);
        inventory.add_prime_blue(399);
        assert_eq!(inventory.prime_blue(), 0x0399);
        assert!(!inventory.take_prime_blue(400) && inventory.take_prime_blue(1));
        assert_eq!(inventory.prime_blue(), 0x0398);
        inventory.add_prime_blue(2);
        assert_eq!(inventory.prime_blue(), 0x03E7, "400 or more: the raw cap");
    }
}
