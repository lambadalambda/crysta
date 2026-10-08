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
fn no_script_on_the_underworld_map_freezes() {
    // `$03`'s player record (`FD`, header flags `$0400`, `$80:F46E`) is
    // Ark; the horizon band (`$84:E3E6`) and the view's HDMA (`$87:990A`)
    // are the hosts' to draw (`docs/world-map-mode7.md`).
    for rom in roms() {
        let mut events = after_the_intro();
        for flag in 0x20..=0x2D {
            events[flag / 8] |= 1 << (flag % 8);
        }
        let mut world = World::enter_with_events(rom.image(), 0x0003, 216, 880, events).unwrap();
        for _ in 0..60 {
            world.update(None, Presses::default()).unwrap();
        }
        assert_eq!(world.frozen_scripts(), [], "{:?}", rom.revision());
        assert!(world.residents().is_empty(), "{:?}", rom.revision());
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
fn the_spawn_lists_mark_the_tower_floors() {
    // `$048A` bit 15, from each list's header (`$86:957B`): the floors, not
    // the entrances, the light room or the Hole's rim.
    for rom in roms() {
        for (map, floor) in [
            (0x100, false),
            (0x101, true),
            (0x106, false),
            (0x123, true),
            (0x127, false),
        ] {
            let world = World::enter_with_events(rom.image(), map, 128, 128, after_the_intro())
                .unwrap_or_else(|error| panic!("{:?} {map:#x}: {error}", rom.revision()));
            assert_eq!(world.tower_floor(), floor, "{:?} {map:#x}", rom.revision());
        }
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

#[test]
fn ark_takes_the_magirock_on_the_second_floor() {
    // `docs/chests.md` §5: the stone at (744,496) on `$102`; facing it, A
    // lifts it, the text names it, then one more Magirock and its flag.
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x0102, 744, 512, after_the_intro()).unwrap();
        world.face(Direction::Up);
        for _ in 0..4 {
            world.update(None, Presses::default()).unwrap();
        }
        let before = world.residents().len();
        let stone = world
            .residents()
            .iter()
            .position(|resident| resident.position == (744, 496))
            .expect("the stone");
        let number = rom.image()[world.residents()[stone].record + 3];
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
        assert_eq!(world.save_slot().prime_blue(), 1, "{:?}", rom.revision());
        let flag = 0x900 + usize::from(number);
        assert!(world.events()[flag / 8] & (1 << (flag % 8)) != 0);
        assert!(world.residents().len() <= before);
        // Taken, it is not there on the next visit.
        let events = world.events().to_vec();
        let again = World::enter_with_events(rom.image(), 0x0102, 744, 512, events).unwrap();
        let mut again = again;
        for _ in 0..3 {
            again.update(None, Presses::default()).unwrap();
        }
        assert!(
            again
                .residents()
                .iter()
                .zip(0..)
                .all(|(resident, _)| resident.position != (744, 496) || resident.hidden),
            "{:?}",
            rom.revision()
        );
    }
}

#[test]
fn ark_walks_the_tower_tops_floor() {
    // Attribute 1 floors the top (`$105`); the player's tables read it as 0.
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x0105, 504, 400, after_the_intro()).unwrap();
        for _ in 0..20 {
            world
                .update(Some(Direction::Up), Presses::default())
                .unwrap();
        }
        assert!(world.position().1 < 400, "{:?}", rom.revision());
    }
}

#[test]
fn the_four_hiballs_fight_opens_the_door_to_the_light() {
    // `$105`: the hooded guardian's talk sets flag 1, which wakes the Four
    // Hiballs (`$90:9390`); the door (`$90:93E0`) waits for `$0498` to fall
    // to 0, speaks, sets `$114`, then waits for Ark in front of it
    // (`COP 0C`) and leaves for the light room.
    let hiballs = |world: &World<'_>| -> Vec<usize> {
        world
            .residents()
            .iter()
            .filter(|resident| world.foe_life(resident.record).is_some())
            .map(|resident| resident.record)
            .collect()
    };
    let a = Presses {
        confirm: true,
        ..Presses::default()
    };
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x0105, 512, 400, after_the_intro()).unwrap();
        assert_eq!(hiballs(&world).len(), 4, "{:?}", rom.revision());
        world.place(512, 336);
        world.face(Direction::Up);
        world.update(None, Presses::default()).unwrap();
        world.update(None, a).unwrap();
        let mut seen = false;
        for _ in 0..400 {
            let reading = world.dialogue().is_some() && !world.typing();
            world
                .update(None, if reading { a } else { Presses::default() })
                .unwrap();
            seen |= world
                .residents()
                .iter()
                .any(|resident| world.foe_life(resident.record).is_some() && !resident.hidden);
        }
        assert!(world.events()[0] & 2 != 0, "{:?}: flag 1", rom.revision());
        assert!(seen, "{:?}: the Hiballs show", rom.revision());
        assert!(world.frozen_scripts().is_empty(), "{:?}", rom.revision());
        for record in hiballs(&world) {
            world.kill_foe(record);
        }
        let flag = |world: &World<'_>| world.events()[0x114 / 8] & (1 << (0x114 % 8)) != 0;
        for _ in 0..400 {
            let reading = world.dialogue().is_some() && !world.typing();
            world
                .update(None, if reading { a } else { Presses::default() })
                .unwrap();
        }
        assert!(flag(&world), "{:?}: `$114`", rom.revision());
        // Through the door, the light room and the parchment's text to the
        // underworld with flag `$101` (`docs/light-room.md`).
        let mut texts = 0;
        for _ in 0..3000 {
            let up = (!world.in_transition() && world.map() == 0x0105).then_some(Direction::Up);
            let reading = world.dialogue().is_some() && !world.typing();
            texts += u32::from(reading && world.resurrecting());
            world
                .update(up, if reading { a } else { Presses::default() })
                .unwrap();
            if world.map() == 0x0003 && !world.in_transition() {
                break;
            }
        }
        assert_eq!(world.map(), 0x0003, "{:?}", rom.revision());
        assert!(texts > 0, "{:?}: the parchment's text", rom.revision());
        let flag = 0x101;
        assert!(world.events()[flag / 8] & (1 << (flag % 8)) != 0);
        assert_eq!(world.position().0, 216, "{:?}", rom.revision());
    }
}

#[test]
fn the_top_floors_door_and_the_light_rooms_orb_draw() {
    // `docs/mode4-descriptors.md`: the door `$82:F5B2` uploads sheet tiles
    // `$20..` to the same OBJ tiles (`d15 = d16 = $10`) with one palette;
    // the orb `$82:F645` starts in pose 0, its header byte not a list.
    for rom in roms() {
        let image = rom.image();
        let events = after_the_intro();
        for (map, at) in [(0x0105, (512, 400)), (0x0106, (128, 184))] {
            let world = World::enter_with_events(image, map, at.0, at.1, events.clone()).unwrap();
            let art = crysta_runtime::art::residents_art(
                image,
                map,
                world.residents(),
                assets::maps::scripts::EventFlags::Bitmap(&events),
                assets::maps::scripts::EventFlags::Bitmap(world.events()),
            );
            let refused: Vec<_> = world
                .residents()
                .iter()
                .zip(&art)
                .filter(|(_, art)| matches!(art, Err(crysta_runtime::art::Placeholder::Refused(_))))
                .map(|(resident, _)| resident.record)
                .collect();
            assert!(
                refused.is_empty(),
                "{:?} {map:#x}: {refused:x?}",
                rom.revision()
            );
        }
    }
}

#[test]
fn tower_twos_gate_opens_once_tower_one_is_done() {
    // `$90:8F83`: unless flag `$100 + 1` is set, `COP 3F` seals the door
    // cells (16,56), (17,56); Ark's position (`$0954`) gates the rest.
    for rom in roms() {
        for done in [false, true] {
            let mut events = after_the_intro();
            if done {
                events[0x101 / 8] |= 1 << (0x101 % 8);
            }
            let mut world =
                World::enter_with_events(rom.image(), 0x0107, 256, 1007, events).unwrap();
            for _ in 0..600 {
                let up = (!world.in_transition() && !world.pad_locked()).then_some(Direction::Up);
                world.update(up, Presses::default()).unwrap();
                if world.map() != 0x0107 {
                    break;
                }
            }
            let entered = world.map() == 0x0108;
            assert_eq!(entered, done, "{:?}", rom.revision());
        }
    }
}

#[test]
fn ark_pushes_tower_twos_statue_aside() {
    // `docs/tower-two.md`: the statue (`$90:958A`) slides one cell when Ark
    // pushes it from its left for 61 frames, and sets flag `$284`.
    for rom in roms() {
        let mut events = after_the_intro();
        events[0x101 / 8] |= 1 << (0x101 % 8);
        let mut world =
            World::enter_with_events(rom.image(), 0x0108, 128, 400, events.clone()).unwrap();
        world.update(None, Presses::default()).unwrap();
        let statue = world
            .residents()
            .iter()
            .find(|resident| resident.position == (192, 208))
            .unwrap_or_else(|| panic!("{:?}: no statue", rom.revision()))
            .record;
        let mut world = World::enter_with_events(rom.image(), 0x0108, 168, 208, events).unwrap();
        for _ in 0..120 {
            world
                .update(Some(Direction::Right), Presses::default())
                .unwrap();
        }
        let moved = world
            .residents()
            .iter()
            .find(|resident| resident.record == statue)
            .map(|resident| resident.position);
        assert_eq!(moved, Some((208, 208)), "{:?}", rom.revision());
        assert!(world.events()[0x284 / 8] & (1 << (0x284 % 8)) != 0);
    }
}

#[test]
fn a_pushed_block_on_its_mark_sets_its_flag() {
    // `docs/tower-two.md`: on `$10B` the watcher `$90:9677` spawns a block
    // (`$90:FC6E`) at (160,368); pushed Left one cell onto x `$90`, it sets
    // flag `$002`, and the watcher takes its place.
    for rom in roms() {
        let mut events = after_the_intro();
        events[0x101 / 8] |= 1 << (0x101 % 8);
        let mut world = World::enter_with_events(rom.image(), 0x010B, 184, 368, events).unwrap();
        for _ in 0..200 {
            world
                .update(Some(Direction::Left), Presses::default())
                .unwrap();
        }
        assert!(world.events()[0] & 4 != 0, "{:?}: flag 2", rom.revision());
        assert!(
            world.events()[0] & 2 == 0,
            "{:?}: no wrong mark",
            rom.revision()
        );
        assert!(world.frozen_scripts().is_empty(), "{:?}", rom.revision());
    }
}

#[test]
fn tower_twos_switches_open_the_stairs() {
    // `docs/tower-two.md`: the two hidden switches on `$109` answer A from
    // beside them (flags `$286`, `$287`, a cell patch each); with both,
    // the controller sets `$28C` and `$28D` and opens the stairs.
    let a = Presses {
        confirm: true,
        ..Presses::default()
    };
    for rom in roms() {
        let mut events = after_the_intro();
        events[0x101 / 8] |= 1 << (0x101 % 8);
        let mut world = World::enter_with_events(rom.image(), 0x0109, 264, 160, events).unwrap();
        for (x, y) in [(264, 144), (504, 144)] {
            world.place(x, y + 16);
            world.face(Direction::Up);
            for _ in 0..3 {
                world.update(None, Presses::default()).unwrap();
            }
            world.update(None, a).unwrap();
            for _ in 0..30 {
                world.update(None, Presses::default()).unwrap();
            }
        }
        for _ in 0..200 {
            let reading = world.dialogue().is_some() && !world.typing();
            world
                .update(None, if reading { a } else { Presses::default() })
                .unwrap();
        }
        let flag = |n: usize| world.events()[n / 8] & (1 << (n % 8)) != 0;
        assert!(
            flag(0x286) && flag(0x287) && flag(0x28C) && flag(0x28D),
            "{:?}",
            rom.revision()
        );
    }
}

/// Walks Ark right for `steps` frames or until he falls, waits for the
/// fall, then waits `frames` frames more.
fn fall_right(world: &mut World, steps: usize, frames: usize) {
    for _ in 0..steps {
        if world.falling() {
            break;
        }
        world
            .update(Some(Direction::Right), Presses::default())
            .unwrap();
    }
    for _ in 0..120 {
        if world.falling() {
            break;
        }
        world.update(None, Presses::default()).unwrap();
    }
    assert!(world.falling(), "no fall at {:?}", world.position());
    for _ in 0..frames {
        world.update(None, Presses::default()).unwrap();
    }
}

#[test]
fn the_crumbling_row_drops_ark_a_floor_down() {
    // `docs/tower-three.md` §2, §3: `$10F`'s row tiles over the pit are
    // floor (`COP 3F 02`) until Ark comes near, then pits 60 frames later.
    // The exit over rows 0-38 has the conditional destination `$F144`:
    // flag `$1F` -> `$114`.
    for rom in roms() {
        let mut events = after_the_intro();
        events[0x103 / 8] |= 1 << (0x103 % 8);
        let mut world = World::enter_with_events(rom.image(), 0x010F, 168, 528, events).unwrap();
        fall_right(&mut world, 16, 200);
        // The loader clears flags `$00-$1F`, `$1F` with them.
        assert_eq!(world.map(), 0x0114, "{:?}", rom.revision());
        assert_eq!(world.position(), (248, 144), "{:?}", rom.revision());
    }
}

#[test]
fn ark_drops_onto_the_floor_below_and_walks_on() {
    // `docs/tower-three.md` §2: the `FD` record at (15,9) hides Ark, drops
    // him 256 pixels in resource 0's `$13`, lands him in `$14`, then frees
    // the pad.
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x0114, 248, 144, after_the_intro()).unwrap();
        let (mut dropped, mut landed) = (false, false);
        for _ in 0..120 {
            world.update(None, Presses::default()).unwrap();
            let list = world.ark_pose().map(|pose| pose.list);
            dropped |= world.ark_lift() < 0 && list == Some(0x13);
            landed |= world.ark_lift() == 0 && list == Some(0x14);
        }
        assert!(dropped && landed, "{:?}", rom.revision());
        assert!(!world.ark_blinks(), "{:?}", rom.revision());
        assert!(!world.pad_locked(), "{:?}", rom.revision());
        assert_eq!(world.ark_pose(), None, "{:?}", rom.revision());
    }
}

#[test]
fn a_fall_elsewhere_costs_life_and_puts_ark_back() {
    // Rows 39+ of `$10F` have no exit: damage, then the last safe spot.
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x010F, 152, 688, after_the_intro()).unwrap();
        let (life, _) = world.life();
        fall_right(&mut world, 60, 50);
        // Then back, hidden, a pixel a frame each way (`$84:A01E`).
        let mut path = vec![world.position()];
        while world.falling() && path.len() < 400 {
            assert!(world.ark_blinks());
            world.update(None, Presses::default()).unwrap();
            path.push(world.position());
        }
        assert!(!world.falling(), "{:?}", rom.revision());
        assert!(path.len() > 2, "{:?}: {path:?}", rom.revision());
        assert!(path.windows(2).all(|pair| {
            pair[0].0.abs_diff(pair[1].0) <= 1 && pair[0].1.abs_diff(pair[1].1) <= 1
        }));
        assert_eq!(world.map(), 0x010F);
        assert!(world.life().0 < life);
        let (x, y) = world.position();
        assert!(x < 176 && y == 688, "{:?}: {:?}", rom.revision(), (x, y));
    }
}

#[test]
fn a_fall_that_takes_the_last_life_sends_ark_down() {
    // `$84:D4F4` takes at least 4; at no life the upkeep sends Ark down
    // (`$85:E15B`), and he wakes at home with all his life.
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x010F, 152, 688, after_the_intro()).unwrap();
        world.set_life(3);
        fall_right(&mut world, 60, 0);
        while world.life().0 > 0 && world.falling() {
            world.update(None, Presses::default()).unwrap();
        }
        assert_eq!(world.life().0, 0, "{:?}", rom.revision());
        let digits = world.digits();
        assert_eq!(digits.last().map(|digits| digits.amount), Some(4));
        for _ in 0..1200 {
            if world.map() != 0x010F {
                break;
            }
            world.update(None, Presses::default()).unwrap();
        }
        assert_ne!(world.map(), 0x010F, "{:?}", rom.revision());
        let (life, max) = world.life();
        assert_eq!(life, max);
    }
}

#[test]
fn a_lip_drop_and_its_fall_keep_the_native_frames() {
    // A native JP trace (2026-10-08): from the first frame Ark sinks 3
    // pixels (d), the drop's `$13` shows and its sound plays; at d+12 the
    // fall starts, its `$18` and sound `$12` a frame later, `$10` 9 frames
    // after; at d+62 Ark loses life and walks back a pixel a frame, at 736
    // by d+109, and walks again 3 frames on.
    use crysta_runtime::audio::Cue;
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x010F, 136, 736, after_the_intro()).unwrap();
        let mut rows = vec![];
        let mut previous = world.position().1;
        for _ in 0..160 {
            world
                .update(Some(Direction::Down), Presses::default())
                .unwrap();
            let sounds: Vec<u16> = world
                .take_cues()
                .into_iter()
                .filter_map(|cue| match cue {
                    Cue::Sound(sound) => Some(sound),
                    Cue::Track { .. } => None,
                })
                .collect();
            let y = world.position().1;
            let pose = world.ark_pose().map(|pose| (pose.list, pose.age));
            rows.push((y, y.wrapping_sub(previous), pose, sounds, world.life().0));
            previous = y;
        }
        let d = rows.iter().position(|row| row.1 == 3).unwrap();
        let at = |frame: usize| &rows[d + frame];
        assert_eq!(at(0).2, Some((0x13, 0)), "{:?}", rom.revision());
        assert!(at(0).3.contains(&0x1000), "{:?}", rom.revision());
        assert_eq!(rows[d - 1].2, None, "{:?}", rom.revision());
        assert_eq!(at(12).0, 784, "{:?}", rom.revision());
        assert_eq!(
            at(12).2.map(|pose| pose.0),
            Some(0x13),
            "{:?}",
            rom.revision()
        );
        assert_eq!(at(13).2, Some((0x18, 0)), "{:?}", rom.revision());
        assert!(at(13).3.contains(&0x1200), "{:?}", rom.revision());
        assert!(at(22).3.contains(&0x1000), "{:?}", rom.revision());
        assert_eq!((at(61).0, at(61).4), (784, 28), "{:?}", rom.revision());
        assert_eq!((at(62).0, at(62).4), (783, 24), "{:?}", rom.revision());
        assert_eq!((at(108).0, at(109).0), (737, 736), "{:?}", rom.revision());
        assert_eq!((at(111).0, at(112).0), (736, 737), "{:?}", rom.revision());
    }
}

#[test]
fn a_lip_drops_ark_three_pixels_a_frame_into_the_pit_below() {
    // `$10F` rows 46-47 are lips (attribute 8) over the pits from row 48:
    // standing on them starts the drop (`$80:CED1`, `$80:CF50`), stream
    // `$27` of resource 0 (3 pixels down a frame, walls off), until every
    // sample is a pit (`$80:CDA2`): then the fall.
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x010F, 136, 736, after_the_intro()).unwrap();
        let mut ys = vec![];
        for _ in 0..40 {
            if world.falling() {
                break;
            }
            world
                .update(Some(Direction::Down), Presses::default())
                .unwrap();
            ys.push(world.position().1);
        }
        assert!(world.falling(), "{:?}: {ys:?}", rom.revision());
        let drops = ys.windows(2).filter(|pair| pair[1] == pair[0] + 3).count();
        assert!(drops >= 5, "{:?}: {ys:?}", rom.revision());
        // Every sample a pit: row 48 under the box's top.
        assert!(world.position().1 - 16 >= 48 * 16, "{:?}", rom.revision());
    }
}

#[test]
fn no_map_puts_a_partial_cell_beside_a_lip() {
    // room-core refuses that pair for vertical steps (`lip_second`): a
    // corner nudge no material pair gives.
    for rom in roms() {
        for map in 0..0x200 {
            let Ok(built) = crysta_runtime::room(rom.image(), map) else {
                continue;
            };
            let width = usize::from(built.width);
            let kind = |cell: &u16| (cell >> 9) & 0x1F;
            for row in built.room.cells().chunks(width) {
                for pair in row.windows(2) {
                    let (a, b) = (kind(&pair[0]), kind(&pair[1]));
                    assert!(
                        !((matches!(a, 5 | 16) && b == 8) || (a == 8 && matches!(b, 5 | 16))),
                        "{:?} {map:#x}",
                        rom.revision()
                    );
                }
            }
        }
    }
}

#[test]
fn the_darts_of_tower_threes_second_floor_hurt_ark() {
    // `docs/darts-and-burn.md` §1: the right launcher at (704, 464) (row
    // `$1D`) shoots a box over x 656..704, y 456..464 once Ark is within
    // `$60`; profile 5's attack.
    for rom in roms() {
        let mut events = after_the_intro();
        events[0x103 / 8] |= 1 << (0x103 % 8);
        let mut world = World::enter_with_events(rom.image(), 0x0110, 680, 472, events).unwrap();
        world.set_life(99);
        // A 194-frame cycle; the launcher (profile 5, life 0, `+$04 & $20`:
        // no target) keeps shooting.
        let mut hits = 0;
        for _ in 0..600 {
            let before = world.life().0;
            world.update(None, Presses::default()).unwrap();
            if world.life().0 < before {
                hits += 1;
                world.place(680, 472);
            }
        }
        assert!(hits >= 2, "{:?}: {hits}", rom.revision());
    }
}

#[test]
fn a_dart_pushes_ark_as_the_native_trace_does() {
    // A native JP trace (2026-10-08) of Ark idle at (680,472): the hit's
    // sound on port 2; list `$0E` and a pixel a frame left for 10 frames,
    // one more at h+27 (stream `$38`); free at h+29; hit again at h+43 in
    // the same box; the launcher's cycle is 194 frames.
    use crysta_runtime::audio::Cue;
    for rom in roms() {
        let mut events = after_the_intro();
        events[0x103 / 8] |= 1 << (0x103 % 8);
        let mut world = World::enter_with_events(rom.image(), 0x0110, 680, 472, events).unwrap();
        world.set_life(99);
        let mut rows = vec![];
        for _ in 0..500 {
            world.update(None, Presses::default()).unwrap();
            let sounds: Vec<u16> = world
                .take_cues()
                .into_iter()
                .filter_map(|cue| match cue {
                    Cue::Sound(sound) => Some(sound),
                    Cue::Track { .. } => None,
                })
                .collect();
            let pose = world.ark_pose().map(|pose| (pose.list, pose.age));
            rows.push((world.position().0, world.life().0, pose, sounds));
        }
        let hits: Vec<usize> = (1..rows.len())
            .filter(|&i| rows[i].1 < rows[i - 1].1)
            .collect();
        let h = hits[0];
        assert!(rows[h].3.contains(&0x0007), "{:?}", rom.revision());
        let xs: Vec<u16> = (h..=h + 28).map(|i| rows[i].0).collect();
        let mut expected: Vec<u16> = (0..=10).map(|step| 680 - step).collect();
        expected.extend([670; 16]);
        expected.extend([669, 669]);
        assert_eq!(xs, expected, "{:?}", rom.revision());
        assert_eq!(rows[h + 1].2, Some((0x0E, 0)), "{:?}", rom.revision());
        assert_eq!(rows[h + 28].2, None, "{:?}", rom.revision());
        assert_eq!(hits[1], h + 43, "{:?}: {hits:?}", rom.revision());
        let natively = [h, h + 43, h + 198, h + 241, h + 392];
        assert_eq!(hits[..5], natively, "{:?}", rom.revision());
    }
}

#[test]
fn the_burn_follows_the_native_trace() {
    // A native JP trace (2026-10-08) of a flyer's bullet on `$116`: the
    // hit (h) sets the burn; at h+1 the push's first pixel and pose, and
    // the burn's script takes Ark; resource 5's list 4 from h+2 with the
    // text, list 5 from h+59; the burn ends at h+120 and Ark stands at
    // h+122.
    for rom in roms() {
        let mut events = after_the_intro();
        for flag in [0x103, 0x105] {
            events[flag / 8] |= 1 << (flag % 8);
        }
        let mut world = World::enter_with_events(rom.image(), 0x0116, 300, 760, events).unwrap();
        let mut rows = vec![];
        let mut burned = None;
        for i in 0..4000 {
            world.set_life(99);
            let before = world.life().0;
            world.update(None, Presses::default()).unwrap();
            let pose = world
                .ark_pose()
                .map(|pose| (pose.resource, pose.list, pose.age));
            rows.push((world.life().0 < before, pose, world.dialogue().is_some()));
            if burned.is_none() && pose.is_some_and(|pose| (pose.0, pose.1) == (5, 4)) {
                burned = Some(i);
            }
            if burned.is_some_and(|start| i > start + 130) {
                break;
            }
            if burned.is_none() && world.ark_pose().is_none() && i % 200 == 199 {
                world.place(300, 760);
            }
        }
        let start = burned.unwrap_or_else(|| panic!("{:?}: no burn", rom.revision()));
        let h = start - 2;
        assert!(rows[h].0, "{:?}", rom.revision());
        assert_eq!(
            rows[h + 1].1.map(|pose| pose.1),
            Some(0x0D),
            "{:?}",
            rom.revision()
        );
        assert_eq!(rows[h + 2].1, Some((5, 4, 0)), "{:?}", rom.revision());
        assert!(rows[h + 2].2, "{:?}", rom.revision());
        assert_eq!(
            rows[h + 58].1.map(|pose| pose.1),
            Some(4),
            "{:?}",
            rom.revision()
        );
        assert_eq!(rows[h + 59].1, Some((5, 5, 0)), "{:?}", rom.revision());
        assert_eq!(
            rows[h + 121].1.map(|pose| pose.1),
            Some(5),
            "{:?}",
            rom.revision()
        );
        assert_eq!(rows[h + 122].1, None, "{:?}", rom.revision());
    }
}

#[test]
fn a_pedestal_on_tower_threes_second_floor_toggles_its_flag() {
    // `docs/tower-three.md` §3: A on a pedestal (`$90:FBB3`) sets its flag
    // (`$001-$004`, `JSL $80:BBCD`); on `$110` the next press clears it.
    let a = Presses {
        confirm: true,
        ..Presses::default()
    };
    for rom in roms() {
        let mut events = after_the_intro();
        events[0x103 / 8] |= 1 << (0x103 % 8);
        let mut world = World::enter_with_events(rom.image(), 0x0110, 296, 608, events).unwrap();
        let mut flags = vec![];
        for _ in 0..2 {
            world.place(296, 608);
            world.face(Direction::Up);
            for _ in 0..3 {
                world.update(None, Presses::default()).unwrap();
            }
            world.update(None, a).unwrap();
            for _ in 0..30 {
                world.update(None, Presses::default()).unwrap();
            }
            flags.push(world.events()[0] & 0x1E);
        }
        assert!(
            flags[0].is_power_of_two(),
            "{:?}: {flags:?}",
            rom.revision()
        );
        assert_eq!(flags[1], 0, "{:?}", rom.revision());
        assert!(world.frozen_scripts().is_empty(), "{:?}", rom.revision());
    }
}

#[test]
fn tower_threes_ball_wave_drops_eight_hiballs() {
    // `docs/tower-three.md` §3: with both pedestals (`$001`, `$002`) the
    // `$112` controller sets `$003` and opens the door (`$112`); the eight
    // balls wait a random multiple of 8 frames (`ASL`), drop and fight.
    for rom in roms() {
        let mut events = after_the_intro();
        for flag in [0x103, 0x001, 0x002] {
            events[flag / 8] |= 1 << (flag % 8);
        }
        let mut world = World::enter_with_events(rom.image(), 0x0112, 384, 700, events).unwrap();
        let wave = |resident: &&crysta_runtime::residents::Resident| {
            matches!(resident.script, Some(0x90_9BFC | 0x90_9EB7))
        };
        // Each ball's record, once it has left its start row as a foe.
        let mut dropped = std::collections::BTreeSet::new();
        for _ in 0..600 {
            world.update(None, Presses::default()).unwrap();
            for resident in world.residents().iter().filter(wave) {
                if resident.position.1 != 224 && world.foe_life(resident.record).is_some() {
                    dropped.insert(resident.record);
                }
            }
        }
        let flag = |n: usize| world.events()[n / 8] & (1 << (n % 8)) != 0;
        assert!(flag(0x003) && flag(0x112), "{:?}", rom.revision());
        assert!(world.frozen_scripts().is_empty(), "{:?}", rom.revision());
        assert_eq!(dropped.len(), 8, "{:?}", rom.revision());
    }
}

#[test]
fn the_high_cadet_falls_after_three_real_hits() {
    // `docs/tower-three.md` §3: Ark walks up to the guardian; the room
    // closes and the High Cadet comes as three copies. A hit on the real
    // one costs it a life; a fake leaves a Cadet. After three, flag `$002`,
    // then `$11A` and the door opens.
    let a = Presses {
        confirm: true,
        ..Presses::default()
    };
    for rom in roms() {
        let mut events = after_the_intro();
        events[0x103 / 8] |= 1 << (0x103 % 8);
        let mut world = World::enter_with_events(rom.image(), 0x0113, 392, 440, events).unwrap();
        world.set_life(999);
        let flag = |world: &World, n: usize| world.events()[n / 8] & (1 << (n % 8)) != 0;
        for frame in 0..6000 {
            let reading = world.dialogue().is_some() && !world.typing();
            let up = (frame < 20).then_some(Direction::Up);
            world
                .update(up, if reading { a } else { Presses::default() })
                .unwrap();
            if frame % 40 == 0 {
                world.hit_spawned(1);
            }
            if flag(&world, 0x11A) {
                break;
            }
        }
        assert!(
            flag(&world, 0x002) && flag(&world, 0x11A),
            "{:?}",
            rom.revision()
        );
        assert!(world.frozen_scripts().is_empty(), "{:?}", rom.revision());
    }
}

#[test]
fn tower_fours_cadets_have_bodies() {
    // `docs/tower-four.md` §5.1: the Cadets of `$118` share `$82:EB0B`
    // (European `$82:EA98`), whose graphics start 16 blocks into the sheet.
    for rom in roms() {
        let mut events = after_the_intro();
        events[0x105 / 8] |= 1 << (0x105 % 8);
        let world = World::enter_with_events(rom.image(), 0x0118, 376, 880, events).unwrap();
        let cadets: Vec<_> = world
            .residents()
            .iter()
            .filter(|resident| matches!(resident.descriptor, Some(0x02_EB0B | 0x02_EA98)))
            .map(|resident| resident.body)
            .collect();
        assert!(!cadets.is_empty(), "{:?}", rom.revision());
        assert!(
            cadets.iter().all(|&body| body),
            "{:?}: {cadets:?}",
            rom.revision()
        );
    }
}

/// `$118` with the Cadets beaten (`$11C`): the platform's walls are open.
fn tower_four_platform(rom: &Rom) -> World<'_> {
    let mut events = after_the_intro();
    for flag in [0x105, 0x11C] {
        events[flag / 8] |= 1 << (flag % 8);
    }
    World::enter_with_events(rom.image(), 0x0118, 328, 730, events).unwrap()
}

#[test]
fn a_pit_takes_ark_carrying_a_pot_and_the_pot_flies_on() {
    // `$117`'s pot at (8,46), its pit at rows 35-36 (`$84:9F53`): the
    // fall lets the pot fly from 16 pixels ahead (`$84:C649`).
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x0117, 152, 752, after_the_intro()).unwrap();
        for _ in 0..20 {
            world.update(None, Presses::default()).unwrap();
        }
        world
            .update(Some(Direction::Left), Presses::default())
            .unwrap();
        let lift = Presses {
            confirm: true,
            ..Presses::default()
        };
        world.update(None, lift).unwrap();
        for _ in 0..40 {
            world.update(None, Presses::default()).unwrap();
        }
        assert!(world.carry().is_some(), "{:?}", rom.revision());
        while world.position().0 < 200 {
            world
                .update(Some(Direction::Right), Presses::default())
                .unwrap();
        }
        let mut flew = false;
        for _ in 0..120 {
            if world.falling() {
                break;
            }
            world
                .update(Some(Direction::Up), Presses::default())
                .unwrap();
        }
        assert!(
            world.falling(),
            "{:?}: {:?}",
            rom.revision(),
            world.position()
        );
        assert_eq!(world.carry(), None, "{:?}", rom.revision());
        for _ in 0..120 {
            flew |= world.pot().is_some_and(|pot| pot.flight.is_some());
            world.update(None, Presses::default()).unwrap();
        }
        assert!(flew, "{:?}", rom.revision());
        assert_eq!(world.pot(), None, "{:?}", rom.revision());
        assert!(!world.falling(), "{:?}", rom.revision());
    }
}

#[test]
fn ark_crosses_tower_fours_rope() {
    // `docs/tower-four.md` §2: the rope (attribute `$12`, row 45) between
    // the platform and the left ledge; Left and Right walk it.
    for rom in roms() {
        let mut world = tower_four_platform(&rom);
        for _ in 0..120 {
            world
                .update(Some(Direction::Left), Presses::default())
                .unwrap();
        }
        assert!(!world.falling(), "{:?}", rom.revision());
        assert_eq!(world.map(), 0x0118);
        assert!(
            world.position().0 < 232,
            "{:?}: {:?}",
            rom.revision(),
            world.position()
        );
    }
}

#[test]
fn leaning_off_the_rope_drops_ark_a_floor() {
    // Up on the rope leans; without Down within 60 frames, Ark falls, into
    // the pit's exit to `$117`.
    for rom in roms() {
        let mut world = tower_four_platform(&rom);
        for _ in 0..40 {
            world
                .update(Some(Direction::Left), Presses::default())
                .unwrap();
        }
        assert!(world.on_rope(), "{:?}", rom.revision());
        let y = world.position().1;
        world
            .update(Some(Direction::Up), Presses::default())
            .unwrap();
        assert_eq!(
            world.position().1,
            y,
            "{:?}: Up leans, it does not move",
            rom.revision()
        );
        for _ in 0..300 {
            world.update(None, Presses::default()).unwrap();
            if world.map() != 0x0118 {
                break;
            }
        }
        assert_eq!(world.map(), 0x0117, "{:?}", rom.revision());
    }
}

#[test]
fn the_dancing_huball_troupe_ends_when_its_balls_are_hit() {
    // `docs/tower-four.md` §3: eight balls in formation; each hit turns one
    // into a Hiball. At none: the end text, flag `$11D`, back to `$11A`.
    let a = Presses {
        confirm: true,
        ..Presses::default()
    };
    for rom in roms() {
        let mut events = after_the_intro();
        events[0x105 / 8] |= 1 << (0x105 % 8);
        let mut world = World::enter_with_events(rom.image(), 0x011B, 376, 552, events).unwrap();
        world.set_life(999);
        for frame in 0..6000 {
            let reading = world.dialogue().is_some() && !world.typing();
            world
                .update(None, if reading { a } else { Presses::default() })
                .unwrap();
            if frame % 30 == 0 {
                world.hit_spawned(1);
            }
            if world.map() != 0x011B {
                break;
            }
        }
        assert_eq!(world.map(), 0x011A, "{:?}", rom.revision());
        assert!(world.events()[0x11D / 8] & (1 << (0x11D % 8)) != 0);
    }
}

#[test]
fn tower_fours_top_floor_runs_without_freezing() {
    // `$11A`'s ring follows walls; blocked every way it waits (`$97:B445`).
    for rom in roms() {
        let mut events = after_the_intro();
        events[0x105 / 8] |= 1 << (0x105 % 8);
        let mut world = World::enter_with_events(rom.image(), 0x011A, 376, 552, events).unwrap();
        for _ in 0..300 {
            world.update(None, Presses::default()).unwrap();
        }
        assert!(
            world.frozen_scripts().is_empty(),
            "{:?}: {:x?}",
            rom.revision(),
            world.frozen_scripts()
        );
    }
}

#[test]
fn guardners_have_bodies_and_can_be_hit() {
    // Their records are the 16-byte form (parameter `$C0`, `$80:F564`):
    // the descriptor as in the 10-byte one, then `7F:1018..101E`.
    for rom in roms() {
        let mut events = after_the_intro();
        events[0x103 / 8] |= 1 << (0x103 % 8);
        let world = World::enter_with_events(rom.image(), 0x0111, 176, 464, events).unwrap();
        let guardners: Vec<_> = world
            .residents()
            .iter()
            .filter(|resident| matches!(resident.script, Some(0x97_C34A | 0x99_8E00)))
            .map(|resident| (resident.body, world.foe_life(resident.record).is_some()))
            .collect();
        assert_eq!(guardners, [(true, true); 2], "{:?}", rom.revision());
    }
}

#[test]
fn a_guardners_bolt_puts_ark_to_sleep_and_its_vacuum_draws_him_in() {
    // Native JP traces (2026-10-08) on `$117`, the Guardner at (384,480):
    // the bolt hurts and its contact callback takes Ark at once (`$97:C5A9`):
    // list 7 for 132 frames, 8 for 32; the vacuum pulls him a pixel every
    // third frame, x to within 2 of the Guardner's, then y until his probe
    // is 26 below it (y 514), through the sleep and after; then `$118`.
    for rom in roms() {
        let mut world =
            World::enter_with_events(rom.image(), 0x0117, 376, 576, after_the_intro()).unwrap();
        let mut rows = vec![];
        for _ in 0..700 {
            world.update(None, Presses::default()).unwrap();
            let pose = world.ark_pose().map(|pose| (pose.resource, pose.list));
            rows.push((world.map(), world.position(), pose, world.life().0));
        }
        let hit = (1..rows.len())
            .find(|&i| rows[i].3 < rows[i - 1].3)
            .unwrap();
        let sleep = (hit..rows.len())
            .find(|&i| rows[i].2 == Some((5, 7)))
            .unwrap();
        assert!(sleep - hit <= 2, "{:?}: {hit} {sleep}", rom.revision());
        let lists: Vec<_> = rows[sleep..sleep + 165].iter().map(|row| row.2).collect();
        assert!(
            lists[..132].iter().all(|&list| list == Some((5, 7))),
            "{:?}",
            rom.revision()
        );
        assert!(
            lists[132..164].iter().all(|&list| list == Some((5, 8))),
            "{:?}",
            rom.revision()
        );
        // The hit's first pixel, then the pull.
        assert_eq!(rows[sleep].1, (376, 577), "{:?}", rom.revision());
        let moves: Vec<usize> = (sleep + 1..rows.len())
            .filter(|&i| rows[i].0 == 0x0117 && rows[i].1 != rows[i - 1].1)
            .collect();
        assert!(
            moves.windows(2).all(|pair| pair[1] - pair[0] == 3),
            "{:?}",
            rom.revision()
        );
        let path: Vec<(u16, u16)> = moves.iter().map(|&i| rows[i].1).collect();
        assert_eq!(path.first(), Some(&(377, 577)), "{:?}", rom.revision());
        assert!(path.contains(&(382, 577)), "{:?}", rom.revision());
        assert_eq!(path.last(), Some(&(382, 514)), "{:?}", rom.revision());
        let last = rows.last().unwrap();
        assert_eq!(
            (last.0, last.1),
            (0x0118, (384, 848)),
            "{:?}",
            rom.revision()
        );
    }
}

/// Frames of a scene, A on each read page, and A every second when `talk`.
fn play(world: &mut World, frames: usize, talk: bool) {
    let a = Presses {
        confirm: true,
        ..Presses::default()
    };
    for frame in 0..frames {
        let reading = world.dialogue().is_some() && !world.typing();
        let ask = talk && world.dialogue().is_none() && frame % 60 == 5;
        world
            .update(
                None,
                if reading || ask {
                    a
                } else {
                    Presses::default()
                },
            )
            .unwrap();
    }
}

/// Elle's house (`$14`), Ark in front of Elle.
fn before_elle(image: &[u8], events: Vec<u8>) -> World<'_> {
    let mut world = World::enter_with_events(image, 0x0014, 600, 144, events).unwrap();
    play(&mut world, 30, false);
    let elle = world
        .residents()
        .iter()
        .find(|resident| matches!(resident.script, Some(0x88_BC7B | 0x88_C852)))
        .map(|resident| resident.position)
        .unwrap();
    world.place(elle.0, elle.1 + 16);
    world.face(Direction::Up);
    world
}

#[test]
fn elle_weaves_the_cape_from_the_crystal_thread() {
    // `docs/tower-five.md` §2: the thread (`$29`), the bed (`$2A`, night),
    // the weaving and the talk (`$30`, `$2B`), the bed (`$2C`), the cape
    // (`$2D`, item `$BF`).
    for rom in roms() {
        let mut events = after_the_intro();
        for flag in (0x20..=0x28).chain(0x101..=0x107).chain([0x589]) {
            events[flag / 8] |= 1 << (flag % 8);
        }
        let mut world = before_elle(rom.image(), events);
        world.give_item(0x32);
        play(&mut world, 900, true);
        for flags in [[0x29, 0x2A], [0x2B, 0x2C]] {
            let events = world.events().to_vec();
            let mut bed = World::enter_with_events(rom.image(), 0x000F, 296, 116, events).unwrap();
            play(&mut bed, 700, false);
            assert!(flags
                .iter()
                .all(|&flag| bed.events()[flag / 8] & (1 << (flag % 8)) != 0));
            world = before_elle(rom.image(), bed.events().to_vec());
            play(&mut world, 1500, true);
        }
        assert!(
            world.events()[0x2D / 8] & (1 << (0x2D % 8)) != 0,
            "{:?}",
            rom.revision()
        );
        assert!(
            world.has_item(0xBF) && !world.has_item(0x32),
            "{:?}",
            rom.revision()
        );
        assert_eq!(world.armor(), Some(0xBF), "{:?}: worn", rom.revision());
    }
}

#[test]
fn the_orb_check_lets_the_cape_through_and_throws_ark_out_without_it() {
    // `docs/tower-five.md` §1: `$11D`'s orb is reflected by the cape (flag
    // `$19B`); without it, it pushes Ark onto the exit back to `$11C`.
    for rom in roms() {
        for cape in [true, false] {
            let mut events = after_the_intro();
            for flag in 0x101..=0x107 {
                events[flag / 8] |= 1 << (flag % 8);
            }
            let mut world =
                World::enter_with_events(rom.image(), 0x011D, 128, 192, events).unwrap();
            if cape {
                world.equip_armor(0xBF);
            }
            play(&mut world, 700, false);
            let reflected = world.events()[0x19B / 8] & (1 << (0x19B % 8)) != 0;
            assert_eq!(reflected, cape, "{:?}", rom.revision());
            assert_eq!(world.map(), if cape { 0x011D } else { 0x011C });
            assert!(!world.pad_locked(), "{:?}", rom.revision());
        }
    }
}

#[test]
fn shadowkeeper_falls_after_two_lives_and_the_tower_ends() {
    // `docs/tower-five.md` §1: Ark walks up to the boss (y < 272: the
    // fight, flag `$001`); two lives; at none the end controller sends Ark
    // to the light room `$106`.
    let a = Presses {
        confirm: true,
        ..Presses::default()
    };
    for rom in roms() {
        let mut events = after_the_intro();
        for flag in (0x101..=0x107).chain([0x19B]) {
            events[flag / 8] |= 1 << (flag % 8);
        }
        let mut world = World::enter_with_events(rom.image(), 0x0123, 136, 300, events).unwrap();
        world.set_life(999);
        let boss = world
            .residents()
            .iter()
            .find(|resident| matches!(resident.script, Some(0x93_D876 | 0x99_9B5E)))
            .map(|resident| resident.record)
            .unwrap();
        let mut began = false;
        for frame in 0..3000 {
            let reading = world.dialogue().is_some() && !world.typing();
            let up = (frame < 30).then_some(Direction::Up);
            world
                .update(up, if reading { a } else { Presses::default() })
                .unwrap();
            began |= world.map() == 0x0123 && world.events()[0] & 2 != 0;
            if began && frame % 20 == 0 && world.foe_life(boss).is_some_and(|life| life > 0) {
                world.kill_foe(boss);
            }
            if world.map() != 0x0123 {
                break;
            }
        }
        assert!(began, "{:?}: the fight began", rom.revision());
        assert_eq!(world.map(), 0x0106, "{:?}", rom.revision());
        assert!(world.frozen_scripts().is_empty(), "{:?}", rom.revision());
    }
}

#[test]
fn the_continents_door_raises_mu() {
    // `docs/underworld-end.md` §2: after tower 5 (`$109`) the door on `$12A`
    // sets `$11E` and reloads the map; then the text, flags `$11F`, `$40E`,
    // and back to the underworld.
    for rom in roms() {
        let mut events = after_the_intro();
        for flag in 0x101..=0x109 {
            events[flag / 8] |= 1 << (flag % 8);
        }
        let mut world = World::enter_with_events(rom.image(), 0x012A, 1120, 124, events).unwrap();
        world.face(Direction::Up);
        play(&mut world, 900, false);
        let flag = |n: usize| world.events()[n / 8] & (1 << (n % 8)) != 0;
        assert_eq!(world.map(), 0x0003, "{:?}", rom.revision());
        assert!(flag(0x11F) && flag(0x40E), "{:?}", rom.revision());
    }
}

#[test]
fn the_way_to_the_hole_opens_with_the_elders_flag() {
    // `$03`'s exit at (41,36) names a list (`$81:F046`): with `$74` it
    // leads to the Hole `$127`.
    for rom in roms() {
        for told in [false, true] {
            let mut events = after_the_intro();
            for flag in (0x101..=0x109).chain(told.then_some(0x74)) {
                events[flag / 8] |= 1 << (flag % 8);
            }
            let mut world =
                World::enter_with_events(rom.image(), 0x0003, 664, 560, events).unwrap();
            for _ in 0..60 {
                world
                    .update(Some(Direction::Down), Presses::default())
                    .unwrap();
            }
            assert_eq!(world.map() == 0x0127, told, "{:?}", rom.revision());
        }
    }
}

#[test]
fn elles_farewell_on_crysta_sets_its_flag() {
    // `docs/underworld-end.md` §2: on `$13`, Ark in the zone with Right
    // held: the farewell, flag `$247`.
    for rom in roms() {
        let mut events = after_the_intro();
        for flag in (0x20..=0x2D).chain(0x101..=0x109).chain([0x74]) {
            events[flag / 8] |= 1 << (flag % 8);
        }
        let mut world = World::enter_with_events(rom.image(), 0x0013, 476, 168, events).unwrap();
        let a = Presses {
            confirm: true,
            ..Presses::default()
        };
        for frame in 0..1500 {
            let reading = world.dialogue().is_some() && !world.typing();
            let free = !world.pad_locked() && world.dialogue().is_none() && frame < 300;
            world
                .update(
                    free.then_some(Direction::Right),
                    if reading { a } else { Presses::default() },
                )
                .unwrap();
        }
        assert!(
            world.events()[0x247 / 8] & (1 << (0x247 % 8)) != 0,
            "{:?}",
            rom.revision()
        );
    }
}

#[test]
fn ark_jumps_into_the_hole_and_the_chapter_ends() {
    // `docs/underworld-end.md` §2: the elder's story in `$127`; "No" with
    // `$247` opens the rim (attribute 8); Ark jumps in, the exit to the
    // vortex ends Chapter 1 (flag `$06F`).
    let a = Presses {
        confirm: true,
        ..Presses::default()
    };
    let down = Presses {
        down: true,
        ..Presses::default()
    };
    for rom in roms() {
        let mut events = after_the_intro();
        for flag in (0x20..=0x2D).chain(0x101..=0x109).chain([0x74, 0x247]) {
            events[flag / 8] |= 1 << (flag % 8);
        }
        let mut world = World::enter_with_events(rom.image(), 0x0127, 296, 192, events).unwrap();
        world.face(Direction::Up);
        let mut moved = false;
        for frame in 0..4000 {
            let view = world.dialogue();
            let choosing = view.as_ref().is_some_and(|view| view.cursor.is_some());
            let reading = view.is_some() && !world.typing();
            let talking = view.is_none() && frame % 60 == 5 && frame < 400;
            let presses = if choosing && !moved {
                moved = true;
                down
            } else if reading || talking {
                a
            } else {
                Presses::default()
            };
            let walk = (frame > 800 && view.is_none()).then_some(Direction::Down);
            world.update(walk, presses).unwrap();
            if world.chapter_over() {
                break;
            }
        }
        assert!(world.chapter_over(), "{:?}", rom.revision());
        assert!(
            world.events()[0x6F / 8] & (1 << (0x6F % 8)) != 0,
            "{:?}",
            rom.revision()
        );
    }
}
