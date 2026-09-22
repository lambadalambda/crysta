use crate::{Direction, Unqualified};
use alloc::vec::Vec;

mod directional;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Material {
    Open,
    Solid,
    Partial,
}

/// Finite source-qualified aliases, not arbitrary material remapping.
///
/// The enclosing data identity must authenticate the map/profile and policy.
/// Coordinates alone do not establish that a room is Town, C, E or 20.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialAlias {
    /// Town type25 uses the solid12 handler in all sixteen direction/pair tables.
    TownSolid25,
    /// C's closed lower door type5 uses partial16, only Up at cell (11,21).
    ClosedDoorPartial5,
    /// Type29 is open only Up at C (11,21), E (6,53) or 20 (22,53).
    /// This is collision admission, not ownership of a stair transfer.
    StairOpen29,
}

impl MaterialAlias {
    const fn kind(self) -> u8 {
        match self {
            Self::TownSolid25 => 25,
            Self::ClosedDoorPartial5 => 5,
            Self::StairOpen29 => 29,
        }
    }

    const fn material(self) -> Material {
        match self {
            Self::TownSolid25 => Material::Solid,
            Self::ClosedDoorPartial5 => Material::Partial,
            Self::StairOpen29 => Material::Open,
        }
    }
}

/// One immutable rule after validation by [`Room::with_material_policy`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialRule {
    /// Half-open cell bounds `[left, top, right, bottom]`.
    /// Door/stair aliases require exactly one of their qualified cells.
    pub bounds: [u16; 4],
    /// Collision's delayed resolve direction; `None` admits all directions for
    /// town25 only. Door/stair aliases require `Some(Direction::Up)`.
    pub direction: Option<Direction>,
    /// The only stored type and classification this rule can admit.
    pub alias: MaterialAlias,
}

impl MaterialRule {
    fn valid_scope(self) -> bool {
        match self.alias {
            MaterialAlias::TownSolid25 => true,
            MaterialAlias::ClosedDoorPartial5 => {
                self.direction == Some(Direction::Up) && self.bounds == [11, 21, 12, 22]
            }
            MaterialAlias::StairOpen29 => {
                self.direction == Some(Direction::Up)
                    && matches!(
                        self.bounds,
                        [11, 21, 12, 22] | [6, 53, 7, 54] | [22, 53, 23, 54]
                    )
            }
        }
    }

    fn overlaps(self, other: Self) -> bool {
        self.alias.kind() == other.alias.kind()
            && (self.direction.is_none()
                || other.direction.is_none()
                || self.direction == other.direction)
            && self.bounds[0] < other.bounds[2]
            && other.bounds[0] < self.bounds[2]
            && self.bounds[1] < other.bounds[3]
            && other.bounds[1] < self.bounds[3]
    }

    fn matches(self, col: u16, row: u16, direction: Direction, kind: u8) -> bool {
        self.alias.kind() == kind
            && self.direction.is_none_or(|admitted| admitted == direction)
            && (self.bounds[0]..self.bounds[2]).contains(&col)
            && (self.bounds[1]..self.bounds[3]).contains(&row)
    }
}

/// Invalid immutable material policy; no policy is installed on failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialPolicyError {
    /// Empty/reversed bounds, or bounds outside the grid or current sample halo.
    Bounds,
    /// Alias direction or exact-cell shape is outside its finite qualification.
    Scope,
    /// Two rules admit the same stored type, cell and direction, even identically.
    /// Different stored types (closed5/open29) may share a cell and direction.
    Overlap,
}

impl core::fmt::Display for MaterialPolicyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "invalid room material policy: {self:?}")
    }
}
impl core::error::Error for MaterialPolicyError {}

fn contains_bounds(outer: [u16; 4], inner: [u16; 4]) -> bool {
    inner[0] < inner[2]
        && inner[1] < inner[3]
        && outer[0] <= inner[0]
        && outer[1] <= inner[1]
        && inner[2] <= outer[2]
        && inner[3] <= outer[3]
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
    passive_directional: bool,
    sample_halo: Option<[u16; 4]>,
    material_policy: Vec<MaterialRule>,
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
            passive_directional: false,
            sample_halo: None,
            material_policy: Vec::new(),
        })
    }
    /// Constructs a grid admitting bit-15 cells as passive class-3 solids.
    ///
    /// The caller must establish ordinary, action-free movement: native collision
    /// action hooks must be inactive (reference `$0980 & $0050 == 0`). This is a
    /// collision policy assertion, not a simulation of those hooks. Do not use it
    /// for interactions, attacks, pushing, or other unqualified controller modes.
    /// The separate [`crate::pots`] component qualifies bounded held movement
    /// (`$0980=$0020`) reusing this geometry, not arbitrary carrying/action hooks.
    /// Stored old-edge slopes 6/7 remain unsupported even when flagged unless
    /// [`Self::with_passive_directional_collision`] is explicitly enabled. Raw cells
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

    /// Opt into source-derived passive directional geometry for types 5/6/7/21/29.
    ///
    /// The caller asserts the ordinary special-player resolver, fixed (-8,-16)
    /// offsets and 16×16 bounds, and inactive action hooks (`$0980 & $0050 == 0`).
    /// This also enables the passive bit15 contract of [`Self::new_passive`].
    /// Old-edge slope dispatch and raw slope-neighbor probes precede/ignore that
    /// override, respectively. This is geometry only, not gameplay side effects
    /// or qualification of a room/route. Type8 remains unsupported: its Up branch
    /// needs the additional `$097C & 4` input. Other unknown types still fail closed.
    /// The immutable policy is retained on clone/patch; hosts must bind it to room
    /// identity on snapshot restore. Existing constructors keep their old behavior.
    #[must_use]
    pub fn with_passive_directional_collision(mut self) -> Self {
        self.passive_flags = true;
        self.passive_directional = true;
        self
    }

    /// Whether the caller opted into passive directional geometry.
    #[must_use]
    pub const fn passive_directional_collision(&self) -> bool {
        self.passive_directional
    }

    /// Restrict actual collision samples to half-open cell bounds
    /// `[left, top, right, bottom]`, replacing any previous sample halo.
    /// Position bounds remain the full grid; this does not add bounding-box samples.
    ///
    /// # Errors
    /// Returns [`Unqualified::RoomDimensions`] for empty, reversed or out-of-grid bounds,
    /// or if an installed material rule would extend outside the new halo.
    pub fn with_sample_halo(mut self, bounds: [u16; 4]) -> Result<Self, Unqualified> {
        if !contains_bounds([0, 0, self.width, self.height], bounds)
            || self
                .material_policy
                .iter()
                .any(|rule| !contains_bounds(bounds, rule.bounds))
        {
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

    /// Install finite classification rules without rewriting any raw source word,
    /// replacing the previous policy. An empty policy restores default dispatch.
    ///
    /// Rules need not currently match a stored cell: cloned rooms retain policy
    /// across source-authenticated pot/door patches. Town bounds may be any valid
    /// subset of the grid/current halo; the host must authenticate actual Town
    /// bounds and each map/profile's allowed rules through its enclosing data
    /// identity, including on restore. This does not authenticate source data,
    /// grant a whole-map walking profile or enable the passive bit15 assertion.
    ///
    /// # Errors
    /// Rejects invalid bounds, unqualified alias scopes and overlapping entries
    /// for the same type/direction. Validation is independent of rule order.
    pub fn with_material_policy(
        mut self,
        rules: Vec<MaterialRule>,
    ) -> Result<Self, MaterialPolicyError> {
        let bounds = self.sample_halo.unwrap_or([0, 0, self.width, self.height]);
        for (index, rule) in rules.iter().enumerate() {
            if !contains_bounds(bounds, rule.bounds) {
                return Err(MaterialPolicyError::Bounds);
            }
            if !rule.valid_scope() {
                return Err(MaterialPolicyError::Scope);
            }
            if rules[..index].iter().any(|other| rule.overlaps(*other)) {
                return Err(MaterialPolicyError::Overlap);
            }
        }
        self.material_policy = rules;
        Ok(self)
    }

    /// Validated rules, retained on clone/patch and available for host identity checks.
    #[must_use]
    pub fn material_policy(&self) -> &[MaterialRule] {
        &self.material_policy
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

    // Authenticated house construction and pots' private collision overlays only.
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

    fn material(
        &self,
        x: u16,
        y: u16,
        direction: Direction,
        old_edge: bool,
    ) -> Result<Material, Unqualified> {
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
        if let Some(rule) = self
            .material_policy
            .iter()
            .find(|rule| rule.matches(col, row, direction, kind))
        {
            return Ok(rule.alias.material());
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
        let first = self.material(u, v, direction, old_edge)?;
        let perpendicular = if direction.horizontal() { v } else { u };
        let second = if perpendicular & 15 == 0 {
            first
        } else {
            let neighbor = (perpendicular & !15)
                .checked_add(16)
                .ok_or(Unqualified::ArithmeticOverflow)?;
            if direction.horizontal() {
                self.material(u, neighbor, direction, old_edge)?
            } else {
                self.material(neighbor, v, direction, old_edge)?
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
        if self.passive_directional {
            return self.resolve_directional(x, y, direction, dx, dy);
        }
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

#[cfg(test)]
mod tests {
    use super::{MaterialAlias, MaterialRule, Room};
    use crate::{pots, Direction, WalkingState};
    use alloc::vec;

    #[test]
    fn raw_door_patch_retains_both_classifications_without_touching_source() {
        let mut cells = vec![0; 32 * 64];
        cells[21 * 32 + 11] = 0x0b81;
        let rules = vec![
            MaterialRule {
                bounds: [11, 21, 12, 22],
                direction: Some(Direction::Up),
                alias: MaterialAlias::ClosedDoorPartial5,
            },
            MaterialRule {
                bounds: [11, 21, 12, 22],
                direction: Some(Direction::Up),
                alias: MaterialAlias::StairOpen29,
            },
        ];
        let original = Room::new_passive(32, 64, cells)
            .unwrap()
            .with_sample_halo([1, 20, 14, 32])
            .unwrap()
            .with_material_policy(rules)
            .unwrap();
        assert_eq!(
            original.resolve(184, 360, Some(Direction::Up), 0, -1),
            Ok((184, 360, true))
        );
        let mut patched = original.clone();
        patched.replace_cell(21 * 32 + 11, 0x3acb);
        assert_eq!(patched.material_policy(), original.material_policy());
        assert_eq!(patched.sample_halo(), original.sample_halo());
        assert_eq!(original.cells()[21 * 32 + 11], 0x0b81);
        assert_eq!(patched.cells()[21 * 32 + 11], 0x3acb);
        assert_eq!(
            patched.resolve(184, 360, Some(Direction::Up), 0, -1),
            Ok((184, 359, false))
        );
    }

    #[test]
    fn pots_consumed_overlay_preserves_raw_room_policy() {
        let mut cells = vec![0; 32 * 64];
        cells[21 * 32 + 3] = 0x18fa;
        cells[21 * 32 + 11] = 0x3acb;
        let room = Room::new_passive(32, 64, cells)
            .unwrap()
            .with_material_policy(vec![MaterialRule {
                bounds: [11, 21, 12, 22],
                direction: Some(Direction::Up),
                alias: MaterialAlias::StairOpen29,
            }])
            .unwrap();
        let original = room.clone();
        let objects = [pots::SourceObject {
            cell: 21 * 32 + 3,
            raw: 0x18fa,
            replacement: 0xf8,
        }];
        let admission = pots::Admission {
            room: &room,
            objects: &objects,
            cellar_up_lanes: false,
            door_hit_enabled: false,
        };
        let mut state =
            pots::PotState::with_ledger(&admission, WalkingState::new(184, 360), Direction::Up, 1)
                .unwrap();
        let input = pots::Input {
            direction: Some(Direction::Up),
            action: false,
        };
        for _ in 0..2 {
            state.step(&admission, input).unwrap();
        }
        let out = state
            .step(&admission, pots::Input::default())
            .unwrap()
            .movement
            .unwrap();
        assert_eq!((out.dx, out.dy, out.blocked), (0, -1, false));
        assert_eq!(room, original);
    }
}
