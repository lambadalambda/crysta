//! Optional integration checks against the user-provided Japanese ROM.
//!
//! The test skips on clean checkouts and never embeds or writes ROM bytes.

use disasm::{DispatchSource, PointerEncoding, RomMap};
use rom::{Revision, Rom, RuntimeRomAddress};

fn local_japanese_rom() -> Option<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local/Tenchi Souzou (Japan).sfc");
    std::fs::read(path).ok()
}

#[test]
fn japanese_rom_matches_map_and_resolves_known_dispatch() {
    let Some(image) = local_japanese_rom() else {
        eprintln!("skipping: local Japanese dump not present");
        return;
    };
    let rom = Rom::load(&image).expect("known-good Japanese dump must load");
    assert_eq!(rom.revision(), Revision::Japan);

    let map = RomMap::built_in_japan().expect("built-in Japanese ROM map must validate");
    map.validate_rom(&rom)
        .expect("ROM map must match the authenticated image");

    let cop_targets = map
        .resolve_dispatch("cop_services", &rom)
        .expect("every declared COP pointer must resolve to its exact entry start");
    assert_eq!(cop_targets.len(), 125);
    assert!(cop_targets
        .iter()
        .enumerate()
        .all(|(index, target)| target.index == u32::try_from(index).unwrap()));

    let boundary = u16::from_le_bytes(
        rom.image()[0x84AC..0x84AE]
            .try_into()
            .expect("two-byte word after COP table"),
    );
    assert_eq!(boundary, 0x109A);
    assert!(RuntimeRomAddress::new(0x80_0000 | u32::from(boundary)).is_err());

    let dispatch = map
        .indirect_dispatch_by_id("top_level_state")
        .expect("top-level mutable dispatch metadata");
    let DispatchSource::MutableMemory {
        address,
        pointer_encoding,
    } = dispatch.source
    else {
        panic!("top-level state dispatch must read mutable memory");
    };
    assert_eq!(address.value(), 0x7E_049E);
    assert_eq!(
        pointer_encoding,
        PointerEncoding::BankLocalU16 { runtime_bank: 0x80 }
    );
    assert_eq!(dispatch.targets.len(), 1);
    let target = map
        .entry_by_id(&dispatch.targets[0].entry_id)
        .expect("declared top-level target");
    assert_eq!(target.id, "top_level_handler_805d");
    assert_eq!(target.runtime.value(), 0x80_805D);
}
