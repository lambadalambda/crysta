//! Short native runs between script commands that only use script scratch
//! words and the display: the tour's guide and controller take turns through
//! `$04BC` with `INC`, `LDA`/`CMP` and a branch to `RTL` (`$89:D3E6`,
//! `$89:D2EC`); the freezing's whitening narrows the accumulator, writes PPU
//! registers and calls palette routines in a loop (`$88:B507`).
//!
//! A run starts 16-bit, as the scheduler enters scripts, and stops before the
//! first opcode the script loop owns (`COP`, `RTL`, `JMP`, `BRA`) or another
//! recogniser may take, once its stack is balanced, its accumulator wide
//! and X the actor's again. Anything else -- another address, another call,
//! a register or flag the run has not set -- is refused, so the script freezes
//! as before rather than guessing. A scratch word never written reads 0, as
//! the words start cleared. Writes to the PPU's colour math registers go to
//! the [`Display`]; an actor's own bytes in bank `$7F` (`$7F:xxxx,X`) are
//! its [`Own`] bytes, 0 until written.

use crate::display::Display;
use std::collections::BTreeMap;

/// Scratch words by address.
pub type Scratch = BTreeMap<u16, u16>;
/// An actor's own bytes in bank `$7F`, by address less the actor's offset.
pub type Own = BTreeMap<u16, u8>;

/// What a run may change: the scratch words, the actor's own bytes and the
/// display.
pub struct Memory<'m> {
    /// The scratch words.
    pub words: &'m mut Scratch,
    /// The running actor's own bytes.
    pub own: &'m mut Own,
    /// The colour math.
    pub display: &'m mut Display,
    /// `$0408`, the random generator's word.
    pub random: u16,
    /// `$0966`/`$0968`: Ark's probe (x, y - 8), which enemies aim at.
    pub probe: (u16, u16),
    /// The event flags, which `$80:BBC7` tests.
    pub events: &'m mut [u8],
    /// `+$0E`, the frames the scheduler skips after the next yield, as a
    /// random start delay stores it (`STA $00:000E,X`, `$90:939C`).
    pub sleep: &'m mut u16,
    /// The actor's x and y (`$0000,X`, `$0002,X`).
    pub position: &'m mut (u16, u16),
    /// Ark's entity, through `LDY $0DEA`.
    pub player: View,
    /// The parent's, through `LDA $7F:001E,X; TAY`, as it was at the
    /// spawn (`$97:C1E6`).
    pub parent: Option<View>,
    /// The child `COP 99` linked, which Y holds as the run starts.
    pub linked: Option<u16>,
    /// Every actor by id, for an id in a field (`LDY $0026,X`).
    pub views: &'m [(u16, View)],
    /// A left at a `BRA`/`JMP` the script loop follows, and its target,
    /// for a run that starts there (`$90:FCC6`: `INC; BRA` to `STA $0026,X`).
    pub carried: &'m mut Option<(usize, u16)>,
}

/// What a run reads of another entity through Y.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct View {
    /// `+$00`.
    pub x: u16,
    /// `+$02`.
    pub y: u16,
    /// `+$14`, the facing code.
    pub facing: u16,
}

impl View {
    /// The word at `+field`: x, y, the facing, the layer (always 0 here).
    fn field(self, field: u16) -> Option<u16> {
        match field {
            0x00 => Some(self.x),
            0x02 => Some(self.y),
            0x14 => Some(self.facing),
            0x16 => Some(0),
            _ => None,
        }
    }
}

/// The entity Y holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entity {
    Player,
    Parent,
    /// An actor by its id.
    Id(u16),
}

/// The actor's own bytes runs may use: the wake callbacks and the attack
/// kind (`$7F:1000..102D,X`, `docs/enemy-scripts.md` §5), `COP CC`'s target
/// and counters (`$7F:2004..200B,X`), `COP 46`'s row offset and layer
/// (`$7F:201A/201B,X`), the voice fade's intensity (`$7F:201C,X`,
/// `$88:9CD8`) and the callbacks' script bank (`$7F:2020,X`, `$90:93B9`).
/// Other fields are the engine's.
const OWN: [std::ops::RangeInclusive<u16>; 4] = [
    0x1000..=0x102D,
    0x2004..=0x200B,
    0x201A..=0x201D,
    0x2020..=0x2021,
];
/// `$00:000E,X`: the entity's sleep (`+$0E`).
const SLEEP: u32 = 0x00_000E;
/// `$0966`/`$0968`, Ark's probe, which runs may read.
const PROBE: [u16; 2] = [0x0966, 0x0968];
/// `$0408`: the random generator's word, which runs may read.
const RANDOM: u16 = 0x0408;
/// The entity's own words runs may use through `,X`: `+$14`, a bullet's
/// direction, `+$16` the layer (0: the runtime keeps one), and `+$24` and
/// `+$26` (the blob's counters, `docs/enemy-scripts.md`).
const FIELDS: [u16; 4] = [0x14, 0x16, 0x24, 0x26];
/// `$7E:46E6`, `COP 6A`'s square: a run stores 0 to stop it.
const SPIN: u32 = 0x7E_46E6;
/// The colour math registers' shadows (`$0468..$046B`), which the NMI
/// copies only with `$091C` set; runs write the registers too.
const SHADOWS: std::ops::RangeInclusive<u16> = 0x0468..=0x046B;
/// The PPU's registers.
const PPU: std::ops::RangeInclusive<u16> = 0x2100..=0x213F;

/// Words runs may use: scripts' own variables, and one engine word the
/// runtime does not read. With the evidence.
const SCRATCH: [(u16, u16); 7] = [
    // `$89:D2B2` clears `$0440`, `$04BC`, `$04BE`, `$04C0`, `$04C2`.
    (0x0440, 0x0441),
    (0x04BC, 0x04C3),
    // An engine word the runtime does not read: the spear's grant sets and
    // clears its bit 8 (`$89:DA96`, `$89:DA31`); bit 15 places windows.
    (0x048A, 0x048B),
    // The pending map (`$90:8B0F`), which the world takes after the run,
    // and the tower's index (`$04CC`, `$90:8B31`).
    (PENDING_MAP, PENDING_MAP + 1),
    (0x04CC, 0x04CD),
    // The light room's BG3 scroll (`$90:8AC9`, `$90:8AD5`), not drawn.
    (0x0886, 0x0889),
    // An engine word the runtime does not read: the pedestals set its bit 7
    // (`$90:FBE4`).
    (0x045A, 0x045B),
];

/// `$047C`, the map a script asks for next (`$90:8B0C`).
pub const PENDING_MAP: u16 = 0x047C;
/// `$0482`, the map before this one, as the light room reads it.
pub const PREVIOUS_MAP: u16 = 0x0482;
/// `$0042`, the frame counter.
pub const FRAMES: u16 = 0x0042;

/// `$097C`, the player's action word: runs may read it, as a watcher in
/// `$11` does (`$88:A9EF`); the world keeps it in the scratch words.
pub const PLAYER_ACTION: u16 = 0x097C;
/// Engine words runs may read but not write: Ark's position, the map
/// before and the frame counter, the player's action word,
/// the Prime Blue count (`$07ED`, BCD, `$8D:95A8`), which a resident in
/// the Prime Blue shop `$1D` tests (`$88:C7ED`), and the enemy count.
const READABLE: [u16; 9] = [
    ARK_FLAGS,
    WINDOW_BUSY,
    PLAYER_ACTION,
    PRIME_BLUE,
    ENEMIES,
    PREVIOUS_MAP,
    FRAMES,
    PLAYER_X,
    PLAYER_Y,
];
/// `$0978`, a copy of Ark's `+$04`: bit 7 out of play (`$90:FC9B`).
pub const ARK_FLAGS: u16 = 0x0978;
/// `$0DC2`, nonzero while the text window is busy (`$97:BF59`).
pub const WINDOW_BUSY: u16 = 0x0DC2;
/// `$0952`, Ark's x less 8.
pub const PLAYER_X: u16 = 0x0952;
/// `$0954`, Ark's y less 16, as the towers' gates read it (`$90:8F8F`).
pub const PLAYER_Y: u16 = 0x0954;
/// `$0498`, the enemies a room waits on (`docs/combat.md`), as the tower
/// floors' controllers poll it.
pub const ENEMIES: u16 = 0x0498;
/// `$07ED`, the Prime Blue count in BCD, as runs read it.
pub const PRIME_BLUE: u16 = 0x07ED;

/// Instructions one run may take: the freezing's whitening loops 37 times.
const STEPS: usize = 512;

/// Engine routines a run may call, with the evidence: `$8D:A8EA` / `$8D:A8FD`
/// save and restore the palette buffer (`MVN $7F,$7F` between `$0600` and
/// `$0400`), `$8D:AA96` steps it toward white. `$80:80DF` runs a nested
/// frame, in which `$80:C85E` runs the actors with `+$04` bit 12 -- in the
/// whitening, the figure and the particles; here the run pauses until the
/// next frame and every actor runs. The calls clobber A and the flags, and
/// `$8D:AA96` also X.
const CALLS: [usize; 7] = [
    SAVE_PALETTE,
    RAISE_PALETTE,
    RESTORE_PALETTE,
    NESTED_FRAME,
    FLAG_TEST,
    FLAG_WRITE,
    POSE_STEP,
];
const SAVE_PALETTE: usize = 0x0D_A8EA;
const RAISE_PALETTE: usize = 0x0D_AA96;
const RESTORE_PALETTE: usize = 0x0D_A8FD;
const NESTED_FRAME: usize = 0x00_80DF;
/// `$80:BBC7`: carry = event flag `A & $FFF` (the Magirock's taken flag,
/// `$84:DD8D`).
const FLAG_TEST: usize = 0x00_BBC7;
/// `$80:BBCD`: writes event flag `A & $FFF`, set with bit 15, else clear
/// (tower 3's pedestals, `$90:FBD7`).
const FLAG_WRITE: usize = 0x00_BBCD;
/// `$80:ED75`: one step of the pose, which the runtime's pose clock keeps.
const POSE_STEP: usize = 0x00_ED75;
/// The call among [`CALLS`] that also clobbers X.
const CLOBBERS_X: usize = RAISE_PALETTE;

/// A pushed value: A with its width, or whether X was still the actor's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pushed {
    A(Option<u16>, bool),
    X(bool),
}

/// Whether event flag `word & $FFF` is set (`$80:BBA6`).
fn flag_set(events: &[u8], word: u16) -> bool {
    let flag = usize::from(word & 0x0FFF);
    events
        .get(flag / 8)
        .is_some_and(|byte| byte & (1 << (flag % 8)) != 0)
}

/// Whether an address is a scratch word's: even, so that no two words
/// overlap.
fn scratch(address: u16) -> bool {
    address.is_multiple_of(2)
        && SCRATCH
            .iter()
            .any(|&(first, last)| (first..=last).contains(&address))
}

/// How a run ended.
#[derive(Debug)]
pub enum Ran {
    /// Before an instruction the script loop owns, at this address.
    Next(usize),
    /// In a nested frame (`$80:80DF`): it goes on next frame from here.
    Frame(Paused),
}

/// A run waiting for the next frame: its registers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paused {
    pc: usize,
    a: Option<u16>,
    zero: Option<bool>,
    negative: Option<bool>,
    carry: Option<bool>,
    narrow: bool,
    x: bool,
    stack: Vec<Pushed>,
}

/// Runs native code at `at` and returns where the script loop takes over,
/// or `None` when the code is not a run this module admits.
pub(super) fn run(image: &[u8], at: usize, memory: &mut Memory<'_>) -> Option<Ran> {
    let fresh = Paused {
        pc: at,
        a: memory
            .carried
            .take()
            .and_then(|(target, a)| (target == at).then_some(a)),
        zero: None,
        negative: None,
        carry: None,
        narrow: false,
        x: true,
        stack: Vec::new(),
    };
    go(Machine::resumed(image, fresh), memory, true)
}

/// Goes on with a run paused a frame ago.
pub(super) fn resume(image: &[u8], paused: Paused, memory: &mut Memory<'_>) -> Option<Ran> {
    go(Machine::resumed(image, paused), memory, false)
}

fn go(mut machine: Machine<'_>, memory: &mut Memory<'_>, fresh: bool) -> Option<Ran> {
    for step in 0..STEPS {
        match machine.step(memory)? {
            Flow::On => {}
            Flow::Frame => return Some(Ran::Frame(machine.paused())),
            Flow::Stop => {
                let settled = (step > 0 || !fresh)
                    && !machine.narrow
                    && machine.x
                    && machine.stack.is_empty();
                *memory.carried = machine.jump_target().zip(machine.a).filter(|_| settled);
                return settled.then_some(Ran::Next(machine.pc));
            }
        }
    }
    None
}

/// What an instruction leaves the run to do.
enum Flow {
    /// The next instruction.
    On,
    /// Stop before this one.
    Stop,
    /// End the frame after this one.
    Frame,
}

/// The registers a run uses. A value or flag is `None` until the run sets it:
/// a use before that is refused rather than guessed.
struct Machine<'a> {
    image: &'a [u8],
    pc: usize,
    a: Option<u16>,
    zero: Option<bool>,
    negative: Option<bool>,
    carry: Option<bool>,
    /// `SEP #$20` narrowed the accumulator.
    narrow: bool,
    /// X still holds the actor, as the script loop and the other
    /// recognisers need.
    x: bool,
    /// Values pushed and not yet pulled.
    stack: Vec<Pushed>,
    /// The entity Y holds, and the one A holds for a `TAY`.
    y: Option<Entity>,
    entity: Option<Entity>,
    /// Before the run's first instruction, where Y is a linked child's.
    pc_is_start: bool,
}

impl<'a> Machine<'a> {
    fn resumed(image: &'a [u8], paused: Paused) -> Self {
        let Paused {
            pc,
            a,
            zero,
            negative,
            carry,
            narrow,
            x,
            stack,
        } = paused;
        Self {
            image,
            pc,
            a,
            zero,
            negative,
            carry,
            narrow,
            x,
            stack,
            y: None,
            entity: None,
            pc_is_start: true,
        }
    }

    fn paused(self) -> Paused {
        Paused {
            pc: self.pc,
            a: self.a,
            zero: self.zero,
            negative: self.negative,
            carry: self.carry,
            narrow: self.narrow,
            x: self.x,
            stack: self.stack,
        }
    }

    /// Executes one instruction, or stops before it for the script loop or
    /// another recogniser; `None` refuses the run.
    fn step(&mut self, memory: &mut Memory<'_>) -> Option<Flow> {
        let opcode = *self.image.get(self.pc)?;
        if self.pc_is_start && memory.linked.is_some() {
            self.y = memory.linked.map(Entity::Id);
        }
        self.pc_is_start = false;
        if matches!(
            opcode,
            0xAC | 0xA8 | 0x98 | 0xBC | 0xB9 | 0xD9 | 0x4A | 0x6D | 0xBF
        ) && self.entity_op(opcode, memory).is_some()
        {
            return Some(Flow::On);
        }
        let words = &mut *memory.words;
        self.pc = match opcode {
            0xE2 | 0xC2 => self.width(opcode)?,
            0x8D | 0x9C if self.operand().is_some_and(|address| PPU.contains(&address)) => {
                self.register(opcode, memory.display)?
            }
            0x8D | 0x9C
                if self
                    .operand()
                    .is_some_and(|address| SHADOWS.contains(&address)) =>
            {
                if opcode == 0x8D {
                    self.a?;
                }
                self.pc + 3
            }
            0x09 | 0x29 | 0x49 => self.logic(opcode)?,
            0xBD | 0xDD | 0xFD | 0x7D | 0x9D
                if self.x && !self.narrow && matches!(self.operand(), Some(0 | 2)) =>
            {
                self.position(opcode, memory.position)?
            }
            0xC3 => self.compare_stacked()?,
            0xAD if self.operand() == Some(RANDOM) && !self.narrow => {
                self.set(memory.random);
                self.pc + 3
            }
            0xAD if !self.narrow && self.operand().is_some_and(|at| PROBE.contains(&at)) => {
                let (x, y) = memory.probe;
                self.set(if self.operand() == Some(PROBE[0]) {
                    x
                } else {
                    y
                });
                self.pc + 3
            }
            0x9D | 0x9E | 0xBD | 0xDD | 0xDE
                if self.operand().is_some_and(|field| FIELDS.contains(&field)) =>
            {
                self.field(opcode, memory.own)?
            }
            0x9F if self.long() == Some(SLEEP) && self.x && !self.narrow => {
                *memory.sleep = self.a?;
                self.pc + 4
            }
            0xBF | 0x9F => self.own(opcode, memory.own)?,
            // The palette buffer (`$7F:0600..07FF`): a room's colour effect,
            // not drawn here (`$90:A45C`).
            0x8F if (0x7F_0600..0x7F_0800).contains(&self.long()?) => {
                self.a?;
                self.pc + 4
            }
            0x8F => self.stop_spin(memory.display)?,
            // Scratch words are words: a narrow accumulator refuses them.
            0x9C | 0x8D | 0xAD | 0xEE | 0xCE | 0xC9 | 0xCD | 0x0C | 0x1C if self.narrow => {
                return None;
            }
            0xAD if self
                .operand()
                .is_some_and(|address| READABLE.contains(&address)) =>
            {
                let address = self.operand()?;
                self.set(words.get(&address).copied().unwrap_or(0));
                self.pc + 3
            }
            0x9C | 0x8D | 0xAD | 0xEE | 0xCE | 0x0C | 0x1C => self.memory(opcode, words)?,
            0xA9 | 0xC9 | 0xCD | 0x1A | 0x3A | 0x89 => self.accumulator(opcode, words)?,
            0x48 | 0x68 | 0xDA | 0xFA => self.stack_op(opcode)?,
            0xF0 | 0xD0 | 0x90 | 0xB0 | 0x10 | 0x30 => self.branch(opcode)?,
            0x18 | 0x38 => {
                self.carry = Some(opcode == 0x38);
                self.pc + 1
            }
            0x69 | 0xE9 => self.add(opcode)?,
            0x22 => {
                let (next, frame) = self.call(memory)?;
                self.pc = next;
                return Some(if frame { Flow::Frame } else { Flow::On });
            }
            // The loop's own (`COP`, `RTL`, `JMP`, `BRA`) or another
            // recogniser's: stop before it.
            _ => return Some(Flow::Stop),
        };
        Some(Flow::On)
    }

    /// Where the `BRA` or `JMP` at the pc goes, if it is one.
    fn jump_target(&self) -> Option<usize> {
        let bank = self.pc & 0xFF_0000;
        match *self.image.get(self.pc)? {
            0x80 => {
                let offset = i8::from_ne_bytes([*self.image.get(self.pc + 1)?]);
                Some(bank | ((self.pc + 2).wrapping_add_signed(isize::from(offset)) & 0xFFFF))
            }
            0x4C => Some(bank | usize::from(self.operand()?)),
            _ => None,
        }
    }

    /// `STA` / `STZ` on a PPU register, a byte, or two when wide.
    fn register(&self, opcode: u8, display: &mut Display) -> Option<usize> {
        let address = self.operand()?;
        let value = if opcode == 0x8D { self.a? } else { 0 };
        let [low, high] = value.to_le_bytes();
        display.write(address, low);
        if !self.narrow {
            display.write(address + 1, high);
        }
        Some(self.pc + 3)
    }

    /// `ORA #` and `AND #` at the accumulator's width.
    fn logic(&mut self, opcode: u8) -> Option<usize> {
        let (value, next) = self.immediate()?;
        let a = self.a?;
        self.set(match opcode {
            0x09 => a | value,
            0x29 => a & value,
            _ => a ^ value,
        });
        Some(next)
    }

    /// `LDA`, `CMP`, `SBC`, `ADC`, `STA` on the actor's own x or y
    /// (`$0000,X`, `$0002,X`): the Cadet's aim (`$97:BE2B`), its spells'
    /// place (`$97:C1FC`).
    fn position(&mut self, opcode: u8, position: &mut (u16, u16)) -> Option<usize> {
        let first = self.operand()? == 0;
        let value = if first { position.0 } else { position.1 };
        match opcode {
            0x9D => {
                let a = self.a?;
                if first {
                    position.0 = a;
                } else {
                    position.1 = a;
                }
            }
            0xBD => self.set(value),
            0xDD => self.compare(value)?,
            _ => {
                let (a, carry) = (u32::from(self.a?), u32::from(self.carry?));
                let sum = if opcode == 0x7D {
                    a + u32::from(value) + carry
                } else {
                    a + u32::from(value ^ 0xFFFF) + carry
                };
                self.set(u16::try_from(sum & 0xFFFF).ok()?);
                self.carry = Some(sum > 0xFFFF);
            }
        }
        Some(self.pc + 3)
    }

    /// Y on another entity: `LDY $0DEA` (Ark), `LDA $7F:001E,X; TAY` (the
    /// parent), `LDA`/`CMP` of its fields through `,Y`; and `LSR A`, `ADC`
    /// of a readable word. `None` leaves the instruction to the others.
    fn entity_op(&mut self, opcode: u8, memory: &Memory<'_>) -> Option<()> {
        if self.narrow {
            return None;
        }
        let view = |entity| match entity {
            Entity::Player => Some(memory.player),
            Entity::Parent => memory.parent,
            Entity::Id(id) => memory
                .views
                .iter()
                .find_map(|&(other, view)| (other == id).then_some(view)),
        };
        self.pc = match opcode {
            0xAC if self.operand()? == 0x0DEA => {
                self.y = Some(Entity::Player);
                self.pc + 3
            }
            0xBF if self.long()? == 0x7F_001E && self.x && memory.parent.is_some() => {
                (self.a, self.entity) = (None, Some(Entity::Parent));
                // A pointer, never 0.
                (self.zero, self.negative) = (Some(false), Some(false));
                self.pc + 4
            }
            0xA8 => {
                self.y = Some(self.entity?);
                self.pc + 1
            }
            // TYA of a linked child: its id.
            0x98 => {
                let Entity::Id(id) = self.y? else {
                    return None;
                };
                self.set(id);
                self.pc + 1
            }
            // LDY $0026,X: an id kept there.
            0xBC if self.operand()? == 0x26 && self.x => {
                let id = |at| u16::from(memory.own.get(&at).copied().unwrap_or(0));
                self.y = Some(Entity::Id(id(0x26) | id(0x27) << 8));
                self.pc + 3
            }
            0xB9 | 0xD9 => {
                let value = view(self.y?)?.field(self.operand()?)?;
                if opcode == 0xB9 {
                    self.set(value);
                } else {
                    self.compare(value)?;
                }
                self.pc + 3
            }
            0x4A => {
                let a = self.a?;
                self.set(a >> 1);
                self.carry = Some(a & 1 != 0);
                self.pc + 1
            }
            0x6D => {
                let at = self.operand()?;
                let value = if at == PROBE[0] {
                    memory.probe.0
                } else if at == PROBE[1] {
                    memory.probe.1
                } else if READABLE.contains(&at) {
                    memory.words.get(&at).copied().unwrap_or(0)
                } else {
                    return None;
                };
                let sum = u32::from(self.a?) + u32::from(value) + u32::from(self.carry?);
                self.set(u16::try_from(sum & 0xFFFF).ok()?);
                self.carry = Some(sum > 0xFFFF);
                self.pc + 3
            }
            _ => return None,
        };
        Some(())
    }

    /// `CMP $01,S` against a wide A pushed last.
    fn compare_stacked(&mut self) -> Option<usize> {
        if *self.image.get(self.pc + 1)? != 1 || self.narrow {
            return None;
        }
        let Some(&Pushed::A(Some(value), false)) = self.stack.last() else {
            return None;
        };
        self.compare(value)?;
        Some(self.pc + 2)
    }

    /// The flags of `CMP` of A with `value`, wide.
    fn compare(&mut self, value: u16) -> Option<()> {
        let a = self.a?;
        self.carry = Some(a >= value);
        let difference = a.wrapping_sub(value);
        self.zero = Some(difference == 0);
        self.negative = Some(difference & 0x8000 != 0);
        Some(())
    }

    /// `LDA` / `STA $7F:xxxx,X` on [`OWN`], X still the actor.
    fn own(&mut self, opcode: u8, own: &mut Own) -> Option<usize> {
        let long = self.long()?;
        let width: u16 = if self.narrow { 1 } else { 2 };
        let address = u16::try_from(long & 0xFFFF).ok()?;
        let owned = OWN
            .iter()
            .any(|own| own.contains(&address) && own.contains(&(address + width - 1)));
        if long >> 16 != 0x7F || !self.x || !owned {
            return None;
        }
        if opcode == 0xBF {
            let byte = |at: u16| u16::from(own.get(&at).copied().unwrap_or(0));
            let high = if self.narrow { 0 } else { byte(address + 1) };
            self.set(byte(address) | high << 8);
        } else {
            let value = self.a?.to_le_bytes();
            for (at, byte) in (address..address + width).zip(value) {
                own.insert(at, byte);
            }
        }
        Some(self.pc + 4)
    }

    /// `STA`, `STZ`, `LDA`, `CMP`, `DEC` on one of the entity's [`FIELDS`] (`,X`,
    /// X the actor, 16-bit), kept with its own bytes.
    fn field(&mut self, opcode: u8, own: &mut Own) -> Option<usize> {
        if self.narrow || !self.x {
            return None;
        }
        let at = self.operand()?;
        let read = |own: &Own| {
            u16::from_le_bytes([
                own.get(&at).copied().unwrap_or(0),
                own.get(&(at + 1)).copied().unwrap_or(0),
            ])
        };
        let write = |own: &mut Own, value: u16| {
            let [low, high] = value.to_le_bytes();
            own.insert(at, low);
            own.insert(at + 1, high);
        };
        match opcode {
            0x9D => write(own, self.a?),
            0x9E => write(own, 0),
            0xBD => self.set(read(own)),
            0xDD => self.compare(read(own))?,
            _ => {
                let value = read(own).wrapping_sub(1);
                write(own, value);
                self.zero = Some(value == 0);
                self.negative = Some(value & 0x8000 != 0);
            }
        }
        Some(self.pc + 3)
    }

    /// `STA $7E:46E6` of 0: `COP 6A`'s square stops.
    fn stop_spin(&self, display: &mut Display) -> Option<usize> {
        if self.long()? != SPIN || self.a? != 0 {
            return None;
        }
        display.stop_spin();
        Some(self.pc + 4)
    }

    /// An immediate operand at the accumulator's width, and the next pc.
    fn immediate(&self) -> Option<(u16, usize)> {
        Some(if self.narrow {
            (u16::from(*self.image.get(self.pc + 1)?), self.pc + 2)
        } else {
            (self.operand()?, self.pc + 3)
        })
    }

    /// A long operand.
    fn long(&self) -> Option<u32> {
        let bytes = self.image.get(self.pc + 1..self.pc + 4)?;
        Some(u32::from(bytes[0]) | u32::from(bytes[1]) << 8 | u32::from(bytes[2]) << 16)
    }

    fn operand(&self) -> Option<u16> {
        let bytes = self.image.get(self.pc + 1..self.pc + 3)?;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn address(&self) -> Option<u16> {
        self.operand().filter(|&address| scratch(address))
    }

    fn set(&mut self, value: u16) {
        self.entity = None;
        self.a = Some(value);
        self.zero = Some(value == 0);
        self.negative = Some(value & if self.narrow { 0x80 } else { 0x8000 } != 0);
    }

    /// `SEP` / `REP #$20`: the accumulator's width, and nothing else.
    fn width(&mut self, opcode: u8) -> Option<usize> {
        if *self.image.get(self.pc + 1)? != 0x20 {
            return None;
        }
        let widened = self.narrow && opcode == 0xC2;
        self.narrow = opcode == 0xE2;
        if self.narrow {
            self.a = self.a.map(|value| value & 0xFF);
        }
        // B, the hidden high byte, is not tracked: wide again, A is unknown.
        if widened {
            self.a = None;
        }
        Some(self.pc + 2)
    }

    /// `STZ`, `STA`, `LDA`, `INC`, `DEC`, `TSB`, `TRB` on a scratch word.
    fn memory(&mut self, opcode: u8, words: &mut Scratch) -> Option<usize> {
        let address = self.address()?;
        match opcode {
            0x9C => {
                words.insert(address, 0);
            }
            0x8D => {
                words.insert(address, self.a?);
            }
            0xAD => {
                let value = words.get(&address).copied().unwrap_or(0);
                self.set(value);
            }
            0xEE | 0xCE => {
                let word = words.entry(address).or_insert(0);
                *word = if opcode == 0xEE {
                    word.wrapping_add(1)
                } else {
                    word.wrapping_sub(1)
                };
                let value = *word;
                self.zero = Some(value == 0);
                self.negative = Some(value & 0x8000 != 0);
            }
            _ => {
                // TSB / TRB: Z from `A & word`, before the write.
                let a = self.a?;
                let word = words.entry(address).or_insert(0);
                self.zero = Some(*word & a == 0);
                *word = if opcode == 0x0C {
                    *word | a
                } else {
                    *word & !a
                };
            }
        }
        Some(self.pc + 3)
    }

    /// `LDA #`, `CMP #`, `CMP` a scratch word, `BIT #`, `INC A`, `DEC A`.
    fn accumulator(&mut self, opcode: u8, words: &Scratch) -> Option<usize> {
        match opcode {
            // Immediate BIT sets only Z.
            0x89 if !self.narrow => {
                self.zero = Some(self.a? & self.operand()? == 0);
                Some(self.pc + 3)
            }
            0x89 => None,
            0xA9 => {
                let (value, next) = self.immediate()?;
                self.set(value);
                Some(next)
            }
            0xC9 | 0xCD => {
                let value = if opcode == 0xC9 {
                    self.operand()?
                } else {
                    words.get(&self.address()?).copied().unwrap_or(0)
                };
                self.compare(value)?;
                Some(self.pc + 3)
            }
            _ => {
                let mask = if self.narrow { 0xFF } else { 0xFFFF };
                let a = self.a?;
                let value = if opcode == 0x1A {
                    a.wrapping_add(1)
                } else {
                    a.wrapping_sub(1)
                } & mask;
                self.set(value);
                Some(self.pc + 1)
            }
        }
    }

    /// `PHA`, `PLA`, `PHX`, `PLX`, each pulled as it was pushed.
    fn stack_op(&mut self, opcode: u8) -> Option<usize> {
        match opcode {
            0x48 => self.stack.push(Pushed::A(self.a, self.narrow)),
            0x68 => match self.stack.pop()? {
                Pushed::A(value, narrow) if narrow == self.narrow => self.set(value?),
                _ => return None,
            },
            0xDA => self.stack.push(Pushed::X(self.x)),
            _ => match self.stack.pop()? {
                Pushed::X(valid) => self.x = valid,
                Pushed::A(..) => return None,
            },
        }
        Some(self.pc + 1)
    }

    /// `BEQ`, `BNE`, `BCC`, `BCS`, `BPL`, `BMI`, within the bank.
    fn branch(&self, opcode: u8) -> Option<usize> {
        let taken = match opcode {
            0xF0 => self.zero?,
            0xD0 => !self.zero?,
            0x90 => !self.carry?,
            0xB0 => self.carry?,
            0x10 => !self.negative?,
            _ => self.negative?,
        };
        let next = self.pc + 2;
        if !taken {
            return Some(next);
        }
        let displacement = i8::from_ne_bytes([*self.image.get(self.pc + 1)?]);
        Some((self.pc & 0xFF_0000) | (next.wrapping_add_signed(isize::from(displacement)) & 0xFFFF))
    }

    /// `ADC #` and `SBC #`, wide, the carry known.
    fn add(&mut self, opcode: u8) -> Option<usize> {
        if self.narrow {
            return None;
        }
        // A `SBC` without `SEC` after a `COP` (the Guardner's aim,
        // `$97:C541`): the carry the handler leaves is not tracked; taken as
        // set, the usual state there (guess).
        let carry = self.carry.unwrap_or(opcode == 0xE9);
        let (a, value) = (u32::from(self.a?), u32::from(self.operand()?));
        let sum = if opcode == 0x69 {
            a + value + u32::from(carry)
        } else {
            a + (value ^ 0xFFFF) + u32::from(carry)
        };
        self.set(u16::try_from(sum & 0xFFFF).ok()?);
        self.carry = Some(sum > 0xFFFF);
        Some(self.pc + 3)
    }

    /// `JSL` to one of [`CALLS`]: A and the flags are unknown after it.
    fn call(&mut self, memory: &mut Memory<'_>) -> Option<(usize, bool)> {
        let display = &mut *memory.display;
        let bytes = self.image.get(self.pc + 1..self.pc + 4)?;
        let target =
            usize::from(bytes[0]) | usize::from(bytes[1]) << 8 | usize::from(bytes[2] & 0x7F) << 16;
        let target = target & 0x3F_FFFF;
        // Named in the image's revision; one unrecorded matches nothing.
        let named = |call: usize| assets::layout::offset(self.image, call) == Some(target);
        let call = CALLS.into_iter().find(|&call| named(call))?;
        match call {
            SAVE_PALETTE => display.save_palette(),
            RAISE_PALETTE => display.raise_palette(),
            RESTORE_PALETTE => display.restore_palette(),
            _ => {}
        }
        let flag = (call == FLAG_TEST)
            .then(|| self.a.map(|a| flag_set(memory.events, a)))
            .flatten();
        if call == FLAG_TEST && flag.is_none() {
            return None;
        }
        if call == FLAG_WRITE {
            crate::scene::write_flag(memory.events, self.a?);
        }
        self.a = None;
        (self.zero, self.negative, self.carry) = (None, None, flag);
        if call == CLOBBERS_X {
            self.x = false;
        }
        Some((self.pc + 4, call == NESTED_FRAME))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: usize = 0x09_8000;

    /// A run on `words` alone, to where the script loop takes over.
    fn run(image: &[u8], at: usize, words: &mut Scratch) -> Option<usize> {
        match super::run(
            image,
            at,
            &mut Memory {
                words,
                own: &mut Own::new(),
                display: &mut Display::default(),
                random: 0,
                probe: (0, 0),
                events: &mut [],
                sleep: &mut 0,
                position: &mut (0, 0),
                player: View::default(),
                parent: None,
                linked: None,
                views: &[],
                carried: &mut None,
            },
        )? {
            Ran::Next(next) => Some(next),
            Ran::Frame(_) => None,
        }
    }

    fn next(ran: Option<Ran>) -> Option<usize> {
        match ran? {
            Ran::Next(next) => Some(next),
            Ran::Frame(_) => None,
        }
    }

    #[test]
    fn the_voice_fade_writes_the_colour_math_and_steps_its_own_byte() {
        // `$88:9CD8`'s set-up, then one step of its ramp:
        // SEP #$20; LDA #$16; STA $212C; STA $0468; LDA #$20; STA $2130;
        // LDA #$A3; STA $2131; LDA #$21; STA $2125; STZ $2127; REP #$20;
        // LDA #0; STA $7F:201C,X; COP; SEP #$20; LDA $7F:201C,X; INC;
        // STA $7F:201C,X; ORA #$E0; STA $2132; REP #$20; COP.
        let code = [
            0xE2, 0x20, 0xA9, 0x16, 0x8D, 0x2C, 0x21, 0x8D, 0x68, 0x04, 0xA9, 0x20, 0x8D, 0x30,
            0x21, 0xA9, 0xA3, 0x8D, 0x31, 0x21, 0xA9, 0x21, 0x8D, 0x25, 0x21, 0x9C, 0x27, 0x21,
            0xC2, 0x20, 0xA9, 0x00, 0x00, 0x9F, 0x1C, 0x20, 0x7F, 0x02, 0xE2, 0x20, 0xBF, 0x1C,
            0x20, 0x7F, 0x1A, 0x9F, 0x1C, 0x20, 0x7F, 0x09, 0xE0, 0x8D, 0x32, 0x21, 0xC2, 0x20,
            0x02,
        ];
        let fade = image(&code);
        let (mut words, mut own, mut display) = (Scratch::new(), Own::new(), Display::default());
        let mut memory = Memory {
            words: &mut words,
            own: &mut own,
            display: &mut display,
            random: 0,
            probe: (0, 0),
            events: &mut [],
            sleep: &mut 0,
            position: &mut (0, 0),
            player: View::default(),
            parent: None,
            linked: None,
            views: &[],
            carried: &mut None,
        };
        assert_eq!(next(super::run(&fade, AT, &mut memory)), Some(AT + 37));
        assert!(!memory.display.shows_bg1());
        assert_eq!(memory.display.darkening(), None);
        for step in 1..=2 {
            assert_eq!(next(super::run(&fade, AT + 38, &mut memory)), Some(AT + 56));
            assert_eq!(memory.own.get(&0x201C), Some(&step));
            assert_eq!(memory.display.darkening().unwrap().fixed, [step; 3]);
        }
        // `$88:ABF8`'s door: LDA #0; STA $7E:46E6 stops the square.
        memory.display.spin((184, 352), 3);
        let stop = image(&[0xA9, 0x00, 0x00, 0x8F, 0xE6, 0x46, 0x7E, 0x02]);
        assert_eq!(next(super::run(&stop, AT, &mut memory)), Some(AT + 7));
        memory.display.write(0x2132, 0xE7);
        assert_eq!(memory.display.darkening().unwrap().spared, None);
    }

    fn image(code: &[u8]) -> Vec<u8> {
        let mut image = vec![0; AT];
        image.extend_from_slice(code);
        image
    }

    #[test]
    fn a_flyer_aims_at_arks_probe_and_registers_a_callback() {
        // `$97:B9DA`: LDA $0966; STA $7F:2004,X; LDA $0968; STA $7F:2006,X;
        // `$97:BB92`: LDA #$BB92; STA $7F:1016,X; COP.
        let code = [
            0xAD, 0x66, 0x09, 0x9F, 0x04, 0x20, 0x7F, 0xAD, 0x68, 0x09, 0x9F, 0x06, 0x20, 0x7F,
            0xA9, 0x92, 0xBB, 0x9F, 0x16, 0x10, 0x7F, 0x02,
        ];
        let flyer = image(&code);
        let (mut words, mut own, mut display) = (Scratch::new(), Own::new(), Display::default());
        let mut memory = Memory {
            words: &mut words,
            own: &mut own,
            display: &mut display,
            random: 0,
            probe: (0x0123, 0x0456),
            events: &mut [],
            sleep: &mut 0,
            position: &mut (0, 0),
            player: View::default(),
            parent: None,
            linked: None,
            views: &[],
            carried: &mut None,
        };
        assert_eq!(next(super::run(&flyer, AT, &mut memory)), Some(AT + 21));
        let bytes: Vec<u8> = [0x2004, 0x2005, 0x2006, 0x2007, 0x1016, 0x1017]
            .iter()
            .map(|at| own[at])
            .collect();
        assert_eq!(bytes, [0x23, 0x01, 0x56, 0x04, 0x92, 0xBB]);
    }

    #[test]
    fn the_magirock_tests_its_taken_flag() {
        // `$84:DD83`: LDA $0026,X; AND #$FF; CLC; ADC #$0900; JSL $80:BBC7;
        // BCC +3; COP A7; RTL; COP.
        let mut code = vec![
            0xBD, 0x26, 0x00, 0x29, 0xFF, 0x00, 0x18, 0x69, 0x00, 0x09, 0x22, 0xC7, 0xBB, 0x80,
            0x90, 0x03, 0x02, 0xA7, 0x6B, 0x02,
        ];
        code.resize(code.len() + 4, 0);
        let magirock = image(&code);
        let mut events = vec![0; 0x200];
        for (taken, at) in [(false, AT + 19), (true, AT + 16)] {
            events[0x905 / 8] = u8::from(taken) << (0x905 % 8);
            let (mut words, mut display) = (Scratch::new(), Display::default());
            let mut own = Own::from([(0x26, 5), (0x27, 0)]);
            let mut memory = Memory {
                words: &mut words,
                own: &mut own,
                display: &mut display,
                random: 0,
                probe: (0, 0),
                events: &mut events,
                sleep: &mut 0,
                position: &mut (0, 0),
                player: View::default(),
                parent: None,
                linked: None,
                views: &[],
                carried: &mut None,
            };
            assert_eq!(next(super::run(&magirock, AT, &mut memory)), Some(at));
        }
    }

    #[test]
    fn the_pedestal_sets_then_clears_its_flag() {
        // `$90:FBD7`: LDA $0026,X; ORA #$8000; JSL $80:BBCD; LDA #$0080;
        // TSB $045A; COP. `$90:FBFB` drops the ORA: the flag clears.
        let set = [
            0xBD, 0x26, 0x00, 0x09, 0x00, 0x80, 0x22, 0xCD, 0xBB, 0x80, 0xA9, 0x80, 0x00, 0x0C,
            0x5A, 0x04, 0x02,
        ];
        let clear = [
            0xBD, 0x26, 0x00, 0x22, 0xCD, 0xBB, 0x80, 0xA9, 0x80, 0x00, 0x0C, 0x5A, 0x04, 0x02,
        ];
        let mut events = vec![0; 0x200];
        for (code, on) in [(&set[..], true), (&clear[..], false)] {
            let pedestal = image(code);
            let (mut words, mut display) = (Scratch::new(), Display::default());
            let mut own = Own::from([(0x26, 3), (0x27, 0)]);
            let mut memory = Memory {
                words: &mut words,
                own: &mut own,
                display: &mut display,
                random: 0,
                probe: (0, 0),
                events: &mut events,
                sleep: &mut 0,
                position: &mut (0, 0),
                player: View::default(),
                parent: None,
                linked: None,
                views: &[],
                carried: &mut None,
            };
            let end = AT + code.len() - 1;
            assert_eq!(next(super::run(&pedestal, AT, &mut memory)), Some(end));
            assert_eq!(events[0] & 1 << 3 != 0, on);
        }
    }

    #[test]
    fn the_cadet_compares_its_distances_to_the_target_on_the_stack() {
        // `$97:BE26`: LDA $7F:2004,X; SEC; SBC $0000,X; BPL +4; EOR #$FFFF;
        // INC; PHA; LDA $7F:2006,X; SEC; SBC $0002,X; BPL +4; EOR #$FFFF;
        // INC; CMP $01,S; BCC +1; PLA; PLA; COP.
        let code = [
            0xBF, 0x04, 0x20, 0x7F, 0x38, 0xFD, 0x00, 0x00, 0x10, 0x04, 0x49, 0xFF, 0xFF, 0x1A,
            0x48, 0xBF, 0x06, 0x20, 0x7F, 0x38, 0xFD, 0x02, 0x00, 0x10, 0x04, 0x49, 0xFF, 0xFF,
            0x1A, 0xC3, 0x01, 0x90, 0x01, 0x68, 0x68, 0x02,
        ];
        let cadet = image(&code);
        // Target 64 left and 8 below: |dy| < |dx|, the BCC skips one PLA.
        let (mut words, mut display) = (Scratch::new(), Display::default());
        let mut own = Own::from([(0x2004, 0x40), (0x2005, 0), (0x2006, 0x88), (0x2007, 0)]);
        let mut memory = Memory {
            words: &mut words,
            own: &mut own,
            display: &mut display,
            random: 0,
            probe: (0, 0),
            events: &mut [],
            sleep: &mut 0,
            position: &mut (0x80, 0x80),
            player: View::default(),
            parent: None,
            linked: None,
            views: &[],
            carried: &mut None,
        };
        assert_eq!(next(super::run(&cadet, AT, &mut memory)), Some(AT + 35));
        // Target 8 left and 64 below: no branch, and the second PLA finds
        // nothing to pull.
        memory.own.insert(0x2004, 0x78);
        memory.own.insert(0x2006, 0xC0);
        assert!(super::run(&cadet, AT, &mut memory).is_none());
    }

    #[test]
    fn a_spell_aims_between_its_parent_and_ark() {
        // `$97:C209`: LDA $7F:001E,X; TAY; LDA $0000,Y; CLC; ADC $0966; LSR;
        // STA $7F:2004,X; LDY $0DEA; LDA $0014,Y; STA $0000,X; COP.
        let code = [
            0xBF, 0x1E, 0x00, 0x7F, 0xA8, 0xB9, 0x00, 0x00, 0x18, 0x6D, 0x66, 0x09, 0x4A, 0x9F,
            0x04, 0x20, 0x7F, 0xAC, 0xEA, 0x0D, 0xB9, 0x14, 0x00, 0x9D, 0x00, 0x00, 0x02,
        ];
        let spell = image(&code);
        let (mut words, mut own, mut display) = (Scratch::new(), Own::new(), Display::default());
        let mut position = (0, 0);
        let mut memory = Memory {
            words: &mut words,
            own: &mut own,
            display: &mut display,
            random: 0,
            probe: (0x100, 0x80),
            events: &mut [],
            sleep: &mut 0,
            position: &mut position,
            player: View {
                x: 0x100,
                y: 0x88,
                facing: 3,
            },
            parent: Some(View {
                x: 0x80,
                ..View::default()
            }),
            linked: None,
            views: &[],
            carried: &mut None,
        };
        assert_eq!(next(super::run(&spell, AT, &mut memory)), Some(AT + 26));
        assert_eq!((own[&0x2004], own[&0x2005]), (0xC0, 0));
        assert_eq!(position, (3, 0), "Ark's facing, as written");
    }

    #[test]
    fn palette_buffer_writes_and_a_subtraction_after_a_cop_go_by() {
        // `$90:A459`: LDA #0; STA $7F:0640; `$97:C53E`: LDA $0968; SBC #$10;
        // STA $7F:2006,X; COP.
        let code = [
            0xA9, 0x00, 0x00, 0x8F, 0x40, 0x06, 0x7F, 0xAD, 0x68, 0x09, 0xE9, 0x10, 0x00, 0x9F,
            0x06, 0x20, 0x7F, 0x02,
        ];
        let run = image(&code);
        let (mut words, mut own, mut display) = (Scratch::new(), Own::new(), Display::default());
        let mut memory = Memory {
            words: &mut words,
            own: &mut own,
            display: &mut display,
            random: 0,
            probe: (0, 0x80),
            events: &mut [],
            sleep: &mut 0,
            position: &mut (0, 0),
            player: View::default(),
            parent: None,
            linked: None,
            views: &[],
            carried: &mut None,
        };
        assert_eq!(next(super::run(&run, AT, &mut memory)), Some(AT + 17));
        assert_eq!(own[&0x2006], 0x70);
    }

    #[test]
    fn a_watcher_reads_the_players_action_word_and_tests_its_bits() {
        // LDA $097C; BIT #$0300; BNE +2; BRA (the loop's).
        let code = [
            0xAD, 0x7C, 0x09, 0x89, 0x00, 0x03, 0xD0, 0x02, 0x80, 0xE5, 0x02,
        ];
        let watcher = image(&code);
        let mut words = BTreeMap::new();
        assert_eq!(
            run(&watcher, AT, &mut words),
            Some(AT + 8),
            "idle: on to the BRA"
        );
        words.insert(PLAYER_ACTION, 0x0100);
        assert_eq!(
            run(&watcher, AT, &mut words),
            Some(AT + 10),
            "acting: past it"
        );
        // The word is read, never written.
        let store = image(&[0xA9, 0x01, 0x00, 0x8D, 0x7C, 0x09, 0x02]);
        assert_eq!(run(&store, AT, &mut words), None);
        // No Prime Blue: LDA $07ED; BNE +9 falls through to the COP.
        let broke = image(&[0xAD, 0xED, 0x07, 0xD0, 0x09, 0x02, 0x3B]);
        assert_eq!(run(&broke, AT, &mut words), Some(AT + 5));
    }

    #[test]
    fn the_controller_steps_the_shared_counter_and_waits_for_its_turn() {
        // INC $04BC; COP BC (the loop's); LDA $04BC; CMP #2; BEQ +1; RTL.
        let code = [
            0xEE, 0xBC, 0x04, 0x02, 0xBC, 0xAD, 0xBC, 0x04, 0xC9, 0x02, 0x00, 0xF0, 0x01, 0x6B,
            0x02,
        ];
        let image = image(&code);
        let mut words = BTreeMap::new();
        assert_eq!(
            run(&image, AT, &mut words),
            Some(AT + 3),
            "stops at the COP"
        );
        assert_eq!(words.get(&0x04BC), Some(&1));
        assert_eq!(
            run(&image, AT + 5, &mut words),
            Some(AT + 13),
            "not yet: RTL"
        );
        words.insert(0x04BC, 2);
        assert_eq!(run(&image, AT + 5, &mut words), Some(AT + 14), "its turn");
    }

    #[test]
    fn the_guide_clears_its_words_and_loops_back_until_the_count_is_reached() {
        // STZ $0440; STZ $04BC; LDA $04BC; CMP #1; BNE -7 (to the LDA); COP.
        let code = [
            0x9C, 0x40, 0x04, 0x9C, 0xBC, 0x04, 0xAD, 0xBC, 0x04, 0xC9, 0x01, 0x00, 0xD0, 0xF8,
            0x02,
        ];
        let image = image(&code);
        let mut words = BTreeMap::from([(0x04BC, 7), (0x0440, 3)]);
        assert_eq!(
            run(&image, AT, &mut words),
            None,
            "spins: no COP within budget"
        );
        assert_eq!(words.get(&0x0440), Some(&0));
        words.insert(0x04BC, 1);
        assert_eq!(run(&image, AT + 6, &mut words), Some(AT + 14));
    }

    #[test]
    fn tsb_and_trb_set_and_clear_bits_of_048a() {
        // LDA #$0100; TSB $048A; COP; then LDA #$0100; TRB $048A; COP.
        let code = [
            0xA9, 0x00, 0x01, 0x0C, 0x8A, 0x04, 0x02, 0xA9, 0x00, 0x01, 0x1C, 0x8A, 0x04, 0x02,
        ];
        let image = image(&code);
        let mut words = BTreeMap::from([(0x048A, 0x8000)]);
        assert_eq!(run(&image, AT, &mut words), Some(AT + 6));
        assert_eq!(words.get(&0x048A), Some(&0x8100));
        assert_eq!(run(&image, AT + 7, &mut words), Some(AT + 13));
        assert_eq!(words.get(&0x048A), Some(&0x8000));
    }

    #[test]
    fn a_whitening_loop_runs_through_display_calls_and_hands_back() {
        // PHX; SEP #$20; LDA #$A3; STA $2131; LDA #$02; PHA; JSL $8D:AA96;
        // JSL $80:80DF; PLA; DEC; BPL -13; REP #$20; PLX; LDA $0004,X.
        let code = [
            0xDA, 0xE2, 0x20, 0xA9, 0xA3, 0x8D, 0x31, 0x21, 0xA9, 0x02, 0x48, 0x22, 0x96, 0xAA,
            0x8D, 0x22, 0xDF, 0x80, 0x80, 0x68, 0x3A, 0x10, 0xF3, 0xC2, 0x20, 0xFA, 0xBD, 0x04,
            0x00,
        ];
        let whitening = image(&code);
        let (mut words, mut own, mut display) = (Scratch::new(), Own::new(), Display::default());
        let mut memory = Memory {
            words: &mut words,
            own: &mut own,
            display: &mut display,
            random: 0,
            probe: (0, 0),
            events: &mut [],
            sleep: &mut 0,
            position: &mut (0, 0),
            player: View::default(),
            parent: None,
            linked: None,
            views: &[],
            carried: &mut None,
        };
        // Each pass raises the palette and ends the frame in `$80:80DF`.
        let mut ran = super::run(&whitening, AT, &mut memory);
        for step in 1..=3 {
            let Some(Ran::Frame(paused)) = ran else {
                panic!("pass {step}: {ran:?}");
            };
            assert_eq!(memory.display.whitening(), Some(step));
            ran = resume(&whitening, paused, &mut memory);
        }
        assert_eq!(next(ran), Some(AT + 26), "at the LDA $0004,X");
        // A call outside the display routines is refused.
        let code = [0x22, 0x00, 0x80, 0x80, 0x02];
        assert_eq!(run(&image(&code), AT, &mut words), None);
    }

    #[test]
    fn a_call_clobbers_a_and_the_flags_and_aa96_x_until_pulled() {
        // LDA #1; JSL $8D:A8EA; BNE: the flags are gone.
        let code = [0xA9, 0x01, 0x00, 0x22, 0xEA, 0xA8, 0x8D, 0xD0, 0x00, 0x02];
        assert_eq!(run(&image(&code), AT, &mut BTreeMap::new()), None);
        // JSL $8D:AA96 without PHX/PLX around it: X is not the actor's.
        let code = [0x22, 0x96, 0xAA, 0x8D, 0x02];
        assert_eq!(run(&image(&code), AT, &mut BTreeMap::new()), None);
        // PHX; JSL $8D:AA96; PLX: hands back.
        let code = [0xDA, 0x22, 0x96, 0xAA, 0x8D, 0xFA, 0x02];
        assert_eq!(run(&image(&code), AT, &mut BTreeMap::new()), Some(AT + 6));
        // A narrow PHA pulled wide is refused.
        let code = [0xE2, 0x20, 0xA9, 0x05, 0x48, 0xC2, 0x20, 0x68, 0x02];
        assert_eq!(run(&image(&code), AT, &mut BTreeMap::new()), None);
    }

    #[test]
    fn other_addresses_widths_and_opcodes_are_refused() {
        let mut words = BTreeMap::new();
        for code in [
            &[0x9C, 0x00, 0x05, 0x02][..], // STZ $0500
            &[0xAD, 0x54, 0x04, 0x02][..], // LDA $0454 (the pad)
            &[0xE2, 0x20, 0x02][..],       // SEP #$20
            &[0x02, 0xBD][..],             // nothing native to run
            &[0xAD, 0xBD, 0x04, 0x02][..], // LDA $04BD: an odd address
            &[0x8D, 0xBC, 0x04, 0x02][..], // STA before A is known
            &[0xD0, 0x00, 0x02][..],       // BNE before the flags are
        ] {
            assert_eq!(run(&image(code), AT, &mut words), None, "{code:02X?}");
        }
    }
}
