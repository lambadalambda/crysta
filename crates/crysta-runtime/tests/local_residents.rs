//! ROM-backed checks that residents appear and can be talked to.

use assets::maps::scripts::EventFlags;
use crysta_runtime::residents::{residents, talk_to, Conversation};
use crysta_runtime::MAPS;
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

/// The measured new-game flag state: 32 and 251 are set.
fn new_game() -> Vec<u8> {
    let mut bitmap = vec![0u8; 512];
    bitmap[32 / 8] |= 1 << (32 % 8);
    bitmap[251 / 8] |= 1 << (251 % 8);
    bitmap
}

#[test]
fn every_slice_map_resolves_its_residents() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let bitmap = new_game();
    let mut total = 0;
    for map in MAPS {
        let found = residents(cartridge.image(), map, EventFlags::Bitmap(&bitmap))
            .unwrap_or_else(|error| panic!("map {map:#06x}: {error}"));
        total += found.len();
    }
    assert!(total > 40, "only {total} residents across the slice");
}

#[test]
fn the_documented_resident_stands_where_the_game_puts_them() {
    // Cross-checked against actor positions read out of WRAM on the reference
    // emulator, not against a derived document.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let bitmap = new_game();
    let found = residents(cartridge.image(), 0x000B, EventFlags::Bitmap(&bitmap)).unwrap();
    let resident = found
        .iter()
        .find(|resident| resident.position == (120, 112))
        .expect("the documented resident stands at (120,112)");
    assert_eq!(resident.record, 0x03_8B96);
    assert_eq!(resident.script, Some(0x88_8E50));
    assert_eq!(resident.cell(), (7, 7));
}

/// The six `$08` conditions guarding the documented resident's callback, and
/// the flag their entry script pairs with one of them.
///
/// Each condition has bit 15 set, so each branches when its flag is **clear**.
/// Setting all six is what makes the walk fall through to the arm
/// `docs/house-dialogue.md` records. The entry script opens with a `$47`
/// despawn on the exclusive-or of `$027` and `$021`, so `$027` has to be set
/// alongside `$021` or the resident is not in the room to talk to.
const ALREADY_MET: [u16; 7] = [0x109, 0x03B, 0x296, 0x021, 0x028, 0x026, 0x027];

#[test]
fn the_documented_resident_speaks_their_own_pages() {
    // record -> script -> callback -> text, every hop derived rather than
    // tabulated, and the pages decoded from where the script points.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let bitmap = new_game();
    let found = residents(cartridge.image(), 0x000B, EventFlags::Bitmap(&bitmap)).unwrap();
    let resident = found
        .iter()
        .find(|resident| resident.position == (120, 112))
        .unwrap();
    // The documented pages are the already-met arm, not the first-visit one.
    let mut met = new_game();
    for flag in ALREADY_MET {
        met[usize::from(flag) / 8] |= 1 << (flag % 8);
    }
    match talk_to(cartridge.image(), resident, EventFlags::Bitmap(&met)) {
        Conversation::Speaks { pages, flags } => {
            assert_eq!(pages.len(), 2, "the resident's two pages");
            assert!(
                pages.iter().all(|page| !page.glyphs().is_empty()),
                "every page must carry glyphs"
            );
            // $0026 is the progression flag the documented chain writes.
            assert!(
                flags.iter().any(|flag| flag & 0x0FFF == 0x0026),
                "the progression flag was not written: {flags:04x?}"
            );
        }
        other => panic!("the documented resident must speak, got {other:?}"),
    }
}

#[test]
fn a_fresh_resident_says_something_different_from_a_met_one() {
    // The callback is a six-way dispatch over progression, so the flags choose
    // what is said. On a new game the first branch fires and the resident
    // gives their first-visit line, which is a choice prompt the text decoder
    // does not render -- reported as unsupported, not as silence.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let fresh = new_game();
    let found = residents(image, 0x000B, EventFlags::Bitmap(&fresh)).unwrap();
    let resident = found
        .iter()
        .find(|resident| resident.position == (120, 112))
        .unwrap();
    match talk_to(image, resident, EventFlags::Bitmap(&fresh)) {
        Conversation::Unsupported { source } => assert_eq!(source, 0x95B3),
        other => panic!("expected the first-visit choice prompt, got {other:?}"),
    }
}

#[test]
fn a_resident_the_flags_exclude_is_absent() {
    // Lists routinely repeat a position under different conditions, so the
    // flags decide who is present. Two different states must not agree
    // everywhere, or conditions are not being evaluated at all.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let fresh = new_game();
    let all_set = vec![0xFFu8; 512];
    let mut differed = 0;
    for map in MAPS {
        let (Ok(a), Ok(b)) = (
            residents(image, map, EventFlags::Bitmap(&fresh)),
            residents(image, map, EventFlags::Bitmap(&all_set)),
        ) else {
            continue;
        };
        if a != b {
            differed += 1;
        }
    }
    assert!(
        differed > 0,
        "no map's roster depends on the flags, so conditions are inert"
    );
}

#[test]
fn a_script_the_walker_cannot_follow_is_reported_not_silent() {
    // A resident who appears to have nothing to say must be distinguishable
    // from one the walker lost track of.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let bitmap = new_game();
    let (mut speaks, mut silent, mut unaccounted, mut unsupported) = (0, 0, 0, 0);
    for map in MAPS {
        let Ok(found) = residents(image, map, EventFlags::Bitmap(&bitmap)) else {
            continue;
        };
        for resident in &found {
            match talk_to(image, resident, EventFlags::Bitmap(&bitmap)) {
                Conversation::Speaks { .. } => speaks += 1,
                Conversation::Silent => silent += 1,
                Conversation::Unaccounted { .. } => unaccounted += 1,
                Conversation::Unsupported { .. } => unsupported += 1,
            }
        }
    }
    eprintln!(
        "residents: {speaks} speak, {silent} silent, {unaccounted} unaccounted, \
         {unsupported} unsupported"
    );
    assert!(
        speaks + unsupported > 0,
        "no resident in the slice reaches dialogue"
    );
    assert!(
        speaks + silent + unaccounted + unsupported > 40,
        "too few residents were considered"
    );
}

#[test]
fn a_resident_whose_despawn_condition_holds_is_absent() {
    // `$88:8E50` opens with `COP 47` on the exclusive-or of `$027` and
    // `$021`: the handler unlinks the actor when it holds. So the documented
    // resident is present on a new game, gone with `$021` alone, and back
    // when both are set.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let at_home = |flags: &[u8]| {
        residents(image, 0x000B, EventFlags::Bitmap(flags))
            .unwrap()
            .iter()
            .any(|resident| resident.position == (120, 112))
    };
    let fresh = new_game();
    assert!(at_home(&fresh), "present on a new game");
    let mut one = new_game();
    one[0x21 / 8] |= 1 << (0x21 % 8);
    assert!(!at_home(&one), "despawned with $021 alone");
    let mut both = one.clone();
    both[0x27 / 8] |= 1 << (0x27 % 8);
    assert!(at_home(&both), "present again with $027 as well");
}
