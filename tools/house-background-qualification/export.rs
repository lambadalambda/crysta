//! ROM-only full-sheet export for local qualification, never a runtime fixture.
use assets::{graphics::IndexedPixel, maps::visual::StaticBackground};
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
    for id in [0xb, 0xc, 0xd, 0xf, 0x10, 0x11] {
        let bg = StaticBackground::from_rom(r.image(), id).unwrap();
        assert_eq!((bg.layer().width(), bg.layer().height()), (32, 64));
        let directory = out.join(format!("{id:x}"));
        std::fs::create_dir(&directory).unwrap();
        let mut indices = Vec::new();
        let mut priorities = Vec::new();
        let mut rgb = Vec::new();
        for y in 0..1024 {
            for x in 0..512 {
                let (i, p) = match bg.pixel(x, y).unwrap() {
                    IndexedPixel::Transparent => (0, false),
                    IndexedPixel::Opaque {
                        palette_index,
                        priority,
                    } => (palette_index, priority),
                };
                indices.push(i);
                priorities.push(u8::from(p));
                rgb.extend(if i == 0 {
                    [if (x / 8 + y / 8) % 2 == 0 { 48 } else { 80 }; 3]
                } else {
                    bg.palette()[usize::from(i)].rgb8()
                });
            }
        }
        let palette: Vec<_> = bg
            .palette()
            .iter()
            .flat_map(|c| c.raw().to_le_bytes())
            .collect();
        let attributes: &[u8; 512] = bg.resources()[3].decoded().try_into().unwrap();
        let grid: Vec<_> = bg
            .layer()
            .attributed_cells(attributes)
            .iter()
            .flat_map(|c| c.raw().to_le_bytes())
            .collect();
        let mut files = serde_json::Map::new();
        for (name, bytes) in [
            ("indices", indices.as_slice()),
            ("priorities", &priorities),
            ("rgb", &rgb),
            ("palette", &palette),
            ("static-grid", &grid),
            ("cells", &bg.layer().layer_bytes()),
            ("graphics", bg.resources()[0].decoded()),
            ("definitions", bg.resources()[2].decoded()),
            ("attributes", attributes),
        ] {
            std::fs::write(directory.join(name), bytes).unwrap();
            files.insert(name.into(), json!(hash(bytes)));
        }
        let sources: Vec<_> = bg.resources().iter().map(|v| json!({"start":v.source_range().start,"end":v.source_range().end,"sha256":hash(v.source_bytes())})).chain(std::iter::once(json!({"start":bg.layer().source_range().start,"end":bg.layer().source_range().end,"sha256":hash(bg.layer().source_bytes())}))).collect();
        reports.push(json!({"map":id,"files":files,"sources":sources}));
    }
    std::fs::write(
        out.join("export.json"),
        serde_json::to_vec_pretty(&reports).unwrap(),
    )
    .unwrap();
}
