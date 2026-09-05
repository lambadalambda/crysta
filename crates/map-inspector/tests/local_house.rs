//! Optional CPU-free fresh-start end-to-end verification.
use std::{path::Path, process::Command};

#[test]
fn new_game_explores_both_rooms_and_revisits_without_scope_errors() {
    let rom = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    if !rom.try_exists().unwrap() {
        eprintln!("skipping: local Japanese ROM absent");
        return;
    }
    let run = || {
        let output = Command::new(env!("CARGO_BIN_EXE_map-inspector"))
            .args(["verify-house", rom.to_str().unwrap(), "semantic-preview"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
    };
    let result = run();
    assert_eq!(
        result,
        run(),
        "two fresh CPU-free processes must agree byte-for-byte"
    );
    for (key, tick, map, x, y, phase) in [
        ("initial", 0, 15, 304, 112, "walking"),
        ("outbound_handoff", 167, 15, 392, 208, "departing"),
        ("arrival", 202, 16, 392, 353, "walking"),
        ("return_handoff", 266, 16, 392, 336, "departing"),
        ("returned", 301, 15, 392, 191, "walking"),
        ("revisited", 511, 15, 392, 191, "walking"),
    ] {
        let state = &result[key];
        assert_eq!(state["start_kind"], "new-game");
        assert_eq!(state["tick"], tick, "{key}");
        assert_eq!(state["map_id"], map, "{key}");
        assert_eq!(state["x"], x, "{key}");
        assert_eq!(state["y"], y, "{key}");
        assert_eq!(state["phase"], phase, "{key}");
        assert_eq!(state["error"], serde_json::Value::Null, "{key}");
    }
}
