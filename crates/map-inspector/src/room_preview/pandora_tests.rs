use super::*;

fn owned_rom() -> Option<Rom> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local/Tenchi Souzou (Japan).sfc");
    if !path.try_exists().unwrap() {
        eprintln!("SKIP: local Japanese ROM absent");
        return None;
    }
    Some(Rom::load(&std::fs::read(path).unwrap()).unwrap())
}

#[test]
fn opt_in_host_starts_from_same_new_game_and_reserves_complete_presentation() {
    let Some(rom) = owned_rom() else {
        return;
    };
    let mut preview = Preview::new_profile(&rom, true).unwrap();
    preview.new_game();
    let state = preview.state();
    assert_eq!(state["pot_action"], true);
    assert_eq!(state["owner"], "player");
    assert_eq!(state["dialogue_ready"], false);
    assert_eq!(
        state["world_background"],
        json!({"key":"house","patches":[]})
    );
    assert!(state.get("scene_phase").is_none());
    assert_eq!(
        (state["x"].as_u64(), state["y"].as_u64()),
        (Some(304), Some(112))
    );
    assert_eq!(state["events"], json!([0x20, 0xfb]));
    assert_eq!(state["error"], Value::Null);
    for key in ["town13", "cellars", "box", "tour"] {
        assert!(preview.extra_bitmap(key).unwrap().starts_with(b"BM"));
    }
    let mut old = Preview::new(&rom).unwrap();
    old.new_game();
    let legacy = old.state();
    for key in [
        "owner",
        "world_background",
        "scene_phase",
        "carry",
        "dialogue_ready",
    ] {
        assert!(legacy.get(key).is_none());
    }
    assert_eq!(legacy["pot_action"], false);
    assert_eq!(legacy["scene"], state["scene"]);
}

#[test]
fn failed_projection_reports_primary_error_without_changing_simulation() {
    let Some(rom) = owned_rom() else {
        return;
    };
    let mut preview = Preview::new_profile(&rom, true).unwrap();
    preview.new_game();
    preview.cameras.remove(&15);
    let snapshot = preview.state.snapshot();
    for _ in 0..2 {
        let state = preview.state();
        assert_eq!(state["error"], "missing compiled camera");
        assert_eq!(state["scene"], json!([]));
        assert_eq!(preview.state.snapshot(), snapshot);
    }
}

#[test]
fn source_itinerary_projects_every_host_state_and_restores_through_final_control() {
    let Some(rom) = owned_rom() else {
        return;
    };
    let mut preview = Preview::new_profile(&rom, true).unwrap();
    preview.new_game();
    let route: Value = serde_json::from_str(include_str!(
        "../../../../tools/pandora-runtime-qualification/route.json"
    ))
    .unwrap();
    let mut frames = Vec::new();
    for command in route["actions"].as_array().unwrap() {
        let button = u8::try_from(command[0].as_u64().unwrap()).unwrap();
        for _ in 0..command[1].as_u64().unwrap() {
            preview.step(button);
            let state = preview.state();
            assert_eq!(
                state["error"],
                Value::Null,
                "tick {} command {button}",
                state["tick"]
            );
            let snapshot = preview.state.snapshot();
            preview.state = GameState::restore(&preview.data, &snapshot).unwrap();
            assert_eq!(preview.state(), state);
            frames.push(state);
        }
    }
    for tick in (5706..=5722).chain([9763]) {
        let state = &frames[tick - 1];
        assert!(state["dialogue"].is_object());
        assert_eq!(
            state["dialogue_ready"], false,
            "unfinished visible arrival at {tick}"
        );
    }
    for tick in [5723, 9764] {
        assert_eq!(frames[tick - 1]["dialogue_ready"], true);
    }
    let final_state = frames.last().unwrap();
    assert_eq!(final_state["tick"], route["expected"]["tick"]);
    assert_eq!(
        final_state["snapshot_sha256"],
        route["expected"]["snapshot_sha256"]
    );
    assert_eq!(final_state["map_id"], 0x41);
    assert_eq!(final_state["scene_phase"], "tour-control");
    assert_eq!(final_state["owner"], "player");
    let flags = final_state["events"].as_array().unwrap();
    assert!(flags.contains(&json!(0x243)) && flags.contains(&json!(0x244)));
    assert!(frames.iter().any(|s| s["carry"].is_object()));
    assert!(frames.iter().any(|s| s["world_background"]["patches"]
        .as_array()
        .is_some_and(|p| p.iter().any(|p| p["tile"] == 0x1a7))));
    if let Some(path) = std::env::var_os("PANDORA_PREVIEW_EXPORT") {
        std::fs::write(path,serde_json::to_vec(&json!({"route":route,"states":frames,"art":serde_json::from_slice::<Value>(preview.art()).unwrap()})).unwrap()).unwrap();
    }
}
