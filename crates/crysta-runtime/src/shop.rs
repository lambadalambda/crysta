//! A shop's talk callback `$92:CD70`, which runs in native code
//! ([notes](../../../docs/shops.md)): the greeting, then browsing the stock
//! with the pad, the confirm choice, the refusals and the purchase.
//!
//! Each native pass waits a frame (`$92:CF30`); a text request (`COP 1C`)
//! with its wait (`COP 1F`) holds the loop until the window is free. The
//! display actor `$92:D190` and Ark's held-item pose are the host's to
//! draw from [`Shop::showing`].

use crate::audio::Audio;
use crate::inventory::{bcd, Inventory, MOST};
use crate::scene::{Dialogue, Presses};
use assets::shops::{prime_blue_cost, Shop as Record, ShopItem};
use assets::text::{DialoguePage, HouseDialogue};
use std::collections::VecDeque;

/// The texts, in bank `$92`: each picks its shop type's part (`$0DE8`).
const SOLD_OUT: u32 = 0x92_A1ED;
const GREETING: u32 = 0x92_A2A0;
const HELP: u32 = 0x92_A355;
const DESCRIPTION: u32 = 0x92_A4FB;
const CONFIRM: u32 = 0x92_A765;
const THANKS: u32 = 0x92_A86C;
/// `$92:8095` closes the window (`$D7`) as the shop ends.
const FAREWELL: u32 = 0x92_8095;
/// Refusals by `$92:D120`'s code: 0 money, 1 too many, 2 Prime Blue, 3 no
/// free slot.
const REFUSALS: [u32; 4] = [0x92_A541, 0x92_A65D, 0x92_A8EE, 0x92_A911];
/// The confirm's choice catalog (`COP 1A 0A`): 1 buys.
const CHOICE: u8 = 0x0A;
const BUY: u8 = 1;
/// Port 3: a change of item or count (`COP 36 22`), a purchase (`$47`).
const CHANGE_SOUND: u8 = 0x22;
const BUY_SOUND: u8 = 0x47;
/// The Prime Blue shop's type.
const PRIME_BLUE_SHOP: u8 = 3;
/// Frames Ark holds a bought item up (`$92:CE84`), and the one after
/// (`$92:CE9E`).
const HOLD: u16 = 61;

/// What the shop does next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// Shows a text and waits for the window (`COP 1C`, `COP 1F`).
    Say(u32),
    /// Opens the confirm choice and waits for its answer.
    Ask,
    /// Buys the chosen item and count.
    Buy,
    /// Holds the bought item up for some frames.
    Hold(u16),
    /// Finds the first item for sale again, or says it is sold out.
    Restock,
    /// Reads the pad each frame.
    Browse,
    /// Removes the display and Ark's held item (`$92:CEC8`).
    Close,
    /// Ends the shop.
    End,
}

/// What the world lends a shop frame.
pub struct Counter<'a, 'b> {
    /// The ROM, for the texts and the Prime Blue costs.
    pub image: &'a [u8],
    /// The window the shop talks through.
    pub dialogue: &'b mut Dialogue,
    /// What the player pays with and carries away.
    pub inventory: &'b mut Inventory,
    /// Where the shop's sounds go.
    pub audio: &'b mut Audio,
}

/// A shop under way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shop {
    kind: u8,
    stock: Vec<ShopItem>,
    /// The chosen stock entry (`+$16`) and count (`$0DD2`).
    index: usize,
    quantity: u8,
    steps: VecDeque<Step>,
    /// Whether the step at the front has made its request.
    asked: bool,
    /// Whether the display actor and Ark's held item show (`$92:CD8F`
    /// to `$92:CEC8`).
    display: bool,
}

impl Shop {
    /// Opens `record`'s shop, as the talk callback does: the greeting and
    /// the help, or "sold out" and the end.
    #[must_use]
    pub fn open(record: &Record, inventory: &Inventory) -> Self {
        let mut shop = Self {
            kind: record.kind,
            stock: record.stock.clone(),
            index: 0,
            quantity: 1,
            steps: VecDeque::new(),
            asked: false,
            display: false,
        };
        match shop.first_for_sale(inventory) {
            Some(index) => {
                shop.index = index;
                shop.display = true;
                shop.steps
                    .extend([Step::Say(GREETING), Step::Say(HELP), Step::Browse]);
            }
            None => shop.steps.extend([Step::Say(SOLD_OUT), Step::End]),
        }
        shop
    }

    /// The item on offer and its count, while the shop shows one.
    #[must_use]
    pub fn showing(&self) -> Option<(u8, u8)> {
        self.display.then(|| (self.item().item, self.quantity))
    }

    /// Whether the loop reads the pad now, between its texts.
    #[must_use]
    pub fn browsing(&self) -> bool {
        self.steps.front() == Some(&Step::Browse)
    }

    fn item(&self) -> ShopItem {
        self.stock[self.index]
    }

    /// Whether `entry` may be offered: a unique item held is not
    /// (`$92:D05B`).
    fn offered(entry: ShopItem, inventory: &Inventory) -> bool {
        !(entry.unique && inventory.count(entry.item) > 0)
    }

    /// `$92:D098`: the first entry on offer.
    fn first_for_sale(&self, inventory: &Inventory) -> Option<usize> {
        self.stock
            .iter()
            .position(|&entry| Self::offered(entry, inventory))
    }

    /// The price of the chosen count in money, and in Prime Blue.
    fn price(&self, image: &[u8]) -> (u32, u32) {
        let count = u32::from(self.quantity);
        let prime_blue = if self.kind == PRIME_BLUE_SHOP {
            prime_blue_cost(image, self.item().item).unwrap_or(0) * count
        } else {
            0
        };
        (self.item().price * count, prime_blue)
    }

    /// `$92:D120`: why the chosen item and count cannot be bought.
    fn refusal(&self, image: &[u8], inventory: &Inventory) -> Option<usize> {
        let item = self.item().item;
        let (money, prime_blue) = self.price(image);
        if !inventory.has_room(item) && inventory.count(item) == 0 {
            return Some(3);
        }
        // `CMP` on the BCD words.
        if self.kind == PRIME_BLUE_SHOP && inventory.prime_blue() < bcd(prime_blue) {
            return Some(2);
        }
        if inventory.money() < money {
            return Some(0);
        }
        (inventory.count(item) + self.quantity > MOST).then_some(1)
    }

    /// One frame. Returns whether the shop has ended. Steps go on in the
    /// same frame, as native code does after `COP 1F`, until one waits:
    /// for the window, a hold frame, or the next pass of the loop.
    pub fn frame(&mut self, presses: Presses, counter: &mut Counter<'_, '_>) -> bool {
        let answer = counter.dialogue.press(presses);
        let mut browsed = false;
        for _ in 0..8 {
            let Some(&step) = self.steps.front() else {
                return true;
            };
            let go_on = match step {
                Step::Say(text) => self.say(text, counter),
                Step::Ask => self.ask(answer, counter),
                Step::Buy => {
                    self.buy(counter);
                    false
                }
                Step::Hold(frames) if frames <= 1 => {
                    self.next([Step::Restock]);
                    true
                }
                Step::Hold(frames) => {
                    self.steps[0] = Step::Hold(frames - 1);
                    false
                }
                Step::Restock => {
                    self.restock(counter.inventory);
                    true
                }
                Step::Close => {
                    self.display = false;
                    self.next([]);
                    true
                }
                // One pass of the loop a frame.
                Step::Browse if browsed => false,
                Step::Browse => {
                    browsed = true;
                    self.browse(presses, counter)
                }
                Step::End => return true,
            };
            if !go_on {
                break;
            }
        }
        false
    }

    /// Replaces the step at the front with `steps`.
    fn next<const N: usize>(&mut self, steps: [Step; N]) {
        self.steps.pop_front();
        for step in steps.into_iter().rev() {
            self.steps.push_front(step);
        }
        self.asked = false;
    }

    /// Requests a text, then waits until the window is free. Returns
    /// whether the shop goes on this frame.
    fn say(&mut self, text: u32, counter: &mut Counter<'_, '_>) -> bool {
        if !self.asked {
            let pages = self.pages(counter.image, text);
            self.asked = counter.dialogue.request(pages);
            false
        } else if counter.dialogue.busy() {
            false
        } else {
            self.next([]);
            true
        }
    }

    /// Opens the confirm choice, then takes its answer; a catalog that does
    /// not decode answers "stop".
    fn ask(&mut self, answer: Option<u8>, counter: &mut Counter<'_, '_>) -> bool {
        let answer = match answer {
            Some(answer) => answer,
            None if self.asked => return false,
            None => match HouseDialogue::choice_at(counter.image, CHOICE) {
                Ok(choice) => {
                    self.asked = counter.dialogue.ask(choice);
                    return false;
                }
                Err(_) => 0,
            },
        };
        if answer == BUY {
            self.next([Step::Say(THANKS), Step::Buy]);
        } else {
            self.next([Step::Say(HELP), Step::Browse]);
        }
        true
    }

    /// `$92:D098` after a purchase: the first item on offer again, or
    /// "sold out" and the shop's end.
    fn restock(&mut self, inventory: &Inventory) {
        if let Some(index) = self.first_for_sale(inventory) {
            self.index = index;
            self.quantity = 1;
            self.next([Step::Say(HELP), Step::Browse]);
        } else {
            self.next([
                Step::Say(SOLD_OUT),
                Step::Close,
                Step::Say(FAREWELL),
                Step::End,
            ]);
        }
    }

    /// A text's pages as this shop and its item pick them.
    fn pages(&self, image: &[u8], text: u32) -> Vec<DialoguePage> {
        let (kind, item) = (self.kind, self.item().item);
        HouseDialogue::decode_reading(image, text, |address| match address {
            0x0DE8 => Some(kind),
            0x0DD0 => Some(item),
            _ => None,
        })
        .unwrap_or_default()
    }

    /// `$92:CE45`: the purchase, after the thanks.
    fn buy(&mut self, counter: &mut Counter<'_, '_>) {
        let (money, prime_blue) = self.price(counter.image);
        for _ in 0..self.quantity {
            counter.inventory.add(self.item().item);
        }
        counter.inventory.take_money(money);
        if self.kind == PRIME_BLUE_SHOP {
            counter.inventory.take_prime_blue(prime_blue);
        }
        counter.audio.sound_port3(BUY_SOUND);
        self.next([Step::Hold(HOLD)]);
    }

    /// A pass of the browsing loop `$92:CDE2`: B leaves, A buys, Left
    /// (first) or Right choose the item; else Up and Down the count, and L
    /// describes it. Returns whether the shop goes on this frame.
    fn browse(&mut self, presses: Presses, counter: &mut Counter<'_, '_>) -> bool {
        if presses.cancel {
            self.next([Step::Close, Step::Say(FAREWELL), Step::End]);
            return true;
        }
        if presses.confirm {
            match self.refusal(counter.image, counter.inventory) {
                Some(code) => self.next([Step::Say(REFUSALS[code]), Step::Say(HELP), Step::Browse]),
                None => self.next([Step::Say(CONFIRM), Step::Ask]),
            }
            return true;
        }
        if presses.left || presses.right {
            if let Some(index) = self.step_item(!presses.left, counter.inventory) {
                self.index = index;
                self.quantity = 1;
                counter.audio.sound_port3(CHANGE_SOUND);
                return false;
            }
        }
        if (presses.up || presses.down) && !self.item().unique {
            self.quantity = if presses.up {
                self.quantity % MOST + 1
            } else {
                (self.quantity + MOST - 2) % MOST + 1
            };
            counter.audio.sound_port3(CHANGE_SOUND);
        }
        if presses.describe {
            self.next([Step::Say(DESCRIPTION), Step::Say(HELP), Step::Browse]);
            return true;
        }
        false
    }

    /// `$92:CF61`: the next entry on offer to the right or left, wrapping;
    /// `None` when it is the one chosen.
    fn step_item(&self, right: bool, inventory: &Inventory) -> Option<usize> {
        let count = self.stock.len();
        let mut index = self.index;
        for _ in 0..count {
            index = if right {
                (index + 1) % count
            } else {
                (index + count - 1) % count
            };
            if Self::offered(self.stock[index], inventory) {
                break;
            }
        }
        (index != self.index).then_some(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> Record {
        let item = |item, price, unique| ShopItem {
            item,
            price,
            unique,
        };
        Record {
            record: 0x96_C6DC,
            map: 0x1E,
            flag: None,
            position: (632, 96),
            kind: 0,
            stock: vec![
                item(0x10, 10, false),
                item(0x80, 170, true),
                item(0x13, 13, false),
            ],
        }
    }

    fn browsing(inventory: &Inventory) -> Shop {
        let mut shop = Shop::open(&record(), inventory);
        shop.steps = VecDeque::from([Step::Browse]);
        shop
    }

    #[test]
    fn a_held_unique_item_is_skipped_both_ways() {
        let mut inventory = Inventory::default();
        let shop = browsing(&inventory);
        assert_eq!(shop.step_item(true, &inventory), Some(1));
        inventory.add(0x80);
        assert_eq!(shop.step_item(true, &inventory), Some(2));
        assert_eq!(shop.step_item(false, &inventory), Some(2), "wraps");
    }

    #[test]
    fn refusals_follow_the_native_order() {
        let mut inventory = Inventory::default();
        let mut shop = browsing(&inventory);
        assert_eq!(shop.refusal(&[], &inventory), Some(0), "no money");
        inventory.add_money(100);
        assert_eq!(shop.refusal(&[], &inventory), None);
        for _ in 0..8 {
            inventory.add(0x10);
        }
        shop.quantity = 2;
        assert_eq!(shop.refusal(&[], &inventory), Some(1), "8 held and 2 more");
        inventory.add(0x10);
        assert_eq!(shop.refusal(&[], &inventory), Some(1), "a full own slot");
        let mut full = Inventory::default();
        full.add_money(100);
        for item in 0x20..0x20 + 27 {
            full.add(item);
        }
        shop.quantity = 1;
        assert_eq!(shop.refusal(&[], &full), Some(3), "no free slot");
    }

    #[test]
    fn the_prime_blue_shop_also_wants_prime_blue() {
        // `$92:D57D` at item `$10` in this image: a cost of 2.
        let mut image = vec![0; 0x13_0000];
        image[0x12_D57D + 0x20] = 0x02;
        let mut inventory = Inventory::default();
        inventory.add_money(100);
        let mut shop = browsing(&inventory);
        shop.kind = PRIME_BLUE_SHOP;
        assert_eq!(shop.price(&image), (10, 2));
        assert_eq!(shop.refusal(&image, &inventory), Some(2), "none held");
        inventory.add_prime_blue(2);
        assert_eq!(shop.refusal(&image, &inventory), None);
    }

    #[test]
    fn the_count_wraps_from_nine_to_one_and_back() {
        let inventory = Inventory::default();
        let mut shop = browsing(&inventory);
        let mut counter_parts = (Dialogue::default(), inventory, Audio::default());
        let mut press = |shop: &mut Shop, presses| {
            let (dialogue, inventory, audio) = &mut counter_parts;
            shop.frame(
                presses,
                &mut Counter {
                    image: &[],
                    dialogue,
                    inventory,
                    audio,
                },
            );
        };
        let down = Presses {
            down: true,
            ..Presses::NONE
        };
        press(&mut shop, down);
        assert_eq!(shop.quantity, 9);
        let up = Presses {
            up: true,
            ..Presses::NONE
        };
        press(&mut shop, up);
        assert_eq!(shop.quantity, 1);
    }
}
