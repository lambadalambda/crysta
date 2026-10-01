//! Spear combat in the towers (`docs/combat.md`), on both ROMs.

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

const A: Presses = Presses {
    confirm: true,
    ..Presses::NONE
};

/// Tower 1's first floor, the spear in hand, Ark facing Up at (112,607).
fn first_floor(rom: &Rom) -> World<'_> {
    let mut events = fresh_game_flags();
    events[0x100 / 8] |= 1 << (0x100 % 8);
    let mut world = World::enter_with_events(rom.image(), 0x0101, 112, 607, events).unwrap();
    world.equip_weapon(0x81);
    world.face(Direction::Up);
    for _ in 0..4 {
        world.update(None, Presses::NONE).unwrap();
    }
    world
}

#[test]
fn a_thrust_plays_sixteen_frames_and_hits_from_the_second() {
    // `docs/combat.md`, thrust facing Up: P+1..P+16 the list `$01` of
    // resource 4 (4, 4, 2, 2, 2+2 frames), attacking from P+2; P+17 stands.
    for rom in roms() {
        let mut world = first_floor(&rom);
        // The poses after frame P (the press), P+1, ...
        world.update(None, A).unwrap();
        let mut poses = vec![world.attack_pose()];
        let mut boxes = vec![world.attack_box()];
        for _ in 0..17 {
            world.update(None, Presses::NONE).unwrap();
            poses.push(world.attack_pose());
            boxes.push(world.attack_box());
        }
        assert_eq!(poses[0], None, "P still stands");
        let ages: Vec<_> = poses[1..17]
            .iter()
            .map(|pose| pose.map(|(list, age, _)| (list, age)))
            .collect();
        assert_eq!(
            ages,
            (0..16).map(|age| Some((1, age))).collect::<Vec<_>>(),
            "{:?}",
            rom.revision()
        );
        assert_eq!(poses[17], None, "P+17 stands");
        assert_eq!(boxes[1], None, "P+1: not yet attacking");
        // P+5: (-7,17,-40,25) at Ark (112,607): x 105..122, y 567..592.
        assert_eq!(boxes[5], Some((105, 567, 122, 592)));
        assert!(boxes[2..17].iter().all(Option::is_some));
        assert_eq!(boxes[17], None);
    }
}

#[test]
fn enemy_profiles_read_from_the_stat_table() {
    // `$8D:BDFA` (European `$8D:BCC3`): profile 1 is the blob, 4 life.
    for rom in roms() {
        let blob = crysta_runtime::combat::profile(rom.image(), 1).unwrap();
        assert_eq!(
            (blob.level, blob.life, blob.exp, blob.gems),
            (1, 4, 2, 3),
            "{:?}",
            rom.revision()
        );
        assert_eq!(
            crysta_runtime::combat::profile(rom.image(), 0),
            None,
            "Ark's own"
        );
    }
}

#[test]
fn a_thrust_kills_a_blob() {
    // `docs/combat.md` §4, measured (JP): the blob poked to (120,150) with
    // life 1, Ark at (120,182) facing Up. A at P, the hit at P+5; the death
    // check when the knockback ends adds 2 EXP; the explosion follows.
    for rom in roms() {
        let mut world = first_floor(&rom);
        let record = world
            .residents()
            .iter()
            .find(|resident| resident.position == (120, 160))
            .expect("the far blob")
            .record;
        world.place(120, 182);
        world.face(Direction::Up);
        assert!(
            world.poke_foe(record, (120, 150), 1),
            "{:?}",
            rom.revision()
        );
        world.update(None, A).unwrap();
        let overlay = |world: &World<'_>| {
            world
                .residents()
                .iter()
                .find(|resident| resident.record == record)
                .and_then(|resident| resident.overlay.map(|(_, list)| (list, resident.position)))
        };
        let mut lives = Vec::new();
        let mut seen = Vec::new();
        for _ in 0..80 {
            world.update(None, Presses::NONE).unwrap();
            lives.push(world.foe_life(record));
            seen.push(overlay(&world).map(|(list, _)| list));
        }
        let hit = lives
            .iter()
            .position(|&life| life == Some(0))
            .expect("the hit");
        assert_eq!(hit + 1, 5, "P+5");
        assert_eq!(world.exp(), 2, "{:?}", rom.revision());
        // The explosion, then a gem one time in two (`$85:E2E9`).
        assert!(seen.contains(&Some(0x16)), "{seen:?}");
        if let Some((0x0B, at)) = overlay(&world) {
            let money = world.money();
            world.place(at.0, at.1);
            world.update(None, Presses::NONE).unwrap();
            assert_eq!(world.money(), money + 3, "{:?}", rom.revision());
            assert_eq!(overlay(&world), None);
        }
    }
}

#[test]
fn a_blob_on_ark_costs_life_once_while_he_is_immune() {
    // `docs/combat.md` §5: a blob does 3 (2 with the variance) per contact;
    // Ark is pushed for 26 frames and can be hit again 43 frames later.
    for rom in roms() {
        let mut world = first_floor(&rom);
        let record = world
            .residents()
            .iter()
            .find(|resident| resident.position == (120, 160))
            .expect("the far blob")
            .record;
        world.place(120, 182);
        assert!(world.poke_foe(record, (120, 182), 4));
        let mut lives = Vec::new();
        for _ in 0..43 {
            world.update(None, Presses::NONE).unwrap();
            lives.push(world.life().0);
        }
        let first = lives.iter().position(|&life| life < 28).expect("a hit");
        assert!((25..=26).contains(&lives[first]), "{lives:?}");
        assert!(
            lives[first..].iter().all(|&life| life == lives[first]),
            "{lives:?}"
        );
    }
}

#[test]
fn killing_the_floors_blobs_opens_its_stairs() {
    // `docs/block-patch.md`: `$90:905E` polls `$0498`; at zero it sets
    // `$280`/`$281` and `COP 46` copies cells: (7,4) of the first layer
    // turns passable.
    for rom in roms() {
        let mut world = first_floor(&rom);
        let records: Vec<usize> = world
            .residents()
            .iter()
            .filter(|resident| resident.descriptor.is_some())
            .map(|resident| resident.record)
            .collect();
        assert_eq!(records.len(), 3, "{:?}", rom.revision());
        assert!(!flag(&world, 0x280));
        for record in records {
            assert!(world.kill_foe(record));
        }
        for _ in 0..120 {
            world.update(None, Presses::NONE).unwrap();
        }
        assert!(
            flag(&world, 0x280) && flag(&world, 0x281),
            "{:?}",
            rom.revision()
        );
        assert!(world
            .patched_cells()
            .iter()
            .any(|&(column, row, _)| (column, row) == (7, 4)));
        assert!(!world.pad_locked());
    }
}

fn flag(world: &World<'_>, flag: usize) -> bool {
    world.events()[flag / 8] & (1 << (flag % 8)) != 0
}

#[test]
fn reaching_38_exp_raises_ark_to_level_2() {
    // `docs/combat.md` §6: the level table `$8D:BA61` (European `$8D:B92A`):
    // level 2 at 38 EXP, life 33, attack 4, defense 3, luck 4.
    for rom in roms() {
        let mut world = first_floor(&rom);
        world.set_exp(36);
        let record = world
            .residents()
            .iter()
            .find(|resident| resident.descriptor.is_some())
            .expect("a blob")
            .record;
        assert!(world.kill_foe(record));
        for _ in 0..10 {
            world.update(None, Presses::NONE).unwrap();
        }
        let stats = world.stats();
        assert_eq!(
            (
                stats.level,
                stats.max_life,
                stats.attack,
                stats.defense,
                stats.luck
            ),
            (2, 33, 4, 3, 4),
            "{:?}",
            rom.revision()
        );
        assert_eq!(stats.life, 33, "the rise is added to the life too");
    }
}

#[test]
fn at_no_life_ark_wakes_at_the_saved_place_with_full_life() {
    // `docs/combat.md` §5: at 0 life he collapses, a message plays, then
    // life = max and a transfer to `$0600..$0607` (natively 707 frames
    // from the hit to map `$0F`).
    for rom in roms() {
        let mut world = first_floor(&rom);
        world.set_life(1);
        let record = world
            .residents()
            .iter()
            .find(|resident| resident.position == (120, 160))
            .expect("the far blob")
            .record;
        world.place(120, 182);
        assert!(world.poke_foe(record, (120, 182), 4));
        let mut frames = 0;
        while world.map() != 0x000F && frames < 1200 {
            world.update(None, Presses::NONE).unwrap();
            frames += 1;
        }
        assert_eq!(world.map(), 0x000F, "{:?}", rom.revision());
        assert!((600..=800).contains(&frames), "{frames}");
        assert_eq!(world.life(), (28, 28));
    }
}

#[test]
fn nobody_in_crysta_is_an_enemy() {
    // An enemy needs a profile and the hittable header bit `$0200`.
    for rom in roms() {
        for map in crysta_runtime::MAPS {
            let world =
                World::enter_with_events(rom.image(), map, 128, 128, fresh_game_flags()).unwrap();
            for resident in world.residents() {
                assert_eq!(
                    world.foe_life(resident.record),
                    None,
                    "{map:#x} {:#x}",
                    resident.record
                );
            }
        }
    }
}

#[test]
fn blobs_stay_off_the_walls() {
    // `docs/enemy-scripts.md` §6: an enemy's box (x-8..x+8, y-16..y) stops
    // at cells whose attribute is not 0, 1, 17 or 22.
    for rom in roms() {
        let mut events = fresh_game_flags();
        events[0x100 / 8] |= 1 << (0x100 % 8);
        let mut world = World::enter_with_events(rom.image(), 0x0101, 128, 400, events).unwrap();
        let blocks = |world: &World<'_>, x: u16, y: u16| {
            world
                .base_cell(x >> 4, y >> 4)
                .is_none_or(|word| !matches!((word >> 9) & 0x1F, 0 | 1 | 17 | 22))
        };
        let mut moved = 0;
        for _ in 0..3000 {
            let before: Vec<_> = world.residents().iter().map(|r| r.position).collect();
            world.update(None, Presses::NONE).unwrap();
            for (resident, was) in world.residents().iter().zip(before) {
                if world.foe_life(resident.record).is_none() || resident.position == was {
                    continue;
                }
                moved += 1;
                let (x, y) = resident.position;
                for (cx, cy) in [
                    (x - 8, y - 16),
                    (x + 7, y - 16),
                    (x - 8, y - 1),
                    (x + 7, y - 1),
                ] {
                    assert!(
                        !blocks(&world, cx, cy),
                        "{:?}: a blob at {:?} overlaps a wall",
                        rom.revision(),
                        resident.position
                    );
                }
            }
        }
        assert!(moved > 100, "{:?}: {moved}", rom.revision());
    }
}

#[test]
fn the_knight_guards_and_thrusts_at_ark() {
    // `$95:EFB0` (`docs/enemy-scripts.md` §8): an enemy (profile `$0E`, 23
    // life, its palette table entry `$9E`) that thrusts when Ark stands in
    // its front box and walks at him.
    for rom in roms() {
        let mut events = fresh_game_flags();
        events[0x100 / 8] |= 1 << (0x100 % 8);
        let world =
            World::enter_with_events(rom.image(), 0x0103, 128, 400, events.clone()).unwrap();
        let knight = world
            .residents()
            .iter()
            .find(|resident| world.foe_life(resident.record) == Some(23))
            .unwrap_or_else(|| panic!("{:?}: no knight", rom.revision()));
        let (at, record) = (knight.position, knight.record);
        let mut world =
            World::enter_with_events(rom.image(), 0x0103, at.0, at.1 + 40, events).unwrap();
        for _ in 0..120 {
            world.update(None, Presses::NONE).unwrap();
        }
        assert!(
            world.life().0 < 28,
            "{:?}: {:?}",
            rom.revision(),
            world.life()
        );
        assert_eq!(world.foe_life(record), Some(23));
    }
}
