//! Host-free, source-derived Pandora preview compiler and simulation.

use std::io;

mod house_navigation;
mod house_profiles;
mod house_progression;
mod new_game;
#[allow(dead_code)]
mod pandora_navigation;
mod pandora_progression;
mod room_art;
mod room_camera;
mod room_dialogue;
mod room_preview;
#[path = "static_background.rs"]
mod visual_export;

/// Error returned while authenticating or compiling preview data.
pub type Error = Box<dyn std::error::Error>;
pub(crate) type Result<T> = std::result::Result<T, Error>;

/// The accepted Pandora-enabled preview, backed entirely by owned memory.
pub use room_preview::Preview as PandoraPreview;
/// Pure rendered-background output used by the native exporter.
#[doc(hidden)]
pub use visual_export::{render as render_static_background, RenderedBackground};

/// Apply the renderer's exact palette/checkerboard policy to one source pixel.
#[doc(hidden)]
#[must_use]
pub fn static_pixel_rgb(
    index: u8,
    scene: &assets::maps::visual::StaticBackground,
    x: usize,
    y: usize,
) -> [u8; 3] {
    visual_export::pixel_rgb(index, scene, x, y)
}

/// Run the retained native room semantic verification through the library path.
#[doc(hidden)]
pub fn verify_room(rom: &rom::Rom) -> Result<serde_json::Value> {
    room_preview::verify(rom)
}

/// Run the retained native house semantic verification through the library path.
#[doc(hidden)]
pub fn verify_house(rom: &rom::Rom) -> Result<serde_json::Value> {
    room_preview::verify_house(rom)
}

pub(crate) fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    rom::digests(bytes)
        .sha256
        .iter()
        .fold(String::new(), |mut text, byte| {
            write!(text, "{byte:02x}").expect("writing to String cannot fail");
            text
        })
}

pub(crate) fn bitmap(rgb: &[u8], width: usize, height: usize) -> Result<Vec<u8>> {
    if width == 0 || height == 0 || width > 4096 || height > 4096 || rgb.len() != width * height * 3
    {
        return Err(invalid("invalid RGB bitmap dimensions or data length").into());
    }
    let stride = (width * 3 + 3) & !3;
    let size = 54 + stride * height;
    let mut bytes = vec![0; size];
    bytes[..2].copy_from_slice(b"BM");
    for (offset, value) in [
        (2, size),
        (10, 54),
        (14, 40),
        (18, width),
        (34, stride * height),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&u32::try_from(value)?.to_le_bytes());
    }
    bytes[22..26].copy_from_slice(&(-i32::try_from(height)?).to_le_bytes());
    bytes[26..28].copy_from_slice(&1_u16.to_le_bytes());
    bytes[28..30].copy_from_slice(&24_u16.to_le_bytes());
    for y in 0..height {
        for x in 0..width {
            let from = (y * width + x) * 3;
            let to = 54 + y * stride + x * 3;
            bytes[to..to + 3].copy_from_slice(&[rgb[from + 2], rgb[from + 1], rgb[from]]);
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn bitmap_encodes_rgb_in_top_down_bgr_rows() {
        let bytes = super::bitmap(&[255, 0, 0, 0, 255, 0], 2, 1).unwrap();
        assert_eq!(&bytes[..2], b"BM");
        assert_eq!(bytes.len(), 62);
        assert_eq!(&bytes[54..], [0, 0, 255, 0, 255, 0, 0, 0]);
        assert_eq!(i32::from_le_bytes(bytes[22..26].try_into().unwrap()), -1);
        assert!(super::bitmap(&[0], 2, 1).is_err());
        assert!(super::bitmap(&[], 0, 0).is_err());
    }
}
