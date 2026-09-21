//! ROM-backed checks that the player moves between maps through real exits.

use assets::maps::exits::ExitList;
use crysta_runtime::{room, world::World, MAPS};
use rom::{Revision, Rom};
use room_core::Direction;
use std::collections::{BTreeSet, HashSet, VecDeque};
use std::path::Path;

fn owned_rom() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join("Tenchi Souzou (Japan).sfc");
    let bytes = std::fs::read(path).ok()?;
    let cartridge = Rom::load(&bytes).expect("local dump must authenticate");
    (cartridge.revision() == Revision::Japan).then_some(cartridge)
}

/// Every exit trigger in the slice whose destination is also in the slice.
fn triggers(image: &[u8], map: u16) -> Vec<((u16, u16), u16)> {
    let Ok(list) = ExitList::from_rom(image, map) else {
        return Vec::new();
    };
    list.records()
        .iter()
        .filter_map(|record| {
            let destination = record.direct_destination().ok()?;
            MAPS.contains(&destination).then(|| {
                let (x, y) = record.destination_position();
                ((x, y), destination)
            })
        })
        .collect()
}

#[test]
fn a_world_starts_where_it_is_told_and_knows_its_map() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let world = World::enter(cartridge.image(), 0x000B, 120, 112).unwrap();
    assert_eq!(world.map(), 0x000B);
    assert_eq!(world.position(), (120, 112));
}

#[test]
fn every_slice_map_can_be_entered_at_a_declared_arrival() {
    // Each map's arrivals come from other maps' exit records, so entering at
    // one is entering where the game itself sends the player.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut entered = 0;
    for map in MAPS {
        for source in MAPS {
            for ((x, y), destination) in triggers(image, source) {
                if destination != map {
                    continue;
                }
                let world = World::enter(image, map, x, y)
                    .unwrap_or_else(|error| panic!("map {map:#06x} at ({x},{y}): {error}"));
                assert_eq!(world.map(), map);
                entered += 1;
            }
        }
    }
    assert!(entered > 20, "only {entered} arrivals were exercised");
}

/// Walks the slice from one start, returning every map reached.
///
/// A breadth-first search over `(map, cell)`, where every edge is realised by
/// actually stepping the world. Searching pixel positions instead would never
/// get anywhere: a step moves about a pixel and a half, so a doorway sixty
/// frames away sits far below any reachable BFS depth.
fn reachable_maps(image: &[u8], start: u16, x: u16, y: u16) -> BTreeSet<u16> {
    const DIRECTIONS: [Direction; 4] = [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ];
    let cell = |world: &World| {
        let (x, y) = world.position();
        (world.map(), x / 16, y / 16)
    };
    let origin = World::enter(image, start, x, y).expect("the start must build");
    let mut reached = BTreeSet::from([start]);
    let mut seen = HashSet::from([cell(&origin)]);
    let mut queue = VecDeque::from([origin]);
    while let Some(world) = queue.pop_front() {
        if seen.len() > 20_000 {
            break;
        }
        let mut found: Vec<World> = Vec::new();
        for direction in DIRECTIONS {
            // Walk until the cell changes or the step stalls.
            let mut next = world.clone();
            let from = cell(&world);
            for _ in 0..32 {
                next.step(Some(direction));
                if cell(&next) != from {
                    found.push(next);
                    break;
                }
            }
            // Face the neighbour and try to open it as a doorway. Most town
            // entrances sit on cells the player cannot walk onto.
            let mut opened = world.clone();
            opened.face(direction);
            opened.interact();
            if opened.map() != world.map() {
                found.push(opened);
            }
        }
        for next in found {
            if seen.insert(cell(&next)) {
                reached.insert(next.map());
                queue.push_back(next);
            }
        }
    }
    reached
}

#[test]
fn the_player_can_walk_out_of_the_opening_house() {
    // The first thing free roam has to do that the qualified corridor could
    // not: leave a room by walking into its doorway.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let reached = reachable_maps(cartridge.image(), 0x000B, 120, 112);
    assert!(
        reached.len() > 1,
        "the player never left map $000B, reaching only {reached:?}"
    );
}

#[test]
fn walking_and_doorways_connect_most_of_the_slice() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let reached = reachable_maps(cartridge.image(), 0x000B, 120, 112);
    // The town itself must be among them, or nothing else in the slice opens up.
    assert!(
        reached.contains(&0x000A),
        "the town was not reached: {reached:?}"
    );
    // 19 of the 24 are reachable by walking and opening doorways. The five that
    // are not are recorded rather than rounded away: $1A, $1B and $1C are
    // southern town houses, and $20 and $21 are the cellar and Pandora's Box,
    // which the route reaches through progression rather than geometry.
    let missing: Vec<_> = MAPS.filter(|map| !reached.contains(map)).collect();
    assert_eq!(
        missing,
        vec![0x001A, 0x001B, 0x001C, 0x0020, 0x0021],
        "reachability changed; reached {reached:?}"
    );
}

#[test]
fn an_arrival_does_not_immediately_bounce_back() {
    // The player arrives standing on geometry that is often an exit in its own
    // right, since a doorway leads back the way it came. Until they step clear
    // of it, it must not fire, or two maps ping-pong forever.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    for map in MAPS {
        for source in MAPS {
            for ((x, y), destination) in triggers(image, source) {
                if destination != map {
                    continue;
                }
                let mut world = World::enter(image, map, x, y).unwrap();
                let position = world.position();
                for _ in 0..8 {
                    world.step(None);
                }
                assert_eq!(world.map(), map, "map {map:#06x} bounced after arriving");
                assert_eq!(world.position(), position);
            }
        }
    }
}

#[test]
fn an_exit_leaving_the_slice_never_transitions() {
    // Walking the whole reachable state space must never land the player in a
    // map outside the slice, however many out-of-slice exits it crosses.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut leaving = 0;
    for map in MAPS {
        let Ok(list) = ExitList::from_rom(image, map) else {
            continue;
        };
        leaving += list
            .records()
            .iter()
            .filter(|record| {
                record
                    .direct_destination()
                    .is_ok_and(|destination| !MAPS.contains(&destination))
            })
            .count();
    }
    assert!(leaving > 0, "the slice has exits leaving it");
    for map in reachable_maps(image, 0x000B, 120, 112) {
        assert!(
            MAPS.contains(&map),
            "walked out of the slice into {map:#06x}"
        );
    }
}

#[test]
fn walking_is_still_bounded_by_collision() {
    // Transitions must not have loosened movement: a solid neighbour still
    // blocks, and the player stays inside the grid.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let built = room(image, 0x000B).unwrap();
    let mut world = World::enter(image, 0x000B, 120, 112).unwrap();
    for _ in 0..200 {
        for direction in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            world.step(Some(direction));
            let (x, y) = world.position();
            if world.map() != 0x000B {
                return;
            }
            assert!(
                x < built.width * 16 && y < built.height * 16,
                "walked outside the grid to ({x},{y})"
            );
        }
    }
}

#[test]
fn the_player_can_walk_up_to_a_resident_and_talk() {
    use crysta_runtime::residents::Conversation;
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    // The documented resident stands at (120,112) in map $000B. Stand in the
    // cell below and face them.
    let mut world = World::enter(image, 0x000B, 120, 112 + 16).unwrap();
    assert!(
        world.residents().iter().any(|r| r.cell() == (7, 7)),
        "the resident must be present"
    );
    world.face(Direction::Up);
    match world.talk() {
        Some(Conversation::Speaks { pages, .. }) => assert_eq!(pages.len(), 2),
        other => panic!("expected the documented two pages, got {other:?}"),
    }
    // Facing away reaches nobody.
    world.face(Direction::Down);
    assert!(world.talk().is_none());
}

#[test]
fn a_resident_who_is_a_body_stops_the_player() {
    // Occupancy blocks the collision cell, one row above the visual one,
    // because movement samples at (x - 8, y - 16). It is applied on entry to
    // residents that decode to a body, and to nobody else.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut world = World::enter(image, 0x000B, 120, 112 + 16).unwrap();
    let resident = world
        .residents()
        .iter()
        .find(|resident| resident.cell() == (7, 7))
        .expect("the documented resident");
    assert!(resident.body);
    assert_eq!(resident.collision_cell(), (7, 6));
    let index = usize::from(resident.collision_cell().1) * usize::from(world.dimensions().0)
        + usize::from(resident.collision_cell().0);
    assert_eq!(
        world.room().cells()[index] >> 9,
        14,
        "the collision cell must carry a solid attribute"
    );
    // Script-only records are not bodies.
    assert!(world.residents().iter().any(|resident| !resident.body));
    for _ in 0..64 {
        world.step(Some(Direction::Up));
    }
    assert!(
        world.position().1 >= 128,
        "the resident must stop the player, ended at {:?}",
        world.position()
    );
}

#[test]
fn talking_applies_the_flags_the_script_writes() {
    use crysta_runtime::residents::Conversation;
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    // On a new game the callback's dispatch falls through to the documented
    // arm, whose script writes the $0026 progression flag; check the world
    // records it.
    let events = crysta_runtime::world::new_game_flags();
    let mut world = World::enter_with_events(image, 0x000B, 120, 112 + 16, events).unwrap();
    world.face(Direction::Up);
    let spoken = world.talk().expect("the resident is there");
    let Conversation::Speaks { flags, .. } = &spoken else {
        panic!("expected pages, got {spoken:?}");
    };
    // The script writes $0026 with bit 15 set, which means set rather than clear.
    let written = flags
        .iter()
        .find(|flag| *flag & 0x0FFF == 0x0026)
        .copied()
        .expect("the progression flag must be written");
    assert_ne!(written & 0x8000, 0, "bit 15 selects set over clear");
    assert_ne!(
        world.events()[0x026 / 8] & (1 << (0x026 % 8)),
        0,
        "the progression flag must be set after talking"
    );
}
