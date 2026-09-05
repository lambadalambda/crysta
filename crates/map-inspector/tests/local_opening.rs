//! Optional owned-ROM input-only Crysta replay and exit-stage qualification.
use std::{path::Path, process::Command};

#[test]
fn opening_doorway_repeats_and_matches_decoded_exit() {
    let local = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local");
    let rom = local.join("Tenchi Souzou (Japan).sfc");
    let save = local.join("saves/Terranigma.srm");
    if !rom.try_exists().unwrap() || !save.try_exists().unwrap() {
        eprintln!("skipping: local Japanese ROM or qualified SRAM not present");
        return;
    }
    let run = |mode| {
        let output = Command::new(env!("CARGO_BIN_EXE_map-inspector"))
            .arg(mode)
            .arg(&rom)
            .arg(&save)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
    };
    let first = run("qualify-opening");
    assert_eq!(first, run("qualify-opening"));
    assert_eq!(first["scenario"], "qualified-slot-1-crysta-doorway");
    assert_eq!(
        first["checkpoints"][0]["player"],
        serde_json::json!([472, 176])
    );
    assert_eq!(
        first["checkpoints"][6]["player"],
        serde_json::json!([392, 353])
    );
    assert_eq!(first["checkpoints"][6]["map_id"], 16);
    let trace = run("trace-opening");
    assert_eq!(
        trace["exit"]["record_source"],
        serde_json::json!([0x1_8e3c, 0x1_8e48])
    );
    assert_eq!(trace["exit"]["destination"], 16);
    assert_eq!(trace["stops"].as_array().unwrap().len(), 8);
}
