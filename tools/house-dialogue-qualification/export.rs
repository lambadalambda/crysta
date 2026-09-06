//! ROM-only dialogue export. No oracle/native event routing belongs in this tool.
use assets::text::{Acknowledgement, HouseDialogue, TEXT_SOURCES};
use serde_json::json;
fn sha(bytes: &[u8]) -> String {
    rom::digests(bytes)
        .sha256
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "export ROM local/OUTPUT");
    let out = std::path::Path::new(&args[2]);
    assert!(
        out.starts_with("local")
            && !out
                .components()
                .any(|c| c == std::path::Component::ParentDir)
    );
    std::fs::create_dir(out).unwrap();
    let image = rom::Rom::load(&std::fs::read(&args[1]).unwrap()).unwrap();
    let text = HouseDialogue::from_rom(image.image()).unwrap();
    let mut pages = Vec::new();
    for source in TEXT_SOURCES {
        for (index, page) in text.pages(source).unwrap().iter().enumerate() {
            let filename = format!("{source:06x}-{index}.indexed");
            std::fs::write(out.join(&filename), page.indexed()).unwrap();
            pages.push(json!({"text_source":source,"index":index,"width":page.width(),"height":page.height(),
                "file":filename,"sha256":sha(page.indexed()),"boundary_source":page.boundary_source(),
                "acknowledgement":match page.acknowledgement() { Acknowledgement::Next=>"next",Acknowledgement::End=>"end",Acknowledgement::None=>"none" },
                "glyphs":page.glyphs().iter().map(|g| json!({"text_source":g.text_source,
                    "font_source":g.font_source,"position":g.position})).collect::<Vec<_>>() }));
        }
    }
    std::fs::write(
        out.join("export.json"),
        serde_json::to_vec_pretty(&json!({
        "rom_sha256":sha(image.image()),"pages":pages,
        "choices":(0..2).map(|catalog| { let choice=text.choice(catalog).unwrap(); json!({
            "catalog":catalog,"options":choice.options.iter().map(|o|json!({"source":o.source,
                "result":o.result,"position":o.position,"neighbors":o.neighbors})).collect::<Vec<_>>()})
        }).collect::<Vec<_>>() }))
        .unwrap(),
    )
    .unwrap();
}
