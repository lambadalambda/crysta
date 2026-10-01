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

#[test]
fn ark_walks_in_from_the_bottom_of_tower_one() {
    // `docs/tower-entry.md`: selector `$66` places Ark at (256,1024) and
    // walks him up 17 pixels to (256,1007), the pad locked.
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x0003, 216, 880, after_the_intro()).unwrap();
        while world.map() != 0x0100 {
            let up = (!world.in_transition()).then_some(Direction::Up);
            world.update(up, Presses::default()).unwrap();
        }
        assert_eq!(world.position(), (256, 1024), "{:?}", rom.revision());
        let mut path = Vec::new();
        while world.in_transition() {
            world.update(None, Presses::default()).unwrap();
            path.push(world.position().1);
        }
        assert_eq!(path.last(), Some(&1007), "{:?}", rom.revision());
        assert_eq!(world.position().0, 256);
    }
}

#[test]
fn walking_down_out_of_tower_one_returns_to_the_underworld() {
    // The exit `$81:C2E2` (cells (0,63) 32x2, selector `$55`) takes Ark at
    // y 1025; on `$03` he lands at (216,816) and walks down to (216,832).
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x0100, 256, 1007, after_the_intro()).unwrap();
        for _ in 0..10 {
            world.update(None, Presses::default()).unwrap();
        }
        for _ in 0..400 {
            let down = (!world.in_transition()).then_some(Direction::Down);
            world.update(down, Presses::default()).unwrap();
            if world.map() == 0x0003 && !world.in_transition() {
                break;
            }
        }
        // The plane's own arrival walk (`$84:E337`).
        for _ in 0..20 {
            world.update(None, Presses::default()).unwrap();
        }
        assert_eq!(
            (world.map(), world.position()),
            (0x0003, (216, 832)),
            "{:?}",
            rom.revision()
        );
    }
}

#[test]
fn the_first_visit_pans_down_to_ark_then_he_speaks() {
    // `$90:8F28`: `COP DD` puts the camera 768 above Ark, `COP DE` brings it
    // back at 2 a frame (384 frames), two pages of text, then `COP 29`
    // unlocks the pad (`docs/tower-entry.md`).
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x0100, 256, 1007, fresh_game_flags()).unwrap();
        let mut offsets = Vec::new();
        let mut pages = 0;
        for _ in 0..1000 {
            let reading = world.dialogue().is_some() && !world.typing();
            pages += u32::from(reading);
            let a = Presses {
                confirm: reading,
                ..Presses::default()
            };
            world.update(None, a).unwrap();
            offsets.push(world.camera_offset());
            if !world.pad_locked() {
                break;
            }
        }
        assert!(!world.pad_locked(), "{:?}", rom.revision());
        assert_eq!(offsets[0], (0, -768), "{:?}", rom.revision());
        let panning = offsets
            .iter()
            .take_while(|&&offset| offset != (0, 0))
            .count();
        assert!(
            (383..=387).contains(&panning),
            "{:?}: {panning}",
            rom.revision()
        );
        assert!(offsets
            .windows(2)
            .take(panning - 1)
            .all(|pair| pair[1].1 - pair[0].1 == 2));
        assert_eq!(pages, 2, "{:?}", rom.revision());
    }
}

#[test]
fn tower_ones_statues_and_plaque_stand_and_draw() {
    // `docs/mode4-descriptors.md`: the plaque at (256,888) (`COP B3 8,8`),
    // the statues at (152,992) and, mirrored, (360,992) (`COP B1`, `B7`),
    // all from descriptor `$82:F65A`.
    for rom in roms() {
        let image = rom.image();
        let events = after_the_intro();
        let mut world = World::enter_with_events(image, 0x0100, 256, 1007, events.clone()).unwrap();
        for _ in 0..5 {
            world.update(None, Presses::default()).unwrap();
        }
        let present = world.residents();
        let placed: Vec<_> = present
            .iter()
            .filter(|resident| resident.descriptor.is_some())
            .map(|resident| (resident.position, resident.hflip))
            .collect();
        assert_eq!(
            placed,
            [((256, 888), false), ((152, 992), false), ((360, 992), true)],
            "{:?}",
            rom.revision()
        );
        let art = crysta_runtime::art::residents_art(
            image,
            0x0100,
            present,
            assets::maps::scripts::EventFlags::Bitmap(&events),
            assets::maps::scripts::EventFlags::Bitmap(world.events()),
        );
        for (resident, body) in present.iter().zip(&art) {
            if resident.descriptor.is_none() {
                continue;
            }
            let body = body
                .as_ref()
                .unwrap_or_else(|error| panic!("{:?}: {error:?}", rom.revision()));
            let animation = body.animation(body.initial(), resident.hflip).unwrap();
            let raster = animation.frame_at(0);
            assert!(raster.is_visible());
            assert!(
                raster.height >= 96,
                "{:?}: {}",
                rom.revision(),
                raster.height
            );
        }
    }
}

#[test]
fn every_tower_floor_loads() {
    // `docs/underworld-inventory.md`; the light room `$106` comes with the
    // towers' tops.
    for rom in roms() {
        for &map in &crysta_runtime::TOWER_MAPS {
            World::enter_with_events(rom.image(), map, 128, 128, after_the_intro())
                .unwrap_or_else(|error| panic!("{:?} {map:#x}: {error}", rom.revision()));
        }
        assert!(crysta_runtime::TOWER_MAPS.len() >= 36);
    }
}

#[test]
fn the_tower_door_leads_to_the_first_floor() {
    // `$81:C2EE`: cell (15,54) 2x2, selector `$62` -> `$101`, natively
    // arriving at (128,623) (`tools/tower-approach-qualification/TOWER.md`).
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x0100, 256, 1007, after_the_intro()).unwrap();
        for _ in 0..600 {
            let up = (!world.in_transition()).then_some(Direction::Up);
            world.update(up, Presses::default()).unwrap();
            if world.map() == 0x0101 && !world.in_transition() {
                break;
            }
        }
        assert_eq!(
            (world.map(), world.position()),
            (0x0101, (128, 623)),
            "{:?}",
            rom.revision()
        );
    }
}

#[test]
fn the_first_chest_gives_a_bulb_once() {
    // `docs/chests.md`: `$103` cell (12,37) holds S.Bulb `$10`, flag `$580`;
    // Ark faces Up from (200,624), the cell 24 pixels above his feet.
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x0103, 200, 624, after_the_intro()).unwrap();
        world.face(Direction::Up);
        for _ in 0..4 {
            world.update(None, Presses::default()).unwrap();
        }
        let a = Presses {
            confirm: true,
            ..Presses::default()
        };
        world.update(None, a).unwrap();
        let mut shown = false;
        for _ in 0..200 {
            shown |= world.dialogue().is_some();
            let reading = world.dialogue().is_some() && !world.typing();
            world
                .update(None, if reading { a } else { Presses::default() })
                .unwrap();
        }
        assert!(shown, "{:?}", rom.revision());
        assert!(world.items().contains(&0x10), "{:?}", rom.revision());
        let flag = |world: &World<'_>| world.events()[0x580 / 8] & (1 << (0x580 % 8)) != 0;
        assert!(flag(&world), "{:?}", rom.revision());
        assert!(world.patched_cells().contains(&(12, 37, 0xF1)));
        // A second A on the open lid does nothing.
        world.update(None, a).unwrap();
        world.update(None, Presses::default()).unwrap();
        assert!(world.dialogue().is_none(), "{:?}", rom.revision());
        // Opened, it stays open and gives nothing more.
        let events = world.events().to_vec();
        let mut again = World::enter_with_events(rom.image(), 0x0103, 200, 624, events).unwrap();
        again.update(None, Presses::default()).unwrap();
        assert!(again.patched_cells().contains(&(12, 37, 0xF1)));
    }
}
