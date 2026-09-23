//! The underworld map `$03`: a Mode 7 byte layer from its spawn stream.
use assets::maps::visual::world::WorldMap;
use rom::{Revision, Rom};
use std::path::Path;

fn owned_rom() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    let rom = Rom::load(&std::fs::read(path).ok()?).ok()?;
    (rom.revision() == Revision::Japan).then_some(rom)
}

#[test]
fn the_underworld_layer_is_a_byte_per_cell_from_its_f0_record() {
    // `$83:88FB`: `F0 04 04 00 00 00 D6`: 64x64 cells at `$D6:0000`.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let world = WorldMap::from_rom(image, 0x0003).unwrap();
    assert_eq!((world.width(), world.height()), (64, 64));
    assert_eq!(world.cells(), &image[0x16_0000..0x16_1000]);
    // The native route's collision stops (`$80:C469`: blocked at `$A0`+).
    assert_eq!(world.cell(33, 47), Some(0xC1));
    assert_eq!(world.cell(23, 43), Some(0xCA));
    assert_eq!(world.cell(25, 56), Some(0xF7));
    assert!(
        world.cell(33, 34).is_some_and(|byte| byte < 0xA0),
        "the arrival"
    );
}

#[test]
fn the_underworld_draws_from_its_metatiles_and_8bpp_characters() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let world = WorldMap::from_rom(cartridge.image(), 0x0003).unwrap();
    // Every pixel resolves; indices come from the four characters of the
    // cell's metatile.
    let mut used = [false; 256];
    for y in (0..1024).step_by(3) {
        for x in (0..1024).step_by(3) {
            used[usize::from(world.pixel(x, y))] = true;
        }
    }
    assert!(used.iter().filter(|&&used| used).count() > 16);
    // The palette carries the map's colours at `$20` and `$D0`.
    assert_ne!(world.color(0x20).raw(), world.color(0x21).raw());
    assert!(
        WorldMap::from_rom(cartridge.image(), 0x000A).is_err(),
        "not a world map"
    );
}
