//! Lifts and throws against native traces (`docs/pots.md`): each fixture in
//! `local/pots/{jp,eu}/` replays its prefix from the cellar's ready state
//! (Ark at (104,352) facing left, after agreeing to help), then its recorded
//! inputs, comparing every recorded frame.
use crysta_runtime::audio::Cue;
use crysta_runtime::scene::Presses;
use crysta_runtime::world::{new_game_flags, World};
use rom::Rom;
use room_core::Direction;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Not modelled yet: the dash-throw and jump-throw (measured once), and
/// the fall the pot shows as an exit drops it (`$84:C5CB`, stream `$6A`);
/// the drop itself is [`carrying_onto_an_exit_drops_the_pot`].
const LATER: [&str; 3] = [
    "exit-while-carrying",
    "throw-right-dashing",
    "throw-left-jumping",
];

/// The pot's break sound.
const BREAK: u8 = 0x13;

fn local() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local")
}

fn direction(buttons: &[&str]) -> Option<Direction> {
    buttons.iter().find_map(|&button| match button {
        "Up" => Some(Direction::Up),
        "Down" => Some(Direction::Down),
        "Left" => Some(Direction::Left),
        "Right" => Some(Direction::Right),
        _ => None,
    })
}

/// One frame with `buttons` held; A acts on its press.
fn frame(world: &mut World<'_>, buttons: &[&str], held_a: &mut bool) -> Vec<u8> {
    let a = buttons.contains(&"A");
    let presses = Presses {
        confirm: a && !*held_a,
        ..Presses::NONE
    };
    *held_a = a;
    world.update(direction(buttons), presses).unwrap();
    world
        .take_cues()
        .into_iter()
        .filter_map(|cue| match cue {
            Cue::Sound(latch) => Some((latch >> 8) as u8).filter(|&port3| port3 != 0),
            Cue::Track { .. } => None,
        })
        .collect()
}

/// The fixture's view of the pot: its phase and, in flight, x, y, height.
fn pot(world: &World<'_>) -> (String, Option<(u16, u16, i16)>) {
    use assets::sprites::PandoraCarryMotion as Motion;
    let flight = world.pot().and_then(|pot| pot.flight);
    let carry = world.carry();
    let phase = match (carry.map(|carry| (carry.motion, carry.tick)), flight) {
        (_, Some(_)) => "flight",
        // Broken, Ark still recovering: the pot's slot is parked.
        _ if world.pot().is_none() => "parked",
        // The pot's script holds a frame before Ark's does.
        (Some((Motion::Lifting, 22)), None) => "held",
        (Some((Motion::Lifting, _)), None) => "lift",
        (Some((Motion::Throwing, _)), None) => "windup",
        (Some(_) | None, None) => "held",
    };
    (phase.into(), flight)
}

fn replay(image: &[u8], name: &str, recipe: &Value, csv: &str) -> Result<(), String> {
    let mut events = new_game_flags();
    for set in [0x26, 0x27, 0x28, 0x2E] {
        events[set / 8] |= 1 << (set % 8);
    }
    let mut world = World::enter_with_events(image, 0x000C, 104, 352, events).unwrap();
    world.face(Direction::Left);
    let mut held_a = false;
    for step in recipe["prefix"].as_array().unwrap() {
        let buttons: Vec<&str> = step["buttons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b.as_str().unwrap())
            .collect();
        for _ in 0..step["frames"].as_u64().unwrap() {
            frame(&mut world, &buttons, &mut held_a);
        }
    }
    let mut lines = csv.lines();
    let header: Vec<&str> = lines.next().unwrap().split(',').collect();
    let column = |name: &str| header.iter().position(|&h| h == name).unwrap();
    let mut early_break = false;
    let mut lifted = !recipe["prefix"].as_array().unwrap().is_empty();
    for line in lines {
        let row: Vec<&str> = line.split(',').collect();
        let buttons: Vec<&str> = row[column("input")]
            .split('+')
            .filter(|b| !b.is_empty())
            .collect();
        let sounds = frame(&mut world, &buttons, &mut held_a);
        let number = |name: &str| row[column(name)].parse::<i32>().unwrap();
        let i = row[column("i")];
        let (x, y) = world.position();
        let facing = world.facing() as i32;
        let expected = (number("px"), number("py"), number("facing"));
        // Before the first lift, ordinary walking turns a frame before the
        // native game does; that is the world's walking, not the pot's.
        lifted |= row[column("phase")] != "parked";
        let facing = if lifted { facing } else { expected.2 };
        if (i32::from(x), i32::from(y), facing) != expected {
            return Err(format!(
                "{name} row {i}: player {:?}, native {expected:?}",
                (x, y, facing)
            ));
        }
        let (phase, flight) = pot(&world);
        if phase != row[column("phase")] {
            return Err(format!(
                "{name} row {i}: phase {phase}, native {}",
                row[column("phase")]
            ));
        }
        if phase == "flight" {
            let expected = (number("potx"), number("poty"), number("height"));
            let seen = flight.map(|(x, y, h)| (i32::from(x), i32::from(y), i32::from(h)));
            if seen != Some(expected) {
                return Err(format!("{name} row {i}: pot {seen:?}, native {expected:?}"));
            }
        }
        let mut native: Vec<u8> = row[column("sound")]
            .split('+')
            .filter(|s| !s.is_empty())
            .map(|s| u8::from_str_radix(s, 16).unwrap())
            .collect();
        // The break's `$13` natively comes at the stopping sample or a frame
        // later (`docs/pots.md`, open): ours at the sample stands for either.
        // Only the pot's own sounds: the lift, the release, the break. What
        // a struck door or resident says next is story (`local_story`).
        let pot_sound = |sound: &u8| matches!(sound, 0x11..=0x13);
        native.retain(pot_sound);
        let mut sounds: Vec<u8> = sounds.into_iter().filter(pot_sound).collect();
        if std::mem::take(&mut early_break) {
            if !native.contains(&BREAK) {
                return Err(format!("{name} row {i}: our break sound had no native one"));
            }
            native.retain(|&sound| sound != BREAK);
        }
        if sounds.contains(&BREAK) && !native.contains(&BREAK) {
            early_break = true;
            sounds.retain(|&sound| sound != BREAK);
        }
        if sounds != native {
            return Err(format!(
                "{name} row {i}: sounds {sounds:02x?}, native {native:02x?}"
            ));
        }
        let door = u16::from_str_radix(row[column("door640")], 16).unwrap();
        if world.counter(0) != door {
            return Err(format!(
                "{name} row {i}: door {}, native {door}",
                world.counter(0)
            ));
        }
    }
    Ok(())
}

#[test]
fn lifts_and_throws_match_native_traces_on_both_roms() {
    let mut failures = Vec::new();
    for (rom, revision) in [
        ("Tenchi Souzou (Japan).sfc", "jp"),
        ("Terranigma (E) [!].smc", "eu"),
    ] {
        let (Ok(bytes), Ok(recipes)) = (
            std::fs::read(local().join(rom)),
            std::fs::read_to_string(local().join("pots").join(revision).join("recipes.json")),
        ) else {
            continue;
        };
        let rom = Rom::load(&bytes).unwrap();
        let recipes: Value = serde_json::from_str(&recipes).unwrap();
        for (name, recipe) in recipes.as_object().unwrap() {
            if LATER.contains(&name.as_str()) {
                continue;
            }
            let csv = std::fs::read_to_string(
                local()
                    .join("pots")
                    .join(revision)
                    .join(format!("{name}.csv")),
            )
            .unwrap();
            if let Err(failure) = replay(rom.image(), name, recipe, &csv) {
                failures.push(format!("{revision} {failure}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn carrying_onto_an_exit_drops_the_pot() {
    for (rom, revision) in [
        ("Tenchi Souzou (Japan).sfc", "jp"),
        ("Terranigma (E) [!].smc", "eu"),
    ] {
        let (Ok(bytes), Ok(recipes)) = (
            std::fs::read(local().join(rom)),
            std::fs::read_to_string(local().join("pots").join(revision).join("recipes.json")),
        ) else {
            continue;
        };
        let rom = Rom::load(&bytes).unwrap();
        let recipes: Value = serde_json::from_str(&recipes).unwrap();
        let recipe = &recipes["exit-while-carrying"];
        // The prefix and the recorded rows, as a player would press them.
        let csv = std::fs::read_to_string(
            local()
                .join("pots")
                .join(revision)
                .join("exit-while-carrying.csv"),
        )
        .unwrap();
        let rows: Vec<String> = csv
            .lines()
            .skip(1)
            .map(|line| line.split(',').nth(2).unwrap().to_owned())
            .collect();
        let mut events = new_game_flags();
        for set in [0x26, 0x27, 0x28, 0x2E] {
            events[set / 8] |= 1 << (set % 8);
        }
        let mut world = World::enter_with_events(rom.image(), 0x000C, 104, 352, events).unwrap();
        world.face(Direction::Left);
        let (mut held_a, mut sounds) = (false, Vec::new());
        let prefix = recipe["prefix"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|step| {
                let buttons: Vec<String> = step["buttons"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|b| b.as_str().unwrap().to_owned())
                    .collect();
                std::iter::repeat_n(
                    buttons.join("+"),
                    usize::try_from(step["frames"].as_u64().unwrap()).unwrap(),
                )
            });
        for input in prefix
            .chain(rows)
            .chain(std::iter::repeat_n(String::new(), 200))
        {
            let buttons: Vec<&str> = input.split('+').filter(|b| !b.is_empty()).collect();
            sounds.extend(frame(&mut world, &buttons, &mut held_a));
        }
        assert_eq!(world.map(), 0x000D, "{revision}: through the exit");
        assert!(
            world.pot().is_none() && world.carry().is_none(),
            "{revision}: dropped"
        );
        let pot: Vec<u8> = sounds
            .into_iter()
            .filter(|s| matches!(s, 0x11..=0x13))
            .collect();
        assert_eq!(
            pot,
            [0x11, 0x12],
            "{revision}: the lift, then the drop's `$12`, no break"
        );
    }
}
