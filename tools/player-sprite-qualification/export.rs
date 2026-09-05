//! Deterministic ROM-only transparent export, not a screenshot atlas.
use assets::sprites::{ArkSprites, SpritePixel};
use serde_json::json;
fn hash(b: &[u8]) -> String {
    rom::digests(b)
        .sha256
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect()
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "export ROM local/OUT");
    let out = std::path::Path::new(&args[2]);
    assert!(
        out.starts_with("local")
            && !out
                .components()
                .any(|c| c == std::path::Component::ParentDir)
    );
    std::fs::create_dir(out).unwrap();
    let r = rom::Rom::load(&std::fs::read(&args[1]).unwrap()).unwrap();
    assert_eq!(r.revision(), rom::Revision::Japan);
    let a = ArkSprites::from_rom(r.image()).unwrap();
    let mut frames = Vec::new();
    for (i, frame) in a.frames().iter().enumerate() {
        let horizontal = i == 2 || i >= 15;
        for mirror in [false, true] {
            if mirror && !horizontal {
                continue;
            }
            let c = frame.composition();
            let (l, t, right, bottom) = c.bounds(mirror, false);
            let tiles = a.graphics(frame.resource()).unwrap();
            let mut rgba = Vec::new();
            let mut priorities = Vec::new();
            for y in t..bottom {
                for x in l..right {
                    match c.sample(tiles, mirror, false, x, y).unwrap() {
                        SpritePixel::Transparent => {
                            rgba.extend([0; 4]);
                            priorities.push(255);
                        }
                        SpritePixel::Opaque {
                            palette_index,
                            priority,
                            ..
                        } => {
                            rgba.extend(a.palette()[usize::from(palette_index - 128)].rgb8());
                            rgba.push(255);
                            priorities.push(priority);
                        }
                    }
                }
            }
            let name = format!(
                "{:06x}-{}",
                frame.id(),
                if mirror { "left" } else { "normal" }
            );
            std::fs::write(out.join(format!("{name}.rgba")), &rgba).unwrap();
            std::fs::write(out.join(format!("{name}.priority")), &priorities).unwrap();
            frames.push(json!({"id":frame.id(),"resource":frame.resource(),"mirror":mirror,
                "bounds":[l,t,right,bottom],"width":right-l,"height":bottom-t,
                "rgba_sha256":hash(&rgba),"priority_sha256":hash(&priorities),"metadata_sha256":hash(c.source_bytes())}));
        }
    }
    let sources: Vec<_> = a
        .source_ranges()
        .iter()
        .map(|rge| json!({"start":rge.start,"end":rge.end,"sha256":hash(&r.image()[rge.clone()])}))
        .collect();
    let report =
        json!({"version":1,"rom_sha256":hash(r.image()),"sources":sources,"frames":frames});
    std::fs::write(
        out.join("export.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
