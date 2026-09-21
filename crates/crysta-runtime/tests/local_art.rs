//! ROM-backed checks that sprite art rasterizes for the player and residents.

use assets::maps::actors::SpawnList;
use assets::maps::scripts::EventFlags;
use crysta_runtime::art::{residents_art, ArkAtlas, Placeholder};
use crysta_runtime::residents::{residents, Resident};
use crysta_runtime::world::World;
use crysta_runtime::MAPS;
use rom::{Revision, Rom};
use room_core::{AnimationSet, Direction};
use std::path::Path;

fn owned_rom() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join("Tenchi Souzou (Japan).sfc");
    let bytes = std::fs::read(path).ok()?;
    let cartridge = Rom::load(&bytes).expect("local dump must authenticate");
    (cartridge.revision() == Revision::Japan).then_some(cartridge)
}

/// The measured new-game flag state: 32 and 251 are set.
fn new_game() -> Vec<u8> {
    let mut bitmap = vec![0u8; 512];
    bitmap[32 / 8] |= 1 << (32 % 8);
    bitmap[251 / 8] |= 1 << (251 % 8);
    bitmap
}

#[test]
fn every_ordinary_player_frame_rasterizes_with_pixels() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let atlas = ArkAtlas::from_rom(cartridge.image()).unwrap();
    // Three standing, eighteen walking, and the seven horizontal ones again
    // mirrored.
    assert_eq!(atlas.len(), 28);
    let mut world = World::enter(cartridge.image(), 0x000B, 120, 128).unwrap();
    for direction in [
        Direction::Down,
        Direction::Up,
        Direction::Left,
        Direction::Right,
    ] {
        for _ in 0..60 {
            world.step(Some(direction));
            let key = world.animation();
            let raster = atlas.frame(key);
            assert!(raster.is_visible(), "{key:?} has no opaque pixel");
            // A body stands on its origin: the frame reaches up from it and
            // never far below.
            assert!(raster.offset.1 < 0 && raster.offset.1 >= -40, "{key:?}");
            assert!(raster.width <= 32 && raster.height <= 40, "{key:?}");
        }
    }
}

#[test]
fn walking_animates_and_releasing_the_direction_stands() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let mut world = World::enter(cartridge.image(), 0x000B, 120, 128).unwrap();
    assert_eq!(world.animation().set, AnimationSet::Standing);
    let mut records = std::collections::BTreeSet::new();
    for _ in 0..60 {
        world.step(Some(Direction::Right));
        let key = world.animation();
        if key.set == AnimationSet::Walking {
            records.insert(key.record);
            assert_eq!(key.sequence, 2, "horizontal");
            assert!(!key.mirror_x, "right is the unmirrored horizontal");
        }
    }
    assert!(
        records.len() >= 4,
        "walking cycles its records: {records:?}"
    );
    for _ in 0..4 {
        world.step(Some(Direction::Left));
    }
    assert!(
        world.animation().mirror_x,
        "left mirrors the horizontal frames"
    );
    for _ in 0..4 {
        world.step(None);
    }
    assert_eq!(world.animation().set, AnimationSet::Standing);
    assert_eq!(world.animation().record, 0);
}

#[test]
fn the_documented_resident_has_art_and_the_slice_mostly_does() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let flags = new_game();
    let present = residents(image, 0x000B, EventFlags::Bitmap(&flags)).unwrap();
    let art = residents_art(image, 0x000B, &present, EventFlags::Bitmap(&flags));
    let index = present
        .iter()
        .position(|resident| resident.position == (120, 112))
        .unwrap();
    let raster = art[index].as_ref().expect("the documented resident's art");
    assert!(raster.is_visible());
    // A body stands on its origin, reaching up from it.
    assert!(raster.offset.1 < 0, "{:?}", raster.offset);
    assert!(raster.width <= 64 && raster.height <= 64);

    let (mut drawn, mut invisible, mut placeholders, mut total) = (0, 0, 0, 0);
    for map in MAPS {
        let present = residents(image, map, EventFlags::Bitmap(&flags)).unwrap_or_default();
        for entry in residents_art(image, map, &present, EventFlags::Bitmap(&flags)) {
            total += 1;
            match entry {
                Ok(_) => drawn += 1,
                Err(Placeholder::Invisible) => invisible += 1,
                Err(_) => placeholders += 1,
            }
        }
    }
    eprintln!(
        "resident art: {drawn} drawn, {invisible} invisible, {placeholders} placeholders, of {total}"
    );
    assert!(drawn >= 9, "at least the frozen nine draw");
}

#[test]
fn a_reuse_after_a_refused_record_is_a_placeholder_not_the_wrong_body() {
    // Map $000A: the record at $83:8A19 has a descriptor mode the loader does
    // not qualify, and the three records after it reuse its resource. They
    // must not be handed the body of the record before it.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let flags = new_game();
    // Every record, whether the flags would install it or not: the reuse
    // chain is a property of the list, not of who is present.
    let everyone: Vec<Resident> = SpawnList::from_rom(image, 0x000A)
        .unwrap()
        .records()
        .iter()
        .map(|record| Resident {
            position: record.origin(),
            record: record.offset(),
            script: record.script(),
        })
        .collect();
    let art = residents_art(image, 0x000A, &everyone, EventFlags::Bitmap(&flags));
    let mut seen = 0;
    for (resident, entry) in everyone.iter().zip(&art) {
        if [0x03_8A23, 0x03_8A2D, 0x03_8A37].contains(&resident.record) {
            assert_eq!(
                *entry,
                Err(Placeholder::PredecessorRefused),
                "{:#08x}",
                resident.record
            );
            seen += 1;
        }
    }
    assert_eq!(seen, 3);
    // And the reuse before the refused record still resolves.
    let fine = everyone
        .iter()
        .position(|resident| resident.record == 0x03_8A05)
        .unwrap();
    assert!(art[fine].is_ok(), "{:?}", art[fine]);
}
