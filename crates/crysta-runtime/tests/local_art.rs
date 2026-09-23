//! ROM-backed checks that sprite art rasterizes for the player and residents.

use assets::maps::actors::SpawnList;
use assets::maps::scripts::EventFlags;
use crysta_runtime::art::{residents_art, ArkAtlas, Placeholder, Raster};
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
    // Room C: B's Elder speaks on arrival and holds the world.
    let mut world = World::enter(cartridge.image(), 0x000C, 136, 300).unwrap();
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
    let art = residents_art(
        image,
        0x000B,
        &present,
        EventFlags::Bitmap(&flags),
        EventFlags::Bitmap(&flags),
    );
    let index = present
        .iter()
        .position(|resident| resident.position == (120, 112))
        .unwrap();
    let body = art[index].as_ref().expect("the documented resident's art");
    let animation = body
        .animation(body.initial(), false)
        .expect("the setup sequence");
    let raster = animation.frame_at(0);
    assert!(raster.is_visible());
    // A body stands on its origin, reaching up from it.
    assert!(raster.offset.1 < 0, "{:?}", raster.offset);
    assert!(raster.width <= 64 && raster.height <= 64);

    let (mut drawn, mut invisible, mut placeholders, mut total) = (0, 0, 0, 0);
    for map in MAPS {
        let present = residents(image, map, EventFlags::Bitmap(&flags)).unwrap_or_default();
        for entry in residents_art(
            image,
            map,
            &present,
            EventFlags::Bitmap(&flags),
            EventFlags::Bitmap(&flags),
        ) {
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
fn exterior_bird_root_and_three_reuses_draw_without_placeholders() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let flags = new_game();
    let present = residents(image, 0x000A, EventFlags::Bitmap(&flags)).unwrap();
    let art = residents_art(
        image,
        0x000A,
        &present,
        EventFlags::Bitmap(&flags),
        EventFlags::Bitmap(&flags),
    );
    for offset in [0x03_8A19, 0x03_8A23, 0x03_8A2D, 0x03_8A37] {
        let index = present.iter().position(|r| r.record == offset).unwrap();
        let body = art[index].as_ref().unwrap_or_else(|error| {
            panic!("exterior bird {offset:#08x}, descriptor $83:ED37: {error:?}")
        });
        for selector in 0..=8 {
            for hflip in [false, true] {
                let animation = body.animation(selector, hflip).unwrap();
                assert!(!animation.frames.is_empty());
                assert!(
                    animation.frames.iter().all(Raster::is_visible),
                    "bird {offset:#08x}, selector {selector}"
                );
            }
        }
    }
}

#[test]
fn a_reuse_after_a_refused_record_is_a_placeholder_not_the_wrong_body() {
    // Mutate the now-qualified bird descriptor to unsupported mode $0021.
    // Its three reuses must not be handed the body of the record before it.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let mut mutated = cartridge.image().to_vec();
    mutated[0x03_ED3A] = 0x21;
    let image = mutated.as_slice();
    let flags = new_game();
    // Every record the stream runs, present after its script or not: the
    // reuse chain is a property of the executed list, not of who is present.
    let everyone: Vec<Resident> = SpawnList::resolve(image, 0x000A, EventFlags::Bitmap(&flags))
        .unwrap()
        .iter()
        .map(|record| Resident {
            position: record.origin(),
            record: record.offset(),
            script: record.script(),
            body: false,
            initial: 0,
            selector: 0,
            hflip: false,
            pose_age: 0,
            walking: false,
            descriptor: None,
            hidden: false,
        })
        .collect();
    let art = residents_art(
        image,
        0x000A,
        &everyone,
        EventFlags::Bitmap(&flags),
        EventFlags::Bitmap(&flags),
    );
    let mut seen = 0;
    for (resident, entry) in everyone.iter().zip(&art) {
        if [0x03_8A23, 0x03_8A2D, 0x03_8A37].contains(&resident.record) {
            assert!(
                matches!(entry, Err(Placeholder::PredecessorRefused)),
                "{:#08x}: {entry:?}",
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

#[test]
fn a_four_record_resident_cycles_by_duration() {
    // Map $000C's residents have four raw countdown-7 records: eight ticks
    // per record, 32 ticks per cycle (decrement before testing negative).
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let flags = new_game();
    let present = residents(image, 0x000C, EventFlags::Bitmap(&flags)).unwrap();
    let art = residents_art(
        image,
        0x000C,
        &present,
        EventFlags::Bitmap(&flags),
        EventFlags::Bitmap(&flags),
    );
    let animated = art
        .iter()
        .filter_map(|entry| entry.as_ref().ok())
        .flat_map(|body| (0..8u8).filter_map(move |selector| body.animation(selector, false).ok()))
        .find(|animation| animation.frames.len() == 4)
        .expect("a four-record resident");
    assert_eq!(animated.durations, vec![7, 7, 7, 7]);
    assert!(std::ptr::eq(
        animated.frame_at(0),
        &raw const animated.frames[0]
    ));
    assert!(std::ptr::eq(
        animated.frame_at(7),
        &raw const animated.frames[0]
    ));
    assert!(std::ptr::eq(
        animated.frame_at(8),
        &raw const animated.frames[1]
    ));
    assert!(std::ptr::eq(
        animated.frame_at(31),
        &raw const animated.frames[3]
    ));
    assert!(std::ptr::eq(
        animated.frame_at(32),
        &raw const animated.frames[0]
    ));
    // A single zero-countdown list repeats the same raster each tick.
    let held = art
        .iter()
        .filter_map(|entry| entry.as_ref().ok())
        .flat_map(|body| (0..8u8).filter_map(move |selector| body.animation(selector, false).ok()))
        .find(|animation| animation.durations == [0])
        .expect("a held resident");
    assert!(std::ptr::eq(held.frame_at(1000), &raw const held.frames[0]));
}

#[test]
fn a_walking_resident_shows_walking_sequences_the_packet_holds() {
    // The map-$000D wanderer's packet must hold sequences 0..=5: the three
    // standing and three walking ones a running script selects.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let flags = new_game();
    let present = residents(image, 0x000D, EventFlags::Bitmap(&flags)).unwrap();
    let art = residents_art(
        image,
        0x000D,
        &present,
        EventFlags::Bitmap(&flags),
        EventFlags::Bitmap(&flags),
    );
    let index = present
        .iter()
        .position(|resident| resident.record == 0x03_8CB4)
        .unwrap();
    let body = art[index].as_ref().unwrap();
    for selector in 0..6u8 {
        let animation = body
            .animation(selector, selector == 2 || selector == 5)
            .unwrap_or_else(|e| panic!("sequence {selector}: {e}"));
        assert!(
            animation.frames.iter().all(Raster::is_visible),
            "sequence {selector}"
        );
    }
}

#[test]
fn a_roster_keeps_its_bodies_after_a_conversation_changes_the_flags() {
    // The roster spawned under the entry flags; a conversation may flip a
    // flag that sends the spawn stream down another branch. Art for the
    // same roster must still find every record in the entry's order.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let entry = new_game();
    let later = vec![0xFFu8; 512];
    let mut checked = 0;
    for map in 0x000A..=0x0021 {
        let present = residents(image, map, EventFlags::Bitmap(&entry)).unwrap();
        let art = residents_art(
            image,
            map,
            &present,
            EventFlags::Bitmap(&entry),
            EventFlags::Bitmap(&later),
        );
        for (resident, body) in present.iter().zip(&art) {
            if resident.body {
                checked += 1;
                assert!(
                    !matches!(body, Err(Placeholder::Invisible)),
                    "map {map:#06x} record {:06X}",
                    resident.record
                );
            }
        }
    }
    assert!(checked > 20);
}

#[test]
fn yomi_draws_in_the_weapons_room_and_the_spears_display_does_not_borrow_it() {
    // Yomi's descriptor `$83:F8A8` has mode `$0023`; the spear's display
    // `$83:957C` reuses it but sets its own art base (`$89:D9FE`: `COP B2`,
    // `COP D8 $A2C000`), so borrowing Yomi's body would be wrong.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let flags = new_game();
    let present = residents(image, 0x0042, EventFlags::Bitmap(&flags)).unwrap();
    let art = residents_art(
        image,
        0x0042,
        &present,
        EventFlags::Bitmap(&flags),
        EventFlags::Bitmap(&flags),
    );
    let of = |record: usize| {
        present
            .iter()
            .position(|resident| resident.record == record)
            .map(|index| &art[index])
            .expect("present")
    };
    assert!(of(0x03_9572).is_ok(), "Yomi");
    assert!(
        matches!(of(0x03_957C), Err(Placeholder::Refused(_))),
        "the spear's display"
    );
}
