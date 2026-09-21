//! A record-driven art loader must reproduce the frozen house roster exactly.
//!
//! `HouseScenes` names nine residents by hand-written profile. The free-roam
//! runtime has only spawn records, so the loader that follows a record's
//! descriptor and its script's pose selection is held to the frozen roster:
//! same composition bytes, palette, palette base, position and pose.

use assets::maps::actors::SpawnList;
use assets::maps::scripts::EventFlags;
use assets::sprites::{HouseActor, HousePoseKey, HouseScenes, RecordRefusal, ResidentPose};
use rom::{Revision, Rom};
use std::path::Path;

fn owned_rom() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    match std::fs::read(&path) {
        Ok(bytes) => {
            let cartridge = Rom::load(&bytes).unwrap();
            assert_eq!(cartridge.revision(), Revision::Japan);
            Some(cartridge)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping: local Japanese ROM not present");
            None
        }
        Err(e) => panic!("reading {}: {e}", path.display()),
    }
}

/// The measured new-game flag state: 32 and 251 are set.
fn new_game() -> Vec<u8> {
    let mut bitmap = vec![0u8; 512];
    bitmap[32 / 8] |= 1 << (32 % 8);
    bitmap[251 / 8] |= 1 << (251 % 8);
    bitmap
}

/// Decodes a map's records with the pose the walked script selects.
fn decode_map(image: &[u8], map: u16, flags: &[u8]) -> Vec<Result<HouseActor, RecordRefusal>> {
    let list = SpawnList::from_rom(image, map).unwrap();
    HouseActor::from_records(image, map, list.records(), |record| {
        ResidentPose::from_script(image, record, EventFlags::Bitmap(flags)).unwrap()
    })
}

#[test]
fn record_derived_art_matches_every_frozen_resident() {
    // End to end: the pose comes from the walked script, not from the frozen
    // profile, and the art must still be the frozen art. Two residents take
    // the pose fallback: the map-$0011 resident's new-game intro ends in
    // native code, and the map-$000D wanderer loops on $026 until the budget
    // runs out. With every flag set the first reaches its ordinary loop and
    // the second stops at an unaccounted service short of any wait, which
    // leaves the header's setup pose -- what the frozen roster records.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let scenes = HouseScenes::from_rom(image).unwrap();
    let mut checked = 0;
    for frozen in scenes.actors() {
        // The prop is created by a script, not placed by a record.
        if matches!(frozen.setup_frame().key(), HousePoseKey::Direct(_)) {
            continue;
        }
        let offset = usize::try_from(frozen.source_id() & 0x3F_FFFF).unwrap();
        let decoded = decode_map(image, frozen.map_id(), &new_game());
        let list = SpawnList::from_rom(image, frozen.map_id()).unwrap();
        let index = list
            .records()
            .iter()
            .position(|record| record.offset() == offset)
            .unwrap_or_else(|| {
                panic!(
                    "record {offset:#08x} missing from map {:#06x}",
                    frozen.map_id()
                )
            });
        let actor = decoded[index]
            .as_ref()
            .unwrap_or_else(|error| panic!("record {offset:#08x} did not decode: {error}"));
        assert_eq!(actor.source_id(), frozen.source_id());
        assert_eq!(actor.position(), frozen.position(), "{offset:#08x}");
        assert_eq!(actor.palette_base(), frozen.palette_base(), "{offset:#08x}");
        assert_eq!(actor.palette(), frozen.palette(), "{offset:#08x}");
        assert_eq!(actor.selector(), frozen.selector(), "{offset:#08x}");
        assert_eq!(actor.hflip(), frozen.hflip(), "{offset:#08x}");
        assert_eq!(actor.facing(), frozen.facing(), "{offset:#08x}");
        assert_eq!(
            actor.setup_frame().composition().source_bytes(),
            frozen.setup_frame().composition().source_bytes(),
            "{offset:#08x}"
        );
        assert_eq!(
            actor.graphics().len(),
            frozen.graphics().len(),
            "{offset:#08x}"
        );
        assert!(
            actor
                .graphics()
                .iter()
                .zip(frozen.graphics())
                .all(|(a, b)| a.pixels() == b.pixels()),
            "{offset:#08x}: graphics differ"
        );
        checked += 1;
    }
    assert_eq!(checked, 9, "all nine frozen residents");
}

#[test]
fn a_reuse_after_a_refused_record_is_refused_not_given_the_wrong_body() {
    // Map $000A: the record at $83:8A19 has a descriptor mode the loader does
    // not qualify, and the three records after it reuse its resource. The
    // native loader would hand them $8A19's body; this one cannot, and must
    // not reach past it to the last body it did decode.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let decoded = decode_map(image, 0x000A, &new_game());
    let list = SpawnList::from_rom(image, 0x000A).unwrap();
    let at = |offset: usize| {
        let index = list
            .records()
            .iter()
            .position(|record| record.offset() == offset)
            .unwrap();
        &decoded[index]
    };
    assert!(matches!(at(0x03_8A19), Err(RecordRefusal::Invalid(_))));
    for offset in [0x03_8A23, 0x03_8A2D, 0x03_8A37] {
        assert!(
            matches!(at(offset), Err(RecordRefusal::PredecessorRefused)),
            "{offset:#08x}: {:?}",
            at(offset).as_ref().err()
        );
    }
    // The reuses before the refused record still resolve.
    assert!(at(0x03_8A05).is_ok(), "{:?}", at(0x03_8A05).as_ref().err());
    // And a script-only record carries no body at all.
    assert!(matches!(at(0x03_89B8), Err(RecordRefusal::NoDescriptor)));
}

#[test]
fn census_of_records_whose_art_decodes() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let flags = new_game();
    let (mut decoded, mut bodiless, mut poisoned, mut refused, mut total) =
        (0, 0, 0, Vec::new(), 0);
    for map in 0x000A..=0x0021u16 {
        let list = SpawnList::from_rom(image, map).unwrap();
        for (record, result) in list.records().iter().zip(decode_map(image, map, &flags)) {
            total += 1;
            match result {
                Ok(_) => decoded += 1,
                Err(RecordRefusal::NoDescriptor) => bodiless += 1,
                Err(RecordRefusal::PredecessorRefused) => poisoned += 1,
                Err(error) => refused.push(format!("{map:#06x} {:#08x}: {error}", record.offset())),
            }
        }
    }
    eprintln!(
        "resident art: {decoded} of {total} records decode; {bodiless} carry no descriptor, \
         {poisoned} reuse a refused predecessor, {} refused:",
        refused.len()
    );
    for line in &refused {
        eprintln!("  {line}");
    }
    assert!(decoded >= 9, "at least the frozen nine must decode");
}
