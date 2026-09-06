//! ROM-only house setup/list export. All raw products must remain local.
use assets::sprites::{HouseGraphicsKey, HousePoseKey, HouseScenes, SpritePixel};
use serde_json::json;
fn hash(b: &[u8]) -> String {
    rom::digests(b)
        .sha256
        .iter()
        .map(|v| format!("{v:02x}"))
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
    let scenes = HouseScenes::from_rom(r.image()).unwrap();
    let mut actors = Vec::new();
    for actor in scenes.actors() {
        let id = actor.source_id();
        let dir = out.join(format!("{id:06x}"));
        std::fs::create_dir(&dir).unwrap();
        let tiles: Vec<_> = actor
            .graphics()
            .iter()
            .flat_map(|t| t.pixels().iter().copied())
            .collect();
        let palette: Vec<_> = actor
            .palette()
            .iter()
            .flat_map(|c| c.raw().to_le_bytes())
            .collect();
        std::fs::write(dir.join("tiles"), &tiles).unwrap();
        std::fs::write(dir.join("palette"), &palette).unwrap();
        let mut frames = Vec::new();
        for (index, frame) in actor.frames().iter().enumerate() {
            let c = frame.composition();
            let (l, t, right, bottom) = c.bounds(actor.hflip(), false);
            let mut indexed = Vec::new();
            let mut rgba = Vec::new();
            let mut priorities = Vec::new();
            for y in t..bottom {
                for x in l..right {
                    match c
                        .sample(actor.graphics(), actor.hflip(), false, x, y)
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
                            rgba.extend(
                                actor.palette()[usize::from(palette_index - actor.palette_base())]
                                    .rgb8(),
                            );
                            rgba.push(255);
                            priorities.push(priority);
                        }
                    }
                }
            }
            for (name, bytes) in [
                ("composition", c.source_bytes()),
                (
                    "source-composition",
                    frame.source_composition().source_bytes(),
                ),
                ("indexed", indexed.as_slice()),
                ("rgba", rgba.as_slice()),
                ("priorities", priorities.as_slice()),
            ] {
                std::fs::write(dir.join(format!("{index}-{name}")), bytes).unwrap();
            }
            let key = match frame.key() {
                HousePoseKey::Compressed { packet, offset } => {
                    json!({"packet":packet,"offset":offset})
                }
                HousePoseKey::Direct(address) => json!({"direct":address}),
            };
            frames.push(json!({"index":index,"key":key,"duration":frame.duration(),"facing":frame.facing(),"bounds":[l,t,right,bottom],"opaque":indexed.iter().filter(|&&p|p!=0).count(),"indexed_sha256":hash(&indexed),"rgba_sha256":hash(&rgba),"priorities_sha256":hash(&priorities),"composition_sha256":hash(c.source_bytes()),"source_composition_sha256":hash(frame.source_composition().source_bytes())}));
        }
        let HouseGraphicsKey::Compressed(graphics) = actor.graphics_key();
        let sources:Vec<_>=actor.source_ranges().iter().map(|range|json!({"start":range.start,"end":range.end,"sha256":hash(&r.image()[range.clone()])})).collect();
        actors.push(json!({"id":id,"map":actor.map_id(),"position":actor.position(),"selector":actor.selector(),"hflip":actor.hflip(),"tie_rank":actor.tie_rank(),"ark_tie_rank":HouseScenes::ark_tie_rank(actor.map_id()).unwrap(),"palette_base":actor.palette_base(),"graphics_packet":graphics,"tiles_sha256":hash(&tiles),"palette_sha256":hash(&palette),"frames":frames,"sources":sources}));
    }
    std::fs::write(
        out.join("export.json"),
        serde_json::to_vec_pretty(&json!({"actors":actors})).unwrap(),
    )
    .unwrap();
}
