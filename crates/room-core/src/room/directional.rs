//! Direct translation of the bounded special-player branches at $80:D344,
//! D739, DAE0 and DE57. Coordinates `u/v` are $1C/$22 (positive edge NOT -1).
//! No map wrapping, alternate bounds, action hooks or arbitrary CPU execution.
use super::{Material, Room};
use crate::{Direction, Unqualified};

#[derive(Clone, Copy)]
struct Edge {
    u: i32,
    v: i32,
    direction: Direction,
}

impl Edge {
    fn new(x: u16, y: u16, direction: Direction) -> Self {
        Self {
            u: i32::from(x) + if direction == Direction::Right { 8 } else { -8 },
            v: i32::from(y) + if direction == Direction::Down { 0 } else { -16 },
            direction,
        }
    }

    fn cell(self) -> (i32, i32) {
        (
            (self.u - i32::from(self.direction == Direction::Right)).div_euclid(16),
            (self.v - i32::from(self.direction == Direction::Down)).div_euclid(16),
        )
    }

    fn q(self) -> i32 {
        if self.direction.horizontal() {
            self.v & 15
        } else {
            self.u & 15
        }
    }

    fn neighbor(self, cell: (i32, i32)) -> (i32, i32) {
        if self.direction.horizontal() {
            (cell.0, cell.1 + 1)
        } else {
            (cell.0 + 1, cell.1)
        }
    }

    fn first_slope(self) -> u8 {
        if matches!(self.direction, Direction::Up | Direction::Left) {
            6
        } else {
            7
        }
    }

    // $E7D4/E7E1/E7FA/E808: CMP #$11, not a guessed triangle test.
    #[allow(clippy::verbose_bit_mask)] // Preserve the source AND #$000F test.
    fn sum(self, first: bool) -> i32 {
        let positive = |n: i32| if n & 15 == 0 { 16 } else { n & 15 };
        let x = match self.direction {
            Direction::Left => 16 - (self.u & 15),
            Direction::Right => positive(self.u),
            _ if first => 16 - (self.u & 15),
            _ => positive(self.u),
        };
        let y = match self.direction {
            Direction::Up => 16 - (self.v & 15),
            Direction::Down => positive(self.v),
            _ if first => 16 - (self.v & 15),
            _ => positive(self.v),
        };
        x + y
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Response {
    Pass,
    Block(i32), // perpendicular nudge, not attempted-axis displacement
    Slide(i32),
}

impl Room {
    fn directional_raw(&self, cell: (i32, i32)) -> Result<u16, Unqualified> {
        let (col, row) = cell;
        if self.sample_halo.is_some_and(|b| {
            col < i32::from(b[0])
                || row < i32::from(b[1])
                || col >= i32::from(b[2])
                || row >= i32::from(b[3])
        }) {
            return Err(Unqualified::SampleOutsideAdmission);
        }
        if col < 0 || row < 0 || col >= i32::from(self.width) || row >= i32::from(self.height) {
            return Err(Unqualified::SampleOutOfBounds);
        }
        let col = usize::try_from(col).map_err(|_| Unqualified::SampleOutOfBounds)?;
        let row = usize::try_from(row).map_err(|_| Unqualified::SampleOutOfBounds)?;
        Ok(self.cells[row * usize::from(self.width) + col])
    }

    fn directional_kind(&self, cell: (i32, i32), direction: Direction) -> Result<u8, Unqualified> {
        let raw = self.directional_raw(cell)?;
        if raw & 0x8000 != 0 {
            return Ok(12);
        } // $E838/$E750/$E777 class3 override
        let kind = ((raw >> 9) & 31) as u8;
        if matches!(kind, 5 | 6 | 7 | 21 | 29) || (kind == 8 && self.type8_special_bit_clear) {
            return Ok(kind);
        }
        // Retain existing finite policies (notably town25), without broadening them.
        let x = u16::try_from(cell.0 * 16).map_err(|_| Unqualified::SampleOutOfBounds)?;
        let y = u16::try_from(cell.1 * 16).map_err(|_| Unqualified::SampleOutOfBounds)?;
        self.material(x, y, direction, false).map(|m| match m {
            Material::Open => 0,
            Material::Solid => 12,
            Material::Partial => 16,
        })
    }

    // $E849/$E87C both use raw type, deliberately NOT the bit15 override.
    // The tables differ for types17/30, which remain outside this admission.
    fn slope_probe(&self, cell: (i32, i32), direction: Direction) -> Result<u8, Unqualified> {
        let raw = self.directional_raw(cell)?;
        let kind = ((raw >> 9) & 31) as u8;
        match kind {
            0 | 2 | 22 => Ok(0),
            6 | 7 => Ok(kind),
            // Both $E85C[8] and $E88F[8] are $0F, even with raw bit15 set.
            8 if self.type8_special_bit_clear => Ok(15),
            5 | 12 | 14 | 16 | 21 | 29 => Ok(15),
            25 => {
                // Authenticate the existing scoped alias even for a raw probe.
                let col = u16::try_from(cell.0).map_err(|_| Unqualified::SampleOutOfBounds)?;
                let row = u16::try_from(cell.1).map_err(|_| Unqualified::SampleOutOfBounds)?;
                if self
                    .material_policy
                    .iter()
                    .any(|rule| rule.matches(col, row, direction, kind))
                {
                    Ok(15)
                } else {
                    Err(Unqualified::UnsupportedType(kind))
                }
            }
            _ => Err(Unqualified::UnsupportedType(kind)),
        }
    }

    pub(super) fn resolve_directional(
        &self,
        x: u16,
        y: u16,
        direction: Direction,
        dx: i16,
        dy: i16,
    ) -> Result<(u16, u16, bool), Unqualified> {
        let nx = x
            .checked_add_signed(dx)
            .ok_or(Unqualified::ArithmeticOverflow)?;
        let ny = y
            .checked_add_signed(dy)
            .ok_or(Unqualified::ArithmeticOverflow)?;
        let old = Edge::new(x, y, direction);
        let mut edge = Edge::new(nx, ny, direction);
        let old_cell = old.cell();
        let first_raw = ((self.directional_raw(old_cell)? >> 9) & 31) as u8;
        // Aligned: either slope in the single sample. Unaligned: only 6/7 in
        // the direction-specific first/second positions can divert. $D35D etc.
        let slope =
            if first_raw == edge.first_slope() || (old.q() == 0 && matches!(first_raw, 6 | 7)) {
                Some(first_raw)
            } else {
                self.directional_kind(old_cell, direction)?;
                if old.q() == 0 {
                    None
                } else {
                    let cell = old.neighbor(old_cell);
                    let raw = ((self.directional_raw(cell)? >> 9) & 31) as u8;
                    if raw == 13 - edge.first_slope() {
                        Some(raw)
                    } else {
                        self.directional_kind(cell, direction)?;
                        None
                    }
                }
            };
        let mut px = i32::from(nx);
        let mut py = i32::from(ny);
        let response = if let Some(kind) = slope {
            let first = kind == edge.first_slope();
            let crossed = if direction.horizontal() {
                (old.u ^ edge.u) & 16 != 0
            } else {
                (old.v ^ edge.v) & 16 != 0
            };
            if crossed {
                // $D424/D488/D80B/D86A/DBAD/DBFA/DF29/DF76: align
                // perpendicular coordinate and redispatch ONE new sample.
                if direction.horizontal() {
                    edge.v = (edge.v & !15) + if first { 16 } else { 0 };
                    py = edge.v + 16;
                } else {
                    edge.u = (edge.u & !15) + if first { 16 } else { 0 };
                    px = edge.u + 8;
                }
                self.new_directional(edge)?
            } else if direction == Direction::Left
                && first
                && self.slope_probe(edge.neighbor(edge.cell()), direction)? != 0
            {
                // $DBAD branches to $DBC9, not $DBD0: Left6 alone retains
                // the unconditional below-cell check even on an old edge.
                Response::Block(0)
            } else {
                self.slope_response(edge, first)?
            }
        } else {
            self.new_directional(edge)?
        };
        let blocked = matches!(response, Response::Block(_));
        match response {
            Response::Pass => {}
            Response::Slide(n) | Response::Block(n) => {
                if direction.horizontal() {
                    py += n;
                } else {
                    px += n;
                }
            }
        }
        if blocked {
            let axis = if direction.horizontal() {
                edge.u
            } else {
                edge.v
            };
            let snap = (axis & 8 != 0) == direction.negative();
            let corrected = if snap {
                match direction {
                    Direction::Left => (axis & !15) + 24,
                    Direction::Right => (axis & !15) - 8,
                    Direction::Up => (axis & !15) + 32,
                    Direction::Down => axis & !15,
                }
            } else if direction.horizontal() {
                i32::from(x)
            } else {
                i32::from(y)
            };
            if direction.horizontal() {
                px = corrected;
            } else {
                py = corrected;
            }
        }
        let px = u16::try_from(px).map_err(|_| Unqualified::ArithmeticOverflow)?;
        let py = u16::try_from(py).map_err(|_| Unqualified::ArithmeticOverflow)?;
        self.validate_position(px, py)?;
        Ok((px, py, blocked))
    }

    fn slope_response(&self, edge: Edge, first: bool) -> Result<Response, Unqualified> {
        let sum = edge.sum(first);
        if sum < 17 {
            return Ok(Response::Pass);
        }
        let (col, row) = edge.cell();
        let base = if first {
            edge.neighbor((col, row))
        } else if edge.q() != 0 {
            (col, row)
        } else if edge.direction.horizontal() {
            (col, row - 1)
        } else {
            (col - 1, row)
        };
        if self.slope_probe(base, edge.direction)? != 0 {
            return Ok(Response::Block(0));
        }
        let neighbor = match edge.direction {
            Direction::Up => (base.0, base.1 + 1),
            Direction::Down => (base.0, base.1 - 1),
            Direction::Left => (base.0 + 1, base.1),
            Direction::Right => (base.0 - 1, base.1),
        };
        // Vertical second-slope branches notably test the FIRST slope value:
        // $D4EA compares6, $D8CC compares7. Do not impose mirror symmetry here.
        let allowed = if first || !edge.direction.horizontal() {
            edge.first_slope()
        } else {
            13 - edge.first_slope()
        };
        let probe = self.slope_probe(neighbor, edge.direction)?;
        Ok(if probe == 0 || probe == allowed {
            Response::Slide(if first { sum - 16 } else { 16 - sum })
        } else {
            Response::Block(0)
        })
    }

    fn new_directional(&self, edge: Edge) -> Result<Response, Unqualified> {
        let cell = edge.cell();
        let first = self.directional_kind(cell, edge.direction)?;
        // $D542[8] -> $D506: qualified $097C&4 clear -> partial ($D3B6).
        // $D8E8/$DC60/$DFDC[8] use open FIRST dispatch only, not a raw alias.
        let first = if first == 8 {
            if edge.direction == Direction::Up {
                16
            } else {
                0
            }
        } else {
            first
        };
        if first == edge.first_slope() {
            // First-table6/7: $D440/$D827/$DBC9/$DF45 unconditional probe.
            if self.slope_probe(edge.neighbor(cell), edge.direction)? != 0 {
                return Ok(Response::Block(0));
            }
            return self.slope_response(edge, true);
        }
        if first == 13 - edge.first_slope() {
            if edge.q() == 0 {
                return self.slope_response(edge, false);
            }
            // First-table opposite slope: $D4A0/$D882/$DC12/$DF8E.
            // Left really probes ABOVE, unlike Right's BELOW ($E7BA/$E78E).
            let probe_cell = if edge.direction == Direction::Left {
                (cell.0, cell.1 - 1)
            } else {
                edge.neighbor(cell)
            };
            if self.slope_probe(probe_cell, edge.direction)? != 15 {
                return self.corner_response(edge);
            }
            // $D893..$D8A0: Down6 checks stored8 AFTER the raw probe, before
            // second-table bit15 override, and takes the both-way corner path.
            if edge.direction == Direction::Down
                && (self.directional_raw(probe_cell)? >> 9) & 31 == 8
            {
                return self.corner_response(edge);
            }
            // Up's analogous raw26 exception remains outside admission.
            let second = self.directional_kind(edge.neighbor(cell), edge.direction)?;
            return self.pair_response(edge, 16, second);
        }
        if edge.q() == 0 {
            return Ok(if matches!(first, 0 | 2 | 22 | 29) {
                Response::Pass
            } else {
                Response::Block(0)
            });
        }
        let second = self.directional_kind(edge.neighbor(cell), edge.direction)?;
        self.pair_response(edge, first, second)
    }

    fn corner_response(&self, edge: Edge) -> Result<Response, Unqualified> {
        // Up $D3CD compares the two dispatch indexes before its q>=8 nudge.
        if edge.direction == Direction::Up && edge.q() != 0 {
            let raw_index = |cell| {
                self.directional_raw(cell).map(|raw| {
                    if raw & 0x8000 != 0 {
                        3
                    } else {
                        (raw >> 9) & 31
                    }
                })
            };
            if raw_index(edge.cell())? == raw_index(edge.neighbor(edge.cell()))? {
                return Ok(Response::Block(0));
            }
        }
        Ok(Response::Block(if edge.q() >= 8 { 1 } else { -1 }))
    }

    fn pair_response(&self, edge: Edge, first: u8, second: u8) -> Result<Response, Unqualified> {
        let open = |k| matches!(k, 0 | 2 | 22 | 29);
        let partial = |k| matches!(k, 5 | 16);
        let plus = Response::Block(i32::from(edge.q() >= 8));
        let minus = Response::Block(-i32::from(edge.q() < 8));
        let solid = Response::Block(0);
        if second == 8 {
            // O/P/S table [8] targets are direction- and order-dependent.
            // Up: D3F1/D3CD/D3E6; Down: D7BE/D7C1/D7C8;
            // Left: DB66/DB7B/DB86; Right: DEDC/DEF1/DEFC.
            return if open(first) {
                Ok(if edge.direction == Direction::Up {
                    minus
                } else {
                    Response::Pass
                })
            } else if partial(first) {
                if edge.direction.horizontal() {
                    Ok(minus)
                } else {
                    self.corner_response(edge)
                }
            } else {
                Ok(if edge.direction.horizontal() {
                    solid
                } else {
                    plus
                })
            };
        }
        if matches!(second, 6 | 7) {
            if open(first) {
                return if second == edge.first_slope() {
                    self.corner_response(edge)
                } else {
                    self.slope_response(edge, false)
                };
            }
            if second != edge.first_slope() {
                return Ok(solid);
            }
            // Up P/6 uses the both-way corner path; other P/first-slope
            // entries and every S/first-slope use only the positive nudge.
            if partial(first) && edge.direction == Direction::Up {
                return self.corner_response(edge);
            }
            return Ok(plus);
        }
        Ok(if open(first) {
            if open(second) {
                Response::Pass
            } else {
                minus
            }
        } else if partial(first) {
            if open(second) {
                plus
            } else if partial(second) {
                solid
            } else {
                minus
            }
        } else if open(second) || partial(second) {
            // $E09C[29] -> $DEFC, not Open's $DEE6.
            if edge.direction == Direction::Right && second == 29 {
                solid
            } else {
                plus
            }
        } else {
            solid
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn grid(cells: &[(usize, usize, u16)]) -> Room {
        let mut raw = vec![0; 8 * 8];
        for &(col, row, word) in cells {
            raw[row * 8 + col] = word;
        }
        Room::new(8, 8, raw)
            .unwrap()
            .with_passive_directional_collision()
    }

    fn resolve(
        room: &Room,
        x: u16,
        y: u16,
        d: Direction,
        step: i16,
    ) -> Result<(u16, u16, bool), Unqualified> {
        let delta = if d.negative() { -step } else { step };
        room.resolve(
            x,
            y,
            Some(d),
            if d.horizontal() { delta } else { 0 },
            if d.horizontal() { 0 } else { delta },
        )
    }

    #[test]
    fn type8_first_dispatch_is_partial_up_and_open_elsewhere_only() {
        // First tables D542/D8E8/DC60/DFDC [8] -> D506/D7A0/DB48/DEBE.
        // With $097C&4 clear, D506 -> D3B6 (partial), not D3AC (open).
        for d in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            for q in 0..16 {
                let edge = Edge {
                    u: 48 + if d.horizontal() { 8 } else { q },
                    v: 48 + if d.horizontal() { q } else { 8 },
                    direction: d,
                };
                let a = edge.cell();
                let b = edge.neighbor(a);
                for second in [0, 5, 6, 7, 12, 16, 21, 29] {
                    let cells = [
                        (
                            usize::try_from(a.0).unwrap(),
                            usize::try_from(a.1).unwrap(),
                            8 << 9,
                        ),
                        (
                            usize::try_from(b.0).unwrap(),
                            usize::try_from(b.1).unwrap(),
                            second << 9,
                        ),
                    ];
                    let candidate = grid(&cells).with_passive_directional_type8_special_bit_clear();
                    let mut aliases = cells;
                    aliases[0].2 = if d == Direction::Up { 16 << 9 } else { 0 };
                    // First dispatch is open, but the opposite-slope handler's
                    // base probe still sees raw8 (15), not the open alias (0).
                    let expected = if d != Direction::Up
                        && q != 0
                        && second == u16::from(13 - edge.first_slope())
                        && edge.sum(false) >= 17
                    {
                        Ok(Response::Block(0))
                    } else {
                        grid(&aliases).new_directional(edge)
                    };
                    assert_eq!(
                        candidate.new_directional(edge),
                        expected,
                        "{d:?} q{q} 8/{second}"
                    );
                }
            }
        }
    }

    #[test]
    fn type8_second_dispatch_keeps_all_twelve_pair_table_targets() {
        // [8], O/P/S: Up D582/D5C2/D602 -> D3F1/D3CD/D3E6;
        // Down D928/D968/D9A8 -> D7BE/D7C1/D7C8;
        // Left DCA0/DCE0/DD20 -> DB66/DB7B/DB86;
        // Right E01C/E05C/E09C -> DEDC/DEF1/DEFC.
        for d in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            for q in 1..16 {
                let edge = Edge {
                    u: 48 + if d.horizontal() { 8 } else { q },
                    v: 48 + if d.horizontal() { q } else { 8 },
                    direction: d,
                };
                let a = edge.cell();
                let b = edge.neighbor(a);
                for first in [0, 5, 8, 12, 16, 21, 29] {
                    let candidate = grid(&[
                        (
                            usize::try_from(a.0).unwrap(),
                            usize::try_from(a.1).unwrap(),
                            first << 9,
                        ),
                        (
                            usize::try_from(b.0).unwrap(),
                            usize::try_from(b.1).unwrap(),
                            8 << 9,
                        ),
                    ])
                    .with_passive_directional_type8_special_bit_clear();
                    let expected = match (d, first) {
                        // Up8/8 suppresses D3CD's nudge for equal indexes.
                        (Direction::Up, 8) | (Direction::Left | Direction::Right, 12 | 21) => {
                            Response::Block(0)
                        }
                        (Direction::Up, 0 | 29) | (Direction::Left | Direction::Right, 5 | 16) => {
                            Response::Block(-i32::from(q < 8))
                        }
                        (Direction::Up | Direction::Down, 5 | 16) => {
                            Response::Block(if q < 8 { -1 } else { 1 })
                        }
                        (Direction::Up | Direction::Down, 12 | 21) => {
                            Response::Block(i32::from(q >= 8))
                        }
                        _ => Response::Pass,
                    };
                    assert_eq!(
                        candidate.new_directional(edge),
                        Ok(expected),
                        "{d:?} q{q} {first}/8"
                    );
                }
            }
        }
    }

    #[test]
    fn down6_raw_neighbor8_takes_both_way_corner_even_when_flagged() {
        // D882 unaligned -> E767 -> E849[8]=15; D893 reads raw stored8,
        // D89B/D8A0 -> D7C1 instead of the normal partial redispatch D7AA.
        for q in 1..16 {
            let edge = Edge {
                u: 48 + q,
                v: 56,
                direction: Direction::Down,
            };
            for flag in [0, 0x8000] {
                let candidate = grid(&[(3, 3, 6 << 9), (4, 3, flag | 8 << 9)])
                    .with_passive_directional_type8_special_bit_clear();
                assert_eq!(
                    candidate.new_directional(edge),
                    Ok(Response::Block(if q < 8 { -1 } else { 1 }))
                );
                let control = grid(&[(3, 3, 6 << 9), (4, 3, flag | 12 << 9)]);
                assert_eq!(
                    control.new_directional(edge),
                    Ok(Response::Block(-i32::from(q < 8)))
                );
            }
        }
    }

    #[test]
    fn type8_raw_slope_probes_are_obstructions_not_open_or_flag_overrides() {
        for d in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            for flag in [0, 0x8000] {
                let ordinary = grid(&[(3, 3, flag | 8 << 9)]);
                assert_eq!(
                    ordinary.slope_probe((3, 3), d),
                    Err(Unqualified::UnsupportedType(8))
                );
                let admitted = ordinary.with_passive_directional_type8_special_bit_clear();
                // Both E85C[8] and E88F[8] are 15, raw bit15 ignored.
                assert_eq!(admitted.slope_probe((3, 3), d), Ok(15));
                assert_eq!(
                    admitted.directional_kind((3, 3), d),
                    Ok(if flag == 0 { 8 } else { 12 })
                );
            }
        }
    }

    #[test]
    fn old_left6_probes_below_even_without_slope_penetration() {
        // $DBAD -> $DBC9 (unlike Right7's $DF29 -> $DF4C).
        // sum=(16-10)+(16-10)=12 <17; the below-S check still blocks.
        let room = grid(&[(3, 3, 6 << 9), (3, 4, 12 << 9)]);
        assert_eq!(
            resolve(&room, 67, 74, Direction::Left, 1),
            Ok((72, 74, true))
        );
    }

    #[test]
    fn ordinary_pairs_match_legacy_for_every_subcell_and_step() {
        // Independent existing O/S/P resolver remains the oracle for all nine
        // ordered pairs, including equal pairs and both orders of each mixture.
        for first in [0, 12, 16] {
            for second in [0, 12, 16] {
                for d in [
                    Direction::Up,
                    Direction::Down,
                    Direction::Left,
                    Direction::Right,
                ] {
                    // Stripes are perpendicular to travel, so every unaligned
                    // sample sees first/second even when the moving edge crosses
                    // a cell boundary. Aligned samples see only first.
                    let cells = (0..64)
                        .map(|i| {
                            let perpendicular = if d.horizontal() { i / 8 } else { i % 8 };
                            (if perpendicular <= 3 { first } else { second }) << 9
                        })
                        .collect();
                    let legacy = Room::new_passive(8, 8, cells).unwrap();
                    let candidate = legacy.clone().with_passive_directional_collision();
                    for x in 56..72 {
                        for y in 64..80 {
                            for step in [1, 2] {
                                assert_eq!(
                                    resolve(&candidate, x, y, d, step),
                                    resolve(&legacy, x, y, d, step),
                                    "{first}/{second} {d:?} {x},{y} step{step}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn passive5_and21_are_exact_pair_geometry_aliases() {
        for (kind, alias) in [(5, 16), (21, 12)] {
            // Full ordered products also put the alias in either sample and in
            // both samples, rather than relying on a cyclic material pattern.
            for first in [0, kind, 12, 16] {
                for second in [0, kind, 12, 16] {
                    for d in [
                        Direction::Up,
                        Direction::Down,
                        Direction::Left,
                        Direction::Right,
                    ] {
                        let cells: alloc::vec::Vec<_> = (0..64)
                            .map(|i| {
                                let perpendicular = if d.horizontal() { i / 8 } else { i % 8 };
                                (if perpendicular <= 3 { first } else { second }) << 9
                            })
                            .collect();
                        let alias_cells = cells
                            .iter()
                            .map(|&w| if w == kind << 9 { alias << 9 } else { w })
                            .collect();
                        let candidate = Room::new(8, 8, cells)
                            .unwrap()
                            .with_passive_directional_collision();
                        let legacy = Room::new(8, 8, alias_cells).unwrap();
                        for x in 56..72 {
                            for y in 64..80 {
                                for step in [1, 2] {
                                    assert_eq!(
                                        resolve(&candidate, x, y, d, step),
                                        resolve(&legacy, x, y, d, step),
                                        "alias{kind}->{alias} {first}/{second} {d:?} {x},{y} step{step}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn pair_tables_keep_order_and_direction_specific_slope_targets() {
        // O/P/S second-table entries at D582/D5C2/D602 and mirrors.
        for d in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            for q in [4, 12] {
                let edge = Edge {
                    u: 48 + if d.horizontal() { 8 } else { q },
                    v: 48 + if d.horizontal() { q } else { 8 },
                    direction: d,
                };
                for first in [0, 16, 12] {
                    for second in [6, 7] {
                        let a = edge.cell();
                        let b = edge.neighbor(a);
                        let room = grid(&[
                            (
                                usize::try_from(a.0).unwrap(),
                                usize::try_from(a.1).unwrap(),
                                first << 9,
                            ),
                            (
                                usize::try_from(b.0).unwrap(),
                                usize::try_from(b.1).unwrap(),
                                second << 9,
                            ),
                        ]);
                        let response = room.new_directional(edge).unwrap();
                        let expected = if second == edge.first_slope().into() {
                            Response::Block(if first == 0 || (first == 16 && d == Direction::Up) {
                                if q >= 8 {
                                    1
                                } else {
                                    -1
                                }
                            } else {
                                i32::from(q >= 8)
                            })
                        } else if first == 0 {
                            // q4 -> no overlap, q12 -> sum20 -> -4 slide.
                            if q == 4 {
                                Response::Pass
                            } else {
                                Response::Slide(-4)
                            }
                        } else {
                            Response::Block(0)
                        };
                        assert_eq!(response, expected, "{d:?} q{q} {first}/{second}");
                    }
                }
            }
        }
    }

    #[test]
    fn new_aligned_slopes_use_source_remainders_for_one_and_two_pixels() {
        for step in [1, 2] {
            for (d, x, y, col, row, expected) in [
                (Direction::Up, 56, 80, 3, 3, (56 + step, 80 - step, false)),
                (Direction::Down, 56, 64, 3, 4, (56 + step, 64 + step, false)),
                (Direction::Left, 72, 64, 3, 3, (72 - step, 64 + step, false)),
                (
                    Direction::Right,
                    56,
                    64,
                    4,
                    3,
                    (56 + step, 64 + step, false),
                ),
            ] {
                let kind = if matches!(d, Direction::Up | Direction::Left) {
                    6
                } else {
                    7
                };
                assert_eq!(
                    resolve(
                        &grid(&[(col, row, kind << 9)]),
                        x,
                        y,
                        d,
                        i16::try_from(step).unwrap()
                    ),
                    Ok(expected),
                    "{d:?}"
                );
                // Mirror slopes in the same aligned sample use the reverse slide.
                let mirrored = (
                    expected.0 - if d.horizontal() { 0 } else { 2 * step },
                    expected.1 - if d.horizontal() { 2 * step } else { 0 },
                    false,
                );
                assert_eq!(
                    resolve(
                        &grid(&[(col, row, (13 - kind) << 9)]),
                        x,
                        y,
                        d,
                        i16::try_from(step).unwrap()
                    ),
                    Ok(mirrored),
                    "mirror {d:?}"
                );
            }
        }
    }

    #[test]
    fn old_slope_axis_crossing_aligns_perpendicular_then_redispatches() {
        for step in [1, 2] {
            for (d, x, y, col, row, expected) in [
                (Direction::Up, 63, 64, 3, 3, (72, 64 - step, false)),
                (Direction::Down, 63, 63, 3, 3, (72, 63 + step, false)),
                (Direction::Left, 56, 71, 3, 3, (56 - step, 80, false)),
                (Direction::Right, 55, 71, 3, 3, (55 + step, 80, false)),
            ] {
                let kind = if matches!(d, Direction::Up | Direction::Left) {
                    6
                } else {
                    7
                };
                assert_eq!(
                    resolve(
                        &grid(&[(col, row, kind << 9)]),
                        x,
                        y,
                        d,
                        i16::try_from(step).unwrap()
                    ),
                    Ok(expected),
                    "{d:?}"
                );
            }
        }
    }

    #[test]
    fn right_solid_first29_blocks_without_open_nudge() {
        let stairs = grid(&[(4, 3, 12 << 9), (4, 4, 29 << 9)]);
        let open = grid(&[(4, 3, 12 << 9)]);
        for step in [1, 2] {
            assert_eq!(
                resolve(&stairs, 56, 76, Direction::Right, step),
                Ok((56, 76, true))
            );
            assert_eq!(
                resolve(&open, 56, 76, Direction::Right, step),
                Ok((56, 77, true))
            );
        }
        // P-first29 still uses the positive nudge, and Left S-first29 does too.
        assert_eq!(
            resolve(
                &grid(&[(4, 3, 16 << 9), (4, 4, 29 << 9)]),
                56,
                76,
                Direction::Right,
                1
            ),
            Ok((56, 77, true))
        );
        assert_eq!(
            resolve(
                &grid(&[(3, 3, 12 << 9), (3, 4, 29 << 9)]),
                72,
                76,
                Direction::Left,
                1
            ),
            Ok((72, 77, true))
        );
    }

    #[test]
    fn flags_override_new_edges_but_not_old_slopes_or_raw_probes() {
        let slope = grid(&[(3, 4, 6 << 9)]);
        let flagged_slope = grid(&[(3, 4, 0x8000 | 6 << 9)]);
        for step in [1, 2] {
            assert_eq!(
                resolve(&flagged_slope, 48, 73, Direction::Right, step),
                resolve(&slope, 48, 73, Direction::Right, step)
            );
        }
        // New flagged slope6 is class3, not a slide or unknown error.
        assert_eq!(
            resolve(
                &grid(&[(4, 3, 0x8000 | 6 << 9)]),
                56,
                64,
                Direction::Right,
                1
            ),
            Ok((56, 64, true))
        );
        // Raw helper probes ignore bit15 even on a stored Open neighbor.
        let raw_probe = grid(&[(3, 4, 6 << 9), (3, 3, 0x8000)]);
        assert_eq!(
            resolve(&raw_probe, 48, 73, Direction::Right, 1),
            resolve(&slope, 48, 73, Direction::Right, 1)
        );
        // Unknown stored neighbors must not acquire a guessed probe-table class.
        let unknown = grid(&[(3, 4, 6 << 9), (2, 3, 0x8000 | 9 << 9)]);
        assert_eq!(
            resolve(&unknown, 48, 73, Direction::Right, 1),
            Err(Unqualified::UnsupportedType(9))
        );
    }

    #[test]
    fn slope_neighbor_samples_respect_halo_and_grid_bounds() {
        let room = grid(&[(3, 4, 6 << 9)])
            .with_sample_halo([3, 3, 4, 5])
            .unwrap();
        // Old/new leading samples fit; $DFBB additionally probes LEFT of base.
        assert_eq!(
            resolve(&room, 48, 73, Direction::Right, 1),
            Err(Unqualified::SampleOutsideAdmission)
        );
        let room = grid(&[(0, 3, 7 << 9)]);
        // Aligned Up7 uses the cell LEFT of the leading sample ($D4D1).
        assert_eq!(
            resolve(&room, 8, 72, Direction::Up, 1),
            Err(Unqualified::SampleOutOfBounds)
        );
    }

    #[test]
    fn old_slopes_mirror_step2_and_flag_precedence_in_all_directions() {
        for (kind, d, x, y, col, row, expected) in [
            (6, Direction::Up, 63, 72, 3, 3, (66, 70, false)),
            (7, Direction::Up, 65, 72, 4, 3, (62, 70, false)),
            (7, Direction::Down, 63, 56, 3, 3, (66, 58, false)),
            (6, Direction::Down, 65, 56, 4, 3, (62, 58, false)),
            (6, Direction::Left, 64, 71, 3, 3, (62, 74, false)),
            (7, Direction::Left, 64, 73, 3, 4, (62, 70, false)),
            (7, Direction::Right, 48, 71, 3, 3, (50, 74, false)),
            (6, Direction::Right, 48, 73, 3, 4, (50, 70, false)),
        ] {
            for flags in [0, 0x4000, 0x8000, 0xc000] {
                // Bits14/15 and metatile index must not change stored old slope dispatch.
                let room = grid(&[(col, row, (kind << 9) | flags | 0x1ab)]);
                assert_eq!(
                    resolve(&room, x, y, d, 2),
                    Ok(expected),
                    "{d:?} type{kind} flags{flags:x}"
                );
            }
        }
    }

    #[test]
    fn vertical_second_slope_diagonal_probe_is_not_the_mirror_type() {
        for (kind, d, y, neighbor_row, accepted, rejected) in [
            (7, Direction::Up, 72, 4, 6, 7),
            (6, Direction::Down, 56, 2, 7, 6),
        ] {
            let baseline = resolve(&grid(&[(4, 3, kind << 9)]), 65, y, d, 1).unwrap();
            assert!(!baseline.2);
            let same = grid(&[(4, 3, kind << 9), (3, neighbor_row, accepted << 9)]);
            assert_eq!(resolve(&same, 65, y, d, 1), Ok(baseline));
            let opposite = grid(&[(4, 3, kind << 9), (3, neighbor_row, rejected << 9)]);
            assert!(resolve(&opposite, 65, y, d, 1).unwrap().2);
        }
    }

    #[test]
    fn source_sum_threshold_is_seventeen_not_sixteen() {
        let room = grid(&[(3, 4, 6 << 9)]);
        // Right6: new positive-edge remainder8 + top remainder8 ==16 -> pass;
        // remainder9 +8 ==17 -> climb1. Cell lookup still uses edge-1.
        assert_eq!(
            resolve(&room, 47, 72, Direction::Right, 1),
            Ok((48, 72, false))
        );
        assert_eq!(
            resolve(&room, 48, 72, Direction::Right, 1),
            Ok((49, 71, false))
        );
    }

    #[test]
    fn row_stride_is_room_width_not_height_or_player_extent() {
        for (width, height) in [(7, 9), (9, 7)] {
            let mut cells = vec![0; usize::from(width) * usize::from(height)];
            cells[4 * usize::from(width) + 3] = 6 << 9;
            let room = Room::new(width, height, cells)
                .unwrap()
                .with_passive_directional_collision();
            assert_eq!(
                resolve(&room, 48, 73, Direction::Right, 1),
                Ok((49, 71, false))
            );
        }
    }
}
