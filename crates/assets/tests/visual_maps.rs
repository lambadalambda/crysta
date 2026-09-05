//! Synthetic complete resource recipe: no extracted graphics or map bytes.
use assets::maps::scripts::{resolve_map, Command, Limits};
use assets::{compression, graphics::IndexedPixel, maps::visual::CavernBackground};

fn fixture() -> Vec<u8> {
    let mut image = vec![0; 0x6_A505];
    let entry = 0x6_959C + 0x128 * 3;
    image[entry..entry + 3].copy_from_slice(&[0, 1, 0xC0]);
    // All packed sources are within the root's C0 bank.
    let script = [
        0x80, 0, 0x20, 1, 0, 0x10, 0, 0, 0, 0x40, 0, 0x60, 0x20, 0, 0x20, 0, 0x20, 0, 0x40, 0, 1,
        0, 0x30, 0, 0x20, 0, 8, 0, 0x81, 0, 0x40, 0, 0x10, 1, 0, 0x50, 0, 0x80, 0, 8, 0, 0, 0x60,
        0, 0x70, 0, 0x40, 0, 0x20, 0, 0, 0x70, 0, 0,
    ];
    image[0x100..0x100 + script.len()].copy_from_slice(&script);
    let mut tiles = vec![0; 0x4000];
    // Tile 1: color 1 everywhere. Tile 0 stays transparent.
    for row in 0..8 {
        tiles[32 + row * 2] = 0xFF;
    }
    let mut definitions = vec![0; 0x1000];
    // Metatile 1: transparent TL/BR; opaque palette 2 TR; priority palette 3 BL.
    definitions[10..12].copy_from_slice(&0x0801_u16.to_le_bytes());
    definitions[12..14].copy_from_slice(&0x2C01_u16.to_le_bytes());
    let mut layer = vec![0; 512];
    layer[0] = 1;
    for (at, data) in [
        (0x1000, tiles),
        (0x3000, definitions),
        (0x4000, vec![0; 512]),
    ] {
        let packet = compression::encode(&data).unwrap();
        image[at..at + packet.len()].copy_from_slice(&packet);
    }
    let packet = compression::encode(&layer).unwrap();
    image[0x5000..0x5002].copy_from_slice(&[1, 1]);
    image[0x5002..0x5002 + packet.len()].copy_from_slice(&packet);
    image[0x2002..0x2004].copy_from_slice(&31_u16.to_le_bytes());
    image
}

#[test]
fn resolves_and_renders_the_profile_without_runtime_memory() {
    let image = fixture();
    let scene = CavernBackground::from_rom(&image).unwrap();
    assert_eq!((scene.layer().width(), scene.layer().height()), (16, 16));
    assert_eq!(scene.tiles().len(), 512);
    assert_eq!(scene.metatiles().len(), 512);
    assert_eq!(scene.palette()[33].rgb8(), [255, 0, 0]);
    assert_eq!(scene.pixel(0, 0).unwrap(), IndexedPixel::Transparent);
    assert_eq!(
        scene.pixel(8, 0).unwrap(),
        IndexedPixel::Opaque {
            palette_index: 33,
            priority: false
        }
    );
    assert_eq!(
        scene.pixel(0, 8).unwrap(),
        IndexedPixel::Opaque {
            palette_index: 49,
            priority: true
        }
    );
    assert!(scene.pixel(256, 0).is_err());
    assert!(scene.pixel(0, usize::MAX).is_err());
    for resource in scene.resources() {
        assert_eq!(resource.source_bytes(), &image[resource.source_range()]);
    }
}

#[test]
fn rejects_unknown_recipe_duplicate_loads_and_bad_extents() {
    let good = fixture();
    for (at, value) in [(0x103, 2), (0x10C, 0x61), (0x114, 0x80), (0x121, 2)] {
        let mut image = good.clone();
        image[at] = value;
        assert!(CavernBackground::from_rom(&image).is_err(), "at {at:x}");
    }
    let mut truncated = good.clone();
    // Packed source wraps to an invalid ROM window.
    truncated[0x10D..0x110].copy_from_slice(&[0xFF, 0xFF, 0x7F]);
    assert!(CavernBackground::from_rom(&truncated).is_err());
    let mut image = good.clone();
    image[0x1000..0x1003].copy_from_slice(&[0, 1, 0]);
    assert!(CavernBackground::from_rom(&image).is_err());
    assert!(CavernBackground::from_rom(&[]).is_err());
    // Replace root END with a second layer load: never silently compose it.
    let program = resolve_map(&good, 0x128, Limits::default()).unwrap();
    let end = program
        .instructions
        .last()
        .unwrap()
        .address
        .normalized()
        .value() as usize;
    assert_eq!(good[end], 0);
    image = good;
    image[end..end + 6].copy_from_slice(&[0x10, 1, 0, 0x50, 0, 0]);
    let program = resolve_map(&image, 0x128, Limits::default()).unwrap();
    assert_eq!(
        program
            .instructions
            .iter()
            .filter(|i| matches!(i.command, Command::Resource { .. }))
            .count(),
        8
    );
    assert!(CavernBackground::from_rom(&image).is_err());
}

#[test]
fn rejects_unqualified_cell_bits_and_definition_adjustment() {
    for (at, size) in [(0x3000, 4096), (0x5002, 512)] {
        let mut image = fixture();
        let mut data = vec![0; size];
        data[..2].copy_from_slice(&0x0200_u16.to_le_bytes());
        let packet = compression::encode(&data).unwrap();
        image[at..at + packet.len()].copy_from_slice(&packet);
        assert!(CavernBackground::from_rom(&image).is_err());
    }
}
