//! ROM-backed checks of the area titles.
use assets::labels::{area_title, TitleMotion};
use assets::maps::scripts::EventFlags;
use rom::{Revision, Rom};
use std::path::Path;

fn owned_rom() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    let rom = Rom::load(&std::fs::read(path).ok()?).ok()?;
    (rom.revision() == Revision::Japan).then_some(rom)
}

#[test]
fn the_slices_rooms_carry_their_titles_after_the_spawn_list() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let flags = vec![0u8; 512];
    let count = |map| {
        area_title(image, map, EventFlags::Bitmap(&flags))
            .unwrap()
            .map(|glyphs| glyphs.len())
    };
    // 長老の家, クリスタルホルム, よろず屋, 中央の部屋; none in C.
    assert_eq!(count(0x0D), Some(4));
    assert_eq!(count(0x0A), Some(8));
    assert_eq!(count(0x1E), Some(4));
    assert_eq!(count(0x41), Some(5));
    assert_eq!(count(0x0C), None);
    // Flag `$14` hides every title.
    let mut hidden = flags.clone();
    hidden[0x14 / 8] |= 1 << (0x14 % 8);
    assert_eq!(
        area_title(image, 0x0D, EventFlags::Bitmap(&hidden)).unwrap(),
        None
    );
}

#[test]
fn a_titles_letters_fly_in_hold_and_fly_away() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let motion = TitleMotion::from_rom(cartridge.image()).unwrap();
    assert_eq!(motion.length(), 137);
    // Four glyphs: all in place from the last glyph's arrival on.
    let settled = motion.positions(4, 4 * 4 + 50);
    assert_eq!(
        settled,
        [(0, 104, 48), (1, 116, 48), (2, 128, 48), (3, 140, 48)]
    );
    // Before its delay a glyph is off screen to the right.
    assert!(motion.positions(4, 0).is_empty());
    // After the last glyph's script, nothing.
    assert!(motion.positions(4, 4 * 4 + 137).is_empty());
}
