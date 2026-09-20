//! The Crysta slice as portable walkable rooms, built only from decoded ROM.
//!
//! This is the integration the slice needs: every map of `$000A..=$0021`
//! decoded to a collision grid and handed to the portable walking core, with
//! no per-room hand-written profile. It asserts the rooms are genuinely
//! walkable rather than merely constructible.
use assets::maps::visual::StaticBackground;
use rom::{Revision, Rom};
use room_core::{Direction, FrameInput, Room, WalkingState};
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
