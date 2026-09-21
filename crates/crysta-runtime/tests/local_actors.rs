//! ROM-backed checks that residents' scripts run: the wanderer wanders, the
//! rest stand still, and nobody walks through anybody.

use crysta_runtime::world::World;
use rom::{Revision, Rom};
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
