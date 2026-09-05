//! Map-ID resource inspection needs the ROM, not SRAM or emulator execution.
use std::{path::Path, process::Command};

#[test]
fn resolves_cavern_and_a_two_layer_map_without_booting() {
    let rom = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    if !rom.try_exists().unwrap() {
        eprintln!("skipping: local Japanese ROM not present");
        return;
    }
    for (id, count) in [("128", 1), ("25", 2)] {
        let output = Command::new(env!("CARGO_BIN_EXE_map-inspector"))
            .arg("resolve-map")
            .arg(&rom)
            .arg(id)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let manifest: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(manifest["map_id"], u16::from_str_radix(id, 16).unwrap());
        assert_eq!(manifest["layers"].as_array().unwrap().len(), count);
        assert!(!manifest["instructions"].as_array().unwrap().is_empty());
        if id == "128" {
            assert_eq!(manifest["entry"], 0xB3_89D3);
            assert_eq!(
                manifest["layers"][0]["source_range"],
                serde_json::json!([0x09_0000, 0x09_05F8])
            );
            assert_eq!(
                manifest["layers"][0]["cells"].as_array().unwrap().len(),
                2560
            );
        }
    }
    let output = Command::new(env!("CARGO_BIN_EXE_map-inspector"))
        .arg("resolve-map")
        .arg(rom)
        .arg("450")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}
