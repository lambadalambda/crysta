//! ROM font pages and choice-label rasters; no conversation control flow.
use crate::{invalid, Result};
use assets::text::{Acknowledgement, DialogueChoice, DialoguePage, HouseDialogue, TEXT_SOURCES};
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
    pub(super) choice_contexts: Map<String, Value>,
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
    let mut palette = PALETTE;
    palette[usize::from(page.background_index())] = [0; 4];
    let rgba: Vec<_> = (y..y + height)
        .flat_map(|row| {
            (x..x + width)
                .flat_map(move |column| palette[usize::from(page.indexed()[row * stride + column])])
        })
        .collect();
    Ok(json!({"width":width,"height":height,"rgba":rgba}))
}

pub(super) fn compile(rom: &rom::Rom) -> Result<DialogueArt> {
    // Live house policy stays unchanged until the Pandora core/profile is admitted.
    compile_profile(rom, false)
}

pub(super) fn compile_profile(rom: &rom::Rom, include_pandora: bool) -> Result<DialogueArt> {
    let dialogue = HouseDialogue::from_rom(rom.image())?;
    let mut art = DialogueArt {
        pages: Map::new(),
        choices: Map::new(),
        requests: Map::new(),
        choice_contexts: Map::new(),
    };
    for source in TEXT_SOURCES {
        append_request(
            &mut art,
            source,
            dialogue
                .pages(source)
                .ok_or_else(|| invalid("missing source dialogue"))?,
        )?;
    }
    for (catalog, source) in [(0, assets::text::FIRST_TEXT), (1, 0x88_9156)] {
        let choice = dialogue
            .choice(catalog)
            .ok_or_else(|| invalid("missing source choice"))?;
        let pages = dialogue
            .pages(source)
            .ok_or_else(|| invalid("missing choice context"))?;
        let keys = append_choice(
            &mut art,
            source,
            pages,
            choice,
            &format!("choice:{catalog}"),
        )?;
        art.choices.insert(catalog.to_string(), json!(keys));
    }
    if include_pandora {
        let dialogue = assets::text::pandora::PandoraDialogue::from_rom(rom.image())?;
        for request in dialogue.requests() {
            append_request(&mut art, request.source(), request.pages())?;
            if let Some(catalog) = request.choice_catalog() {
                let choice = dialogue
                    .choice(catalog)
                    .ok_or_else(|| invalid("missing Pandora choice"))?;
                append_choice(
                    &mut art,
                    request.source(),
                    request.pages(),
                    choice,
                    &format!("choice:{:06x}", request.source()),
                )?;
            }
        }
    }
    Ok(art)
}

fn append_request(art: &mut DialogueArt, source: u32, pages: &[DialoguePage]) -> Result<()> {
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
    Ok(())
}

fn append_choice(
    art: &mut DialogueArt,
    source: u32,
    pages: &[DialoguePage],
    choice: &DialogueChoice,
    prefix: &str,
) -> Result<Vec<String>> {
    let tail = pages
        .last()
        .ok_or_else(|| invalid("missing choice context"))?;
    if tail.acknowledgement() != Acknowledgement::None {
        return Err(invalid("choice context must complete without page acknowledgement").into());
    }
    let mut keys = Vec::new();
    for option in choice.options {
        let key = format!("{prefix}:{}", option.result);
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
    art.choice_contexts.insert(
        key(page_key(source, pages.len() - 1)?),
        json!({"catalog":choice.catalog,"options":keys}),
    );
    Ok(keys)
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
    fn pandora_pages_keep_geometry_background_and_context_specific_choice_labels() {
        use assets::text::pandora::PandoraDialogue;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        if !path.try_exists().unwrap() {
            eprintln!("SKIP: local Japanese ROM absent");
            return;
        }
        let rom = rom::Rom::load(&std::fs::read(path).unwrap()).unwrap();
        let house = compile(&rom).unwrap();
        let art = compile_profile(&rom, true).unwrap();
        for (key, value) in &house.pages {
            assert_eq!(&art.pages[key], value);
        }
        assert_eq!(art.choices, house.choices); // Native catalog reuse cannot overwrite old labels.
        assert_eq!(art.pages.len(), 18 + 76 + 6);
        assert_eq!(art.requests.len(), 7 + 33);
        assert_eq!(art.choice_contexts.len(), 2 + 3);
        let source = PandoraDialogue::from_rom(rom.image()).unwrap();
        for request in source.requests() {
            for (index, page) in request.pages().iter().enumerate() {
                let id = request.page_id(index).unwrap();
                let raster = &art.pages[&key(id)];
                assert_eq!(raster["width"], page.width());
                assert_eq!(raster["height"], page.height());
                for (at, &pixel) in page.indexed().iter().enumerate() {
                    let color = if pixel == page.background_index() {
                        [0; 4]
                    } else {
                        PALETTE[usize::from(pixel)]
                    };
                    assert_eq!(
                        &raster["rgba"].as_array().unwrap()[at * 4..at * 4 + 4],
                        json!(color).as_array().unwrap()
                    );
                }
                assert_eq!(
                    art.requests[&format!("{:06x}", request.source())][index]["page_id"],
                    id
                );
            }
            if let Some(catalog) = request.choice_catalog() {
                let index = request.pages().len() - 1;
                let context = &art.choice_contexts[&key(request.page_id(index).unwrap())];
                assert_eq!(context["catalog"], catalog);
                let page = &request.pages()[index];
                for (index, option) in source.choice(catalog).unwrap().options.iter().enumerate() {
                    let label = &art.pages[context["options"][index].as_str().unwrap()];
                    let x = usize::from(option.position[0]) + 12;
                    let width = usize::from(page.width()) - x;
                    assert_eq!(label["width"], width);
                    assert_eq!(label["height"], 16);
                    for row in 0..16 {
                        for column in 0..width {
                            let pixel = page.indexed()[(usize::from(option.position[1]) + row)
                                * usize::from(page.width())
                                + x
                                + column];
                            let color = if pixel == page.background_index() {
                                [0; 4]
                            } else {
                                PALETTE[usize::from(pixel)]
                            };
                            let at = (row * width + column) * 4;
                            assert_eq!(
                                &label["rgba"].as_array().unwrap()[at..at + 4],
                                json!(color).as_array().unwrap()
                            );
                        }
                    }
                }
            }
        }
        assert_ne!(art.pages["choice:88b6c7:1"], art.pages["choice:889efc:1"]);
        assert_eq!(art.pages["choice:88b6c7:1"], art.pages["choice:88b722:1"]);
        assert_eq!(art.requests["88adf2"].as_array().unwrap().len(), 2);
        assert_eq!(art.requests["88b7e3"].as_array().unwrap().len(), 2);
        assert_eq!(art.requests["88b7e3"][0]["acknowledgement"], "next");
        assert_eq!(art.requests["88b7e3"][1]["acknowledgement"], "end");
    }

    #[test]
    fn numeric_keys_are_page_identifiers_not_adjusted_rom_addresses() {
        assert_eq!(key(page_key(0x88_8ff0, 1).unwrap()), "text:888ff0:1");
        assert!(page_key(0x100_0000, 0).is_err());
        assert!(page_key(0x88_8ff0, 16).is_err());
    }
}
