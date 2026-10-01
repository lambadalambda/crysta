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

#[test]
fn the_chest_texts_decode_with_the_item_and_the_gems() {
    // Gems, item, empty, fanfare item, its close, overloaded (JP, EU).
    let sources = [
        [
            0x92_810E, 0x92_812C, 0x92_8164, 0x92_8147, 0x92_8162, 0x92_8096,
        ],
        [
            0x92_8131, 0x92_8151, 0x92_818F, 0x92_816F, 0x92_818D, 0x92_80B0,
        ],
    ];
    for image in images() {
        let europe = assets::layout::per_revision(&image, 0, 1);
        for source in sources[europe] {
            let pages =
                assets::text::HouseDialogue::decode_reading(&image, source, |at| match at {
                    0x09C7 => Some(0x10),
                    0x09CB => Some(0x30),
                    0x09C8 | 0x09CC => Some(0),
                    _ => None,
                });
            assert!(pages.is_ok(), "{source:x}: {pages:?}");
        }
    }
}
