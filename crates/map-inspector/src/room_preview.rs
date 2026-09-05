//! Authenticated ROM-to-preview adapter. No oracle session is constructed here.
use crate::{invalid, sha256, Result};
use assets::maps::{exits::ExitList, visual::StaticBackground};
use rom::Rom;
use room_core::{
    slice::{DataIdentity, Exit, GameData, GameState, Phase, Policy},
    Direction, FrameInput, Room,
};
use serde_json::{json, Value};

pub(super) struct Preview {
    data: GameData,
    state: GameState,
    bitmap: Vec<u8>,
    error: Option<String>,
}
impl Preview {
    pub(super) fn new(rom: &Rom) -> Result<Self> {
        let data = compile(rom)?;
        let state = GameState::new(&data, Policy::SemanticPreview);
        let viewer = crate::visual_export::export(rom, 15)?;
        let bitmap = std::fs::read(viewer.with_file_name("map.bmp"))?;
        Ok(Self {
            data,
            state,
            bitmap,
            error: None,
        })
    }
    pub(super) fn bitmap(&self) -> &[u8] {
        &self.bitmap
    }
    pub(super) fn step(&mut self, button: u8) {
        if self.error.is_some() {
            return;
        }
        let direction = match button {
            0 => None,
            1 => Some(Direction::Left),
            2 => Some(Direction::Right),
            3 => Some(Direction::Up),
            4 => Some(Direction::Down),
            _ => {
                self.error = Some("unsupported input".into());
                return;
            }
        };
        if let Err(error) = self.state.step(&self.data, FrameInput { direction }) {
            self.error = Some(error.to_string());
        }
    }
    pub(super) fn reset(&mut self) {
        self.state = GameState::new(&self.data, Policy::SemanticPreview);
        self.error = None;
    }
    pub(super) fn state(&self) -> Value {
        let output = self.state.output();
        json!({"schema_version":1,"policy":"semantic-preview","map_id":output.map_id,
            "x":output.position.0,"y":output.position.1,"tick":output.tick,
            "phase":match output.phase{Phase::Walking=>"walking",Phase::Departing=>"departing",Phase::Arriving=>"arriving"},
            "camera":[256,if output.map_id==15{0}else{256}],"error":self.error,
            "snapshot_sha256":sha256(&self.state.snapshot())})
    }
}

fn compile_room(rom: &Rom, id: u16) -> Result<Room> {
    let background = StaticBackground::from_rom(rom.image(), id)?;
    let attributes: &[u8; 512] = background.resources()[3].decoded().try_into()?;
    let mut cells: Vec<_> = background
        .layer()
        .attributed_cells(attributes)
        .iter()
        .map(|c| c.raw())
        .collect();
    // These cells are changed by runtime state at the authenticated checkpoints.
    // Mark them outside the static simulation domain, not as emulated event effects.
    // The walking core rejects bit 15 rather than treating it as a generic wall.
    let excluded: &[usize] = if id == 15 {
        &[317]
    } else {
        &[731, 732, 826, 827]
    };
    for &index in excluded {
        cells[index] |= 0x8000;
    }
    Ok(Room::new(32, 64, cells)?)
}

fn compile(rom: &Rom) -> Result<GameData> {
    let rooms = [compile_room(rom, 15)?, compile_room(rom, 16)?];
    let lists = [
        ExitList::from_rom(rom.image(), 15)?,
        ExitList::from_rom(rom.image(), 16)?,
    ];
    let exits = lists.each_ref().map(|list| {
        list.records()
            .iter()
            .map(|record| Exit(*record.bytes()))
            .collect()
    });
    let record = lists[0]
        .select(384, 193)
        .ok_or_else(|| invalid("missing semantic doorway"))?;
    let word = |at| u16::from_le_bytes([rom.image()[at], rom.image()[at + 1]]);
    let pointer =
        |at| u32::from_le_bytes([rom.image()[at], rom.image()[at + 1], rom.image()[at + 2], 0]);
    let raw = record.destination_position();
    // Selector 5's signed adjustment and the player FD/header anchor override.
    let adjustment = (
        i16::from_le_bytes(word(0x0d_8985 + 5 * 4).to_le_bytes()),
        i16::from_le_bytes(word(0x0d_8987 + 5 * 4).to_le_bytes()),
    );
    let queue = (
        raw.0.wrapping_add_signed(adjustment.0),
        raw.1.wrapping_add_signed(adjustment.1),
    );
    let spawn = (queue.0.checked_add(8), queue.1.checked_add(16));
    if record.direct_destination() != Ok(16)
        || record.transition_mode() != 0
        || record.selector() != 5
        || adjustment != (0, -16)
        || queue != (384, 320)
        || spawn != (Some(392), Some(336))
        || pointer(0x0d_895b + 4 * 3) != 0x84_b94d
        || word(0x04_8808) != 0xbb3b
        || word(0x03_8020) != 0x8d69
        || rom.image()[0x03_8d6b] != 0xfd
        || pointer(0x03_8d6f) != 0x84_a129
        || word(0x04_a12a) & 0x0400 == 0
    {
        return Err(invalid("unqualified departure/spawn/arrival profile").into());
    }
    let mut content = Vec::new();
    for room in &rooms {
        content.extend(room.width().to_le_bytes());
        content.extend(room.height().to_le_bytes());
        for &cell in room.cells() {
            content.extend(cell.to_le_bytes());
        }
    }
    for list in &lists {
        content.extend(list.source_bytes());
    }
    content.extend([1, room_core::slice::PROFILE_VERSION, 0, 1]); // slice/profile/no-RNG/policy versions
    content.extend(queue.0.to_le_bytes());
    content.extend(queue.1.to_le_bytes());
    let identity = DataIdentity {
        rom_sha256: rom::digests(rom.image()).sha256,
        content_sha256: rom::digests(&content).sha256,
    };
    Ok(GameData::new(rooms, exits, identity)?)
}

pub(super) fn verify(rom: &Rom) -> Result<Value> {
    let mut preview = Preview::new(rom)?;
    let initial = preview.state();
    for _ in 0..56 {
        preview.step(1);
    }
    for _ in 0..24 {
        preview.step(4);
    }
    let handoff = preview.state();
    if handoff["x"] != 392 || handoff["y"] != 209 || handoff["phase"] != "departing" {
        return Err(invalid("semantic approach differs").into());
    }
    for _ in 0..17 {
        preview.step(0);
    }
    let departure = preview.state();
    preview.step(0);
    let spawn = preview.state();
    for _ in 0..17 {
        preview.step(0);
    }
    let arrival = preview.state();
    if arrival["map_id"] != 16
        || arrival["x"] != 392
        || arrival["y"] != 353
        || arrival["phase"] != "walking"
    {
        return Err(invalid("semantic arrival differs").into());
    }
    let snapshot = preview.state.snapshot();
    let restored = GameState::restore(&preview.data, &snapshot)?;
    if restored != preview.state {
        return Err(invalid("preview snapshot roundtrip differs").into());
    }
    Ok(
        json!({"kind":"cpu-free-semantic-room-preview","initial":initial,"handoff":handoff,"departure":departure,"spawn":spawn,"arrival":arrival,
        "limits":"Walking frames reference-qualified on bounded flat paths. Doorway is opt-in endpoint-qualified 17/load/17 logical policy, NOT native scheduling or video-frame fidelity. Marker only; no actors, sprites, combat, audio or events."}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rom_compiled_grids_equal_the_authenticated_walking_fixture_grids() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        if !path.try_exists().unwrap() {
            eprintln!("skipping: local Japanese ROM absent");
            return;
        }
        let rom = Rom::load(&std::fs::read(path).unwrap()).unwrap();
        for (id, hash) in [
            (
                15,
                "c5d86aec915b09ec3481d48e903bd1d94a824303f4b4eee24da19acfbf8028e1",
            ),
            (
                16,
                "261e3b4637587b69465178667f70cddb5eb6d996650f7008c37aefbec5325eed",
            ),
        ] {
            let grid = compile_room(&rom, id).unwrap();
            let bytes: Vec<_> = grid
                .cells()
                .iter()
                .flat_map(|cell| cell.to_le_bytes())
                .collect();
            assert_eq!(
                sha256(&bytes),
                hash,
                "map {id:X} must equal the grids used by all1971 portable reference steps"
            );
        }
    }
}
