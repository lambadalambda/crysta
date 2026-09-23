//! Optional authenticated ROM-only actor spawn fixtures.
use assets::maps::actors::{ActorError, SpawnList, SpawnRecord};
use rom::{Revision, Rom};
use std::path::Path;

fn owned_rom() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    match std::fs::read(&path) {
        Ok(bytes) => {
            let cartridge = Rom::load(&bytes).unwrap();
            assert_eq!(cartridge.revision(), Revision::Japan);
            Some(cartridge)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping: local Japanese ROM not present");
            None
        }
        Err(e) => panic!("reading {}: {e}", path.display()),
    }
}

/// The nine residents `docs/house-scene.md` documents, with their origins.
const DOCUMENTED: [(u16, usize, u16, u16); 9] = [
    (0x000B, 0x03_8B96, 120, 112),
    (0x000C, 0x03_8C0A, 88, 416),
    (0x000C, 0x03_8C14, 56, 384),
    (0x000C, 0x03_8C1E, 72, 368),
    (0x000C, 0x03_8C28, 104, 368),
    (0x000D, 0x03_8CB4, 72, 672),
    (0x0010, 0x03_8D7C, 424, 416),
    (0x0010, 0x03_8D86, 440, 416),
    (0x0011, 0x03_8DE2, 440, 640),
];

#[test]
fn every_documented_resident_is_decoded_at_its_source_address() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let mut checked = 0;
    for (map, offset, x, y) in DOCUMENTED {
        let list = SpawnList::from_rom(cartridge.image(), map)
            .unwrap_or_else(|e| panic!("map {map:#06x}: {e}"));
        let records = list.records().to_vec();
        let found = records
            .iter()
            .find(|record| record.offset() == offset)
            .unwrap_or_else(|| panic!("map {map:#06x} missing record at {offset:#08x}"));
        assert_eq!(found.origin(), (x, y), "map {map:#06x} at {offset:#08x}");
        assert_eq!(found.opcode(), 0x01);
        checked += 1;
    }
    // All nine, now that every element length comes from the interpreter.
    assert_eq!(checked, DOCUMENTED.len());
}

#[test]
fn every_crysta_spawn_list_decodes_end_to_end() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let mut total = 0;
    for map in 0x000Au16..=0x0021 {
        let list = SpawnList::from_rom(cartridge.image(), map)
            .unwrap_or_else(|e| panic!("map {map:#06x}: {e}"));
        assert!(!list.records().is_empty(), "map {map:#06x} has no records");
        // Every decoded position must be inside a plausible map extent. A
        // misparse reads operand bytes as tiles and lands far outside.
        for record in list.records() {
            let (x, y) = record.origin();
            assert!(x < 2048 && y < 2048, "map {map:#06x} origin ({x},{y})");
        }
        total += list.records().len();
    }
    assert_eq!(total, 115, "record count across the slice");
}

#[test]
fn an_unaccounted_opcode_is_refused_rather_than_resynchronised() {
    // Guessing a length would emit positions read from operand bytes, which
    // look exactly like plausible spawns, so an unknown opcode must be fatal.
    let mut image = vec![0u8; 0x04_0000];
    // A list whose first element is an opcode the interpreter does not define.
    image[0x03_8000..0x03_8002].copy_from_slice(&0x9000u16.to_le_bytes());
    image[0x03_9000..0x03_9004].copy_from_slice(&[0x00, 0x06, 0x7C, 0x11]);
    assert!(matches!(
        SpawnList::from_rom(&image, 0),
        Err(ActorError::Unqualified {
            opcode: 0x7C,
            selector: 0x11,
            offset: 2,
        })
    ));
}

#[test]
fn bounds_and_absent_lists_are_rejected() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    assert_eq!(
        SpawnList::from_rom(cartridge.image(), 0x0450),
        Err(ActorError::MapIndex { index: 0x0450 })
    );
    assert!(matches!(
        SpawnList::from_rom(&[], 0),
        Err(ActorError::Truncated { .. })
    ));
}

#[test]
fn decoded_origins_match_positions_measured_in_the_running_game() {
    // Stronger than matching the scene census, which is itself a derived
    // document: these are actor slot positions read out of WRAM while walking
    // the reference emulator through the house.
    //
    // Note the installer at `$80:F52B` stores `tile * 16` with no bias, yet the
    // running game reports the origin eight pixels right. The `+8` is applied
    // somewhere after installation; `origin()` reports what the game shows.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    for (map, measured) in [
        (0x000Bu16, &[(120u16, 112u16)][..]),
        (0x000C, &[(56, 384), (72, 368), (88, 416), (104, 368)][..]),
    ] {
        let list = SpawnList::from_rom(cartridge.image(), map).unwrap();
        let decoded: Vec<(u16, u16)> = list.records().iter().map(SpawnRecord::origin).collect();
        for position in measured {
            assert!(
                decoded.contains(position),
                "map {map:#06x}: runtime actor at {position:?} not in {decoded:?}"
            );
        }
    }
}

/// The measured new-game event-flag state: flags 32 and 251 set.
///
/// `docs/new-game-bootstrap.md` records `$06C0..0700` as having only offsets
/// 4 and 31 nonzero on a fresh save.
fn new_game_flags() -> Vec<u8> {
    let mut bitmap = vec![0u8; 512];
    bitmap[4] = 1;
    bitmap[31] = 8;
    bitmap
}

#[test]
fn a_fresh_game_resolves_every_crysta_spawn_list() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let flags = new_game_flags();
    for map in 0x000Au16..=0x0021 {
        let active = SpawnList::resolve(
            cartridge.image(),
            map,
            assets::maps::scripts::EventFlags::Bitmap(&flags),
        )
        .unwrap_or_else(|e| panic!("map {map:#06x}: {e}"));
        assert!(!active.is_empty(), "map {map:#06x} resolves to nothing");
    }
}

#[test]
fn fresh_ordinary_records_reproduce_the_scene_census() {
    // `docs/house-scene.md` counts *visible ordinary residents*, excluding
    // hidden interaction actors. Those are `$01` records here too, so the
    // counts agree in four rooms and exceed the census in the two that have a
    // documented hidden actor.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let flags = new_game_flags();
    // (map, census count, ordinary $01 records this resolves)
    for (map, census, resolved) in [
        (0x000Bu16, 1usize, 1usize),
        (0x000C, 4, 5),
        (0x000D, 1, 2),
        (0x000F, 0, 0),
        (0x0010, 2, 2),
        (0x0011, 1, 1),
    ] {
        let active = SpawnList::resolve(
            cartridge.image(),
            map,
            assets::maps::scripts::EventFlags::Bitmap(&flags),
        )
        .unwrap();
        // The `(8,16)` record appears in every list and is not a resident.
        let ordinary: Vec<(u16, u16)> = active
            .iter()
            .filter(|record| record.opcode() == 0x01 && record.origin() != (8, 16))
            .map(SpawnRecord::origin)
            .collect();
        assert_eq!(ordinary.len(), resolved, "map {map:#06x}: {ordinary:?}");
        assert!(resolved >= census, "map {map:#06x} lost a census resident");
    }
}

#[test]
fn map_000d_resolves_the_documented_hidden_gate_actor() {
    // `docs/house-scene.md`: "The hidden actor at `(120,720)` supplies
    // occupancy while event `$0026` is clear." It is one of the two records
    // that put a room above its census count, which is what identifies the
    // excess as documented rather than as a decode error.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let flags = new_game_flags();
    let active = SpawnList::resolve(
        cartridge.image(),
        0x000D,
        assets::maps::scripts::EventFlags::Bitmap(&flags),
    )
    .unwrap();
    let at_gate: Vec<u8> = active
        .iter()
        .filter(|record| record.origin() == (120, 720))
        .map(SpawnRecord::opcode)
        .collect();
    assert_eq!(at_gate, [0x01, 0xFD], "records at the gate: {at_gate:02x?}");
}

#[test]
fn conditions_actually_select_records() {
    // Resolution must depend on flag state, or it is just a linear walk with
    // extra steps.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let fresh = new_game_flags();
    let all_set = vec![0xFFu8; 512];
    let mut differing = 0;
    for map in 0x000Au16..=0x0021 {
        let a = SpawnList::resolve(
            cartridge.image(),
            map,
            assets::maps::scripts::EventFlags::Bitmap(&fresh),
        )
        .unwrap();
        let Ok(b) = SpawnList::resolve(
            cartridge.image(),
            map,
            assets::maps::scripts::EventFlags::Bitmap(&all_set),
        ) else {
            continue;
        };
        if a != b {
            differing += 1;
        }
    }
    assert!(
        differing >= 8,
        "only {differing} maps changed with every flag set"
    );
}

#[test]
fn spawn_record_pointers_are_scripts_rather_than_text() {
    // Dialogue is not reachable from a spawn record by decoding its pointers.
    // Of the 189 pointer fields across the slice's records, five decode as
    // dialogue and each yields a single page, which is what a coincidentally
    // valid byte sequence looks like rather than a resident's lines.
    //
    // Reaching a resident's dialogue means following the script the record
    // points at, which is the shared dependency behind dialogue and
    // progression.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let (mut tried, mut decoded) = (0, 0);
    for map in 0x000Au16..=0x0021 {
        let list = SpawnList::from_rom(cartridge.image(), map).unwrap();
        for record in list.records() {
            let bytes = record.bytes();
            for at in [4usize, 7] {
                let Some(field) = bytes.get(at..at + 3) else {
                    continue;
                };
                let pointer =
                    u32::from(field[0]) | (u32::from(field[1]) << 8) | (u32::from(field[2]) << 16);
                tried += 1;
                if assets::text::HouseDialogue::decode_at(cartridge.image(), pointer)
                    .is_ok_and(|pages| !pages.is_empty())
                {
                    decoded += 1;
                }
            }
        }
    }
    assert_eq!((tried, decoded), (189, 5));
}

#[test]
fn decode_at_reproduces_the_qualified_text_sources() {
    // The generalised entry must agree with the compiled list, and reject an
    // address that is not a text source rather than returning noise.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let dialogue = assets::text::HouseDialogue::from_rom(cartridge.image()).unwrap();
    let mut checked = 0;
    for source in assets::text::TEXT_SOURCES {
        let compiled = dialogue.pages(source).expect("a compiled source");
        let decoded = assets::text::HouseDialogue::decode_at(cartridge.image(), source)
            .expect("the same source decodes");
        assert_eq!(compiled.len(), decoded.len(), "source {source:#08x}");
        for (a, b) in compiled.iter().zip(&decoded) {
            assert_eq!(a.indexed(), b.indexed(), "source {source:#08x}");
            assert_eq!(a.acknowledgement(), b.acknowledgement());
            assert_eq!(a.boundary_source(), b.boundary_source());
        }
        checked += 1;
    }
    assert_eq!(checked, assets::text::TEXT_SOURCES.len());
    // Addresses outside the ROM window, and RAM, are refused.
    for source in [0u32, 0x7E_0000, 0xC0_8000] {
        assert!(assets::text::HouseDialogue::decode_at(cartridge.image(), source).is_err());
    }
}

#[test]
fn the_documented_callback_yields_its_dialogue_and_its_flag() {
    // The chain `docs/house-dialogue.md` records by hand -- resident $838B96,
    // callback $888EDE, flag write at $888F08, text $888FF0 -- decoded by
    // walking COP commands with operand lengths read from their handlers.
    use assets::maps::actor_script::{walk, Stop, SHOW_TEXT, WRITE_FLAG};
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let effects = walk(cartridge.image(), 0x88_8EDE).unwrap();
    assert_eq!(effects.text, [0x8FF0], "the resident's dialogue source");
    assert_eq!(effects.flags, [0x8026], "the $0026 progression flag, set");

    // At the addresses the document names.
    let site = |service: u8| {
        effects
            .commands
            .iter()
            .find(|command| command.service == service)
            .map(|command| command.offset)
    };
    assert_eq!(site(SHOW_TEXT), Some(0x08_8F02));
    assert_eq!(site(WRITE_FLAG), Some(0x08_8F08));

    // Bit 15 of a flag operand selects set over clear, the same encoding
    // `map-inspector`'s new-game projection derives from COP 07.
    assert_eq!(effects.flags[0] & 0x0FFF, 0x0026);
    assert_ne!(effects.flags[0] & 0x8000, 0);

    // And the walk stops rather than running off into data. `$88:8F0C` is
    // `COP $1A`, whose advance is not accounted for.
    assert_eq!(
        effects.stop,
        Stop::Unaccounted {
            offset: 0x08_8F0C,
            service: 0x1A
        }
    );
}

#[test]
fn a_walked_text_address_decodes_as_dialogue() {
    // Closes the loop: the address the script asks for is a real text source,
    // not an address that merely looks like one.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let effects = assets::maps::actor_script::walk(cartridge.image(), 0x88_8EDE).unwrap();
    let source = 0x88_0000 | u32::from(effects.text[0]);
    assert_eq!(source, assets::text::FIRST_TEXT);
    let pages = assets::text::HouseDialogue::decode_at(cartridge.image(), source).unwrap();
    assert!(!pages.is_empty(), "the walked source has dialogue pages");
}

#[test]
fn operand_lengths_come_from_the_handlers() {
    use assets::maps::actor_script::operand_length;
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    // $80:8669 advances $36 twice; $80:8BEB likewise, on the path that is not
    // its text-busy stall.
    assert_eq!(operand_length(image, 0x07), Some(2));
    assert_eq!(operand_length(image, 0x1B), Some(2));
    assert_eq!(operand_length(image, 0x21), Some(2));
    // $80:8678 and $80:96CB open identically -- the same eight bytes of flag
    // test -- but are *not* the same length. Their not-taken arms differ:
    // $80:8397 adds four, so $08 carries a condition and a branch target, while
    // $80:8390 adds two, so $48 carries a condition alone and aborts. The
    // stream agrees: `02 48 74 80` is followed by `02 47`.
    assert_eq!(operand_length(image, 0x08), Some(4));
    assert_eq!(operand_length(image, 0x48), Some(2));
    // $80:8C4A pushes $36, runs the text renderer to completion and restores
    // it, so it takes none. It reaches that through a back edge, which is why
    // the derivation has to tolerate loops.
    assert_eq!(operand_length(image, 0x1F), Some(0));
    // $80:AB17 commits to the actor slot's own script pointer rather than to
    // the dispatcher's resume address: the actor yields and continues there.
    assert_eq!(operand_length(image, 0xC1), Some(2));
    // $80:9301 and $80:AA33 never read through $36.
    assert_eq!(operand_length(image, 0x3B), Some(0));
    assert_eq!(operand_length(image, 0xB6), Some(0));
    // $80:8D0B advances once, an odd length the stream confirms: `02 24 06 78
    // 8e` is followed by `02 b6`.
    assert_eq!(operand_length(image, 0x24), Some(3));
    // $80:BC2F reads a tile coordinate from the stream inside a call. COP 13
    // takes a column and a row through it plus a facing byte: `02 13 0b 1a
    // 01` is followed by `02 08`. COP 0F reads two through it.
    assert_eq!(operand_length(image, 0x13), Some(3));
    assert_eq!(operand_length(image, 0x0F), Some(5));
}

#[test]
fn a_chained_condition_is_sized_from_the_stream_not_the_handler() {
    use assets::maps::actor_script::{chained_condition_length, operand_length, CHAINED_CONDITION};
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    // $80:8695 and $80:963A pull in a further condition word for as long as the
    // previous one has a bit in $F000, so no single length describes them and
    // the handler derivation must refuse them.
    for service in CHAINED_CONDITION {
        assert_eq!(
            operand_length(image, service),
            None,
            "service {service:02x}"
        );
    }
    // `02 47 27 10 21 00`: $1027 has $1000 set and chains, $0021 does not.
    assert_eq!(chained_condition_length(image, 0x08_8E56), Some(4));
    assert_eq!(image[0x08_8E5A..0x08_8E5C], [0x02, 0x3B], "lands on a COP");
    // A lone word ends the chain immediately.
    assert_eq!(chained_condition_length(&[0x21, 0x00], 0), Some(2));
    // Truncation is refused rather than guessed.
    assert_eq!(chained_condition_length(&[0x00, 0x10], 0), None);
}

#[test]
fn a_spawn_record_reaches_its_script_five_bytes_past_its_pointer() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    // Ground truth, read out of WRAM while the reference emulator ran map
    // $000F: the actor at (8,16) carries script $88:803D in its slot, and that
    // map's record for (8,16) holds the pointer $88:8038.
    let list = SpawnList::from_rom(image, 0x000F).unwrap();
    let measured = list
        .records()
        .iter()
        .find(|record| record.origin() == (8, 16))
        .expect("map $000F spawns an actor at (8,16)");
    assert_eq!(measured.script(), Some(0x88_803D));

    // The documented resident, whose whole chain is recorded by hand.
    let list = SpawnList::from_rom(image, 0x000B).unwrap();
    let resident = list
        .records()
        .iter()
        .find(|record| record.origin() == (120, 112))
        .expect("the documented resident");
    assert_eq!(resident.offset(), 0x03_8B96);
    assert_eq!(resident.script(), Some(0x88_8E50));

    // Across the slice the offset is what makes the entry a `COP` command:
    // the pointer itself lands on a five-byte header. This is a census, not a
    // universal: a minority of records use a form whose pointer does not
    // resolve this way, and those are reported rather than assumed away.
    let (mut cop_at_five, mut cop_at_zero, mut neither) = (0, 0, 0);
    for map in 0x000A..=0x0021u16 {
        let Ok(list) = SpawnList::from_rom(image, map) else {
            continue;
        };
        for record in list.records() {
            let Some(script) = record.script() else {
                continue;
            };
            let at = (script & 0x3F_FFFF) as usize;
            if image[at] == 0x02 {
                cop_at_five += 1;
            } else if image[at - 5] == 0x02 {
                cop_at_zero += 1;
            } else {
                neither += 1;
            }
        }
    }
    assert_eq!(
        (cop_at_five, cop_at_zero, neither),
        (90, 0, 25),
        "the slice's record-pointer census"
    );
}

#[test]
fn the_resident_chain_runs_from_the_spawn_record_without_a_table() {
    use assets::maps::actor_script::walk;
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let list = SpawnList::from_rom(image, 0x000B).unwrap();
    let resident = list
        .records()
        .iter()
        .find(|record| record.origin() == (120, 112))
        .expect("the documented resident");

    // record -> script -> callback -> text and flag, every hop derived.
    let script = resident.script().unwrap();
    let effects = walk(image, script).unwrap();
    assert_eq!(effects.callbacks, [0x8EDE], "the registered callback");
    let callback = walk(
        image,
        (script & 0xFF_0000) | u32::from(effects.callbacks[0]),
    )
    .unwrap();
    assert_eq!(callback.text, [0x8FF0]);
    assert_eq!(callback.flags, [0x8026]);
    let pages =
        assets::text::HouseDialogue::decode_at(image, 0x88_0000 | u32::from(callback.text[0]))
            .unwrap();
    assert_eq!(pages.len(), 2, "the resident's two pages");
}

#[test]
fn following_a_branch_needs_the_flags_that_decide_it() {
    use assets::maps::actor_script::{walk, walk_with_events};
    use assets::maps::scripts::EventFlags;
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    // $88:8EDE opens with six `$08` branches: a dispatch over progression
    // state. A straight-line walk falls through all six and reports the
    // fall-through arm's effects.
    let straight = walk(image, 0x88_8EDE).unwrap();
    assert_eq!(straight.text, [0x8FF0]);

    // With flags supplied, the first branch whose condition holds is taken, so
    // the walk reports a different arm. The first condition is $8109: bit 15
    // set, so it branches when flag $109 is set ($80:8678).
    let mut bitmap = vec![0u8; 512];
    bitmap[0x109 / 8] |= 1 << (0x109 % 8);
    let taken = walk_with_events(image, 0x88_8EDE, EventFlags::Bitmap(&bitmap)).unwrap();
    assert_ne!(
        taken.commands.get(1).map(|command| command.offset),
        straight.commands.get(1).map(|command| command.offset),
        "a taken branch must leave the fall-through path"
    );

    // Flags that cannot answer the question are refused rather than assumed.
    assert!(walk_with_events(image, 0x88_8EDE, EventFlags::Bitmap(&[])).is_err());
}

#[test]
fn only_the_flag_branch_service_is_followed_as_a_branch() {
    use assets::maps::actor_script::{walk_with_events, Stop};
    use assets::maps::scripts::EventFlags;
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();

    // Twenty services other than `$08` also take four operand bytes. Sizing a
    // branch by its length rather than its service reads their operands as a
    // condition and a target, and the target is decodable, so the walk
    // resynchronises onto a stream that does not exist.
    //
    // The documented resident's own script is the case: `$88:8E54` is
    // `COP $47`, a chained condition, whose words are `$1027` and `$0021`.
    // Read as a branch with flag $027 set, `$0021` becomes a target.
    let all_set = vec![0xFFu8; 512];
    let effects = walk_with_events(image, 0x88_8E50, EventFlags::Bitmap(&all_set)).unwrap();
    assert!(
        effects.commands.len() > 1,
        "the walk must not leave the script at its first command"
    );
    for command in &effects.commands {
        assert!(
            command.offset & 0xFFFF >= 0x8000,
            "command at {:#08x} is below $8000, so the walk left the script",
            command.offset
        );
    }
    assert!(
        !matches!(effects.stop, Stop::EndOfCommands { offset, .. } if offset & 0xFFFF < 0x8000),
        "stopped at {:?}, outside ROM",
        effects.stop
    );

    // Slice-wide, under both flag states: no walk may end outside ROM.
    let mut new_game = vec![0u8; 512];
    new_game[4] = 1;
    new_game[31] = 8;
    for bitmap in [&new_game, &all_set] {
        for map in 0x000A..=0x0021u16 {
            let Ok(list) = SpawnList::from_rom(image, map) else {
                continue;
            };
            for record in list.records() {
                let Some(script) = record.script() else {
                    continue;
                };
                let Ok(effects) = walk_with_events(image, script, EventFlags::Bitmap(bitmap))
                else {
                    continue;
                };
                for command in &effects.commands {
                    assert!(
                        command.offset & 0xFFFF >= 0x8000,
                        "map {map:#06x} script {script:#08x} reached {:#08x}",
                        command.offset
                    );
                }
            }
        }
    }
}

#[test]
fn a_chained_branch_carries_a_target_and_a_negated_word_ends_the_chain() {
    use assets::maps::actor_script::{chained_condition_length, walk_with_events, Stop};
    use assets::maps::scripts::EventFlags;
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    // `02 09 2a 10 2c 00 e0 98` at $88:98AE: $102A chains, $002C ends, and
    // the two bytes after are the target $98E0 -- which is a COP.
    assert_eq!(chained_condition_length(image, 0x08_98B0), Some(4));
    assert_eq!(
        image[0x08_98E0..0x08_98E2],
        [0x02, 0x13],
        "target lands on a COP"
    );
    // `02 47 2a 10 2c 80 02 99` at $88:804D: $802C has bit 15, so the chain
    // ends there and the halt form has no target; `02 99` is the next command.
    assert_eq!(chained_condition_length(image, 0x08_804F), Some(4));
    assert_eq!(
        image[0x08_8053..0x08_8055],
        [0x02, 0x99],
        "the next COP follows"
    );
    // Walking the map-$0010 resident's script on a new game now reaches its
    // ordinary loop: register callback, clear the mirror, select pose 2, wait.
    let mut bitmap = vec![0u8; 512];
    bitmap[4] |= 1;
    bitmap[31] |= 1 << 3;
    let walked = walk_with_events(image, 0x88_98AE, EventFlags::Bitmap(&bitmap)).unwrap();
    let services: Vec<u8> = walked.commands.iter().map(|c| c.service).collect();
    assert!(
        services.ends_with(&[0x21, 0x65, 0x23, 0xB6, 0x80, 0x8E]),
        "{services:02x?} stopping {:?}",
        walked.stop
    );
    assert_eq!(
        walked.stop,
        Stop::EndOfCommands {
            offset: 0x08_98DE,
            opcode: 0x80
        },
        "the BRA back to the wait"
    );
}

#[test]
fn map_0021_resolves_its_fe_entry_controller() {
    // `docs/house-scene.md`: `$8392AD` FE, `$88AD84` -> `$88AD89`, at (8,0).
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let flags = new_game_flags();
    let active = SpawnList::resolve(
        cartridge.image(),
        0x0021,
        assets::maps::scripts::EventFlags::Bitmap(&flags),
    )
    .unwrap();
    let controller = active
        .iter()
        .find(|record| record.offset() == 0x03_92AD)
        .expect("the FE record");
    assert_eq!(controller.opcode(), 0xFE);
    assert_eq!(controller.origin(), (8, 0));
    assert_eq!(controller.script(), Some(0x88_AD89));
}
