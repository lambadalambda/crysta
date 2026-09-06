//! Bounded ROM compiler for six source-qualified house rooms, not an event VM.
use crate::{house_profiles, invalid, sha256, Result};
use assets::maps::{exits::ExitList, visual::StaticBackground};
use room_core::{
    slice::{DataIdentity, Exit, GameData, HouseRoom, NewGameData},
    Room,
};
use serde_json::{json, Value};

const NAVIGATION: &str =
    include_str!("../../../tools/house-navigation-qualification/reference.json");
const BACKGROUND: &str =
    include_str!("../../../tools/house-background-qualification/reference.json");
const PROFILES: &str = include_str!("../../../tools/house-background-qualification/profiles.json");
// Changing admission, occupancy, transition or action semantics changes snapshot identity.
const POLICY: &[u8] = b"house-navigation-v1:passive,no-rng,frozen-source-occupancy,17-load-17,atomic-final-door,unsupported-interaction-noop";

#[cfg(test)]
pub(super) fn compile(rom: &rom::Rom) -> Result<GameData> {
    Ok(compile_with_identity(rom)?.0)
}

/// Preserve the authenticated house/startup identity when composing extensions.
pub(super) fn compile_with_identity(rom: &rom::Rom) -> Result<(GameData, DataIdentity)> {
    let navigation: Value = serde_json::from_str(NAVIGATION)?;
    let background: Value = serde_json::from_str(BACKGROUND)?;
    let profiles: Value = serde_json::from_str(PROFILES)?;
    for pins in [&navigation, &background, &profiles] {
        ensure(
            json!(sha256(rom.image())) == pins["rom_sha256"],
            "house ROM identity",
        )?;
    }
    let mut content = POLICY.to_vec();
    content.extend([1, room_core::slice::PROFILE_VERSION]); // schema and core profile
    authenticate_sources(rom.image(), &navigation, &background, &mut content)?;
    qualify_door(rom, &navigation, &mut content)?;
    let graph = array(&navigation["graph"])?;
    ensure(
        graph.len() == house_profiles::MAPS.len(),
        "house graph extent",
    )?;
    let mut rooms = Vec::new();
    for (&map_id, pin) in house_profiles::MAPS.iter().zip(graph) {
        ensure(pin["map"] == map_id, "house graph source order")?;
        let collision = compile_room(rom, map_id, false, &profiles, &mut content)?;
        let list = ExitList::from_rom(rom.image(), map_id)?;
        let range = list
            .source_range()
            .ok_or_else(|| invalid("missing house exits"))?;
        ensure(
            json!([range.start, range.end]) == pin["extent"],
            "house exit source extent",
        )?;
        // Retain the complete ordered list, including A/E/122 unsupported boundaries.
        content.extend(list.source_bytes());
        rooms.push(HouseRoom {
            map_id,
            collision,
            exits: list.records().iter().map(|r| Exit(*r.bytes())).collect(),
        });
    }
    let bedroom = compile_room(rom, 15, true, &profiles, &mut content)?;
    let startup = crate::new_game::compile(rom)?;
    content.extend(startup.events);
    content.extend(startup.position.0.to_le_bytes());
    content.extend(startup.position.1.to_le_bytes());
    let identity = DataIdentity {
        rom_sha256: rom::digests(rom.image()).sha256,
        content_sha256: rom::digests(&content).sha256,
    };
    let data = GameData::new_house(
        rooms.try_into().map_err(|_| invalid("house room count"))?,
        identity,
        NewGameData {
            bedroom,
            position: startup.position,
        },
    )?;
    Ok((data, identity))
}

fn compile_room(
    rom: &rom::Rom,
    map: u16,
    fresh: bool,
    pins: &Value,
    content: &mut Vec<u8>,
) -> Result<Room> {
    let grid = house_profiles::compile(rom, map, fresh)?;
    let camera = house_profiles::camera(rom.image(), map)?;
    let history = if fresh {
        "post-intro-first-load"
    } else {
        "ordinary-load"
    };
    let matching: Vec<_> = array(&pins["profiles"])?
        .iter()
        .filter(|p| p["map"] == map && p["history"] == history)
        .collect();
    let [pin] = matching.as_slice() else {
        return Err(invalid("house profile uniqueness").into());
    };
    let bytes: Vec<_> = grid.cells().iter().flat_map(|c| c.to_le_bytes()).collect();
    ensure(
        json!(sha256(&bytes)) == pin["grid_sha256"] && json!(camera) == pin["camera"],
        "house compiled grid/camera differs",
    )?;
    content.extend(map.to_le_bytes());
    content.push(u8::from(fresh));
    content.extend(grid.width().to_le_bytes());
    content.extend(grid.height().to_le_bytes());
    content.extend(camera.into_iter().flat_map(u16::to_le_bytes));
    content.extend(bytes);
    Ok(grid)
}

fn ensure(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(invalid(message).into())
    }
}
fn array(value: &Value) -> Result<&Vec<Value>> {
    value
        .as_array()
        .ok_or_else(|| invalid("house source array missing").into())
}
fn offset(value: &Value) -> Result<usize> {
    Ok(usize::try_from(
        value
            .as_u64()
            .ok_or_else(|| invalid("house source offset"))?,
    )?)
}
fn pin_range(
    image: &[u8],
    start: &Value,
    end: &Value,
    hash: &Value,
    content: &mut Vec<u8>,
) -> Result<()> {
    let (start, end) = (offset(start)?, offset(end)?);
    ensure(start < end, "house source extent")?;
    let bytes = image
        .get(start..end)
        .ok_or_else(|| invalid("truncated house source"))?;
    ensure(json!(sha256(bytes)) == *hash, "house source hash mismatch")?;
    content.extend(u32::try_from(start)?.to_le_bytes());
    content.extend(u32::try_from(end)?.to_le_bytes());
    content.extend(bytes);
    Ok(())
}

/// Only source contracts are consumed; native checkpoints/captures never initialize data.
fn authenticate_sources(
    image: &[u8],
    navigation: &Value,
    background: &Value,
    content: &mut Vec<u8>,
) -> Result<()> {
    for key in ["ranges", "graph"] {
        for pin in array(&navigation[key])? {
            pin_range(
                image,
                &pin["extent"][0],
                &pin["extent"][1],
                &pin["sha256"],
                content,
            )?;
        }
    }
    // Background setup pins cover actor identities/conditions, stamp geometry,
    // D's hidden gate, fresh F residue and the map camera/header dispatch.
    // Vendor emulator-file pins belong to offline qualification, not this host.
    for pin in array(&background["sources"])? {
        if pin.get("path").is_none() {
            pin_range(image, &pin["start"], &pin["end"], &pin["sha256"], content)?;
        }
    }
    Ok(())
}

fn qualify_door(rom: &rom::Rom, navigation: &Value, content: &mut Vec<u8>) -> Result<()> {
    let background = StaticBackground::from_rom(rom.image(), 12)?;
    let pins = array(&navigation["source_door"]["resources"])?;
    ensure(pins.len() == 2, "house door resource count")?;
    for (index, pin) in [2, 3].into_iter().zip(pins) {
        let resource = &background.resources()[index];
        let range = resource.source_range();
        ensure(
            json!([range.start, range.end]) == pin["extent"]
                && json!(sha256(resource.decoded())) == pin["decoded_sha256"],
            "house door decoded resource",
        )?;
        pin_range(
            rom.image(),
            &pin["extent"][0],
            &pin["extent"][1],
            &pin["source_sha256"],
            content,
        )?;
        content.extend(resource.decoded());
    }
    // Core's final collision words must agree with COP44's source reconstruction.
    let attributes = background.resources()[3].decoded();
    ensure(
        attributes.get(0xf6) == Some(&14) && attributes.get(0xf7) == Some(&0),
        "house door final attributes",
    )
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
    fn source_pins_reject_mutation_truncation_bad_bounds_and_missing_hash() {
        let image = [1, 2, 3, 4];
        let hash = json!(sha256(&image[1..3]));
        let mut content = Vec::new();
        pin_range(&image, &json!(1), &json!(3), &hash, &mut content).unwrap();
        assert_eq!(content, [1, 0, 0, 0, 3, 0, 0, 0, 2, 3]);
        for (bytes, start, end, pin) in [
            (&[1, 2, 0, 4][..], json!(1), json!(3), hash.clone()),
            (&image[..2], json!(1), json!(3), hash.clone()),
            (&image[..], json!(3), json!(1), hash.clone()),
            (&image[..], json!(1), json!(1), hash.clone()),
            (&image[..], json!(-1), json!(3), hash.clone()),
            (&image[..], json!(1), json!(3), Value::Null),
        ] {
            let before = content.clone();
            assert!(pin_range(bytes, &start, &end, &pin, &mut content).is_err());
            assert_eq!(content, before);
        }
    }

    #[test]
    fn all_navigation_and_occupancy_source_windows_are_checked_independently() {
        let Some(rom) = owned_rom() else { return };
        let navigation: Value = serde_json::from_str(NAVIGATION).unwrap();
        let background: Value = serde_json::from_str(BACKGROUND).unwrap();
        let mut content = Vec::new();
        authenticate_sources(rom.image(), &navigation, &background, &mut content).unwrap();
        let mut offsets = Vec::new();
        for key in ["ranges", "graph"] {
            for pin in array(&navigation[key]).unwrap() {
                offsets.push(offset(&pin["extent"][0]).unwrap());
            }
        }
        for pin in array(&background["sources"]).unwrap() {
            if pin.get("path").is_none() {
                offsets.push(offset(&pin["start"]).unwrap());
            }
        }
        assert_eq!(offsets.len(), 62); // 13 navigation + 6 exits + 43 setup windows
        let mut image = rom.image().to_vec();
        for at in offsets {
            image[at] ^= 1;
            assert!(
                authenticate_sources(&image, &navigation, &background, &mut Vec::new()).is_err(),
                "source {at:X}"
            );
            image[at] ^= 1;
        }
        // These checks are independent of whole-ROM authentication and its digest.
        let mut altered = navigation.clone();
        altered["source_door"]["resources"][0]["decoded_sha256"] = json!("wrong");
        assert!(qualify_door(&rom, &altered, &mut Vec::new()).is_err());
        altered = navigation.clone();
        altered["source_door"]["resources"][1]["source_sha256"] = json!("wrong");
        assert!(qualify_door(&rom, &altered, &mut Vec::new()).is_err());
        qualify_door(&rom, &navigation, &mut content).unwrap();
    }

    #[test]
    fn compiler_authenticates_image_not_a_caller_supplied_revision_label() {
        let Some(rom) = owned_rom() else { return };
        let mut image = rom.image().to_vec();
        image[0x7_9819] ^= 1; // final door COP44
        let digests = rom::digests(&image);
        let changed = rom::Rom::load_with_known(
            &image,
            &[rom::KnownRom {
                revision: rom::Revision::Japan,
                sha256: digests.sha256,
                crc32: digests.crc32,
            }],
        )
        .unwrap();
        assert!(compile(&changed).is_err());
    }

    #[test]
    fn compiled_recipe_identity_is_repeatable_and_profiles_are_not_initializers() {
        let Some(rom) = owned_rom() else { return };
        let data = compile(&rom).unwrap();
        assert!(data.door_interaction());
        let snapshot = GameState::new_game(&data, Policy::SemanticPreview).snapshot();
        let repeated = compile(&rom).unwrap();
        assert_eq!(
            snapshot,
            GameState::new_game(&repeated, Policy::SemanticPreview).snapshot()
        );
        assert_eq!(&snapshot[8..40], rom::Revision::Japan.sha256());
        assert_eq!(
            snapshot[40..72],
            [
                0xc5, 0x95, 0xae, 0x53, 0x9d, 0x06, 0x9b, 0xec, 0x93, 0xbe, 0x9a, 0x9f, 0xfc, 0x01,
                0xe3, 0xb5, 0x0f, 0x00, 0x51, 0xc4, 0xfc, 0xba, 0x2c, 0x99, 0x54, 0x47, 0x12, 0x9e,
                0x39, 0x93, 0x69, 0x30
            ]
        );
        let mut wrong_identity = snapshot.clone();
        wrong_identity[40] ^= 1;
        assert!(GameState::restore(&data, &wrong_identity).is_err());

        let profiles: Value = serde_json::from_str(PROFILES).unwrap();
        for index in 0..7 {
            let pin = &profiles["profiles"][index];
            let map = u16::try_from(pin["map"].as_u64().unwrap()).unwrap();
            let fresh = pin["history"] == "post-intro-first-load";
            for key in ["grid_sha256", "camera"] {
                let mut altered = profiles.clone();
                altered["profiles"][index][key] = Value::Null;
                assert!(compile_room(&rom, map, fresh, &altered, &mut Vec::new()).is_err());
            }
        }
    }
}
