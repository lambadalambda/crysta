//! Owned-ROM checks of the collision probe table and its bounded source contract.
use assets::maps::collision::{probe_attribute, ProbeTable};
use rom::{Revision, Rom};
use std::path::Path;

fn owned_rom() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("SKIP: owned Japanese ROM absent at {}", path.display());
            return None;
        }
        Err(error) => panic!("cannot read {}: {error}", path.display()),
    };
    let cartridge = Rom::load(&bytes).expect("local dump must authenticate");
    assert_eq!(cartridge.revision(), Revision::Japan);
    Some(cartridge)
}

#[test]
fn the_probe_table_matches_measured_classes_but_does_not_decode_the_resolver() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let table = ProbeTable::from_rom(cartridge.image()).unwrap();
    // `docs/collision.md`: stood on in the sweeps.
    for walkable in [0, 2, 22] {
        assert!(
            table.is_clear(walkable),
            "attribute {walkable} was stood on"
        );
    }
    // Stopped a sustained press.
    for solid in [12, 14, 16, 25] {
        assert!(!table.is_clear(solid), "attribute {solid} stopped a press");
    }
    // Probe verdicts only: nonzero does not imply a fully solid cell.
    for attribute in [5, 8, 21, 29] {
        assert!(!table.is_clear(attribute), "attribute {attribute}");
    }
    assert_eq!(table.entry(6), 0x06);
    assert_eq!(table.entry(7), 0x07);
    assert_eq!(
        table.clear_attributes(),
        vec![0, 1, 2, 17, 19, 20, 22, 23, 24, 30],
        "the whole admitted set, so a changed byte is noticed"
    );
    // The dynamic override selects entry 3 after the handler shifts 6.
    assert!(!table.cell_is_clear(0x8000));
    assert_eq!(probe_attribute(0x8000 | (22 << 9)), 3);
}

#[test]
fn ca_skips_the_accelerated_branch_not_ordinary_stream_selection() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    // COP CA $8E76; COP 2B $4100,$90F3; COP 61. COP operands are not CPU code.
    assert_eq!(
        &image[0x04_8E6C..0x04_8E78],
        &[0x02, 0xCA, 0x76, 0x8E, 0x02, 0x2B, 0x00, 0x41, 0xF3, 0x90, 0x02, 0x61]
    );
    // LDA #6; LSR; AND #$001F; TAX: the override selects entry 3, not 6.
    assert_eq!(
        &image[0xAD9E..0xADA6],
        &[0xA9, 6, 0, 0x4A, 0x29, 0x1F, 0, 0xAA]
    );
    // LDA $80:E85C,X; AND #$00FF; BNE $ADB7.
    assert_eq!(
        &image[0xADA6..0xADAF],
        &[0xBF, 0x5C, 0xE8, 0x80, 0x29, 0xFF, 0, 0xD0, 8]
    );
}

#[test]
fn directional_dispatch_refutes_a_single_passability_partition() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let target = |table: usize, kind: usize| {
        let at = table + kind * 2;
        u16::from_le_bytes([image[at], image[at + 1]])
    };
    // First, O-first, P-first, S-first: each table has 32 word targets.
    for base in [0xD542, 0xD8E8, 0xDC60, 0xDFDC] {
        for pair in 0..4 {
            let table = base + pair * 64;
            assert_eq!(target(table, 5), target(table, 16), "5 is P geometry");
            assert_eq!(target(table, 21), target(table, 12), "21 is S geometry");
            assert_eq!(
                target(table, 29) == target(table, 0),
                table != 0xE09C,
                "29 differs from Open in Right S-first dispatch"
            );
        }
    }
    // Type8 is neither globally open nor globally solid. Up's first handler
    // tests $097C bit2; Down's first handler shares Open's target.
    assert_eq!(target(0xD542, 8), 0xD506);
    assert_ne!(target(0xD542, 8), target(0xD542, 0));
    assert_eq!(target(0xD8E8, 8), target(0xD8E8, 0));
    assert_eq!(&image[0xD506..0xD50C], &[0xAD, 0x7C, 0x09, 0x89, 4, 0]);
}
