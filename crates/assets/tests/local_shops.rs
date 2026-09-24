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
