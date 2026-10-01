//! Depth facts from the native game (`docs/depth-order.md`): Yomi's
//! priority 3 in the Box, the weapon's place and its sparkle's loop.
use crysta_runtime::scene::Presses;
use crysta_runtime::world::World;
use rom::Rom;
use room_core::Direction;
use std::path::Path;

fn load(name: &str) -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(name);
    Rom::load(&std::fs::read(path).ok()?).ok()
}

#[test]
fn yomi_draws_over_the_bookcases_and_the_weapon_sparkles_on_its_pedestal() {
    for name in ["Tenchi Souzou (Japan).sfc", "Terranigma (E) [!].smc"] {
        let Some(rom) = load(name) else {
            continue;
        };
        // Yomi's guide script sets OBJ priority 3 (`COP BA $30`).
        let mut world = World::enter(rom.image(), 0x41, 136, 200).unwrap();
        for _ in 0..40 {
            world.update(None, Presses::NONE).unwrap();
        }
        let yomi = world
            .residents()
            .iter()
            .find(|r| r.selector != 0 && !r.hidden);
        assert_eq!(yomi.map(|r| r.priority), Some(3), "{name}");
        // The weapon: `COP B2 $FFF8` lifts it to y 360; its sparkle loops
        // every 24 frames (`COP D8 $A2:C000` lists, `COP 8E`).
        let mut world = World::enter(rom.image(), 0x42, 136, 464).unwrap();
        let mut ages = Vec::new();
        for _ in 0..60 {
            world.update(None, Presses::NONE).unwrap();
            let weapon = world.residents().iter().find(|r| r.selector == 8).unwrap();
            assert_eq!(weapon.position, (72, 360), "{name}");
            ages.push(weapon.pose_age);
        }
        assert!(
            ages.windows(2).any(|pair| pair == [23, 0]),
            "{name}: {ages:?}"
        );
        assert_eq!(ages.iter().max(), Some(&23), "{name}");
    }
}

#[test]
fn walking_up_into_a_door_raises_the_mask_over_ark() {
    // The door-walk helper's priority-1 mask (`docs/depth-order.md`): from
    // 17 pixels into the walk, anchored 16 pixels above where it began.
    let up = Some(room_core::Direction::Up);
    let a = Presses {
        confirm: true,
        ..Presses::NONE
    };
    // The native recipe (`local/depth/README.txt`): Up, open the door with
    // A, wait, then hold Up through it.
    let steps = [
        (8, up, Presses::NONE),
        (20, None, Presses::NONE),
        (1, up, a),
        (100, None, Presses::NONE),
        (120, up, Presses::NONE),
    ];
    for name in ["Tenchi Souzou (Japan).sfc", "Terranigma (E) [!].smc"] {
        let Some(rom) = load(name) else {
            continue;
        };
        let mut world = World::enter(rom.image(), 0x0C, 136, 352).unwrap();
        let mut seen = None;
        for (frames, direction, presses) in steps {
            for _ in 0..frames {
                world.update(direction, presses).unwrap();
                let masks = world.masks();
                if !masks.is_empty() {
                    seen.get_or_insert((world.position(), masks));
                }
            }
        }
        // Anchored at the threshold (136,336): x -8..+8 on rows -64..-56,
        // x -16..+16 on rows -56..-32.
        assert_eq!(
            seen,
            Some(((136, 335), vec![(128, 272, 144, 280), (120, 280, 152, 304)])),
            "{name}"
        );
    }
}

#[test]
fn the_stairs_mask_hides_ark_in_the_stairwell() {
    // `docs/depth-order.md` section 4: going down from E, pose `$3B` at Ark's
    // place on the exit frame (104,864) for 18 frames, then `$3A`; arriving
    // in `$20`, pose `$39` at the stair tile (408,864).
    let up = Some(Direction::Up);
    for name in ["Tenchi Souzou (Japan).sfc", "Terranigma (E) [!].smc"] {
        let Some(rom) = load(name) else {
            continue;
        };
        let mut events = crysta_runtime::world::fresh_game_flags();
        for set in [0x26, 0x27, 0x28, 0x2E] {
            events[set / 8] |= 1 << (set % 8);
        }
        let mut world = World::enter_with_events(rom.image(), 0x0E, 104, 880, events).unwrap();
        let mut leaving = Vec::new();
        let mut arriving = Vec::new();
        for _ in 0..400 {
            let direction = (!world.in_transition()).then_some(up).flatten();
            world.update(direction, Presses::NONE).unwrap();
            let masks = world.masks();
            match world.map() {
                0x0E if world.in_transition() => leaving.push(masks),
                0x20 if world.in_transition() => arriving.push(masks),
                0x20 => break,
                _ => {}
            }
        }
        let three_b = vec![(96, 824, 128, 832), (112, 832, 128, 872)];
        assert_eq!(leaving[0], three_b, "{name}");
        assert_eq!(leaving[17], three_b, "{name}");
        let mut three_a = three_b.clone();
        three_a.extend([
            (112, 872, 128, 880),
            (96, 860, 112, 872),
            (104, 872, 112, 880),
        ]);
        assert_eq!(leaving[18], three_a, "{name}");
        let three_nine = vec![
            (384, 816, 400, 864),
            (400, 816, 408, 832),
            (408, 824, 416, 832),
        ];
        assert!(arriving.contains(&three_nine), "{name}");
        assert!(world.masks().is_empty(), "{name}");
    }
}
