//! Local full-layer export from ROM assets, without an emulator session.
use crate::{bitmap, invalid, run_directory, sha256, Result};
use assets::{graphics::IndexedPixel, maps::visual::CavernBackground};
use rom::Rom;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const VIEWER: &str = include_str!("../web/static-viewer.html");
pub(super) const LIMITS:&str="Static portal-cavern background only: natural full-brightness ROM palette, checkerboard transparency. No sprites, animation, windows, color math, brightness effects, cached graphics transfers or multi-layer composition. Cell high-bit collision semantics are not inferred.";

pub(super) fn export(rom: &Rom, id: u16) -> Result<PathBuf> {
    if id != 0x128 {
        return Err(invalid("static rendering is qualified only for map $0128").into());
    }
    let scene = CavernBackground::from_rom(rom.image())?;
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
    let mut resources:Vec<_>=scene.resources().iter().map(|resource|json!({
        "kind":format!("{:?}",resource.kind()),"source_range":[resource.source_range().start,resource.source_range().end],
        "source_sha256":sha256(resource.source_bytes()),"decoded_sha256":sha256(resource.decoded())
    })).collect();
    resources.push(json!({"kind":"Layer","source_range":[scene.layer().source_range().start,scene.layer().source_range().end],
        "source_sha256":sha256(scene.layer().source_bytes()),"decoded_sha256":sha256(&scene.layer().layer_bytes())}));
    let metadata = json!({
        "schema_version":1,"kind":"static-cavern-background","map_id":id,"revision":rom.revision().id(),
        "rom_sha256":sha256(rom.image()),"width":scene.layer().width(),"height":scene.layer().height(),
        "image":"map.bmp","cells":scene.layer().cells().iter().map(|c|c.raw()).collect::<Vec<_>>(),
        "metatiles":scene.metatiles().iter().map(|words|words.map(assets::graphics::BgTileWord::raw)).collect::<Vec<_>>(),
        "palette":scene.palette().iter().map(|c|c.raw()).collect::<Vec<_>>(),"resources":resources,
        "indexed_sha256":sha256(&indices),"priority_sha256":sha256(&priorities),"rgb_sha256":sha256(&rgb),
        "indices":"indices.bin","priorities":"priority.bin","limits":LIMITS
    });
    let bmp = bitmap(&rgb, width, height)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/static-maps");
    let directory = run_directory(&root)?;
    fs::write(directory.join("map.bmp"), bmp)?;
    fs::write(directory.join("indices.bin"), indices)?;
    fs::write(directory.join("priority.bin"), priorities)?;
    fs::write(
        directory.join("map.json"),
        serde_json::to_vec_pretty(&metadata)?,
    )?;
    fs::write(directory.join("index.html"), render_html(&metadata))?;
    Ok(directory.canonicalize()?.join("index.html"))
}
fn pixel_rgb(index: u8, scene: &CavernBackground, x: usize, y: usize) -> [u8; 3] {
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
fn render_html(metadata: &Value) -> String {
    VIEWER.replace(
        "__STATIC_JSON__",
        &metadata.to_string().replace('<', "\\u003c"),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn static_html_escapes_data_and_checker_is_map_anchored() {
        let html = render_html(&json!({"limits":"</script><script>bad()</script>"}));
        assert!(!html.contains("__STATIC_JSON__"));
        assert!(!html.contains("</script><script>bad"));
        assert!(html.contains("\\u003c/script>"));
        assert_eq!(checker(0, 0), checker(8, 8));
        assert_ne!(checker(0, 0), checker(8, 0));
    }
}
