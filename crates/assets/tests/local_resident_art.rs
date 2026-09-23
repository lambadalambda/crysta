//! A record-driven art loader must reproduce the frozen house roster exactly.
//!
//! `HouseScenes` names nine residents by hand-written profile. The free-roam
//! runtime has only spawn records, so the loader that follows a record's
//! descriptor and its script's pose selection is held to the frozen roster:
//! same composition bytes, palette, palette base, position and pose.

use assets::maps::actors::SpawnList;
use assets::maps::scripts::EventFlags;
use assets::sprites::{
    HouseActor, HousePoseKey, HouseScenes, PandoraSprites, RecordRefusal, ResidentPose,
};
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
        // The whole pose list, not just the setup frame: the frozen loader
        // keeps one or four records per resident, and the walked one must
        // find the same records with the same durations.
        assert_eq!(actor.frames().len(), frozen.frames().len(), "{offset:#08x}");
        for (walked, kept) in actor.frames().iter().zip(frozen.frames()) {
            assert_eq!(walked.duration(), kept.duration(), "{offset:#08x}");
            assert_eq!(walked.facing(), kept.facing(), "{offset:#08x}");
            assert_eq!(
                walked.composition().source_bytes(),
                kept.composition().source_bytes(),
                "{offset:#08x}"
            );
        }
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
fn exterior_bird_root_and_three_reuses_match_qualified_pandora_art() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let decoded = decode_map(image, 0x000A, &new_game());
    let list = SpawnList::from_rom(image, 0x000A).unwrap();
    // Descriptor $83:ED37 already has a native witness at map $000A,
    // selector 1: pandora-scene-qualification/reference.json, town-gap-up-rest.
    let pandora = PandoraSprites::from_rom(image).unwrap();
    let bird = pandora.get(0x83_ED37).unwrap();
    assert_eq!(&image[0x03_ED3A..0x03_ED3C], &[0x22, 0]);
    for offset in [0x03_8A19, 0x03_8A23, 0x03_8A2D, 0x03_8A37] {
        let index = list
            .records()
            .iter()
            .position(|r| r.offset() == offset)
            .unwrap();
        let actor = decoded[index].as_ref().unwrap_or_else(|error| {
            panic!("exterior bird {offset:#08x}, descriptor $83:ED37: {error}")
        });
        assert_eq!(actor.palette_base(), bird.palette_base());
        assert_eq!(actor.palette(), bird.palette());
        assert_eq!(actor.graphics().len(), bird.graphics().len());
        assert!(actor
            .graphics()
            .iter()
            .zip(bird.graphics())
            .all(|(a, b)| a.pixels() == b.pixels()));
        for selector in 0..=8 {
            let expected = bird.list(selector).unwrap();
            for hflip in [false, true] {
                let frames = actor.sequence(selector, hflip).unwrap();
                assert_eq!(frames.len(), expected.frames().len());
                for (index, (actual, native)) in frames.iter().zip(expected.frames()).enumerate() {
                    assert_eq!(actual.key(), native.key());
                    assert_eq!(actual.duration(), native.duration());
                    assert_eq!(
                        Some(actual.facing()),
                        expected.effective_facing(index, hflip)
                    );
                    assert_eq!(
                        actual.source_composition().source_bytes(),
                        native.source_composition().source_bytes()
                    );
                    assert_eq!(
                        actual.composition().source_bytes(),
                        native.composition().source_bytes()
                    );
                }
            }
        }
    }
}

#[test]
fn a_reuse_after_a_refused_record_is_refused_not_given_the_wrong_body() {
    // Mutate the now-qualified bird descriptor to unsupported mode $0021.
    // Its three reuses must not reach past it to the last decoded body.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let mut mutated = cartridge.image().to_vec();
    mutated[0x03_ED3A] = 0x21;
    let image = mutated.as_slice();
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

#[test]
fn elle_decodes_on_the_shared_record_path() {
    // Elle's `$00` record shares `$80:F51A` with `$01` records; her descriptor
    // `$83:F881` is mode `$A0` (class 0, common movement, streamed sprite),
    // palette `$90`, graphics `$30`. The twin `$83:F88E` is mode `$20`.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut fresh = vec![0u8; 512];
    fresh[251 / 8] |= 1 << (251 % 8);
    let list = SpawnList::from_rom(image, 0x000F).unwrap();
    let decoded = decode_map(image, 0x000F, &fresh);
    let index = list
        .records()
        .iter()
        .position(|record| record.offset() == 0x03_8D36)
        .expect("Elle's record");
    assert_eq!(list.records()[index].opcode(), 0);
    let elle = decoded[index].as_ref().expect("Elle's art");
    assert_eq!(elle.initial(), 2, "her header's first pose");
}
