//! Local full-layer export from ROM assets, without an emulator session.
use crate::{run_directory, Result};
use rom::Rom;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

const VIEWER: &str = include_str!("../web/static-viewer.html");

pub(super) fn render(rom: &Rom, id: u16) -> Result<map_inspector::RenderedBackground> {
    map_inspector::render_static_background(rom, id)
}

pub(super) fn pixel_rgb(
    index: u8,
    scene: &assets::maps::visual::StaticBackground,
    x: usize,
    y: usize,
) -> [u8; 3] {
    map_inspector::static_pixel_rgb(index, scene, x, y)
}

pub(super) fn export(rom: &Rom, id: u16) -> Result<PathBuf> {
    let rendered = render(rom, id)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/static-maps");
    let directory = run_directory(&root)?;
    fs::write(directory.join("map.bmp"), rendered.bitmap)?;
    fs::write(directory.join("indices.bin"), rendered.indices)?;
    fs::write(directory.join("priority.bin"), rendered.priorities)?;
    fs::write(
        directory.join("map.json"),
        serde_json::to_vec_pretty(&rendered.metadata)?,
    )?;
    fs::write(
        directory.join("index.html"),
        render_html(&rendered.metadata),
    )?;
    Ok(directory.canonicalize()?.join("index.html"))
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
    use serde_json::json;

    #[test]
    fn static_html_escapes_data() {
        let html = render_html(&json!({"limits":"</script><script>bad()</script>"}));
        assert!(!html.contains("__STATIC_JSON__"));
        assert!(!html.contains("</script><script>bad"));
        assert!(html.contains("\\u003c/script>"));
    }
}
