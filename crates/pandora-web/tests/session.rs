//! Native parity checks for the stateful browser adapter.

use pandora_web::Session;

#[test]
fn invalid_replacement_does_not_leave_a_stale_session() {
    let mut slot = None;
    assert!(Session::replace(&mut slot, &[]).is_err());
    assert!(slot.is_none());
}

#[test]
fn adapter_matches_the_public_preview_and_bounds_commands() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local/Tenchi Souzou (Japan).sfc");
    let Ok(bytes) = std::fs::read(path) else {
        eprintln!("SKIP: local Japanese ROM absent");
        return;
    };
    let mut direct = map_inspector::PandoraPreview::from_rom_bytes(&bytes).unwrap();
    let mut adapter = Session::new(&bytes).unwrap();
    assert_eq!(adapter.state(), direct.state().to_string());
    for input in [0, 2, 2, 0, 5, 6, 10] {
        direct.step(input);
        assert_eq!(adapter.step(input).unwrap(), direct.state().to_string());
    }
    let before = adapter.state();
    assert!(adapter.step(11).is_err());
    assert_eq!(adapter.state(), before);
    assert!(adapter.run_parity_inputs(&[11]).is_err());
    assert_eq!(adapter.state(), before);
    let mut art = adapter.art();
    assert_eq!(art, direct.art());
    art[0] ^= 0xff;
    assert_eq!(adapter.art(), direct.art());
    let mut house = adapter.background("house").unwrap();
    assert_eq!(house, direct.bitmap());
    house[0] ^= 0xff;
    assert_eq!(adapter.background("house").unwrap(), direct.bitmap());
    assert_eq!(
        adapter.background("exterior").unwrap(),
        direct.exterior_bitmap()
    );
    assert!(adapter.background("unknown").is_err());
    assert_eq!(adapter.reset(), direct_reset(&mut direct));
    assert_eq!(adapter.new_game(), direct_new_game(&mut direct));
}

fn direct_reset(preview: &mut map_inspector::PandoraPreview) -> String {
    preview.reset();
    preview.state().to_string()
}

fn direct_new_game(preview: &mut map_inspector::PandoraPreview) -> String {
    preview.new_game();
    preview.state().to_string()
}
