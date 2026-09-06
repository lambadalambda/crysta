//! Optional end-to-end capture qualification, using a fresh oracle process.
use std::{path::Path, process::Command};

const OBSERVER: &str = include_str!("../../../tools/map-inspector-qualification/observer.json");
const ARCHIVE: &str = include_str!(
    "../../../tools/map-inspector-qualification/epochs/threaded-video-v0/reference.json"
);
const SOURCE_FILES: &[&str] = &[
    "Cargo.lock",
    "crates/map-inspector/Cargo.toml",
    "crates/map-inspector/src/main.rs",
    "crates/map-inspector/web/viewer.html",
    "crates/oracle/build.rs",
    "crates/oracle/src/lib.rs",
    "vendor/ares/ares-unity.cpp",
    "vendor/ares/shims.cpp",
    "vendor/ares/ares/ares/ares.hpp",
    "vendor/ares/ares/ares/node/video/screen.cpp",
    "vendor/ares/ares/sfc/ppu/main.cpp",
    "vendor/ares/ares/sfc/ppu/color.cpp",
    "vendor/ares/ares/sfc/system/serialization.cpp",
];

fn sha256(bytes: &[u8]) -> String {
    use std::fmt::Write;
    rom::digests(bytes)
        .sha256
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            write!(&mut hex, "{byte:02x}").expect("writing to a String");
            hex
        })
}

fn check_source_inventory(observer: &serde_json::Value) {
    use std::collections::BTreeSet;
    assert_eq!(
        observer["source_hashes"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        SOURCE_FILES.iter().copied().collect::<BTreeSet<_>>(),
        "observer source inventory"
    );
}

#[test]
fn incomplete_observer_sources_are_rejected() {
    let observer: serde_json::Value = serde_json::from_str(OBSERVER).unwrap();
    for name in SOURCE_FILES {
        let mut changed = observer.clone();
        changed["source_hashes"]
            .as_object_mut()
            .unwrap()
            .remove(*name);
        assert!(std::panic::catch_unwind(|| check_source_inventory(&changed)).is_err());
    }
    let mut empty = observer;
    empty["source_hashes"] = serde_json::json!({});
    assert!(std::panic::catch_unwind(|| check_source_inventory(&empty)).is_err());
}

#[test]
fn fixture_observer_sources_match() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let observer: serde_json::Value = serde_json::from_str(OBSERVER).unwrap();
    assert_eq!(observer["epoch"], "headless-sync-video-v1");
    check_source_inventory(&observer);
    for (name, expected) in observer["source_hashes"].as_object().unwrap() {
        assert_eq!(
            sha256(&std::fs::read(root.join(name)).expect("read observer source")),
            expected.as_str().unwrap(),
            "observer source changed: {name}; review fixture policy before renewal"
        );
    }
}

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
    // The archived complete manifest (including every cell and non-pixel surface
    // hash) is invariant across epochs, not just the selected assertions below.
    let archive: serde_json::Value = serde_json::from_str(ARCHIVE).unwrap();
    let mut nonpixels = manifest.clone();
    for cp in nonpixels["checkpoints"].as_array_mut().unwrap() {
        cp.as_object_mut().unwrap().remove("rgb_sha256");
    }
    assert_eq!(
        sha256(&serde_json::to_vec(&nonpixels).unwrap()),
        archive["nonpixel_manifest_sha256"].as_str().unwrap()
    );
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
                // Reviewed slot3/no-save epoch renewal; see tools/map-inspector-qualification.
                "3833dbdf403939dc5836dca4a49b49e36424360597e5acfc6eddb714372da432"
            }
        );
    }
}
