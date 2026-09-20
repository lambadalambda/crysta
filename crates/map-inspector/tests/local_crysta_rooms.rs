//! The Crysta slice as portable walkable rooms, built only from decoded ROM.
//!
//! This is the integration the slice needs: every map of `$000A..=$0021`
//! decoded to a collision grid and handed to the portable walking core, with
//! no per-room hand-written profile. It asserts the rooms are genuinely
//! walkable rather than merely constructible.
use assets::maps::visual::StaticBackground;
use rom::{Revision, Rom};
use room_core::{Direction, FrameInput, Room, WalkingState};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

/// Town exterior, the first house, and every building in it.
const CRYSTA: std::ops::RangeInclusive<u16> = 0x000A..=0x0021;

fn owned_rom() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    match std::fs::read(&path) {
        Ok(bytes) => {
            let rom = Rom::load(&bytes).unwrap();
            assert_eq!(rom.revision(), Revision::Japan);
            Some(rom)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping: local Japanese ROM not present");
            None
        }
        Err(e) => panic!("reading {}: {e}", path.display()),
    }
}

/// Builds a walkable room for any Crysta map from decoded ROM data alone.
fn crysta_room(cartridge: &Rom, map: u16) -> (Room, u16, u16) {
    let background = StaticBackground::from_rom(cartridge.image(), map)
        .unwrap_or_else(|e| panic!("map {map:#06x} background: {e}"));
    let attributes: &[u8; 512] = background.resources()[3]
        .decoded()
        .try_into()
        .expect("512-byte attribute table");
    let cells: Vec<u16> = background
        .layer()
        .attributed_cells(attributes)
        .iter()
        .map(|c| c.raw())
        .collect();
    let width = u16::try_from(background.layer().width()).unwrap();
    let height = u16::try_from(background.layer().height()).unwrap();
    let room =
        Room::new(width, height, cells).unwrap_or_else(|e| panic!("map {map:#06x} room: {e:?}"));
    (room, width, height)
}

/// Any cell the portable core accepts a step from, searched rather than pinned.
fn walkable_start(room: &Room, width: u16, height: u16) -> Option<(u16, u16)> {
    for row in 1..height.saturating_sub(1) {
        for col in 1..width.saturating_sub(1) {
            let (x, y) = (col * 16 + 8, row * 16 + 8);
            let mut state = WalkingState::new(x, y);
            let admitted = (0..24).any(|_| {
                state
                    .step(
                        room,
                        FrameInput {
                            direction: Some(Direction::Right),
                        },
                    )
                    .is_ok_and(|out| out.dx != 0)
            });
            if admitted {
                return Some((x, y));
            }
        }
    }
    None
}

#[test]
fn every_crysta_map_becomes_a_walkable_portable_room() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let mut walked = 0;
    for map in CRYSTA {
        let (room, width, height) = crysta_room(&cartridge, map);
        assert_eq!(room.cells().len(), usize::from(width) * usize::from(height));
        let (x, y) = walkable_start(&room, width, height)
            .unwrap_or_else(|| panic!("map {map:#06x} has no cell the core can walk from"));
        // Walking must actually move the player, not merely avoid erroring.
        let mut state = WalkingState::new(x, y);
        let mut moved = 0i32;
        for _ in 0..48 {
            if let Ok(out) = state.step(
                &room,
                FrameInput {
                    direction: Some(Direction::Right),
                },
            ) {
                moved += i32::from(out.dx);
            }
        }
        assert!(
            moved > 0,
            "map {map:#06x} admitted no movement from ({x},{y})"
        );
        walked += 1;
    }
    assert_eq!(walked, 24, "the Crysta slice is 24 maps");
}

#[test]
fn rooms_refuse_to_walk_into_the_towns_impassable_band() {
    // Map $000A's attribute-25 band runs the full height of the town and was
    // measured solid; the portable core must agree with that measurement.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let (room, width, _) = crysta_room(&cartridge, 0x000A);
    let band: Vec<usize> = room
        .cells()
        .iter()
        .enumerate()
        .filter(|(_, c)| assets::maps::MapCell::from_raw(**c).base_attribute() == 25)
        .map(|(i, _)| i)
        .collect();
    assert!(!band.is_empty(), "the town has an attribute-25 band");
    let mut refused = 0;
    // Guards against passing vacuously: a player that never moves also never
    // crosses the band.
    let mut approached = 0;
    for index in band {
        let (col, row) = (index % usize::from(width), index / usize::from(width));
        // Start three cells back so the player actually walks before stopping.
        // Starting flush against the band refuses on the first frame, which
        // would pass without demonstrating anything.
        let Some(start) = col.checked_sub(3) else {
            continue;
        };
        let walkable = |c: usize| {
            assets::maps::MapCell::from_raw(room.cells()[row * usize::from(width) + c])
                .qualified_passability()
                == Some(assets::maps::Passability::Walkable)
        };
        if !(start..col).all(walkable) {
            continue;
        }
        let (x, y) = (
            u16::try_from(start * 16 + 8).unwrap(),
            u16::try_from(row * 16 + 8).unwrap(),
        );
        let mut state = WalkingState::new(x, y);
        let mut crossed = false;
        let mut moved = 0i32;
        for _ in 0..64 {
            if let Ok(out) = state.step(
                &room,
                FrameInput {
                    direction: Some(Direction::Right),
                },
            ) {
                moved += i32::from(out.dx);
                if usize::from(out.x) / 16 >= col {
                    crossed = true;
                    break;
                }
            }
        }
        assert!(!crossed, "walked into the band at cell ({col},{row})");
        refused += 1;
        if moved > 0 {
            approached += 1;
        }
    }
    assert!(refused > 0, "no approach to the band was testable");
    assert!(
        approached > 0,
        "every approach was stationary, so nothing was actually refused"
    );
}

/// Cells the player can occupy, by the measured passability partition.
fn open_grid(room: &Room, width: u16, height: u16) -> Vec<bool> {
    room.cells()
        .iter()
        .map(|raw| {
            assets::maps::MapCell::from_raw(*raw).qualified_passability()
                == Some(assets::maps::Passability::Walkable)
        })
        .take(usize::from(width) * usize::from(height))
        .collect()
}

/// Maps reachable today by walking decoded geometry through decoded exits.
///
/// This is the measured frontier, not the target. See
/// `meta/issues/decode-door-entry-trigger.md` for what the rest needs.
const REACHABLE_TODAY: [u16; 6] = [0x000A, 0x000C, 0x000D, 0x000F, 0x0010, 0x0011];

#[test]
fn walking_decoded_geometry_reaches_the_measured_set_of_maps() {
    // Connectivity over decoded data only: four-directional movement across
    // cells the collision decode calls walkable, plus the static exit records.
    // Nothing here is a hand-written room graph.
    //
    // It deliberately asserts the exact set rather than a lower bound, so that
    // decoding a new exit or attribute fails this test and forces the frontier
    // to be restated rather than quietly drifting.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let mut grids: HashMap<u16, (Vec<bool>, u16, u16)> = HashMap::new();
    let mut exits: HashMap<u16, Vec<assets::maps::exits::ExitRecord>> = HashMap::new();
    for map in CRYSTA {
        let (room, width, height) = crysta_room(&cartridge, map);
        grids.insert(map, (open_grid(&room, width, height), width, height));
        let list = assets::maps::exits::ExitList::from_rom(cartridge.image(), map)
            .unwrap_or_else(|e| panic!("map {map:#06x} exits: {e}"));
        exits.insert(map, list.records().to_vec());
    }

    // The fresh game begins in the bedroom, map $000F.
    let start = (0x000Fu16, 19usize, 7usize);
    let mut seen: HashSet<(u16, usize, usize)> = HashSet::new();
    let mut maps: HashSet<u16> = HashSet::new();
    let mut queue = VecDeque::new();
    seen.insert(start);
    maps.insert(start.0);
    queue.push_back(start);

    while let Some((map, col, row)) = queue.pop_front() {
        let (grid, width, height) = &grids[&map];
        // Standing anywhere in an exit rectangle leaves the map. The record's
        // own fine test is tighter than this, so treating the whole rectangle
        // as a trigger is the generous reading; it still does not connect the
        // town, which is the point.
        for record in &exits[&map] {
            let (rx, ry) = (usize::from(record.x()), usize::from(record.y()));
            let (rw, rh) = (usize::from(record.width()), usize::from(record.height()));
            if rw == 0 || rh == 0 || col < rx || row < ry || col >= rx + rw || row >= ry + rh {
                continue;
            }
            let Ok(dest) = record.direct_destination() else {
                continue; // Conditional destinations are not decoded.
            };
            if !grids.contains_key(&dest) {
                continue; // Leaves the slice, e.g. the wider world.
            }
            let (dx, dy) = record.destination_position();
            let node = (dest, usize::from(dx) / 16, usize::from(dy) / 16);
            maps.insert(dest);
            if seen.insert(node) {
                queue.push_back(node);
            }
        }
        for (ux, uy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
            let (Ok(nc), Ok(nr)) = (
                usize::try_from(i32::try_from(col).unwrap() + ux),
                usize::try_from(i32::try_from(row).unwrap() + uy),
            ) else {
                continue;
            };
            if nc >= usize::from(*width) || nr >= usize::from(*height) {
                continue;
            }
            if !grid[nr * usize::from(*width) + nc] {
                continue;
            }
            let node = (map, nc, nr);
            if seen.insert(node) {
                queue.push_back(node);
            }
        }
    }

    let mut reached: Vec<u16> = maps.into_iter().collect();
    reached.sort_unstable();
    assert_eq!(
        reached, REACHABLE_TODAY,
        "reachable map set changed; restate the frontier"
    );
}

#[test]
fn most_town_entrances_sit_on_cells_the_player_cannot_stand_on() {
    // This is what stops the other eighteen maps, stated as a fact about the
    // data. Each town entrance is a 1x1 rectangle, and the fine geometry test
    // admits a single origin for those, which for seven of the eight would put
    // the player's body inside a solid wall cell. The eighth is walkable, and
    // is unreached for a different reason: no decoded path leads to it.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let (room, width, _) = crysta_room(&cartridge, 0x000A);
    let (mut solid, mut walkable) = (0, 0);
    for record in assets::maps::exits::ExitList::from_rom(cartridge.image(), 0x000A)
        .unwrap()
        .records()
    {
        if record.width() != 1 || record.height() != 1 {
            continue;
        }
        let (col, row) = (usize::from(record.x()), usize::from(record.y()));
        let cell = assets::maps::MapCell::from_raw(room.cells()[row * usize::from(width) + col]);
        // Whatever the doorway cell is, the cell below it is standable: the
        // player approaches from there.
        let below =
            assets::maps::MapCell::from_raw(room.cells()[(row + 1) * usize::from(width) + col]);
        assert_eq!(
            below.qualified_passability(),
            Some(assets::maps::Passability::Walkable),
            "cell below entrance ({col},{row}) is not standable"
        );
        match cell.qualified_passability() {
            Some(assets::maps::Passability::Solid) => solid += 1,
            Some(assets::maps::Passability::Walkable) => walkable += 1,
            other => panic!("entrance ({col},{row}) has passability {other:?}"),
        }
    }
    assert_eq!(
        (solid, walkable),
        (7, 1),
        "town entrance cell classification"
    );
}
