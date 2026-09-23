//! The Crysta story slice, played through the world with real presses and
//! compared against the native input-only route.
use crysta_runtime::scene::Presses;
use crysta_runtime::world::{fresh_game_flags, World};
use rom::{Revision, Rom};
use room_core::Direction;
use std::path::Path;

fn owned_rom() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    let rom = Rom::load(&std::fs::read(path).ok()?).ok()?;
    (rom.revision() == Revision::Japan).then_some(rom)
}

const A: Presses = Presses {
    confirm: true,
    cancel: false,
    up: false,
    down: false,
};

fn flag(world: &World<'_>, flag: usize) -> bool {
    world.events()[flag / 8] & (1 << (flag % 8)) != 0
}

/// Frames until `done`, each one neutral.
fn frames_until(world: &mut World<'_>, limit: u32, done: impl Fn(&World<'_>) -> bool) -> u32 {
    for frame in 0..limit {
        if done(world) {
            return frame;
        }
        world.update(None, Presses::default()).unwrap();
    }
    panic!("not within {limit} frames");
}

const ELLE: usize = 0x03_8D36;
/// The blue door in C.
const DOOR: usize = 0x03_8C32;

#[test]
fn elle_wakes_ark_then_walks_out() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut world = World::enter_with_events(image, 0x000F, 304, 112, fresh_game_flags()).unwrap();
    let elle = |world: &World<'_>| world.residents().iter().find(|r| r.record == ELLE).cloned();
    let at_start = elle(&world).expect("Elle is in the bedroom before the intro");
    assert!(at_start.body, "and drawn");
    assert_eq!(at_start.position, (328, 112));

    // She locks the pad at once, waits 120 frames, then speaks.
    world
        .update(Some(Direction::Down), Presses::default())
        .unwrap();
    assert_eq!(
        world.position(),
        (304, 112),
        "COP 2A $FF50 from the first frame"
    );
    let silent = frames_until(&mut world, 400, |world| world.dialogue().is_some());
    assert!((115..=125).contains(&silent), "spoke after {silent} frames");
    assert!(world.in_scene(), "COP 1F holds the world");

    // Five pages over three requests, each acknowledged with A.
    let mut pages = 0;
    while world.in_scene() {
        world.update(None, A).unwrap();
        pages += 1;
        assert!(pages < 20);
    }
    assert_eq!(pages, 5);
    assert!(flag(&world, 0x20), "set right after the last page");

    // She walks out while the pad stays locked for ten 32-frame steps, then
    // unlocks it; natively 5903 -> 6223.
    assert!(world.pad_locked());
    let locked = frames_until(&mut world, 600, |world| !world.pad_locked());
    assert!(
        (318..=322).contains(&locked),
        "unlocked after {locked} frames"
    );
    // Right, as the native control probe did: the bed is below. Walking
    // starts a few frames after the press.
    let (mut probe, before) = (world.clone(), world.position());
    for _ in 0..12 {
        probe
            .update(Some(Direction::Right), Presses::default())
            .unwrap();
    }
    assert_ne!(probe.position(), before, "Ark walks once unlocked");
    // Two more down steps to (392,256), then she deletes herself.
    let gone = frames_until(&mut world, 200, |world| elle(world).is_none());
    assert!(
        (94..=98).contains(&gone),
        "deleted {gone} frames after the unlock"
    );
    assert_eq!(world.items(), [0x7A, 0xA0], "she gave Ark two items");
}

#[test]
fn the_box_rooms_figure_stays_hidden_until_its_flag() {
    // `$83:9271` in `$21` hides itself (entity +$04 bit 15) and waits for
    // flag `$03`, as natively on the Pandora route: not drawn, not blocking.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let events = crysta_runtime::world::new_game_flags();
    let mut world = World::enter_with_events(image, 0x0021, 136, 368, events).unwrap();
    world.update(None, Presses::default()).unwrap();
    let figure = world
        .residents()
        .iter()
        .find(|r| r.record == 0x03_9271)
        .expect("present while $101 is clear");
    assert!(figure.body && figure.hidden);
    let (column, row) = figure.collision_cell();
    let cell = world.room().cells()
        [usize::from(row) * usize::from(world.dimensions().0) + usize::from(column)];
    // A marked cell is written solid (attribute 14); this one stays floor.
    assert_ne!(cell >> 9, 14, "a hidden body does not occupy its cell");
}

#[test]
fn d_gate_holds_the_house_door_until_the_elder_has_spoken() {
    // Facing the exit from (120,704): refused while the gate's COP 3B stamp
    // stands, open once `$26` deletes the gate on entry.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut after = crysta_runtime::world::new_game_flags();
    after[0x26 / 8] |= 1 << (0x26 % 8);
    for (flags, opens) in [
        (crysta_runtime::world::new_game_flags(), false),
        (after, true),
    ] {
        let mut world = World::enter_with_events(image, 0x000D, 120, 704, flags).unwrap();
        world.update(None, Presses::default()).unwrap();
        world.face(Direction::Down);
        let step = world.interact_checked().unwrap();
        assert_eq!(world.map() == 0x000A, opens, "{step:?}");
    }
}

#[test]
fn the_friends_ask_for_help_at_the_blue_door() {
    // After the weaver (`$28`), entering C: the resident `$83:8C1E` moves to
    // (184,416), sets `$27`, holds the pad, and after about 41 frames asks
    // over three requests and six pages, walking left 64 pixels in 64 frames
    // between the second and third; natively 16749 -> 17987.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut events = crysta_runtime::world::new_game_flags();
    for set in [0x26, 0x28] {
        events[set / 8] |= 1 << (set % 8);
    }
    let mut world = World::enter_with_events(image, 0x000C, 136, 368, events).unwrap();
    let friend = |world: &World<'_>| {
        world
            .residents()
            .iter()
            .find(|r| r.record == 0x03_8C1E)
            .unwrap()
            .position
    };
    world.update(None, Presses::default()).unwrap();
    assert_eq!(friend(&world), (184, 416));
    assert!(flag(&world, 0x27) && world.pad_locked());
    let silent = frames_until(&mut world, 200, |world| world.dialogue().is_some());
    assert!((38..=44).contains(&silent), "asked after {silent} frames");
    let (mut pages, mut gaps) = (0, Vec::new());
    while world.dialogue().and_then(|view| view.cursor).is_none() {
        world.update(None, A).unwrap();
        pages += 1;
        if world.dialogue().is_none() {
            gaps.push(frames_until(&mut world, 200, |world| {
                world.dialogue().is_some()
            }));
        }
        assert!(pages < 20);
    }
    // One of the pauses between requests is the walk: 64 pixels, one a frame.
    assert!(
        gaps.iter().any(|gap| (63..=66).contains(gap)),
        "gaps {gaps:?}"
    );
    assert_eq!(friend(&world).0, 120);
    assert_eq!(pages, 6);
    // Option 1, help: two more pages, then `$2E`; the friend walks back and
    // the pad unlocks.
    world.update(None, A).unwrap();
    let mut follow = 0;
    while world.in_scene() || world.dialogue().is_some() {
        world.update(None, A).unwrap();
        follow += 1;
        assert!(follow < 20);
    }
    assert_eq!(follow, 2);
    assert!(flag(&world, 0x2E));
    frames_until(&mut world, 200, |world| !world.pad_locked());
    assert_eq!(friend(&world).0, 184, "back at the door");
}

#[test]
fn a_map_load_clears_local_flags_and_counters_and_keeps_items() {
    // `$8D:8AED` clears the locals (flags 0..31) and `$0640` on every load.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut world = World::enter_with_events(image, 0x000F, 304, 112, fresh_game_flags()).unwrap();
    // Through the wake-up, so Elle's items are held.
    frames_until(&mut world, 400, |world| world.dialogue().is_some());
    while world.in_scene() {
        world.update(None, A).unwrap();
    }
    frames_until(&mut world, 600, |world| !world.pad_locked());
    let mut local = world.clone();
    local.set_flag(0x01);
    assert!(flag(&local, 0x01));
    // Walk out of the bedroom: right to the door column (exit tile (24,12)),
    // then down through it.
    while local.position().0 < 396 {
        local
            .update(Some(Direction::Right), Presses::default())
            .unwrap();
    }
    let map = local.map();
    let entered = (0..600).find(|_| {
        local
            .update(Some(Direction::Down), Presses::default())
            .unwrap();
        local.map() != map
    });
    assert!(entered.is_some(), "left the bedroom");
    assert!(!flag(&local, 0x01), "local flag cleared");
    assert!(
        flag(&local, 0x20) && flag(&local, 0xFB),
        "global flags kept"
    );
    assert_eq!(local.items(), [0x7A, 0xA0], "items kept");
}

#[test]
// `World::pad_locked` as a path is not general over the world's lifetime.
#[allow(clippy::redundant_closure_for_method_calls)]
fn two_hits_break_the_blue_door_and_open_the_stairs() {
    // The door `$83:8C32` counts hits in `$0640`: the second patches the
    // stair cells (11,21)/(11,20), sets `$292` at `$88:ABEE`, and the friends'
    // reaction runs through locals 4..9 before control returns.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut events = crysta_runtime::world::new_game_flags();
    for set in [0x26, 0x27, 0x28, 0x2E] {
        events[set / 8] |= 1 << (set % 8);
    }
    let mut world = World::enter_with_events(image, 0x000C, 184, 400, events).unwrap();
    let read_out = |world: &mut World<'_>| {
        for _ in 0..3000 {
            let reading = world.dialogue().is_some() || world.in_scene();
            world
                .update(None, if reading { A } else { Presses::default() })
                .unwrap();
            if !reading && !world.pad_locked() && world.dialogue().is_none() {
                return;
            }
        }
        panic!("the reaction never ended");
    };
    // Its script registers the hit (`COP 65`) after its opening pose.
    for _ in 0..60 {
        world.update(None, Presses::default()).unwrap();
    }
    assert_eq!(
        world
            .frozen_scripts()
            .iter()
            .filter(|(r, _)| *r == DOOR)
            .count(),
        0
    );
    assert!(world.strike(DOOR), "the door takes hits");
    assert!(!world.strike(DOOR), "not again within sixteen frames");
    frames_until(&mut world, 10, |world| world.pad_locked());
    read_out(&mut world);
    assert!(!flag(&world, 0x292));
    assert!(world.strike(DOOR));
    frames_until(&mut world, 10, |world| flag(world, 0x292));
    read_out(&mut world);
    assert!(
        world.residents().iter().all(|r| r.record != DOOR),
        "the door is gone"
    );
    let cell = |column: usize, row: usize| {
        world.room().cells()[row * usize::from(world.dimensions().0) + column]
    };
    assert_eq!(cell(11, 21) & 0x1FF, 0xCB);
    assert_eq!(cell(11, 20) & 0x1FF, 0xF6);
    assert_ne!(cell(11, 21) >> 9, 14, "and not stamped any more");
    assert!(flag(&world, 0x09), "the reaction reached its last local");
    // B shares C's first layer, so the game does not reload it: the opened
    // door stays open there (`$86:9145`).
    world.place(136, 300);
    world.update(None, Presses::default()).unwrap();
    assert_eq!(world.map(), 0x000B);
    assert!(world.patched_cells().contains(&(11, 21, 0xCB)));
    assert!(world.patched_cells().contains(&(11, 20, 0xF6)));
}

/// Native pad runs from the Pandora journey's pot segments
/// (`docs/pandora-pots.md`): 0 Down, 1 Up, 2 Left, 3 Right, 4 A, 5 neutral.
const MISS: &[(u8, u16)] = &[
    (4, 1),
    (5, 120),
    (0, 22),
    (5, 60),
    (3, 55),
    (5, 60),
    (1, 1),
    (5, 40),
    (4, 1),
    (5, 180),
];
const FA_HIT: &[(u8, u16)] = &[
    (4, 1),
    (5, 120),
    (0, 45),
    (5, 60),
    (0, 22),
    (5, 60),
    (3, 66),
    (5, 60),
    (1, 40),
    (5, 60),
    (3, 33),
    (5, 60),
    (1, 20),
    (5, 80),
    (4, 1),
    (5, 180),
];
const FB_HIT: &[(u8, u16)] = &[
    (4, 1),
    (5, 120),
    (3, 33),
    (5, 60),
    (0, 28),
    (5, 60),
    (3, 33),
    (5, 60),
    (1, 20),
    (5, 80),
    (4, 1),
    (5, 240),
];

/// Plays pad runs through the world, as the host would.
fn replay(world: &mut World<'_>, runs: &[(u8, u16)]) {
    for &(pad, frames) in runs {
        let direction = match pad {
            0 => Some(Direction::Down),
            1 => Some(Direction::Up),
            2 => Some(Direction::Left),
            3 => Some(Direction::Right),
            _ => None,
        };
        let presses = if pad == 4 { A } else { Presses::default() };
        for _ in 0..frames {
            world.update(direction, presses).unwrap();
        }
    }
}

#[test]
fn pots_lifted_and_thrown_with_the_native_presses_break_the_blue_door() {
    // The three native segments, each from its lift pose: FA from (104,352)
    // thrown from (136,368) misses; FA from (40,352) and FB from (88,352)
    // thrown from (184,368) hit, and the second hit opens the stairs.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut events = crysta_runtime::world::new_game_flags();
    for set in [0x26, 0x27, 0x28, 0x2E] {
        events[set / 8] |= 1 << (set % 8);
    }
    let mut world = World::enter_with_events(image, 0x000C, 184, 400, events).unwrap();
    for _ in 0..60 {
        world.update(None, Presses::default()).unwrap();
    }
    let read_out = |world: &mut World<'_>| {
        for _ in 0..3000 {
            let reading = world.dialogue().is_some() || world.in_scene();
            world
                .update(None, if reading { A } else { Presses::default() })
                .unwrap();
            if !reading && !world.pad_locked() && world.dialogue().is_none() {
                return;
            }
        }
        panic!("the reaction never ended");
    };
    let segment = |world: &mut World<'_>, (x, y, facing), runs| {
        world.place(x, y);
        world.face(facing);
        replay(world, runs);
    };

    segment(&mut world, (104, 352, Direction::Left), MISS);
    assert_eq!(world.position(), (136, 368));
    assert!(world.patched_cells().contains(&(5, 21, 0xF8)), "lifted");
    assert!(world.pot().is_none(), "broken");
    assert!(!world.pad_locked() && world.dialogue().is_none(), "no hit");

    segment(&mut world, (40, 352, Direction::Right), FA_HIT);
    // Holding Up against the door runs its push test (`COP 2F`) unfrozen.
    assert!(world
        .frozen_scripts()
        .iter()
        .all(|(record, _)| *record != DOOR));
    assert!(world.patched_cells().contains(&(3, 21, 0xF8)));
    assert!(
        world.patched_cells().contains(&(11, 21, 0x181)),
        "the first hit"
    );
    read_out(&mut world);
    assert!(!flag(&world, 0x292));

    segment(&mut world, (88, 352, Direction::Left), FB_HIT);
    assert!(flag(&world, 0x292), "the second hit");
    read_out(&mut world);
    assert!(world.patched_cells().contains(&(11, 21, 0xCB)));
}

#[test]
fn a_carried_pot_is_shown_in_hand_then_in_flight() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut events = crysta_runtime::world::new_game_flags();
    for set in [0x26, 0x27, 0x28, 0x2E] {
        events[set / 8] |= 1 << (set % 8);
    }
    let mut world = World::enter_with_events(image, 0x000C, 184, 400, events).unwrap();
    world.update(None, Presses::default()).unwrap();
    world.place(88, 352);
    world.face(Direction::Left);
    assert!(world.pot().is_none());
    replay(&mut world, &FB_HIT[..2]);
    assert_eq!(
        world.pot().map(|pot| (pot.tile, pot.flight)),
        Some((0xFB, None)),
        "held"
    );
    // The last walk and the throw, then 19 frames to the first flight sample.
    replay(&mut world, &FB_HIT[2..10]);
    assert_eq!(world.position(), (184, 368));
    replay(&mut world, &[(4, 1), (5, 19)]);
    assert_eq!(world.pot().and_then(|pot| pot.flight), Some((184, 357)));
}

/// The story flags after the blue door (`$292`).
fn after_the_door() -> Vec<u8> {
    let mut events = crysta_runtime::world::new_game_flags();
    for set in [0x26, 0x27, 0x28, 0x2E, 0x292] {
        events[set / 8] |= 1 << (set % 8);
    }
    events
}

/// Acknowledges pages until the pad unlocks and nothing is on the window.
fn read_out(world: &mut World<'_>) -> u32 {
    let mut pages = 0;
    for _ in 0..3000 {
        let reading = world.dialogue().is_some() || world.in_scene();
        pages += u32::from(reading);
        world
            .update(None, if reading { A } else { Presses::default() })
            .unwrap();
        if !reading && !world.pad_locked() && world.dialogue().is_none() {
            return pages;
        }
    }
    panic!("never released");
}

#[test]
// `World::pad_locked` as a path is not general over the world's lifetime.
#[allow(clippy::redundant_closure_for_method_calls)]
fn the_opened_stairs_lead_through_e_and_20_to_the_box_room() {
    // Selector-14 stairs settle at the raw anchor plus (8,16), as natively:
    // E (152,880), $20 (408,880), $21 (136,128).
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut events = crysta_runtime::world::new_game_flags();
    for set in [0x26, 0x27, 0x28, 0x2E] {
        events[set / 8] |= 1 << (set % 8);
    }
    let mut world = World::enter_with_events(image, 0x000C, 184, 400, events).unwrap();
    for _ in 0..60 {
        world.update(None, Presses::default()).unwrap();
    }
    for _ in 0..2 {
        assert!(world.strike(DOOR));
        frames_until(&mut world, 10, |world| world.pad_locked());
        read_out(&mut world);
    }
    let mut landings = Vec::new();
    for (column, map) in [(None, 0x000E), (Some(104), 0x0020), (Some(360), 0x0021)] {
        if let Some(column) = column {
            while world.position().0 > column {
                world
                    .update(Some(Direction::Left), Presses::default())
                    .unwrap();
            }
        }
        for _ in 0..200 {
            world
                .update(Some(Direction::Up), Presses::default())
                .unwrap();
            if world.map() == map {
                break;
            }
        }
        landings.push((world.map(), world.position()));
    }
    assert_eq!(
        landings,
        [
            (0x000E, (152, 880)),
            (0x0020, (408, 880)),
            (0x0021, (136, 128))
        ]
    );
}

#[test]
fn the_box_warns_on_contact_then_opens_for_the_next_approach() {
    // `$83:928F` with the native route's presses from `21-toward-box`
    // (`tools/pandora-qualification/route.jsonl`): Down into the box runs its
    // contact callback a frame later (local 1) and recoils Ark north, as
    // natively 26805 -> 26832; the warning sets local 2; the next approach
    // inside the gate sets `$22` and `COP 14` reloads `$21` at (136,368).
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut world = World::enter_with_events(image, 0x0021, 136, 128, after_the_door()).unwrap();
    assert_eq!(read_out(&mut world), 1, "the voice asks for help");
    replay(&mut world, &[(0, 150), (5, 180)]);
    assert_eq!(world.position(), (136, 351), "21-near-box");
    let mut trail = Vec::new();
    for _ in 0..25 {
        world
            .update(Some(Direction::Down), Presses::default())
            .unwrap();
        trail.push((world.position().1, flag(&world, 0x01)));
    }
    let contact = trail.iter().position(|&(_, local)| local).expect("contact");
    assert_eq!(
        trail[contact - 1..=contact],
        [(370, false), (369, true)],
        "contact at 370, callback and recoil the next frame"
    );
    // Nine more single pixels, sixteen frames at rest, the last pixel, while
    // Down stays held (native 26806 -> 26832).
    let recoil: Vec<u16> = trail[contact..].iter().map(|&(y, _)| y).collect();
    let mut expected: Vec<u16> = (360..=369).rev().collect();
    expected.resize(recoil.len(), 360);
    assert_eq!(recoil, expected);
    replay(&mut world, &[(5, 180)]);
    assert_eq!(world.position(), (136, 359), "21-box-contact-rest");
    // Down while the warning is pending does not move Ark.
    replay(&mut world, &[(0, 120)]);
    assert_eq!(world.position(), (136, 359));
    assert!(world.dialogue().is_some(), "the warning");
    replay(
        &mut world,
        &[(4, 1), (5, 180), (4, 1), (5, 180), (4, 1), (5, 360)],
    );
    assert!(flag(&world, 0x02) && !flag(&world, 0x22));
    assert_eq!(world.position(), (136, 359), "outside the gate");
    let mut opened = None;
    for frame in 0..315 {
        let pad = if frame < 15 {
            Some(Direction::Down)
        } else {
            None
        };
        world.update(pad, Presses::default()).unwrap();
        if opened.is_none() && flag(&world, 0x22) {
            opened = Some(world.position());
        }
    }
    assert_eq!(opened, Some((136, 368)), "opened inside the gate");
    assert_eq!((world.map(), world.position()), (0x0021, (136, 368)));
    assert!(!flag(&world, 0x01), "reloaded: locals cleared");
    assert!(world.pad_locked(), "the mask survives the reload");
}

#[test]
fn a_fresh_load_of_c_after_the_door_opens_the_stairs() {
    // `$8D:8FB4` applies `$96:CD9D`'s `$292` entries on every load of C, as
    // natively on the return from the box (`return-C-left`: `$1CF6`/`$3ACB`).
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut world = World::enter_with_events(image, 0x000C, 184, 400, after_the_door()).unwrap();
    assert!(world.patched_cells().contains(&(11, 21, 0xCB)));
    assert!(world.patched_cells().contains(&(11, 20, 0xF6)));
    for _ in 0..200 {
        world
            .update(Some(Direction::Up), Presses::default())
            .unwrap();
        if world.map() == 0x000E {
            break;
        }
    }
    assert_eq!((world.map(), world.position()), (0x000E, (152, 880)));
}

#[test]
fn the_opened_box_takes_ark_inside_to_map_41() {
    // `$88:AE64` in the reloaded `$21`: four requests with local `$0A` cues,
    // then `COP 14` to `$41` (mode 4, selector 2, raw (128,192)); native
    // settled (136,208).
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut events = after_the_door();
    events[0x22 / 8] |= 1 << (0x22 % 8);
    let mut world = World::enter_with_events(image, 0x0021, 136, 368, events).unwrap();
    let mut reading_frames = 0;
    for _ in 0..2000 {
        let reading = world.dialogue().is_some() || world.in_scene();
        reading_frames += u32::from(reading);
        world
            .update(None, if reading { A } else { Presses::default() })
            .unwrap();
        if world.map() == 0x0041 {
            break;
        }
    }
    assert_eq!((world.map(), world.position()), (0x0041, (136, 208)));
    assert_eq!(reading_frames, 14, "one A per page over the four requests");
}

#[test]
fn the_guide_tours_44_42_43_and_back_to_41_then_frees_ark() {
    // `$89:D3B1` and the guide `$89:D2AD` take turns through `$04BC`; the
    // controller transfers 41 -> 44 -> 42 -> 43 -> 41, sets `$243` on the
    // last, then `$244` after its last request, and unlocks the pad.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut events = after_the_door();
    events[0x22 / 8] |= 1 << (0x22 % 8);
    let mut world = World::enter_with_events(image, 0x0041, 136, 208, events).unwrap();
    let mut landings = Vec::new();
    for _ in 0..3000 {
        let reading = world.dialogue().is_some() || world.in_scene();
        let map = world.map();
        world
            .update(None, if reading { A } else { Presses::default() })
            .unwrap();
        if world.map() != map {
            landings.push((world.map(), world.position()));
        }
        if flag(&world, 0x244) && !world.pad_locked() && world.dialogue().is_none() {
            break;
        }
    }
    assert_eq!(
        landings,
        [
            (0x0044, (392, 464)),
            (0x0042, (136, 464)),
            (0x0043, (392, 208)),
            (0x0041, (136, 208))
        ]
    );
    assert!(flag(&world, 0x243) && flag(&world, 0x244));
    assert!(
        world
            .frozen_scripts()
            .iter()
            .all(|&(_, at)| at == 0x09_D253),
        "only the graphics controller, whose loads the background compiles: {:x?}",
        world.frozen_scripts()
    );
    // Ark walks again: native `pandora-left-rest`, Left 12 then neutral.
    replay(&mut world, &[(2, 12), (5, 120)]);
    assert_eq!(world.position(), (120, 208));
}
