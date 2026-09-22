//! ROM-backed integration of source-bound arrival ownership into both exit paths.
use assets::maps::exits::ExitList;
use crysta_runtime::world::{new_game_flags, Step, World};
use rom::Rom;
use room_core::{
    arrival::{Arrival, ReturnRoute},
    Direction,
};

fn owned_rom() -> Option<Rom> {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../local/Tenchi Souzou (Japan).sfc"
    ))
    .ok()?;
    Some(Rom::load(&bytes).unwrap())
}

#[test]
fn transition_paths_own_arrival_until_free_despite_hostile_inputs() {
    let Some(rom) = owned_rom() else { return };
    for (source, destination, x, y, route, advances) in [
        (0x1E, 0xA, 632, 208, ReturnRoute::Town, 36),
        (0x19, 0x17, 936, 336, ReturnRoute::Stairs, 78),
    ] {
        for interaction in [false, true] {
            let (start_y, direction) = if source == 0x1E {
                (y - 4, Direction::Down)
            } else {
                (y + 4, Direction::Up)
            };
            let mut world =
                World::enter_candidate(rom.image(), source, x, start_y, new_game_flags()).unwrap();
            // A genuine checked approach arms walking exits before entering their fine band.
            if interaction {
                world.face(direction);
                assert_eq!(
                    world.interact_checked().unwrap(),
                    Step::Entered {
                        from: source,
                        to: destination
                    }
                );
            } else {
                for _ in 0..20 {
                    assert!(!matches!(
                        world.step_checked(Some(direction)).unwrap(),
                        Step::Refused(_)
                    ));
                    if world.map() != source {
                        break;
                    }
                }
            }
            assert_eq!(world.map(), destination);
            let mut expected = Arrival::new(route);
            assert_eq!(world.position(), expected.position());
            assert_eq!(world.arrival(), Some(expected));
            let events = world.events().to_vec();
            for _ in 0..advances {
                world.face(Direction::Up);
                assert!(world.talk().is_none());
                assert_eq!(world.interact_checked().unwrap(), Step::Stayed);
                assert_eq!(world.events(), events);
                expected.advance();
                assert!(!matches!(
                    world.step_checked(Some(Direction::Left)).unwrap(),
                    Step::Refused(_)
                ));
                assert_eq!(world.position(), expected.position());
                assert_eq!(world.arrival(), expected.owns_player().then_some(expected));
                assert_eq!(
                    world.map(),
                    destination,
                    "reverse exit must not fire during arrival"
                );
            }
            assert_eq!(world.arrival(), None);
            assert_eq!(world.facing(), Direction::Down);
            let endpoint = world.position();
            for _ in 0..12 {
                assert_eq!(world.step_checked(None).unwrap(), Step::Stayed);
            }
            for _ in 0..24 {
                assert!(!matches!(
                    world.step_checked(Some(Direction::Down)).unwrap(),
                    Step::Refused(_)
                ));
            }
            assert_eq!(world.map(), destination);
            assert!(
                world.position().1 > endpoint.1,
                "ordinary walking resumes without bounce"
            );
        }
    }
}

#[test]
fn explicit_entry_remains_raw_and_unqualified_target_records_fail_closed() {
    let Some(rom) = owned_rom() else { return };
    for (source, destination, offset, raw) in [
        (0x1E, 0xA, 0x18F9B, (784, 752)),
        (0x19, 0x17, 0x18F42, (448, 352)),
    ] {
        let placed =
            World::enter_candidate(rom.image(), destination, raw.0, raw.1, new_game_flags())
                .unwrap();
        assert_eq!(placed.position(), raw);
        assert_eq!(placed.arrival(), None);
        // Every operand is part of admission, not just the low selector nibble.
        for byte in 0..12 {
            let mut image = rom.image().to_vec();
            image[offset + byte] ^= if byte == 7 { 0x50 } else { 2 };
            let list = ExitList::from_rom(&image, source).unwrap();
            let record = list
                .records()
                .iter()
                .find(|r| r.source_range().start == offset)
                .unwrap();
            let mut world = World::enter_candidate(
                &image,
                source,
                u16::from(record.x()) * 16 + 8,
                u16::from(record.y()) * 16 + 16,
                new_game_flags(),
            )
            .unwrap();
            assert!(
                world.interact_checked().is_err(),
                "source{source:X} changed byte{byte}"
            );
            assert_eq!(world.map(), source);
            assert!(world.arrival().is_none());
        }
    }
}
