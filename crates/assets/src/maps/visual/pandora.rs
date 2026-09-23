//! Bounded source compiler for the Pandora route, separate from the static allowlist.

use super::{StaticBackground, VisualMapError};
use crate::maps::MapCell;
mod recipes;

/// Extent/meaning of the returned grid, independent of movement qualification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridPolicy {
    /// Every source cell with loader attributes, without actor stamps or script patches.
    FullSourceAttributed,
}
/// A decoded full sheet grants no movement coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionPolicy {
    /// Parent must supply its qualified route halo/material policy separately.
    CallerSuppliedHalo,
}
/// Runtime state is not inferred from map ID or copied from a capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicPhasePolicy {
    /// Parent applies source-qualified phase patches and occupancy to a copy of the base.
    CallerAppliedSourcePatches,
}
/// Explicit compiler limits transported to the parent host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundPolicies {
    /// Full-grid compilation extent.
    pub grid: GridPolicy,
    /// Independent movement admission obligation.
    pub admission: AdmissionPolicy,
    /// Independent runtime-state obligation.
    pub dynamic_phase: DynamicPhasePolicy,
}
/// Loading history represented by this compilation; not a reachability assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Initialization {
    /// Ordinary source map-load projection.
    MapLoad,
    /// Map21 shared colors, then controller41 graphics/palettes and map41 definitions;
    /// maps42–44 only replace the layer. Not a standalone standard map recipe.
    Map21ThenController41Tour,
}
/// ROM-only first resources, full attributed base and camera contract for the wider route.
#[derive(Debug)]
pub struct PandoraBackground {
    background: StaticBackground,
    grid: Vec<MapCell>,
    camera: SourceCamera,
    initialization: Initialization,
}
impl PandoraBackground {
    /// Compile A, E, 13, 20, 21 or the controller-initialized 41–44 tour.
    /// The caller authenticates the Japanese ROM. No native capture is an input.
    /// # Errors
    /// Rejects other maps, unaudited source shapes, invalid resource bounds/sizes,
    /// and tour cells referencing tiles outside the controller graphics transfer.
    pub fn from_rom(image: &[u8], map_id: u16) -> Result<Self, VisualMapError> {
        let camera = SourceCamera::from_rom(image, map_id)?;
        let (background, initialization) = recipes::compile(image, map_id)?;
        let expected = match map_id {
            0xa => (64, 80),
            0x13 => (64, 32),
            0xe | 0x20 => (32, 64),
            0x21 => (16, 32),
            _ => (32, 32),
        };
        if (background.layer().width(), background.layer().height()) != expected {
            return Err(VisualMapError::Unsupported(
                "unqualified Pandora full-sheet dimensions",
            ));
        }
        let attributes = background.resources()[3]
            .decoded()
            .try_into()
            .map_err(|_| VisualMapError::Unsupported("invalid attribute extent"))?;
        let grid = background.layer().attributed_cells(attributes);
        Ok(Self {
            background,
            grid,
            camera,
            initialization,
        })
    }
    /// Source resources, raw grid, natural palette and indexed/priority pixel sampling.
    #[must_use]
    pub const fn background(&self) -> &StaticBackground {
        &self.background
    }
    /// The background alone, for a caller that loads the map as the game does.
    #[must_use]
    pub fn into_background(self) -> StaticBackground {
        self.background
    }
    /// Full initialized grid, not a movement halo or a dynamic-phase snapshot.
    #[must_use]
    pub fn attributed_grid(&self) -> &[MapCell] {
        &self.grid
    }
    /// Source camera bounds/hardware assignment and ordinary settled clamp.
    #[must_use]
    pub const fn camera(&self) -> &SourceCamera {
        &self.camera
    }
    /// Source loading-history prerequisite.
    #[must_use]
    pub const fn initialization(&self) -> Initialization {
        self.initialization
    }
    /// Production use must honor these independent admission/state boundaries.
    #[must_use]
    pub const fn policies(&self) -> BackgroundPolicies {
        BackgroundPolicies {
            grid: GridPolicy::FullSourceAttributed,
            admission: AdmissionPolicy::CallerSuppliedHalo,
            dynamic_phase: DynamicPhasePolicy::CallerAppliedSourcePatches,
        }
    }
}

const CAMERAS: &[(u16, usize, u8, [u8; 2])] = &[
    (0xa, 0x03_89a7, 8, [0x40, 0x40]),
    (0xe, 0x03_8cfa, 0x1b, [0x10, 0x13]),
    (0x13, 0x03_8eab, 6, [0x11, 0x10]),
    (0x20, 0x03_923a, 0x1b, [0x11, 0x13]),
    (0x21, 0x03_9268, 0x1b, [0x10, 0x20]),
    (0x41, 0x03_9527, 3, [0x10, 0x10]),
    (0x42, 0x03_9569, 3, [0x10, 0x11]),
    (0x43, 0x03_95b7, 3, [0x11, 0x10]),
    (0x44, 0x03_95f4, 3, [0x11, 0x11]),
];
const DISPLAYS: &[(u8, usize, [u8; 9])] = &[
    (3, 0x16_bc02, [0x15, 0, 0x80, 0, 0x64, 0, 9, 0, 0]),
    (
        6,
        0x16_bc1d,
        [0x17, 0x12, 0x82, 0x21, 0x64, 0x80, 9, 0x11, 0x11],
    ),
    (
        8,
        0x16_bc2f,
        [0x16, 1, 0x82, 0x33, 0x64, 0xc0, 9, 0xed, 0x13],
    ),
    (0x1b, 0x16_bcda, [0x15, 0, 0x20, 0xb3, 0x64, 0, 9, 0, 0]),
];

fn expect(image: &[u8], at: usize, bytes: &[u8]) -> Result<(), VisualMapError> {
    if image.get(at..at + bytes.len()) != Some(bytes) {
        return Err(VisualMapError::Unsupported(
            "unqualified Pandora source profile",
        ));
    }
    Ok(())
}

/// ROM-derived transport, not a camera animation scheduler. Coordinates are pixels.
/// The ordinary settled clamp does not model transition interpolation or forced pans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceCamera {
    /// Normalized two-byte camera table source.
    pub record_offset: usize,
    /// Retained source record.
    pub record: [u8; 2],
    /// Left, top, right, bottom in map pixels (exclusive right/bottom).
    pub bounds: [u16; 4],
    /// Normalized scene and nine-byte display-profile sources.
    pub scene_offset: usize,
    /// Display table target.
    pub display_offset: usize,
    /// Qualified source display bytes; runtime color effects may change shadows.
    pub display: [u8; 9],
    /// Source $0866: vertical clamp extent, NOT the 224-line viewport height.
    pub vertical_extent: u16,
    /// First source layer is hardware BG1 or BG2 according to the display swap bit.
    pub hardware_background: u8,
    /// VRAM word address of the first layer's 32x32 tile ring.
    pub ring_word_base: u16,
    /// BGMODE source write.
    pub bgmode: u8,
}
impl SourceCamera {
    /// Reads only audited Pandora scene/display/camera records.
    /// Caller authenticates the Japanese ROM; no original CPU execution.
    /// # Errors
    /// Rejects unqualified maps, modified records/tables and truncated input.
    pub fn from_rom(image: &[u8], map_id: u16) -> Result<Self, VisualMapError> {
        let &(_, scene, selector, record) =
            CAMERAS
                .iter()
                .find(|r| r.0 == map_id)
                .ok_or(VisualMapError::Unsupported(
                    "unqualified Pandora camera map",
                ))?;
        expect(image, 0x28000 + usize::from(map_id) * 2, &[0, 0])?;
        expect(
            image,
            0x38000 + usize::from(map_id) * 2,
            &scene.to_le_bytes()[..2],
        )?;
        expect(image, scene, &[0, selector])?;
        let &(_, display_offset, display) = DISPLAYS
            .iter()
            .find(|r| r.0 == selector)
            .ok_or(VisualMapError::Unsupported("unaudited display selector"))?;
        expect(
            image,
            0x16_bb64 + usize::from(selector) * 2,
            &display_offset.to_le_bytes()[..2],
        )?;
        expect(image, display_offset, &display)?;
        let region = super::camera::CameraRegion::from_rom(image, map_id)?;
        expect(image, region.record_offset, &record)?;
        Ok(Self {
            record_offset: region.record_offset,
            record,
            bounds: region.bounds,
            scene_offset: scene,
            display_offset,
            display,
            vertical_extent: region.vertical_extent,
            hardware_background: if display[5] & 0x80 != 0 { 2 } else { 1 },
            ring_word_base: 0x3800,
            bgmode: display[6],
        })
    }
    /// Ordinary source follow/clamp at a settled state; do not use for in-flight pans.
    /// Use the unmodified contract returned by [`Self::from_rom`].
    /// # Panics
    /// Caller-manufactured bounds may panic if they do not contain a 256-pixel-wide
    /// region of at least `vertical_extent` height. ROM-derived contracts satisfy this.
    #[must_use]
    pub fn settled_origin(&self, player: [u16; 2]) -> [u16; 2] {
        super::camera::clamp_origin(self.bounds, self.vertical_extent, player)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn camera_fixture() -> Vec<u8> {
        let mut image = vec![0; 0x17_0000];
        for &(id, scene, selector, record) in CAMERAS {
            image[0x38000 + id as usize * 2..0x38002 + id as usize * 2]
                .copy_from_slice(&scene.to_le_bytes()[..2]);
            image[scene..scene + 2].copy_from_slice(&[0, selector]);
            image[0x16_be30 + id as usize * 2..0x16_be32 + id as usize * 2]
                .copy_from_slice(&record);
        }
        for &(selector, offset, bytes) in DISPLAYS {
            image[0x16_bb64 + selector as usize * 2..0x16_bb66 + selector as usize * 2]
                .copy_from_slice(&offset.to_le_bytes()[..2]);
            image[offset..offset + 9].copy_from_slice(&bytes);
        }
        image
    }

    #[test]
    fn cameras_transport_source_bounds_hardware_and_clamps() {
        let image = camera_fixture();
        for &(id, _, selector, record) in CAMERAS {
            let c = SourceCamera::from_rom(&image, id).unwrap();
            assert_eq!(c.record, record);
            assert_eq!(c.vertical_extent, 256);
            assert_eq!(
                c.hardware_background,
                if matches!(selector, 6 | 8) { 2 } else { 1 }
            );
            assert_eq!(c.ring_word_base, 0x3800);
            assert_eq!(c.bgmode, 9);
        }
        let a = SourceCamera::from_rom(&image, 10).unwrap();
        assert_eq!(a.settled_origin([364, 815]), [236, 703]);
        assert_eq!(a.settled_origin([0, 0]), [0, 0]);
        assert_eq!(a.settled_origin([65535, 65535]), [768, 768]);
        let e = SourceCamera::from_rom(&image, 14).unwrap();
        assert_eq!(e.settled_origin([144, 864]), [0, 768]);
        let box_room = SourceCamera::from_rom(&image, 33).unwrap();
        assert_eq!(box_room.settled_origin([128, 359]), [0, 247]);
        assert_eq!(box_room.settled_origin([128, 500]), [0, 256]);
    }

    #[test]
    fn camera_source_controls_reject_mutation_truncation_and_gated_maps() {
        let good = camera_fixture();
        for &(id, scene, selector, _) in CAMERAS {
            let (_, display, _) = DISPLAYS.iter().find(|x| x.0 == selector).unwrap();
            for at in (scene..scene + 2)
                .chain(0x28000 + id as usize * 2..0x28002 + id as usize * 2)
                .chain(0x38000 + id as usize * 2..0x38002 + id as usize * 2)
                .chain(0x16_be30 + id as usize * 2..0x16_be32 + id as usize * 2)
                .chain(0x16_bb64 + selector as usize * 2..0x16_bb66 + selector as usize * 2)
                .chain(*display..*display + 9)
            {
                let mut image = good.clone();
                image[at] ^= 1;
                assert!(
                    SourceCamera::from_rom(&image, id).is_err(),
                    "map {id:x} at {at:x}"
                );
                assert!(SourceCamera::from_rom(&good[..at], id).is_err());
            }
        }
        for id in [0, 11, 12, 13, 0x128, 0xffff] {
            assert!(SourceCamera::from_rom(&good, id).is_err());
        }
    }
}

#[cfg(test)]
mod compiler_tests;
