//! Local source-only export; no native capture or CPU execution is an input.
use assets::{graphics::IndexedPixel, maps::visual::pandora::PandoraBackground};
use serde_json::json;
fn hash(bytes: &[u8]) -> String {
    rom::digests(bytes)
        .sha256
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn main() {
    let a: Vec<_> = std::env::args().collect();
    assert_eq!(a.len(), 3, "export ROM local/OUT");
    let out = std::path::Path::new(&a[2]);
    assert!(
        out.starts_with("local")
            && !out
                .components()
                .any(|c| c == std::path::Component::ParentDir)
    );
    std::fs::create_dir(out).unwrap();
    let r = rom::Rom::load(&std::fs::read(&a[1]).unwrap()).unwrap();
    assert_eq!(r.revision(), rom::Revision::Japan);
    let mut reports = Vec::new();
    for id in [0xa, 0xe, 0x13, 0x20, 0x21, 0x41, 0x42, 0x43, 0x44] {
        let p = PandoraBackground::from_rom(r.image(), id).unwrap();
        let bg = p.background();
        let width = bg.layer().width() * 16;
        let height = bg.layer().height() * 16;
        let directory = out.join(format!("{id:x}"));
        std::fs::create_dir(&directory).unwrap();
        let mut indices = Vec::new();
        let mut priorities = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let (i, p) = match bg.pixel(x, y).unwrap() {
                    IndexedPixel::Transparent => (0, false),
                    IndexedPixel::Opaque {
                        palette_index,
                        priority,
                    } => (palette_index, priority),
                };
                indices.push(i);
                priorities.push(u8::from(p));
            }
        }
        let palette: Vec<_> = bg
            .palette()
            .iter()
            .flat_map(|c| c.raw().to_le_bytes())
            .collect();
        let grid: Vec<_> = p
            .attributed_grid()
            .iter()
            .flat_map(|c| c.raw().to_le_bytes())
            .collect();
        let mut files = serde_json::Map::new();
        for (name, bytes) in [
            ("indices", indices.as_slice()),
            ("priorities", &priorities),
            ("palette", &palette),
            ("static-grid", &grid),
            ("cells", &bg.layer().layer_bytes()),
            ("graphics", bg.resources()[0].decoded()),
            ("definitions", bg.resources()[2].decoded()),
            ("attributes", bg.resources()[3].decoded()),
        ] {
            std::fs::write(directory.join(name), bytes).unwrap();
            files.insert(name.into(), json!(hash(bytes)));
        }
        let sources: Vec<_> = bg.resources().iter().map(|v| json!({"kind":format!("{:?}",v.kind()),"start":v.source_range().start,"end":v.source_range().end,"decoded_bytes":v.decoded().len(),"sha256":hash(v.source_bytes())})).chain(std::iter::once(json!({"kind":"Layer","start":bg.layer().source_range().start,"end":bg.layer().source_range().end,"decoded_bytes":bg.layer().cells().len()*2,"sha256":hash(bg.layer().source_bytes())}))).collect();
        let c = p.camera();
        reports.push(json!({"map":id,"width":width,"height":height,"files":files,"sources":sources,
            "initialization":format!("{:?}",p.initialization()),"policies":format!("{:?}",p.policies()),
            "camera":{"record_offset":c.record_offset,"record":c.record,"scene_offset":c.scene_offset,
                "display_offset":c.display_offset,"display":c.display,"bounds":c.bounds,
                "vertical_extent":c.vertical_extent,"hardware_background":c.hardware_background,
                "ring_word_base":c.ring_word_base,"bgmode":c.bgmode}}));
    }
    std::fs::write(
        out.join("export.json"),
        serde_json::to_vec_pretty(&reports).unwrap(),
    )
    .unwrap();
}
