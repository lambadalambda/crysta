//! Pure static-background rendering shared by native export and portable preview.
use crate::{bitmap, sha256, Result};
use assets::{graphics::IndexedPixel, maps::visual::StaticBackground};
use rom::Rom;
use serde_json::{json, Value};

const LIMITS: &str = "Static first background only, for allowlisted Japanese maps $000F, $0010 and $0128: natural full-brightness ROM palette, checkerboard transparency. No sprites, animation, windows, color math, brightness effects, cached graphics transfers, BG2 or multi-layer composition. Room profiles recognize fixed script shapes without evaluating conditional audio state. Cell high-bit collision semantics are not inferred.";

/// Complete in-memory output of the static-background renderer.
#[doc(hidden)]
pub struct RenderedBackground {
    /// Encoded top-down 24-bit BMP.
    pub bitmap: Vec<u8>,
    /// Palette index for every pixel.
    pub indices: Vec<u8>,
    /// Priority bit for every pixel.
    pub priorities: Vec<u8>,
    /// Existing static-viewer metadata.
    pub metadata: Value,
}

/// Render one allowlisted source background without host services.
///
/// # Errors
///
/// Returns an error when source validation, decoding, or bitmap encoding fails.
#[doc(hidden)]
pub fn render(rom: &Rom, id: u16) -> Result<RenderedBackground> {
    let scene = StaticBackground::from_rom(rom.image(), id)?;
    let width = scene.layer().width() * 16;
    let height = scene.layer().height() * 16;
    let mut indices = Vec::with_capacity(width * height);
    let mut priorities = Vec::with_capacity(width * height);
    let mut rgb = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            let (index, priority) = match scene.pixel(x, y)? {
                IndexedPixel::Transparent => (0, false),
                IndexedPixel::Opaque {
                    palette_index,
                    priority,
                } => (palette_index, priority),
            };
            indices.push(index);
            priorities.push(u8::from(priority));
            rgb.extend(pixel_rgb(index, &scene, x, y));
        }
    }
    let mut resources: Vec<_> = scene
        .resources()
        .iter()
        .map(|resource| json!({
            "kind":format!("{:?}",resource.kind()),"source_range":[resource.source_range().start,resource.source_range().end],
            "source_sha256":sha256(resource.source_bytes()),"decoded_sha256":sha256(resource.decoded())
        }))
        .collect();
    resources.push(json!({"kind":"Layer","source_range":[scene.layer().source_range().start,scene.layer().source_range().end],
        "source_sha256":sha256(scene.layer().source_bytes()),"decoded_sha256":sha256(&scene.layer().layer_bytes())}));
    let metadata = json!({
        "schema_version":1,"kind":if id == 0x128 { "static-cavern-background" } else { "static-room-background" },"map_id":id,"revision":rom.revision().id(),
        "rom_sha256":sha256(rom.image()),"width":scene.layer().width(),"height":scene.layer().height(),
        "image":"map.bmp","cells":scene.layer().cells().iter().map(|c|c.raw()).collect::<Vec<_>>(),
        "metatiles":scene.metatiles().iter().map(|words|words.map(assets::graphics::BgTileWord::raw)).collect::<Vec<_>>(),
        "palette":scene.palette().iter().map(|c|c.raw()).collect::<Vec<_>>(),"resources":resources,
        "indexed_sha256":sha256(&indices),"priority_sha256":sha256(&priorities),"rgb_sha256":sha256(&rgb),
        "indices":"indices.bin","priorities":"priority.bin","limits":LIMITS
    });
    Ok(RenderedBackground {
        bitmap: bitmap(&rgb, width, height)?,
        indices,
        priorities,
        metadata,
    })
}

pub(crate) fn pixel_rgb(index: u8, scene: &StaticBackground, x: usize, y: usize) -> [u8; 3] {
    if index == 0 {
        checker(x, y)
    } else {
        scene.palette()[usize::from(index)].rgb8()
    }
}

fn checker(x: usize, y: usize) -> [u8; 3] {
    if (x / 8 + y / 8).is_multiple_of(2) {
        [24, 34, 40]
    } else {
        [36, 47, 55]
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn checker_is_map_anchored() {
        assert_eq!(super::checker(0, 0), super::checker(8, 8));
        assert_ne!(super::checker(0, 0), super::checker(8, 0));
    }
}
