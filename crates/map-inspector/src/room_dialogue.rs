//! ROM font pages and choice-label rasters; no conversation control flow.
use crate::{invalid, Result};
use assets::text::{Acknowledgement, HouseDialogue, TEXT_SOURCES};
use serde_json::{json, Map, Value};

// Presentation policy: native font indices, high contrast instead of SNES windows/colors.
const PALETTE: [[u8; 4]; 4] = [
    [0, 0, 0, 255],
    [240, 244, 248, 255],
    [36, 47, 55, 255],
    [0, 0, 0, 0],
];

pub(super) struct DialogueArt {
    pub(super) pages: Map<String, Value>,
    pub(super) choices: Map<String, Value>,
    pub(super) requests: Map<String, Value>,
}

/// Pack a logical (source request, page ordinal), NOT a CPU pointer adjustment.
pub(super) fn page_key(source: u32, index: usize) -> Result<u32> {
    if source > 0xff_ffff || index >= 16 {
        return Err(invalid("dialogue page key outside bounded identity").into());
    }
    Ok((source << 4) | u32::try_from(index)?)
}
pub(super) fn key(page: u32) -> String {
    format!("text:{:06x}:{}", page >> 4, page & 15)
}

fn raster(page: &assets::text::DialoguePage, rect: [usize; 4]) -> Result<Value> {
    let [x, y, width, height] = rect;
    let stride = usize::from(page.width());
    if width == 0 || height == 0 || x + width > stride || y + height > usize::from(page.height()) {
        return Err(invalid("dialogue crop outside source page").into());
    }
    let rgba: Vec<_> = (y..y + height)
        .flat_map(|row| {
            (x..x + width)
                .flat_map(move |column| PALETTE[usize::from(page.indexed()[row * stride + column])])
        })
        .collect();
    Ok(json!({"width":width,"height":height,"rgba":rgba}))
}

pub(super) fn compile(rom: &rom::Rom) -> Result<DialogueArt> {
    let dialogue = HouseDialogue::from_rom(rom.image())?;
    let mut art = DialogueArt {
        pages: Map::new(),
        choices: Map::new(),
        requests: Map::new(),
    };
    for source in TEXT_SOURCES {
        let pages = dialogue
            .pages(source)
            .ok_or_else(|| invalid("missing source dialogue"))?;
        let mut request = Vec::new();
        for (index, page) in pages.iter().enumerate() {
            let page_id = page_key(source, index)?;
            let key = key(page_id);
            art.pages.insert(
                key.clone(),
                raster(
                    page,
                    [0, 0, usize::from(page.width()), usize::from(page.height())],
                )?,
            );
            request.push(json!({"key":key,"page_id":page_id,"boundary_source":page.boundary_source(),
                "glyph_count":page.glyphs().len(),"acknowledgement":match page.acknowledgement() {
                    Acknowledgement::Next=>"next",Acknowledgement::End=>"end",Acknowledgement::None=>"none",
                }}));
        }
        art.requests.insert(format!("{source:06x}"), json!(request));
    }
    for (catalog, source) in [(0, assets::text::FIRST_TEXT), (1, 0x88_9156)] {
        let choice = dialogue
            .choice(catalog)
            .ok_or_else(|| invalid("missing source choice"))?;
        let tail = dialogue
            .pages(source)
            .and_then(|pages| pages.last())
            .ok_or_else(|| invalid("missing choice context"))?;
        if tail.acknowledgement() != Acknowledgement::None {
            return Err(
                invalid("choice context must complete without page acknowledgement").into(),
            );
        }
        let mut keys = Vec::new();
        for option in choice.options {
            let key = format!("choice:{catalog}:{}", option.result);
            let x = usize::from(option.position[0]) + 12;
            let width = usize::from(tail.width())
                .checked_sub(x)
                .ok_or_else(|| invalid("invalid choice label origin"))?;
            art.pages.insert(
                key.clone(),
                raster(tail, [x, usize::from(option.position[1]), width, 16])?,
            );
            keys.push(key);
        }
        art.choices.insert(catalog.to_string(), json!(keys));
    }
    Ok(art)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_pages_and_choice_labels_keep_real_acknowledgement_boundaries() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        if !path.try_exists().unwrap() {
            eprintln!("SKIP: local Japanese ROM absent");
            return;
        }
        let rom = rom::Rom::load(&std::fs::read(path).unwrap()).unwrap();
        let art = compile(&rom).unwrap();
        assert_eq!(art.pages.len(), 18); // 14 original pages + four option crops.
        let source = HouseDialogue::from_rom(rom.image()).unwrap();
        for id in TEXT_SOURCES {
            let pages = source.pages(id).unwrap();
            for (index, page) in pages.iter().enumerate() {
                let key = key(page_key(id, index).unwrap());
                assert_eq!(art.pages[&key]["width"], 224);
                assert_eq!(art.pages[&key]["height"], 48);
                let pixels = art.pages[&key]["rgba"].as_array().unwrap();
                for (at, &pixel) in page.indexed().iter().enumerate() {
                    assert_eq!(
                        &pixels[at * 4..at * 4 + 4],
                        &json!(PALETTE[usize::from(pixel)]).as_array().unwrap()[..]
                    );
                }
            }
        }
        assert_eq!(art.requests["888ff0"][0]["acknowledgement"], "next");
        assert_eq!(art.requests["888ff0"][1]["acknowledgement"], "none");
        assert_eq!(art.requests["889156"][0]["acknowledgement"], "none");
        for (catalog, prompt) in [(0, 0x88_8ff0), (1, 0x88_9156)] {
            let pages = source.pages(prompt).unwrap();
            let page = pages.last().unwrap();
            assert_eq!(
                art.choices[&catalog.to_string()].as_array().unwrap().len(),
                2
            );
            for result in 1..=2 {
                let label = &art.pages[&format!("choice:{catalog}:{result}")];
                assert_eq!(label["width"], 212);
                assert_eq!(label["height"], 16);
                for y in 0..16 {
                    for x in 0..212 {
                        let expected = page.indexed()[(result * 16 + y) * 224 + x + 12];
                        let at = (y * 212 + x) * 4;
                        assert_eq!(
                            &label["rgba"].as_array().unwrap()[at..at + 4],
                            &json!(PALETTE[usize::from(expected)]).as_array().unwrap()[..]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn numeric_keys_are_page_identifiers_not_adjusted_rom_addresses() {
        assert_eq!(key(page_key(0x88_8ff0, 1).unwrap()), "text:888ff0:1");
        assert!(page_key(0x100_0000, 0).is_err());
        assert!(page_key(0x88_8ff0, 16).is_err());
    }
}
