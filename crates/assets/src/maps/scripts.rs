//! Bounded Japanese map-loading script projection, not a gameplay event VM.
//!
//! Resource operands and executed instruction bytes are preserved. Calls, jumps,
//! deferred streams and returns are followed. The one state-dependent branch,
//! `$08 FD`, is evaluated against caller-supplied event flags.
//! Audio/display operations are recorded but not executed. See `docs/map-scripts.md`.
use rom::{AddressError, RuntimeRomAddress};
use std::fmt;

/// Number of entries before the adjacent Japanese subscript table.
pub const MAP_COUNT: u16 = 0x450;
/// Number of entries before the Japanese code entry at `$86:A505`.
pub const SUBSCRIPT_COUNT: u16 = 0xD3;
const MAP_TABLE: usize = 0x06_959C;
const SUBSCRIPT_TABLE: usize = 0x06_A28C;

/// The event-flag state a loading script's branches are evaluated against.
///
/// Mirrors `$80:BBA6`: flag `n` is bit `n & 7` of byte `(n & $0FFF) >> 3`,
/// counting from `$7E:06C0`.
#[derive(Debug, Clone, Copy)]
pub enum EventFlags<'a> {
    /// Every flag reads clear.
    ///
    /// This is **not** the measured new-game state, which sets flags 32 and
    /// 251; see `docs/new-game-bootstrap.md`. No branch in the Japanese map
    /// table references either, so the two agree today, but a caller that
    /// wants the real thing should supply it as a [`Self::Bitmap`].
    AllClear,
    /// A caller-owned copy of the bitmap at `$7E:06C0`.
    ///
    /// A branch on a flag past the end of this slice is refused rather than
    /// answered, because silently reading an uncovered flag as clear would
    /// pick a path on no evidence.
    Bitmap(&'a [u8]),
}
impl EventFlags<'_> {
    /// Whether one flag is set, or `None` when a bitmap does not cover it.
    #[must_use]
    pub fn get(self, flag: u16) -> Option<bool> {
        let index = usize::from(flag & 0x0FFF);
        match self {
            Self::AllClear => Some(false),
            Self::Bitmap(bitmap) => bitmap.get(index / 8).map(|b| b >> (index % 8) & 1 != 0),
        }
    }
}

/// Work and stack limits; allocation is bounded by these values.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Maximum executed instructions, in `1..=65536`.
    pub instructions: usize,
    /// Maximum active calls, at most 64 (zero prohibits calls).
    pub call_depth: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            instructions: 4096,
            call_depth: 32,
        }
    }
}

/// Invalid input or behavior outside the supported loading-script projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptError {
    /// An invalid ROM-backed CPU address.
    Address(AddressError),
    /// A map or subscript index exceeds its bounded table.
    TableIndex {
        /// Whether the subscript table was requested.
        subscript: bool,
        /// Rejected index.
        index: u16,
    },
    /// Missing bytes at a normalized offset.
    Truncated {
        /// Offset of the requested slice.
        offset: usize,
        /// Required bytes.
        needed: usize,
    },
    /// A stream would cross a 64 KiB bank; this is deliberately unsupported.
    BankCrossing,
    /// Unknown opcode or state-dependent/unsupported control variant.
    Unsupported {
        /// Runtime instruction address.
        address: RuntimeRomAddress,
        /// Opcode, or the control subopcode for opcode `$08`.
        opcode: u8,
    },
    /// F9 bit 14 changes table-byte indexing; that variant is not qualified.
    CallFlags {
        /// Original call operand.
        operand: u16,
    },
    /// Caller limits lie outside the allowed bounded range.
    InvalidLimits,
    /// The instruction budget was exhausted before termination.
    InstructionLimit,
    /// A call would exceed the stack budget.
    CallDepth,
    /// A branch read a flag the supplied bitmap does not cover.
    EventFlagUncovered {
        /// Flag index the script asked for.
        flag: u16,
    },
}
impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "map loading script: {self:?}")
    }
}
impl std::error::Error for ScriptError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Address(error) = self {
            Some(error)
        } else {
            None
        }
    }
}
impl From<AddressError> for ScriptError {
    fn from(error: AddressError) -> Self {
        Self::Address(error)
    }
}

/// A resource command family. Graphics and sound behavior are not emulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    /// Opcode `$80`: graphics transfer parameters.
    Graphics,
    /// Opcode `$40`: palette transfer parameters.
    Palette,
    /// Opcode `$20`: metatile or attribute transfer parameters.
    Metatiles,
    /// Opcode `$10`: dimension-prefixed layer or alternate layer mode.
    Layer,
    /// Opcode `$04`: background tilemap transfer parameters.
    Background,
    /// Opcode `$02`: audio selection/resource parameters.
    Audio,
    /// Opcode `$01`: sprite resource parameters.
    Sprite,
}

/// Decoded loading command; exact parameters remain in [`Instruction::bytes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `$00`: return from a call, or follow a pending stream, or finish.
    End,
    /// A typed resource reference with a decoded CPU pointer.
    Resource {
        /// Resource command family.
        kind: ResourceKind,
        /// Resolved pointer, not an assertion that its contents have been decoded.
        source: RuntimeRomAddress,
    },
    /// `$08 F9`: call; bit 15 enables conditional unwind at FF/FE.
    Call {
        /// Subscript index (bit 14 variants rejected).
        index: u16,
        /// Whether FF/FE returns instead of jumping/continuing.
        flagged: bool,
    },
    /// `$08 FF`: jump unless the current call is flagged, in which case return.
    Jump(u16),
    /// `$08 F8`: return if inside a call; otherwise continue.
    Return,
    /// `$08 FA`: replace the pending subscript, followed at root END (zero clears).
    Defer(u16),
    /// `$08 FD`: jump through the subscript table when an event flag matches.
    ///
    /// Six bytes: `08 FD <condition> <target>`. `$86:907D` masks the condition
    /// with `$7FFF`, tests it through `$80:BBC7`, and takes the jump on carry
    /// when condition bit 15 is set or on no-carry when it is clear.
    BranchOnEventFlag {
        /// Flag index, `condition & $0FFF`, addressing `$7E:06C0` bit-wise.
        flag: u16,
        /// Whether the jump is taken when the flag is set.
        jump_if_set: bool,
        /// Subscript index jumped to when taken.
        target: u16,
    },
    /// `$08 FE`: unwind a flagged call, otherwise skip the opaque operand word.
    EndIfFlagged,
    /// `$08 FC`: select audio through the global audio list; not executed here.
    AudioSelection,
    /// `$08 00`: display configuration; not executed here.
    Display,
}

/// One executed instruction, retaining its runtime address and original bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    /// Actual stream address, preserving its runtime ROM mirror.
    pub address: RuntimeRomAddress,
    /// Exact opcode and operand bytes, including ignored fields/bits.
    pub bytes: Vec<u8>,
    /// Typed interpretation for the supported subset.
    pub command: Command,
}

/// Resource-loading path for one map ID, not a complete gameplay map state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapProgram {
    /// Caller-selected Japanese map ID.
    pub map_id: u16,
    /// Initial script pointer from the map table.
    pub entry: RuntimeRomAddress,
    /// Executed path, including returns and repeated instructions.
    pub instructions: Vec<Instruction>,
}

/// Reproduces `$86:90E7`'s packed-pointer arithmetic with binary A8/A16 and X/Y16.
///
/// Bits 0..14 are the offset, bits 15..22 are an eight-bit bank increment, and
/// bit 23 is ignored. Addition wraps in eight bits. Address bit 15 is set only
/// when both the original stream bank and the wrapped result are below `$C0`.
/// The source stream's *base bank*, not the instruction offset, is used.
///
/// # Errors
/// Rejects results outside ROM-backed `HiROM` windows; WRAM/I/O pointers are not
/// supported by this static reader even if the original CPU could access them.
pub fn unpack_pointer(bytes: [u8; 3], base_bank: u8) -> Result<RuntimeRomAddress, ScriptError> {
    let packed = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]);
    let bank = base_bank.wrapping_add(((packed >> 15) & 0xFF) as u8);
    let mut offset = u16::from_le_bytes([bytes[0], bytes[1]]) & 0x7FFF;
    if base_bank < 0xC0 && bank < 0xC0 {
        offset |= 0x8000;
    }
    Ok(RuntimeRomAddress::from_parts(bank, offset)?)
}

fn slice(image: &[u8], offset: usize, length: usize) -> Result<&[u8], ScriptError> {
    image
        .get(offset..offset + length)
        .ok_or(ScriptError::Truncated {
            offset,
            needed: length,
        })
}
fn entry(image: &[u8], index: u16, subscript: bool) -> Result<RuntimeRomAddress, ScriptError> {
    let (table, count) = if subscript {
        (SUBSCRIPT_TABLE, SUBSCRIPT_COUNT)
    } else {
        (MAP_TABLE, MAP_COUNT)
    };
    if index >= count {
        return Err(ScriptError::TableIndex { subscript, index });
    }
    let bytes = slice(image, table + usize::from(index) * 3, 3)?;
    Ok(RuntimeRomAddress::new(u32::from_le_bytes([
        bytes[0], bytes[1], bytes[2], 0,
    ]))?)
}

#[derive(Clone, Copy)]
struct Cursor {
    base: RuntimeRomAddress,
    offset: u32,
    flagged: bool,
}
impl Cursor {
    fn new(base: RuntimeRomAddress) -> Self {
        Self {
            base,
            offset: 0,
            flagged: false,
        }
    }
    fn read(&mut self, image: &[u8]) -> Result<Instruction, ScriptError> {
        let within = u32::from(self.base.offset()) + self.offset;
        if within >= 0x10000 {
            return Err(ScriptError::BankCrossing);
        }
        let address = RuntimeRomAddress::new(self.base.value() + self.offset)?;
        let start = address.normalized().value() as usize;
        let get = |length: u8| {
            if within + u32::from(length) > 0x10000 {
                return Err(ScriptError::BankCrossing);
            }
            slice(image, start, usize::from(length))
        };
        let opcode = get(1)?[0];
        let unsupported = |opcode| ScriptError::Unsupported { address, opcode };
        let (length, command) = if opcode == 0 {
            (1, Command::End)
        } else if opcode == 8 {
            let sub = get(2)?[1];
            // $FD carries a second operand word; $F8 carries none.
            let length = match sub {
                0xF8 => 2,
                0xFD => 6,
                _ => 4,
            };
            if !matches!(sub, 0 | 0xF8..=0xFE | 0xFF) {
                return Err(unsupported(sub));
            }
            let bytes = get(length)?;
            // $F8 is the only two-byte control; every longer one carries an
            // operand word here, and $FD carries a second one after it.
            let word = if length >= 4 {
                u16::from_le_bytes([bytes[2], bytes[3]])
            } else {
                0
            };
            let command = match sub {
                0 => Command::Display,
                0xF8 => Command::Return,
                0xF9 => {
                    if word & 0x4000 != 0 {
                        return Err(ScriptError::CallFlags { operand: word });
                    }
                    Command::Call {
                        index: word & 0x7FFF,
                        flagged: word & 0x8000 != 0,
                    }
                }
                0xFA => Command::Defer(word),
                0xFD => Command::BranchOnEventFlag {
                    flag: word & 0x0FFF,
                    jump_if_set: word & 0x8000 != 0,
                    target: u16::from_le_bytes([bytes[4], bytes[5]]),
                },
                0xFC => Command::AudioSelection,
                0xFE => Command::EndIfFlagged,
                0xFF => Command::Jump(word),
                _ => return Err(unsupported(sub)),
            };
            (length, command)
        } else {
            let (length, pointer, kind) = match opcode {
                0x80 => (9, 4, ResourceKind::Graphics),
                0x40 => (7, 4, ResourceKind::Palette),
                0x20 => (8, 5, ResourceKind::Metatiles),
                0x10 => (5, 2, ResourceKind::Layer),
                4 => (
                    if get(5)?[4] == 0 { 5 } else { 6 },
                    1,
                    ResourceKind::Background,
                ),
                2 => (6, 3, ResourceKind::Audio),
                1 => (7, 4, ResourceKind::Sprite),
                _ => return Err(unsupported(opcode)),
            };
            let bytes = get(length)?;
            let source = unpack_pointer(
                [bytes[pointer], bytes[pointer + 1], bytes[pointer + 2]],
                self.base.bank(),
            )?;
            slice(image, source.normalized().value() as usize, 1)?;
            (length, Command::Resource { kind, source })
        };
        let bytes = get(length)?.to_vec();
        self.offset += u32::from(length);
        Ok(Instruction {
            address,
            bytes,
            command,
        })
    }
}

/// Follows the supported Japanese map-loading path with explicit work budgets.
///
/// The caller authenticates the ROM revision. This projects resource references,
/// not sound/display effects, cached transfers or fully composed layers. It does
/// not scan the global audio list for FC. Conditional FD commands fail rather
/// than reading unspecified game flags. There is no arbitrary event execution.
///
/// # Errors
/// Rejects bad table indices/pointers, truncated or bank-crossing instructions,
/// unqualified commands, uncovered event flags and exhausted budgets.
pub fn resolve_map(image: &[u8], map_id: u16, limits: Limits) -> Result<MapProgram, ScriptError> {
    resolve_map_with_events(image, map_id, limits, EventFlags::AllClear)
}

/// Resolves one map's loading path against an explicit event-flag bitmap.
///
/// Flag state only affects scripts containing `$08 FD`; every other script
/// resolves identically for any bitmap. [`resolve_map`] is this function with
/// [`EventFlags::AllClear`].
///
/// # Errors
///
/// As [`resolve_map`].
pub fn resolve_map_with_events(
    image: &[u8],
    map_id: u16,
    limits: Limits,
    events: EventFlags<'_>,
) -> Result<MapProgram, ScriptError> {
    if limits.instructions == 0 || limits.instructions > 65536 || limits.call_depth > 64 {
        return Err(ScriptError::InvalidLimits);
    }
    let initial = entry(image, map_id, false)?;
    let mut cursor = Cursor::new(initial);
    let mut stack = Vec::new();
    let mut pending = 0;
    let mut instructions = Vec::new();
    for _ in 0..limits.instructions {
        let instruction = cursor.read(image)?;
        match instruction.command {
            Command::End => {
                if let Some(parent) = stack.pop() {
                    cursor = parent;
                } else if pending != 0 {
                    cursor = Cursor::new(entry(image, pending, true)?);
                    pending = 0;
                } else {
                    instructions.push(instruction);
                    return Ok(MapProgram {
                        map_id,
                        entry: initial,
                        instructions,
                    });
                }
            }
            Command::Call { index, flagged } => {
                if stack.len() >= limits.call_depth {
                    return Err(ScriptError::CallDepth);
                }
                stack.push(cursor);
                cursor = Cursor::new(entry(image, index, true)?);
                cursor.flagged = flagged;
            }
            Command::Return | Command::EndIfFlagged | Command::Jump(_) if cursor.flagged => {
                if let Some(parent) = stack.pop() {
                    cursor = parent;
                }
            }
            Command::Return => {
                if let Some(parent) = stack.pop() {
                    cursor = parent;
                }
            }
            Command::Jump(index) => {
                cursor.base = entry(image, index, true)?;
                cursor.offset = 0;
            }
            Command::Defer(index) => pending = index,
            Command::BranchOnEventFlag {
                flag,
                jump_if_set,
                target,
            } => {
                let set = events
                    .get(flag)
                    .ok_or(ScriptError::EventFlagUncovered { flag })?;
                // Not taken simply continues: the instruction length already
                // skipped both operand words, exactly as $86:9097 does.
                if set == jump_if_set {
                    cursor.base = entry(image, target, true)?;
                    cursor.offset = 0;
                }
            }
            _ => {}
        }
        instructions.push(instruction);
    }
    Err(ScriptError::InstructionLimit)
}
