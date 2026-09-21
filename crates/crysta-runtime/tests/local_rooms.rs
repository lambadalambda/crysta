//! ROM-backed checks that every Crysta map becomes a walkable room.
//!
//! Skipped when the authenticated local dump is absent, as the other
//! ROM-backed suites are.

use crysta_runtime::{room, MAPS};
use rom::{Revision, Rom};
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
fn every_crysta_map_builds_a_room() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let mut built = 0;
    for map in MAPS {
        let built_room =
            room(cartridge.image(), map).unwrap_or_else(|error| panic!("map {map:#06x}: {error}"));
        assert_eq!(built_room.map, map);
        assert!(
            built_room.width > 0 && built_room.height > 0,
            "map {map:#06x} has an empty grid"
        );
        assert_eq!(
            built_room.room.cells().len(),
            usize::from(built_room.width) * usize::from(built_room.height),
            "map {map:#06x} cell count must match its dimensions"
        );
        built += 1;
    }
    assert_eq!(built, 24, "the slice spans 24 maps");
}

#[test]
fn a_room_admits_more_than_a_qualified_corridor() {
    // The point of the free-roam runtime: no sample halo, so the room admits
    // every cell its material policy allows rather than a hand-drawn window.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let built = room(cartridge.image(), 0x000B).unwrap();
    let walkable = built.walkable_cells();
    assert!(
        walkable > 100,
        "map $000B admitted only {walkable} cells, which is a corridor"
    );
}

#[test]
fn the_town_map_needs_its_qualified_alias_to_build() {
    // Map $0A is covered by TownSolid25 across the whole grid. Without the
    // alias the town band's attribute 25 is undecided and the room is refused,
    // which is what makes installing it meaningful rather than decorative.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    assert!(room(cartridge.image(), 0x000A).is_ok());
}

#[test]
fn a_map_outside_the_slice_is_refused() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    assert!(room(cartridge.image(), 0x0009).is_err());
    assert!(room(cartridge.image(), 0x0022).is_err());
}

#[test]
fn several_maps_share_one_layer_and_occupy_distinct_regions_of_it() {
    use assets::maps::exits::ExitList;
    use std::collections::BTreeSet;
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();

    // The six southern town houses are one 48x32 layer with six walkable
    // regions, and each map's ROM-declared arrivals land in a distinct one.
    // That bijection is what says a map is a region rather than a layer.
    let shared = room(image, 0x001A).unwrap();
    let regions = shared.regions();
    assert_eq!(regions.count(), 6, "the $1A layer holds six rooms");

    let mut occupied = BTreeSet::new();
    for map in 0x001A..=0x001Fu16 {
        assert_eq!(
            room(image, map).unwrap().room.cells(),
            shared.room.cells(),
            "map {map:#06x} must share the $1A layer"
        );
        let mut landed = BTreeSet::new();
        for source in MAPS {
            let Ok(list) = ExitList::from_rom(image, source) else {
                continue;
            };
            for record in list.records() {
                if record.direct_destination().ok() != Some(map) {
                    continue;
                }
                let (x, y) = record.destination_position();
                if let Some(region) = regions.at(x / 16, y / 16) {
                    landed.insert(region);
                }
            }
        }
        assert_eq!(
            landed.len(),
            1,
            "map {map:#06x} arrivals must agree on one region, got {landed:?}"
        );
        occupied.extend(landed);
    }
    assert_eq!(
        occupied.len(),
        6,
        "the six maps must occupy six distinct regions, got {occupied:?}"
    );
}

#[test]
fn a_shared_layers_regions_are_separated_by_collision() {
    // The consequence that makes the shared layer safe for free roam: the
    // player cannot walk out of their map's room into a neighbouring map's,
    // because the regions are not connected.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let shared = room(cartridge.image(), 0x000B).unwrap();
    let regions = shared.regions();
    assert!(regions.count() >= 6, "the house layer holds several rooms");
    for row in 0..shared.height {
        for column in 0..shared.width {
            let Some(here) = regions.at(column, row) else {
                continue;
            };
            for (next_column, next_row) in [(column + 1, row), (column, row + 1)] {
                if let Some(there) = regions.at(next_column, next_row) {
                    assert_eq!(here, there, "adjacent standable cells must share a region");
                }
            }
        }
    }
}
