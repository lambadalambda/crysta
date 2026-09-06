//! ROM-only adapter export. Raw output is local qualification data, not shipped art.
use assets::sprites::{HouseNpc, SpritePixel};
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
    let n = HouseNpc::from_rom(r.image()).unwrap();
    let (l, t, right, bottom) = n.composition().bounds(false, false);
    let mut indexed = Vec::new();
    let mut rgba = Vec::new();
    let mut priorities = Vec::new();
    for y in t..bottom {
        for x in l..right {
            match n
                .composition()
                .sample(n.graphics(), false, false, x, y)
                .unwrap()
            {
                SpritePixel::Transparent => {
                    indexed.push(0);
                    rgba.extend([0; 4]);
                    priorities.push(255);
                }
                SpritePixel::Opaque {
                    palette_index,
                    priority,
                    ..
                } => {
                    indexed.push(palette_index);
                    rgba.extend(n.palette()[usize::from(palette_index - n.palette_base())].rgb8());
                    rgba.push(255);
                    priorities.push(priority);
                }
            }
        }
    }
    let tiles: Vec<_> = n
        .graphics()
        .iter()
        .flat_map(|t| t.pixels().iter().copied())
        .collect();
    std::fs::write(out.join("tiles"), &tiles).unwrap();
    let colors: Vec<_> = n.palette().iter().map(|c| c.rgb8()).collect();
    let sources: Vec<_> = n
        .source_ranges()
        .iter()
        .map(|s| json!({"start":s.start,"end":s.end,"sha256":hash(&r.image()[s.clone()])}))
        .collect();
    for (name, b) in [
        ("indexed", indexed.as_slice()),
        ("rgba", rgba.as_slice()),
        ("priorities", priorities.as_slice()),
        ("composition", n.composition().source_bytes()),
        ("source-composition", n.source_composition().source_bytes()),
    ] {
        std::fs::write(out.join(name), b).unwrap();
    }
    std::fs::write(out.join("export.json"),serde_json::to_vec_pretty(&json!({"map":n.map_id(),"position":n.position(),"pose_key":n.pose_key(),"graphics_packet":n.graphics_packet(),"facing":n.facing(),"hflip":n.hflip(),"palette_base":n.palette_base(),"colors":colors,"bounds":[l,t,right,bottom],"indexed_sha256":hash(&indexed),"rgba_sha256":hash(&rgba),"priorities_sha256":hash(&priorities),"sources":sources})).unwrap()).unwrap();
}
