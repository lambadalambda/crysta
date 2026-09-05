//! Optional authenticated, CPU-free semantic preview integration.
use std::{path::Path, process::Command};
#[test]
fn semantic_preview_matches_approach_and_transition_endpoints() {
    let rom = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    if !rom.try_exists().unwrap() {
        eprintln!("skipping: local Japanese ROM absent");
        return;
    }
    let run = || {
        let out = Command::new(env!("CARGO_BIN_EXE_map-inspector"))
            .arg("verify-room")
            .arg(&rom)
            .arg("semantic-preview")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&out.stdout).unwrap()
    };
    let value = run();
    assert_eq!(value, run());
    assert_eq!(value["handoff"]["tick"], 80);
    assert_eq!(value["departure"]["map_id"], 15);
    assert_eq!(value["departure"]["y"], 226);
    assert_eq!(value["spawn"]["map_id"], 16);
    assert_eq!(value["spawn"]["y"], 336);
    assert_eq!(value["arrival"]["y"], 353);
    // This is a logical policy, not an assertion of reference video timing.
    assert_eq!(value["arrival"]["tick"], 115);
    assert_eq!(value["return_handoff"]["y"], 336);
    assert_eq!(value["return_handoff"]["phase"], "departing");
    assert_eq!(value["returned"]["map_id"], 15);
    assert_eq!(value["returned"]["x"], 392);
    assert_eq!(value["returned"]["y"], 191);
    assert_eq!(value["returned"]["tick"], 164);
    assert_eq!(value["returned"]["error"], serde_json::Value::Null);
}
