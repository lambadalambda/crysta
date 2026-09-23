//! Walking the underworld's plane against the native route's legs.
use assets::maps::visual::world::WorldMap;
use crysta_runtime::plane::Plane;
use rom::{Revision, Rom};
use room_core::Direction;
use std::path::Path;

fn owned_rom() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    let rom = Rom::load(&std::fs::read(path).ok()?).ok()?;
    (rom.revision() == Revision::Japan).then_some(rom)
}

#[test]
fn the_native_underworld_legs_rest_where_they_did() {
    // `tools/tower-approach-qualification/tower-route.jsonl`, from the
    // arrival at (536,544): each leg then 12 neutral frames.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let world = WorldMap::from_rom(cartridge.image(), 0x0003).unwrap();
    let mut plane = Plane::new(world.cells().to_vec(), 64, 64);
    let mut at = (536, 528);
    for _ in 0..16 {
        at = plane.tick(at, None);
    }
    assert_eq!(at, (536, 544), "underworld-arrival");
    let mut rests = Vec::new();
    for (direction, frames) in [
        (Direction::Down, 160),
        (Direction::Left, 160),
        (Direction::Down, 100),
        (Direction::Left, 120),
        (Direction::Right, 16),
    ] {
        for _ in 0..frames {
            at = plane.tick(at, Some(direction));
        }
        for _ in 0..12 {
            at = plane.tick(at, None);
        }
        rests.push(at);
    }
    assert_eq!(
        rests,
        [(536, 752), (392, 704), (408, 896), (184, 880), (216, 880)]
    );
}
