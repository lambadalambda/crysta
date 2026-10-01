//! Depth facts from the native game (`docs/depth-order.md`): Yomi's
//! priority 3 in the Box, the weapon's place and its sparkle's loop.
use crysta_runtime::scene::Presses;
use crysta_runtime::world::World;
use rom::Rom;
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
