//! The Crysta slice as portable walkable rooms, built only from decoded ROM.
//!
//! This is the integration the slice needs: every map of `$000A..=$0021`
//! decoded to a collision grid and handed to the portable walking core, with
//! no per-room hand-written profile. It asserts the rooms are genuinely
//! walkable rather than merely constructible.
use assets::maps::visual::StaticBackground;
use rom::{Revision, Rom};
use room_core::{Direction, FrameInput, MaterialAlias, MaterialRule, Room, WalkingState};
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
    let room = room
        .with_material_policy(qualified_policy(map, width, height))
        .unwrap_or_else(|e| panic!("map {map:#06x} policy: {e:?}"));
    (room, width, height)
}

/// Classifications `room-core` already qualified, scoped to their own cells.
///
/// Its default table leaves attributes 5, 25 and 29 undecided, but it carries
/// aliases for them that a room must opt into. Installing them is what lets
/// the core agree with the measured town band and traverse the stairs instead
/// of refusing. The alias scopes are validated by `room-core`, so a wrong cell
/// is rejected rather than silently accepted.
fn qualified_policy(map: u16, width: u16, height: u16) -> Vec<MaterialRule> {
    match map {
        0x000A => vec![MaterialRule {
            bounds: [0, 0, width, height],
            direction: None,
            alias: MaterialAlias::TownSolid25,
        }],
        0x000C => vec![
            MaterialRule {
                bounds: [11, 21, 12, 22],
                direction: Some(Direction::Up),
                alias: MaterialAlias::ClosedDoorPartial5,
            },
            MaterialRule {
                bounds: [11, 21, 12, 22],
                direction: Some(Direction::Up),
                alias: MaterialAlias::StairOpen29,
            },
        ],
        0x000E => vec![MaterialRule {
            bounds: [6, 53, 7, 54],
            direction: Some(Direction::Up),
            alias: MaterialAlias::StairOpen29,
        }],
        0x0020 => vec![MaterialRule {
            bounds: [22, 53, 23, 54],
            direction: Some(Direction::Up),
            alias: MaterialAlias::StairOpen29,
        }],
        _ => Vec::new(),
    }
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
fn the_portable_core_refuses_the_towns_impassable_band() {
    // The decode measured attribute 25 solid. room-core's default table has no
    // entry for 25 at all -- it carries a `TownSolid25` alias a room must opt
    // into -- so a room built without the policy refuses to classify the cell
    // instead of agreeing. `crysta_room` installs it, and this asserts the
    // agreement that results, plus the refusal without it.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let (room, width, _) = crysta_room(&cartridge, 0x000A);
    let bare = Room::new(width, room.height(), room.cells().to_vec()).unwrap();
    let mut checked = 0;
    for (index, raw) in room.cells().iter().enumerate() {
        if assets::maps::MapCell::from_raw(*raw).base_attribute() != 25 {
            continue;
        }
        let (col, row) = (index % usize::from(width), index / usize::from(width));
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
        let mut moved = 0i32;
        let mut state = WalkingState::new(x, y);
        for _ in 0..64 {
            match state.step(
                &room,
                FrameInput {
                    direction: Some(Direction::Right),
                },
            ) {
                Ok(out) => {
                    moved += i32::from(out.dx);
                    assert!(
                        usize::from(out.x) / 16 < col,
                        "walked into the band at ({col},{row})"
                    );
                }
                // Some approaches meet another undecoded attribute on the way.
                // What must not happen is a refusal on 25 itself, which the
                // installed policy exists to decide.
                Err(room_core::Unqualified::UnsupportedType(kind)) => {
                    assert_ne!(kind, 25, "the policy did not decide 25 at ({col},{row})");
                    break;
                }
                Err(e) => panic!("unexpected refusal at ({col},{row}): {e:?}"),
            }
        }
        assert!(moved > 0, "approach to ({col},{row}) never moved");

        // Without the policy the same approach ends unclassified, which is
        // what the alias exists to resolve.
        let mut bare_state = WalkingState::new(x, y);
        let refusal = (0..64).find_map(|_| {
            bare_state
                .step(
                    &bare,
                    FrameInput {
                        direction: Some(Direction::Right),
                    },
                )
                .err()
        });
        assert!(
            matches!(refusal, Some(room_core::Unqualified::UnsupportedType(_))),
            "expected an unclassified refusal without the policy at ({col},{row})"
        );
        checked += 1;
    }
    assert!(checked > 0, "no approach to the band was testable");
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

/// Maps reachable by walking decoded geometry alone, with no door interaction.
///
/// Doorways whose rectangle sits on solid cells cannot be walked into, so this
/// is the frontier without the interaction measured below.
const WALKING_ONLY: [u16; 6] = [0x000A, 0x000C, 0x000D, 0x000F, 0x0010, 0x0011];

/// Where an exit actually puts the player, as a cell the core can move from.
///
/// `destination_position` is documented as the raw landing before arrival
/// adjustment, and `room-core` samples collision a row above the position
/// word, so converting the landing with a bare `pixel / 16` disagrees with the
/// core for 16-aligned landings. This asks the core instead.
fn landing_cell(
    room: &Room,
    width: u16,
    height: u16,
    x: u16,
    y: u16,
    strict: bool,
) -> Option<(usize, usize)> {
    let offsets: &[(i32, i32)] = if strict {
        &[(0, 0)]
    } else {
        &[(0, 0), (0, 16), (0, -16), (8, 0), (-8, 0)]
    };
    for &(dx, dy) in offsets {
        let (px, py) = (
            u16::try_from(i32::from(x) + dx).ok()?,
            u16::try_from(i32::from(y) + dy).ok()?,
        );
        if usize::from(px) / 16 >= usize::from(width) || usize::from(py) / 16 >= usize::from(height)
        {
            continue;
        }
        let mut state = WalkingState::new(px, py);
        let usable = [
            Direction::Right,
            Direction::Left,
            Direction::Up,
            Direction::Down,
        ]
        .into_iter()
        .any(|direction| {
            let mut probe = state;
            (0..8).any(|_| {
                probe
                    .step(
                        room,
                        FrameInput {
                            direction: Some(direction),
                        },
                    )
                    .is_ok_and(|out| out.dx != 0 || out.dy != 0)
            })
        });
        if usable {
            let _ = &mut state;
            return Some((usize::from(px) / 16, usize::from(py) / 16));
        }
    }
    None
}

/// Connectivity over decoded data: four-directional movement across cells the
/// collision decode calls walkable, plus the static exit records.
///
/// `doorways` models the measured interaction: standing in a walkable cell
/// orthogonally adjacent to an exit rectangle, facing it and interacting.
fn reachable_maps(
    cartridge: &Rom,
    doorways: bool,
    strict_landing: bool,
    ignore_collision: bool,
) -> Vec<u16> {
    let mut grids: HashMap<u16, (Vec<bool>, u16, u16)> = HashMap::new();
    let mut rooms: HashMap<u16, Room> = HashMap::new();
    let mut exits: HashMap<u16, Vec<assets::maps::exits::ExitRecord>> = HashMap::new();
    for map in CRYSTA {
        let (room, width, height) = crysta_room(cartridge, map);
        let grid = if ignore_collision {
            vec![true; usize::from(width) * usize::from(height)]
        } else {
            open_grid(&room, width, height)
        };
        grids.insert(map, (grid, width, height));
        rooms.insert(map, room);
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
        for record in &exits[&map] {
            let (rx, ry) = (usize::from(record.x()), usize::from(record.y()));
            let (rw, rh) = (usize::from(record.width()), usize::from(record.height()));
            if rw == 0 || rh == 0 {
                continue;
            }
            let inside = col >= rx && row >= ry && col < rx + rw && row < ry + rh;
            let adjacent = doorways
                && ((rx..rx + rw).contains(&col) && (row + 1 == ry || row == ry + rh)
                    || (ry..ry + rh).contains(&row) && (col + 1 == rx || col == rx + rw));
            if !(inside || adjacent) {
                continue;
            }
            let Ok(dest) = record.direct_destination() else {
                continue; // Conditional destinations are not decoded.
            };
            if !grids.contains_key(&dest) {
                continue; // Leaves the slice, e.g. the wider world.
            }
            let (dx, dy) = record.destination_position();
            let (dw, dh) = (grids[&dest].1, grids[&dest].2);
            // Only count a map as reached if the core can move from where the
            // exit lands the player.
            let Some((lc, lr)) = landing_cell(&rooms[&dest], dw, dh, dx, dy, strict_landing) else {
                continue;
            };
            maps.insert(dest);
            let node = (dest, lc, lr);
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
    reached
}

#[test]
fn walking_alone_reaches_only_part_of_the_slice() {
    // Asserts the exact set rather than a lower bound, so decoding a new
    // mechanism fails this and forces the frontier to be restated.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    assert_eq!(
        reachable_maps(&cartridge, false, false, false),
        WALKING_ONLY
    );
    // The collision decode is what produces that number: treating every cell
    // as walkable opens the whole slice, so this test is sensitive to it.
    assert_eq!(reachable_maps(&cartridge, false, false, true).len(), 24);
}

#[test]
fn doorway_interaction_connects_the_whole_slice() {
    // Measured in the reference emulator: standing below the house's own front
    // door in the town at (504,768), facing Up and pressing the action button,
    // then holding Up, walks the player through the solid attribute-14 door
    // cell and into map $000D. Collision is suspended for that walk: the
    // player passes y=767 down to y=735, which ordinary movement refuses.
    //
    // This models that as "an exit rectangle is enterable from a walkable cell
    // orthogonally adjacent to it". One door was confirmed live; the model's
    // generality across the other town entrances is not yet verified, which is
    // why this asserts reachability rather than claiming the mechanism is
    // fully qualified.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let reached = reachable_maps(&cartridge, true, false, false);
    assert_eq!(reached.len(), 24, "reached {reached:x?}");
    assert!(CRYSTA.eq(reached.iter().copied()));
    // The record's landing is documented as raw, before arrival adjustment,
    // and the measured arrival for $000A -> $000D is two rows from it. Taking
    // the record literally therefore loses maps, and how many is worth
    // pinning: this is a model of arrival, not a decode of it.
    let strict = reachable_maps(&cartridge, true, true, false);
    assert_eq!(
        strict.len(),
        19,
        "literal landings reach {strict:x?}; restate if arrival is decoded"
    );
    // Stated rather than hidden: once doorways are enterable, the collision
    // decode no longer constrains reachability at all. This result is carried
    // by the exit graph, and `walking_alone_reaches_only_part_of_the_slice` is
    // where the collision decode is actually exercised.
    assert_eq!(reachable_maps(&cartridge, true, false, true), reached);
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

#[test]
fn a_played_traversal_is_blocked_on_the_collision_reference() {
    // Every other connectivity result here is a graph over the collision grid.
    // Simulating the walk through room-core frame by frame does not reproduce
    // them, and the reason is concrete rather than a bug in the route.
    //
    // In map $000A the first position the core will move from is (136,24).
    // Indexing that by `pos / 16` gives cell (8,1), whose attribute is 12 --
    // solid. The core nonetheless moves, first nudging the player down to
    // y=32. So `pos / 16` is not the cell room-core samples, and any path
    // expressed in those cells desynchronises from the simulated player.
    //
    // Fixing this means settling the collision reference, tracked in
    // meta/issues/decode-door-entry-trigger.md. Asserting the mismatch keeps
    // it visible and makes this test fail once it is resolved.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let (room, width, height) = crysta_room(&cartridge, 0x000A);
    let (sx, sy) = walkable_start(&room, width, height).expect("a walkable start");
    assert_eq!((sx, sy), (136, 24));
    let naive = (usize::from(sx) / 16, usize::from(sy) / 16);
    let cell =
        assets::maps::MapCell::from_raw(room.cells()[naive.1 * usize::from(width) + naive.0]);
    assert_eq!(cell.base_attribute(), 12);
    assert_eq!(
        cell.qualified_passability(),
        Some(assets::maps::Passability::Solid),
        "the naive cell for a position the core walks from is solid"
    );

    // And the core relocates the player before moving horizontally.
    let mut state = WalkingState::new(sx, sy);
    let mut first_horizontal = None;
    for _ in 0..24 {
        let out = state
            .step(
                &room,
                FrameInput {
                    direction: Some(Direction::Right),
                },
            )
            .expect("the core moves from this position");
        if out.dx != 0 {
            first_horizontal = Some((out.x, out.y));
            break;
        }
    }
    let (_, y) = first_horizontal.expect("the player eventually moves right");
    assert_ne!(
        y, sy,
        "the core nudged the player before moving horizontally"
    );
}

/// Positions the player can actually arrive at and move from.
///
/// `destination_position` is documented as the landing *before* arrival
/// adjustment, and the measured arrival for `$000A` differs from it by
/// `(8,17)`. Rather than fit that offset, this searches a small neighbourhood
/// for a position the core will move from.
fn arrival_starts(image: &[u8], room: &Room, map: u16, width: u16, height: u16) -> Vec<(u16, u16)> {
    let mut landings: Vec<(u16, u16)> = Vec::new();
    if map == 0x000F {
        landings.push((312, 120));
    }
    for source in CRYSTA {
        let Ok(list) = assets::maps::exits::ExitList::from_rom(image, source) else {
            continue;
        };
        for record in list.records() {
            if record.direct_destination() == Ok(map) {
                landings.push(record.destination_position());
            }
        }
    }
    landings
        .iter()
        .flat_map(|(x, y)| {
            (-16i32..17).step_by(8).flat_map(move |dy| {
                (-16i32..17).step_by(8).map(move |dx| {
                    (
                        u16::try_from(i32::from(*x) + dx).unwrap_or(0),
                        u16::try_from(i32::from(*y) + dy).unwrap_or(0),
                    )
                })
            })
        })
        .filter(|(x, y)| {
            usize::from(*x) / 16 < usize::from(width)
                && usize::from(*y) / 16 < usize::from(height)
                && [
                    Direction::Right,
                    Direction::Left,
                    Direction::Up,
                    Direction::Down,
                ]
                .into_iter()
                .any(|direction| steps_between(room, (*x, *y), direction).is_some())
        })
        .collect()
}

/// Positions whose exit probe lands in a rectangle, or one cell short of one.
///
/// Inverts `probe = (x - 8, y - 16)`, which is where the measured door
/// interaction happens. Exits leaving the slice are not onward destinations.
fn exit_approaches(list: &assets::maps::exits::ExitList) -> HashSet<(u16, u16)> {
    list.records()
        .iter()
        .filter(|record| record.width() != 0 && record.height() != 0)
        .filter(|record| {
            record
                .direct_destination()
                .is_ok_and(|destination| CRYSTA.contains(&destination))
        })
        .flat_map(|record| {
            let (rx, ry) = (i64::from(record.x()), i64::from(record.y()));
            let (rw, rh) = (i64::from(record.width()), i64::from(record.height()));
            // The rectangle itself, plus a one-cell border: a solid doorway is
            // approached from whichever side is open, and that side varies --
            // map $001A's is on the left edge, not below.
            (ry - 1..=ry + rh).flat_map(move |row| {
                (rx - 1..=rx + rw).map(move |col| {
                    (
                        u16::try_from(col * 16 + 8).unwrap_or(u16::MAX),
                        u16::try_from(row * 16 + 16).unwrap_or(u16::MAX),
                    )
                })
            })
        })
        .collect()
}

/// Simulated movement between lattice positions, 16 pixels apart.
///
/// Expressed in pixels rather than cells on purpose. `$80:940D` computes the
/// collision sample as `(x - 8, y - 16)`, the same corner the exit probe uses,
/// and `room-core` samples a 16x16 box from it. The player therefore occupies
/// a box, not a cell, and `pos / 16` is not a position it can be indexed by.
fn steps_between(room: &Room, from: (u16, u16), direction: Direction) -> Option<(u16, u16)> {
    let mut state = WalkingState::new(from.0, from.1);
    for _ in 0..48 {
        let out = state
            .step(
                room,
                FrameInput {
                    direction: Some(direction),
                },
            )
            .ok()?;
        let moved = (i64::from(out.x) - i64::from(from.0)).abs()
            + (i64::from(out.y) - i64::from(from.1)).abs();
        if moved >= 16 {
            return Some((out.x, out.y));
        }
    }
    None
}

#[test]
fn the_player_can_walk_to_an_exit_approach_in_every_crysta_map() {
    // A simulated walk, not a graph over cells: every edge below is produced by
    // stepping room-core until the player has actually moved a full cell.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let mut walked = 0;
    let mut explored = 0;
    let mut dead_ends = 0;
    let mut stranded: Vec<u16> = Vec::new();
    for map in CRYSTA {
        let (room, width, height) = crysta_room(&cartridge, map);
        let list = assets::maps::exits::ExitList::from_rom(cartridge.image(), map).unwrap();
        // Positions whose exit probe lands inside a rectangle, or one cell
        // short of one, which is where the measured door interaction happens.
        let approaches: HashSet<(u16, u16)> = exit_approaches(&list);
        assert!(
            !approaches.is_empty(),
            "map {map:#06x} has no exit approach"
        );

        let starts = arrival_starts(cartridge.image(), &room, map, width, height);
        assert!(
            !starts.is_empty(),
            "map {map:#06x}: nothing movable near its landings"
        );
        // Simulated movement does not land on a clean lattice -- stepping
        // right can nudge y -- so the visited set is quantised. Without this
        // the search wanders through near-duplicate positions and never
        // converges.
        let key = |(x, y): (u16, u16)| (x / 8, y / 8);
        let mut seen: HashSet<(u16, u16)> = starts.iter().copied().map(key).collect();
        let mut queue: VecDeque<(u16, u16)> = starts.iter().copied().collect();
        // The player lands on the approach of the door it came through, which
        // would satisfy this trivially. Require reaching a different one.
        let onward: HashSet<(u16, u16)> = approaches
            .iter()
            .map(|position| key(*position))
            .filter(|position| !seen.contains(position))
            .collect();
        let mut arrived = false;
        // Bounded. The largest map is 64x80 cells, and positions are
        // quantised to 8 pixels, so the reachable set is at most ~20k keys.
        for _ in 0..60_000 {
            let Some(position) = queue.pop_front() else {
                break;
            };
            if onward.contains(&key(position)) {
                arrived = true;
                break;
            }
            for direction in [
                Direction::Right,
                Direction::Left,
                Direction::Up,
                Direction::Down,
            ] {
                if let Some(next) = steps_between(&room, position, direction) {
                    if seen.insert(key(next)) {
                        queue.push_back(next);
                    }
                }
            }
        }
        // A map whose only exit is the one the player arrived through has no
        // onward approach; there the claim is just that the player can move.
        if onward.is_empty() {
            assert!(
                seen.len() > starts.len(),
                "map {map:#06x}: single exit, and the player cannot move"
            );
            dead_ends += 1;
        } else if arrived {
            assert!(
                seen.len() > starts.len(),
                "map {map:#06x} never stepped anywhere"
            );
        } else {
            stranded.push(map);
        }
        explored += seen.len();
        walked += 1;
    }
    assert_eq!(walked, 24);
    // Simulated stepping is expensive, so the totals are worth pinning: a
    // collapse here means the walk stopped exploring.
    assert!(explored > 1500, "only {explored} positions were walked");
    // Every map. This is the graph connectivity result confirmed by
    // simulation: the same adjacency model, but with each edge produced by
    // stepping room-core until the player actually moved a cell, rather than
    // asserted.
    //
    // That adjacency model is only *measured* for approach from below, which
    // is where the live door interaction was observed. Approach from the other
    // three sides is a generalisation, and map $001A needs it: its doorway is
    // on the left edge. See meta/issues/decode-door-entry-trigger.md.
    assert_eq!(dead_ends, 0, "maps with no onward exit at all");
    assert!(
        stranded.is_empty(),
        "maps with no walkable onward exit: {stranded:x?}"
    );
}

#[test]
fn the_traversal_depends_on_simulated_movement() {
    // Guards the result above against being an artefact of a generous approach
    // set: a room the core cannot move in at all must strand the player, and
    // the approach set must not already contain every position.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let (room, width, height) = crysta_room(&cartridge, 0x000A);
    let list = assets::maps::exits::ExitList::from_rom(cartridge.image(), 0x000A).unwrap();
    let approaches = exit_approaches(&list);
    let lattice = usize::from(width) * usize::from(height);
    assert!(
        approaches.len() * 8 < lattice,
        "approaches {} cover too much of {lattice} cells",
        approaches.len()
    );

    // A solid room admits no movement, so no simulated step can succeed.
    let solid = Room::new(width, height, vec![14 << 9; lattice]).unwrap();
    for direction in [
        Direction::Right,
        Direction::Left,
        Direction::Up,
        Direction::Down,
    ] {
        assert_eq!(steps_between(&solid, (136, 200), direction), None);
    }
    // And the real room does admit movement, so the helper is not vacuous.
    let start = arrival_starts(cartridge.image(), &room, 0x000A, width, height);
    assert!(!start.is_empty());
}
