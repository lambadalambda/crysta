//! ROM-backed checks that residents' scripts run: the wanderer wanders, the
//! rest stand still, and nobody walks through anybody.

use crysta_runtime::world::World;
use rom::{Revision, Rom};
use room_core::Direction;
use std::collections::BTreeSet;
use std::path::Path;

fn owned_rom() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join("Tenchi Souzou (Japan).sfc");
    let bytes = std::fs::read(path).ok()?;
    let cartridge = Rom::load(&bytes).expect("local dump must authenticate");
    (cartridge.revision() == Revision::Japan).then_some(cartridge)
}

#[test]
fn the_wanderer_wanders_inside_their_rectangle_and_nobody_else_moves() {
    // `$83:8CB4`'s loop is `COP 26 04 0A 29 29`. The handler lets the actor
    // one cell past the far operands, so columns 4..=11 and rows 41..=42;
    // the room's cells there are all passable.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut world = World::enter(image, 0x000D, 200, 700).unwrap();
    let index = world
        .residents()
        .iter()
        .position(|resident| resident.record == 0x03_8CB4)
        .expect("the wanderer is present");
    let others: Vec<_> = world
        .residents()
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != index)
        .map(|(_, resident)| (resident.record, resident.position))
        .collect();
    let mut cells = BTreeSet::new();
    let mut walked_frames = 0;
    for _ in 0..4000 {
        world.step(None);
        let wanderer = &world.residents()[index];
        let (column, row) = wanderer.collision_cell();
        assert!((4..=11).contains(&column), "{:?}", wanderer.position);
        assert!((41..=42).contains(&row), "{:?}", wanderer.position);
        if wanderer.walking {
            walked_frames += 1;
            assert!((3..=5).contains(&wanderer.selector));
        }
        cells.insert(column);
    }
    assert!(cells.len() >= 3, "the wanderer stayed put: {cells:?}");
    assert!(walked_frames > 100);
    let still: Vec<_> = world
        .residents()
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != index)
        .map(|(_, resident)| (resident.record, resident.position))
        .collect();
    assert_eq!(still, others, "a resident without a walk loop moved");
}

#[test]
fn a_walking_resident_still_blocks_the_player_where_they_stand() {
    // Follow the wanderer: after they have moved, the cell they occupy is
    // solid, and the cell they left is open again.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut world = World::enter(image, 0x000D, 200, 700).unwrap();
    let index = world
        .residents()
        .iter()
        .position(|resident| resident.record == 0x03_8CB4)
        .unwrap();
    let start = world.residents()[index].collision_cell();
    let mut moved = None;
    for _ in 0..4000 {
        world.step(None);
        let now = world.residents()[index].collision_cell();
        if now != start && !world.residents()[index].walking {
            moved = Some(now);
            break;
        }
    }
    let now = moved.expect("the wanderer moved to another cell");
    let width = usize::from(world.dimensions().0);
    let cells = world.room().cells();
    assert_eq!(
        cells[usize::from(now.1) * width + usize::from(now.0)] >> 9,
        14
    );
    assert_ne!(
        cells[usize::from(start.1) * width + usize::from(start.0)] >> 9,
        14
    );
}

#[test]
fn a_walker_stops_and_faces_a_player_who_faces_them_and_walks_on_when_they_look_away() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let origin = World::enter(image, 0x000D, 200, 700)
        .unwrap()
        .residents()
        .iter()
        .find(|resident| resident.record == 0x03_8CB4)
        .expect("the wanderer is present")
        .position;
    // One cell to the wanderer's right, looking left at them.
    let mut world = World::enter(image, 0x000D, origin.0 + 16, origin.1).unwrap();
    world.face(Direction::Left);
    let index = world
        .residents()
        .iter()
        .position(|resident| resident.record == 0x03_8CB4)
        .unwrap();
    let cell = world.residents()[index].collision_cell();
    for _ in 0..600 {
        world.step(None);
        let wanderer = &world.residents()[index];
        assert_eq!(wanderer.collision_cell(), cell, "walked off while faced");
    }
    let wanderer = &world.residents()[index];
    assert_eq!(
        (wanderer.selector, wanderer.hflip),
        (2, false),
        "faces right"
    );
    assert!(!wanderer.walking);
    // From below, facing up: the wanderer faces down, so the vertical
    // convention is checked too.
    let mut below = World::enter(image, 0x000D, origin.0, origin.1 + 16).unwrap();
    below.face(Direction::Up);
    for _ in 0..100 {
        below.step(None);
    }
    let wanderer = &below.residents()[index];
    assert_eq!(
        (wanderer.selector, wanderer.hflip),
        (0, false),
        "faces down"
    );
    assert_eq!(wanderer.collision_cell(), cell);
    // Looking away releases them.
    world.face(Direction::Up);
    let mut cells = BTreeSet::new();
    for _ in 0..4000 {
        world.step(None);
        cells.insert(world.residents()[index].collision_cell());
    }
    assert!(cells.len() >= 2, "stayed put after the player looked away");
}
