//! Optional end-to-end capture qualification, using a fresh oracle process.
use std::{path::Path, process::Command};

#[test]
fn loaded_map_matches_qualified_runtime_checkpoint() {
    let local = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local");
    let rom = local.join("Tenchi Souzou (Japan).sfc");
    let save = local.join("saves/Terranigma.srm");
    if !rom.try_exists().expect("inspect local ROM path")
        || !save.try_exists().expect("inspect local SRAM path")
    {
        eprintln!("skipping: local Japanese ROM or qualified SRAM not present");
        return;
    }
    let output = Command::new(env!("CARGO_BIN_EXE_map-inspector"))
        .arg("verify")
        .arg(rom)
        .arg(save)
        .output()
        .expect("spawn isolated map capture");
    assert!(
        output.status.success(),
        "capture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("capture manifest");
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["revision"], "japan");
    let checkpoints = manifest["checkpoints"].as_array().unwrap();
    assert_eq!(checkpoints.len(), 2);
    for (index, cp) in checkpoints.iter().enumerate() {
        assert_eq!(cp["map_id"], 0x0128);
        assert_eq!(cp["width"], 80);
        assert_eq!(cp["height"], 32);
        assert_eq!(cp["cells"].as_array().unwrap().len(), 2560);
        assert_eq!(cp["cells"][364], 0x0E11);
        assert_eq!(cp["cells"][448], 0x8007);
        assert_eq!(cp["cells"][608], 5);
        assert_eq!(cp["cells"][2559], 0x1C39);
        assert_eq!(
            cp["layer_sha256"],
            "c3c7af3a0ef3c6c53e641b058ccaccad8a9b5ea42dca79c28e41c41ecaa450c5"
        );
        assert_eq!(cp["frame"], if index == 0 { 1601 } else { 1841 });
        assert_eq!(
            cp["camera"],
            if index == 0 {
                serde_json::json!([648, 0])
            } else {
                serde_json::json!([706, 16])
            }
        );
        assert_eq!(
            cp["player"],
            if index == 0 {
                serde_json::json!([776, 112])
            } else {
                serde_json::json!([834, 128])
            }
        );
        assert_eq!(
            cp["rgb_sha256"],
            if index == 0 {
                "78c20d6a5dca2a006815c13f6577b33bcdc1ddb788ddad9949bdf889eaa7b0ea"
            } else {
                "93a224b980b538bf5bbed26c7d7052a6c14777608e6d02162eb55cfb2ef74c8a"
            }
        );
    }
}
