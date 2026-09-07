//! Optional end-to-end capture qualification, using a fresh oracle process.
use std::{path::Path, process::Command};

const CURRENT: &str =
    include_str!("../../../tools/map-inspector-qualification/current-producer.json");
const OBSERVER: &str = include_str!("../../../tools/map-inspector-qualification/observer.json");
const MIGRATION: &str = include_str!("../../../tools/map-inspector-qualification/migration.json");
const OBSERVER_SHA: &str = "7fabf5688943eca89c43ad5aee02d348187fc3491296b1c401535553fbe3a718";
const MIGRATION_SHA: &str = "db249179718cb6bcf9c1755093d441d0094dd39fc836d079220defd3b289ab3c";
const OLD_MAIN_SHA: &str = "7736b543c442e6e4c2789fb13f6f177d1335e78f11810d5023a49c313b27a4d3";
const MAIN: &str = "crates/map-inspector/src/main.rs";
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

// Separate reviewed preview hooks; the historical 13-file inventory above is unchanged.
const ADDITIONAL_FILES: &[&str] = &[
    "crates/map-inspector/src/pandora_navigation.rs",
    "crates/map-inspector/src/pandora_progression.rs",
    "crates/map-inspector/src/room_art.rs",
    "crates/map-inspector/src/room_art/backgrounds.rs",
    "crates/map-inspector/src/room_art/carry.rs",
    "crates/map-inspector/src/room_art/door.rs",
    "crates/map-inspector/src/room_art/pandora.rs",
    "crates/map-inspector/src/room_art/world_patches.rs",
    "crates/map-inspector/src/room_camera.rs",
    "crates/map-inspector/src/room_preview.rs",
    "crates/map-inspector/src/room_server.rs",
    "crates/map-inspector/web/room-slice.html",
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

fn check_inventory(hashes: &serde_json::Value, files: &[&str]) {
    use std::collections::BTreeSet;
    assert_eq!(
        hashes
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        files.iter().copied().collect::<BTreeSet<_>>(),
        "producer source inventory"
    );
}

fn check_source_inventory(observer: &serde_json::Value) {
    check_inventory(&observer["source_hashes"], SOURCE_FILES);
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

fn check_producer_identity(current: &serde_json::Value) {
    assert_eq!(sha256(OBSERVER.as_bytes()), OBSERVER_SHA);
    assert_eq!(sha256(MIGRATION.as_bytes()), MIGRATION_SHA);
    let original: serde_json::Value = serde_json::from_str(OBSERVER).unwrap();
    assert_eq!(current["schema_version"], 1);
    assert_eq!(current["epoch"], "headless-sync-video-v1");
    assert_eq!(current["epoch"], original["epoch"]);
    assert_eq!(current["policy"], original["policy"]);
    assert_eq!(current["original_descriptor_sha256"], OBSERVER_SHA);
    assert_eq!(current["migration_sha256"], MIGRATION_SHA);
    check_source_inventory(current);
    check_inventory(&current["additional_source_hashes"], ADDITIONAL_FILES);
    for name in SOURCE_FILES.iter().copied().filter(|name| *name != MAIN) {
        assert_eq!(
            current["source_hashes"][name],
            original["source_hashes"][name]
        );
    }
}

#[test]
fn historical_identity_substitution_is_rejected() {
    let current: serde_json::Value = serde_json::from_str(CURRENT).unwrap();
    check_producer_identity(&current);
    for field in [
        "schema_version",
        "epoch",
        "policy",
        "original_descriptor_sha256",
        "migration_sha256",
    ] {
        let mut changed = current.clone();
        changed[field] = serde_json::json!("substitution");
        assert!(std::panic::catch_unwind(|| check_producer_identity(&changed)).is_err());
    }
    let old = serde_json::from_str(OBSERVER).unwrap();
    assert!(std::panic::catch_unwind(|| check_producer_identity(&old)).is_err());
}

fn check_main_registration(main: &[u8]) {
    const ANCHOR: &[u8] = b"mod opening_qualification;\n";
    const INSERT: &[u8] = b"pub mod pandora_navigation;\npub mod pandora_progression;\n";
    let registered = [ANCHOR, INSERT].concat();
    let positions: Vec<_> = main
        .windows(registered.len())
        .enumerate()
        .filter_map(|(index, bytes)| (bytes == registered).then_some(index))
        .collect();
    assert_eq!(positions.len(), 1, "exact registration anchor required");
    let index = positions[0];
    // Undo ONLY the exact anchored insertion and authenticate every remaining
    // byte as the old producer. No line stripping or whitespace normalization.
    let old = [
        &main[..index + ANCHOR.len()],
        &main[index + registered.len()..],
    ]
    .concat();
    assert_eq!(sha256(&old), OLD_MAIN_SHA, "non-registration main change");
}

#[test]
fn non_registration_main_changes_are_rejected() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let main = std::fs::read(root.join("crates/map-inspector/src/main.rs")).unwrap();
    check_main_registration(&main);
    let text = std::str::from_utf8(&main).unwrap();
    for changed in [
        format!("{text}\n"),
        text.replace('\n', "\r\n"),
        text.replace("fn main()", "fn changed()"),
        text.replace("pub mod pandora_navigation;\n", ""),
        text.replace("pub mod pandora_progression;\n", ""),
        text.replace("mod opening_qualification;", "mod opening_qualification; "),
    ] {
        assert!(std::panic::catch_unwind(|| check_main_registration(changed.as_bytes())).is_err());
    }
}

fn check_current_sources(root: &Path, current: &serde_json::Value) {
    check_producer_identity(current);
    for field in ["source_hashes", "additional_source_hashes"] {
        for (name, expected) in current[field].as_object().unwrap() {
            assert_eq!(
                sha256(&std::fs::read(root.join(name)).expect("read producer source")),
                expected.as_str().unwrap(),
                "current producer source changed: {name}; explicitly revalidate before repinning"
            );
        }
    }
    check_main_registration(&std::fs::read(root.join(MAIN)).unwrap());
}

#[test]
fn fixture_current_producer_sources_match() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let current = serde_json::from_str(CURRENT).unwrap();
    check_current_sources(&root, &current);
}

#[test]
fn incomplete_or_substituted_current_sources_are_rejected() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let current: serde_json::Value = serde_json::from_str(CURRENT).unwrap();
    for field in ["source_hashes", "additional_source_hashes"] {
        for name in current[field].as_object().unwrap().keys() {
            let mut changed = current.clone();
            changed[field].as_object_mut().unwrap().remove(name);
            assert!(std::panic::catch_unwind(|| check_current_sources(&root, &changed)).is_err());
            let mut changed = current.clone();
            changed[field][name] = serde_json::json!("substituted digest");
            assert!(std::panic::catch_unwind(|| check_current_sources(&root, &changed)).is_err());
        }
        let mut changed = current.clone();
        changed[field]["unexpected/source.rs"] = serde_json::json!("extra source");
        assert!(std::panic::catch_unwind(|| check_current_sources(&root, &changed)).is_err());
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
