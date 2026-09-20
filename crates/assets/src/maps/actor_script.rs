//! Bounded actor script walking, not an actor runtime.
//!
//! Actor scripts are native 65C816 code whose commands are `COP` signatures
//! dispatched through the table at `$00:83B2`. Each service's handler advances
//! the stream pointer `$36` by its own operand length, so lengths are read from
//! the handlers rather than assumed.
//!
//! This walks a script and reports the two effects the Crysta slice needs —
//! dialogue requests and event-flag writes — and refuses a service whose
//! advance it cannot account for. It does not execute anything: conditions are
//! not evaluated and branches are not followed, so the result is every effect
//! the straight-line stream contains.
use std::fmt;

const COP_TABLE: usize = 0x00_83B2;
/// Hard decoding budget for one script, in commands.
pub const MAX_COMMANDS: usize = 512;

/// Requests dialogue at a bank-relative address; `$80:8BEB` stores it to `$0DC2`.
pub const SHOW_TEXT: u8 = 0x1B;
/// Writes an event flag through `$80:BB77`; see `$80:8669`.
pub const WRITE_FLAG: u8 = 0x07;
/// Registers an interaction callback address.
pub const REGISTER_CALLBACK: u8 = 0x21;
/// Branches on an event flag through `$80:BBA6`; see `$80:8678`.
pub const BRANCH_ON_FLAG: u8 = 0x08;
/// Services that test a chain of event-flag conditions.
///
/// `$80:8695` and `$80:963A` share an opening with [`BRANCH_ON_FLAG`] but,
/// after testing a word, examine its high nibble: a bit in `$F000` selects a
/// boolean combinator and pulls in a further condition word, while a clear
/// nibble ends the chain. Their length is therefore a property of the stream,
/// not of the handler, exactly as the spawn stream's `$FA` is -- there the mask
/// is `$F800`.
pub const CHAINED_CONDITION: [u8; 2] = [0x09, 0x47];
/// Mask whose bits continue a chained condition.
const CHAIN_CONTINUES: u16 = 0xF000;

/// Operand bytes a chained condition consumes, starting at its first word.
///
/// At least one word, then one more for as long as the previous word has a bit
/// in [`CHAIN_CONTINUES`].
#[must_use]
pub fn chained_condition_length(image: &[u8], first_word: usize) -> Option<usize> {
    let mut length = 0usize;
    for _ in 0..MAX_CHAIN_WORDS {
        let bytes = image.get(first_word + length..first_word + length + 2)?;
        length += 2;
        if u16::from_le_bytes([bytes[0], bytes[1]]) & CHAIN_CONTINUES == 0 {
            return Some(length);
        }
    }
    None
}

/// Condition words one chain may hold before it is treated as desynchronised.
const MAX_CHAIN_WORDS: usize = 16;

/// Invalid input or a script outside the accounted subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptError {
    /// The address is not inside a ROM-backed bank.
    Address {
        /// Rejected address.
        source: u32,
    },
    /// Missing bytes at a normalized offset.
    Truncated {
        /// Offset of the requested slice.
        offset: usize,
    },
}
impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "actor script: {self:?}")
    }
}
impl std::error::Error for ScriptError {}

/// One decoded command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command {
    /// `COP` signature byte.
    pub service: u8,
    /// Normalized offset of the `COP` opcode.
    pub offset: usize,
    /// First operand word, when the service takes at least two operand bytes.
    pub operand: Option<u16>,
}

/// Why a straight-line walk stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// A byte that is not a `COP`, so the command stream ended here.
    EndOfCommands {
        /// Normalized offset.
        offset: usize,
        /// The byte found.
        opcode: u8,
    },
    /// A service whose operand length is not accounted for.
    ///
    /// The effects collected before it are still returned, but everything
    /// after is unknown: guessing the length would resynchronise the walk onto
    /// operand bytes, which decode into plausible-looking commands.
    Unaccounted {
        /// Normalized offset.
        offset: usize,
        /// The `COP` signature.
        service: u8,
    },
    /// The command budget was exhausted.
    Budget,
    /// A flag branch named a target that is not a command address.
    ///
    /// Bank-relative targets below `$8000` are RAM, not ROM, so a script
    /// cannot legitimately branch to one. Reported rather than walked, because
    /// in a normalized image such an offset still decodes into
    /// plausible-looking commands.
    InvalidBranch {
        /// Normalized offset of the branch.
        offset: usize,
        /// The bank-relative target named.
        target: u16,
    },
}

/// Effects a straight-line walk of one script contains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptEffects {
    /// Why the walk stopped. Effects before that point are complete.
    pub stop: Stop,
    /// Bank-relative dialogue addresses requested by [`SHOW_TEXT`].
    pub text: Vec<u16>,
    /// Event-flag operands written by [`WRITE_FLAG`], retaining bit 15.
    pub flags: Vec<u16>,
    /// Callback addresses registered by [`REGISTER_CALLBACK`].
    pub callbacks: Vec<u16>,
    /// Every command walked, in order.
    pub commands: Vec<Command>,
}

/// Operand bytes a service consumes, or `None` when unaccounted.
///
/// Derived by exploring the service's handler, **instruction-aligned** with
/// [`crate::cpu`]. Byte scanning does not work: `STA $40` contains `$40`
/// (`RTI`) and `AND #$0001` contains `$01`, so a scan desynchronises inside
/// operand data. Nor does a walk to the first branch, because handlers open
/// with guards -- `$80:8BEB` tests three busy flags before fetching anything.
///
/// Every path is followed, and each terminal is classified by how it leaves the
/// stream pointer `$36`:
///
/// - `LDA $36 (+delta) : STA $02,S : RTI` **proceeds**: the handler overwrites
///   the dispatcher's resume address with `$36`, so the operand length is the
///   run of `INC $36` plus `delta`.
/// - `PLA : PLA : RTL` **stalls**: `$80:8BEB` reaches it when text is still on
///   screen, after rewinding `$36` by two so the `COP` re-runs next frame. It
///   is not a length.
/// - `LDA [$36] : STA $02,S` **branches**: the resume address comes from the
///   stream, as in `$80:8678`'s taken arm. Also not a length.
///
/// A service whose proceeding paths disagree is refused. `$47` is the case:
/// `$80:963A` chains a further condition word whenever the previous one has a
/// bit in `$F000`, the same shape as the spawn stream's `$FA`, so its length is
/// a property of the stream rather than of the handler.
#[must_use]
pub fn operand_length(image: &[u8], service: u8) -> Option<usize> {
    let entry = handler(image, service)?;
    let mut lengths = Vec::new();
    let mut seen = Vec::new();
    let mut queue = vec![Path {
        at: entry,
        advance: 0,
        widths: crate::cpu::Widths::native(),
    }];
    for _ in 0..MAX_PATHS {
        let Some(path) = queue.pop() else { break };
        let key = (path.at, path.advance, path.widths);
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        explore(image, path, &mut queue, &mut lengths)?;
    }
    // Paths still queued means the budget ran out, not that the handler was
    // understood; a length derived from a partial exploration would be a guess.
    if queue.is_empty() && !lengths.is_empty() && lengths.iter().all(|&n| n == lengths[0]) {
        return usize::try_from(lengths[0]).ok();
    }
    None
}

/// Paths one handler's exploration may visit.
const MAX_PATHS: usize = 512;
/// Instructions one path may walk.
const MAX_PATH_STEPS: usize = 512;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Path {
    at: usize,
    advance: isize,
    /// Register widths on entry. A branch target reached after `SEP #$20` must
    /// be decoded narrow, or its immediates desynchronise the walk.
    widths: crate::cpu::Widths,
}

/// What the accumulator holds, as far as the length derivation cares.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Value {
    /// `$36` plus an offset: a resume address the handler computed.
    Pointer(isize),
    /// A word read through `$36`: a branch target, not a length.
    Stream,
    /// Anything else.
    Other,
}

/// Opcodes that leave the accumulator alone.
///
/// The model is deliberately inverted: anything not listed here clobbers the
/// accumulator. Forgetting an opcode that writes `A` would leave a stale
/// [`Value::Pointer`] live and yield a *wrong length*, while forgetting one
/// that preserves `A` merely costs a refusal. Only the second is safe.
const PRESERVES_ACCUMULATOR: [u8; 98] = [
    // STA, STX, STY, STZ.
    0x81, 0x83, 0x85, 0x87, 0x8D, 0x8F, 0x91, 0x92, 0x93, 0x95, 0x97, 0x99, 0x9D, 0x9F, 0x86, 0x8E,
    0x96, 0x84, 0x8C, 0x94, 0x64, 0x74, 0x9C, 0x9E, // INC and DEC on memory.
    0xE6, 0xF6, 0xEE, 0xFE, 0xC6, 0xD6, 0xCE, 0xDE, // CMP, CPX, CPY.
    0xC9, 0xC5, 0xCD, 0xD5, 0xDD, 0xD9, 0xC1, 0xD1, 0xD2, 0xC7, 0xD7, 0xC3, 0xD3, 0xCF, 0xDF, 0xE0,
    0xE4, 0xEC, 0xC0, 0xC4, 0xCC, // BIT, TSB, TRB.
    0x89, 0x24, 0x2C, 0x34, 0x3C, 0x04, 0x0C, 0x14, 0x1C, // Pushes.
    0x48, 0xDA, 0x5A, 0x8B, 0x08, 0x0B, 0x4B, 0xD4, 0xF4, 0x62,
    // Pulls and transfers that do not target the accumulator.
    0xFA, 0x7A, 0xAB, 0x2B, 0xAA, 0xA8, 0xBA, 0x9A, 0xBB, 0x9B, 0x5B, 0x1B, 0xCA, 0x88, 0xE8,
    0xC8, // Flag operations, width changes and NOP.
    0x18, 0x38, 0x58, 0x78, 0xD8, 0xF8, 0xB8, 0xE2, 0xC2, 0xEA,
];

/// Follows one path to its terminal, pushing forks onto `queue` and recording
/// the operand length of any terminal that proceeds.
///
/// Returns `None` when the path reaches something the model cannot account
/// for, which refuses the whole derivation rather than yielding a partial one.
fn explore(
    image: &[u8],
    path: Path,
    queue: &mut Vec<Path>,
    lengths: &mut Vec<isize>,
) -> Option<()> {
    let mut widths = path.widths;
    let mut cursor = path.at;
    let mut advance = path.advance;
    let mut value = Value::Other;
    let mut committed = None;
    // Handlers loop: `$80:8C4A` spins on the text renderer until it reports
    // done. A back edge is not a terminal, and the loop's exits are already on
    // the queue, so revisiting an offset simply ends this path.
    let mut visited = Vec::new();
    for _ in 0..MAX_PATH_STEPS {
        if visited.contains(&cursor) {
            return Some(());
        }
        visited.push(cursor);
        let opcode = *image.get(cursor)?;
        let operand = image.get(cursor + 1).copied();
        let next = crate::cpu::step(image, cursor, &mut widths)?;
        match opcode {
            // INC $36 / DEC $36 move the stream pointer itself.
            0xE6 if operand == Some(0x36) => advance += 1,
            0xC6 if operand == Some(0x36) => advance -= 1,
            0xA5 if operand == Some(0x36) => value = Value::Pointer(0),
            0xA7 if operand == Some(0x36) => value = Value::Stream,
            0x1A => value = shift(value, 1),
            0x3A => value = shift(value, -1),
            // ADC #imm. Every one of these that acts on a live pointer in this
            // ROM is preceded by `CLC` on the same path.
            0x69 => {
                let bytes = image.get(cursor + 1..next)?;
                let immediate = if bytes.len() == 2 {
                    isize::from(i16::from_le_bytes([bytes[0], bytes[1]]))
                } else {
                    isize::from(i8::from_ne_bytes([bytes[0]]))
                };
                value = shift(value, immediate);
            }
            // STA $02,S overwrites the dispatcher's resume address.
            0x83 if operand == Some(0x02) => committed = Some(value),
            // STA $000A,X stores the resume address into the actor slot's own
            // script-pointer field -- the actor yields and continues there next
            // frame. `$80:AB17` ends this way. It is the same commit, read back
            // out of WRAM at slot offset ten.
            0x9D if image.get(cursor + 1..cursor + 3) == Some(&[0x0A, 0x00]) => {
                committed = Some(value);
            }
            // Conditional branches fork; both arms are explored, each carrying
            // the widths in force where it was taken.
            0x10 | 0x30 | 0x50 | 0x70 | 0x90 | 0xB0 | 0xD0 | 0xF0 => {
                queue.push(Path {
                    at: relative(image.get(cursor + 1).copied()?, next)?,
                    advance,
                    widths,
                });
            }
            // Unconditional transfers continue the same path.
            0x80 => {
                cursor = relative(image.get(cursor + 1).copied()?, next)?;
                continue;
            }
            // BRL, a sixteen-bit BRA.
            0x82 => {
                let bytes = image.get(cursor + 1..cursor + 3)?;
                let displacement = i16::from_le_bytes([bytes[0], bytes[1]]);
                cursor = next.checked_add_signed(isize::from(displacement))?;
                continue;
            }
            0x4C => {
                let bytes = image.get(cursor + 1..cursor + 3)?;
                cursor =
                    (cursor & 0xFF_0000) | usize::from(u16::from_le_bytes([bytes[0], bytes[1]]));
                continue;
            }
            // Terminals. Only a committed pointer is an operand length; a
            // stall or a stream-sourced branch is not.
            0x40 | 0x6B => {
                if let Some(Value::Pointer(delta)) = committed {
                    // A commit at or before the `COP` is a stall, not a length:
                    // `$80:8BEB` rewinds two bytes so the command re-runs once
                    // the text engine is free.
                    let total = advance + delta;
                    if total >= 0 {
                        lengths.push(total);
                    }
                }
                return Some(());
            }
            // Transfers this model cannot follow, and `PLP`, which leaves the
            // register widths unknown. `RTS` belongs here too: the dispatcher
            // enters handlers with `JMP ($83B2,X)`, never `JSR`, so reaching one
            // means the walk has gone somewhere it should not have.
            0x5C | 0x6C | 0x7C | 0xDC | 0x28 | 0xFB | 0x60 => return None,
            opcode if PRESERVES_ACCUMULATOR.contains(&opcode) => {}
            // Everything else is assumed to clobber the accumulator.
            _ => value = Value::Other,
        }
        cursor = next;
    }
    None
}

fn shift(value: Value, by: isize) -> Value {
    match value {
        Value::Pointer(delta) => Value::Pointer(delta + by),
        other => other,
    }
}

/// Target of an eight-bit relative branch whose next instruction is at `next`.
fn relative(displacement: u8, next: usize) -> Option<usize> {
    next.checked_add_signed(isize::from(i8::from_ne_bytes([displacement])))
}

fn handler(image: &[u8], service: u8) -> Option<usize> {
    let at = COP_TABLE + usize::from(service) * 2;
    let bytes = image.get(at..at + 2)?;
    Some(usize::from(u16::from_le_bytes([bytes[0], bytes[1]])))
}

/// Walks one script, following flag branches against `events`.
///
/// `$80:8678` tests the condition through `$80:BBA6` and, per its own branch
/// structure, takes the branch when the flag is set for a non-negative
/// condition word and when it is clear for a negative one — the same
/// convention the spawn stream uses.
///
/// # Errors
/// As [`walk`].
pub fn walk_with_events(
    image: &[u8],
    source: u32,
    events: super::scripts::EventFlags<'_>,
) -> Result<ScriptEffects, ScriptError> {
    walk_inner(image, source, Some(events))
}

/// Walks one script from a runtime address, collecting its effects.
///
/// # Errors
/// Rejects non-ROM addresses and truncation. A non-`COP` byte, an unaccounted
/// service and budget exhaustion are reported through [`ScriptEffects::stop`],
/// because the effects collected before that point are still complete.
pub fn walk(image: &[u8], source: u32) -> Result<ScriptEffects, ScriptError> {
    walk_inner(image, source, None)
}

fn walk_inner(
    image: &[u8],
    source: u32,
    events: Option<super::scripts::EventFlags<'_>>,
) -> Result<ScriptEffects, ScriptError> {
    if !(0x80..=0xBF).contains(&(source >> 16)) || source & 0xFFFF < 0x8000 {
        return Err(ScriptError::Address { source });
    }
    let start = (source & 0x3F_FFFF) as usize;
    // Branch targets are bank-relative, and the image is normalized.
    let bank = start & 0xFF_0000;
    let mut cursor = start;
    // `operand_length` explores a handler's whole control-flow graph, so the
    // same service must not be derived twice in one walk.
    let mut derived: [Option<Option<usize>>; 256] = [None; 256];
    let mut effects = ScriptEffects {
        stop: Stop::Budget,
        text: Vec::new(),
        flags: Vec::new(),
        callbacks: Vec::new(),
        commands: Vec::new(),
    };
    for _ in 0..MAX_COMMANDS {
        let window = image
            .get(cursor..cursor + 2)
            .ok_or(ScriptError::Truncated { offset: cursor })?;
        if window[0] != 0x02 {
            // The command stream ends where it stops being COP commands.
            effects.stop = Stop::EndOfCommands {
                offset: cursor,
                opcode: window[0],
            };
            return Ok(effects);
        }
        let service = window[1];
        let length = if CHAINED_CONDITION.contains(&service) {
            chained_condition_length(image, cursor + 2)
        } else {
            *derived[usize::from(service)].get_or_insert_with(|| operand_length(image, service))
        };
        let Some(length) = length else {
            effects.stop = Stop::Unaccounted {
                offset: cursor,
                service,
            };
            return Ok(effects);
        };
        let operand = (length >= 2)
            .then(|| {
                image
                    .get(cursor + 2..cursor + 4)
                    .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            })
            .flatten();
        match service {
            SHOW_TEXT => effects.text.extend(operand),
            WRITE_FLAG => effects.flags.extend(operand),
            REGISTER_CALLBACK => effects.callbacks.extend(operand),
            _ => {}
        }
        effects.commands.push(Command {
            service,
            offset: cursor,
            operand,
        });
        // A flag branch is followed only when the caller supplied flags; the
        // straight-line walk falls through it, reporting both arms' effects.
        //
        // Keyed on the service, never on the length. Twenty other services also
        // take four operand bytes -- `$42` carries two coordinate deltas, `$0A`
        // compares a map ID against `$047E` -- and reading their operands as a
        // condition and a target sends the walk somewhere it can still decode,
        // which is the worst possible failure.
        if service == BRANCH_ON_FLAG {
            if let (Some(flags), Some(condition)) = (events, operand) {
                let set = flags
                    .get(condition & 0x0FFF)
                    .ok_or(ScriptError::Truncated { offset: cursor })?;
                if super::actors::condition_takes_branch(condition, set) {
                    let target = image
                        .get(cursor + 4..cursor + 6)
                        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
                        .ok_or(ScriptError::Truncated { offset: cursor })?;
                    if target < 0x8000 {
                        effects.stop = Stop::InvalidBranch {
                            offset: cursor,
                            target,
                        };
                        return Ok(effects);
                    }
                    cursor = bank | usize::from(target);
                    continue;
                }
            }
        }
        cursor += 2 + length;
    }
    Ok(effects)
}
