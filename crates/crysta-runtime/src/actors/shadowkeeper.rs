//! Rust models of Shadowkeeper's native code (`$93:D876`..`$93:E8BC`,
//! European `$99:9B5E`..; `docs/tower-five.md`): its parts follow, place
//! and order each other through the actor list (`+$2E`, the next entity)
//! and a sine table. Offsets are the Japanese image's.

use super::{Actor, Poke, Surroundings};

/// The sine table (`$81:F422`, signed, 256 a turn) and its cosine, a
/// quarter on (`$81:F462`).
const SINE: usize = 0x01_F422;
const COSINE: usize = 0x01_F462;
/// The tail: its controller's per-frame placement (`JSR $E53B`), the
/// segments it leads, and its phases' list walks (`$93:DD38`..`DE10`).
const TAIL_PLACE: usize = 0x13_DD06;
const TAIL_PHASES: usize = 0x13_DD09;
const TAIL_PRIORITY: usize = 0x13_DD38;
const TAIL_HEAD_TO: usize = 0x13_DD50;
const TAIL_HIDE: usize = 0x13_DD5D;
const TAIL_SHOW: usize = 0x13_DD80;
const TAIL_HEAD_SWING: usize = 0x13_DDC0;
const TAIL_HEAD_STING: usize = 0x13_DDCD;
const TAIL_BREAK: usize = 0x13_DDDA;
const TAIL_BREAK_WAIT: usize = 0x13_DDE7;
const TAIL_BREAK_NEXT: usize = 0x13_DDEB;
const TAIL_BREAK_DONE: usize = 0x13_DE12;
const TAIL_LOOP: usize = 0x13_DD04;
/// The scripts the controller points the tail's head at (`STA $000A,Y`).
const HEAD_ORBIT: u16 = 0xDE8A;
const HEAD_SWING: u16 = 0xDEAF;
const HEAD_STING: u16 = 0xDF4E;
const SEGMENT_BREAKS: u16 = 0xDE12;
/// The tail's head circling its parent (`$93:DE4A`..`DE89`).
const HEAD_CIRCLE: usize = 0x13_DE4A;
const HEAD_CIRCLE_END: usize = 0x13_DE89;
/// The tail's head swaying above its parent before a sting (`$93:DF56`):
/// at the sway's ends (`$DC`, `$5C`) by Ark's side, it stings
/// (`$93:DEF7`).
const HEAD_SWAY: usize = 0x13_DF56;
const HEAD_SWAY_END: usize = 0x13_DF8E;
const HEAD_STINGS: usize = 0x13_DEF7;
/// The segments the walks take: the head and ten more.
const SEGMENTS: usize = 11;
/// The claw holder's phases (`$93:E060`'s table): both claws and itself
/// behind the high tiles; a claw toward Ark's side (`$0958`) swipes; the
/// claw by the holder's torch row toggles its torch (`$E45F`).
const HOLDER_SINK: usize = 0x13_E086;
const HOLDER_LOOP: usize = 0x13_E03B;
const HOLDER_SIDE: usize = 0x13_E0C9;
const HOLDER_LEFT: usize = 0x13_E0D1;
const HOLDER_RIGHT: usize = 0x13_E0E0;
const HOLDER_TORCH: usize = 0x13_E0ED;
const HOLDER_TORCH_LEFT: usize = 0x13_E124;
const HOLDER_TORCH_RIGHT: usize = 0x13_E13D;
const CLAW_PUTS_OUT: u16 = 0xE2DA;
/// The torches' rows (`$93:E147`, six words) and their bits (`$93:E534`).
const TORCH_ROWS: usize = 0x13_E147;
const TORCH_BITS: usize = 0x13_E534;
const ROWS: usize = 6;
/// The body's tests that answer in the carry, each followed by a `BCC` or
/// `BCS` (`$93:DA3D`, `DA9D`, `DC08`): its y at least `$390` (`$E6AE`),
/// at least `$C0` (`$E6B5`); Ark's safe place level with its mouth and not
/// under it (`$E682`).
const TESTS: [usize; 3] = [0x13_DA3D, 0x13_DA9D, 0x13_DC08];
/// The body's count in `$0498` (`INC` at stage 2, `$93:DA4D`; `STZ` at its
/// last death, `$93:DB61`): the end controller waits for none.
const BODY_COUNTS: usize = 0x13_DA4D;
const BODY_COUNTED: usize = 0x13_DA50;
const BODY_UNCOUNTS: usize = 0x13_DB61;
const BODY_UNCOUNTED: usize = 0x13_DB64;
/// The body's main loop: by a lit torch's row (`JSR $E6BC; BEQ`).
const BODY_TORCH: usize = 0x13_D9C2;
const BODY_TORCH_YES: usize = 0x13_D9C7;
const BODY_TORCH_NO: usize = 0x13_D9CC;

impl Actor {
    /// The routine of Shadowkeeper's at `at`, if modelled: where the script
    /// goes on.
    pub(super) fn shadowkeeper(
        &mut self,
        at: usize,
        around: &mut Surroundings<'_>,
    ) -> Option<usize> {
        self.tail_routine(at, around)
            .or_else(|| self.holder_routine(at, around))
            .or_else(|| self.body_routine_at(at, around))
    }

    /// The image offset of a Japanese address of Shadowkeeper's code in the
    /// script's bank ([`local`]).
    fn in_bank(&self, image: &[u8], address: u16) -> usize {
        (self.pc & 0xFF_0000) | usize::from(local(image, address))
    }

    /// The tail controller's and its head's routines.
    fn tail_routine(&mut self, at: usize, around: &mut Surroundings<'_>) -> Option<usize> {
        let chain = |around: &Surroundings<'_>, from: u16| chain(around, from, SEGMENTS);
        Some(match at {
            TAIL_PLACE => {
                self.place_tail(around);
                TAIL_PHASES
            }
            TAIL_PRIORITY => {
                for id in chain(around, self.id).into_iter().chain([self.id]) {
                    around
                        .globals
                        .pokes
                        .push(Poke::Priority { id, priority: 0 });
                }
                TAIL_LOOP
            }
            TAIL_HEAD_TO | TAIL_HEAD_SWING | TAIL_HEAD_STING => {
                let target = match at {
                    TAIL_HEAD_TO => HEAD_ORBIT,
                    TAIL_HEAD_SWING => HEAD_SWING,
                    _ => HEAD_STING,
                };
                if let Some(&head) = chain(around, self.id).first() {
                    // `STA $000A,Y` alone: it goes on there when it next runs.
                    let pc = self.in_bank(around.image, target);
                    around.globals.pokes.push(Poke::Script { id: head, pc });
                }
                TAIL_LOOP
            }
            TAIL_HIDE | TAIL_SHOW => {
                let hidden = at == TAIL_HIDE;
                let place = (!hidden)
                    .then(|| self.parent_place(around))
                    .flatten()
                    .map(|(x, y)| (x, y.wrapping_sub(0x58)));
                let (set, cleared) = if hidden { (0x8000, 0) } else { (0, 0x8000) };
                for id in chain(around, self.id) {
                    around.globals.pokes.push(Poke::Flags { id, set, cleared });
                    if let Some(at) = place {
                        around.globals.pokes.push(Poke::Place { id, at });
                    }
                }
                self.hidden = hidden;
                TAIL_LOOP
            }
            TAIL_BREAK => {
                self.set_own_word(0x201C, 10);
                let first = next(around, self.id).unwrap_or(0);
                self.set_own_word(0x26, first);
                TAIL_BREAK_WAIT
            }
            TAIL_BREAK_NEXT => {
                let id = self.own_word(0x26);
                if id != 0 {
                    let pc = self.in_bank(around.image, SEGMENT_BREAKS);
                    around.globals.pokes.push(Poke::Script { id, pc });
                }
                self.set_own_word(0x26, next(around, id).unwrap_or(0));
                let left = self.own_word(0x201C).wrapping_sub(1);
                self.set_own_word(0x201C, left);
                if left.cast_signed() >= 0 {
                    TAIL_BREAK_WAIT
                } else {
                    TAIL_BREAK_DONE
                }
            }
            HEAD_SWAY => {
                if self.sway(around) {
                    HEAD_SWAY_END
                } else {
                    HEAD_STINGS
                }
            }
            HEAD_CIRCLE => {
                self.circle_parent(around);
                HEAD_CIRCLE_END
            }
            _ => return None,
        })
    }

    /// The claw holder's routines.
    fn holder_routine(&mut self, at: usize, around: &mut Surroundings<'_>) -> Option<usize> {
        let word = |around: &Surroundings<'_>, at| around.globals.scratch.get(&at).copied();
        Some(match at {
            HOLDER_SINK => {
                for field in [0x14, 0x16] {
                    let id = self.own_word(field);
                    around
                        .globals
                        .pokes
                        .push(Poke::Priority { id, priority: 0 });
                }
                self.priority = 0;
                HOLDER_LOOP
            }
            HOLDER_SIDE => {
                if word(around, super::SAFE_X).unwrap_or(0) >= self.position.0 {
                    HOLDER_RIGHT
                } else {
                    HOLDER_LEFT
                }
            }
            HOLDER_TORCH => {
                let shift = self.pc - at;
                let Some(row) = torch_row(around.image, shift, self.position.1) else {
                    return Some(HOLDER_LOOP);
                };
                let field = if row & 1 == 0 { 0x14 } else { 0x16 };
                let id = self.own_word(field);
                if id == 0 {
                    return Some(HOLDER_LOOP);
                }
                let pc = self.in_bank(around.image, CLAW_PUTS_OUT);
                around.globals.pokes.push(Poke::Script { id, pc });
                if field == 0x14 {
                    HOLDER_TORCH_LEFT
                } else {
                    HOLDER_TORCH_RIGHT
                }
            }
            _ => return None,
        })
    }

    /// The body's routines: its tests in the main loop.
    fn body_routine_at(&mut self, at: usize, around: &mut Surroundings<'_>) -> Option<usize> {
        let shift = self.pc - at;
        if at == BODY_COUNTS || at == BODY_UNCOUNTS {
            let hold = &mut around.globals.enemy_hold;
            if at == BODY_COUNTS {
                *hold += 1;
            } else {
                *hold = 0;
                if let Some(foe) = &mut self.foe {
                    foe.counted = false;
                }
            }
            return Some(if at == BODY_COUNTS {
                BODY_COUNTED
            } else {
                BODY_UNCOUNTED
            });
        }
        if at == BODY_TORCH {
            let torches = around.globals.scratch.get(&super::TORCHES).copied();
            let lit = torch_row(around.image, shift, self.position.1).is_some_and(|row| {
                around
                    .image
                    .get(TORCH_BITS + shift + row)
                    .is_some_and(|&bit| u16::from(bit) & torches.unwrap_or(0) != 0)
            });
            return Some(if lit { BODY_TORCH_YES } else { BODY_TORCH_NO });
        }
        if !TESTS.contains(&at) {
            return None;
        }
        let carry = self.body_test(around);
        let branch = around.image.get(self.pc + 3..self.pc + 5)?;
        let taken = match branch[0] {
            0x90 => !carry,
            0xB0 => carry,
            _ => return None,
        };
        let next = self.pc + 5 - shift;
        Some(if taken {
            next.wrapping_add_signed(isize::from(branch[1].cast_signed()))
        } else {
            next
        })
    }

    /// The carry of the body's test the `JSR` at the script calls.
    fn body_test(&self, around: &Surroundings<'_>) -> bool {
        let (x, y) = self.position;
        let image = around.image;
        let target = image
            .get(self.pc + 1..self.pc + 3)
            .map(|word| u16::from_le_bytes([word[0], word[1]]));
        match target {
            Some(at) if at == local(image, 0xE6AE) => y >= 0x390,
            Some(at) if at == local(image, 0xE6B5) => y >= 0xC0,
            _ => {
                let word = |at| around.globals.scratch.get(&at).copied().unwrap_or(0);
                let (safe_x, safe_y) = (word(super::SAFE_X), word(super::SAFE_Y));
                (y + 0x30).abs_diff(safe_y) < 0x10 && x.abs_diff(safe_x) >= 0x10
            }
        }
    }

    /// The parent's place (`$7F:102E`).
    pub(super) fn parent_place(&self, around: &Surroundings<'_>) -> Option<(u16, u16)> {
        view(around, self.parent_id()?)
    }

    /// `$93:E53B`: the tail's controller 80 pixels above its parent; while
    /// `$04AA` is not `$FFFF` it sets it to 1 and lays the segments after
    /// its head on an arc from itself to the head (`$93:E568`).
    fn place_tail(&mut self, around: &mut Surroundings<'_>) {
        let Some((x, y)) = self.parent_place(around) else {
            return;
        };
        self.position = (x, y.wrapping_sub(0x50));
        // `COP 22 01 06` on `$04AA + 1`: stage 0 returns (`$E567`), 1 to 5
        // lay the arc and set it to 1 (`$E568`).
        let stage = around.globals.scratch.entry(0x04AA).or_insert(0);
        if !(1..=5).contains(stage) {
            return;
        }
        *stage = 1;
        let segments = chain(around, self.id, usize::from(self.own_word(0x24)));
        let Some((&head, rest)) = segments.split_first() else {
            return;
        };
        let Some(head) = view(around, head) else {
            return;
        };
        let count = self.own_word(0x24).wrapping_sub(1) & 0xFF;
        let arc = Arc {
            centre: (self.position.0, head.1),
            radius: (
                self.position.0.abs_diff(head.0),
                head.1.abs_diff(self.position.1),
            ),
            mirrored: self.position.0.wrapping_sub(head.0).cast_signed() < 0,
        };
        let step = arc_step(count);
        let mut angle = 0xC000u16.wrapping_sub(step);
        for &id in rest.iter().take(usize::from(count)) {
            let at = arc.place(around.image, (angle >> 8) as u8);
            around.globals.pokes.push(Poke::Place { id, at });
            angle = angle.wrapping_sub(step);
        }
    }

    /// `$93:DF56`: the head 32 above its parent, swaying on an ellipse of
    /// 50 by 25 two steps a frame; false when it stings instead (`$E62A`,
    /// `$E64B`: at an end of the sway, Ark past it and level with it).
    fn sway(&mut self, around: &Surroundings<'_>) -> bool {
        let Some((x, y)) = self.parent_place(around) else {
            return true;
        };
        let centre = (x, y.wrapping_sub(0x20));
        let word = |at| around.globals.scratch.get(&at).copied().unwrap_or(0);
        let (safe_x, ark_y) = (word(super::SAFE_X), word(super::PLAYER_Y));
        let angle = self.own_word(0x24) & 0xFF;
        let level = || (centre.1 + 0x30).abs_diff(ark_y) < 0x18;
        let stings = (angle.abs_diff(0xDC) < 5 && centre.0 < safe_x && level())
            || (angle.abs_diff(0x5C) < 5 && safe_x < centre.0 && level());
        if stings {
            return false;
        }
        let at = self.own_word(0x24);
        self.set_own_word(0x24, at.wrapping_add(2));
        self.position = ellipse(
            around.image,
            centre,
            (0x32, 0x19),
            ((at & 0xFF) as u8, ((at << 1) & 0xFF) as u8),
        );
        true
    }

    /// `$93:DE4A`: the tail's head on an ellipse 32 left of and 20 above its
    /// parent, its angle 4 on each frame, its radius growing to 31.
    fn circle_parent(&mut self, around: &mut Surroundings<'_>) {
        let Some((x, y)) = self.parent_place(around) else {
            return;
        };
        let centre = (x.wrapping_sub(0x20), y.wrapping_sub(0x14));
        let angle = self.own_word(0x24);
        let next = angle.wrapping_add(4) & 0xFF;
        self.set_own_word(0x24, next);
        let radius = self.own_word(0x26);
        if radius + 1 < 0x20 {
            self.set_own_word(0x26, radius + 1);
        }
        let radius = (radius & 0xFF) as u8;
        self.position = ellipse(
            around.image,
            centre,
            (radius, radius),
            ((angle & 0xFF) as u8, (next.wrapping_add(0x20) & 0xFF) as u8),
        );
    }
}

/// The torches' row within 16 pixels of `y` (`$93:E6BC`, `$93:E0ED`).
fn torch_row(image: &[u8], shift: usize, y: u16) -> Option<usize> {
    (0..ROWS).find(|&row| {
        let at = TORCH_ROWS + shift + 2 * row;
        image
            .get(at..at + 2)
            .is_some_and(|word| y.abs_diff(u16::from_le_bytes([word[0], word[1]])) < 0x10)
    })
}

/// A bank-relative address of Shadowkeeper's code in the image's revision:
/// the European one moved it `$3D18` down (`$93:E703` is `$99:A9EB`).
pub(super) fn local(image: &[u8], address: u16) -> u16 {
    assets::layout::per_revision(image, address, address.wrapping_sub(0x3D18))
}

/// An actor's view by id.
fn view(around: &Surroundings<'_>, id: u16) -> Option<(u16, u16)> {
    let view = around.globals.views.iter().find(|(at, _)| *at == id)?.1;
    Some((view.x, view.y))
}

/// The entity after `id` in the list (`+$2E`): the one whose previous it is.
fn next(around: &Surroundings<'_>, id: u16) -> Option<u16> {
    around
        .globals
        .views
        .iter()
        .find(|(_, view)| view.previous == Some(id))
        .map(|&(id, _)| id)
}

/// The `count` entities after `from` in the list.
fn chain(around: &Surroundings<'_>, from: u16, count: usize) -> Vec<u16> {
    let mut ids = Vec::with_capacity(count);
    let mut at = from;
    while ids.len() < count {
        let Some(id) = next(around, at) else {
            break;
        };
        ids.push(id);
        at = id;
    }
    ids
}

/// A signed table byte.
fn table(image: &[u8], at: usize, index: u8) -> i8 {
    image
        .get(at + usize::from(index))
        .map_or(0, |&byte| byte.cast_signed())
}

/// `$86:81B0`'s product of a radius and a table byte's size, and the byte's
/// sign.
fn product(radius: u8, byte: i8) -> (u16, bool) {
    (u16::from(radius) * u16::from(byte.unsigned_abs()), byte < 0)
}

/// `$93:E854`: a place `x - 2(rx·cos a >> 8)`, `y + 2(ry·sin b >> 8)`
/// around `centre`, the sizes rounded down before doubling.
fn ellipse(image: &[u8], centre: (u16, u16), radius: (u8, u8), angle: (u8, u8)) -> (u16, u16) {
    let offset = |(product, negative): (u16, bool)| {
        let size = (product >> 8) << 1;
        if negative {
            size.cast_signed()
        } else {
            -size.cast_signed()
        }
    };
    let x = offset(product(radius.0, table(image, COSINE, angle.0)));
    let y = -offset(product(radius.1, table(image, SINE, angle.1)));
    (
        centre.0.wrapping_add_signed(x),
        centre.1.wrapping_add_signed(y),
    )
}

/// The arc the tail's segments lie on (`$93:E568`): from the controller to
/// the head, mirrored when the head is to the right.
struct Arc {
    centre: (u16, u16),
    radius: (u16, u16),
    mirrored: bool,
}

impl Arc {
    /// `$93:E76E`, or `$E7E2` mirrored: x ± (rx·sin a >> 7), y + (ry·cos a
    /// >> 7).
    fn place(&self, image: &[u8], angle: u8) -> (u16, u16) {
        let signed = |(product, negative): (u16, bool)| {
            let size = (product >> 7) & 0x1FF;
            if negative {
                -size.cast_signed()
            } else {
                size.cast_signed()
            }
        };
        let rx = (self.radius.0 & 0xFF) as u8;
        let ry = (self.radius.1 & 0xFF) as u8;
        let mut x = signed(product(rx, table(image, SINE, angle)));
        if self.mirrored {
            x = -x;
        }
        let y = signed(product(ry, table(image, COSINE, angle)));
        (
            self.centre.0.wrapping_add_signed(x),
            self.centre.1.wrapping_add_signed(y),
        )
    }
}

/// `$93:E5A3`: a quarter turn in 8.8 parts over `count` (`$4000 / count`
/// by two of `$86:81C7`'s divisions).
fn arc_step(count: u16) -> u16 {
    let count = count & 0xFF;
    if count == 0 {
        return 0;
    }
    let (whole, rest) = (0x40 / count, 0x40 % count);
    (whole << 8) | (((rest << 8) / count) & 0xFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ROM's quarter wave (`$81:F422`): `sin` and `cos` from it.
    const QUARTER: [u8; 65] = [
        0x00, 0x03, 0x06, 0x09, 0x0C, 0x0F, 0x12, 0x15, 0x18, 0x1C, 0x1F, 0x22, 0x25, 0x28, 0x2B,
        0x2E, 0x30, 0x33, 0x36, 0x39, 0x3C, 0x3F, 0x41, 0x44, 0x47, 0x49, 0x4C, 0x4E, 0x51, 0x53,
        0x55, 0x58, 0x5A, 0x5C, 0x5E, 0x60, 0x62, 0x64, 0x66, 0x68, 0x6A, 0x6C, 0x6D, 0x6F, 0x70,
        0x72, 0x73, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x7B, 0x7C, 0x7C, 0x7D, 0x7E, 0x7E, 0x7F,
        0x7F, 0x7F, 0x7F, 0x7F, 0x7F,
    ];

    fn tables() -> Vec<u8> {
        let mut image = vec![0; COSINE + 0x100];
        for index in 0..0x140 {
            let (quarter, rest) = ((index / 64) % 4, index % 64);
            let size = if quarter % 2 == 0 {
                QUARTER[rest]
            } else {
                QUARTER[64 - rest]
            };
            image[SINE + index] = if quarter < 2 {
                size
            } else {
                size.wrapping_neg()
            };
        }
        image
    }

    #[test]
    fn the_quarter_turn_is_shared_out_in_eighths_of_a_pixel() {
        assert_eq!(arc_step(10), 0x0666);
        assert_eq!(arc_step(4), 0x1000);
        assert_eq!(arc_step(0), 0);
    }

    #[test]
    fn the_ellipse_runs_left_from_the_centre_at_angle_zero() {
        let image = tables();
        // cos 0 = 127: x = 100 - 2(32·127 >> 8) = 70; sin 0 = 0.
        assert_eq!(ellipse(&image, (100, 100), (32, 32), (0, 0)), (70, 100));
        // A quarter on: cos 64 = 0, sin 64 = 127 for y.
        assert_eq!(ellipse(&image, (100, 100), (32, 32), (64, 64)), (100, 130));
    }

    #[test]
    fn the_arc_mirrors_its_x_only() {
        let image = tables();
        let arc = |mirrored| Arc {
            centre: (100, 100),
            radius: (40, 40),
            mirrored,
        };
        // sin 64 = 127, cos 64 = 0: x = 100 ± (40·127 >> 7) = 100 ± 39.
        assert_eq!(arc(false).place(&image, 64), (139, 100));
        assert_eq!(arc(true).place(&image, 64), (61, 100));
    }
}
