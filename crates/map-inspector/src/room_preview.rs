//! Authenticated ROM-to-preview adapter. No oracle session is constructed here.
use crate::{invalid, sha256, Result};
use rom::Rom;
use room_core::{
    slice::{GameData, GameState, Phase, Policy},
    Direction, FrameInput,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub(super) struct Preview {
    data: GameData,
    state: GameState,
    bitmap: Vec<u8>,
    art: crate::room_art::Art,
    cameras: BTreeMap<u16, [u16; 2]>,
    error: Option<String>,
    fresh_start: bool,
}
impl Preview {
    pub(super) fn new(rom: &Rom) -> Result<Self> {
        let data = compile(rom)?;
        let state = GameState::new(&data, Policy::SemanticPreview);
        let art = crate::room_art::compile(rom)?;
        let cameras = crate::house_profiles::MAPS
            .into_iter()
            .map(|id| crate::house_profiles::camera(rom.image(), id).map(|camera| (id, camera)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        let viewer = crate::visual_export::export(rom, 15)?;
        let bitmap = std::fs::read(viewer.with_file_name("map.bmp"))?;
        Ok(Self {
            data,
            state,
            bitmap,
            art,
            cameras,
            error: None,
            fresh_start: false,
        })
    }
    pub(super) fn art(&self) -> &[u8] {
        &self.art.bytes
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
    pub(super) fn new_game(&mut self) {
        self.state = GameState::new_game(&self.data, Policy::SemanticPreview);
        self.fresh_start = true;
        self.error = None;
    }
    pub(super) fn reset(&mut self) {
        self.state = GameState::new(&self.data, Policy::SemanticPreview);
        self.fresh_start = false;
        self.error = None;
    }
    pub(super) fn state(&self) -> Value {
        let output = self.state.output();
        let actor_key = crate::room_art::frame_key(output.animation);
        json!({"schema_version":1,"policy":"semantic-preview","map_id":output.map_id,
            "start_kind":if self.fresh_start { "new-game" } else { "saved-checkpoint" },
            "actor_key":actor_key,
            "scene":self.art.scene(output.map_id, &actor_key, output.position),
            "animation":{"set":match output.animation.set {
                room_core::AnimationSet::Standing=>"standing",room_core::AnimationSet::Walking=>"walking"},
                "sequence":output.animation.sequence,"record":output.animation.record,"mirror_x":output.animation.mirror_x},
            "x":output.position.0,"y":output.position.1,"tick":output.tick,
            "phase":match output.phase{Phase::Walking=>"walking",Phase::Departing=>"departing",Phase::Arriving=>"arriving"},
            "camera":self.cameras[&output.map_id],"error":self.error,
            "snapshot_sha256":sha256(&self.state.snapshot())})
    }
}

#[cfg(test)]
fn compile_room(rom: &Rom, id: u16, fresh: bool) -> Result<room_core::Room> {
    crate::house_profiles::compile(rom, id, fresh)
}

fn compile(rom: &Rom) -> Result<GameData> {
    crate::house_navigation::compile(rom)
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
    for _ in 0..14 {
        preview.step(3);
    }
    let return_handoff = preview.state();
    if return_handoff["y"] != 336 || return_handoff["phase"] != "departing" {
        return Err(invalid("semantic return approach differs").into());
    }
    for _ in 0..35 {
        let mut restored = GameState::restore(&preview.data, &preview.state.snapshot())?;
        preview.step(0);
        restored.step(&preview.data, FrameInput::default())?;
        if restored != preview.state {
            return Err(invalid("return snapshot continuation differs").into());
        }
    }
    let returned = preview.state();
    if returned["map_id"] != 15 || returned["y"] != 191 || returned["phase"] != "walking" {
        return Err(invalid("semantic return endpoint differs").into());
    }
    Ok(
        json!({"kind":"cpu-free-semantic-room-preview","initial":initial,"handoff":handoff,"departure":departure,"spawn":spawn,"arrival":arrival,
        "return_handoff":return_handoff,"returned":returned,
        "limits":"Walking frames reference-qualified on bounded paths. Doorways are opt-in endpoint-qualified 17/load/17 logical policy, NOT native scheduling or video-frame fidelity. Ordinary Ark sprites, nine frozen fresh residents and a table with static house BG2 priority; six-room semantic endpoints, no native AI/dialogue, shadows, effects, combat, audio or events."}),
    )
}

/// The fresh reference route, with loader/arrival waits replaced by the named
/// endpoint policy. Every logical update also runs from its restored snapshot.
pub(super) fn verify_house(rom: &Rom) -> Result<Value> {
    fn advance(preview: &mut Preview, button: u8, count: usize) -> Result<Value> {
        for _ in 0..count {
            let before = preview.state.snapshot();
            preview.step(button);
            if let Some(error) = &preview.error {
                return Err(invalid(error).into());
            }
            let expected = preview.state.clone();
            preview.state = GameState::restore(&preview.data, &before)?;
            preview.step(button);
            if preview.state != expected || preview.error.is_some() {
                return Err(invalid("fresh house snapshot continuation differs").into());
            }
        }
        Ok(preview.state())
    }
    let mut preview = Preview::new(rom)?;
    preview.new_game();
    let initial = preview.state();
    advance(&mut preview, 2, 62)?;
    advance(&mut preview, 0, 38)?;
    let outbound_handoff = advance(&mut preview, 4, 67)?;
    let arrival = advance(&mut preview, 0, 35)?;
    advance(&mut preview, 0, 50)?;
    let return_handoff = advance(&mut preview, 3, 14)?;
    let returned = advance(&mut preview, 0, 35)?;
    advance(&mut preview, 0, 10)?;
    for button in [2, 1, 2, 1] {
        advance(&mut preview, button, 20)?;
        advance(&mut preview, 0, 30)?;
    }
    let revisited = preview.state();
    if initial["x"] != 304
        || initial["y"] != 112
        || outbound_handoff["y"] != 208
        || outbound_handoff["phase"] != "departing"
        || arrival["map_id"] != 16
        || arrival["y"] != 353
        || return_handoff["y"] != 336
        || return_handoff["phase"] != "departing"
        || revisited["map_id"] != 15
        || revisited["x"] != 392
        || revisited["y"] != 191
        || revisited["tick"] != 511
        || revisited["phase"] != "walking"
    {
        return Err(invalid("fresh house route differs").into());
    }
    Ok(
        json!({"kind":"cpu-free-semantic-new-game-house-route", "initial":initial,
        "outbound_handoff":outbound_handoff,"arrival":arrival,"return_handoff":return_handoff,
        "returned":returned,"revisited":revisited,
        "limits":"ROM-derived default-name start with explicit intro presentation omission.441 reference walking steps +70 semantic doorway updates; ordinary Ark sprites, nine frozen fresh residents and a table with static house BG2 priority; six-room semantic endpoints, no native loader timing, AI/dialogue, shadows, effects, combat or audio."}),
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
        for (id, fresh, hash) in [
            (
                15,
                false,
                "c5d86aec915b09ec3481d48e903bd1d94a824303f4b4eee24da19acfbf8028e1",
            ),
            (
                16,
                false,
                "261e3b4637587b69465178667f70cddb5eb6d996650f7008c37aefbec5325eed",
            ),
            (
                15,
                true,
                "a4c86ef52fc84b6d0d24c80c288df19ffebf19e12691deabc1d1dc9e1dc3b94f",
            ),
        ] {
            let grid = compile_room(&rom, id, fresh).unwrap();
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
        let mut preview = Preview::new(&rom).unwrap();
        for _ in 0..189 {
            preview.step(3);
        }
        assert_eq!(preview.state()["error"], Value::Null);
        assert_eq!(
            (preview.state()["x"].as_u64(), preview.state()["y"].as_u64()),
            (Some(472), Some(176))
        );
    }
    #[test]
    fn new_game_is_distinct_from_checkpoint_and_clears_latched_errors() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        if !path.try_exists().unwrap() {
            eprintln!("skipping: local Japanese ROM absent");
            return;
        }
        let rom = Rom::load(&std::fs::read(path).unwrap()).unwrap();
        let mut preview = Preview::new(&rom).unwrap();
        let checkpoint = preview.state();
        let art = preview.art().to_vec();
        assert_eq!(checkpoint["actor_key"], "0:0");
        assert_eq!(
            checkpoint["animation"],
            json!({"set":"standing", "sequence":0, "record":0, "mirror_x":false})
        );
        preview.step(255);
        assert!(preview.state()["error"].is_string());
        preview.new_game();
        let fresh = preview.state();
        assert_eq!(fresh["start_kind"], "new-game");
        assert_eq!(fresh["map_id"], 15);
        assert_eq!(fresh["x"], 304);
        assert_eq!(fresh["y"], 112);
        assert_eq!(fresh["tick"], 0);
        assert_eq!(fresh["phase"], "walking");
        assert_eq!(fresh["error"], Value::Null);
        assert_ne!(fresh["snapshot_sha256"], checkpoint["snapshot_sha256"]);
        preview.step(2);
        preview.new_game();
        assert_eq!(preview.state(), fresh);
        preview.reset();
        assert_eq!(preview.state(), checkpoint);
        assert_eq!(preview.art(), art);
    }
    fn assert_actor(preview: &Preview, art: &Value) {
        let output = preview.state.output();
        let state = preview.state();
        let key = crate::room_art::frame_key(output.animation);
        assert_eq!(state["actor_key"], key);
        let scene = state["scene"].as_array().unwrap();
        let mut ids: Vec<_> = scene
            .iter()
            .map(|entry| entry["id"].as_str().unwrap())
            .collect();
        ids.sort_unstable();
        let mut expected: Vec<_> = art["scene_ids"][output.map_id.to_string()]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap())
            .collect();
        expected.sort_unstable();
        assert_eq!(ids, expected);
        for entry in scene {
            assert!(art["frames"].get(entry["key"].as_str().unwrap()).is_some());
        }
        assert!(scene.contains(
            &json!({ "id":"ark", "key":key, "position":[output.position.0,output.position.1] })
        ));
        assert!(art["frames"].get(&key).is_some(), "missing {key}");
        assert_eq!(state["animation"]["sequence"], output.animation.sequence);
        assert_eq!(state["animation"]["record"], output.animation.record);
        assert_eq!(state["animation"]["mirror_x"], output.animation.mirror_x);
    }
    fn fresh_route_fixture() -> Option<(Rom, String)> {
        let explicit = std::env::var_os("HOUSE_ROUTE_FIXTURES");
        let root = explicit.as_ref().map_or_else(
            || {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../local/new-game-qualification/fresh-house-route")
            },
            std::path::PathBuf::from,
        );
        if !root.exists() && explicit.is_none() {
            eprintln!(
                "SKIP: optional fresh house route absent; set HOUSE_ROUTE_FIXTURES to require it"
            );
            return None;
        }
        let pins: Value = serde_json::from_str(include_str!(
            "../../../tools/new-game-qualification/route-reference.json"
        ))
        .unwrap();
        let mut reference = Vec::new();
        for run in ["a", "b"] {
            let csv = std::fs::read(root.join(run).join("frames.csv")).unwrap();
            assert_eq!(sha256(&csv), pins["frames_sha256"]);
            if reference.is_empty() {
                reference = csv;
            } else {
                assert_eq!(reference, csv);
            }
            for pin in pins["checkpoints"].as_array().unwrap() {
                let wram =
                    std::fs::read(root.join(run).join(format!("f{}.wram", pin["frame"]))).unwrap();
                assert_eq!(sha256(&wram), pin["wram_sha256"]);
                assert_eq!(sha256(&wram[0xa000..0xb000]), pin["grid_sha256"]);
            }
        }
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        let rom = Rom::load(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(sha256(rom.image()), pins["rom_sha256"]);
        Some((rom, String::from_utf8(reference).unwrap()))
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Authenticated walking, doorway, snapshot and art checks along one route.
    fn authenticated_fresh_route_matches_compiled_core_across_both_doorways() {
        let Some((rom, text)) = fresh_route_fixture() else {
            return;
        };
        let mut preview = Preview::new(&rom).unwrap();
        preview.new_game();
        let rows: Vec<Vec<_>> = text
            .lines()
            .skip(1)
            .map(|l| l.split(',').collect())
            .collect();
        assert_eq!(rows.len(), 661);
        let art: Value = serde_json::from_slice(preview.art()).unwrap();
        let art_hash = sha256(preview.art());
        let mut total = 0;
        for (start, end, id, fresh) in [
            (6800, 6967, 15, true),
            (7050, 7114, 16, false),
            (7250, 7460, 15, false),
        ] {
            let grid = compile_room(&rom, id, fresh).unwrap();
            let segment: Vec<_> = rows
                .iter()
                .filter(|r| {
                    let frame = r[0].parse::<u16>().unwrap();
                    frame >= start && frame <= end
                })
                .collect();
            assert_eq!(segment.len(), usize::from(end - start) + 1);
            let position = preview.state.output().position;
            assert_eq!(
                position,
                (
                    segment[0][3].parse().unwrap(),
                    segment[0][4].parse().unwrap()
                )
            );
            let mut walking = room_core::WalkingState::new(position.0, position.1);
            for (index, row) in segment.iter().enumerate().skip(1) {
                assert_eq!(
                    row[0].parse::<u16>().unwrap(),
                    start + u16::try_from(index).unwrap()
                );
                let (button, direction) = match row[1] {
                    "" => (0, None),
                    "Left" => (1, Some(Direction::Left)),
                    "Right" => (2, Some(Direction::Right)),
                    "Up" => (3, Some(Direction::Up)),
                    "Down" => (4, Some(Direction::Down)),
                    _ => panic!("unqualified input"),
                };
                assert_eq!(u16::from_str_radix(row[5], 16).unwrap() & 0x1406, 0x0404);
                let input = FrameInput { direction };
                let component = walking.step(&grid, input).unwrap();
                assert_eq!(
                    (component.attempted_dx, component.attempted_dy),
                    (row[14].parse().unwrap(), row[15].parse().unwrap()),
                    "streams at {}",
                    row[0]
                );
                let mut restored =
                    GameState::restore(&preview.data, &preview.state.snapshot()).unwrap();
                preview.step(button);
                assert_actor(&preview, &art);
                assert!(
                    preview.error.is_none(),
                    "frame {}: {:?}",
                    row[0],
                    preview.error
                );
                assert_eq!(
                    preview.state.output(),
                    restored.step(&preview.data, input).unwrap()
                );
                assert_eq!(preview.state, restored);
                assert_eq!(preview.state.output().position, (component.x, component.y));
                assert_eq!(
                    preview.state.output().position,
                    (row[3].parse().unwrap(), row[4].parse().unwrap()),
                    "position at {}",
                    row[0]
                );
                assert_eq!(
                    preview.state.output().map_id,
                    u16::from_str_radix(row[2], 16).unwrap()
                );
                total += 1;
            }
            if end != 7460 {
                assert_eq!(preview.state.output().phase, Phase::Departing);
                for _ in 0..35 {
                    preview.step(0);
                    assert_actor(&preview, &art);
                    assert!(preview.error.is_none());
                }
            }
        }
        assert_eq!(total, 441);
        assert_eq!(sha256(preview.art()), art_hash);
        assert_eq!(preview.state.output().tick, 511);
        assert_eq!(preview.state.output().position, (392, 191));
        eprintln!("Matched441 authenticated fresh-start walking/stream steps across both semantic doorways, including per-step restored snapshots");
    }
}
