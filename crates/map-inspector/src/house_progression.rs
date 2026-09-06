//! Authenticated, bounded house conversation and exterior compiler; no event VM.
use crate::{house_navigation, invalid, room_dialogue, sha256, Result};
use assets::{
    maps::visual::StaticBackground,
    text::{Acknowledgement, HouseDialogue, TEXT_SOURCES},
};
use room_core::{
    conversation::{ConversationPages, ConversationSpec},
    slice::{DataIdentity, GameData},
    Room,
};
use serde_json::{json, Value};

const SOURCE: &str = include_str!("../../../tools/house-conversation-qualification/source.json");
const TEXT: &str = include_str!("../../../tools/house-dialogue-qualification/reference.json");
const EXTERIOR: &str = include_str!("../../../tools/house-exterior-qualification/reference.json");
const POLICY: &[u8] = b"house-progression-v1:source-graph,grant-before-choice,gate-on-reload,passive-A,no-A-actors-or-exits,halo-half-open-29-47-36-53,camera-256x256-follow-128x112";

pub(super) fn compile(rom: &rom::Rom) -> Result<GameData> {
    let (house, base) = house_navigation::compile_with_identity(rom)?;
    let source: Value = serde_json::from_str(SOURCE)?;
    let text: Value = serde_json::from_str(TEXT)?;
    let exterior: Value = serde_json::from_str(EXTERIOR)?;
    let rom_hash = json!(sha256(rom.image()));
    for hash in [
        &source["rom_sha256"],
        &text["rom_sha256"],
        &exterior["contract"]["rom_sha256"],
    ] {
        ensure(*hash == rom_hash, "progression ROM identity")?;
    }
    let mut source_content = Vec::new();
    authenticate_conversation(rom.image(), &source, &mut source_content)?;
    let dialogue = HouseDialogue::from_rom(rom.image())?;
    let (pages, text_content) = compile_dialogue(&dialogue, &text)?;
    let (grid, exterior_content) = compile_exterior(rom, &exterior)?;
    let identity = aggregate(base, [&source_content, &text_content, &exterior_content]);
    Ok(house.with_progression(ConversationSpec::new(pages)?, grid, identity)?)
}

fn ensure(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(invalid(message).into())
    }
}

// Fixed-size section digests make the ordered identity encoding unambiguous.
fn aggregate(base: DataIdentity, sections: [&[u8]; 3]) -> DataIdentity {
    let mut content = POLICY.to_vec();
    content.push(room_core::slice::PROFILE_VERSION);
    content.extend(base.rom_sha256);
    content.extend(base.content_sha256); // Includes fresh startup flags and position.
    for section in sections {
        content.extend(rom::digests(section).sha256);
    }
    DataIdentity {
        rom_sha256: base.rom_sha256,
        content_sha256: rom::digests(&content).sha256,
    }
}

/// Conversation pins use CPU addresses; exterior pins use normalized offsets.
fn authenticate_ranges(
    image: &[u8],
    pins: &Value,
    count: usize,
    banked: bool,
    content: &mut Vec<u8>,
) -> Result<()> {
    let pins = pins
        .as_array()
        .ok_or_else(|| invalid("missing progression source ranges"))?;
    ensure(pins.len() == count, "progression source range count")?;
    for pin in pins {
        let address = |key: &str| -> Result<usize> {
            let value = usize::try_from(
                pin[key]
                    .as_u64()
                    .ok_or_else(|| invalid("progression source offset"))?,
            )?;
            ensure(
                if banked {
                    (0x80_0000..0xc0_0000).contains(&value)
                } else {
                    value <= 0x40_0000
                },
                "progression source address space",
            )?;
            Ok(if banked { value & 0x3f_ffff } else { value })
        };
        let (start, end) = (address("start")?, address("end")?);
        ensure(start < end, "progression source extent")?;
        let bytes = image
            .get(start..end)
            .ok_or_else(|| invalid("truncated progression source"))?;
        ensure(
            pin["sha256"] == json!(sha256(bytes)),
            &format!("progression source hash at {start:#x}..{end:#x}"),
        )?;
        content.extend(u32::try_from(start)?.to_le_bytes());
        content.extend(u32::try_from(end)?.to_le_bytes());
        content.extend(bytes);
    }
    Ok(())
}

fn authenticate_conversation(image: &[u8], source: &Value, content: &mut Vec<u8>) -> Result<()> {
    authenticate_ranges(image, &source["ranges"], 20, true, content)?;
    // The fixed core graph implements precisely these authenticated operands.
    // Reject semantic metadata drift as well as changed code/data bytes. This is
    // an admission contract, not fixture-initialized actors, events or positions.
    let expected = json!({
        "resident": {"source":0x83_8b96,"position":[120,112],"callback":0x88_8ede,"entry_request":0x88_8fda},
        "first": {"request":0x88_8ff0,"set_flag_source":0x88_8f08,"flag":0x26,
            "choice":{"source":0x88_8f0c,"catalog":0,"table":0x88_8f11,"cancel_then_options":[0x88_8f17,0x88_8f1d,0x88_8f17]},
            "first_option_followup":0x88_90d9,"second_or_cancel_followup":0x88_905a},
        "repeat": {"request":0x88_9156,
            "choice":{"source":0x88_8f29,"catalog":1,"table":0x88_8f2e,"cancel_then_options":[0x88_8f3b,0x88_8f34,0x88_8f3b]},
            "first_option_followup":0x88_918c,"second_or_cancel_followup":0x88_91d6},
        "gate": {"source":0x83_8cc8,"position":[120,720],"entry":0x88_a9b4,"flag":0x26,"occupancy_cell":1415,
            "saved_continuation":0x88_a9bc,"policy":"reject-if-set-at-load; otherwise stamp-and-return-forever"},
        "exit": {"source":0x81_8dfe,"rectangle":[7,44,1,4],"destination":10,"mode":0,"selector":5,
            "raw_position":[496,752],"destination_player_record":0x83_89b8}
    });
    for (key, value) in expected.as_object().expect("object") {
        ensure(
            source[key] == *value,
            &format!("progression semantic source contract: {key}"),
        )?;
    }
    content.extend(serde_json::to_vec(&expected)?);
    Ok(())
}

fn acknowledgement(ack: Acknowledgement) -> &'static str {
    match ack {
        Acknowledgement::Next => "next",
        Acknowledgement::End => "end",
        Acknowledgement::None => "none",
    }
}

fn compile_dialogue(
    dialogue: &HouseDialogue,
    pins: &Value,
) -> Result<(ConversationPages, Vec<u8>)> {
    use Acknowledgement::{End, Next, None as Auto};
    let mut metadata = Vec::new();
    let mut content = Vec::new();
    for source in TEXT_SOURCES {
        let pages = dialogue
            .pages(source)
            .ok_or_else(|| invalid("missing progression text"))?;
        for (index, page) in pages.iter().enumerate() {
            let key = room_dialogue::page_key(source, index)?;
            metadata.push(json!({"text_source":source,"index":index,"boundary_source":page.boundary_source(),
                "acknowledgement":acknowledgement(page.acknowledgement()),"glyph_count":page.glyphs().len(),"sha256":sha256(page.indexed())}));
            content.extend(key.to_le_bytes());
            content.extend(page.width().to_le_bytes());
            content.extend(page.height().to_le_bytes());
            content.extend(page.indexed());
            content.extend(u32::try_from(page.glyphs().len())?.to_le_bytes());
            for glyph in page.glyphs() {
                content.extend(glyph.text_source.to_le_bytes());
                content.extend(glyph.font_source.to_le_bytes());
                content.extend(glyph.position.into_iter().flat_map(u16::to_le_bytes));
            }
        }
    }
    ensure(
        json!(metadata) == pins["pages"],
        "progression text page contract",
    )?;
    content.extend(serde_json::to_vec(&metadata)?); // Ordered boundary + acknowledgement + raster hashes.
    for catalog in 0..2 {
        let choice = dialogue
            .choice(catalog)
            .ok_or_else(|| invalid("missing progression choice"))?;
        ensure(choice.catalog == catalog, "progression choice catalog")?;
        content.push(catalog);
        for (index, option) in choice.options.iter().enumerate() {
            let result = u8::try_from(index + 1)?;
            ensure(
                option.result == result
                    && option.neighbors == [Some(3 - result), Some(3 - result), None, None],
                "progression choice graph",
            )?;
            content.extend(option.source.to_le_bytes());
            content.push(option.result);
            content.extend(option.position.into_iter().flat_map(u16::to_le_bytes));
            content.extend(option.neighbors.map(|v| v.unwrap_or(0)));
        }
    }
    // Shape checks prevent an auto-completing choice context becoming a page wait.
    let keys = |source, expected: &[Acknowledgement]| -> Result<Vec<u32>> {
        let pages = dialogue
            .pages(source)
            .ok_or_else(|| invalid("missing progression request"))?;
        ensure(
            pages
                .iter()
                .map(assets::text::DialoguePage::acknowledgement)
                .eq(expected.iter().copied()),
            "progression acknowledgement graph",
        )?;
        (0..pages.len())
            .map(|index| room_dialogue::page_key(source, index))
            .collect()
    };
    let first = keys(0x88_8ff0, &[Next, Auto])?;
    let repeat = keys(0x88_9156, &[Auto])?;
    let pages = ConversationPages {
        first: first[0],
        first_choice: first[1],
        repeat_choice: repeat[0],
        first_option1: keys(0x88_90d9, &[Next, Next, End])?
            .try_into()
            .map_err(|_| invalid("first option1 pages"))?,
        first_option2: keys(0x88_905a, &[Next, Next, End])?
            .try_into()
            .map_err(|_| invalid("first option2 pages"))?,
        repeat_option1: keys(0x88_918c, &[Next, End])?
            .try_into()
            .map_err(|_| invalid("repeat option1 pages"))?,
        repeat_option2: keys(0x88_91d6, &[Next, End])?
            .try_into()
            .map_err(|_| invalid("repeat option2 pages"))?,
    };
    // Bind the ordered mapping into core roles too, not just the decoded catalog.
    for key in [pages.first, pages.first_choice, pages.repeat_choice]
        .into_iter()
        .chain(pages.first_option1)
        .chain(pages.first_option2)
        .chain(pages.repeat_option1)
        .chain(pages.repeat_option2)
    {
        content.extend(key.to_le_bytes());
    }
    Ok((pages, content))
}

fn compile_exterior(rom: &rom::Rom, pins: &Value) -> Result<(Room, Vec<u8>)> {
    let mut content = Vec::new();
    authenticate_ranges(
        rom.image(),
        &pins["source_windows"],
        12,
        false,
        &mut content,
    )?;
    let exports = pins["export"]
        .as_array()
        .ok_or_else(|| invalid("missing exterior export contract"))?;
    ensure(exports.len() == 1, "exterior export count")?;
    let export = &exports[0];
    authenticate_ranges(rom.image(), &export["sources"], 6, false, &mut content)?;
    let bg = StaticBackground::from_rom(rom.image(), 10)?;
    let (width, height) = (
        u16::try_from(bg.layer().width())?,
        u16::try_from(bg.layer().height())?,
    );
    ensure(
        (width, height) == (64, 80)
            && export["map"] == 10
            && export["width_cells"] == width
            && export["height_cells"] == height,
        "exterior grid extent",
    )?;
    let attributes: &[u8; 512] = bg.resources()[3].decoded().try_into()?;
    let cells: Vec<_> = bg
        .layer()
        .attributed_cells(attributes)
        .iter()
        .map(|cell| cell.raw())
        .collect();
    let bytes: Vec<_> = cells.iter().flat_map(|cell| cell.to_le_bytes()).collect();
    ensure(
        export["files"]["static-grid"] == json!(sha256(&bytes)),
        "exterior grid hash",
    )?;
    // Source camera bounds are distinct from the 1024x1280 sheet dimensions.
    let record = rom
        .image()
        .get(0x16_be44..0x16_be46)
        .ok_or_else(|| invalid("truncated exterior camera"))?;
    let left = u16::from(record[0] & 15) * 256;
    let top = u16::from(record[1] & 15) * 256;
    let camera = [
        left,
        top,
        left + u16::from(record[0] >> 4) * 256,
        top + u16::from(record[1] >> 4) * 256,
    ];
    let contract = &pins["contract"];
    ensure(camera == [0,0,1024,1024] && contract["camera_bounds"] == json!(camera)
        && contract["schema"] == 1 && contract["map"] == 10 && contract["sheet_pixels"] == json!([1024,1280])
        && contract["camera_clamp_height"] == 256 && contract["sample_window_cells"] == json!([29,47,36,53])
        && contract["global_event_bits"] == json!([32,38,251]) && contract["passive_action_mask_clear"] == 80
        && contract["policy"] == "base first BG only; bounded sample window, not whole-map actors/material admission", "exterior admission contract")?;
    content.extend(10_u16.to_le_bytes());
    content.extend(width.to_le_bytes());
    content.extend(height.to_le_bytes());
    content.extend(bytes); // Full ROM grid, not the admitted window or native differences.
    content.extend(serde_json::to_vec(contract)?);
    Ok((Room::new_passive(width, height, cells)?, content))
}

#[cfg(test)]
mod tests {
    use super::*;
    use room_core::slice::{GameState, Policy};

    fn owned_rom() -> Option<rom::Rom> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        if !path.try_exists().unwrap() {
            eprintln!("SKIP: local Japanese ROM absent");
            return None;
        }
        Some(rom::Rom::load(&std::fs::read(path).unwrap()).unwrap())
    }

    #[test]
    fn aggregate_binds_base_startup_and_every_ordered_section() {
        let base = DataIdentity {
            rom_sha256: [1; 32],
            content_sha256: [2; 32],
        };
        let sections: [&[u8]; 3] = [b"source", b"pages", b"exterior"];
        let identity = aggregate(base, sections);
        assert_eq!(identity.rom_sha256, base.rom_sha256);
        let mut changed_base = base;
        changed_base.content_sha256[0] ^= 1;
        assert_ne!(identity, aggregate(changed_base, sections));
        changed_base = base;
        changed_base.rom_sha256[0] ^= 1;
        assert_ne!(identity, aggregate(changed_base, sections));
        for index in 0..3 {
            let mut changed = sections;
            changed[index] = b"changed";
            assert_ne!(identity, aggregate(base, changed));
        }
        assert_ne!(
            identity,
            aggregate(base, [sections[1], sections[0], sections[2]])
        );
    }

    #[test]
    fn ranges_reject_bad_addresses_hashes_and_truncation() {
        let pins = json!([{"start":0x80_0001,"end":0x80_0003,"sha256":sha256(&[2,3])}]);
        let mut content = Vec::new();
        authenticate_ranges(&[1, 2, 3, 4], &pins, 1, true, &mut content).unwrap();
        assert_eq!(content, [1, 0, 0, 0, 3, 0, 0, 0, 2, 3]);
        for (key, value) in [
            ("start", json!(-1)),
            ("end", json!(0x80_0001)),
            ("start", json!(0xc0_0001)),
            ("sha256", Value::Null),
        ] {
            let mut changed = pins.clone();
            changed[0][key] = value;
            assert!(
                authenticate_ranges(&[1, 2, 3, 4], &changed, 1, true, &mut Vec::new()).is_err()
            );
        }
        assert!(authenticate_ranges(&[1, 2], &pins, 1, true, &mut Vec::new()).is_err());
        assert!(authenticate_ranges(&[1, 2, 0, 4], &pins, 1, true, &mut Vec::new()).is_err());
        assert!(authenticate_ranges(&[1, 2, 3, 4], &json!([]), 1, true, &mut Vec::new()).is_err());
        assert!(authenticate_ranges(&[1, 2, 3, 4], &pins, 1, false, &mut Vec::new()).is_err());
    }

    #[test]
    fn semantic_windows_and_metadata_are_checked_without_whole_rom_shortcut() {
        let Some(rom) = owned_rom() else { return };
        let source: Value = serde_json::from_str(SOURCE).unwrap();
        let exterior: Value = serde_json::from_str(EXTERIOR).unwrap();
        for (pins, banked) in [
            (&source["ranges"], true),
            (&exterior["source_windows"], false),
            (&exterior["export"][0]["sources"], false),
        ] {
            let pins = pins.as_array().unwrap();
            for pin in pins {
                let at = usize::try_from(pin["start"].as_u64().unwrap()).unwrap() & 0x3f_ffff;
                let mut image = rom.image().to_vec();
                image[at] ^= 1;
                assert!(authenticate_ranges(
                    &image,
                    &json!(pins),
                    pins.len(),
                    banked,
                    &mut Vec::new()
                )
                .is_err());
            }
        }
        for key in ["resident", "first", "repeat", "gate", "exit"] {
            for field in source[key].as_object().unwrap().keys() {
                let mut changed = source.clone();
                changed[key][field] = Value::Null;
                assert!(
                    authenticate_conversation(rom.image(), &changed, &mut Vec::new()).is_err(),
                    "{key}.{field}"
                );
            }
        }
    }

    #[test]
    fn dialogue_and_exterior_are_rom_derived_not_reference_initializers() {
        let Some(rom) = owned_rom() else { return };
        let dialogue = HouseDialogue::from_rom(rom.image()).unwrap();
        let text: Value = serde_json::from_str(TEXT).unwrap();
        let (pages, _) = compile_dialogue(&dialogue, &text).unwrap();
        assert_eq!(room_dialogue::key(pages.first), "text:888ff0:0");
        assert_eq!(room_dialogue::key(pages.first_choice), "text:888ff0:1");
        assert_eq!(room_dialogue::key(pages.repeat_choice), "text:889156:0");
        for index in 0..14 {
            for field in [
                "text_source",
                "index",
                "boundary_source",
                "acknowledgement",
                "glyph_count",
                "sha256",
            ] {
                let mut changed = text.clone();
                changed["pages"][index][field] = Value::Null;
                assert!(
                    compile_dialogue(&dialogue, &changed).is_err(),
                    "{index}.{field}"
                );
            }
        }
        let exterior: Value = serde_json::from_str(EXTERIOR).unwrap();
        let (grid, identity) = compile_exterior(&rom, &exterior).unwrap();
        assert_eq!(
            (grid.width(), grid.height(), grid.cells().len()),
            (64, 80, 5120)
        );
        let mut changed = exterior.clone();
        // Native checkpoints and exported graphics hashes are not inputs to collision.
        changed["native"] = Value::Null;
        changed["export"][0]["files"]["rgb"] = Value::Null;
        let (same_grid, same_identity) = compile_exterior(&rom, &changed).unwrap();
        assert_eq!(grid.cells(), same_grid.cells());
        assert_eq!(identity, same_identity);
        for field in [
            "camera_bounds",
            "camera_clamp_height",
            "sample_window_cells",
            "sheet_pixels",
            "policy",
            "global_event_bits",
            "passive_action_mask_clear",
        ] {
            let mut changed = exterior.clone();
            changed["contract"][field] = Value::Null;
            assert!(compile_exterior(&rom, &changed).is_err(), "{field}");
        }
        let mut changed = exterior.clone();
        changed["export"][0]["files"]["static-grid"] = Value::Null;
        assert!(compile_exterior(&rom, &changed).is_err());
    }

    #[test]
    fn exact_1701_tick_core_route_uses_compiled_data_and_restores_every_action() {
        use room_core::conversation::DialogueWait;
        use room_core::{
            slice::Phase,
            Direction::{Down, Left, Right, Up},
            FrameInput,
        };
        let Some(rom) = owned_rom() else { return };
        let data = compile(&rom).unwrap();
        let mut state = GameState::new_game(&data, Policy::SemanticPreview);
        // Input-only core-progression route; no native checkpoint initializes state.
        #[rustfmt::skip]
        let route = [
            (2, 62), (0, 38), (4, 67), (0, 35), (4, 42), (1, 78),
            (0, 90), (1, 12), (0, 100), (1, 55), (3, 70), (0, 100),
            (5, 1), (3, 13), (0, 35), (3, 45), (0, 60), (5, 1),
            (2, 60), (6, 1), (8, 1), (6, 1), (6, 1), (6, 1),
            (5, 1), (7, 1), (6, 1), (6, 1), (4, 60), (0, 100),
            (1, 10), (4, 82), (0, 100), (4, 70), (0, 100), (4, 32),
            (0, 60), (2, 24), (0, 90)
        ];
        let action = |state: &mut GameState, command| match command {
            5 => state.interact(&data),
            6 => state.acknowledge(&data),
            7..=9 => state.choose(&data, command - 7),
            _ => state.step(
                &data,
                FrameInput {
                    direction: match command {
                        1 => Some(Left),
                        2 => Some(Right),
                        3 => Some(Up),
                        4 => Some(Down),
                        _ => None,
                    },
                },
            ),
        };
        for (index, (command, count)) in route.into_iter().enumerate() {
            for _ in 0..count {
                let mut restored = GameState::restore(&data, &state.snapshot()).unwrap();
                assert_eq!(
                    action(&mut state, command).unwrap(),
                    action(&mut restored, command).unwrap()
                );
                assert_eq!(state.snapshot(), restored.snapshot());
            }
            assert_eq!(state.event_flags().contains(0x26).unwrap(), index >= 19);
            if index == 18 {
                assert_eq!(
                    state.dialogue(&data).unwrap().unwrap().wait,
                    DialogueWait::Page(room_dialogue::page_key(0x88_8ff0, 0).unwrap())
                );
            }
            if index == 19 {
                assert_eq!(
                    state.dialogue(&data).unwrap().unwrap().wait,
                    DialogueWait::Choice {
                        catalog: 0,
                        key: room_dialogue::page_key(0x88_8ff0, 1).unwrap()
                    }
                );
            }
            if index == 32 {
                assert_eq!(state.current_room(&data).unwrap().cells()[1415], 0x592);
            }
        }
        let output = state.output();
        assert_eq!(
            (output.tick, output.map_id, output.position, output.phase),
            (1701, 10, (538, 815), Phase::Walking)
        );
        assert!(state.dialogue(&data).unwrap().is_none());
    }

    #[test]
    fn rom_compiler_extends_house_and_binds_a_distinct_repeatable_identity() {
        let Some(rom) = owned_rom() else { return };
        let base = house_navigation::compile(&rom).unwrap();
        let data = compile(&rom).unwrap();
        assert!(data.conversation_progression());
        assert!(data.door_interaction());
        let snapshot = GameState::new_game(&data, Policy::SemanticPreview).snapshot();
        assert_eq!(
            snapshot,
            GameState::new_game(&compile(&rom).unwrap(), Policy::SemanticPreview).snapshot()
        );
        assert!(GameState::restore(&base, &snapshot).is_err());
        let base_snapshot = GameState::new_game(&base, Policy::SemanticPreview).snapshot();
        assert_eq!(&snapshot[8..40], &base_snapshot[8..40]);
        assert_ne!(&snapshot[40..72], &base_snapshot[40..72]);
    }
}
