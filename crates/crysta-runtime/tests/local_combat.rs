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
