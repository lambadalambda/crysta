//! Opt-in comparison: native data is an oracle, never input to animation decoding.
use super::{Bgr555, CrystaAnimation, Tile4bpp};
use crate::{
    graphics::{self, IndexedPixel},
    maps::visual::StaticBackground,
};
use std::{fs, path::PathBuf};

#[test]
#[allow(clippy::too_many_lines)] // one opt-in source -> render -> native comparison
#[ignore = "requires CRYSTA_ANIMATION_ROM and CRYSTA_ANIMATION_CAPTURES; see qualification README"]
fn owned_rom_river_cycle_and_native_membership() {
    let bytes = fs::read(
        std::env::var("CRYSTA_ANIMATION_ROM")
            .expect("set CRYSTA_ANIMATION_ROM to the owned Japanese ROM"),
    )
    .expect("read CRYSTA_ANIMATION_ROM");
    let rom = rom::Rom::load(&bytes).unwrap();
    assert_eq!(rom.revision(), rom::Revision::Japan);
    let image = rom.image();
    let animation = CrystaAnimation::from_rom(image).unwrap();
    let base = StaticBackground::from_rom(image, 0xa).unwrap();
    let snapshot = |age| {
        let mut tiles = base.tiles().to_vec();
        let mut colors = *base.palette();
        animation.apply(age, &mut tiles, &mut colors).unwrap();
        (tiles, colors)
    };
    let rgb = |x: usize, y: usize, tiles: &[Tile4bpp], colors: &[Bgr555; 128]| {
        let cell = base.layer().cells()[y / 16 * base.layer().width() + x / 16];
        match graphics::sample_metatile(
            &base.metatiles()[usize::from(cell.raw() & 511)],
            tiles,
            x % 16,
            y % 16,
        )
        .unwrap()
        {
            IndexedPixel::Transparent => None,
            IndexedPixel::Opaque { palette_index, .. } => {
                Some(colors[usize::from(palette_index)].rgb8())
            }
        }
    };
    // Actual river center, stationary camera/player; include repeated and changed phases.
    let river = |age| {
        let (tiles, colors) = snapshot(age);
        (352..600)
            .map(|y| rgb(672, y, &tiles, &colors))
            .collect::<Vec<_>>()
    };
    let phases = [1, 4, 11, 32].map(river);
    assert_eq!(phases[0], phases[1]); // source delay 5 means six scheduler ticks
    for i in [2, 3] {
        let changes = phases[0]
            .iter()
            .zip(&phases[i])
            .filter(|(a, b)| a != b)
            .count();
        assert!(changes > 0);
        println!(
            "river x672 y352..599: age1 -> age{} changes {changes}/248 RGB pixels",
            [1, 4, 11, 32][i]
        );
    }
    for age in 0..42 {
        assert_eq!(river(age), river(age + 42));
    }
    for age in [
        3,
        6,
        11,
        32,
        63,
        64,
        527,
        528,
        u64::MAX - animation.period(),
    ] {
        assert_eq!(snapshot(age), snapshot(age + animation.period()));
    }
    // Verify which animation destinations are actually used by the first logical background (hardware BG2).
    let mut used = [0; 4];
    for cell in base.layer().cells() {
        for word in &base.metatiles()[usize::from(cell.raw() & 511)] {
            let tile = usize::from(word.tile_index());
            used[0] += usize::from((9..13).contains(&tile));
            used[1] += usize::from((496..512).contains(&tile));
            used[2] += usize::from((word.raw() >> 10) & 7 == 6);
            used[3] += usize::from((word.raw() >> 10) & 7 == 7);
        }
    }
    assert_eq!(used, [8, 0, 0, 1286]);
    println!(
        "First-background tile-word occurrences: graphics0={}, graphics1={}, palette6={}, palette7={}",
        used[0], used[1], used[2], used[3]
    );

    let root = PathBuf::from(
        std::env::var("CRYSTA_ANIMATION_CAPTURES")
            .expect("set CRYSTA_ANIMATION_CAPTURES to the retained journey captures (see README)"),
    );
    for label in [
        "town-west-rest",
        "town-north-rest",
        "town-gap-rest",
        "town-gap-up-rest",
        "town-door-align-rest",
    ] {
        let wram = fs::read(root.join(format!("{label}.wram"))).unwrap();
        assert_eq!(&wram[0x47e..0x480], &[10, 0]);
        let vram = fs::read(root.join(format!("{label}.vram"))).unwrap();
        let cgram = fs::read(root.join(format!("{label}.cgram"))).unwrap();
        let native_tiles = graphics::decode_tiles_4bpp(&vram[..0x6000]).unwrap();
        let native_colors: Vec<_> = cgram[..256]
            .chunks_exact(2)
            .map(|b| Bgr555::new(u16::from_le_bytes([b[0], b[1]])))
            .collect();
        // Search one steady-state period. Matching ages are witnesses, NOT a
        // fitted native frame offset. No production data comes from this search.
        let matches: Vec<_> = (animation.period()..2 * animation.period())
            .filter(|&age| {
                animation
                    .tile_updates(age)
                    .all(|(at, tiles)| tiles == &native_tiles[at..at + tiles.len()])
                    && animation
                        .palette_updates(age)
                        .all(|(at, colors)| colors == &native_colors[at..at + colors.len()])
            })
            .collect();
        assert!(
            !matches.is_empty(),
            "{label}: no complete joint source-state match"
        );
        let (tiles, _) = snapshot(matches[0]);
        assert_eq!(
            tiles, native_tiles,
            "entire base+animated BG graphics plane"
        );
        println!("{label}: {} complete joint matches; age residues {:?}; all 768 tiles and all 24 animated colors match", matches.len(), matches.iter().map(|a| a % animation.period()).collect::<Vec<_>>());
    }
}
