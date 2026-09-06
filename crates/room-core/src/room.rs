use crate::{Direction, Unqualified};
use alloc::vec::Vec;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Material {
    Open,
    Solid,
    Partial,
}

/// Immutable row-major raw collision grid. Construction validates shape, not materials.
///
/// Unknown material cells may exist elsewhere in the room; touching one during
/// movement fails closed. Preserve raw bit 15 instead of masking it during decode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Room {
    width: u16,
    height: u16,
    cells: Vec<u16>,
    passive_flags: bool,
    sample_halo: Option<[u16; 4]>,
}

impl Room {
    /// Takes ownership of a grid, without copying or normalizing raw cells.
    ///
    /// # Errors
    /// Rejects zero/overflowing pixel dimensions and an incorrect cell count.
    pub fn new(width: u16, height: u16, cells: Vec<u16>) -> Result<Self, Unqualified> {
        if width == 0
            || height == 0
            || width.checked_mul(16).is_none()
            || height.checked_mul(16).is_none()
        {
            return Err(Unqualified::RoomDimensions);
        }
        if usize::from(width).checked_mul(usize::from(height)) != Some(cells.len()) {
            return Err(Unqualified::CellCount);
        }
        Ok(Self {
            width,
            height,
            cells,
            passive_flags: false,
            sample_halo: None,
        })
    }
    /// Constructs a grid admitting bit-15 cells as passive class-3 solids.
    ///
    /// The caller must establish ordinary, action-free movement: native collision
    /// action hooks must be inactive (reference `$0980 & $0050 == 0`). This is a
    /// collision policy assertion, not a simulation of those hooks. Do not use it
    /// for interactions, attacks, pushing, or other player/controller modes.
    /// Stored old-edge slopes 6/7 remain unsupported even when flagged. Raw cells
    /// are preserved; new flagged samples override their stored type with solid.
    /// [`Self::new`] keeps rejecting flagged cells when this contract is unavailable.
    ///
    /// # Errors
    /// Returns the same shape/dimension errors as [`Self::new`].
    pub fn new_passive(width: u16, height: u16, cells: Vec<u16>) -> Result<Self, Unqualified> {
        let mut room = Self::new(width, height, cells)?;
        room.passive_flags = true;
        Ok(room)
    }

    /// Restrict actual collision samples to half-open cell bounds
    /// `[left, top, right, bottom]`, replacing any previous sample halo.
    /// Position bounds remain the full grid; this does not add bounding-box samples.
    ///
    /// # Errors
    /// Returns [`Unqualified::RoomDimensions`] for empty, reversed or out-of-grid bounds.
    pub fn with_sample_halo(mut self, bounds: [u16; 4]) -> Result<Self, Unqualified> {
        let [left, top, right, bottom] = bounds;
        if left >= right || top >= bottom || right > self.width || bottom > self.height {
            return Err(Unqualified::RoomDimensions);
        }
        self.sample_halo = Some(bounds);
        Ok(self)
    }

    /// Optional half-open collision sample bounds in cells: `[left, top, right, bottom]`.
    #[must_use]
    pub const fn sample_halo(&self) -> Option<[u16; 4]> {
        self.sample_halo
    }

    /// Width in 16-pixel cells.
    #[must_use]
    pub const fn width(&self) -> u16 {
        self.width
    }
    /// Height in 16-pixel cells.
    #[must_use]
    pub const fn height(&self) -> u16 {
        self.height
    }
    /// Unmodified row-major cells; no mutable grid access is exposed.
    #[must_use]
    pub fn cells(&self) -> &[u16] {
        &self.cells
    }

    // Only the authenticated house constructor uses this on its private clone.
    pub(crate) fn replace_cell(&mut self, index: usize, raw: u16) {
        self.cells[index] = raw;
    }

    pub(crate) fn validate_position(&self, x: u16, y: u16) -> Result<(), Unqualified> {
        if x < 8
            || y < 16
            || x.checked_add(8).is_none_or(|right| right > self.width * 16)
            || y > self.height * 16
        {
            return Err(Unqualified::PositionOutOfBounds);
        }
        Ok(())
    }

    fn material(&self, x: u16, y: u16, old_edge: bool) -> Result<Material, Unqualified> {
        let (col, row) = (x / 16, y / 16);
        if self.sample_halo.is_some_and(|[left, top, right, bottom]| {
            col < left || col >= right || row < top || row >= bottom
        }) {
            return Err(Unqualified::SampleOutsideAdmission);
        }
        if col >= self.width || row >= self.height {
            return Err(Unqualified::SampleOutOfBounds);
        }
        let raw = self.cells[usize::from(row) * usize::from(self.width) + usize::from(col)];
        let kind = ((raw >> 9) & 31) as u8;
        // Old-edge slope dispatch precedes the new-edge high-bit override.
        if old_edge && matches!(kind, 6 | 7) {
            return Err(Unqualified::UnsupportedType(kind));
        }
        if raw & 0x8000 != 0 {
            return if self.passive_flags {
                Ok(Material::Solid)
            } else {
                Err(Unqualified::FlaggedCell(raw))
            };
        }
        match kind {
            0 | 2 | 22 => Ok(Material::Open),
            12 | 14 => Ok(Material::Solid),
            16 => Ok(Material::Partial),
            _ => Err(Unqualified::UnsupportedType(kind)),
        }
    }

    // Old edges only validate types: actual old-edge diversions depend on 6/7,
    // not an O/S pair dispatch. New edges can apply a perpendicular corner nudge.
    #[allow(clippy::verbose_bit_mask)] // Preserve the reference pixel-remainder test.
    fn samples(
        &self,
        x: u16,
        y: u16,
        direction: Direction,
        old_edge: bool,
    ) -> Result<(Material, Material), Unqualified> {
        let (u, v) = match direction {
            Direction::Right => (x.checked_add(7), y.checked_sub(16)),
            Direction::Left | Direction::Up => (x.checked_sub(8), y.checked_sub(16)),
            Direction::Down => (x.checked_sub(8), y.checked_sub(1)),
        };
        let (u, v) = (
            u.ok_or(Unqualified::ArithmeticOverflow)?,
            v.ok_or(Unqualified::ArithmeticOverflow)?,
        );
        let first = self.material(u, v, old_edge)?;
        let perpendicular = if direction.horizontal() { v } else { u };
        let second = if perpendicular & 15 == 0 {
            first
        } else {
            let neighbor = (perpendicular & !15)
                .checked_add(16)
                .ok_or(Unqualified::ArithmeticOverflow)?;
            if direction.horizontal() {
                self.material(u, neighbor, old_edge)?
            } else {
                self.material(neighbor, v, old_edge)?
            }
        };
        Ok((first, second))
    }

    pub(crate) fn resolve(
        &self,
        x: u16,
        y: u16,
        direction: Option<Direction>,
        dx: i16,
        dy: i16,
    ) -> Result<(u16, u16, bool), Unqualified> {
        use Material::{Open, Partial, Solid};
        let Some(direction) = direction.filter(|_| dx != 0 || dy != 0) else {
            return Ok((x, y, false));
        };
        self.samples(x, y, direction, true)?;
        let next_x = x
            .checked_add_signed(dx)
            .ok_or(Unqualified::ArithmeticOverflow)?;
        let next_y = y
            .checked_add_signed(dy)
            .ok_or(Unqualified::ArithmeticOverflow)?;
        let (first, second) = self.samples(next_x, next_y, direction, false)?;
        let (mut resolved_x, mut resolved_y) = (next_x, next_y);
        let blocked = first != Material::Open || second != Material::Open;
        if blocked {
            // Native special-player tables distinguish P16 from S12/14:
            // S/P can nudge +1, P/S can nudge -1; P/P only blocks.
            let perpendicular = if direction.horizontal() {
                next_y - 16
            } else {
                next_x - 8
            };
            let q = perpendicular % 16;
            let nudge = match (first, second) {
                (Solid | Partial, Open) | (Solid, Partial) if q >= 8 => 1,
                (Open, Solid | Partial) | (Partial, Solid) if q < 8 => -1,
                _ => 0,
            };
            if direction.horizontal() {
                resolved_y = resolved_y
                    .checked_add_signed(nudge)
                    .ok_or(Unqualified::ArithmeticOverflow)?;
            } else {
                resolved_x = resolved_x
                    .checked_add_signed(nudge)
                    .ok_or(Unqualified::ArithmeticOverflow)?;
            }
            // Positive samples use edge-1, but correction tests the unmodified edge.
            let edge = match direction {
                Direction::Left => next_x.checked_sub(8),
                Direction::Right => next_x.checked_add(8),
                Direction::Up => next_y.checked_sub(16),
                Direction::Down => Some(next_y),
            }
            .ok_or(Unqualified::ArithmeticOverflow)?;
            let snap = (edge & 8 != 0) == direction.negative();
            let corrected = if snap {
                match direction {
                    Direction::Left => (edge & !15).checked_add(24),
                    Direction::Right => (edge & !15).checked_sub(8),
                    Direction::Up => (edge & !15).checked_add(32),
                    Direction::Down => Some(edge & !15),
                }
                .ok_or(Unqualified::ArithmeticOverflow)?
            } else if direction.horizontal() {
                x
            } else {
                y
            };
            if direction.horizontal() {
                resolved_x = corrected;
            } else {
                resolved_y = corrected;
            }
        }
        self.validate_position(resolved_x, resolved_y)?;
        Ok((resolved_x, resolved_y, blocked))
    }
}
