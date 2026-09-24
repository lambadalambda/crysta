//! The shops' records and stock ([notes](../../../docs/shops.md)).
//!
//! `$92:CC8A` scans the 9-byte records at `$96:C6DC` for the map it runs
//! in: map word, flag word (0 always, else the event flag that must be
//! set), stock pointer in bank `$96`, column, row, shop type. A stock entry
//! is item, price (a BCD word), unique flag; `$FF` ends it. The Prime Blue
//! shop (type 3) also costs Prime Blue, a BCD word per item at `$92:D57D`.

use std::fmt;

use crate::layout::{self, per_revision};

/// European `$99:CE26`, as `$92:E300` reads it; the stock sits in the
/// records' bank.
const RECORDS: u32 = 0x96_C6DC;
const RECORD: usize = 9;
/// European `$92:EBEF`, as `$92:E789` reads it.
const PRIME_BLUE_COSTS: u32 = 0x92_D57D;
/// Records and stock entries read before a table is refused as unending.
const LIMIT: u32 = 256;

/// Why the shop data did not decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShopError {
    /// A read left the image.
    Truncated(u32),
    /// A table did not end, or a price was not BCD.
    Invalid(u32, &'static str),
}

impl fmt::Display for ShopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated(at) => write!(f, "shop data at ${at:06X} leaves the image"),
            Self::Invalid(at, why) => write!(f, "shop data at ${at:06X}: {why}"),
        }
    }
}

impl std::error::Error for ShopError {}

/// One item a shop sells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShopItem {
    /// The item id.
    pub item: u8,
    /// The price of one, in money.
    pub price: u32,
    /// Sold once: skipped when owned, and bought one at a time.
    pub unique: bool,
}

/// A shopkeeper the map spawns, with the stock they sell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shop {
    /// The record's runtime address.
    pub record: u32,
    /// The map it stands in.
    pub map: u16,
    /// The event flag that must be set, if any.
    pub flag: Option<u16>,
    /// Pixel position: the cell's centre column, its bottom row.
    pub position: (u16, u16),
    /// Indexes the shop texts (`$0DE8`); 3 is the Prime Blue shop.
    pub kind: u8,
    /// What it sells, in list order.
    pub stock: Vec<ShopItem>,
}

/// Every shop record, in table order.
///
/// # Errors
/// Refuses a table that leaves the image or does not end, and a price
/// that is not BCD.
pub fn shops(image: &[u8]) -> Result<Vec<Shop>, ShopError> {
    let records = layout::at(image, RECORDS).ok_or(ShopError::Truncated(RECORDS))?;
    // The spawner's row offset: `ADC #$0010` at `$92:CD10`, `#$0008` at the
    // European `$92:E382`.
    let row = per_revision(image, 16, 8);
    let mut shops = Vec::new();
    for index in 0..LIMIT {
        let at = records + index * 9; // RECORD bytes
        let record = read(image, at, RECORD)?;
        let map = word(record, 0);
        if map & 0x8000 != 0 {
            return Ok(shops);
        }
        let flag = word(record, 2);
        shops.push(Shop {
            record: at,
            map,
            flag: (flag != 0).then_some(flag),
            position: (
                u16::from(record[6]) * 16 + 8,
                u16::from(record[7]) * 16 + row,
            ),
            kind: record[8],
            stock: stock(image, (records & 0xFF_0000) | u32::from(word(record, 4)))?,
        });
    }
    Err(ShopError::Invalid(records, "unending shop table"))
}

fn stock(image: &[u8], start: u32) -> Result<Vec<ShopItem>, ShopError> {
    let mut stock = Vec::new();
    for index in 0..LIMIT {
        let at = start + index * 4;
        let item = read(image, at, 1)?[0];
        if item == 0xFF {
            return Ok(stock);
        }
        let entry = read(image, at, 4)?;
        stock.push(ShopItem {
            item,
            price: bcd(word(entry, 1)).ok_or(ShopError::Invalid(at, "price is not BCD"))?,
            unique: entry[3] != 0,
        });
    }
    Err(ShopError::Invalid(start, "unending stock"))
}

/// The Prime Blue one `item` costs in the Prime Blue shop.
///
/// # Errors
/// Refuses a read outside the image and a cost that is not BCD.
pub fn prime_blue_cost(image: &[u8], item: u8) -> Result<u32, ShopError> {
    let costs =
        layout::at(image, PRIME_BLUE_COSTS).ok_or(ShopError::Truncated(PRIME_BLUE_COSTS))?;
    let at = costs + u32::from(item) * 2;
    bcd(word(read(image, at, 2)?, 0)).ok_or(ShopError::Invalid(at, "cost is not BCD"))
}

/// A BCD word's value.
#[must_use]
pub fn bcd(word: u16) -> Option<u32> {
    (0..4).try_fold(0, |value, digit| {
        let nibble = u32::from(word >> (12 - digit * 4) & 0xF);
        (nibble < 10).then_some(value * 10 + nibble)
    })
}

pub(crate) fn read(image: &[u8], at: u32, count: usize) -> Result<&[u8], ShopError> {
    let start = usize::try_from(at & 0x3F_FFFF).map_err(|_| ShopError::Truncated(at))?;
    image
        .get(start..start + count)
        .ok_or(ShopError::Truncated(at))
}

fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bcd_words_read_as_decimal_and_other_nibbles_are_refused() {
        assert_eq!(bcd(0x0170), Some(170));
        assert_eq!(bcd(0x9999), Some(9999));
        assert_eq!(bcd(0x001A), None);
    }

    #[test]
    fn a_record_places_its_shop_and_reads_its_stock() {
        let mut image = vec![0; 0x17_0000];
        let records = 0x16_C6DC;
        image[records..records + 9].copy_from_slice(&[0x1E, 0, 0, 0, 0x00, 0xC9, 39, 5, 0]);
        image[records + 9..records + 11].copy_from_slice(&[0xFF, 0xFF]);
        image[0x16_C900..0x16_C909].copy_from_slice(&[0x10, 0x10, 0, 0, 0x80, 0x70, 1, 1, 0xFF]);
        let shops = shops(&image).unwrap();
        assert_eq!(shops.len(), 1);
        let shop = &shops[0];
        assert_eq!(
            (shop.map, shop.flag, shop.position, shop.kind),
            (0x1E, None, (632, 96), 0)
        );
        assert_eq!(
            shop.stock,
            [
                ShopItem {
                    item: 0x10,
                    price: 10,
                    unique: false
                },
                ShopItem {
                    item: 0x80,
                    price: 170,
                    unique: true
                },
            ]
        );
    }
}
