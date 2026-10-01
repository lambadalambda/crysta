//! The chapter 1 chests (`docs/chests.md`) on both ROMs.

use assets::chests::{chests, Condition, Contents};

fn images() -> Vec<Vec<u8>> {
    ["Tenchi Souzou (Japan).sfc", "Terranigma (E) [!].smc"]
        .iter()
        .filter_map(|name| {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../local")
                .join(name);
            let rom = rom::Rom::load(&std::fs::read(path).ok()?).ok()?;
            Some(rom.image().to_vec())
        })
        .collect()
}

#[test]
fn the_tower_chests_hold_what_the_table_says() {
    for image in images() {
        let first = chests(&image, 0x103);
        assert_eq!(first.len(), 1);
        assert_eq!(
            (first[0].cell, first[0].contents, first[0].opened),
            ((12, 37), Contents::Item(0x10), 0x580)
        );
        assert_eq!(first[0].condition, Condition::None);
        assert_eq!(chests(&image, 0x108)[0].contents, Contents::Gems(30));
        let seal = chests(&image, 0x112)[0];
        assert!(seal.fanfare() && seal.contents == Contents::Item(0x59));
        let two: Vec<_> = chests(&image, 0x119)
            .iter()
            .map(|chest| chest.cell)
            .collect();
        assert_eq!(two, [(5, 24), (10, 24)]);
        assert!(chests(&image, 0x0F).is_empty());
    }
}
