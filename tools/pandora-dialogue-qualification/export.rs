//! ROM-only Pandora text/font export. Raw pages remain under ignored local/.
use assets::text::{
    pandora::{PandoraDialogue, DIRECT_INVOCATIONS},
    Acknowledgement,
};
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
    let text = PandoraDialogue::from_rom(image.image()).unwrap();
    let mut pages = Vec::new();
    let mut requests = Vec::new();
    for request in text.requests() {
        let source = request.source();
        let mut page_ids = Vec::new();
        for (index, page) in request.pages().iter().enumerate() {
            let page_id = request.page_id(index).unwrap();
            page_ids.push(page_id);
            let filename = format!("{source:06x}-{index}.indexed");
            std::fs::write(out.join(&filename), page.indexed()).unwrap();
            pages.push(json!({"text_source":source,"index":index,"page_id":page_id,
                "width":page.width(),"height":page.height(),"background_index":page.background_index(),"file":filename,"sha256":sha(page.indexed()),
                "boundary_source":page.boundary_source(),"acknowledgement":match page.acknowledgement() {
                    Acknowledgement::Next=>"next",Acknowledgement::End=>"end",Acknowledgement::None=>"none" },
                "glyphs":page.glyphs().iter().map(|g| json!({"text_source":g.text_source,
                    "font_source":g.font_source,"position":g.position})).collect::<Vec<_>>() }));
        }
        requests.push(
            json!({"source":source,"choice_catalog":request.choice_catalog(),"page_ids":page_ids}),
        );
    }
    let choice = text.choice(1).unwrap();
    std::fs::write(out.join("export.json"), serde_json::to_vec_pretty(&json!({
        "rom_sha256":sha(image.image()),"requests":requests,"pages":pages,
        "invocations":DIRECT_INVOCATIONS.iter().map(|(site,source)|json!({"site":site,"source":source})).collect::<Vec<_>>(),
        "choices":[{"catalog":choice.catalog,"options":choice.options.iter().map(|o|json!({"source":o.source,
            "result":o.result,"position":o.position,"neighbors":o.neighbors})).collect::<Vec<_>>()}]
    })).unwrap()).unwrap();
}
