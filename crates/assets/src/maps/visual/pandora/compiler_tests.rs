use super::*;
use crate::compression;

fn fixture() -> Vec<u8> {
    let mut image = super::tests::camera_fixture();
    let room = super::super::tests::exterior_fixture();
    image.resize(room.len(), 0);
    // Preserve source camera tables while importing synthetic relocated resources.
    for (at, byte) in room.into_iter().enumerate() {
        if !(0x28000..0x40000).contains(&at) && !(0x16_0000..0x17_0000).contains(&at) {
            image[at] = byte;
        }
    }
    recipes::test_setup(&mut image);
    image
}

#[test]
fn compiler_returns_source_resources_full_grid_pixels_and_separate_policies() {
    let image = fixture();
    for id in [0xa, 0xe, 0x13, 0x20, 0x21, 0x41, 0x42, 0x43, 0x44] {
        let p = PandoraBackground::from_rom(&image, id).unwrap();
        let b = p.background();
        assert_eq!(p.attributed_grid().len(), b.layer().cells().len());
        assert_eq!(p.policies().grid, GridPolicy::FullSourceAttributed);
        assert_eq!(p.policies().admission, AdmissionPolicy::CallerSuppliedHalo);
        assert_eq!(
            p.policies().dynamic_phase,
            DynamicPhasePolicy::CallerAppliedSourcePatches
        );
        for resource in b.resources() {
            assert_eq!(resource.source_bytes(), &image[resource.source_range()]);
        }
        assert!(b
            .pixel(b.layer().width() * 16 - 1, b.layer().height() * 16 - 1)
            .is_ok());
        assert!(b.pixel(b.layer().width() * 16, 0).is_err());
        assert_eq!(
            p.initialization(),
            if id >= 0x41 {
                Initialization::Map21ThenController41Tour
            } else {
                Initialization::MapLoad
            }
        );
    }
    for id in [0xb, 0xc, 0xd, 0xf, 0x10, 0x11, 0x128, 0x45] {
        assert!(PandoraBackground::from_rom(&image, id).is_err());
    }
    for id in [0xe, 0x13, 0x20, 0x21, 0x41, 0x42, 0x43, 0x44] {
        assert!(super::super::StaticBackground::from_rom(&image, id).is_err());
    }
}

#[test]
fn controller_transfers_are_bank_first_ordered_and_inherited() {
    let image = fixture();
    let p = PandoraBackground::from_rom(&image, 0x44).unwrap();
    let b = p.background();
    assert_eq!(b.tiles().len(), 384);
    // Seven transfers follow inherited shared colors. Overwrites must win.
    assert_eq!(b.palette()[0].raw(), 0x1234);
    assert_eq!(b.palette()[15].raw(), 0x1243);
    assert_eq!(b.palette()[16].raw(), 1);
    assert_eq!(b.palette()[127].raw(), 1);
    for (destination, color) in [(24, 2), (40, 3), (56, 4), (72, 5), (88, 4), (104, 6)] {
        assert_eq!(b.palette()[destination].raw(), color);
    }
    assert_eq!(b.palette()[32].raw(), 1);
    assert_eq!(b.resources()[0].source_range().start, 0x26_8000);
    assert_eq!(b.resources()[1].source_range().start, 0x27_8000);
    for (id, offset, cell) in [
        (0x41, 0x24_9800, 0),
        (0x42, 0x24_c000, 1),
        (0x43, 0x24_c800, 2),
        (0x44, 0x24_d000, 3),
    ] {
        let p = PandoraBackground::from_rom(&image, id).unwrap();
        assert_eq!(p.background().layer().source_range().start, offset);
        assert_eq!(p.background().layer().cells()[0].raw(), cell);
    }
}

#[test]
fn compiler_rejects_all_recipe_controls_and_malformed_payloads() {
    let good = fixture();
    for (id, at) in recipes::test_mutations() {
        let mut image = good.clone();
        image[at] ^= 1;
        assert!(
            PandoraBackground::from_rom(&image, id).is_err(),
            "map {id:x} at {at:x}"
        );
        assert!(PandoraBackground::from_rom(&good[..at], id).is_err());
    }
    for (at, size, id) in [
        (0x26_8000, 0x2fff, 0x41),
        (0x22_8000, 0xfff, 0x41),
        (0x23_8000, 511, 0x41),
    ] {
        let mut image = good.clone();
        let data = compression::encode(&vec![0; size]).unwrap();
        image[at..at + data.len()].copy_from_slice(&data);
        assert!(PandoraBackground::from_rom(&image, id).is_err());
    }
    // A full-sheet reference must not escape the controller's 384 supplied tiles.
    let mut image = good;
    let mut data = vec![0; 4096];
    data[..2].copy_from_slice(&384_u16.to_le_bytes());
    let data = compression::encode(&data).unwrap();
    image[0x22_8000..0x22_8000 + data.len()].copy_from_slice(&data);
    assert!(PandoraBackground::from_rom(&image, 0x41).is_err());
}
