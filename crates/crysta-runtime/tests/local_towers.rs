//! The towers on the underworld map (`docs/tower-entry.md`,
//! `docs/world-map-mode7.md`), on both ROMs.

use crysta_runtime::scene::Presses;
use crysta_runtime::world::{fresh_game_flags, World};
use rom::{Revision, Rom};
use room_core::Direction;
use std::path::Path;

fn load(name: &str, revision: Revision) -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(name);
    let rom = Rom::load(&std::fs::read(path).ok()?).ok()?;
    (rom.revision() == revision).then_some(rom)
}

fn roms() -> Vec<Rom> {
    [
        load("Tenchi Souzou (Japan).sfc", Revision::Japan),
        load("Terranigma (E) [!].smc", Revision::EuropeEnglish),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The flags after the first tower's intro (`$100`, `$90:8F28`).
fn after_the_intro() -> Vec<u8> {
    let mut events = fresh_game_flags();
    events[0x100 / 8] |= 1 << (0x100 % 8);
    events
}

#[test]
fn tower_one_loads_and_ark_walks_in_it() {
    for rom in roms() {
        let mut world = World::enter_with_events(rom.image(), 0x0100, 256, 1007, after_the_intro())
            .unwrap_or_else(|error| panic!("{:?}: {error}", rom.revision()));
        for _ in 0..10 {
            world.update(None, Presses::default()).unwrap();
        }
        assert!(!world.pad_locked(), "{:?}", rom.revision());
        for _ in 0..20 {
            world
                .update(Some(Direction::Up), Presses::default())
                .unwrap();
        }
        assert!(
            world.position().1 < 1007,
            "{:?}: {:?}",
            rom.revision(),
            world.position()
        );
    }
}

#[test]
fn the_underworld_entrance_leads_into_tower_one() {
    // `$81:8CCD`: cell (13,49) 1x2 on `$03`; natively Ark walked up from
    // (216,880) and the map changed on the step from (216,816).
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x0003, 216, 880, after_the_intro()).unwrap();
        for _ in 0..400 {
            let up = (!world.in_transition()).then_some(Direction::Up);
            world.update(up, Presses::default()).unwrap();
            if world.map() == 0x0100 {
                break;
            }
        }
        assert_eq!(
            world.map(),
            0x0100,
            "{:?}: {:?}",
            rom.revision(),
            world.position()
        );
    }
}
