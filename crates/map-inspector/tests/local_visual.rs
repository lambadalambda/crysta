//! ROM-only visual export; no SRAM or oracle session is needed.
use std::{fs, path::Path, process::Command};
#[test]
fn exports_a_complete_static_cavern_and_rejects_other_ids() {
    let local = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local");
    let rom = local.join("Tenchi Souzou (Japan).sfc");
    if !rom.try_exists().unwrap() {
        eprintln!("skipping: local Japanese ROM not present");
        return;
    }
    let output = Command::new(env!("CARGO_BIN_EXE_map-inspector"))
        .args(["render-map", rom.to_str().unwrap(), "128"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let path = Path::new(
        stdout
            .trim()
            .strip_prefix("Local static map viewer: ")
            .unwrap(),
    );
    let directory = path.parent().unwrap();
    assert_eq!(
        directory.parent().unwrap(),
        local.join("static-maps").canonicalize().unwrap()
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("map.json")).unwrap()).unwrap();
    assert_eq!(manifest["map_id"], 0x128);
    assert_eq!(manifest["width"], 80);
    assert_eq!(manifest["height"], 32);
    assert_eq!(manifest["metatiles"].as_array().unwrap().len(), 512);
    assert_eq!(manifest["palette"].as_array().unwrap().len(), 128);
    assert_eq!(
        manifest["indexed_sha256"],
        "bb590b388811f2471232fc58e6a081455bd701540d4cf7a962c8b1e71827eb68"
    );
    assert_eq!(
        fs::read(directory.join("map.bmp")).unwrap().len(),
        54 + 1280 * 512 * 3
    );
    for (name, field) in [
        ("indices.bin", "indexed_sha256"),
        ("priority.bin", "priority_sha256"),
    ] {
        let bytes = fs::read(directory.join(name)).unwrap();
        assert_eq!(bytes.len(), 1280 * 512);
        assert_eq!(digest(&bytes), manifest[field]);
    }
    assert!(!fs::read_to_string(path)
        .unwrap()
        .contains("__STATIC_JSON__"));
    fs::remove_dir_all(directory).unwrap();
    for id in ["25", "10000", "junk"] {
        let output = Command::new(env!("CARGO_BIN_EXE_map-inspector"))
            .args(["render-map", rom.to_str().unwrap(), id])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
}

fn digest(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    rom::digests(bytes)
        .sha256
        .iter()
        .fold(String::new(), |mut s, byte| {
            write!(s, "{byte:02x}").unwrap();
            s
        })
}
