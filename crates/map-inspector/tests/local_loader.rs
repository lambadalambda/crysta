//! Optional fresh-process loader trace qualification against the owned ROM/save.
use std::{path::Path, process::Command};

#[test]
fn static_cavern_layers_match_loader_stages() {
    let local = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local");
    let rom = local.join("Tenchi Souzou (Japan).sfc");
    let save = local.join("saves/Terranigma.srm");
    if !rom.try_exists().unwrap() || !save.try_exists().unwrap() {
        eprintln!("skipping: local Japanese ROM or qualified SRAM not present");
        return;
    }
    let output = Command::new(env!("CARGO_BIN_EXE_map-inspector"))
        .arg("qualify-loader")
        .arg(rom)
        .arg(save)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["menu"]["map_id"], 4);
    assert_eq!(result["menu"]["entry"], 0xB3_8002);
    assert_eq!(
        result["menu"]["layer_source"],
        serde_json::json!([0x0D_16C7, 0x0D_1725])
    );
    assert_eq!(result["menu"]["dimensions"], serde_json::json!([16, 48]));
    assert_eq!(
        result["menu"]["layer_sha256"],
        "3d28667ce54abe8e5d5327adac2cddcf87e5ce49c45b137e60f67f6a82d14ac5"
    );
    assert_eq!(result["map_id"], 0x128);
    assert_eq!(
        result["layer_source"],
        serde_json::json!([0x90000, 0x905F8])
    );
    assert_eq!(
        result["attribute_source"],
        serde_json::json!([0x2B_439E, 0x2B_4462])
    );
    assert_eq!(
        result["static_sha256"],
        "6a7495daacd32b54f0b6caf22bde1b873fa444455c5a7c39c854adfa230015fb"
    );
    assert_eq!(
        result["attributed_sha256"],
        "f1b717b31a416aab274c8f62ca53b9b585df7a60a9799c836caeeb9f949b550b"
    );
    assert_eq!(
        result["runtime_sha256"],
        "c3c7af3a0ef3c6c53e641b058ccaccad8a9b5ea42dca79c28e41c41ecaa450c5"
    );
    assert_eq!(
        result["runtime_differences"],
        serde_json::json!([{"index":448,"initialized":7,"runtime":0x8007}])
    );
    let stops = result["stops"].as_array().unwrap();
    assert_eq!(stops.len(), 10);
    assert_eq!(stops[1]["script_pointer"], 0xB3_89D3);
    assert_eq!(stops[2]["source_pointer"], 0xEB_439E);
    assert_eq!(stops[2]["destination_pointer"], 0x7E_5000);
    assert_eq!(stops[5]["source_pointer"], 0xC9_0000);
    assert_eq!(stops[6]["source_pointer"], 0xC9_0002);
    assert_eq!(stops[6]["destination_pointer"], 0x7E_A000);
    assert_eq!(stops[7]["frame"], 1138);
}
