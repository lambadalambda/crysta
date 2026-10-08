//! The map's second layer as scripts change it (`docs/block-patch.md`,
//! `docs/tower-second-layer.md`): `COP 46` with layers 1 to `$7F` copies
//! its cells, and the load's flag table patches them. Only the picture
//! changes; the hosts draw [`World::second_patched_cells`].

use super::World;
use assets::maps::flag_patches::Patch;

/// The second layer's cells: metatile numbers, `width` a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Layer {
    width: u16,
    cells: Vec<u16>,
}

impl Layer {
    fn index(&self, (column, row): (u16, u16)) -> Option<usize> {
        let at = usize::from(row) * usize::from(self.width) + usize::from(column);
        (column < self.width && at < self.cells.len()).then_some(at)
    }
}

impl World<'_> {
    /// The map's second layer, at its load, with the patches a map on the
    /// same layers left (`$86:9145` skips the reload).
    pub(super) fn load_second(&mut self) {
        self.second = assets::maps::visual::SecondLayer::from_rom(self.image, self.map)
            .ok()
            .map(|layer| Layer {
                width: u16::try_from(layer.layer().width()).unwrap_or(0),
                cells: layer
                    .layer()
                    .cells()
                    .iter()
                    .map(|cell| cell.raw() & 0x1FF)
                    .collect(),
            });
        for (column, row, tile) in std::mem::take(&mut self.second_patched) {
            self.patch_second((column, row), tile);
        }
    }

    /// Writes `tile` at `cell` of the second layer and keeps it among the
    /// patched cells.
    fn patch_second(&mut self, cell: (u16, u16), tile: u16) {
        let Some(layer) = &mut self.second else {
            return;
        };
        let Some(at) = layer.index(cell) else {
            return;
        };
        layer.cells[at] = tile;
        self.second_patched.retain(|&(c, r, _)| (c, r) != cell);
        self.second_patched.push((cell.0, cell.1, tile));
    }

    /// Makes the copies scripts asked for, in order: each reads the layer
    /// as the ones before left it.
    pub(super) fn apply_second(&mut self) {
        for (from, to) in std::mem::take(&mut self.globals.second_copies) {
            self.copy_second(from, to);
        }
    }

    /// Copies a cell of the second layer onto another; a cell off the layer
    /// is skipped (the game wraps; no chapter-1 copy needs it).
    fn copy_second(&mut self, from: (u16, u16), to: (u16, u16)) {
        let tile = self
            .second
            .as_ref()
            .and_then(|layer| Some(layer.cells[layer.index(from)?]));
        if let Some(tile) = tile {
            self.patch_second(to, tile);
        }
    }

    /// A load flag patch on the second layer.
    pub(super) fn apply_second_load_patch(&mut self, patch: &Patch) {
        match *patch {
            Patch::Tile { cell, tile } => {
                self.patch_second((cell.0.into(), cell.1.into()), tile & 0x1FF);
            }
            Patch::Copy { from, size, to } => {
                for dy in 0..u16::from(size.1) {
                    for dx in 0..u16::from(size.0) {
                        let source = (u16::from(from.0) + dx, u16::from(from.1) + dy);
                        let target = (u16::from(to.0) + dx, u16::from(to.1) + dy);
                        self.copy_second(source, target);
                    }
                }
            }
        }
    }

    /// Second-layer cells patched in this map: column, row, metatile.
    #[must_use]
    pub fn second_patched_cells(&self) -> &[(u16, u16, u16)] {
        &self.second_patched
    }
}
