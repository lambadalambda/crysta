//! Source-only export; raw products must stay in ignored local/.
use assets::sprites::{HousePoseKey, PandoraGraphicsKey, PandoraSprites, SpritePixel};
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
    let bytes = std::fs::read(&args[1]).unwrap();
    let rom = rom::Rom::load(&bytes).unwrap();
    assert_eq!(rom.revision(), rom::Revision::Japan);
    let sprites = PandoraSprites::from_rom(rom.image()).unwrap();
    let _: &[assets::sprites::PandoraMotion] = sprites.motions();
    let mut arts = Vec::new();
    for art in sprites.art() {
        let dir = out.join(format!("{:06x}", art.source_id()));
        std::fs::create_dir(&dir).unwrap();
        let tiles: Vec<_> = art
            .graphics()
            .iter()
            .flat_map(|t| t.pixels().iter().copied())
            .collect();
        let palette: Vec<_> = art
            .palette()
            .iter()
            .flat_map(|c| c.raw().to_le_bytes())
            .collect();
        std::fs::write(dir.join("tiles"), &tiles).unwrap();
        std::fs::write(dir.join("palette"), &palette).unwrap();
        let mut lists = Vec::new();
        for list in art.lists() {
            let mut frames = Vec::new();
            for (index, frame) in list.frames().iter().enumerate() {
                let mut mirrors = Vec::new();
                for flip in [false, true] {
                    let c = frame.composition();
                    let (l, t, r, b) = c.bounds(flip, false);
                    let mut indexed = Vec::new();
                    let mut rgba = Vec::new();
                    let mut priority = Vec::new();
                    for y in t..b {
                        for x in l..r {
                            match c.sample(art.graphics(), flip, false, x, y).unwrap() {
                                SpritePixel::Transparent => {
                                    indexed.push(0);
                                    rgba.extend([0; 4]);
                                    priority.push(255);
                                }
                                SpritePixel::Opaque {
                                    palette_index,
                                    priority: p,
                                    ..
                                } => {
                                    indexed.push(palette_index);
                                    rgba.extend(
                                        art.palette()
                                            [usize::from(palette_index - art.palette_base())]
                                        .rgb8(),
                                    );
                                    rgba.push(255);
                                    priority.push(p);
                                }
                            }
                        }
                    }
                    let name = format!("{}-{index}-{}", list.selector(), u8::from(flip));
                    for (suffix, data) in [
                        ("indexed", indexed.as_slice()),
                        ("rgba", rgba.as_slice()),
                        ("priority", priority.as_slice()),
                    ] {
                        std::fs::write(dir.join(format!("{name}-{suffix}")), data).unwrap();
                    }
                    mirrors.push(json!({"flip":flip,"bounds":[l,t,r,b],"opaque":indexed.iter().filter(|&&p|p!=0).count(),"indexed_sha256":hash(&indexed),"rgba_sha256":hash(&rgba),"priority_sha256":hash(&priority)}));
                }
                let c = frame.composition().source_bytes();
                let raw = frame.source_composition().source_bytes();
                let name = format!("{}-{index}", list.selector());
                std::fs::write(dir.join(format!("{name}-composition")), c).unwrap();
                std::fs::write(dir.join(format!("{name}-source")), raw).unwrap();
                let key = match frame.key() {
                    HousePoseKey::Direct(p) => json!({"direct":p}),
                    HousePoseKey::Compressed { packet, offset } => {
                        json!({"packet":packet,"offset":offset})
                    }
                };
                frames.push(json!({"index":index,"key":key,"duration":frame.duration(),"facing":frame.facing(),"composition_sha256":hash(c),"source_sha256":hash(raw),"mirrors":mirrors}));
            }
            lists.push(json!({"selector":list.selector(),"frames":frames}));
        }
        let graphics = match art.graphics_key() {
            PandoraGraphicsKey::Direct(p) => json!({"direct":p}),
            PandoraGraphicsKey::Compressed(p) => json!({"packet":p}),
            PandoraGraphicsKey::HeldTile { top, bottom } => {
                json!({"held_top":top,"held_bottom":bottom})
            }
        };
        arts.push(json!({"id":art.source_id(),"graphics":graphics,"palette_base":art.palette_base(),"tiles_sha256":hash(&tiles),"palette_sha256":hash(&palette),"lists":lists}));
    }
    let ranges: Vec<_> = sprites
        .source_ranges()
        .iter()
        .map(|r| json!({"start":r.start,"end":r.end,"sha256":hash(&rom.image()[r.clone()])}))
        .collect();
    let phases:Vec<_>=sprites.phases().iter().map(|p|json!({"id":p.id(),"map":p.map_id(),"ark_tie_rank":p.ark_tie_rank(),"limits":p.limits().iter().map(|v|format!("{v:?}")).collect::<Vec<_>>(),"actors":p.actors().iter().map(|a|json!({"source":a.source_id,"art":a.art_id,"position":a.position,"position_source":a.position_source,"selector":a.selector,"hflip":a.hflip,"tie_rank":a.tie_rank,"priority_override":a.priority_override})).collect::<Vec<_>>()})).collect();
    let motions:Vec<_>=sprites.motions().iter().map(|m|json!({"id":m.id,"actor":m.actor,"source":m.source,"from":m.from,"to":m.to,"selector":m.selector,"motion_operand":m.motion_operand,"removes_actor":m.removes_actor})).collect();
    std::fs::write(
        out.join("export.json"),
        serde_json::to_vec_pretty(
            &json!({"rom_sha256":hash(rom.image()),"art":arts,"sources":ranges,"phases":phases,"motions":motions}),
        )
        .unwrap(),
    )
    .unwrap();
}
