//! Flag-gated tile patches a map load applies (`$8D:8FB4`, table `$96:CD9D`).
//!
//! Each event flag from `$280` upward owns one primary 8-byte entry, in
//! order; entries with bit 14 of their first word continue the one before.
//! When a flag is set and its primary entry names the map being loaded, the
//! primary and its continuations are applied (`$8D:900C`), the continuations
//! without a map test of their own. The table ends at a first word with
//! bit 15 set. This is what reopens C's stairs (`$292`) after the shared
//! layer is decoded again.

const TABLE: usize = 0x16_CD9D;
/// The flag of the first primary entry.
const FIRST_FLAG: u16 = 0x280;
/// A bound on the entries: the table has `$6E` before its end marker
/// (`$96:D10D`).
const MAX_ENTRIES: usize = 256;

/// One applied change to a map's collision grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Patch {
    /// A tile at a cell, under the tile's attribute (`$8D:9036`), as `COP 44`.
    Tile {
        /// Column and row.
        cell: (u8, u8),
        /// Tile number.
        tile: u16,
    },
    /// A block copied within the grid (`$8D:90A1`).
    Copy {
        /// Top-left source column and row.
        from: (u8, u8),
        /// Width and height in cells.
        size: (u8, u8),
        /// Top-left destination column and row.
        to: (u8, u8),
    },
}

/// A patch with the layer it applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagPatch {
    /// The flag that gates it.
    pub flag: u16,
    /// Bit 13: the second layer's grid (`$7E:E000`) rather than the first.
    pub second_layer: bool,
    /// What it changes.
    pub patch: Patch,
}

/// The patches a load of `map` applies for these flags, in order.
///
/// `flag` answers whether an event flag is set. Returns `None` when the table
/// does not decode within its bound.
#[must_use]
pub fn for_map(image: &[u8], map: u16, flag: impl Fn(u16) -> bool) -> Option<Vec<FlagPatch>> {
    let mut patches = Vec::new();
    let mut primary = None;
    let mut applying = false;
    for index in 0..MAX_ENTRIES {
        let entry = image.get(TABLE + index * 8..TABLE + index * 8 + 8)?;
        let head = u16::from_le_bytes([entry[0], entry[1]]);
        if head & 0x8000 != 0 {
            return Some(patches);
        }
        if head & 0x4000 == 0 {
            let owner = primary.map_or(FIRST_FLAG, |flag: u16| flag + 1);
            primary = Some(owner);
            applying = flag(owner) && head & 0x1FFF == map;
        }
        if !applying {
            continue;
        }
        let word = |at: usize| u16::from_le_bytes([entry[at], entry[at + 1]]);
        let patch = if word(2) == 0xFFFF {
            Patch::Tile {
                cell: (entry[6], entry[7]),
                tile: word(4),
            }
        } else {
            Patch::Copy {
                from: (entry[2], entry[3]),
                size: (entry[4], entry[5]),
                to: (entry[6], entry[7]),
            }
        };
        patches.push(FlagPatch {
            flag: primary?,
            second_layer: head & 0x2000 != 0,
            patch,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(entries: &[[u8; 8]]) -> Vec<u8> {
        let mut image = vec![0; TABLE];
        for entry in entries {
            image.extend_from_slice(entry);
        }
        image.extend_from_slice(&[0, 0x80, 0, 0, 0, 0, 0, 0]);
        image
    }

    #[test]
    fn a_set_flag_applies_its_entry_and_continuations_on_its_map_only() {
        let image = image(&[
            // $280: map $101, a copy.
            [0x01, 0x01, 0x00, 0x2F, 0x02, 0x01, 0x07, 0x04],
            // $281: map $0C, a tile, then a continuation on the second layer.
            [0x0C, 0x00, 0xFF, 0xFF, 0xCB, 0x00, 0x0B, 0x15],
            [0x0C, 0x60, 0xFF, 0xFF, 0xF6, 0x00, 0x0B, 0x14],
            // $282: map $0C again.
            [0x0C, 0x00, 0xFF, 0xFF, 0x10, 0x00, 0x01, 0x02],
        ]);
        let set = |flags: &'static [u16]| move |flag| flags.contains(&flag);
        assert_eq!(
            for_map(&image, 0x0C, set(&[0x281])),
            Some(vec![
                FlagPatch {
                    flag: 0x281,
                    second_layer: false,
                    patch: Patch::Tile {
                        cell: (11, 21),
                        tile: 0xCB
                    },
                },
                FlagPatch {
                    flag: 0x281,
                    second_layer: true,
                    patch: Patch::Tile {
                        cell: (11, 20),
                        tile: 0xF6
                    },
                },
            ])
        );
        assert_eq!(for_map(&image, 0x0C, set(&[0x280])), Some(vec![]));
        assert_eq!(
            for_map(&image, 0x101, set(&[0x280])),
            Some(vec![FlagPatch {
                flag: 0x280,
                second_layer: false,
                patch: Patch::Copy {
                    from: (0, 47),
                    size: (2, 1),
                    to: (7, 4)
                },
            }])
        );
        assert_eq!(
            for_map(&image, 0x0C, set(&[0x282])).map(|p| p.len()),
            Some(1),
            "the continuation does not shift the next flag"
        );
    }

    #[test]
    fn an_unterminated_table_is_refused() {
        let mut image = vec![0; TABLE];
        image.extend_from_slice(&[0x0C, 0x00, 0xFF, 0xFF, 0xCB, 0x00, 0x0B, 0x15]);
        assert_eq!(for_map(&image, 0x0C, |_| true), None);
    }
}
