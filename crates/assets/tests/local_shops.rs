//! ROM-backed checks of the shop records and stock.
use assets::shops::{prime_blue_cost, shops, ShopItem};
use rom::{Revision, Rom};
use std::path::Path;

fn owned_rom() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    let rom = Rom::load(&std::fs::read(path).ok()?).ok()?;
    (rom.revision() == Revision::Japan).then_some(rom)
}

fn priced(stock: &[ShopItem]) -> Vec<(u8, u32, bool)> {
    stock.iter().map(|s| (s.item, s.price, s.unique)).collect()
}

#[test]
fn the_crysta_shops_sell_their_native_stock() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let shops = shops(image).unwrap();
    let crysta: Vec<_> = shops
        .iter()
        .filter(|s| (0x1D..=0x1E).contains(&s.map))
        .collect();
    assert_eq!(crysta.len(), 3);
    let item_shop = crysta[0];
    assert_eq!(
        (item_shop.map, item_shop.flag, item_shop.kind),
        (0x1E, None, 0)
    );
    assert_eq!(item_shop.position, (39 * 16 + 8, 5 * 16 + 16));
    assert_eq!(
        priced(&item_shop.stock),
        [
            (0x10, 10, false),
            (0x11, 25, false),
            (0x13, 13, false),
            (0x80, 170, true),
            (0xA1, 190, false)
        ]
    );
    // After flag `$D8` a second record on the same cell sells for less.
    assert_eq!((crysta[1].map, crysta[1].flag), (0x1E, Some(0xD8)));
    assert_eq!(crysta[1].stock[0].price, 5);
    let prime_blue = crysta[2];
    assert_eq!((prime_blue.map, prime_blue.kind), (0x1D, 3));
    assert_eq!(
        priced(&prime_blue.stock),
        [(0x01, 5, false), (0x03, 5, false)]
    );
    for item in [0x01, 0x03] {
        assert_eq!(prime_blue_cost(image, item).unwrap(), 1);
    }
}

#[test]
fn the_shop_texts_pick_their_parts_by_the_words_they_read() {
    use assets::text::HouseDialogue;
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    // `CE E8 0D`: the greeting of shop type `$0DE8`; unresolved, refused.
    assert!(HouseDialogue::decode_at(image, 0x92_A2A0).is_err());
    let glyphs = |source, words: &[(u16, u8)]| {
        let pages = HouseDialogue::decode_reading(image, source, |address| {
            words.iter().find(|w| w.0 == address).map(|w| w.1)
        })
        .unwrap();
        pages.iter().map(|page| page.glyphs().len()).sum::<usize>()
    };
    for kind in [0, 3] {
        for source in [0x92_A1ED, 0x92_A2A0, 0x92_A355] {
            assert!(
                glyphs(source, &[(0x0DE8, kind)]) > 0,
                "{source:#x} type {kind}"
            );
        }
    }
    // The name window's text: `CE D0 0D 79 83`, the name of item `$0DD0`.
    let names: Vec<usize> = [0x10, 0x11, 0x13, 0x80, 0xA1, 0x01, 0x03]
        .iter()
        .map(|&item| glyphs(0x92_A1E1, &[(0x0DD0, item)]))
        .collect();
    assert!(names.iter().all(|&count| count > 0), "{names:?}");
    // The first item's name (`$92:8632`): six glyphs, five of two bytes.
    assert_eq!(names[0], 6);
}

#[test]
fn the_shop_confirm_is_a_two_option_choice() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let choice = assets::text::HouseDialogue::choice_at(cartridge.image(), 0x0A).unwrap();
    assert_eq!(choice.options.map(|option| option.result), [1, 2]);
}
