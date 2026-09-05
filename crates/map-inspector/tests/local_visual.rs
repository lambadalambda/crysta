//! ROM-only visual export; no SRAM or oracle session is needed.
use std::{fs, path::Path, process::Command};
#[test]
fn exports_allowlisted_first_backgrounds_and_rejects_other_ids() {
    let local = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local");
    let rom = local.join("Tenchi Souzou (Japan).sfc");
    if !rom.try_exists().unwrap() {
        eprintln!("skipping: local Japanese ROM not present");
        return;
    }
    for (id, map_id, width, height, indexed_hash) in [
        (
            "128",
            0x128,
            80,
            32,
            "bb590b388811f2471232fc58e6a081455bd701540d4cf7a962c8b1e71827eb68",
        ),
        (
            "f",
            0xf,
            32,
            64,
            "4adec38bf192483ec43e62feaeb9219cc1e0d02b4860bc3b682dd5cc11c8886d",
        ),
        (
            "10",
            0x10,
            32,
            64,
            "4adec38bf192483ec43e62feaeb9219cc1e0d02b4860bc3b682dd5cc11c8886d",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_map-inspector"))
            .args(["render-map", rom.to_str().unwrap(), id])
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
        assert_manifest_format(&manifest, map_id);
        assert_eq!(manifest["map_id"], map_id);
        assert_eq!(manifest["width"], width);
        assert_eq!(manifest["height"], height);
        assert_eq!(manifest["metatiles"].as_array().unwrap().len(), 512);
        assert_eq!(manifest["palette"].as_array().unwrap().len(), 128);
        assert_eq!(manifest["indexed_sha256"], indexed_hash);
        assert_bitmap(
            &fs::read(directory.join("map.bmp")).unwrap(),
            width * 16,
            height * 16,
            &manifest,
        );
        for (name, field) in [
            ("indices.bin", "indexed_sha256"),
            ("priority.bin", "priority_sha256"),
        ] {
            let bytes = fs::read(directory.join(name)).unwrap();
            assert_eq!(bytes.len(), width * height * 256);
            assert_eq!(digest(&bytes), manifest[field]);
        }
        assert!(!fs::read_to_string(path)
            .unwrap()
            .contains("__STATIC_JSON__"));

        assert!(manifest["limits"]
            .as_str()
            .unwrap()
            .contains("first background"));
        if map_id != 0x128 {
            assert_eq!(
                manifest["rgb_sha256"],
                "5fa1f26738451e0b3feb350dc21f1edf587c18a9b43fbc318af7c7fc220160b2"
            );
        }
        fs::remove_dir_all(directory).unwrap();
    }
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

fn assert_manifest_format(manifest: &serde_json::Value, id: u16) {
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(
        manifest["kind"],
        if id == 0x128 {
            "static-cavern-background"
        } else {
            "static-room-background"
        }
    );
    let keys: Vec<_> = manifest
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        [
            "cells",
            "height",
            "image",
            "indexed_sha256",
            "indices",
            "kind",
            "limits",
            "map_id",
            "metatiles",
            "palette",
            "priorities",
            "priority_sha256",
            "resources",
            "revision",
            "rgb_sha256",
            "rom_sha256",
            "schema_version",
            "width",
        ]
    );
}

fn assert_bitmap(bytes: &[u8], width: usize, height: usize, manifest: &serde_json::Value) {
    assert_eq!(&bytes[..2], b"BM");
    assert_eq!(bytes.len(), 54 + width * height * 3);
    assert_eq!(u32::from_le_bytes(bytes[10..14].try_into().unwrap()), 54);
    assert_eq!(
        i32::from_le_bytes(bytes[18..22].try_into().unwrap()),
        i32::try_from(width).unwrap()
    );
    assert_eq!(
        i32::from_le_bytes(bytes[22..26].try_into().unwrap()),
        -i32::try_from(height).unwrap()
    );
    assert_eq!(u16::from_le_bytes(bytes[28..30].try_into().unwrap()), 24);
    // These widths need no row padding. Reconstruct the actual top-down RGB file.
    let rgb: Vec<_> = bytes[54..]
        .chunks_exact(3)
        .flat_map(|bgr| [bgr[2], bgr[1], bgr[0]])
        .collect();
    assert_eq!(digest(&rgb), manifest["rgb_sha256"]);
}
