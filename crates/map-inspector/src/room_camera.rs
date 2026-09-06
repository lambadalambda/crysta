//! Source-selected fixed house cameras and bounded exterior ordinary follow.
use crate::{invalid, Result};

#[derive(Debug, Clone, Copy)]
pub(super) enum Camera {
    Fixed([u16; 2]),
    Follow { maximum: [u16; 2] },
    Pandora(assets::maps::visual::pandora::SourceCamera),
}

impl Camera {
    #[allow(dead_code)] // Consumed by the parent opt-in host wiring.
    pub(super) fn compile_profile(image: &[u8], map: u16, pandora: bool) -> Result<Self> {
        if pandora && matches!(map, 0xa | 0xe | 0x13 | 0x20 | 0x21 | 0x41..=0x44) {
            return assets::maps::visual::pandora::SourceCamera::from_rom(image, map)
                .map(Self::Pandora)
                .map_err(Into::into);
        }
        Self::compile(image, map)
    }

    pub(super) fn compile(image: &[u8], map: u16) -> Result<Self> {
        if map != 10 {
            return crate::house_profiles::camera(image, map).map(Self::Fixed);
        }
        // Map A selector8 -> display profile96BC2F. It selects first-BG2/mode09
        // and a256-pixel clamp height, distinct from the224-pixel visible viewport.
        if image.get(0x03_89a7..0x03_89a9) != Some(&[0, 8])
            || image.get(0x16_bb74..0x16_bb76) != Some(&[0x2f, 0xbc])
            || image.get(0x16_bc33..0x16_bc36) != Some(&[0x64, 0xc0, 9])
        {
            return Err(invalid("unqualified exterior camera profile").into());
        }
        let bounds = image
            .get(0x16_be44..0x16_be46)
            .ok_or_else(|| invalid("truncated exterior camera source"))?;
        if bounds != [0x40, 0x40] {
            return Err(invalid("unqualified exterior camera bounds").into());
        }
        // Low nibbles select zero origin; high nibbles select four256-pixel pages.
        // Sheet height1280 is NOT the camera bottom1024.
        Ok(Self::Follow {
            maximum: [
                (u16::from(bounds[0] >> 4) - 1) * 256,
                (u16::from(bounds[1] >> 4) - 1) * 256,
            ],
        })
    }
    pub(super) fn at(self, position: (u16, u16)) -> [u16; 2] {
        match self {
            Self::Fixed(origin) => origin,
            Self::Pandora(source) => source.settled_origin([position.0, position.1]),
            Self::Follow { maximum } => [
                position.0.saturating_sub(128).min(maximum[0]),
                position.1.saturating_sub(112).min(maximum[1]),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opt_in_cameras_follow_source_sectors_without_broader_map_admission() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        if !path.try_exists().unwrap() {
            eprintln!("SKIP: local Japanese ROM absent");
            return;
        }
        let rom = rom::Rom::load(&std::fs::read(path).unwrap()).unwrap();
        for (map, position, expected) in [
            (0x13, (392, 207), [256, 0]),
            (0xe, (152, 880), [0, 768]),
            (0x20, (408, 880), [256, 768]),
            (0x21, (136, 368), [0, 256]),
            (0x41, (136, 208), [0, 0]),
            (0x42, (136, 464), [0, 256]),
            (0x43, (392, 208), [256, 0]),
            (0x44, (392, 464), [256, 256]),
        ] {
            assert!(Camera::compile_profile(rom.image(), map, false).is_err());
            let camera = Camera::compile_profile(rom.image(), map, true).unwrap();
            assert_eq!(camera.at(position), expected);
            let source =
                assets::maps::visual::pandora::SourceCamera::from_rom(rom.image(), map).unwrap();
            for position in [(0, 0), position, (u16::MAX, u16::MAX)] {
                assert_eq!(
                    camera.at(position),
                    source.settled_origin([position.0, position.1])
                );
            }
            let mut image = rom.image().to_vec();
            image[source.record_offset] ^= 1;
            assert!(Camera::compile_profile(&image, map, true).is_err());
        }
        for map in crate::house_profiles::MAPS.into_iter().chain([10]) {
            for position in [(0, 0), (304, 112), (538, 815)] {
                assert_eq!(
                    Camera::compile_profile(rom.image(), map, false)
                        .unwrap()
                        .at(position),
                    Camera::compile_profile(rom.image(), map, true)
                        .unwrap()
                        .at(position)
                );
            }
        }
        assert!(Camera::compile_profile(rom.image(), 0x45, true).is_err());
    }

    #[test]
    fn follow_matches_settled_reference_and_uses_256_high_camera_bounds() {
        let camera = Camera::Follow {
            maximum: [768, 768],
        };
        for (position, expected) in [
            ((504, 769), [376, 657]),
            ((504, 815), [376, 703]),
            ((538, 815), [410, 703]),
            ((0, 0), [0, 0]),
            ((1024, 1280), [768, 768]),
        ] {
            assert_eq!(camera.at(position), expected);
        }
        assert_eq!(Camera::Fixed([256, 512]).at((538, 815)), [256, 512]);
    }

    #[test]
    fn camera_decoder_checks_exterior_source_shape_and_rejects_gated_maps() {
        let mut image = vec![0; 0x16_bf00];
        image[0x03_89a7..0x03_89a9].copy_from_slice(&[0, 8]);
        image[0x16_bb74..0x16_bb76].copy_from_slice(&0xbc2fu16.to_le_bytes());
        image[0x16_bc33..0x16_bc36].copy_from_slice(&[0x64, 0xc0, 9]);
        image[0x16_be44..0x16_be46].copy_from_slice(&[0x40, 0x40]);
        assert_eq!(
            Camera::compile(&image, 10).unwrap().at((504, 769)),
            [376, 657]
        );
        for at in [
            0x03_89a8, 0x16_bb74, 0x16_bc33, 0x16_bc34, 0x16_bc35, 0x16_be44, 0x16_be45,
        ] {
            image[at] ^= 1;
            assert!(Camera::compile(&image, 10).is_err());
            image[at] ^= 1;
        }
        assert!(Camera::compile(&image[..0x16_be45], 10).is_err());
        assert!(Camera::compile(&image, 14).is_err());
    }
}
