//! External API checks for the host-free Pandora preview library.

use map_inspector::PandoraPreview;

fn digest(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    rom::digests(bytes)
        .sha256
        .iter()
        .fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").unwrap();
            hex
        })
}

#[test]
fn public_preview_rejects_unauthenticated_rom_bytes() {
    assert!(PandoraPreview::from_rom_bytes(&[]).is_err());
    assert!(PandoraPreview::from_rom_bytes(&vec![0; 4 * 1024 * 1024]).is_err());
}

#[test]
fn public_preview_preserves_the_accepted_initial_outputs() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local/Tenchi Souzou (Japan).sfc");
    let Ok(bytes) = std::fs::read(path) else {
        eprintln!("SKIP: local Japanese ROM absent");
        return;
    };

    let preview = PandoraPreview::from_rom_bytes(&bytes).unwrap();
    let state = preview.state();
    assert_eq!(state["schema_version"], 1);
    assert_eq!(state["policy"], "semantic-preview");
    assert_eq!(state["map_id"], 15);
    assert_eq!(state["start_kind"], "saved-checkpoint");
    assert_eq!(state["error"], serde_json::Value::Null);
    assert!(preview.bitmap().starts_with(b"BM"));
    assert!(preview.exterior_bitmap().starts_with(b"BM"));
    assert_eq!(
        digest(preview.bitmap()),
        "b090ab8586596ae6132ed8734ded834ad925666883f5dcef3c07f7cb48083f8a"
    );
    assert_eq!(
        digest(preview.exterior_bitmap()),
        "07c34322dd91e6d2dcb1f8bbf2c93cf41ab45b8bb45a7fc00ef51b034048ca72"
    );
    let rom = rom::Rom::load(&bytes).unwrap();
    assert_eq!(
        preview.bitmap(),
        map_inspector::render_static_background(&rom, 15)
            .unwrap()
            .bitmap
    );
    assert_eq!(
        preview.exterior_bitmap(),
        map_inspector::render_static_background(&rom, 10)
            .unwrap()
            .bitmap
    );
    for key in ["town13", "cellars", "box", "tour"] {
        assert!(preview.extra_bitmap(key).unwrap().starts_with(b"BM"));
    }

    let initial = state;
    let art = preview.art().to_vec();
    let house = preview.bitmap().to_vec();
    let exterior = preview.exterior_bitmap().to_vec();
    let mut preview = preview;
    preview.new_game();
    let fresh = preview.state();
    assert_eq!(fresh["start_kind"], "new-game");
    assert_eq!(
        (fresh["x"].as_u64(), fresh["y"].as_u64()),
        (Some(304), Some(112))
    );
    assert_eq!(fresh["error"], serde_json::Value::Null);
    preview.reset();
    assert_eq!(preview.state(), initial);
    assert_eq!(preview.art(), art);
    assert_eq!(preview.bitmap(), house);
    assert_eq!(preview.exterior_bitmap(), exterior);
}
