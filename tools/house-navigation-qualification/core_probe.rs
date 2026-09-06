//! CPU-free qualification adapter over sibling-qualified SOURCE profiles.
//! No oracle session, native capture, arbitrary checkpoint or mutable grid API.
use assets::maps::{exits::ExitList, visual::StaticBackground};
use room_core::{
    slice::{DataIdentity, GameData, GameState, HouseRoom, NewGameData, Policy},
    Direction, FrameInput, Room,
};
use serde_json::{json, Value};
#[path = "../../crates/map-inspector/src/new_game.rs"]
mod new_game;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn invalid(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message)
}
fn sha(bytes: &[u8]) -> String {
    rom::digests(bytes)
        .sha256
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn compile(rom: &rom::Rom, contract: &[u8]) -> Result<GameData> {
    let metadata: Value = serde_json::from_slice(contract)?;
    assert_eq!(metadata["rom_sha256"], sha(rom.image()));
    assert_eq!(metadata["global_event_bits"], json!([32, 251]));
    assert_eq!(metadata["passive_action_mask_clear"], 80);
    let profiles = metadata["profiles"].as_array().unwrap();
    let mut content = contract.to_vec();
    let mut room = |map: u16, fresh: bool| -> Result<Room> {
        let bg = StaticBackground::from_rom(rom.image(), map)?;
        let attributes = bg.resources()[3].decoded().try_into()?;
        let mut cells: Vec<_> = bg
            .layer()
            .attributed_cells(attributes)
            .into_iter()
            .map(|c| c.raw())
            .collect();
        let bytes: Vec<_> = cells.iter().flat_map(|c| c.to_le_bytes()).collect();
        assert_eq!(sha(&bytes), metadata["base_grid_sha256"]);
        let history = if fresh {
            "post-intro-first-load"
        } else {
            "ordinary-load"
        };
        let candidates: Vec<_> = profiles
            .iter()
            .filter(|p| p["map"] == map && p["history"] == history)
            .collect();
        assert_eq!(candidates.len(), 1);
        let profile = candidates[0];
        for index in profile["added_bit15_cells"].as_array().unwrap() {
            cells[index.as_u64().unwrap() as usize] |= 0x8000;
        }
        let bytes: Vec<_> = cells.iter().flat_map(|c| c.to_le_bytes()).collect();
        assert_eq!(sha(&bytes), profile["grid_sha256"]);
        content.extend(map.to_le_bytes());
        content.push(u8::from(fresh));
        content.extend(bytes);
        Ok(Room::new_passive(32, 64, cells)?)
    };
    let mut rooms = Vec::new();
    for map_id in [11, 12, 13, 15, 16, 17] {
        rooms.push(HouseRoom {
            map_id,
            collision: room(map_id, false)?,
            exits: ExitList::from_rom(rom.image(), map_id)?
                .records()
                .iter()
                .map(|r| room_core::slice::Exit(*r.bytes()))
                .collect(),
        });
    }
    let bedroom = room(15, true)?;
    for profile in &rooms {
        for exit in &profile.exits {
            content.extend(exit.0);
        }
    }
    let startup = new_game::compile(rom)?;
    assert_eq!(startup.position, (304, 112));
    content.extend(startup.events);
    content.extend(startup.position.0.to_le_bytes());
    content.extend(startup.position.1.to_le_bytes());
    content
        .extend(b"house-navigation:passive,frozen-source-occupancy,17-load-17,atomic-final-door");
    content.push(room_core::slice::PROFILE_VERSION);
    let identity = DataIdentity {
        rom_sha256: rom::digests(rom.image()).sha256,
        content_sha256: rom::digests(&content).sha256,
    };
    Ok(GameData::new_house(
        rooms.try_into().unwrap(),
        identity,
        NewGameData {
            bedroom,
            position: startup.position,
        },
    )?)
}
fn apply(state: &mut GameState, data: &GameData, button: u64) -> Result<()> {
    if button == 5 {
        state.interact(data)?;
    } else {
        let direction = match button {
            0 => None,
            1 => Some(Direction::Left),
            2 => Some(Direction::Right),
            3 => Some(Direction::Up),
            4 => Some(Direction::Down),
            _ => panic!("unsupported action"),
        };
        state.step(data, FrameInput { direction })?;
    }
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(
        args.len(),
        4,
        "core-probe ROM SOURCE-profiles.json CORE-route.jsonl"
    );
    let rom = rom::Rom::load(&std::fs::read(&args[1])?)?;
    let data = compile(&rom, &std::fs::read(&args[2])?)?;
    let mut state = GameState::new_game(&data, Policy::SemanticPreview);
    let route = std::fs::read_to_string(&args[3])?;
    let commands: Vec<Value> = route
        .lines()
        .map(serde_json::from_str)
        .collect::<std::result::Result<_, _>>()?;
    assert_eq!(commands.last().unwrap()["finish"], true);
    for (index, command) in commands.iter().enumerate() {
        if command["finish"] == true {
            assert_eq!(index + 1, commands.len());
            break;
        }
        let button = command["button"].as_u64().unwrap();
        let count = command["steps"].as_u64().unwrap();
        assert!((1..=2000).contains(&count));
        assert!(button != 5 || count == 1);
        for _ in 0..count {
            let mut restored = GameState::restore(&data, &state.snapshot())?;
            apply(&mut state, &data, button)?;
            apply(&mut restored, &data, button)?;
            assert_eq!(state, restored);
        }
        let output = state.output();
        let checkpoint = json!([
            output.map_id,
            output.position.0,
            output.position.1,
            format!("{:?}", output.phase),
            state.wooden_door_open()
        ]);
        println!(
            "{}",
            json!({"label":command["label"],"tick":output.tick,"state":checkpoint})
        );
        if let Some(expected) = command.get("expect") {
            assert_eq!(&checkpoint, expected, "{}", command["label"]);
        }
    }
    Ok(())
}
