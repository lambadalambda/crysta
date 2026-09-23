//! ROM-backed checks that the player moves between maps through real exits.

use assets::maps::exits::ExitList;
use crysta_runtime::scene::Presses;
use crysta_runtime::{
    room,
    world::{Step, World, WorldError},
    MAPS,
};
use rom::{Revision, Rom};
use room_core::Direction;
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
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
fn outdoor_unknown_slope_refusal_is_atomic_but_interactive_host_can_escape() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    // Parent's screenshot-matching exterior position. This tests host recovery,
    // not native slope admission: type 6 must remain refused.
    let mut strict = World::enter(cartridge.image(), 0xA, 360, 472).unwrap();
    for _ in 0..2 {
        assert_eq!(
            strict.step_checked(Some(Direction::Right)).unwrap(),
            Step::Stayed
        );
    }
    let poised = strict.clone();
    let refused = Step::Refused(room_core::Unqualified::UnsupportedType(6));
    assert!(!poised.residents().is_empty());
    assert!(poised
        .residents()
        .iter()
        .any(|resident| resident.pose_age == 2));
    for (input, frames) in [
        (Some(Direction::Right), 30),
        (None, 8),
        (Some(Direction::Left), 32),
        (Some(Direction::Up), 32),
    ] {
        for _ in 0..frames {
            assert_eq!(strict.step_checked(input).unwrap(), refused);
            assert_eq!(strict.position(), (360, 472));
            assert_eq!(strict.residents(), poised.residents());
            assert_eq!(strict.events(), poised.events());
        }
    }
    for direction in [Direction::Left, Direction::Up] {
        let mut interactive = poised.clone();
        assert_eq!(
            interactive
                .step_interactive(Some(Direction::Right))
                .unwrap(),
            refused
        );
        assert_eq!(interactive.position(), (360, 472));
        assert_eq!(interactive.residents().len(), poised.residents().len());
        for (after, before) in interactive.residents().iter().zip(poised.residents()) {
            assert_eq!(after.record, before.record);
        }
        // Script pose changes can reset age; unchanged poses continue ticking.
        assert!(interactive
            .residents()
            .iter()
            .zip(poised.residents())
            .any(|(after, before)| after.pose_age == before.pose_age + 1));
        for _ in 0..8 {
            assert_eq!(interactive.step_interactive(None).unwrap(), Step::Stayed);
            assert_eq!(interactive.position(), (360, 472));
        }
        for _ in 0..32 {
            let before = interactive.position();
            if matches!(
                interactive.step_interactive(Some(direction)).unwrap(),
                Step::Refused(_)
            ) {
                // Escape may encounter another unsupported boundary; it too
                // must remain non-displacing rather than being admitted.
                assert_eq!(interactive.position(), before);
            }
        }
        let (x, y) = interactive.position();
        match direction {
            Direction::Left => assert!(x < 360),
            Direction::Up => assert!(y < 472),
            _ => unreachable!(),
        }
        assert_eq!(interactive.map(), 0xA);
        assert_eq!(interactive.events(), poised.events());
    }
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

#[derive(Debug, Clone, Copy)]
enum Action {
    Walk(Direction),
    Open(Direction),
}

impl Action {
    /// A script that holds the world is acknowledged first, as a player
    /// would: discovery is about geometry, and B's Elder speaks on arrival.
    fn apply(self, world: &mut World<'_>) -> Result<Step, WorldError> {
        for _ in 0..64 {
            if !world.in_scene() {
                break;
            }
            world.update(None, PRESS_A)?;
        }
        match self {
            Self::Walk(direction) => world.step_checked(Some(direction)),
            Self::Open(direction) => {
                world.face(direction);
                world.interact_checked()
            }
        }
    }
}

fn accepted(result: Result<Step, WorldError>) -> Result<Step, String> {
    match result {
        Ok(Step::Refused(reason)) => Err(format!("{reason:?}")),
        Ok(step) => Ok(step),
        Err(error) => Err(format!("build: {error}")),
    }
}

#[test]
fn route_actions_reject_refusals_and_build_errors() {
    assert!(
        accepted(Ok(Step::Refused(room_core::Unqualified::UnsupportedType(
            8
        ))))
        .is_err()
    );
    assert!(accepted(Err(
        crysta_runtime::RoomError::OutsideSlice { map: 0 }.into()
    ))
    .is_err());
    assert_eq!(accepted(Ok(Step::Stayed)), Ok(Step::Stayed));
}

struct Link {
    parent: Option<usize>,
    actions: Vec<(Action, Step)>,
}

struct Discovery<'a> {
    links: Vec<Link>,
    // First successful route and actual reached state for each map.
    arrivals: BTreeMap<u16, (usize, World<'a>)>,
    refused: BTreeMap<(u16, String), usize>,
    seen: HashSet<(u16, u16, u16)>,
    truncated: bool,
}

fn cell(world: &World<'_>) -> (u16, u16, u16) {
    let (x, y) = world.position();
    (world.map(), x / 16, y / 16)
}

impl Discovery<'_> {
    fn route(&self, mut index: usize) -> Vec<(Action, Step)> {
        let mut edges = Vec::new();
        while let Some(parent) = self.links[index].parent {
            edges.push(self.links[index].actions.as_slice());
            index = parent;
        }
        edges.into_iter().rev().flatten().copied().collect()
    }
}

/// Cell BFS is discovery, not proof of unreachability: timing, subcell position,
/// facing, actor state and walking history are deliberately absent from the key.
/// Every accepted edge retains its exact frame actions, with no refused prefix.
fn discover(origin: World<'_>, goal: Option<u16>) -> Discovery<'_> {
    discover_with_stride(origin, goal, 16)
}

fn discover_with_stride(origin: World<'_>, goal: Option<u16>, stride: u16) -> Discovery<'_> {
    let key = |world: &World<'_>| {
        let (x, y) = world.position();
        (world.map(), x / stride, y / stride)
    };
    let mut visited = HashSet::from([key(&origin)]);
    let budget = 20_000 * usize::from(16 / stride).pow(2);
    let mut discovery = Discovery {
        links: vec![Link {
            parent: None,
            actions: vec![],
        }],
        arrivals: BTreeMap::from([(origin.map(), (0, origin.clone()))]),
        refused: BTreeMap::new(),
        seen: HashSet::from([cell(&origin)]),
        truncated: false,
    };
    let mut queue = VecDeque::from([(0, origin)]);
    while let Some((parent, world)) = queue.pop_front() {
        if visited.len() > budget {
            discovery.truncated = true;
            break;
        }
        for direction in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            for action in [Action::Walk(direction), Action::Open(direction)] {
                let mut next = world.clone();
                let mut actions = Vec::new();
                let frames = if matches!(action, Action::Walk(_)) {
                    32
                } else {
                    1
                };
                for _ in 0..frames {
                    let step = match accepted(action.apply(&mut next)) {
                        Ok(step) => step,
                        Err(reason) => {
                            *discovery.refused.entry((world.map(), reason)).or_default() += 1;
                            break; // Discard this entire edge, not just the failing frame.
                        }
                    };
                    actions.push((action, step));
                    if key(&next) != key(&world) {
                        if visited.insert(key(&next)) {
                            discovery.seen.insert(cell(&next));
                            let index = discovery.links.len();
                            discovery.links.push(Link {
                                parent: Some(parent),
                                actions,
                            });
                            discovery
                                .arrivals
                                .entry(next.map())
                                .or_insert_with(|| (index, next.clone()));
                            if goal == Some(next.map()) {
                                return discovery;
                            }
                            queue.push_back((index, next));
                        }
                        break;
                    }
                }
            }
        }
    }
    discovery
}

fn replay<'a>(mut origin: World<'a>, actions: &[(Action, Step)]) -> World<'a> {
    let candidate = origin.room().passive_directional_type8_special_bit_clear();
    for (index, &(action, expected)) in actions.iter().enumerate() {
        let actual = accepted(action.apply(&mut origin))
            .unwrap_or_else(|error| panic!("action {index} {action:?}: {error}"));
        assert_eq!(actual, expected, "action {index} {action:?}");
        assert_eq!(
            origin.room().passive_directional_type8_special_bit_clear(),
            candidate,
            "collision mode changed at action {index} {action:?}"
        );
    }
    origin
}

/// Opt-in local artifacts only: directions are runtime frame inputs, while Open
/// is a host face+interaction operation, NOT a claimed native button mapping.
fn retain_route(name: &str, mut world: World<'_>, actions: &[(Action, Step)]) {
    use serde_json::json;
    let Some(directory) = std::env::var_os("CRYSTA_ROUTE_OUTPUT") else {
        return;
    };
    let state = |world: &World<'_>| {
        json!({
            "map": world.map(), "position": world.position(),
            "facing": format!("{:?}", world.facing()),
            "arrival": world.arrival().map(|arrival| json!({
                "elapsed": arrival.elapsed(), "phase": format!("{:?}", arrival.phase()),
            })),
        })
    };
    let start = state(&world);
    let events = world.events().to_vec();
    let mut trace = Vec::new();
    for (index, &(action, expected)) in actions.iter().enumerate() {
        let before = state(&world);
        let (kind, direction) = match action {
            Action::Walk(direction) => ("walk", direction),
            Action::Open(direction) => ("face_and_interact", direction),
        };
        world = replay(world, &[(action, expected)]);
        trace.push(json!({
            "index": index, "action": kind, "direction": format!("{direction:?}"),
            "before": before, "after": state(&world), "step": format!("{expected:?}"),
        }));
    }
    let artifact = json!({
        "schema": "crysta-candidate-route-v1",
        "contract": "passive directional; $097C & 4 == 0; discovery only, not native qualification",
        "start": start, "end": state(&world), "events": events,
        "actions": trace,
    });
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("{name}.json"));
    std::fs::write(&path, serde_json::to_vec_pretty(&artifact).unwrap()).unwrap();
}

fn assert_same_arrival(actual: &World<'_>, expected: &World<'_>) {
    assert_eq!(actual.map(), expected.map());
    assert_eq!(actual.position(), expected.position());
    assert_eq!(actual.facing(), expected.facing());
    assert_eq!(actual.arrival(), expected.arrival());
    assert_eq!(actual.events(), expected.events());
    assert_eq!(actual.residents(), expected.residents());
    assert_eq!(actual.room(), expected.room());
}

fn reachable_maps(image: &[u8], start: u16, x: u16, y: u16) -> BTreeSet<u16> {
    let origin = World::enter(image, start, x, y).expect("the start must build");
    let found = discover(origin.clone(), None);
    assert!(!found.truncated, "discovery exceeded its cell budget");
    for (index, arrival) in found.arrivals.values() {
        assert_same_arrival(&replay(origin.clone(), &found.route(*index)), arrival);
    }
    found.arrivals.keys().copied().collect()
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

const PRESS_A: Presses = Presses {
    confirm: true,
    cancel: false,
    up: false,
    down: false,
};
const PRESS_B: Presses = Presses {
    confirm: false,
    cancel: true,
    up: false,
    down: false,
};

fn flag(world: &World<'_>, flag: usize) -> bool {
    world.events()[flag / 8] & (1 << (flag % 8)) != 0
}

/// Runs neutral frames until the window shows something, then presses A
/// until it is gone and no script holds the world. Returns the presses.
fn read_through(world: &mut World<'_>) -> usize {
    let mut waited = 0;
    while world.dialogue().is_none() {
        world.update(None, Presses::default()).unwrap();
        waited += 1;
        assert!(waited < 400, "nothing was said");
    }
    let mut presses = 0;
    while world.dialogue().is_some() || world.in_scene() {
        assert!(
            world.dialogue().is_none_or(|view| view.cursor.is_none()),
            "a choice is open"
        );
        world.update(None, PRESS_A).unwrap();
        presses += 1;
        assert!(presses < 40, "the text never ended");
    }
    presses
}

/// Presses A on the faced resident, then acknowledges pages until a choice
/// opens. Returns the pages acknowledged.
fn talk_until_choice(world: &mut World<'_>) -> usize {
    // A frame for the resident's loop to face the player and take interaction.
    world.update(None, Presses::default()).unwrap();
    world.update(None, PRESS_A).unwrap();
    assert!(world.in_scene(), "the callback must hold the world");
    let mut pages = 0;
    while world.dialogue().and_then(|view| view.cursor).is_none() {
        assert!(
            world.dialogue().is_some(),
            "the scene ended without a choice"
        );
        world.update(None, PRESS_A).unwrap();
        pages += 1;
        assert!(pages < 40);
    }
    pages
}

#[test]
fn the_elder_grants_26_before_the_choice_and_either_answer_continues() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    for answer in [PRESS_A, PRESS_B] {
        let events = crysta_runtime::world::new_game_flags();
        let mut world = World::enter_with_events(image, 0x000B, 120, 112 + 16, events).unwrap();
        world.face(Direction::Up);
        // Entering, the Elder speaks unprompted and the world waits for it.
        assert!(read_through(&mut world) >= 1);
        assert!(!flag(&world, 0x26), "arrival text grants nothing");
        let pages = talk_until_choice(&mut world);
        assert!(pages >= 1);
        assert!(
            flag(&world, 0x26),
            "granted after the request, before the choice"
        );
        // Either answer: the callback returns and hands the follow-up to the
        // Elder's own script, which shows it while the world runs.
        world.update(None, answer).unwrap();
        assert!(!world.in_scene());
        // The follow-up is cooperative, with the pad's directions locked.
        while world.dialogue().is_none() {
            world.update(None, Presses::default()).unwrap();
        }
        let before = world.position();
        world
            .update(Some(Direction::Down), Presses::default())
            .unwrap();
        assert_eq!(world.position(), before, "COP 2A $FF50 holds the player");
        // Option 1 has three pages natively; the last A must not talk again.
        let pages = read_through(&mut world);
        if answer == PRESS_A {
            assert_eq!(pages, 3);
        }
        assert!(!world.in_scene() && world.dialogue().is_none());
    }
}

#[test]
fn the_weaver_grants_28_only_to_the_first_answer() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut events = crysta_runtime::world::new_game_flags();
    events[0x26 / 8] |= 1 << (0x26 % 8);
    let mut world = World::enter_with_events(image, 0x0013, 360, 144, events).unwrap();
    world.face(Direction::Up);
    let down = Presses {
        down: true,
        ..Presses::default()
    };
    let finish = |world: &mut World<'_>| {
        for _ in 0..40 {
            if !world.in_scene() {
                return;
            }
            world.update(None, PRESS_A).unwrap();
        }
        panic!("the conversation never ended");
    };
    // Refuse: the second option. Nothing granted; the talk ends.
    talk_until_choice(&mut world);
    world.update(None, down).unwrap();
    world.update(None, PRESS_A).unwrap();
    finish(&mut world);
    assert!(!flag(&world, 0x28));
    // Asked again, the first option grants it.
    talk_until_choice(&mut world);
    world.update(None, PRESS_A).unwrap();
    finish(&mut world);
    assert!(flag(&world, 0x28));
}

#[test]
fn candidate_entry_is_explicit_and_occupancy_retains_it() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let flags = crysta_runtime::world::new_game_flags();
    let mut candidate = World::enter_candidate(image, 0xD, 200, 700, flags).unwrap();
    assert!(candidate
        .room()
        .passive_directional_type8_special_bit_clear());
    assert!(!World::enter(image, 0xB, 120, 112)
        .unwrap()
        .room()
        .passive_directional_type8_special_bit_clear());
    let before = candidate.room().cells().to_vec();
    let mut rebuilt = false;
    for _ in 0..4000 {
        assert!(!matches!(
            candidate.step_checked(None).unwrap(),
            crysta_runtime::world::Step::Refused(_)
        ));
        assert!(candidate
            .room()
            .passive_directional_type8_special_bit_clear());
        if candidate.room().cells() != before {
            rebuilt = true;
            break;
        }
    }
    assert!(rebuilt, "exercise an actual actor occupancy rebuild");
}

#[test]
fn interaction_builds_residents_from_the_actual_events() {
    use assets::maps::scripts::EventFlags;
    use crysta_runtime::residents::residents;
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut events = crysta_runtime::world::new_game_flags();
    events[0x21 / 8] |= 1 << (0x21 % 8); // Despawn the opening resident.
    let mut exercised = 0;
    for source in MAPS {
        let exits = ExitList::from_rom(image, source).unwrap();
        for record in exits.records() {
            let Ok(destination) = record.direct_destination() else {
                continue;
            };
            if !MAPS.contains(&destination) {
                continue;
            }
            let actual = residents(image, destination, EventFlags::Bitmap(&events)).unwrap();
            let defaults = residents(
                image,
                destination,
                EventFlags::Bitmap(&crysta_runtime::world::new_game_flags()),
            )
            .unwrap();
            if actual == defaults {
                continue;
            }
            for candidate in [false, true] {
                let x = u16::from(record.x()) * 16 + 8;
                let y = u16::from(record.y()) * 16 + 16;
                let mut world = if candidate {
                    World::enter_candidate(image, source, x, y, events.clone())
                } else {
                    World::enter_with_events(image, source, x, y, events.clone())
                }
                .unwrap();
                assert_eq!(
                    world.interact_checked().unwrap(),
                    Step::Entered {
                        from: source,
                        to: destination
                    }
                );
                assert_eq!(world.events(), events);
                assert_eq!(world.residents(), actual);
                let (x, y) = record.destination_position();
                let expected = if candidate {
                    World::enter_candidate(image, destination, x, y, events.clone())
                } else {
                    World::enter_with_events(image, destination, x, y, events.clone())
                }
                .unwrap();
                assert_same_arrival(&world, &expected);
                exercised += 1;
            }
        }
    }
    assert!(
        exercised > 0,
        "must exercise flag-dependent destination rosters"
    );
}

fn report_missing_routes(image: &[u8], found: &Discovery<'_>) {
    for missing in MAPS.filter(|map| !found.arrivals.contains_key(map)) {
        eprintln!(
            "missing ${missing:04X}: no checked route discovered (not proof of unreachability)"
        );
        for source in MAPS {
            let exits = ExitList::from_rom(image, source).unwrap();
            for exit in exits
                .records()
                .iter()
                .filter(|exit| exit.direct_destination() == Ok(missing))
            {
                let (x, y) = (u16::from(exit.x()), u16::from(exit.y()));
                let nearest = found
                    .seen
                    .iter()
                    .filter(|(map, _, _)| *map == source)
                    .map(|(_, column, row)| column.abs_diff(x) + row.abs_diff(y))
                    .min();
                let built = crysta_runtime::room_candidate(image, source).unwrap();
                let kinds: Vec<_> = [(x, y), (x, y + 1)]
                    .into_iter()
                    .map(|(column, row)| {
                        built
                            .room
                            .cells()
                            .get(usize::from(row) * usize::from(built.width) + usize::from(column))
                            .map(|raw| (raw >> 9) & 0x1F)
                    })
                    .collect();
                eprintln!("  incoming ${source:04X} exit ({x},{y}) {}x{}; source reached={}; nearest explored cell distance={nearest:?}; trigger/below types={kinds:?}",
                    exit.width(), exit.height(), found.arrivals.contains_key(&source));
            }
        }
    }
}

fn report_return_gap(map: u16, reached: &World<'_>, back: &Discovery<'_>) {
    for (&destination, (index, state)) in &back.arrivals {
        if destination == map {
            continue;
        }
        let prefix = back.route(*index);
        retain_route(
            &format!("{map:04X}-return-prefix-to-{destination:04X}"),
            reached.clone(),
            &prefix,
        );
        eprintln!(
            "    return prefix enters ${destination:04X} at {:?}",
            state.position()
        );
    }
}

#[test]
fn candidate_routes_replay_from_the_opening_house_and_return_from_reached_states() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let origin = World::enter_candidate(
        image,
        0xB,
        120,
        112,
        crysta_runtime::world::new_game_flags(),
    )
    .unwrap();
    let found = discover(origin.clone(), None);
    assert!(!found.truncated);
    eprintln!(
        "candidate reached {}/24: {:04X?}",
        found.arrivals.len(),
        found.arrivals.keys().collect::<Vec<_>>()
    );
    let mut totals = BTreeMap::new();
    for ((map, reason), count) in &found.refused {
        *totals.entry(reason).or_insert(0usize) += count;
        eprintln!("  ${map:04X} {reason}: {count} discarded edges");
    }
    eprintln!("candidate refusal totals: {totals:?}");
    report_missing_routes(image, &found);
    assert!(
        found.arrivals.contains_key(&0xA),
        "must leave opening house and reach town"
    );
    let mut returns = BTreeSet::new();
    let mut transition_actions = [0usize; 2];
    for (&map, (index, arrival)) in &found.arrivals {
        let outward = found.route(*index);
        for (action, step) in &outward {
            if matches!(step, Step::Entered { .. }) {
                transition_actions[usize::from(matches!(action, Action::Open(_)))] += 1;
            }
        }
        let reached = replay(origin.clone(), &outward);
        assert_same_arrival(&reached, arrival);
        retain_route(&format!("opening-to-{map:04X}"), origin.clone(), &outward);
        if map == origin.map() {
            continue;
        }
        // Half-cell keys retain doorway approaches lost by full-cell merging.
        // No arbitrary arrival construction: start at the actual outbound endpoint.
        let back = discover_with_stride(reached.clone(), Some(origin.map()), 8);
        if let Some((index, home)) = back.arrivals.get(&origin.map()) {
            let return_actions = back.route(*index);
            retain_route(
                &format!("{map:04X}-to-opening"),
                reached.clone(),
                &return_actions,
            );
            let returned = replay(reached, &return_actions);
            assert_same_arrival(&returned, home);
            returns.insert(map);
            eprintln!(
                "  ${map:04X}: {} outbound actions, {} return actions",
                outward.len(),
                return_actions.len()
            );
        } else {
            eprintln!(
                "  ${map:04X}: no return route discovered; truncated={}, refusals={:?}",
                back.truncated, back.refused
            );
            report_return_gap(map, &reached, &back);
        }
    }
    eprintln!(
        "checked roundtrips from {}/{} non-opening reached maps: {returns:04X?}",
        returns.len(),
        found.arrivals.len() - 1
    );
    assert!(
        transition_actions.iter().all(|count| *count > 0),
        "exercise walking and interaction transitions"
    );
    assert_eq!(
        found.arrivals.len(),
        24,
        "every map has a checked outbound route"
    );
    let missing_returns: Vec<_> = found
        .arrivals
        .keys()
        .copied()
        .filter(|map| *map != origin.map() && !returns.contains(map))
        .collect();
    assert_eq!(
        missing_returns,
        Vec::<u16>::new(),
        "every non-opening map must have a checked return route"
    );
}

#[test]
fn checked_interaction_reports_destination_exit_decode_failure() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let source = 0xB;
    let exits = ExitList::from_rom(cartridge.image(), source).unwrap();
    let record = exits
        .records()
        .iter()
        .find(|record| {
            record
                .direct_destination()
                .is_ok_and(|map| MAPS.contains(&map) && map != source)
        })
        .unwrap();
    let destination = record.direct_destination().unwrap();
    let mut damaged = cartridge.image().to_vec();
    let pointer = 0x18000 + usize::from(destination) * 2;
    damaged[pointer..pointer + 2].copy_from_slice(&1u16.to_le_bytes());
    let mut world = World::enter_candidate(
        &damaged,
        source,
        u16::from(record.x()) * 16 + 8,
        u16::from(record.y()) * 16 + 16,
        crysta_runtime::world::new_game_flags(),
    )
    .unwrap();
    let before = world.clone();
    assert!(
        matches!(world.interact_checked(), Err(WorldError::Exits { map, .. }) if map == destination)
    );
    assert_same_arrival(&world, &before);
}

#[test]
fn resident_failure_is_not_a_successfully_empty_roster() {
    use assets::maps::actors::{ActorError, ResolveError};
    use assets::maps::scripts::EventFlags;
    use crysta_runtime::residents::residents;

    let Some(cartridge) = owned_rom() else {
        return;
    };
    let map = 0xB;
    let pointer = 0x38000 + usize::from(map) * 2;
    let mut image = cartridge.image().to_vec();
    image[pointer..pointer + 2].fill(0);
    let events = crysta_runtime::world::new_game_flags();
    let expected = ResolveError::Decode(ActorError::Absent { index: map });
    assert_eq!(
        residents(&image, map, EventFlags::Bitmap(&events)),
        Err(expected.clone())
    );
    for result in [
        World::enter(&image, map, 120, 112),
        World::enter_with_events(&image, map, 120, 112, events.clone()),
        World::enter_candidate(&image, map, 120, 112, events.clone()),
    ] {
        let error = result
            .err()
            .expect("refused roster must not create an empty world");
        assert_eq!(
            error.to_string(),
            format!("map {map:#06x} residents: {expected}")
        );
    }

    // A valid terminator is genuinely empty, not a failed decode.
    let mut image = cartridge.image().to_vec();
    let entry = 0x30000 | usize::from(u16::from_le_bytes([image[pointer], image[pointer + 1]]));
    image[entry + 2..entry + 4].copy_from_slice(&[0xFF, 0]);
    assert_eq!(
        residents(&image, map, EventFlags::Bitmap(&events)),
        Ok(vec![])
    );
    for result in [
        World::enter(&image, map, 120, 112),
        World::enter_with_events(&image, map, 120, 112, events.clone()),
        World::enter_candidate(&image, map, 120, 112, events),
    ] {
        let world = result.unwrap();
        assert!(world.residents().is_empty());
        assert_eq!(world.map(), map);
    }
}
