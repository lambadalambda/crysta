//! The `COP` sites scripts reach, found by a static walk
//! (`docs/cop-inventory.md`).
//!
//! The walk starts at the scripts a map's spawn list installs and follows
//! the services' fall-through, their branch and jump targets, the scripts
//! they spawn and the native code between them. It executes nothing, so
//! both arms of every test are taken. Where it cannot know how a script
//! goes on it records a [`Stop`] and leaves that path.

use crate::actors::{handling, Handling};
use assets::cpu::{self, Widths};
use assets::maps::actor_script;
use assets::maps::actors::SpawnList;
use std::collections::{BTreeMap, BTreeSet};

/// One service in the [`census`]: where it is and what the runtime does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    /// What the runtime does with it.
    pub handling: Handling,
    /// Its sites, as normalized offsets of the `COP`.
    pub sites: BTreeSet<usize>,
    /// The maps whose scripts reach it.
    pub maps: BTreeSet<u16>,
}

/// The services, native routines and stops of every script that `maps`'
/// spawn lists install, Ark's own excepted (the runtime models him
/// natively).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Census {
    /// The spawn lists' scripts walked.
    pub scripts: usize,
    /// Each service found.
    pub services: BTreeMap<u8, Service>,
    /// The native routines scripts call, with the maps that reach them.
    pub calls: BTreeMap<usize, BTreeSet<u16>>,
    /// Where the walk stopped.
    pub stops: BTreeMap<usize, Stop>,
}

/// Walks each map's scripts.
#[must_use]
pub fn census(image: &[u8], maps: &[u16]) -> Census {
    let mut census = Census::default();
    let mut scripts = BTreeSet::new();
    for &map in maps {
        let Ok(list) = SpawnList::from_rom(image, map) else {
            continue;
        };
        let seeds: Vec<usize> = list
            .records()
            .iter()
            .filter(|record| !record.installs_player(image))
            .filter_map(|record| long(&record.script()?.to_le_bytes()[..3]))
            .collect();
        scripts.extend(seeds.iter().copied());
        let walk = walk(image, &seeds);
        for (service, sites) in walk.sites {
            let entry = census.services.entry(service).or_insert_with(|| Service {
                handling: handling(image, service),
                sites: BTreeSet::new(),
                maps: BTreeSet::new(),
            });
            entry.sites.extend(sites);
            entry.maps.insert(map);
        }
        for routine in walk.calls {
            census.calls.entry(routine).or_default().insert(map);
        }
        census.stops.extend(walk.stops);
    }
    census.scripts = scripts.len();
    census
}

/// A normalized offset as the `$BB:AAAA` address scripts run at.
fn address(offset: usize) -> String {
    format!("${:02X}:{:04X}", 0x80 | (offset >> 16), offset & 0xFFFF)
}

/// Maps as `$100`-`$105`, `$10C` runs.
fn map_runs(maps: &BTreeSet<u16>) -> String {
    let mut runs: Vec<(u16, u16)> = Vec::new();
    for &map in maps {
        match runs.last_mut() {
            Some((_, last)) if *last + 1 == map => *last = map,
            _ => runs.push((map, map)),
        }
    }
    runs.iter()
        .map(|&(first, last)| {
            if first == last {
                format!("`${first:03X}`")
            } else {
                format!("`${first:03X}`-`${last:03X}`")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The census as the Markdown of `docs/cop-inventory.md`.
#[must_use]
pub fn markdown(census: &Census) -> String {
    let count = |handling: Handling| {
        census
            .services
            .values()
            .filter(|service| service.handling == handling)
            .count()
    };
    let sites: usize = census
        .services
        .values()
        .map(|service| service.sites.len())
        .sum();
    let mut lines = vec![
        INTRO.to_owned(),
        format!(
            "{} scripts, {} services at {sites} sites: {} modelled, {} stepped over, {} that stop the actor. {} native routines called, {} walk stops.",
            census.scripts,
            census.services.len(),
            count(Handling::Modelled),
            count(Handling::SteppedOver),
            count(Handling::Stops),
            census.calls.len(),
            census.stops.len(),
        ),
        "\n## Services\n\n| COP | Runtime | Sites | Maps |\n|---|---|---|---|".to_owned(),
    ];
    lines.extend(census.services.iter().map(|(code, service)| {
        let handling = match service.handling {
            Handling::Modelled => "modelled",
            Handling::SteppedOver => "stepped over",
            Handling::Stops => "**stops**",
        };
        format!(
            "| `{code:02X}` | {handling} | {} | {} |",
            service.sites.len(),
            map_runs(&service.maps)
        )
    }));
    lines.push("\n## Sites of the services not modelled\n".to_owned());
    lines.extend(
        census
            .services
            .iter()
            .filter(|(_, service)| service.handling != Handling::Modelled)
            .map(|(code, service)| {
                let sites: Vec<String> = service
                    .sites
                    .iter()
                    .map(|&at| format!("`{}`", address(at)))
                    .collect();
                format!("- `COP {code:02X}`: {}", sites.join(", "))
            }),
    );
    lines
        .push("\n## Native routines called with `JSL`\n\n| Routine | Maps |\n|---|---|".to_owned());
    lines.extend(
        census
            .calls
            .iter()
            .map(|(routine, maps)| format!("| `{}` | {} |", address(*routine), map_runs(maps))),
    );
    lines.push("\n## Walk stops\n".to_owned());
    if census.stops.is_empty() {
        lines.push("None.".to_owned());
    }
    lines.extend(
        census
            .stops
            .iter()
            .map(|(at, stop)| format!("- `{}`: {stop:?}", address(*at))),
    );
    lines.join("\n") + "\n"
}

/// The hand-written head of `docs/cop-inventory.md`.
const INTRO: &str = "# COP services in the underworld scripts

Generated by `crysta_runtime::service_sites` from the Japanese ROM; the
local test `the_cop_inventory_is_current` checks it, and
`CRYSTA_WRITE_INVENTORY=1` rewrites it. Addresses are Japanese.

The walk starts at each script that the spawn lists of the underworld
maps the runtime loads (`TOWER_MAPS`, `WORLD_MAPS`) install, Ark's own
excepted. It follows each service's fall-through and its targets (the
runtime's operand layout where it models the service, else the native
handler's), the scripts spawns start, `JSR` subroutines and the native code
between the services. It takes both arms of every test and runs nothing, so
it finds code that a played game may never reach. Native routines called
with `JSL` are listed, not walked.

\"Modelled\" means the runtime has a handler; some handlers refuse rare
operands (`meta/issues/partial-cop-services.md`). \"Stepped over\" means the
runtime skips the service by its native operand length: right for a service
that only changes the picture or the sound, a gap otherwise. \"Stops\" means
the runtime does not know the length and freezes the actor.
";

/// Where a walk could not go on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stop {
    /// A service whose operand length is not known.
    Service(u8),
    /// A native instruction the walk does not follow: an indirect jump, or
    /// `PLP`/`XCE`, which leave the register widths unknown.
    Native(u8),
    /// A target outside the ROM.
    Target,
    /// The walk's budget ran out before this path.
    Budget,
}

/// What a walk found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Walk {
    /// Each service's sites, as normalized offsets of the `COP`.
    pub sites: BTreeMap<u8, BTreeSet<usize>>,
    /// The native routines scripts call with `JSL`, by normalized offset.
    pub calls: BTreeSet<usize>,
    /// Where paths stopped, by normalized offset.
    pub stops: BTreeMap<usize, Stop>,
}

/// Instructions one walk may decode before it gives up.
const BUDGET: usize = 200_000;

/// Walks every script reachable from `seeds` (normalized offsets).
#[must_use]
pub fn walk(image: &[u8], seeds: &[usize]) -> Walk {
    let mut found = Walk::default();
    let mut seen = BTreeSet::new();
    let mut queue: Vec<(usize, Widths)> = seeds.iter().map(|&at| (at, Widths::native())).collect();
    let mut steps = 0;
    while let Some((start, widths)) = queue.pop() {
        let mut path = Path { at: start, widths };
        while seen.insert(path.at) {
            if steps == BUDGET {
                found.stops.insert(path.at, Stop::Budget);
                break;
            }
            steps += 1;
            match path.step(image, &mut found, &mut queue) {
                Some(next) => path = next,
                None => break,
            }
        }
    }
    found
}

/// A point in a script and the register widths there.
#[derive(Debug, Clone, Copy)]
struct Path {
    at: usize,
    widths: Widths,
}

impl Path {
    /// Records the instruction here and returns the next one on this path;
    /// targets on other paths go onto `queue`.
    fn step(
        self,
        image: &[u8],
        found: &mut Walk,
        queue: &mut Vec<(usize, Widths)>,
    ) -> Option<Self> {
        let Self { at, mut widths } = self;
        let opcode = *image.get(at)?;
        let bank = at & 0xFF_0000;
        let word = |offset: usize| {
            image
                .get(offset..offset + 2)
                .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        };
        let mut jump = |target: Option<usize>, found: &mut Walk| {
            if let Some(target) = reached(target, at, found) {
                queue.push((target, widths));
            }
        };
        let next = match opcode {
            0x02 => {
                let service = *image.get(at + 1)?;
                found.sites.entry(service).or_default().insert(at);
                let shape = shape(image, service, at + 2);
                for target in shape.targets {
                    jump(Some(target), found);
                }
                queue.extend(
                    shape
                        .scripts
                        .iter()
                        .map(|&script| (script, Widths::native())),
                );
                match shape.next {
                    Next::At(next) => next,
                    Next::Never => return None,
                    Next::Unknown => {
                        found.stops.insert(at, Stop::Service(service));
                        return None;
                    }
                }
            }
            // RTS, RTL, RTI end the path.
            0x60 | 0x6B | 0x40 => return None,
            // BRA, BRL, JMP and JML go on at their target.
            0x80 => {
                let displacement = i16::from(i8::from_le_bytes([*image.get(at + 1)?]));
                reached(relative(displacement, at + 2), at, found)?
            }
            0x82 => {
                let displacement = i16::from_le_bytes([*image.get(at + 1)?, *image.get(at + 2)?]);
                reached(relative(displacement, at + 3), at, found)?
            }
            0x4C => reached(in_bank(bank, word(at + 1)?), at, found)?,
            0x5C => reached(long(image.get(at + 1..at + 4)?), at, found)?,
            // Conditional branches fork.
            0x10 | 0x30 | 0x50 | 0x70 | 0x90 | 0xB0 | 0xD0 | 0xF0 => {
                let displacement = i16::from(i8::from_le_bytes([*image.get(at + 1)?]));
                jump(relative(displacement, at + 2), found);
                at + 2
            }
            // JSR walks the subroutine too; JSL is a native routine.
            0x20 => {
                jump(in_bank(bank, word(at + 1)?), found);
                at + 3
            }
            0x22 => {
                if let Some(routine) = long(image.get(at + 1..at + 4)?) {
                    found.calls.insert(routine);
                }
                at + 4
            }
            0x6C | 0x7C | 0xDC | 0xFC | 0x28 | 0xFB | 0x00 | 0xDB => {
                found.stops.insert(at, Stop::Native(opcode));
                return None;
            }
            _ => cpu::step(image, at, &mut widths)?,
        };
        Some(Self { at: next, widths })
    }
}

/// How a service leaves the stream.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Shape {
    next: Next,
    /// Normalized offsets the script may run next instead.
    targets: Vec<usize>,
    /// Scripts started at a long address, with the widths a script starts
    /// with.
    scripts: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Next {
    /// The fall-through: the next instruction's offset.
    At(usize),
    /// The service never falls through.
    Never,
    /// Not known.
    Unknown,
}

/// The shape of the service whose operands start at `operands`: the
/// runtime's layout where it models one, else its native handler's.
fn shape(image: &[u8], service: u8, operands: usize) -> Shape {
    let bank = operands & 0xFF_0000;
    if let Some(shape) = modelled_shape(image, service, operands) {
        return shape;
    }
    let Some(resumes) = actor_script::resumes(image, service) else {
        return Shape {
            next: Next::Unknown,
            targets: Vec::new(),
            scripts: Vec::new(),
        };
    };
    let targets = resumes
        .targets
        .iter()
        .filter_map(|&offset| {
            let bytes = image.get(operands + offset..operands + offset + 2)?;
            in_bank(bank, u16::from_le_bytes([bytes[0], bytes[1]]))
        })
        .collect();
    let next = match resumes.lengths[..] {
        [length] => Next::At(operands + length),
        [] if !resumes.targets.is_empty() => Next::Never,
        _ => Next::Unknown,
    };
    Shape {
        next,
        targets,
        scripts: Vec::new(),
    }
}

/// The runtime's fixed layouts: a service, its operand length and the
/// operand offsets of its word targets. `COP 2E`'s target is the native
/// one; the runtime never takes it. `COP CB` and `DF` name scripts for
/// Ark, whom the runtime models natively, so they are not followed.
const FIXED: &[(u8, usize, &[usize])] = &[
    (0x02, 2, &[]),
    (0x03, 0, &[]),
    (0x04, 0, &[]),
    (0x05, 2, &[]),
    (0x07, 2, &[]),
    (0x08, 4, &[2]),
    (0x0A, 4, &[2]),
    (0x0C, 7, &[5]),
    (0x0D, 7, &[5]),
    (0x0F, 5, &[3]),
    (0x13, 3, &[]),
    (0x14, 8, &[]),
    (0x15, 0, &[]),
    (0x1B, 2, &[]),
    (0x1C, 3, &[]),
    (0x1F, 0, &[]),
    (0x20, 0, &[]),
    (0x23, 2, &[0]),
    (0x24, 3, &[1]),
    (0x25, 0, &[]),
    (0x26, 4, &[]),
    (0x29, 2, &[]),
    (0x2A, 2, &[]),
    (0x2B, 4, &[2]),
    (0x2E, 4, &[2]),
    (0x2F, 4, &[2]),
    (0x30, 1, &[]),
    (0x31, 1, &[]),
    (0x32, 1, &[]),
    (0x33, 0, &[]),
    (0x36, 1, &[]),
    (0x37, 1, &[]),
    (0x38, 2, &[]),
    (0x3B, 0, &[]),
    (0x3D, 3, &[]),
    (0x3E, 3, &[]),
    (0x3F, 3, &[]),
    (0x40, 3, &[]),
    (0x42, 6, &[4]),
    (0x43, 4, &[]),
    (0x44, 4, &[]),
    (0x46, 7, &[]),
    (0x48, 2, &[]),
    (0x49, 2, &[]),
    (0x4A, 5, &[3]),
    (0x54, 3, &[1]),
    (0x55, 1, &[]),
    (0x56, 3, &[1]),
    (0x59, 1, &[]),
    (0x5A, 5, &[]),
    (0x60, 4, &[]),
    (0x63, 2, &[]),
    (0x6A, 2, &[]),
    (0x71, 6, &[4]),
    (0x76, 2, &[]),
    (0x80, 1, &[]),
    (0x81, 1, &[]),
    (0x82, 2, &[]),
    (0x85, 2, &[]),
    (0x86, 2, &[]),
    (0x87, 3, &[]),
    (0x8E, 0, &[]),
    (0x8F, 0, &[]),
    (0xB1, 2, &[]),
    (0xB2, 2, &[]),
    (0xB3, 4, &[]),
    (0xB4, 0, &[]),
    (0xB5, 0, &[]),
    (0xB6, 0, &[]),
    (0xB7, 0, &[]),
    (0xB8, 0, &[]),
    (0xB9, 0, &[]),
    (0xBA, 1, &[]),
    (0xBB, 1, &[]),
    (0xBC, 0, &[]),
    (0xBD, 0, &[]),
    (0xC1, 2, &[]),
    (0xC2, 2, &[0]),
    (0xC3, 2, &[0]),
    (0xC4, 2, &[0]),
    (0xC5, 2, &[0]),
    (0xCB, 4, &[]),
    (0xCC, 5, &[]),
    (0xCD, 0, &[]),
    (0xD0, 16, &[]),
    (0xD1, 0, &[]),
    (0xD2, 5, &[3]),
    (0xD6, 3, &[1]),
    (0xD8, 3, &[]),
    (0xD9, 1, &[]),
    (0xDD, 7, &[]),
    (0xDE, 1, &[]),
    (0xDF, 3, &[]),
    (0xEB, 0, &[]),
    (0xED, 5, &[]),
    (0xEE, 1, &[]),
];

/// The layouts of the services whose handlers [`shape`] cannot read, or
/// whose targets are long, as the runtime's handlers read them
/// (`crate::actors`); `None` for the others.
fn modelled_shape(image: &[u8], service: u8, operands: usize) -> Option<Shape> {
    let bank = operands & 0xFF_0000;
    let byte = |at: usize| image.get(operands + at).copied();
    let word = |at: usize| {
        image
            .get(operands + at..operands + at + 2)
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
    };
    let after = |length: usize| Next::At(operands + length);
    // Word targets by operand offset, long ones by operand offset.
    let (next, words, longs): (Next, Vec<usize>, &[usize]) = match service {
        // The long jump and the redirects.
        0x06 | 0xBE | 0xBF => (Next::Never, vec![], &[0]),
        // The interaction and wall callbacks.
        0x21 | 0xDA => (after(2), vec![0], &[]),
        // The hit target and the long return `COP 66` resumes at.
        0x65 => (after(5), vec![0], &[2]),
        // A return, a frame's end without a continuation, a deletion.
        0x01 | 0x66 | 0x91 | 0xA7 => (Next::Never, vec![], &[]),
        // The spawns by their operand lengths; a call, which `COP 01`
        // returns from, and the continuation.
        0x00 | 0xC0 | 0x9A | 0xA1 | 0xE6 | 0xEA => (after(3), vec![], &[0]),
        0x99 | 0x9B | 0xA0 | 0xA2 | 0xE7 => (after(5), vec![], &[0]),
        0x9C => (after(7), vec![], &[0]),
        0xA4 | 0xE8 => (after(9), vec![], &[0]),
        // The player tests jump to one of their targets.
        0xD3 | 0xD4 => (Next::Never, vec![2, 4, 6], &[]),
        0xD7 => (Next::Never, vec![0, 2], &[]),
        // The condition chains: a branch target after it, or none.
        0x09 => {
            let chain = actor_script::chained_condition_length(image, operands)?;
            (after(chain + 2), vec![chain], &[])
        }
        0x47 => (
            after(actor_script::chained_condition_length(image, operands)?),
            vec![],
            &[],
        ),
        // The switch: a word per value from the lowest to the highest. A
        // table may end early, where its first target's code starts (one
        // value is never used): its words end there.
        0x22 => {
            let count = usize::from(byte(1)?.saturating_sub(byte(0)?)) + 1;
            let mut end = operands + 2 + 2 * count;
            let mut targets = Vec::new();
            let mut at = operands + 2;
            while at < end {
                if let Some(target) = in_bank(bank, word(at - operands)?) {
                    if (at..end).contains(&target) {
                        end = target;
                    }
                    targets.push(target);
                }
                at += 2;
            }
            return Some(Shape {
                next: Next::At(end),
                targets,
                scripts: Vec::new(),
            });
        }
        // A walk leg; at its line it skips the leg's `COP 8E; COP 3B; BRA`
        // loop, and a `COP 84` more with the pose's bit 7.
        0x39 | 0x3A => {
            let skip = operands + 9 + if byte(0)? & 0x80 == 0 { 0 } else { 4 };
            return Some(Shape {
                next: after(3),
                targets: vec![skip],
                scripts: Vec::new(),
            });
        }
        // A counter: bit 6 subtracts and does not skip its word.
        0x4B => (after(if byte(0)? & 0x40 == 0 { 3 } else { 1 }), vec![], &[]),
        // A movement base, a long one after `FF`.
        0xB0 => (after(if byte(0)? == 0xFF { 4 } else { 1 }), vec![], &[]),
        // A choice jumps through its table: cancel, option 1, option 2.
        0x1A => {
            let table = in_bank(bank, word(1)?)?;
            let targets = (0..3)
                .filter_map(|n| {
                    let bytes = image.get(table + 2 * n..table + 2 * n + 2)?;
                    in_bank(table & 0xFF_0000, u16::from_le_bytes([bytes[0], bytes[1]]))
                })
                .collect();
            return Some(Shape {
                next: Next::Never,
                targets,
                scripts: Vec::new(),
            });
        }
        _ => {
            let &(_, length, words) = FIXED.iter().find(|(fixed, ..)| *fixed == service)?;
            (after(length), words.to_vec(), &[])
        }
    };
    let targets = words
        .into_iter()
        .filter_map(|at| in_bank(bank, word(at)?))
        .collect();
    let scripts = longs
        .iter()
        .filter_map(|&at| long(image.get(operands + at..operands + at + 3)?))
        .collect();
    Some(Shape {
        next,
        targets,
        scripts,
    })
}

/// `target`, or a [`Stop::Target`] at `at` when it is not in the ROM.
fn reached(target: Option<usize>, at: usize, found: &mut Walk) -> Option<usize> {
    if target.is_none() {
        found.stops.insert(at, Stop::Target);
    }
    target
}

/// The normalized offset of a long ROM address.
fn long(bytes: &[u8]) -> Option<usize> {
    assets::maps::actors::rom_offset(bytes)
}

/// A bank-relative ROM address in `bank`; `None` below `$8000`.
fn in_bank(bank: usize, address: u16) -> Option<usize> {
    (address >= 0x8000).then_some(bank | usize::from(address))
}

/// The target of a relative branch whose next instruction is at `next`,
/// within its bank; `None` below `$8000`.
fn relative(displacement: i16, next: usize) -> Option<usize> {
    let bank = next & 0xFF_0000;
    let address = u16::try_from(next & 0xFFFF)
        .ok()?
        .wrapping_add_signed(displacement);
    in_bank(bank, address)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An image with handlers at `$00:9000` up for the services given, each
    /// going on after `n` operand bytes, and `script` at `$01:8000`.
    fn image(services: &[(u8, usize)], script: &[u8]) -> Vec<u8> {
        let mut image = vec![0xEA; 0x2_0000];
        for (index, &(service, length)) in services.iter().enumerate() {
            let at = 0x9000 + index * 0x20;
            image[0x83B2 + usize::from(service) * 2..][..2]
                .copy_from_slice(&u16::try_from(at).unwrap().to_le_bytes());
            let mut code = [0xE6, 0x36].repeat(length);
            code.extend([0xA5, 0x36, 0x83, 0x02, 0x40]);
            image[at..at + code.len()].copy_from_slice(&code);
        }
        image[0x1_8000..0x1_8000 + script.len()].copy_from_slice(script);
        image
    }

    #[test]
    fn every_service_the_runtime_models_has_its_layout() {
        // Enough of an image for the variable layouts to read: a chain, a
        // switch and a choice table at `$01:8010`.
        let mut image = vec![0; 0x2_0000];
        image[0x1_8000..0x1_8003].copy_from_slice(&[0, 0x10, 0x80]);
        for service in 0..=0xFF {
            if crate::actors::handling(&image, service) == crate::actors::Handling::Modelled {
                assert!(
                    modelled_shape(&image, service, 0x1_8000).is_some(),
                    "COP {service:02X}"
                );
            }
        }
    }

    #[test]
    fn a_walk_leg_also_goes_on_past_its_loop() {
        let mut image = vec![0; 0x2_0000];
        image[0x1_8000] = 0x80;
        let shape = modelled_shape(&image, 0x39, 0x1_8000).unwrap();
        assert_eq!(
            (shape.next, shape.targets),
            (Next::At(0x1_8003), vec![0x1_800D])
        );
    }

    #[test]
    fn a_spawned_script_is_walked() {
        // COP A1 $81:8010, RTL; at $81:8010 COP 7B, RTL.
        let mut image = image(&[(0x7B, 0)], &[2, 0xA1, 0x10, 0x80, 0x81, 0x6B]);
        image[0x1_8010..0x1_8013].copy_from_slice(&[2, 0x7B, 0x6B]);
        let walk = walk(&image, &[0x1_8000]);
        assert_eq!(walk.sites[&0x7B], BTreeSet::from([0x1_8010]));
    }

    #[test]
    fn a_branch_below_the_rom_is_a_stop() {
        // BRA back over $8000.
        let image = image(&[], &[0x80, 0xF0]);
        let walk = walk(&image, &[0x1_8000]);
        assert_eq!(walk.stops, BTreeMap::from([(0x1_8000, Stop::Target)]));
    }

    /// The runtime's fixed layouts agree with the native handlers, but for
    /// the services whose handlers the derivation misreads (`COP 46`: 3
    /// bytes, where every site has 7) or reads more paths of than the
    /// runtime models.
    #[test]
    fn the_fixed_layouts_agree_with_the_native_handlers() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let rom = rom::Rom::load(&bytes).unwrap();
        let differ = [0x46, 0xCD, 0xD9, 0xDD, 0xDE, 0xEE];
        for &(service, length, words) in FIXED {
            let Some(native) = actor_script::resumes(rom.image(), service) else {
                assert!(differ.contains(&service), "COP {service:02X} unread");
                continue;
            };
            let agrees = native.lengths.contains(&length)
                && words.iter().all(|word| native.targets.contains(word));
            assert!(
                agrees || differ.contains(&service),
                "COP {service:02X}: {native:?}"
            );
        }
    }

    #[test]
    fn the_walk_steps_over_each_service_by_its_length() {
        // COP 7B with two operand bytes, COP 7C with none, then RTL.
        let image = image(&[(0x7B, 2), (0x7C, 0)], &[2, 0x7B, 2, 0x7C, 2, 0x7C, 0x6B]);
        let walk = walk(&image, &[0x1_8000]);
        assert_eq!(walk.sites[&0x7B], BTreeSet::from([0x1_8000]));
        assert_eq!(walk.sites[&0x7C], BTreeSet::from([0x1_8004]));
        assert!(walk.stops.is_empty());
    }
}
