//! ROM-only static extraction; no SRAM or oracle session is needed.
use std::{path::Path, process::Command};

#[test]
fn decodes_cavern_without_booting_the_game() {
    let rom = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    if !rom.try_exists().unwrap() {
        eprintln!("skipping: local Japanese ROM not present");
        return;
    }
    let output = Command::new(env!("CARGO_BIN_EXE_map-inspector"))
        .arg("decode-layer")
        .arg(rom)
        .arg("0x90000")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let layer: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(layer["width"], 80);
    assert_eq!(layer["height"], 32);
    assert_eq!(layer["source_range"], serde_json::json!([0x90000, 0x905F8]));
    assert_eq!(layer["cells"].as_array().unwrap().len(), 2560);
    assert_eq!(
        layer["layer_sha256"],
        "6a7495daacd32b54f0b6caf22bde1b873fa444455c5a7c39c854adfa230015fb"
    );
    assert!(
        layer.get("map_id").is_none(),
        "a source offset alone does not establish map identity"
    );
}
