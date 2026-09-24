//! A bounded interpreter for a resident's ordinary loop.
//!
//! The script walker reads a script once and reports what it contains. A
//! resident who wanders needs it *run*: the loop re-selects poses, draws a
//! random direction, steps, and waits, for as long as they stand there. This
//! executes the handful of services those loops use and the two native
//! opcodes that close them, and freezes a resident at anything else rather
//! than guessing.
//!
//! What is modelled is what `$80:8E56` decides -- the rectangle, the random
//! draw, the probe -- and what `$80:8F32` selects. What is approximated is
//! timing outside a source-admitted COP26 action: other tiles use
//! [`STEP_FRAMES`] and step waits use [`WAIT_FRAMES`]. An admitted action
//! lasts as long as the resident's own walking or idle list; see `cadence`.

mod cadence;
mod ease;
mod motion;
mod native;
pub use native::{Scratch, PLAYER_ACTION, PRIME_BLUE};

use crate::scene::{Globals, Transfer};
use assets::maps::actor_script::{
    self, chained_condition_holds, chained_condition_length, flag_branch_taken, BRANCH_ON_FLAG,
    CHAINED_BRANCH, CHAINED_DESPAWN, REGISTER_CALLBACK, SHOW_TEXT, WRITE_FLAG,
};
use assets::maps::scripts::EventFlags;
use assets::text::HouseDialogue;
use room_core::Direction;

/// Selects an animation sequence; one operand byte.
const SELECT_POSE: u8 = 0x80;
/// Clears the horizontal mirror.
const CLEAR_HFLIP: u8 = 0xB6;
/// Sets the horizontal mirror.
const SET_HFLIP: u8 = 0xB7;
/// Resolves the pose and yields until it is done.
const WAIT: u8 = 0x8E;
/// Shows the published text and blocks the world until it is acknowledged;
/// `$80:8C4A`.
const TEXT_WAIT: u8 = 0x1F;
/// Shows the published text one step a frame while the world runs;
/// `$80:8C9A`.
const TEXT_STEP: u8 = 0x20;
/// Asks a choice and blocks until it is answered, then jumps through a
/// three-entry table (cancel, option 1, option 2); `$80:8B85`. Operands: a
/// catalog byte and the table's bank-relative address.
const CHOICE: u8 = 0x1A;
/// Points the actor's own script at a long address; `$80:AAFB`.
const SET_SCRIPT: u8 = 0xC0;
/// Unlocks pad buttons, `$045E &= !mask`; `$80:8FE6`.
const UNLOCK_INPUT: u8 = 0x29;
/// Locks pad buttons, `$045E |= mask`; `$80:8FF5`.
const LOCK_INPUT: u8 = 0x2A;
/// [`STOP_FOR_PLAYER`] with its own pose base; `$80:8D0B`. Operands: the
/// base selector, bit 7 keeping the mirror, then the target.
const FACE_PLAYER_POSED: u8 = 0x24;
/// Deletes the actor on one flag, set with bit 15, clear without; `$80:96CB`.
const DELETE_ON_FLAG: u8 = 0x48;
/// Long jump; `$80:864C`.
const LONG_JUMP: u8 = 0x06;
/// Stores the next command as the continuation and goes on; `$80:AAA5`.
const CONTINUATION: u8 = 0xBC;
/// Gives an item; `$80:99EB`. Operands: the item and a target in the
/// script's bank, taken when the inventory is full.
const GIVE_ITEM: u8 = 0x54;
/// Grants an item with its presentation: item, the player's pose word and a
/// sound id (`$80:9A04`); `$8D:9653` adds the item, or a unit of one held.
const GRANT_ITEM: u8 = 0x60;
/// Requests dialogue at a bank-first address: bank, then the word
/// (`$80:8C28`), as `COP 1B` does in the script's own bank.
const SHOW_TEXT_BANKED: u8 = 0x1C;
/// Starts a scripted vertical leg toward a tile row; `$80:929C`. Operands:
/// the pose, the movement vector and the row, signed, times 16. At the row it
/// skips the leg's `COP 8E; COP 3B; BRA` loop.
const WALK_TO_ROW: u8 = 0x3A;
/// The same toward a tile column, times 16 plus 8; `$80:921F`.
const WALK_TO_COLUMN: u8 = 0x39;
/// Unlinks the actor; `$80:A876`.
const DELETE: u8 = 0xA7;
/// Marks the actor's cell occupied in the collision grid (`$80:BE8E`).
const OCCUPY: u8 = 0x3B;
/// Places the actor on a tile and faces it; `$80:89DA`. Operands: column
/// and row (signed, read through `$80:BC2F`) and a facing, 0 down, 1 up,
/// 2 left, 3 right; only left mirrors. It lifts the old cell's mark.
const PLACE: u8 = 0x13;
/// Deletes the actor on the map: when `word & $7FFF` is the map, or with
/// bit 15 when it is not.
const DELETE_ON_MAP: u8 = 0x49;
/// Selects a pose and a repeat count that the next `COP 8F` plays out.
/// Operands: the count, then the pose.
const REPEAT_POSE: u8 = 0x85;
/// Map-local counters at `$0640`; see [`Globals::count`].
const COUNT: u8 = 0x4B;
/// Yields for one frame.
const YIELD: u8 = 0xBD;
/// Branches when a cell holds a tile; `$80:9444`. Operands: column and row
/// offsets from the actor's cell (signed), the tile (low nine bits) and the
/// target.
const TILE_BRANCH: u8 = 0x42;
/// Patches a cell's tile; `$80:949B`. Operands: offsets as `COP 42`, then a
/// word: the tile in bits 0-8, and in the high byte, shifted right twice, the
/// frames to wait after.
const PATCH: u8 = 0x44;
/// Registers where a hit sends the script (`+$04 |= $0200`); `$80:9D25`.
/// Operands: the hit target, then a long return address for `COP 66`.
const HIT_TARGET: u8 = 0x65;
/// Returns from a hit to `COP 65`'s return address; `$80:9D5A`.
const HIT_RETURN: u8 = 0x66;
/// Branches when a `COP 4B` counter holds a word, or with the counter's
/// bit 7 / 6 exceeds / is below it; `$80:9713`. Operands: the counter, the
/// word and the target.
const COUNT_BRANCH: u8 = 0x4A;
/// Goes on while any of the mask's pad buttons is held (`$0454`), otherwise
/// jumps; `$80:90C0`.
const HELD_BRANCH: u8 = 0x2F;
/// Inline native code that tests the player's animation: `PHX; LDX $0DEA;
/// LDA $7F:2016,X; CMP #resource; BNE; LDA $7F:0008,X; CMP #selector; BNE;
/// PLX`, both branches to a `PLX`. Operand bytes are wildcards (`None`).
const PLAYER_POSE_TEST: [Option<u8>; 23] = [
    Some(0xDA),
    Some(0xAE),
    Some(0xEA),
    Some(0x0D), // PHX; LDX $0DEA
    Some(0xBF),
    Some(0x16),
    Some(0x20),
    Some(0x7F),
    Some(0xC9),
    None,
    None,
    Some(0xD0),
    None,
    Some(0xBF),
    Some(0x08),
    Some(0x00),
    Some(0x7F),
    Some(0xC9),
    None,
    None,
    Some(0xD0),
    None,
    Some(0xFA),
];
/// Marks, and unmarks, a further cell occupied; `$80:9327`/`935D`.
/// Operands: a mode (0: offsets from the actor), then column and row.
const STAMP: u8 = 0x3D;
const UNSTAMP: u8 = 0x3E;
/// Spawns an actor running a long script with a flags word; `$80:A71B`.
const SPAWN: u8 = 0xA2;
/// Spawns one the same way, at the head of the actor list (`$0DFA`) rather
/// than after its parent, and without a parent link; `$80:A4B6`. Spawns all
/// run from the next frame here.
const SPAWN_LINKED: u8 = 0x99;
/// Frames a hit leaves the target unhittable (`$7F:1020 = $10`).
const HIT_COOLDOWN: u16 = 16;
/// Services without a modelled effect: a cosmetic helper (`6A`), PPU
/// register writes (`76`) and the hit profile (`D9`, `$7F:1022` from
/// `$8D:BDFA`). Their operands are stepped over.
const COSMETIC: [(u8, usize); 3] = [(0x6A, 2), (0x76, 2), (0xD9, 1)];
/// Jumps through a table of words on the spawn parameter (entity `+$26`):
/// operands the lowest and highest value, then a target per value; above
/// the highest, on past the table (`$80:8CD8`). Below the lowest the
/// handler's index wraps; no slice script reaches that, and here it goes
/// on past the table too.
const SWITCH: u8 = 0x22;
/// Picks the movement resource base (`$7F:0022`, `$80:A975`): `$4000 +
/// n << 12`, or with `FF` a word and a bank, which is not modelled.
const SPEED: u8 = 0xB0;
/// Selects a pose and starts the movement streams of the same selector
/// (`$80:A1B9`).
const POSE_MOVING: u8 = 0x81;
/// Sets the next `COP 8F`'s repetitions, a pose, and another selector's
/// movement streams (`$80:A1D4`).
const REPEAT_MOVING: u8 = 0x87;
/// The common movement resource, at `$7F:6000` from `$AB:F037`.
const COMMON_SOURCE: usize = 0x2B_F037;
const COMMON_SIZE: usize = 0x1A0C;

/// Which movement resource an actor's streams read (`$7F:0022`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Base {
    /// `$7F:6000`: descriptor mode `$20`, or `COP B0 02`.
    Common,
    /// The actor's own descriptor's resource. Natively private resources
    /// are packed from `$7F:4000` on in load order (`$80:FB5A`); `COP B0
    /// 00` names the first, taken here to be the actor's own, as for every
    /// mover in the slice.
    Own,
    /// Anywhere else (`COP B0 FF`, other bases): no movement is modelled.
    Unknown,
}
/// Music: play a track (`$80:90D4`), fade out and play one (`$80:9107`),
/// play a selection or the map's (`$80:913C`) -- each through a worker
/// actor the runtime does not need ([`crate::audio`]).
const PLAY_TRACK: u8 = 0x30;
const FADE_TO_TRACK: u8 = 0x31;
const PLAY_SELECTION: u8 = 0x32;
/// Sound effects: port 3 (`$80:91E8`, `$04B7`), port 2 (`$80:91FC`,
/// `$04B6`), both (`$80:9210`).
const SOUND_PORT3: u8 = 0x36;
/// `$92:CC8A`, the shops' spawner: native code that spawns a talk target
/// for each shop record of the map, then deletes itself. The world keeps
/// the targets ([`crate::shop`]), so the spawner just ends.
pub const SHOP_SPAWNER: u32 = 0x92_CC8A;
const SOUND_PORT2: u8 = 0x37;
const SOUND_WORD: u8 = 0x38;
/// Branches on the player inside a rectangle of cells around the actor;
/// `$80:87C2`. Operands: facing, four signed cell offsets, target.
const NEAR_BRANCH: u8 = 0x0D;
/// Takes the player's script once no forced action runs (`$097C & $0810`);
/// `$80:B827`. Operand: the long script.
const TAKE_PLAYER: u8 = 0xDF;
/// Plays the pose and ends the script's frame as an `RTL` does; `$80:A395`.
const ANIMATE_AND_END: u8 = 0x91;
/// Calls a long subroutine, keeping one return (`$7F:0004`); `$80:8592`.
const CALL: u8 = 0x00;
/// Returns from it, or goes on when none is kept; `$80:85B8`.
const RETURN: u8 = 0x01;
/// Starts an eased move: pose, then x and y offsets; `$80:9F4C`.
const EASE_START: u8 = 0xED;
/// Steps it by a speed each frame until it ends; `$80:9F93`.
const EASE_STEP: u8 = 0xEE;
/// Queues a map transfer: map, mode, selector, x, y; `$80:8A23`.
const TRANSFER: u8 = 0x14;
/// Inline native code that registers the contact callback: `LDA #target;
/// STA $7F:1010,X`, with the target's two bytes as wildcards.
const CONTACT: [Option<u8>; 7] = [
    Some(0xA9),
    None,
    None,
    Some(0x9F),
    Some(0x10),
    Some(0x10),
    Some(0x7F),
];
/// Inline native code that makes the player immune to damage (`+$06 |=
/// $20` through `$0DEA`); the runtime has no damage.
const NO_DAMAGE: [u8; 12] = [
    0xAC, 0xEA, 0x0D, 0xB9, 0x06, 0x00, 0x09, 0x20, 0x00, 0x99, 0x06, 0x00,
];
/// Waits for a track to load (`$04B8 == $FFFF`), then three frames;
/// `$80:918F`. The host loads tracks on its own; only the three frames.
const MUSIC_WAIT: u8 = 0x33;
/// Waits for a flag, yielding each frame on itself; `$80:862E`. Without
/// bit 15 it waits until the flag is set, with it until the flag is clear.
const WAIT_FOR_FLAG: u8 = 0x05;
/// Inline native code that hides the actor: `LDA $0004,X; ORA #$8000;
/// STA $0004,X`. Entity `+$04` bit 15 keeps it out of the draw list
/// (`$80:EB68`) and stops its animation and movement; its script runs on.
/// One of the `+$04` writes [`Actor::native_idiom`] reads.
#[cfg(test)]
const HIDE: [u8; 9] = [0xBD, 0x04, 0x00, 0x09, 0x00, 0x80, 0x9D, 0x04, 0x00];
/// Inline native code that shows it again: `AND #$7FFF`.
#[cfg(test)]
const SHOW: [u8; 9] = [0xBD, 0x04, 0x00, 0x29, 0xFF, 0x7F, 0x9D, 0x04, 0x00];
/// Entity `+$06` bit that lets the player interact from any side.
const INTERACT_ANY_SIDE: u16 = 0x0200;
/// Entity `+$06` bit that lets the player interact only facing the actor.
const INTERACT_FACING: u16 = 0x0100;
/// Waits for a step to finish, or for one record when there was no step.
const WAIT_STEP: u8 = 0x8F;
/// Random walk inside a tile rectangle; `$80:8E56`.
const RANDOM_STEP: u8 = 0x26;
/// Branches on whether the player stands at a map position; `$80:888A`.
///
/// Operands: a selector byte, a tile X and Y as signed bytes, and a target.
/// A selector of `$7F` always qualifies; any other is compared with `$0956`,
/// which `docs/house-conversation.md` records as the player's facing, and a
/// mismatch counts as not near. The position, scaled through `$80:BC2F`, is
/// near when it is no more than sixteen pixels beyond the player's on both
/// axes. Bit 7 of the selector inverts the sense: clear branches when near,
/// set when not.
const BRANCH_ON_PLAYER_NEAR: u8 = 0x0F;
/// Branches on a bit of `$0454`; `$80:90AC`.
///
/// `$0454` is the held-button word: `COP 2B`, `COP 2D` and `COP 61` compare
/// it against masks, and a trace shows `$0100` while Right is held and
/// `$0400` while Down is. The runtime has no button state to offer an
/// actor, so it reads the word as zero and never takes the branch.
const BRANCH_ON_GLOBAL: u8 = 0x2E;
/// Stops for a player who stands beside the actor and faces them;
/// `$80:8D1E`. Operand: a two-byte target.
///
/// When the player is nine to sixteen pixels away on one axis and within
/// eight on the other, and `$0956` says they face the actor, the actor turns
/// to face the player, selects the standing sequence for that facing, sets
/// the script pointer to the target and yields. Otherwise the loop continues.
/// The handler skips the test while bit 14 of the slot word at +4 is set,
/// which the runtime never sets, and while `$0999` is nonzero, which a
/// traced conversation never made it.
const STOP_FOR_PLAYER: u8 = 0x23;
/// Holds the actor while the scene pauses them; `$80:9AC7`. One operand
/// byte.
///
/// The handler tests bit 14 of the slot word at +4. When it is set the
/// script pointer is rewound onto the service, the operand becomes the
/// scheduler's countdown at +$0E, and the actor yields. The game sets that
/// bit on every actor for a few frames around a map transition and never
/// during a conversation; the runtime never sets it, so the service
/// continues.
const HOLD_WHILE_PAUSED: u8 = 0x59;
/// Starts a counted loop; `$80:85DF`. Operand: a two-byte count. The count
/// and the address after the operand are the slot's single loop level.
const LOOP_START: u8 = 0x02;
/// Ends a pass of the counted loop; `$80:85F8`. Decrements the count; while
/// it is nonzero, jumps to the loop start and yields one frame.
const LOOP_END: u8 = 0x03;
/// Branches on the map; `$80:8720`. Operands: a word and a two-byte target.
/// Taken when the word's low fifteen bits are the map; bit 15 inverts.
const BRANCH_ON_MAP: u8 = 0x0A;
/// Suspends the script for n frames and resumes on the frame after;
/// `$80:AB17`. Operand: two-byte n. An action under way keeps moving.
const TIMED_WAIT: u8 = 0xC1;

/// Frames a one-tile step takes.
pub const STEP_FRAMES: u16 = 8;
/// Frames a wait lasts.
pub const WAIT_FRAMES: u16 = 8;
/// Commands one frame may execute before the actor is treated as spinning.
const BUDGET: usize = 64;
/// Attributes the step probe accepts; `$80:C0E2`. Bit 15 of the cell counts,
/// so an occupied cell never qualifies.
const PASSABLE: [u16; 3] = [0, 1, 22];

/// What an actor is doing this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Executing commands.
    Running,
    /// Standing for a number of frames.
    Waiting(u16),
    /// Walking one tile.
    Moving {
        /// Direction of travel.
        direction: Direction,
        /// Pixels still to cover.
        remaining: u16,
    },
    /// Source-qualified COP26 plus its following COP8F, including this tick's
    /// final display frame. A zero remainder resumes commands next tick.
    Ordinary {
        direction: Option<Direction>,
        ticks: u16,
        ticks_left: u16,
        destination: Option<(u16, u16)>,
    },
    /// Waiting in a blocking text or choice service; the world stops for it.
    Blocked(Wait),
    /// Stopped at something the interpreter does not model.
    Frozen,
    /// Removed by a despawn.
    Gone,
}

/// What a blocking service waits for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    /// The published text, fully acknowledged.
    Text,
    /// A choice's answer; the jump table's normalized offset.
    Choice(usize),
}

/// How a run of commands ended.
enum Run {
    /// Yielded to the next frame, or froze.
    Yielded,
    /// Reached `RTL` in a callback.
    Ended,
    /// Entered a blocking service.
    Blocked(Wait),
}

/// A callback's view of the actor's own script, saved while it runs.
#[derive(Debug, Clone, Copy)]
struct Outer {
    pc: usize,
    state: State,
}

/// The map an actor moves through, for one frame.
pub struct Surroundings<'a> {
    /// The ROM image.
    pub image: &'a [u8],
    /// Event flags, the dialogue window and the input mask, which scripts change.
    pub globals: &'a mut Globals,
    /// Collision cells, `width * height` of them.
    pub cells: &'a [u16],
    /// Grid width in cells.
    pub width: u16,
    /// Grid height in cells.
    pub height: u16,
    /// Cells other bodies stand on or are stepping into, and the player's.
    pub occupied: &'a [(u16, u16)],
    /// The player's pixel position.
    pub player: (u16, u16),
    /// The way the player faces; `$0956`.
    pub facing: Direction,
}

/// One resident's running script and where it has put them.
#[allow(clippy::struct_excessive_bools)] // independent actor bits, not a state
#[derive(Debug, Clone)]
pub struct Actor {
    /// Pixel position, the record's origin until a step moves it.
    pub position: (u16, u16),
    /// Way the actor faces; the record's until a step turns them.
    pub facing: Direction,
    /// Sequence selected.
    pub selector: u8,
    /// Horizontal mirror in force.
    pub hflip: bool,
    /// Frames since the pose changed or a qualified action restarted it.
    pub pose_age: u32,
    /// Whether a step is under way.
    pub walking: bool,
    /// Entity `+$04` bit 15: not drawn, not animated, not moved.
    pub hidden: bool,
    /// Whether the actor has ever moved.
    walked: bool,
    /// The contact callback (`$7F:1010`).
    contact: Option<usize>,
    /// An eased move `COP ED` started (`$7F:2000..200C`).
    ease: Option<ease::Ease>,
    /// The return `COP 00` kept (`$7F:0004`).
    call: Option<usize>,
    /// `+$04` bit `$0200`: the actor takes contact. Set at spawn, as the
    /// box's `$5220` is natively before any script runs; how the loader
    /// derives `+$04` is not traced.
    touchable: bool,
    /// Normalized offset of the next command.
    pc: usize,
    state: State,
    rng: u32,
    /// Source-derived COP26 timing, until a skipped service revokes it.
    cadence: Option<cadence::Cadence>,
    /// The map the actor was spawned in, which `COP 0A` compares.
    map: u16,
    /// `COP 02`'s loop start and remaining count; one level per actor.
    counted_loop: (usize, u16),
    /// Ticks of each display list in the actor's packet, which `COP 8E`
    /// holds for; `None` when the packet is not known.
    pose_ticks: Option<Vec<Option<u16>>>,
    /// The interaction callback `COP 21` registered, bank-relative.
    callback: Option<u16>,
    /// Entity `+$06`: header word, then `COP 23`/`24` switch
    /// [`INTERACT_ANY_SIDE`] as they face the player or not.
    interaction: u16,
    /// Where `RTL` resumes the actor's own script: `COP C0`/`BC`.
    continuation: Option<usize>,
    /// The actor's own script while a callback runs on it.
    outer: Option<Outer>,
    /// A scripted leg's direction, frames applied and pixels per frame
    /// (0 for the half-speed 1, 0, 1, ... stream), moving the actor through
    /// the next pose wait.
    stream: Option<(Direction, u16, u16)>,
    /// Whether legs may move this actor: its movement base is the common
    /// `$6000` resource and the nine streams are the audited ones.
    legs: bool,
    /// The repeat count `COP 85` set for the next `COP 8F`.
    repeats: Option<u16>,
    /// The cell `COP 3B` marked occupied; a scripted leg clears it
    /// (`$80:BF0E`). Nothing else does, as natively.
    stamp: Option<(u16, u16)>,
    /// Where the script stopped at something the interpreter does not model.
    frozen_at: Option<usize>,
    /// Entity `+$26`: the spawn record's fourth byte (`$80:F541`), which
    /// `COP 22` switches on.
    parameter: u8,
    /// The movement resource the streams read: the descriptor's
    /// (`$80:FAAF`) until `COP B0` picks another.
    base: Base,
    /// The descriptor the actor is built from, whose movement pointer
    /// fills its own base.
    descriptor: Option<usize>,
    /// A pose's movement streams (`COP 81`/`87`), through the next wait.
    motion: Option<motion::Motion>,
    /// The common and the own movement resource, once read.
    resources: [Option<motion::Resource>; 2],
    /// Further cells `COP 3D` marked.
    stamps: Vec<(u16, u16)>,
    /// `COP 65`'s hit target and return address, when the actor can be hit.
    hit: Option<(usize, usize)>,
    /// Frames until the actor can be hit again.
    cooldown: u16,
    /// Derived operand lengths by service, since deriving one explores a
    /// handler's control flow and the loop runs every few frames. The outer
    /// option is whether it has been derived, the inner whether it could be.
    #[allow(clippy::option_option)]
    lengths: Vec<Option<Option<usize>>>,
}

impl Actor {
    /// An actor at its record's origin, about to run its script.
    ///
    /// Without a script the actor stands on its initial selector forever.
    #[must_use]
    pub fn new(position: (u16, u16), script: Option<u32>, initial: u8, seed: u32) -> Self {
        let (pc, state) = match script {
            // The shops' spawner ends at once; the world keeps its targets.
            Some(SHOP_SPAWNER) => (0, State::Gone),
            Some(script) if (0x80..=0xBF).contains(&(script >> 16)) => {
                ((script & 0x3F_FFFF) as usize, State::Running)
            }
            _ => (0, State::Frozen),
        };
        Self {
            position,
            facing: Direction::Down,
            selector: initial,
            hflip: false,
            pose_age: 0,
            walking: false,
            hidden: false,
            walked: false,
            contact: None,
            ease: None,
            call: None,
            touchable: true,
            parameter: 0,
            base: Base::Common,
            descriptor: None,
            motion: None,
            resources: [None, None],
            pc,
            state,
            // A zero seed would stay zero.
            rng: seed | 1,
            cadence: None,
            map: 0,
            counted_loop: (0, 0),
            pose_ticks: None,
            callback: None,
            interaction: 0,
            continuation: None,
            outer: None,
            stream: None,
            legs: false,
            repeats: None,
            stamp: None,
            frozen_at: None,
            stamps: Vec::new(),
            hit: None,
            cooldown: 0,
            lengths: vec![None; 256],
        }
    }

    /// Builds a resident with source-bound timing admission, not a global
    /// slowdown inferred from shared graphics or initial selector.
    pub(crate) fn for_resident(
        image: &[u8],
        map: u16,
        resident: &crate::residents::Resident,
        seed: u32,
    ) -> Self {
        let mut actor = Self::new(resident.position, resident.script, resident.initial, seed);
        actor.map = map;
        actor.parameter = image.get(resident.record + 3).copied().unwrap_or(0);
        actor.descriptor = resident.descriptor;
        if resident.descriptor.is_some() {
            actor.base = if resident
                .descriptor
                .is_some_and(|descriptor| cadence::common_base(image, descriptor))
            {
                Base::Common
            } else {
                Base::Own
            };
        }
        // `$80:F5B0`: the header's last word is entity `+$06`.
        actor.interaction = resident
            .script
            .and_then(|script| usize::try_from(script & 0x3F_FFFF).ok())
            .and_then(|script| image.get(script.checked_sub(2)?..script))
            .map_or(0, |word| u16::from_le_bytes([word[0], word[1]]));
        actor.cadence = resident
            .descriptor
            .filter(|_| resident.body)
            .and_then(|descriptor| cadence::derive(image, descriptor).ok());
        actor.pose_ticks = resident
            .descriptor
            .filter(|_| resident.body)
            .and_then(|descriptor| cadence::pose_ticks(image, descriptor));
        actor.legs = resident.descriptor.is_some_and(|descriptor| {
            cadence::common_base(image, descriptor) && cadence::common_streams(image)
        });
        actor
    }

    /// Whether a despawn removed the actor.
    #[must_use]
    pub const fn is_gone(&self) -> bool {
        matches!(self.state, State::Gone)
    }

    /// Cell the actor occupies in the collision grid: movement samples at
    /// `(x - 8, y - 16)`.
    #[must_use]
    pub const fn collision_cell(&self) -> (u16, u16) {
        (
            self.position.0.saturating_sub(8) / 16,
            self.position.1.saturating_sub(16) / 16,
        )
    }

    /// The cell a step under way leads to, if one is.
    #[must_use]
    pub fn destination(&self) -> Option<(u16, u16)> {
        match self.state {
            State::Ordinary { destination, .. } => destination,
            State::Moving { direction, .. } => {
                let (column, row) = self.collision_cell();
                let (dx, dy) = delta(direction);
                Some((column.wrapping_add_signed(dx), row.wrapping_add_signed(dy)))
            }
            _ => None,
        }
    }

    /// Further cells `COP 3D` marked occupied.
    #[must_use]
    pub fn stamps(&self) -> &[(u16, u16)] {
        &self.stamps
    }

    /// Whether a thrown object can hit the actor now (`COP 65`, cooldown).
    #[must_use]
    pub const fn hittable(&self) -> bool {
        self.hit.is_some() && self.cooldown == 0
    }

    /// A hit (`$85:D5A0` then `$80:CA6D`): the script goes to `COP 65`'s
    /// target, and the actor cannot be hit again for sixteen frames.
    /// A script that holds the world, froze or is gone takes no hit.
    pub fn strike(&mut self) -> bool {
        if matches!(self.state, State::Blocked(_) | State::Frozen | State::Gone) {
            return false;
        }
        let Some((target, _)) = self.hit.filter(|_| self.cooldown == 0) else {
            return false;
        };
        self.cooldown = HIT_COOLDOWN;
        self.pc = target;
        self.state = State::Running;
        self.stream = None;
        self.motion = None;
        true
    }

    /// Runs one frame.
    pub fn tick(&mut self, around: &mut Surroundings<'_>) {
        let start = self.position;
        self.frame(around);
        self.walked |= self.walking || self.position != start;
    }

    /// Whether a body holds its own cell without a mark: a walker, whose
    /// steps the runtime does not mark, or one whose script froze before it
    /// could mark. A body whose script runs and marks nothing -- C's blue
    /// door -- leaves its cell alone, as natively.
    #[must_use]
    pub const fn holds_its_cell(&self) -> bool {
        self.walked || self.frozen_at.is_some()
    }

    fn frame(&mut self, around: &mut Surroundings<'_>) {
        self.cooldown = self.cooldown.saturating_sub(1);
        self.pose_age = self.pose_age.saturating_add(1);
        if matches!(self.state, State::Ordinary { ticks_left: 0, .. }) {
            self.walking = false;
            self.state = State::Running;
        }
        match self.state {
            State::Ordinary { .. } => self.tick_ordinary(),
            State::Frozen | State::Gone | State::Blocked(_) => {}
            State::Waiting(frames) => {
                self.apply_stream();
                self.state = if frames <= 1 {
                    State::Running
                } else {
                    State::Waiting(frames - 1)
                };
            }
            State::Moving {
                direction,
                remaining,
            } => {
                let pixels = (16 / STEP_FRAMES).min(remaining);
                let (dx, dy) = delta(direction);
                let travelled = i16::try_from(pixels).unwrap_or(0);
                self.position = (
                    self.position.0.wrapping_add_signed(dx * travelled),
                    self.position.1.wrapping_add_signed(dy * travelled),
                );
                let remaining = remaining - pixels;
                self.state = if remaining == 0 {
                    self.walking = false;
                    State::Running
                } else {
                    State::Moving {
                        direction,
                        remaining,
                    }
                };
            }
            State::Running => {
                self.run(around);
            }
        }
        if self.state == State::Frozen && self.frozen_at.is_none() {
            self.frozen_at = Some(self.pc);
        }
    }

    /// Where the script stopped at something the interpreter does not model,
    /// for diagnostics.
    #[must_use]
    pub const fn frozen_at(&self) -> Option<usize> {
        self.frozen_at
    }

    /// Sets the map `COP 0A`/`49` compare with.
    pub(crate) fn set_map(&mut self, map: u16) {
        self.map = map;
    }

    /// The contact callback, while armed.
    #[must_use]
    pub fn contact(&self) -> Option<usize> {
        self.contact.filter(|_| self.touchable && !self.hidden)
    }

    /// Runs the contact callback as the scheduler installs it (`$80:CAD5`),
    /// as a subroutine like an interaction callback.
    pub fn run_contact(&mut self, around: &mut Surroundings<'_>) -> Option<(usize, Wait)> {
        let pc = self.contact()?;
        self.enter_callback(pc, around)
    }

    /// Inline native code the runtime recognises: where it goes on, if the
    /// code at the script position is one.
    ///
    /// - `LDA $0004,X; ORA/AND #imm; STA $0004,X` on the bits modelled:
    ///   bit 15 hides the actor, bit 9 (`$0200`) arms its contact.
    /// - [`CONTACT`] registers the contact callback.
    /// - [`NO_DAMAGE`], [`PLAYER_POSE_TEST`], [`display_code`], and runs on
    ///   script scratch words ([`native::run`]).
    fn native_idiom(
        &mut self,
        image: &[u8],
        bank: usize,
        scratch: &mut native::Scratch,
    ) -> Option<usize> {
        // Bits 12 and 8 are accepted and not modelled: the guide clears and
        // sets 12 around the freezing's whitening (`$88:B507`, `$88:B53F`)
        // and clears 8, the dispatcher's target bit, before it leaves
        // (`$88:AF1A`).
        const MODELLED: u16 = 0x8000 | 0x1000 | 0x0200 | 0x0100;
        let at = self.pc;
        if let Some(&[0xBD, 0x04, 0x00, op, low, high, 0x9D, 0x04, 0x00]) = image.get(at..at + 9) {
            let value = u16::from_le_bytes([low, high]);
            let (set, cleared) = match op {
                0x09 if value & !MODELLED == 0 => (value, 0),
                0x29 if !value & !MODELLED == 0 => (0, !value),
                _ => return None,
            };
            self.hidden = (self.hidden || set & 0x8000 != 0) && cleared & 0x8000 == 0;
            self.touchable = (self.touchable || set & 0x0200 != 0) && cleared & 0x0200 == 0;
            return Some(at + 9);
        }
        // `+$06`'s interaction bits (`$0200` any side, `$0100` facing), as
        // the figure in `$21` sets them before registering its callback
        // (`$88:D33D`).
        if let Some(&[0xBD, 0x06, 0x00, op, low, high, 0x9D, 0x06, 0x00]) = image.get(at..at + 9) {
            const INTERACTION: u16 = INTERACT_ANY_SIDE | INTERACT_FACING;
            let value = u16::from_le_bytes([low, high]);
            match op {
                0x09 if value & !INTERACTION == 0 => self.interaction |= value,
                0x29 if !value & !INTERACTION == 0 => self.interaction &= value,
                _ => return None,
            }
            return Some(at + 9);
        }
        let code = image.get(at..at + CONTACT.len())?;
        if CONTACT
            .iter()
            .zip(code)
            .all(|(expected, byte)| expected.is_none_or(|expected| expected == *byte))
        {
            // The store alone: `$0200` comes with the entity. A zero target
            // removes the callback.
            let target = u16::from_le_bytes([code[1], code[2]]);
            self.contact = (target != 0).then_some(bank | usize::from(target));
            return Some(at + CONTACT.len());
        }
        if image.get(at..at + NO_DAMAGE.len()) == Some(&NO_DAMAGE) {
            return Some(at + NO_DAMAGE.len());
        }
        player_pose_mismatch(image, at)
            .or_else(|| display_code(image, at))
            .or_else(|| native::run(image, at, scratch))
    }

    /// The actor with its cell marked, as `COP 3B` would; for tests.
    #[cfg(test)]
    pub(crate) fn marked(mut self) -> Self {
        self.stamp = Some(self.collision_cell());
        self
    }

    /// The cell `COP 3B` marked occupied, if one is.
    #[must_use]
    pub const fn stamp(&self) -> Option<(u16, u16)> {
        self.stamp
    }

    /// The wait a blocking service left the actor's own script in, if any.
    #[must_use]
    pub fn blocked(&self) -> Option<Wait> {
        match self.state {
            State::Blocked(wait) => Some(wait),
            _ => None,
        }
    }

    /// Whether the player can interact with the actor from `facing`: the
    /// dispatcher at `$87:93B9` wants a callback and `+$06` bit `$0200`, or
    /// `$0100` with the player facing opposite to the actor.
    #[must_use]
    pub fn interactable(&self, facing: Direction) -> bool {
        self.callback.is_some()
            && !matches!(self.state, State::Frozen | State::Gone)
            && (self.interaction & INTERACT_ANY_SIDE != 0
                || (self.interaction & INTERACT_FACING != 0 && facing.opposite() == self.facing))
    }

    /// Runs the registered callback as the dispatcher does, as a subroutine
    /// on this actor with its own script held. Returns where it blocked, if it
    /// did, so the world can resume it with [`Self::resume_callback`].
    ///
    /// A callback that yields hands its position to the actor's own script,
    /// as the native `+$0A` write does.
    pub fn run_callback(&mut self, around: &mut Surroundings<'_>) -> Option<(usize, Wait)> {
        let callback = self.callback?;
        let pc = (self.pc & 0xFF_0000) | usize::from(callback);
        self.enter_callback(pc, around)
    }

    /// Continues a blocked callback at `pc` with the answer its wait needed.
    pub fn resume_callback(
        &mut self,
        pc: usize,
        wait: Wait,
        answer: u8,
        around: &mut Surroundings<'_>,
    ) -> Option<(usize, Wait)> {
        let Some(pc) = answered(around.image, pc, wait, answer) else {
            self.state = State::Frozen;
            return None;
        };
        self.enter_callback(pc, around)
    }

    fn enter_callback(
        &mut self,
        pc: usize,
        around: &mut Surroundings<'_>,
    ) -> Option<(usize, Wait)> {
        self.outer = Some(Outer {
            pc: self.pc,
            state: std::mem::replace(&mut self.state, State::Running),
        });
        self.pc = pc;
        let run = self.run(around);
        let outer = self.outer.take()?;
        match run {
            Run::Ended => {
                (self.pc, self.state) = (outer.pc, outer.state);
                None
            }
            Run::Blocked(wait) => {
                let blocked_at = self.pc;
                (self.pc, self.state) = (outer.pc, outer.state);
                Some((blocked_at, wait))
            }
            // The callback's position becomes the actor's own.
            Run::Yielded => None,
        }
    }

    /// Continues the actor's own blocked script with the answer its wait
    /// needed, in the same frame.
    pub fn resume(&mut self, answer: u8, around: &mut Surroundings<'_>) {
        let State::Blocked(wait) = self.state else {
            return;
        };
        match answered(around.image, self.pc, wait, answer) {
            Some(pc) => {
                self.pc = pc;
                self.state = State::Running;
                self.run(around);
            }
            None => self.state = State::Frozen,
        }
    }

    /// Apply this action's frame before presenting it. Streams restart with
    /// one pixel, then zero, including a final zero-displacement frame.
    fn tick_ordinary(&mut self) {
        let State::Ordinary {
            direction,
            ticks,
            ticks_left,
            destination,
        } = self.state
        else {
            return;
        };
        if let Some(direction) = direction.filter(|_| (ticks - ticks_left) % 2 == 0) {
            let (dx, dy) = delta(direction);
            self.position = (
                self.position.0.wrapping_add_signed(dx),
                self.position.1.wrapping_add_signed(dy),
            );
        }
        self.state = State::Ordinary {
            direction,
            ticks,
            ticks_left: ticks_left - 1,
            destination,
        };
    }

    fn set_pose(&mut self, selector: u8, hflip: bool) {
        if self.selector != selector || self.hflip != hflip {
            self.selector = selector;
            self.hflip = hflip;
            self.pose_age = 0;
        }
    }

    fn run(&mut self, around: &mut Surroundings<'_>) -> Run {
        let image = around.image;
        // `+$0A` at entry: where an `RTL` comes back to next frame unless
        // `COP BC`/`C0` point it elsewhere first.
        let entry = self.pc;
        self.continuation = None;
        for _ in 0..BUDGET {
            // Per step: a long jump or call may have changed it.
            let bank = self.pc & 0xFF_0000;
            let Some(window) = image.get(self.pc..self.pc + 2) else {
                self.state = State::Frozen;
                return Run::Yielded;
            };
            match window[0] {
                // COP 91 plays the pose and ends the frame through `PLA; PLA;
                // RTL` without writing `+$0A` (`$80:A395`): like an RTL, it
                // comes back to the last `+$0A` a COP 80, ED or BC wrote.
                0x02 if window[1] != ANIMATE_AND_END => {}
                // RTL: a callback returns; the actor's own script ends the
                // frame and comes back at its continuation or where it began.
                0x02 | 0x6B => {
                    if self.outer.is_some() {
                        return Run::Ended;
                    }
                    self.pc = self.continuation.take().unwrap_or(entry);
                    return Run::Yielded;
                }
                // BRA: the loop's back edge.
                0x80 => {
                    let displacement = i8::from_ne_bytes([window[1]]);
                    self.pc = (self.pc + 2).wrapping_add_signed(displacement as isize);
                    continue;
                }
                0x4C => {
                    let Some(target) = image.get(self.pc + 1..self.pc + 3) else {
                        self.state = State::Frozen;
                        return Run::Yielded;
                    };
                    let target = u16::from_le_bytes([target[0], target[1]]);
                    if target < 0x8000 {
                        self.state = State::Frozen;
                        return Run::Yielded;
                    }
                    self.pc = bank | usize::from(target);
                    continue;
                }
                _ => {
                    if let Some(next) = self.native_idiom(image, bank, &mut around.globals.scratch)
                    {
                        self.pc = next;
                        continue;
                    }
                    self.state = State::Frozen;
                    return Run::Yielded;
                }
            }
            if !self.service(window[1], self.pc + 2, bank, around) {
                return match self.state {
                    State::Blocked(wait) => Run::Blocked(wait),
                    _ => Run::Yielded,
                };
            }
        }
        // Spinning without yielding: a loop with no wait in it.
        self.state = State::Frozen;
        Run::Yielded
    }

    /// Executes one service. Returns whether execution continues this
    /// frame; a yield or a freeze ends it.
    fn service(
        &mut self,
        service: u8,
        operands: usize,
        bank: usize,
        around: &mut Surroundings<'_>,
    ) -> bool {
        let image = around.image;
        match service {
            SHOW_TEXT | SHOW_TEXT_BANKED | TEXT_WAIT | TEXT_STEP | CHOICE => {
                return self.text_service(service, operands, bank, around)
            }
            WRITE_FLAG | REGISTER_CALLBACK | LOCK_INPUT | UNLOCK_INPUT | SET_SCRIPT | LONG_JUMP
            | CONTINUATION | DELETE_ON_FLAG | DELETE | WAIT_FOR_FLAG | OCCUPY => {
                return self.script_service(service, operands, around)
            }
            EASE_START | EASE_STEP => return self.ease_service(service, operands, image),
            SWITCH | SPEED => return self.parameter_service(service, operands, bank, image),
            POSE_MOVING | REPEAT_MOVING => return self.moving_pose(service, operands, image),
            PLAY_TRACK | FADE_TO_TRACK | PLAY_SELECTION | SOUND_PORT3 | SOUND_PORT2
            | SOUND_WORD => return self.audio_service(service, operands, around),
            CALL => {
                let Some(target) = image.get(operands..operands + 3).and_then(long) else {
                    self.state = State::Frozen;
                    return false;
                };
                self.call = Some(operands + 3);
                self.pc = target;
            }
            RETURN => self.pc = self.call.take().unwrap_or(operands),
            GIVE_ITEM | GRANT_ITEM => return self.item_service(service, operands, bank, around),
            PLACE | DELETE_ON_MAP | REPEAT_POSE | COUNT | YIELD => {
                return self.stage_service(service, operands, around)
            }
            TILE_BRANCH | PATCH => return self.tile_service(service, operands, bank, around),
            HIT_TARGET | HIT_RETURN | COUNT_BRANCH | HELD_BRANCH | STAMP | UNSTAMP | SPAWN
            | SPAWN_LINKED | MUSIC_WAIT | 0x6A | 0x76 | 0xD9 => {
                return self.door_service(service, operands, bank, around)
            }
            WALK_TO_ROW | WALK_TO_COLUMN => return self.walk_toward(service, operands, image),
            SELECT_POSE => {
                let Some(selector) = image.get(operands).copied() else {
                    self.state = State::Frozen;
                    return false;
                };
                let hflip = self.hflip;
                self.set_pose(selector, hflip);
                // `$80:A18C` restarts the list even for the same pose. Only
                // where its length is known, or a one-frame fallback wait
                // would hold the raster on its first frame.
                if self.pose_list(selector).is_some() {
                    self.pose_age = 0;
                }
                // `$80:A1A8` writes `+$0A`: an RTL comes back here.
                self.continuation = Some(operands + 1);
                self.pc = operands + 1;
            }
            CLEAR_HFLIP | SET_HFLIP => {
                let selector = self.selector;
                self.set_pose(selector, service == SET_HFLIP);
                self.pc = operands;
            }
            WAIT => return self.wait_for_pose(operands),
            WAIT_STEP => return self.wait_step(operands),
            RANDOM_STEP => return self.random_step_service(operands, around),
            BRANCH_ON_PLAYER_NEAR | NEAR_BRANCH | TAKE_PLAYER | TRANSFER => {
                return self.player_service(service, operands, bank, around)
            }
            BRANCH_ON_GLOBAL => self.pc = operands + 4,
            LOOP_START => return self.loop_start(operands, image),
            LOOP_END => return self.loop_end(operands),
            BRANCH_ON_MAP => return self.branch_on_map(operands, bank, image),
            TIMED_WAIT => return self.timed_wait(operands, image),
            HOLD_WHILE_PAUSED => self.pc = operands + 1,
            STOP_FOR_PLAYER => return self.stop_for_player(operands, None, bank, around),
            FACE_PLAYER_POSED => {
                let Some(&base) = image.get(operands) else {
                    self.state = State::Frozen;
                    return false;
                };
                return self.stop_for_player(operands + 1, Some(base), bank, around);
            }
            BRANCH_ON_FLAG => return self.branch_on_flag(operands, bank, around),
            CHAINED_BRANCH | CHAINED_DESPAWN => {
                return self.branch_on_chain(service, operands, bank, around)
            }
            // Anything else is stepped over by its derived length. That
            // includes text and flag writes: the loop's ambient effects
            // are not the runtime's to apply from here.
            other => {
                if !cadence::benign_skipped_service(image, self.pc) {
                    self.cadence = None;
                }
                let length = *self.lengths[usize::from(other)]
                    .get_or_insert_with(|| actor_script::operand_length(image, other));
                let Some(length) = length else {
                    self.state = State::Frozen;
                    return false;
                };
                self.pc = operands + length;
            }
        }
        true
    }

    /// Ticks of the display list a selector names, when the packet is known.
    fn pose_list(&self, selector: u8) -> Option<u16> {
        *self.pose_ticks.as_ref()?.get(usize::from(selector))?
    }

    /// `COP 8E`. Returns whether execution continues this frame. A scripted
    /// leg moves through it, starting this frame, and stops when it ends.
    fn wait_for_pose(&mut self, operands: usize) -> bool {
        // `$80:A32F` plays the selected list once: the next command runs in
        // the frame its last record ends. Unknown: the old approximation,
        // resuming two frames later.
        if let Some(ticks) = self.pose_list(self.selector) {
            return self.hold(operands, ticks);
        }
        self.pc = operands;
        self.apply_stream();
        self.state = State::Waiting(1);
        false
    }

    /// `COP 8F` outside a qualified walk: after `COP 85`, its pose's list
    /// that many times over; otherwise the old approximation.
    fn wait_step(&mut self, operands: usize) -> bool {
        if let Some(count) = self.repeats.take() {
            if let Some(ticks) = self.pose_list(self.selector) {
                return self.hold(operands, ticks.saturating_mul(count));
            }
        }
        self.pc = operands;
        self.state = State::Waiting(WAIT_FRAMES);
        false
    }

    /// Holds for `ticks` frames counting this one, the next command running
    /// in the last; a leg moves through them.
    fn hold(&mut self, operands: usize, ticks: u16) -> bool {
        self.pc = operands;
        self.apply_stream();
        match ticks {
            0 => true,
            1 => false,
            ticks => {
                self.state = State::Waiting(ticks - 1);
                false
            }
        }
    }

    /// `COP 81 pose` and `COP 87 count pose selector`: a pose that moves
    /// by its movement streams through the next wait. Returns whether
    /// execution continues this frame.
    fn moving_pose(&mut self, service: u8, operands: usize, image: &[u8]) -> bool {
        let length = if service == POSE_MOVING { 1 } else { 3 };
        let (pose, selector) = match image.get(operands..operands + length) {
            Some(&[pose]) => (pose, pose),
            Some(&[count, pose, selector]) => {
                self.repeats = Some(u16::from(count));
                (pose, selector)
            }
            _ => {
                self.state = State::Frozen;
                return false;
            }
        };
        let hflip = self.hflip;
        self.set_pose(pose, hflip);
        self.pose_age = 0;
        self.stream = None;
        let list = self.pose_list(pose);
        self.motion = self.movement(image).and_then(|resource| {
            motion::Motion::start(
                resource,
                selector,
                hflip,
                self.interaction & 0x80 != 0,
                list,
            )
        });
        // As `COP 80`, the handlers write `+$0A` (`$80:A1CB`, `$80:A1F7`).
        self.continuation = Some(operands + length);
        self.pc = operands + length;
        true
    }

    /// The movement resource at the actor's base, read once: the common
    /// one, or its descriptor's own (bytes 5..8, `$D2:7FB1` for the town's
    /// walkers).
    fn movement(&mut self, image: &[u8]) -> Option<motion::Resource> {
        let slot = match self.base {
            Base::Common => 0,
            Base::Own => 1,
            Base::Unknown => return None,
        };
        if self.resources[slot].is_none() {
            let (source, size) = if slot == 0 {
                (COMMON_SOURCE, COMMON_SIZE)
            } else {
                let pointer = image.get(self.descriptor? + 5..self.descriptor? + 8)?;
                (assets::maps::actors::rom_offset(pointer)?, 0x2000)
            };
            let packet = assets::compression::decode(image.get(source..)?, size).ok()?;
            self.resources[slot] = Some(motion::Resource {
                base: if slot == 0 { 0x6000 } else { 0x4000 },
                bytes: packet.data.into(),
            });
        }
        self.resources[slot].clone()
    }

    /// One frame of a scripted leg or a pose's movement; cleared once the
    /// pose wait is over.
    fn apply_stream(&mut self) {
        let (dx, dy) = if let Some(motion) = &mut self.motion {
            motion.step().unwrap_or((0, 0))
        } else if let Some((direction, applied, speed)) = self.stream {
            let pixels = if speed == 0 {
                i16::from(applied % 2 == 0)
            } else {
                speed.cast_signed()
            };
            self.stream = Some((direction, applied + 1, speed));
            let (dx, dy) = delta(direction);
            (dx * pixels, dy * pixels)
        } else {
            return;
        };
        self.position = (
            self.position.0.wrapping_add_signed(dx),
            self.position.1.wrapping_add_signed(dy),
        );
        self.walking = self.stream.is_some() || (dx, dy) != (0, 0);
        let ends = match self.state {
            State::Waiting(frames) => frames <= 1,
            _ => self
                .pose_list(self.selector)
                .is_some_and(|ticks| ticks <= 1),
        };
        if ends {
            self.stream = None;
            self.motion = None;
            self.walking = false;
        }
    }

    /// `COP 3A`/`39`: a scripted leg toward a row or column. Returns whether
    /// execution continues this frame.
    fn walk_toward(&mut self, service: u8, operands: usize, image: &[u8]) -> bool {
        let Some(&[pose, vector, target]) = image.get(operands..operands + 3) else {
            self.state = State::Frozen;
            return false;
        };
        let target = i32::from(i8::from_ne_bytes([target])) * 16;
        let (position, target, vertical) = if service == WALK_TO_ROW {
            (i32::from(self.position.1), target, true)
        } else {
            (i32::from(self.position.0), target + 8, false)
        };
        if position == target {
            // Past this leg's `COP 8E; COP 3B; BRA`.
            self.pc = operands + 3 + 6 + if pose & 0x80 != 0 { 4 } else { 0 };
            return true;
        }
        let forward = target > position;
        let (direction, pose, vector) = match (vertical, forward) {
            (true, true) => (Direction::Down, pose, vector),
            (true, false) => (Direction::Up, pose.wrapping_add(1), vector.wrapping_add(1)),
            (false, true) => (Direction::Right, pose, vector),
            (false, false) => (Direction::Left, pose, vector),
        };
        // Only the audited common streams: across $60/$70/$80, down
        // $68/$78/$88, up $69/$79/$89, at half, one and two pixels a frame.
        let axis = match direction {
            Direction::Down => 0x08,
            Direction::Up => 0x09,
            Direction::Left | Direction::Right => 0x00,
        };
        let speed = match vector.wrapping_sub(axis) {
            0x60 => 0,
            0x70 => 1,
            0x80 => 2,
            _ => u16::MAX,
        };
        if !self.legs || speed == u16::MAX {
            self.state = State::Frozen;
            return false;
        }
        self.facing = direction;
        self.set_pose(pose & 0x7F, direction == Direction::Left);
        self.pose_age = 0;
        self.stream = Some((direction, 0, speed));
        self.motion = None;
        self.stamp = None;
        self.pc = operands + 3;
        true
    }

    /// Hits, counter branches, further stamps, spawns and the fade wait.
    /// Returns whether execution continues this frame.
    fn door_service(
        &mut self,
        service: u8,
        operands: usize,
        bank: usize,
        around: &mut Surroundings<'_>,
    ) -> bool {
        let image = around.image;
        match service {
            HIT_TARGET => {
                let (Some(target), Some(resume)) = (
                    cadence::word(image, operands),
                    image.get(operands + 2..operands + 5).and_then(long),
                ) else {
                    self.state = State::Frozen;
                    return false;
                };
                self.hit = Some((bank | usize::from(target), resume));
                self.pc = operands + 5;
            }
            HIT_RETURN => {
                let Some((_, resume)) = self.hit else {
                    self.state = State::Frozen;
                    return false;
                };
                self.pc = resume;
            }
            HELD_BRANCH => {
                let (Some(mask), Some(target)) = (
                    cadence::word(image, operands),
                    cadence::word(image, operands + 2),
                ) else {
                    self.state = State::Frozen;
                    return false;
                };
                if around.globals.pad & mask == 0 {
                    return self.jump(bank, target);
                }
                self.pc = operands + 4;
            }
            COUNT_BRANCH => {
                let (Some(&counter), Some(word), Some(target)) = (
                    image.get(operands),
                    cadence::word(image, operands + 1),
                    cadence::word(image, operands + 3),
                ) else {
                    self.state = State::Frozen;
                    return false;
                };
                // `$80:9713`: bit 7 branches on greater, bit 6 on less, and
                // equal only without either.
                let branches = match around.globals.counter(counter).cmp(&word) {
                    std::cmp::Ordering::Equal => counter & 0xC0 == 0,
                    std::cmp::Ordering::Greater => counter & 0x80 != 0,
                    std::cmp::Ordering::Less => counter & 0x40 != 0,
                };
                if branches {
                    return self.jump(bank, target);
                }
                self.pc = operands + 5;
            }
            STAMP | UNSTAMP => {
                let Some(&[0, dx, dy]) = image.get(operands..operands + 3) else {
                    self.state = State::Frozen;
                    return false;
                };
                let (column, row) = self.collision_cell();
                let offset = |base: u16, by: u8| {
                    base.wrapping_add_signed(i16::from(i8::from_ne_bytes([by])))
                };
                let cell = (offset(column, dx), offset(row, dy));
                self.stamps.retain(|&stamped| stamped != cell);
                if service == STAMP {
                    self.stamps.push(cell);
                }
                self.pc = operands + 3;
            }
            SPAWN | SPAWN_LINKED => {
                let (Some(script), Some(flags)) = (
                    image.get(operands..operands + 3).and_then(long),
                    cadence::word(image, operands + 3),
                ) else {
                    self.state = State::Frozen;
                    return false;
                };
                around.globals.spawns.push((script, flags, self.position));
                self.pc = operands + 5;
            }
            MUSIC_WAIT => return self.hold(operands, 3),
            cosmetic => {
                let Some(&(_, length)) = COSMETIC.iter().find(|&&(service, _)| service == cosmetic)
                else {
                    self.state = State::Frozen;
                    return false;
                };
                self.pc = operands + length;
            }
        }
        true
    }

    /// `COP 42` and `COP 44`, on cells offset from the actor's own. Returns
    /// whether execution continues this frame.
    fn tile_service(
        &mut self,
        service: u8,
        operands: usize,
        bank: usize,
        around: &mut Surroundings<'_>,
    ) -> bool {
        let Some(&[dx, dy, low, high]) = around.image.get(operands..operands + 4) else {
            self.state = State::Frozen;
            return false;
        };
        let (column, row) = self.collision_cell();
        let offset =
            |base: u16, by: u8| base.wrapping_add_signed(i16::from(i8::from_ne_bytes([by])));
        let (column, row) = (offset(column, dx), offset(row, dy));
        let word = u16::from_le_bytes([low, high]);
        if service == PATCH {
            around.globals.patches.push((column, row, word & 0x1FF));
            return self.hold(operands + 4, u16::from(high >> 2));
        }
        let Some(target) = cadence::word(around.image, operands + 4) else {
            self.state = State::Frozen;
            return false;
        };
        let at = usize::from(row) * usize::from(around.width) + usize::from(column);
        let holds = column < around.width
            && around
                .cells
                .get(at)
                .is_some_and(|cell| cell & 0x1FF == word & 0x1FF);
        if holds {
            return self.jump(bank, target);
        }
        self.pc = operands + 6;
        true
    }

    /// Placement, map deletion, repeated poses, counters and a bare yield.
    /// Returns whether execution continues this frame.
    fn stage_service(
        &mut self,
        service: u8,
        operands: usize,
        around: &mut Surroundings<'_>,
    ) -> bool {
        let image = around.image;
        match service {
            PLACE => {
                let Some(&[column, row, facing]) = image.get(operands..operands + 3) else {
                    self.state = State::Frozen;
                    return false;
                };
                let tile = |byte: u8| i32::from(i8::from_ne_bytes([byte])) * 16;
                let (Ok(x), Ok(y)) = (u16::try_from(tile(column) + 8), u16::try_from(tile(row)))
                else {
                    self.state = State::Frozen;
                    return false;
                };
                self.position = (x, y);
                self.facing = match facing & 3 {
                    0 => Direction::Down,
                    1 => Direction::Up,
                    2 => Direction::Left,
                    _ => Direction::Right,
                };
                // The whole byte goes to `+$14`; only exactly 2 mirrors.
                self.hflip = facing == 2;
                self.stream = None;
                self.motion = None;
                self.stamp = None;
                self.pc = operands + 3;
            }
            DELETE_ON_MAP => {
                let Some(word) = cadence::word(image, operands) else {
                    self.state = State::Frozen;
                    return false;
                };
                if (word & 0x7FFF == self.map) != (word & 0x8000 != 0) {
                    self.state = State::Gone;
                    return false;
                }
                self.pc = operands + 2;
            }
            REPEAT_POSE => {
                let Some(&[count, pose]) = image.get(operands..operands + 2) else {
                    self.state = State::Frozen;
                    return false;
                };
                let hflip = self.hflip;
                self.set_pose(pose, hflip);
                self.pose_age = 0;
                self.stream = None;
                self.motion = None;
                self.repeats = Some(u16::from(count));
                self.pc = operands + 2;
            }
            COUNT => {
                let (Some(&op), Some(word)) =
                    (image.get(operands), cadence::word(image, operands + 1))
                else {
                    self.state = State::Frozen;
                    return false;
                };
                if !around.globals.count(op, word) {
                    self.state = State::Frozen;
                    return false;
                }
                self.pc = operands + 3;
            }
            YIELD => {
                self.pc = operands;
                return false;
            }
            _ => {
                self.state = State::Frozen;
                return false;
            }
        }
        true
    }

    /// Flag writes, callback registration, input locks, jumps and deletion.
    /// Returns whether execution continues this frame.
    fn script_service(
        &mut self,
        service: u8,
        operands: usize,
        around: &mut Surroundings<'_>,
    ) -> bool {
        let image = around.image;
        match service {
            WRITE_FLAG => {
                let Some(word) = cadence::word(image, operands) else {
                    self.state = State::Frozen;
                    return false;
                };
                around.globals.write_flag(word);
                self.pc = operands + 2;
            }
            REGISTER_CALLBACK => {
                let Some(word) = cadence::word(image, operands) else {
                    self.state = State::Frozen;
                    return false;
                };
                self.callback = (word != 0).then_some(word);
                self.pc = operands + 2;
            }
            LOCK_INPUT | UNLOCK_INPUT => {
                let Some(mask) = cadence::word(image, operands) else {
                    self.state = State::Frozen;
                    return false;
                };
                if service == LOCK_INPUT {
                    around.globals.input_mask |= mask;
                } else {
                    around.globals.input_mask &= !mask;
                }
                self.pc = operands + 2;
            }
            SET_SCRIPT | LONG_JUMP => {
                let Some(target) = image.get(operands..operands + 3).and_then(long) else {
                    self.state = State::Frozen;
                    return false;
                };
                if service == LONG_JUMP {
                    self.pc = target;
                } else if let Some(outer) = &mut self.outer {
                    // From a callback: the actor's own script, which runs
                    // from there with its countdown cleared.
                    *outer = Outer {
                        pc: target,
                        state: State::Running,
                    };
                    self.pc = operands + 3;
                } else {
                    self.continuation = Some(target);
                    self.pc = operands + 3;
                }
            }
            CONTINUATION => {
                self.continuation = Some(operands);
                self.pc = operands;
            }
            DELETE => {
                self.state = State::Gone;
                return false;
            }
            OCCUPY => {
                self.stamp = Some(self.collision_cell());
                self.pc = operands;
            }
            WAIT_FOR_FLAG => {
                let Some(word) = cadence::word(image, operands) else {
                    self.state = State::Frozen;
                    return false;
                };
                let set = EventFlags::Bitmap(&around.globals.events).get(word & 0x0FFF);
                if set != Some(word & 0x8000 == 0) {
                    return false;
                }
                self.pc = operands + 2;
            }
            DELETE_ON_FLAG => {
                let Some(word) = cadence::word(image, operands) else {
                    self.state = State::Frozen;
                    return false;
                };
                let set = EventFlags::Bitmap(&around.globals.events).get(word & 0x0FFF);
                if set == Some(word & 0x8000 != 0) {
                    self.state = State::Gone;
                    return false;
                }
                self.pc = operands + 2;
            }
            _ => {
                self.state = State::Frozen;
                return false;
            }
        }
        true
    }

    /// `COP 1B`, `1F`, `20` and `1A`. Returns whether execution continues
    /// this frame. A service that finds the window busy retries next frame.
    fn text_service(
        &mut self,
        service: u8,
        operands: usize,
        bank: usize,
        around: &mut Surroundings<'_>,
    ) -> bool {
        let image = around.image;
        let dialogue = &mut around.globals.dialogue;
        match service {
            SHOW_TEXT | SHOW_TEXT_BANKED => {
                let banked = service == SHOW_TEXT_BANKED;
                let (bank, at) = if banked {
                    let Some(&bank) = image.get(operands) else {
                        self.state = State::Frozen;
                        return false;
                    };
                    (u32::from(bank) << 16, operands + 1)
                } else {
                    (
                        u32::try_from(bank).map_or(0, |bank| 0x80_0000 | bank),
                        operands,
                    )
                };
                let Some(pointer) = cadence::word(image, at) else {
                    self.state = State::Frozen;
                    return false;
                };
                if dialogue.busy() {
                    return false;
                }
                let source = bank | u32::from(pointer);
                let Ok(pages) = HouseDialogue::decode_at(image, source) else {
                    self.state = State::Frozen;
                    return false;
                };
                dialogue.request(pages);
                self.pc = at + 2;
                true
            }
            TEXT_WAIT => {
                self.pc = operands;
                if dialogue.busy() {
                    self.state = State::Blocked(Wait::Text);
                    return false;
                }
                true
            }
            TEXT_STEP => {
                if dialogue.busy() {
                    return false;
                }
                self.pc = operands;
                true
            }
            _ => {
                let (Some(&catalog), Some(table)) =
                    (image.get(operands), cadence::word(image, operands + 1))
                else {
                    self.state = State::Frozen;
                    return false;
                };
                if dialogue.busy() {
                    return false;
                }
                let Ok(choice) = HouseDialogue::choice_at(image, catalog) else {
                    self.state = State::Frozen;
                    return false;
                };
                dialogue.ask(choice);
                self.pc = operands + 3;
                self.state = State::Blocked(Wait::Choice(bank | usize::from(table)));
                false
            }
        }
    }

    /// `COP 02`. Returns whether execution continues this frame.
    fn loop_start(&mut self, operands: usize, image: &[u8]) -> bool {
        let Some(count) = cadence::word(image, operands) else {
            self.state = State::Frozen;
            return false;
        };
        self.pc = operands + 2;
        self.counted_loop = (self.pc, count);
        true
    }

    /// `COP 03`. Without a `COP 02` the count wraps and the start is zero,
    /// which is no command, so the actor freezes there.
    fn loop_end(&mut self, operands: usize) -> bool {
        let (start, count) = self.counted_loop;
        let count = count.wrapping_sub(1);
        self.counted_loop.1 = count;
        if count == 0 {
            self.pc = operands;
            return true;
        }
        self.pc = start;
        false
    }

    /// `COP 0A`. Returns whether execution continues this frame.
    fn branch_on_map(&mut self, operands: usize, bank: usize, image: &[u8]) -> bool {
        let (Some(word), Some(target)) = (
            cadence::word(image, operands),
            cadence::word(image, operands + 2),
        ) else {
            self.state = State::Frozen;
            return false;
        };
        if (word & 0x7FFF == self.map) != (word & 0x8000 != 0) {
            return self.jump(bank, target);
        }
        self.pc = operands + 4;
        true
    }

    /// `COP C1`: always yields.
    fn timed_wait(&mut self, operands: usize, image: &[u8]) -> bool {
        let Some(frames) = cadence::word(image, operands) else {
            self.state = State::Frozen;
            return false;
        };
        self.pc = operands + 2;
        if frames != 0 {
            self.state = State::Waiting(frames);
        }
        false
    }

    /// Resumes at a bank-relative target, or freezes on one below `$8000`,
    /// which is RAM and not a command address.
    fn jump(&mut self, bank: usize, target: u16) -> bool {
        if target < 0x8000 {
            self.state = State::Frozen;
            return false;
        }
        self.pc = bank | usize::from(target);
        true
    }

    /// `COP 23`: stops for a player who stands beside the actor and faces
    /// them. Returns whether execution continues this frame.
    ///
    /// `COP 24` names its own pose base, bit 7 keeping the mirror. Both
    /// switch [`INTERACT_ANY_SIDE`] on when the player faces the actor and
    /// off otherwise, which is what lets the dispatcher run the callback.
    fn stop_for_player(
        &mut self,
        operands: usize,
        base: Option<u8>,
        bank: usize,
        around: &Surroundings<'_>,
    ) -> bool {
        let Some(bytes) = around.image.get(operands..operands + 2) else {
            self.state = State::Frozen;
            return false;
        };
        let faced = approach(self.position, around.player)
            .into_iter()
            .flatten()
            .any(|toward| toward == around.facing);
        if !faced {
            self.interaction &= !INTERACT_ANY_SIDE;
            self.pc = operands + 2;
            return true;
        }
        // `$8DC2`: the actor's facing is the player's, reversed.
        let facing = around.facing.opposite();
        self.facing = facing;
        let (offset, flip) = standing_pose(facing);
        let (selector, hflip) = match base {
            None => (offset, flip),
            Some(base) if base & 0x80 != 0 => ((base & 0x7F) + offset, self.hflip),
            Some(base) => (base + offset, flip),
        };
        self.set_pose(selector, hflip);
        self.interaction |= INTERACT_ANY_SIDE;
        let target = u16::from_le_bytes([bytes[0], bytes[1]]);
        // A taken branch yields whether or not the target was sound.
        self.jump(bank, target);
        false
    }

    /// `COP 0F`: branches on the player standing at a map position. Returns
    /// whether execution continues this frame.
    fn branch_near_player(
        &mut self,
        operands: usize,
        bank: usize,
        around: &Surroundings<'_>,
    ) -> bool {
        let Some(bytes) = around.image.get(operands..operands + 5) else {
            self.state = State::Frozen;
            return false;
        };
        let id = bytes[0];
        // `$88A3`: `SEC; SBC $0966; CMP #$11; BCS` -- an unsigned, one-sided
        // test: the position minus the player's must be 0 to 16. `$0968`
        // holds the player's Y less eight.
        // `$0956`'s codes are the facing's: 0 down, 1 up, 2 left, 3 right.
        let facing = around.facing as u8;
        let near = (id & 0x7F == 0x7F || id & 0x7F == facing) && {
            let tile = |byte: u8| i32::from(i8::from_ne_bytes([byte])) * 16;
            let (px, py) = (i32::from(around.player.0), i32::from(around.player.1) - 8);
            (0..=16).contains(&(tile(bytes[1]) - px)) && (0..=16).contains(&(tile(bytes[2]) - py))
        };
        let target = u16::from_le_bytes([bytes[3], bytes[4]]);
        if near != (id & 0x80 != 0) {
            return self.jump(bank, target);
        }
        self.pc = operands + 5;
        true
    }

    /// `COP 26` with its following `COP 8F`: one step inside a rectangle,
    /// at the derived cadence when there is one. Returns whether execution
    /// continues this frame.
    fn random_step_service(&mut self, operands: usize, around: &mut Surroundings<'_>) -> bool {
        let image = around.image;
        let Some(rect) = image.get(operands..operands + 4) else {
            self.state = State::Frozen;
            return false;
        };
        let qualified = self
            .cadence
            .filter(|_| image.get(operands + 4..operands + 6) == Some(&[2, WAIT_STEP]));
        self.pc = operands + 4;
        let moving = self.random_step([rect[0], rect[1], rect[2], rect[3]], around);
        if let Some(cadence) = qualified {
            // COP8F resolves this very action; it is not an extra wait.
            self.pc += 2;
            self.pose_age = 0;
            self.walking = moving;
            let ticks = if moving {
                cadence.walk(self.facing)
            } else {
                cadence.idle
            };
            self.state = State::Ordinary {
                direction: moving.then_some(self.facing),
                ticks,
                ticks_left: ticks,
                destination: self.destination(),
            };
            self.tick_ordinary();
            return false;
        }
        !moving
    }

    /// `COP 54` and `COP 60`: items the player receives. Returns whether
    /// execution continues this frame.
    fn item_service(
        &mut self,
        service: u8,
        operands: usize,
        bank: usize,
        around: &mut Surroundings<'_>,
    ) -> bool {
        let image = around.image;
        match service {
            GIVE_ITEM => {
                let (Some(&item), Some(full)) =
                    (image.get(operands), cadence::word(image, operands + 1))
                else {
                    self.state = State::Frozen;
                    return false;
                };
                if !around.globals.inventory.add(item) {
                    return self.jump(bank, full);
                }
                self.pc = operands + 3;
            }
            GRANT_ITEM => {
                // The word is the presentation's length in frames, the
                // fourth operand its fanfare (`$80:9A5B`).
                let (Some(&item), Some(frames), Some(&track)) = (
                    image.get(operands),
                    cadence::word(image, operands + 1),
                    image.get(operands + 3),
                ) else {
                    self.state = State::Frozen;
                    return false;
                };
                // A full inventory keeps nothing; the presentation goes on.
                // The player's presentation pose is not drawn.
                around.globals.inventory.add(item);
                around.globals.audio.fanfare(track, frames);
                self.pc = operands + 4;
            }
            _ => {
                self.state = State::Frozen;
                return false;
            }
        }
        true
    }

    /// Music and sound effect services ([`crate::audio`]). Returns whether
    /// execution continues this frame.
    fn audio_service(
        &mut self,
        service: u8,
        operands: usize,
        around: &mut Surroundings<'_>,
    ) -> bool {
        let length = if service == SOUND_WORD { 2 } else { 1 };
        let Some(bytes) = around.image.get(operands..operands + length) else {
            self.state = State::Frozen;
            return false;
        };
        let audio = &mut around.globals.audio;
        match service {
            PLAY_TRACK => audio.play(bytes[0], false),
            FADE_TO_TRACK => audio.play(bytes[0], true),
            PLAY_SELECTION => audio.select(bytes[0]),
            SOUND_PORT3 => audio.sound_port3(bytes[0]),
            SOUND_PORT2 => audio.sound_port2(bytes[0]),
            _ => audio.sound_word(u16::from_le_bytes([bytes[0], bytes[1]])),
        }
        self.pc = operands + length;
        true
    }

    /// `COP 22`'s switch on the spawn parameter and `COP B0`'s speed.
    /// Returns whether execution continues this frame.
    fn parameter_service(
        &mut self,
        service: u8,
        operands: usize,
        bank: usize,
        image: &[u8],
    ) -> bool {
        let fetched = if service == SWITCH {
            self.switch_target(operands, bank, image)
        } else {
            image.get(operands).map(|&base| {
                self.base = match base {
                    2 => Base::Common,
                    0 => Base::Own,
                    _ => Base::Unknown,
                };
                operands + if base == 0xFF { 4 } else { 1 }
            })
        };
        let Some(next) = fetched else {
            self.state = State::Frozen;
            return false;
        };
        self.pc = next;
        true
    }

    /// Where `COP 22` goes: the table's target for the parameter, or past
    /// the table.
    fn switch_target(&self, operands: usize, bank: usize, image: &[u8]) -> Option<usize> {
        let (&low, &high) = (image.get(operands)?, image.get(operands + 1)?);
        let table = operands + 2;
        if (low..=high).contains(&self.parameter) {
            let target = cadence::word(image, table + usize::from(self.parameter - low) * 2)?;
            Some(bank | usize::from(target))
        } else {
            Some(table + (usize::from(high.saturating_sub(low)) + 1) * 2)
        }
    }

    /// `COP ED` and `COP EE`: an eased move ([`ease::Ease`]). Returns
    /// whether execution continues this frame.
    fn ease_service(&mut self, service: u8, operands: usize, image: &[u8]) -> bool {
        match service {
            EASE_START => {
                let (Some(&pose), Some(dx), Some(dy)) = (
                    image.get(operands),
                    cadence::word(image, operands + 1),
                    cadence::word(image, operands + 3),
                ) else {
                    self.state = State::Frozen;
                    return false;
                };
                let hflip = self.hflip;
                self.set_pose(pose, hflip);
                // `$80:9F8A` writes `+$0A`, as COP 80 and BC do.
                self.continuation = Some(operands + 5);
                self.ease = Some(ease::Ease::new(
                    self.position,
                    (dx.cast_signed(), dy.cast_signed()),
                ));
                self.pc = operands + 5;
            }
            EASE_STEP => {
                let (Some(&speed), Some(mut ease)) = (image.get(operands), self.ease) else {
                    self.state = State::Frozen;
                    return false;
                };
                let Some((position, arrived)) = ease.step(image, speed) else {
                    self.state = State::Frozen;
                    return false;
                };
                self.position = position;
                self.walking = !arrived;
                self.ease = (!arrived).then_some(ease);
                if !arrived {
                    // `$80:9FD1`: the same command again next frame.
                    return false;
                }
                self.pc = operands + 1;
            }
            _ => {
                self.state = State::Frozen;
                return false;
            }
        }
        true
    }

    /// `COP 0F`, `0D`, `DF` and `14`: the player's position, script and map.
    /// Returns whether execution continues this frame.
    fn player_service(
        &mut self,
        service: u8,
        operands: usize,
        bank: usize,
        around: &mut Surroundings<'_>,
    ) -> bool {
        let image = around.image;
        match service {
            BRANCH_ON_PLAYER_NEAR => return self.branch_near_player(operands, bank, around),
            NEAR_BRANCH => return self.branch_in_cells(operands, bank, around),
            TAKE_PLAYER => {
                if around.globals.player_action {
                    // `$80:B87B` retries the COP next frame.
                    return false;
                }
                // The player's standing script it installs is the host's
                // standing player; the pad stays masked as the script set it.
                self.pc = operands + 3;
            }
            TRANSFER => {
                let (Some(map), Some(&mode), Some(x), Some(y)) = (
                    cadence::word(image, operands),
                    image.get(operands + 2),
                    cadence::word(image, operands + 4),
                    cadence::word(image, operands + 6),
                ) else {
                    self.state = State::Frozen;
                    return false;
                };
                // The loader places the player at the queued position plus
                // (8,16); the mode picks the fades, the selector only the
                // player's arrival script.
                around.globals.transfer = Some(Transfer {
                    map,
                    position: (x + 8, y + 16),
                    mode,
                });
                self.pc = operands + 8;
            }
            _ => {
                self.state = State::Frozen;
                return false;
            }
        }
        true
    }

    /// `COP 0D`: branches on the player inside a rectangle of cells around
    /// the actor, as `COP 0F` does on a point: a jump when inside differs from
    /// bit 7 of the facing byte. The corners are the actor's position plus
    /// signed cells (`$80:BC2F`), less eight on Y, against `$0966`/`$0968`,
    /// inclusive; negative near corners clamp to 0. Returns whether execution
    /// continues this frame.
    fn branch_in_cells(&mut self, operands: usize, bank: usize, around: &Surroundings<'_>) -> bool {
        let Some(bytes) = around.image.get(operands..operands + 7) else {
            self.state = State::Frozen;
            return false;
        };
        let id = bytes[0];
        let corner = |base: u16, byte: u8, less: i32| {
            i32::from(base) + i32::from(i8::from_ne_bytes([byte])) * 16 - less
        };
        let (x, y) = self.position;
        // Only the near corners clamp (`BPL` at `$80:87DF` / `87F6`).
        let (left, top) = (corner(x, bytes[1], 0).max(0), corner(y, bytes[2], 8).max(0));
        let (right, bottom) = (corner(x, bytes[3], 0), corner(y, bytes[4], 8));
        let (px, py) = (i32::from(around.player.0), i32::from(around.player.1) - 8);
        let inside = (id & 0x7F == 0x7F || id & 0x7F == around.facing as u8)
            && (left..=right).contains(&px)
            && (top..=bottom).contains(&py);
        if inside != (id & 0x80 != 0) {
            return self.jump(bank, u16::from_le_bytes([bytes[5], bytes[6]]));
        }
        self.pc = operands + 7;
        true
    }

    /// `COP 08`: branches on one event flag. Returns whether execution
    /// continues this frame.
    fn branch_on_flag(&mut self, operands: usize, bank: usize, around: &Surroundings<'_>) -> bool {
        let Some(bytes) = around.image.get(operands..operands + 4) else {
            self.state = State::Frozen;
            return false;
        };
        let condition = u16::from_le_bytes([bytes[0], bytes[1]]);
        let target = u16::from_le_bytes([bytes[2], bytes[3]]);
        let Some(set) = EventFlags::Bitmap(&around.globals.events).get(condition) else {
            self.state = State::Frozen;
            return false;
        };
        if flag_branch_taken(condition, set) {
            return self.jump(bank, target);
        }
        self.pc = operands + 4;
        true
    }

    /// A chained condition: despawns, branches or falls through. Returns
    /// whether execution continues this frame.
    fn branch_on_chain(
        &mut self,
        service: u8,
        operands: usize,
        bank: usize,
        around: &Surroundings<'_>,
    ) -> bool {
        let image = around.image;
        let flags = EventFlags::Bitmap(&around.globals.events);
        let (Some(chain), Some(holds)) = (
            chained_condition_length(image, operands),
            chained_condition_holds(image, operands, &flags),
        ) else {
            self.state = State::Frozen;
            return false;
        };
        if service == CHAINED_DESPAWN {
            if holds {
                self.state = State::Gone;
                return false;
            }
            self.pc = operands + chain;
            return true;
        }
        let Some(bytes) = image.get(operands + chain..operands + chain + 2) else {
            self.state = State::Frozen;
            return false;
        };
        let target = u16::from_le_bytes([bytes[0], bytes[1]]);
        if holds {
            return self.jump(bank, target);
        }
        self.pc = operands + chain + 2;
        true
    }

    /// One `COP 26`: draws, bounds-checks, probes, and starts a step or
    /// stands. Returns whether the frame's execution ends here.
    fn random_step(&mut self, rect: [u8; 4], around: &Surroundings<'_>) -> bool {
        let draw = self.next_random();
        let (column, row) = self.collision_cell();
        if draw & 4 != 0 {
            let (selector, hflip) = standing_pose(self.facing);
            self.set_pose(selector, hflip);
            return false;
        }
        let direction = match draw & 3 {
            0 => Direction::Down,
            1 => Direction::Up,
            2 => Direction::Left,
            _ => Direction::Right,
        };
        let [min_column, max_column, min_row, max_row] = rect.map(u16::from);
        // As the handler compares: `$8EA5` stands when the maximum is below
        // the current column, `$8EBE` when the minimum is not below it. The
        // far side is therefore one cell wider than the operands read.
        let inside = match direction {
            Direction::Down => row <= max_row,
            Direction::Up => row > min_row,
            Direction::Left => column > min_column,
            Direction::Right => column <= max_column,
        };
        let (dx, dy) = delta(direction);
        let destination = (column.wrapping_add_signed(dx), row.wrapping_add_signed(dy));
        let passable = destination.0 < around.width
            && destination.1 < around.height
            && around
                .cells
                .get(
                    usize::from(destination.1) * usize::from(around.width)
                        + usize::from(destination.0),
                )
                .is_some_and(|cell| PASSABLE.contains(&(cell >> 9)))
            && !around.occupied.contains(&destination);
        if inside && passable {
            self.facing = direction;
            let (selector, hflip) = match direction {
                Direction::Down => (3, false),
                Direction::Up => (4, false),
                Direction::Right => (5, false),
                Direction::Left => (5, true),
            };
            self.set_pose(selector, hflip);
            self.walking = true;
            self.state = State::Moving {
                direction,
                remaining: 16,
            };
            true
        } else {
            let (selector, hflip) = standing_pose(self.facing);
            self.set_pose(selector, hflip);
            false
        }
    }

    /// The runtime's own generator, not `$86:8236`. An xorshift, seeded per
    /// actor, so a run is repeatable.
    fn next_random(&mut self) -> u8 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 24) as u8
    }
}

/// The standing sequence for a facing: 0, 1 and 2, with 2 mirrored for
/// left, as `$80:8DC8` selects it.
const fn standing_pose(facing: Direction) -> (u8, bool) {
    match facing {
        Direction::Down => (0, false),
        Direction::Up => (1, false),
        Direction::Right => (2, false),
        Direction::Left => (2, true),
    }
}

/// Where a blocked script goes on: after a text wait, where it stood; after
/// a choice, through its table's entry for the answer (0 cancel, 1, 2).
fn answered(image: &[u8], pc: usize, wait: Wait, answer: u8) -> Option<usize> {
    match wait {
        Wait::Text => Some(pc),
        Wait::Choice(table) => {
            let target = cadence::word(image, table + 2 * usize::from(answer))?;
            (target >= 0x8000).then_some((table & 0xFF_0000) | usize::from(target))
        }
    }
}

/// The end of a run of inline native code that only touches the display:
/// the PPU's registers (`$2100..$21FF`), their shadows (`$0468..$046B`), the
/// actor's scratch word (`$7F:201C,X`) and the cosmetic helper's flag
/// (`$7E:46E6`), through immediates, `SEP`/`REP` and `INC`/`DEC A`. Gameplay
/// cannot see these writes, so the script goes on at the next `COP`.
/// Where [`PLAYER_POSE_TEST`] at `at` goes for a player whose animation
/// does not match: past the `PLX` its first branch reaches. The runtime's Ark
/// plays only standing and walking, never the tables these tests ask for
/// (the blue door's push test wants table 1, sequence 4).
fn player_pose_mismatch(image: &[u8], at: usize) -> Option<usize> {
    let code = image.get(at..at + PLAYER_POSE_TEST.len())?;
    if !PLAYER_POSE_TEST
        .iter()
        .zip(code)
        .all(|(expected, byte)| expected.is_none_or(|expected| expected == *byte))
    {
        return None;
    }
    let branch = |offset: usize| {
        (at + offset + 2).wrapping_add_signed(isize::from(i8::from_ne_bytes([code[offset + 1]])))
    };
    let target = branch(11);
    (image.get(target) == Some(&0xFA) && branch(20) == target).then_some(target + 1)
}

fn display_code(image: &[u8], mut at: usize) -> Option<usize> {
    let start = at;
    let mut widths = assets::cpu::Widths::native();
    while *image.get(at)? != 0x02 {
        let bytes = image.get(at..at + 4)?;
        let absolute = u16::from_le_bytes([bytes[1], bytes[2]]);
        let long = u32::from(absolute) | u32::from(bytes[3]) << 16;
        let display = match bytes[0] {
            0xE2 | 0xC2 | 0xA9 | 0x09 | 0x29 | 0x1A | 0x3A => true,
            0x8D | 0x9C => {
                (0x2100..=0x21FF).contains(&absolute) || (0x0468..=0x046B).contains(&absolute)
            }
            0xBF | 0x9F => long == 0x7F_201C,
            0x8F => long == 0x7E_46E6,
            _ => false,
        };
        if !display {
            return None;
        }
        at = assets::cpu::step(image, at, &mut widths)?;
    }
    (at > start).then_some(at)
}

/// A long operand as a normalized ROM offset.
fn long(bytes: &[u8]) -> Option<usize> {
    assets::maps::actors::rom_offset(bytes)
}

/// The facing that looks back at `facing`; `$8DC2`'s `EOR #1` on the game's
/// 0 down, 1 up, 2 left, 3 right.
/// The facings a player at `player` may hold to be looking at an actor at
/// `actor`, as `$80:8D2B`..`$8DB5` classifies their offset; both `None`
/// when the player is not beside them.
///
/// Along one axis the actor must be nine to sixteen pixels away; across it
/// within eight. The handler measures `actor.x - $0966` and
/// `(actor.y - 8) - $0968`, and `$0968` is the player's y less eight, so
/// both are plain differences. Within eight on both axes it tries the
/// vertical facing it settled on and then the horizontal one it had noted.
fn approach(actor: (u16, u16), player: (u16, u16)) -> [Option<Direction>; 2] {
    let dx = i32::from(actor.0) - i32::from(player.0);
    let dy = i32::from(actor.1) - i32::from(player.1);
    let across = |offset: i32| (-8..=8).contains(&offset);
    // `$8D46`..`$8D59`: beside on the horizontal axis.
    if (9..=16).contains(&dx) {
        return [across(dy).then_some(Direction::Right), None];
    }
    if (-16..=-9).contains(&dx) {
        return [across(dy).then_some(Direction::Left), None];
    }
    if !across(dx) {
        return [None, None];
    }
    // `$8D83`..`$8DA3`: aligned horizontally, so the vertical axis decides.
    let horizontal = if dx >= 0 {
        Direction::Right
    } else {
        Direction::Left
    };
    if (9..=16).contains(&dy) {
        return [Some(Direction::Down), None];
    }
    if (-16..=-9).contains(&dy) {
        return [Some(Direction::Up), None];
    }
    if (0..=8).contains(&dy) {
        return [Some(Direction::Down), Some(horizontal)];
    }
    if (-8..=-1).contains(&dy) {
        return [Some(Direction::Up), Some(horizontal)];
    }
    [None, None]
}

/// Cell offset one step in a direction.
const fn delta(direction: Direction) -> (i16, i16) {
    match direction {
        Direction::Up => (0, -1),
        Direction::Down => (0, 1),
        Direction::Left => (-1, 0),
        Direction::Right => (1, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bank-`$88` image with a script at `$88:8000`.
    fn image_with(script: &[u8]) -> Vec<u8> {
        let mut image = vec![0u8; 0x09_0000];
        image[0x08_8000..0x08_8000 + script.len()].copy_from_slice(script);
        image
    }

    fn open(width: u16, height: u16) -> Vec<u16> {
        vec![0; usize::from(width) * usize::from(height)]
    }

    #[test]
    fn a_static_loop_selects_its_pose_and_never_moves() {
        // COP B6, COP 80 02, COP 8E, BRA -8.
        let image = image_with(&[0x02, 0xB6, 0x02, 0x80, 0x02, 0x02, 0x8E, 0x80, 0xF7]);
        let cells = open(8, 8);
        let mut around = Surroundings {
            image: &image,
            globals: &mut Globals::with_events(vec![0; 512]),
            cells: &cells,
            width: 8,
            height: 8,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        };
        let mut actor = Actor::new((40, 48), Some(0x88_8000), 7, 1);
        for _ in 0..200 {
            actor.tick(&mut around);
        }
        assert_eq!(actor.position, (40, 48));
        assert_eq!((actor.selector, actor.hflip), (2, false));
        assert!(!actor.is_gone());
        assert!(matches!(actor.state, State::Waiting(1) | State::Running));
    }

    #[test]
    fn a_random_walk_stays_inside_its_rectangle_and_off_blocked_cells() {
        // COP 26 2 5 2 4, COP 8F, BRA -10: the handler lets the actor reach
        // one cell past the far operands, so columns 2..=6 and rows 2..=5.
        let image = image_with(&[0x02, 0x26, 2, 5, 2, 4, 0x02, 0x8F, 0x80, 0xF6]);
        let mut cells = open(8, 8);
        // Column 4, row 3 is a wall.
        cells[3 * 8 + 4] = 14 << 9;
        let mut around = Surroundings {
            image: &image,
            globals: &mut Globals::with_events(vec![0; 512]),
            cells: &cells,
            width: 8,
            height: 8,
            occupied: &[(3, 4)],
            player: (0, 0),
            facing: Direction::Down,
        };
        // Origin (56, 64): collision cell (3, 3).
        let mut actor = Actor::new((56, 64), Some(0x88_8000), 0, 12345);
        let mut visited = std::collections::BTreeSet::new();
        for _ in 0..5000 {
            actor.tick(&mut around);
            let (column, row) = actor.collision_cell();
            assert!(
                (2..=6).contains(&column) && (2..=5).contains(&row),
                "{:?}",
                actor.position
            );
            assert_ne!((column, row), (4, 3), "walked into a wall");
            assert_ne!((column, row), (3, 4), "walked into someone");
            visited.insert((column, row));
        }
        assert!(visited.len() > 3, "never went anywhere: {visited:?}");
        assert!(
            visited.contains(&(6, 5)),
            "the far corner is reachable: {visited:?}"
        );
        // Between steps, positions stay on the grid the record put them on.
        while actor.walking {
            actor.tick(&mut around);
        }
        assert_eq!(actor.position.0 % 16, 8);
        assert_eq!(actor.position.1 % 16, 0);
    }

    #[test]
    fn walking_selects_the_walking_sequence_and_standing_the_standing_one() {
        // A rectangle whose maxima lie below the actor's cell refuses every
        // direction, so every draw stands.
        let image = image_with(&[0x02, 0x26, 3, 2, 3, 2, 0x02, 0x8F, 0x80, 0xF6]);
        let cells = open(8, 8);
        let mut around = Surroundings {
            image: &image,
            globals: &mut Globals::with_events(vec![0; 512]),
            cells: &cells,
            width: 8,
            height: 8,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        };
        let mut actor = Actor::new((56, 64), Some(0x88_8000), 9, 5);
        for _ in 0..50 {
            actor.tick(&mut around);
        }
        assert_eq!(actor.selector, 0, "standing, facing down");
        assert!(!actor.walking);
        // Open rectangle: some draw walks.
        let image = image_with(&[0x02, 0x26, 0, 7, 0, 7, 0x02, 0x8F, 0x80, 0xF6]);
        let mut around = Surroundings {
            image: &image,
            ..around
        };
        let mut actor = Actor::new((56, 64), Some(0x88_8000), 9, 5);
        let mut walked = false;
        for _ in 0..200 {
            actor.tick(&mut around);
            if actor.walking {
                walked = true;
                assert!((3..=5).contains(&actor.selector));
                assert_eq!(actor.hflip, actor.facing == Direction::Left);
            }
        }
        assert!(walked);
    }

    #[test]
    fn the_near_player_branch_is_one_sided_and_bit_seven_inverts_it() {
        // COP 0F 7F 03 04 <$8010>: near when the player is at most sixteen
        // pixels short of tile (3, 4) on both axes, Y taken less eight.
        let mut script = vec![0x02, 0x0F, 0x7F, 3, 4, 0x10, 0x80];
        script.extend([0x02, 0x8E, 0x80, 0xFC]); // not taken: wait, loop
        script.resize(0x10, 0xEA);
        script.extend([0x02, 0x80, 0x09, 0x02, 0x8E, 0x80, 0xFB]); // taken: pose 9
        let image = image_with(&script);
        let cells = open(8, 8);
        let run = |player: (u16, u16), id: u8| {
            let mut image = image.clone();
            image[0x08_8002] = id;
            let mut around = Surroundings {
                image: &image,
                globals: &mut Globals::with_events(vec![0; 512]),
                cells: &cells,
                width: 8,
                height: 8,
                occupied: &[],
                player,
                facing: Direction::Down,
            };
            let mut actor = Actor::new((8, 16), Some(0x88_8000), 0, 1);
            actor.tick(&mut around);
            actor.selector
        };
        // Tile (3,4) is (48, 64); with Y less eight, a player at (48, 72)
        // sits exactly on it, and sixteen short still qualifies.
        assert_eq!(run((48, 72), 0x7F), 9);
        assert_eq!(run((32, 56), 0x7F), 9);
        // Seventeen short does not, and neither does being past it.
        assert_eq!(run((31, 72), 0x7F), 0);
        assert_eq!(run((49, 72), 0x7F), 0);
        // Bit 7 inverts; another selector never matches.
        assert_eq!(run((48, 72), 0xFF), 0);
        assert_eq!(run((31, 72), 0xFF), 9);
        assert_eq!(run((48, 72), 0x02), 0);
    }

    #[test]
    fn a_jump_into_ram_freezes_and_a_jump_into_rom_is_followed() {
        let image = image_with(&[0x4C, 0x00, 0x40]);
        let cells = open(4, 4);
        let mut around = Surroundings {
            image: &image,
            globals: &mut Globals::with_events(vec![0; 512]),
            cells: &cells,
            width: 4,
            height: 4,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        };
        let mut actor = Actor::new((8, 16), Some(0x88_8000), 0, 3);
        actor.tick(&mut around);
        assert_eq!(actor.state, State::Frozen);
        let mut script = vec![0x4C, 0x10, 0x80];
        script.resize(0x10, 0xEA);
        script.extend([0x02, 0x80, 0x05, 0x02, 0x8E, 0x80, 0xFB]);
        let image = image_with(&script);
        let mut around = Surroundings {
            image: &image,
            ..around
        };
        let mut actor = Actor::new((8, 16), Some(0x88_8000), 0, 3);
        actor.tick(&mut around);
        assert_eq!(actor.selector, 5);
    }

    #[test]
    fn a_despawn_that_holds_removes_the_actor_and_an_unknown_opcode_freezes() {
        // COP 47 <0x0010>, then a static loop.
        let image = image_with(&[0x02, 0x47, 0x10, 0x00, 0x02, 0x8E, 0x80, 0xFC]);
        let cells = open(4, 4);
        let mut set = vec![0u8; 512];
        set[2] = 1;
        let mut around = Surroundings {
            image: &image,
            globals: &mut Globals::with_events(set.clone()),
            cells: &cells,
            width: 4,
            height: 4,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        };
        let mut actor = Actor::new((8, 16), Some(0x88_8000), 0, 3);
        actor.tick(&mut around);
        assert!(actor.is_gone());
        let clear = vec![0u8; 512];
        let mut around = Surroundings {
            globals: &mut Globals::with_events(clear.clone()),
            ..around
        };
        let mut actor = Actor::new((8, 16), Some(0x88_8000), 0, 3);
        actor.tick(&mut around);
        assert!(!actor.is_gone());
        // RTL ends the actor's own script for the frame; it stays there.
        let image = image_with(&[0x6B]);
        let mut around = Surroundings {
            image: &image,
            ..around
        };
        let mut actor = Actor::new((8, 16), Some(0x88_8000), 0, 3);
        actor.tick(&mut around);
        assert_eq!((actor.state, actor.pc), (State::Running, 0x08_8000));
        // A native opcode the interpreter does not model freezes it.
        let image = image_with(&[0xEA]);
        let mut around = Surroundings {
            image: &image,
            ..around
        };
        let mut actor = Actor::new((8, 16), Some(0x88_8000), 0, 3);
        actor.tick(&mut around);
        assert_eq!(actor.state, State::Frozen);
        assert_eq!(actor.frozen_at(), Some(0x08_8000), "and says where");
    }
}

#[cfg(test)]
mod stop_for_player_tests {
    use super::*;

    fn image_with(script: &[u8]) -> Vec<u8> {
        let mut image = vec![0u8; 0x09_0000];
        image[0x08_8000..0x08_8000 + script.len()].copy_from_slice(script);
        image
    }

    /// `COP 23 <$8000>` at the head, then pose 9, wait, and back to the head.
    /// A taken branch lands on the `COP 23` again and never reaches pose 9.
    fn loop_with_stop() -> Vec<u8> {
        vec![
            0x02, 0x23, 0x00, 0x80, 0x02, 0x80, 0x09, 0x02, 0x8E, 0x80, 0xF5,
        ]
    }

    fn run(script: &[u8], player: (u16, u16), facing: Direction, frames: usize) -> Actor {
        let image = image_with(script);
        let cells = vec![0u16; 64];
        let mut around = Surroundings {
            image: &image,
            globals: &mut Globals::with_events(vec![0; 512]),
            cells: &cells,
            width: 8,
            height: 8,
            occupied: &[],
            player,
            facing,
        };
        // Origin (56, 64): the actor's own reference for the near test.
        let mut actor = Actor::new((56, 64), Some(0x88_8000), 0, 1);
        for _ in 0..frames {
            actor.tick(&mut around);
        }
        actor
    }

    #[test]
    fn a_player_one_cell_away_and_facing_the_actor_stops_them_and_is_faced() {
        // Right of the actor, facing left: the actor turns right.
        let actor = run(&loop_with_stop(), (72, 64), Direction::Left, 20);
        assert_eq!((actor.selector, actor.hflip), (2, false));
        assert_eq!(actor.facing, Direction::Right);
        assert_eq!(actor.state, State::Running, "one branch per frame, no spin");
        // Left, facing right: mirrored.
        let actor = run(&loop_with_stop(), (40, 64), Direction::Right, 20);
        assert_eq!((actor.selector, actor.hflip), (2, true));
        assert_eq!(actor.facing, Direction::Left);
        // Above, facing down: the actor faces up.
        let actor = run(&loop_with_stop(), (56, 48), Direction::Down, 20);
        assert_eq!((actor.selector, actor.hflip), (1, false));
        // Below, facing up: the actor faces down.
        let actor = run(&loop_with_stop(), (56, 80), Direction::Up, 20);
        assert_eq!((actor.selector, actor.hflip), (0, false));
    }

    #[test]
    fn a_player_who_is_near_but_faces_away_does_not_stop_the_actor() {
        let actor = run(&loop_with_stop(), (72, 64), Direction::Right, 20);
        assert_eq!(actor.selector, 9);
        let actor = run(&loop_with_stop(), (72, 64), Direction::Up, 20);
        assert_eq!(actor.selector, 9);
    }

    #[test]
    fn the_window_is_nine_to_sixteen_on_the_axis_and_eight_across_it() {
        // `$8D46`..`$8D59`: dx of 9..=16 is beside; 17 is not, and -8 or 8
        // is "aligned", which sends the test to the vertical axis instead.
        assert_eq!(
            run(&loop_with_stop(), (64, 64), Direction::Down, 5).selector,
            1
        );
        assert_eq!(
            run(&loop_with_stop(), (48, 64), Direction::Down, 5).selector,
            1
        );
        assert_eq!(
            run(&loop_with_stop(), (65, 64), Direction::Down, 5).selector,
            9
        );
        assert_eq!(
            run(&loop_with_stop(), (65, 64), Direction::Left, 5).selector,
            2
        );
        assert_eq!(
            run(&loop_with_stop(), (47, 64), Direction::Right, 5).selector,
            2
        );
        assert_eq!(
            run(&loop_with_stop(), (73, 64), Direction::Left, 5).selector,
            9
        );
        assert_eq!(
            run(&loop_with_stop(), (40, 64), Direction::Right, 5).selector,
            2
        );
        assert_eq!(
            run(&loop_with_stop(), (39, 64), Direction::Right, 5).selector,
            9
        );
        // `$8D5B`..`$8D6B`: across the axis, eight either way still counts.
        assert_eq!(
            run(&loop_with_stop(), (72, 72), Direction::Left, 5).selector,
            2
        );
        assert_eq!(
            run(&loop_with_stop(), (72, 56), Direction::Left, 5).selector,
            2
        );
        assert_eq!(
            run(&loop_with_stop(), (72, 73), Direction::Left, 5).selector,
            9
        );
        assert_eq!(
            run(&loop_with_stop(), (72, 55), Direction::Left, 5).selector,
            9
        );
        // Vertically: 9..=16 below or above, at most eight across.
        assert_eq!(
            run(&loop_with_stop(), (56, 73), Direction::Up, 5).selector,
            0
        );
        assert_eq!(
            run(&loop_with_stop(), (56, 81), Direction::Up, 5).selector,
            9
        );
        assert_eq!(
            run(&loop_with_stop(), (64, 80), Direction::Up, 5).selector,
            0
        );
        assert_eq!(
            run(&loop_with_stop(), (48, 48), Direction::Down, 5).selector,
            1
        );
        assert_eq!(
            run(&loop_with_stop(), (56, 47), Direction::Down, 5).selector,
            9
        );
    }

    #[test]
    fn the_overlap_case_tries_the_vertical_facing_then_the_horizontal_one() {
        // `$8D79`: within eight on both axes the handler tests Down (or Up
        // when the actor is above), then the horizontal side it noted.
        assert_eq!(
            run(&loop_with_stop(), (56, 64), Direction::Down, 5).selector,
            1
        );
        assert_eq!(
            run(&loop_with_stop(), (60, 64), Direction::Left, 5).selector,
            2
        );
        assert_eq!(
            run(&loop_with_stop(), (60, 64), Direction::Right, 5).selector,
            9
        );
        assert_eq!(
            run(&loop_with_stop(), (56, 60), Direction::Up, 5).selector,
            9
        );
        assert_eq!(
            run(&loop_with_stop(), (52, 66), Direction::Up, 5).selector,
            0
        );
        assert_eq!(
            run(&loop_with_stop(), (52, 66), Direction::Right, 5).selector,
            2
        );
        assert_eq!(
            run(&loop_with_stop(), (52, 66), Direction::Left, 5).selector,
            9
        );
    }

    #[test]
    fn hold_while_paused_is_a_one_byte_service_that_continues() {
        // COP 59 08, COP 80 05, COP 8E, BRA back.
        let actor = run(
            &[0x02, 0x59, 0x08, 0x02, 0x80, 0x05, 0x02, 0x8E, 0x80, 0xF6],
            (0, 0),
            Direction::Down,
            3,
        );
        assert_eq!(actor.selector, 5);
        assert_eq!(actor.state, State::Waiting(1));
    }
}

#[cfg(test)]
mod cadence_tests {
    use super::*;

    const SITE: usize = 0x08_A868;

    fn seed_for(choice: u8) -> u32 {
        (1..100_000)
            .find(|seed| {
                let mut actor = Actor::new((0, 0), None, 0, *seed);
                actor.next_random() & 7 == choice
            })
            .unwrap()
            | 1
    }

    fn surroundings<'a>(
        image: &'a [u8],
        cells: &'a [u16],
        globals: &'a mut Globals,
    ) -> Surroundings<'a> {
        Surroundings {
            image,
            globals,
            cells,
            width: 8,
            height: 8,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        }
    }

    fn run_action(choice: u8, refused: bool, cadence: cadence::Cadence) {
        let mut image = vec![0; SITE + 10];
        image[SITE..].copy_from_slice(&[2, 0x26, 0, 7, 0, 7, 2, 0x8F, 0x80, 0xF6]);
        let cells = vec![if refused { 14 << 9 } else { 0 }; 64];
        let mut globals = Globals::with_events(vec![0; 512]);
        let mut around = surroundings(&image, &cells, &mut globals);
        let seed = seed_for(choice);
        let mut actor = Actor::new((56, 64), Some(0x88_A868), 0, seed);
        actor.cadence = Some(cadence);
        let moving = choice < 4 && !refused;
        let direction = match choice {
            0 => Direction::Down,
            1 => Direction::Up,
            2 => Direction::Left,
            _ => Direction::Right,
        };
        let (dx, dy) = if moving { delta(direction) } else { (0, 0) };
        let duration = if moving {
            cadence.walk(direction)
        } else {
            cadence.idle
        };
        for action in 0..2 {
            let start = actor.position;
            let cell = actor.collision_cell();
            let destination = moving.then_some((
                cell.0.wrapping_add_signed(dx),
                cell.1.wrapping_add_signed(dy),
            ));
            actor.rng = seed;
            for tick in 0..duration {
                actor.tick(&mut around);
                let pixels = i16::try_from(tick / 2 + 1).unwrap();
                assert_eq!(
                    actor.position,
                    (
                        start.0.wrapping_add_signed(dx * pixels),
                        start.1.wrapping_add_signed(dy * pixels)
                    )
                );
                assert_eq!(actor.pose_age, u32::from(tick), "action {action}");
                assert_eq!(actor.walking, moving);
                assert_eq!(
                    actor.destination(),
                    destination,
                    "reservation must not drift mid-step"
                );
            }
            let travelled = (
                actor.position.0.abs_diff(start.0),
                actor.position.1.abs_diff(start.1),
            );
            assert_eq!(travelled.0 + travelled.1, if moving { 16 } else { 0 });
        }
    }

    const CLASS_ZERO: cadence::Cadence = cadence::Cadence {
        walk: [32; 4],
        idle: 16,
    };

    #[test]
    fn qualified_actions_start_immediately_and_restart_all_four_directions_without_gaps() {
        for choice in 0..4 {
            run_action(choice, false, CLASS_ZERO);
        }
    }

    #[test]
    fn qualified_random_idle_and_all_refusals_take_the_idle_length() {
        run_action(4, false, CLASS_ZERO);
        for choice in 0..4 {
            run_action(choice, true, CLASS_ZERO);
        }
        let short = cadence::Cadence {
            walk: [32; 4],
            idle: 9,
        };
        run_action(4, false, short);
    }

    #[test]
    fn a_thirty_one_tick_list_still_walks_sixteen_pixels() {
        let cadence = cadence::Cadence {
            walk: [32, 31, 32, 32],
            idle: 16,
        };
        run_action(1, false, cadence);
    }

    #[test]
    fn a_skipped_service_that_could_change_movement_revokes_admission() {
        for (bytes, kept) in [
            ([2, 0xBB, 0x0C], true),
            ([2, 0xBB, 0x40], false),
            ([2, 0x28, 4], false),
        ] {
            let mut image = vec![0; SITE + 3];
            image[SITE..].copy_from_slice(&bytes);
            let mut actor = Actor::new((0, 0), Some(0x88_A868), 0, 1);
            actor.cadence = Some(CLASS_ZERO);
            let mut globals = Globals::with_events(vec![0; 512]);
            actor.tick(&mut surroundings(&image, &[], &mut globals));
            assert_eq!(actor.cadence.is_some(), kept, "{bytes:02X?}");
        }
    }
}

#[cfg(test)]
mod script_service_tests {
    use super::*;

    const AT: usize = 0x08_8000;

    /// The door's push test (`$88:AB46`): COP2F Up, else to the reset; the
    /// player pose idiom; pose 7 (pushing); the idiom's PLX; the reset, pose 9.
    fn push_test() -> Vec<u8> {
        let mut code = vec![2, 0x2F, 0x00, 0x08, 0x23, 0x80];
        code.extend_from_slice(&[
            0xDA, 0xAE, 0xEA, 0x0D, 0xBF, 0x16, 0x20, 0x7F, 0xC9, 0x01, 0x00, 0xD0, 0x0F, 0xBF,
            0x08, 0x00, 0x7F, 0xC9, 0x04, 0x00, 0xD0, 0x06, 0xFA,
        ]);
        code.extend_from_slice(&[2, 0x80, 7, 2, 0xBD, 0xFA]);
        assert_eq!(code.len(), 0x23);
        code.extend_from_slice(&[2, 0x80, 9, 2, 0xBD]);
        code
    }

    fn tick_held(actor: &mut Actor, image: &[u8], pad: u16) {
        let mut globals = Globals::with_events(vec![0; 512]);
        globals.pad = pad;
        actor.tick(&mut Surroundings {
            image,
            globals: &mut globals,
            cells: &[],
            width: 0,
            height: 0,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        });
    }

    #[test]
    fn cop_2f_goes_on_while_its_buttons_are_held_and_jumps_otherwise() {
        // COP2F Up to $8023; pose 7; ... ; $8023: pose 9.
        let mut code_with_gap = push_test()[..6].to_vec();
        code_with_gap.extend_from_slice(&[2, 0x80, 7, 2, 0xBD]);
        code_with_gap.resize(0x23, 0);
        code_with_gap.extend_from_slice(&[2, 0x80, 9, 2, 0xBD]);
        for (pad, selector) in [(0, 9), (0x0800, 7), (0x0400, 9)] {
            let (image, mut actor) = actor_running(&code_with_gap);
            tick_held(&mut actor, &image, pad);
            assert_eq!(actor.selector, selector, "pad {pad:#06x}");
        }
    }

    #[test]
    fn cop_4a_branches_on_equal_or_on_its_mode_bits_greater_and_less() {
        // COP4A op, 3, $8020; pose 7; ...; $8020: pose 9. Counter 2 holds 5.
        for (op, branches) in [(0x02, false), (0x82, true), (0x42, false), (0xC2, true)] {
            let mut code = vec![2, 0x4A, op, 3, 0, 0x20, 0x80, 2, 0x80, 7, 2, 0xBD];
            code.resize(0x20, 0);
            code.extend_from_slice(&[2, 0x80, 9, 2, 0xBD]);
            let (image, mut actor) = actor_running(&code);
            let mut globals = Globals::with_events(vec![0; 512]);
            globals.counters[2] = 5;
            actor.tick(&mut Surroundings {
                image: &image,
                globals: &mut globals,
                cells: &[],
                width: 0,
                height: 0,
                occupied: &[],
                player: (0, 0),
                facing: Direction::Down,
            });
            assert_eq!(actor.selector == 9, branches, "op {op:#04x}");
        }
    }

    #[test]
    fn a_hit_does_not_reach_a_script_that_holds_the_world_or_froze() {
        // COP65 $8010 back $88:8008; COP1F with no text blocks; ...
        let mut code = vec![2, 0x65, 0x10, 0x80, 0x08, 0x80, 0x88, 2, 0x8E];
        code.resize(0x10, 0);
        code.extend_from_slice(&[2, 0xBD]);
        let (image, mut actor) = actor_running(&code);
        tick(&mut actor, &image);
        assert!(actor.hittable());
        for state in [State::Blocked(Wait::Text), State::Frozen, State::Gone] {
            let mut held = actor.clone();
            held.state = state;
            assert!(!held.strike(), "{state:?}");
            assert_eq!(held.state, state);
        }
        assert!(actor.strike());
    }

    fn tick_at(actor: &mut Actor, image: &[u8], globals: &mut Globals, player: (u16, u16)) {
        actor.tick(&mut Surroundings {
            image,
            globals,
            cells: &[],
            width: 0,
            height: 0,
            occupied: &[],
            player,
            facing: Direction::Down,
        });
    }

    #[test]
    fn a_contact_callback_registers_natively_and_disarms_itself() {
        // LDA #$8010; STA $7F:1010,X; yield. $8010: COP07 $8001; clear
        // +$04 bit $0200; RTL.
        let mut code = vec![0xA9, 0x10, 0x80, 0x9F, 0x10, 0x10, 0x7F, 2, 0xBD];
        code.resize(0x10, 0);
        code.extend_from_slice(&[2, 0x07, 0x01, 0x80]);
        code.extend_from_slice(&[0xBD, 0x04, 0x00, 0x29, 0xFF, 0xFD, 0x9D, 0x04, 0x00, 0x6B]);
        let (image, mut actor) = actor_running(&code);
        let mut globals = Globals::with_events(vec![0; 512]);
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        assert_eq!(actor.contact(), Some(0x08_8010));
        let mut around = Surroundings {
            image: &image,
            globals: &mut globals,
            cells: &[],
            width: 0,
            height: 0,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        };
        assert_eq!(actor.run_contact(&mut around), None);
        assert_eq!(globals.events[0] & 2, 2, "local 1");
        assert_eq!(actor.contact(), None, "category $0200 cleared");
        // A zero target removes the callback.
        let (image, mut actor) = actor_running(&[0xA9, 0, 0, 0x9F, 0x10, 0x10, 0x7F, 2, 0xBD]);
        actor.contact = Some(0x08_8010);
        tick(&mut actor, &image);
        assert_eq!(actor.contact(), None);
        assert_eq!(actor.frozen_at(), None);
    }

    #[test]
    fn cop_0d_tests_the_player_in_cells_around_the_actor() {
        // COP0D any facing, (-1,-1)..(1,1), else $8020; pose 7. $8020: pose 9.
        let mut code = vec![
            2, 0x0D, 0xFF, 0xFF, 0xFF, 0x01, 0x01, 0x20, 0x80, 2, 0x80, 7, 2, 0xBD,
        ];
        code.resize(0x20, 0);
        code.extend_from_slice(&[2, 0x80, 9, 2, 0xBD]);
        // The box at (136,384): raw X 120..152, raw Y 368..400.
        for (player, selector) in [
            ((136, 368), 7),
            ((120, 400), 7),
            ((152, 368), 7),
            ((136, 367), 9),
            ((153, 380), 9),
            ((136, 401), 9),
        ] {
            let (image, mut actor) = actor_running(&code);
            actor.position = (136, 384);
            tick_at(
                &mut actor,
                &image,
                &mut Globals::with_events(vec![0; 512]),
                player,
            );
            assert_eq!(actor.selector, selector, "{player:?}");
        }
        // A facing that is not the player's fails; with bit 7, a failure
        // jumps.
        code[2] = 0x81;
        let (image, mut actor) = actor_running(&code);
        actor.position = (136, 384);
        tick_at(
            &mut actor,
            &image,
            &mut Globals::with_events(vec![0; 512]),
            (136, 380),
        );
        assert_eq!(actor.selector, 9);
    }

    #[test]
    fn cop_df_waits_for_the_player_to_finish_a_forced_action() {
        let code = [2, 0xDF, 0xA6, 0x8E, 0x88, 2, 0x80, 7, 2, 0xBD];
        let (image, mut actor) = actor_running(&code);
        let mut globals = Globals::with_events(vec![0; 512]);
        globals.player_action = true;
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        assert_eq!(actor.selector, 0, "retries while $097C & $0810");
        globals.player_action = false;
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        assert_eq!(actor.selector, 7);
    }

    #[test]
    fn cop_14_queues_a_transfer_at_its_position_plus_8_16() {
        let code = [
            2, 0x14, 0x21, 0, 7, 1, 0x80, 0, 0x60, 1, 2, 0x80, 7, 2, 0xBD,
        ];
        let (image, mut actor) = actor_running(&code);
        let mut globals = Globals::with_events(vec![0; 512]);
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        let transfer = Transfer {
            map: 0x21,
            position: (136, 368),
            mode: 7,
        };
        assert_eq!(globals.transfer, Some(transfer));
        assert_eq!(actor.selector, 7, "the script goes on");
    }

    #[test]
    fn the_box_steps_over_its_hit_profile_sounds_and_the_players_damage_bit() {
        let code = [
            2, 0xD9, 0x01, 0xAC, 0xEA, 0x0D, 0xB9, 0x06, 0x00, 0x09, 0x20, 0x00, 0x99, 0x06, 0x00,
            2, 0x76, 0x32, 0xE7, 2, 0x38, 0x37, 0x37, 2, 0x80, 7, 2, 0xBD,
        ];
        let (image, mut actor) = actor_running(&code);
        tick(&mut actor, &image);
        assert_eq!((actor.frozen_at(), actor.selector), (None, 7));
    }

    #[test]
    fn cop_54_gives_an_item_or_jumps_when_the_inventory_is_full() {
        // COP54 $10 -> $800A; pose 7; wait; pose 9; wait.
        let code = [
            2, 0x54, 0x10, 0x0A, 0x80, 2, 0x80, 7, 2, 0xBD, 2, 0x80, 9, 2, 0xBD,
        ];
        for (held, selector) in [(0, 7), (9, 9)] {
            let (image, mut actor) = actor_running(&code);
            let mut globals = Globals::with_events(vec![0; 512]);
            for _ in 0..held {
                globals.inventory.add(0x10);
            }
            tick_at(&mut actor, &image, &mut globals, (0, 0));
            assert_eq!(actor.selector, selector, "{held} held");
            assert_eq!(globals.inventory.count(0x10), 9.min(held + 1));
        }
    }

    #[test]
    fn cop_60_grants_an_item_each_time_and_steps_over_its_presentation() {
        // COP60 $81 $01A4 $34, twice; pose 7.
        let code = [
            2, 0x60, 0x81, 0xA4, 0x01, 0x34, 2, 0x60, 0x81, 0xA4, 0x01, 0x34, 2, 0x80, 7, 2, 0xBD,
        ];
        let (image, mut actor) = actor_running(&code);
        let mut globals = Globals::with_events(vec![0; 512]);
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        assert_eq!(globals.inventory.items(), [0x81]);
        assert_eq!(globals.inventory.count(0x81), 2);
        assert_eq!(actor.selector, 7);
    }

    #[test]
    fn cop_00_calls_a_long_subroutine_and_cop_01_returns_once() {
        // COP00 $88:8010; pose 7; yield. $8010: pose 5; COP01; COP01 again
        // goes on (nothing kept); yield.
        let mut code = vec![2, 0x00, 0x10, 0x80, 0x88, 2, 0x80, 7, 2, 0xBD];
        code.resize(0x10, 0);
        code.extend_from_slice(&[2, 0x80, 5, 2, 0x01]);
        let (image, mut actor) = actor_running(&code);
        tick(&mut actor, &image);
        assert_eq!(actor.selector, 7, "returned after the call");
        assert_eq!(actor.call, None);
    }

    #[test]
    fn the_interaction_bits_follow_native_writes_to_06() {
        // LDA $0006,X; ORA #$0200; STA $0006,X; yield.
        let code = [
            0xBD, 0x06, 0x00, 0x09, 0x00, 0x02, 0x9D, 0x06, 0x00, 2, 0xBD,
        ];
        let (image, mut actor) = actor_running(&code);
        tick(&mut actor, &image);
        assert_eq!(actor.interaction, INTERACT_ANY_SIDE);
        // Another bit is refused.
        let code = [
            0xBD, 0x06, 0x00, 0x09, 0x00, 0x04, 0x9D, 0x06, 0x00, 2, 0xBD,
        ];
        let (image, mut actor) = actor_running(&code);
        tick(&mut actor, &image);
        assert!(actor.frozen_at().is_some());
    }

    #[test]
    fn cop_99_spawns_as_cop_a2_does() {
        let code = [2, 0x99, 0x29, 0x80, 0x88, 0x00, 0x40, 2, 0xBD];
        let (image, mut actor) = actor_running(&code);
        let mut globals = Globals::with_events(vec![0; 512]);
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        assert_eq!(globals.spawns, [(0x08_8029, 0x4000, actor.position)]);
    }

    #[test]
    fn the_player_pose_test_fails_for_a_player_who_never_pushes() {
        // The runtime's Ark plays no table-1 sequence 4, so the idiom takes
        // its reset branch, past the PLX there, instead of freezing.
        let (image, mut actor) = actor_running(&push_test());
        tick_held(&mut actor, &image, 0x0800);
        assert_eq!(actor.frozen_at(), None);
        assert_eq!(actor.selector, 9);
    }

    fn actor_running(code: &[u8]) -> (Vec<u8>, Actor) {
        let mut image = vec![0; AT + code.len() + 2];
        image[AT..AT + code.len()].copy_from_slice(code);
        (image, Actor::new((56, 64), Some(0x88_8000), 0, 1))
    }

    fn tick(actor: &mut Actor, image: &[u8]) {
        actor.tick(&mut Surroundings {
            image,
            globals: &mut Globals::with_events(vec![0; 512]),
            cells: &[],
            width: 0,
            height: 0,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        });
    }

    #[test]
    fn a_counted_loop_yields_one_frame_each_time_it_loops_back() {
        // COP02 3; { pose 7; COP03 }; pose 9; wait.
        let (image, mut actor) =
            actor_running(&[2, 0x02, 3, 0, 2, 0x80, 7, 2, 0x03, 2, 0x80, 9, 2, 0x8E]);
        for frame in 0..2 {
            tick(&mut actor, &image);
            assert_eq!(actor.selector, 7, "frame {frame} loops back and yields");
        }
        tick(&mut actor, &image);
        assert_eq!(actor.selector, 9, "the third pass falls through");
        assert_eq!(actor.state, State::Waiting(1));
    }

    #[test]
    fn a_pose_wait_plays_the_selected_list_once() {
        // Pose 6; COP8E; pose 9; COP8E. The command after the first wait
        // runs in the frame the list's last record ends: F + T.
        for (ticks, resumes) in [(Some(18u16), 19), (Some(1), 2), (Some(0), 1), (None, 3)] {
            let (image, mut actor) = actor_running(&[2, 0x80, 6, 2, 0x8E, 2, 0x80, 9, 2, 0x8E]);
            actor.pose_ticks = ticks.map(|ticks| {
                let mut lists = vec![Some(5); 10];
                lists[6] = Some(ticks);
                lists
            });
            for tick_number in 1..=resumes {
                tick(&mut actor, &image);
                assert_eq!(
                    actor.selector == 9,
                    tick_number == resumes,
                    "T {ticks:?} tick {tick_number}"
                );
            }
        }
    }

    #[test]
    fn an_unknown_list_keeps_the_raster_running() {
        // Pose 1 is outside a one-list table: no restart, the old wait,
        // looping back to the start with BRA -7.
        let (image, mut actor) = actor_running(&[2, 0x80, 1, 2, 0x8E, 0x80, 0xF9]);
        actor.pose_ticks = Some(vec![Some(3)]);
        for _ in 0..12 {
            tick(&mut actor, &image);
        }
        assert!(actor.pose_age >= 11, "age {}", actor.pose_age);
    }

    #[test]
    fn selecting_the_same_pose_restarts_its_list() {
        let (image, mut actor) = actor_running(&[2, 0x80, 6, 2, 0x8E, 2, 0x80, 6, 2, 0x8E]);
        actor.pose_ticks = Some(vec![Some(3); 10]);
        // The second COP80 runs on tick F + 3 = 4.
        for _ in 0..4 {
            tick(&mut actor, &image);
        }
        assert_eq!(
            actor.pose_age, 0,
            "COP80 clears the list index even for the same pose"
        );
    }

    #[test]
    fn a_loop_end_without_a_start_freezes_rather_than_running_on() {
        let (image, mut actor) = actor_running(&[2, 0x03, 2, 0x80, 9, 2, 0x8E]);
        tick(&mut actor, &image);
        tick(&mut actor, &image);
        assert_eq!(actor.state, State::Frozen);
        assert_eq!(actor.selector, 0);
    }

    #[test]
    fn a_timed_wait_resumes_n_plus_one_frames_later() {
        for (frames, resumes) in [(2u8, 4), (1, 3), (0, 2)] {
            let (image, mut actor) = actor_running(&[2, 0xC1, frames, 0, 2, 0x80, 5, 2, 0x8E]);
            for tick_number in 1..=resumes {
                tick(&mut actor, &image);
                assert_eq!(
                    actor.selector == 5,
                    tick_number == resumes,
                    "C1 {frames} tick {tick_number}"
                );
            }
        }
    }

    #[test]
    fn the_map_branch_compares_the_low_fifteen_bits_and_inverts_on_bit_fifteen() {
        // COP0A word target; pose 1; wait; target: pose 2; wait.
        for (word, map, taken) in [
            (0x00D5, 0xD5, true),
            (0x00D5, 0x0A, false),
            (0x80D5, 0x0A, true),
            (0x80D5, 0xD5, false),
        ] {
            let [low, high] = u16::to_le_bytes(word);
            let (image, mut actor) = actor_running(&[
                2, 0x0A, low, high, 0x0B, 0x80, 2, 0x80, 1, 2, 0x8E, 2, 0x80, 2, 2, 0x8E,
            ]);
            actor.map = map;
            tick(&mut actor, &image);
            assert_eq!(
                actor.selector,
                if taken { 2 } else { 1 },
                "{word:04X} on {map:#x}"
            );
        }
    }

    #[test]
    fn the_town_walker_loops_with_one_frame_gaps_and_poses_after_sixteen_actions() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let image = rom::Rom::load(&bytes).unwrap().image().to_vec();
        let flags = crate::world::new_game_flags();
        let events = assets::maps::scripts::EventFlags::Bitmap(&flags);
        let resident = crate::residents::residents(&image, 0xA, events)
            .unwrap()
            .into_iter()
            .find(|resident| resident.record == 0x03_8A2D)
            .unwrap();
        let mut actor = Actor::for_resident(&image, 0xA, &resident, 7);
        assert!(actor.cadence.is_some());
        let cells = vec![0u16; 64 * 64];
        let mut around = Surroundings {
            image: &image,
            globals: &mut Globals::with_events(flags.clone()),
            cells: &cells,
            width: 64,
            height: 64,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        };
        // Tick of each action's start, and whether it walked.
        let mut starts = Vec::new();
        for tick in 0..700u32 {
            let before = matches!(actor.state, State::Ordinary { .. });
            actor.tick(&mut around);
            if let State::Ordinary {
                ticks, ticks_left, ..
            } = actor.state
            {
                if !before || ticks_left + 1 == ticks {
                    starts.push((tick, actor.walking));
                }
            }
            if starts.len() == 16 {
                break;
            }
        }
        assert_eq!(starts.len(), 16);
        for pair in starts.windows(2) {
            let period = pair[1].0 - pair[0].0;
            assert_eq!(period, if pair[0].1 { 33 } else { 17 }, "{pair:?}");
        }
        // After the sixteenth action the first counted loop ends: four
        // passes of pose 6's 18-tick list with a loop frame between them,
        // 3 x 19 + 18 = 75 ticks to the next action, as measured natively.
        let mut posed = None;
        for step in 0..200u32 {
            let was_acting = matches!(actor.state, State::Ordinary { .. });
            actor.tick(&mut around);
            if actor.selector == 6 && posed.is_none() {
                posed = Some(step);
            }
            let acting = matches!(actor.state, State::Ordinary { .. });
            if let Some(start) = posed.filter(|_| acting && !was_acting) {
                assert_eq!(step - start, 75);
                return;
            }
        }
        panic!("no action after the pose section");
    }
}

#[cfg(test)]
mod scene_service_tests {
    use super::*;

    const AT: usize = 0x08_8000;

    fn run(code: &[(usize, &[u8])], globals: &mut Globals) -> (Vec<u8>, Actor) {
        let mut image = vec![0; AT + 0x200];
        for (offset, bytes) in code {
            image[AT + offset..AT + offset + bytes.len()].copy_from_slice(bytes);
        }
        let mut actor = Actor::new((56, 64), Some(0x88_8000), 0, 1);
        actor.tick(&mut around(&image, globals));
        (image, actor)
    }

    fn around<'a>(image: &'a [u8], globals: &'a mut Globals) -> Surroundings<'a> {
        Surroundings {
            image,
            globals,
            cells: &[],
            width: 0,
            height: 0,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        }
    }

    #[test]
    fn a_callback_writes_flags_redirects_the_script_and_returns() {
        let mut globals = Globals::with_events(vec![0; 512]);
        // Own: register $8040, face-player target, then idle on a wait.
        // Callback $8040: set flag $30, point the script at $8080, RTL.
        let (image, mut actor) = run(
            &[
                (0, &[2, 0x21, 0x40, 0x80, 2, 0x8E, 0x80, 0xFC]),
                (
                    0x40,
                    &[2, 0x07, 0x30, 0x80, 2, 0xC0, 0x80, 0x80, 0x88, 0x6B],
                ),
                (0x80, &[2, 0x80, 9, 2, 0x8E]),
            ],
            &mut globals,
        );
        assert_eq!(actor.callback, Some(0x8040));
        actor.interaction = INTERACT_ANY_SIDE;
        assert!(actor.interactable(Direction::Up));
        let held = actor.state;
        assert_eq!(actor.run_callback(&mut around(&image, &mut globals)), None);
        assert_eq!(globals.events[6] & 1, 1, "flag $30");
        // The own script resumes at the redirect, its wait cleared.
        assert_eq!((actor.pc, actor.state), (AT + 0x80, State::Running));
        assert_ne!(held, State::Running);
        actor.tick(&mut around(&image, &mut globals));
        assert_eq!(actor.selector, 9);
    }

    #[test]
    fn a_callback_that_yields_takes_over_the_actor() {
        let mut globals = Globals::with_events(vec![0; 512]);
        let (image, mut actor) = run(
            &[
                (0, &[2, 0x21, 0x40, 0x80, 2, 0x8E, 0x80, 0xFC]),
                (0x40, &[2, 0x80, 5, 2, 0x8E]),
            ],
            &mut globals,
        );
        actor.pose_ticks = Some(vec![Some(4); 8]);
        assert_eq!(actor.run_callback(&mut around(&image, &mut globals)), None);
        // The actor now waits in the callback's pose wait, on its own.
        assert_eq!(
            (actor.pc, actor.selector, actor.state),
            (AT + 0x45, 5, State::Waiting(3))
        );
    }

    #[test]
    fn inline_code_hides_and_shows_and_a_flag_wait_holds_between() {
        // Hide; wait until flag $03 is set; show; pose 4; wait.
        let mut code = HIDE.to_vec();
        code.extend_from_slice(&[2, 0x05, 0x03, 0x00]);
        code.extend_from_slice(&SHOW);
        code.extend_from_slice(&[2, 0x80, 4, 2, 0x8E]);
        let mut globals = Globals::with_events(vec![0; 512]);
        let (image, mut actor) = run(&[(0, &code)], &mut globals);
        assert!(actor.hidden);
        for _ in 0..3 {
            actor.tick(&mut around(&image, &mut globals));
            assert!(actor.hidden && actor.selector == 0, "held by the flag wait");
        }
        globals.write_flag(0x8003);
        actor.tick(&mut around(&image, &mut globals));
        assert!(!actor.hidden);
        assert_eq!(actor.selector, 4);
        // With bit 15 the wait holds while the flag is set.
        let mut set = Globals::with_events(vec![0; 512]);
        set.write_flag(0x8003);
        let (_, actor) = run(
            &[(0, &[2, 0x05, 0x03, 0x80, 2, 0x80, 4, 2, 0x8E])],
            &mut set,
        );
        assert_eq!(actor.selector, 0);
    }

    #[test]
    fn a_near_test_with_a_selector_wants_the_player_facing_that_way() {
        // `COP 0F 02 04 04 <target>`: tile (4,4), branch when near and the
        // player faces left (2). Target: pose 9; fall-through: pose 5.
        let code = [
            2, 0x0F, 0x02, 4, 4, 0x0C, 0x80, 2, 0x80, 5, 2, 0x8E, 2, 0x80, 9, 2, 0x8E,
        ];
        for (facing, pose) in [(Direction::Left, 9), (Direction::Up, 5)] {
            let mut image = vec![0; AT + 0x20];
            image[AT..AT + code.len()].copy_from_slice(&code);
            let mut actor = Actor::new((0, 0), Some(0x88_8000), 0, 1);
            let mut globals = Globals::with_events(vec![0; 512]);
            actor.tick(&mut Surroundings {
                player: (60, 60),
                facing,
                ..around(&image, &mut globals)
            });
            assert_eq!(actor.selector, pose, "{facing:?}");
        }
    }

    #[test]
    fn patches_queue_with_their_delay_and_a_tile_branch_reads_the_map() {
        // At (56,64) the actor's cell is (3,3). Patch (3,2) with tile $1A7
        // and a one-frame wait (high byte $05 >> 2), then pose 4.
        let mut globals = Globals::with_events(vec![0; 512]);
        let (image, mut actor) = run(
            &[(0, &[2, 0x44, 0, 0xFF, 0xA7, 0x05, 2, 0x80, 4, 2, 0x8E])],
            &mut globals,
        );
        assert_eq!(globals.patches, [(3, 2, 0x1A7)]);
        assert_eq!(actor.selector, 0, "waits a frame");
        actor.tick(&mut around(&image, &mut globals));
        assert_eq!(actor.selector, 4);
        // `COP 42 00 FF $1A7 -> pose 9`, else pose 5, on a map holding it.
        let code = [
            2, 0x42, 0, 0xFF, 0xA7, 0x01, 0x0D, 0x80, 2, 0x80, 5, 2, 0x8E, 2, 0x80, 9, 2, 0x8E,
        ];
        for (cell, pose) in [(0x1DA7_u16, 9), (0x0581, 5)] {
            let mut image = vec![0; AT + 0x20];
            image[AT..AT + code.len()].copy_from_slice(&code);
            let mut cells = vec![0u16; 64];
            cells[2 * 8 + 3] = cell;
            let mut actor = Actor::new((56, 64), Some(0x88_8000), 0, 1);
            let mut globals = Globals::with_events(vec![0; 512]);
            actor.tick(&mut Surroundings {
                cells: &cells,
                width: 8,
                height: 8,
                ..around(&image, &mut globals)
            });
            assert_eq!(actor.selector, pose, "{cell:04X}");
        }
    }

    #[test]
    fn placing_faces_and_lifts_the_mark() {
        let mut globals = Globals::with_events(vec![0; 512]);
        // Mark (3,3), then place at column 11, row 26 facing left.
        let (_, actor) = run(
            &[(0, &[2, 0x3B, 2, 0x13, 0x0B, 0x1A, 2, 2, 0x8E])],
            &mut globals,
        );
        assert_eq!(actor.position, (184, 416));
        assert_eq!(
            (actor.facing, actor.hflip, actor.stamp()),
            (Direction::Left, true, None)
        );
    }

    #[test]
    fn a_map_delete_compares_the_map_and_inverts_on_bit_fifteen() {
        for (word, gone) in [
            (0x000C_u16, true),
            (0x000D, false),
            (0x800C, false),
            (0x800D, true),
        ] {
            let [low, high] = word.to_le_bytes();
            let mut image = vec![0; AT + 0x10];
            image[AT..AT + 6].copy_from_slice(&[2, 0x49, low, high, 2, 0x8E]);
            let mut actor = Actor::new((56, 64), Some(0x88_8000), 0, 1);
            actor.map = 0x0C;
            let mut globals = Globals::with_events(vec![0; 512]);
            actor.tick(&mut around(&image, &mut globals));
            assert_eq!(actor.is_gone(), gone, "{word:04X}");
        }
    }

    #[test]
    fn a_repeated_pose_holds_its_list_that_many_times() {
        // `85 28 01; 8F; 80 07; 8E`: pose 1, one tick, forty times over.
        let mut image = vec![0; AT + 0x10];
        image[AT..AT + 10].copy_from_slice(&[2, 0x85, 0x28, 1, 2, 0x8F, 2, 0x80, 7, 2]);
        image[AT + 10] = 0x8E;
        let mut actor = Actor::new((56, 64), Some(0x88_8000), 0, 1);
        actor.pose_ticks = Some(vec![Some(1); 8]);
        let mut globals = Globals::with_events(vec![0; 512]);
        let mut frames = 1;
        actor.tick(&mut around(&image, &mut globals));
        while actor.selector != 7 {
            actor.tick(&mut around(&image, &mut globals));
            frames += 1;
            assert!(frames < 100);
        }
        // `COP 8F` ran on frame 1; forty ticks later, frame 41, the next.
        assert_eq!(frames, 41);
    }

    #[test]
    fn counters_store_add_and_a_yield_takes_one_frame() {
        let mut globals = Globals::with_events(vec![0; 512]);
        // Store 0, add 1 twice at $0640, yield between, then pose 3.
        let (image, mut actor) = run(
            &[(
                0,
                &[
                    2, 0x4B, 0, 0, 0, 2, 0x4B, 0x80, 1, 0, 2, 0xBD, 2, 0x4B, 0x80, 1, 0, 2, 0x80,
                    3, 2, 0x8E,
                ],
            )],
            &mut globals,
        );
        assert_eq!((globals.counter(0), actor.selector), (1, 0));
        actor.tick(&mut around(&image, &mut globals));
        assert_eq!((globals.counter(0), actor.selector), (2, 3));
        // BCD: 9 + 1 is $10; the cap is 9999.
        globals.count(0x02, 9);
        globals.count(0x82, 1);
        assert_eq!(globals.counter(2), 0x10);
        globals.count(0x02, 0x9999);
        globals.count(0x82, 5);
        assert_eq!(globals.counter(2), 0x9999);
        // Bit 6's subtraction is refused rather than guessed.
        assert!(!globals.count(0x40, 1));
    }

    #[test]
    fn cop_22_switches_on_the_spawn_parameter_and_b0_sets_a_speed() {
        // COP B0 02 (a speed), then COP 22 01 02 with a table of two
        // targets: parameter 1 jumps to $8030, 2 to $8040, anything else
        // goes on past the table to COP 8E.
        let script: &[u8] = &[
            2, 0xB0, 0x02, 2, 0x22, 0x01, 0x02, 0x30, 0x80, 0x40, 0x80, 2, 0x8E,
        ];
        for (parameter, expected) in [(1, AT + 0x30), (2, AT + 0x40), (0, AT + 11), (3, AT + 11)] {
            let mut image = vec![0; AT + 0x200];
            image[AT..AT + script.len()].copy_from_slice(script);
            for at in [0x30, 0x40] {
                image[AT + at..AT + at + 2].copy_from_slice(&[2, 0x8E]);
            }
            let mut globals = Globals::with_events(vec![0; 512]);
            let mut actor = Actor::new((56, 64), Some(0x88_8000), 0, 1);
            actor.parameter = parameter;
            actor.tick(&mut around(&image, &mut globals));
            assert_ne!(actor.state, State::Frozen, "parameter {parameter}");
            // Held on the `COP 8E` there, past its two bytes.
            assert_eq!(actor.pc, expected + 2, "parameter {parameter}");
        }
        // B0 FF takes a word and a byte after it.
        let mut image = vec![0; AT + 0x200];
        image[AT..AT + 7].copy_from_slice(&[2, 0xB0, 0xFF, 0x34, 0x12, 0x05, 2]);
        image[AT + 7] = 0x8E;
        let mut globals = Globals::with_events(vec![0; 512]);
        let mut actor = Actor::new((56, 64), Some(0x88_8000), 0, 1);
        actor.tick(&mut around(&image, &mut globals));
        assert_eq!(actor.pc, AT + 8);
    }

    #[test]
    fn music_and_sound_services_cue_the_driver() {
        use crate::audio::Cue;
        let mut globals = Globals::with_events(vec![0; 512]);
        globals.audio.load_map(Some(0x1B));
        globals.audio.take();
        // COP 31 01, 30 34, 32 FF, 32 05, 60 (item, word, fanfare), then the
        // latch: 37 1A, 36 13, and 38 $3737 overwrites both.
        let (_, actor) = run(
            &[(
                0,
                &[
                    2, 0x31, 0x01, 2, 0x30, 0x34, 2, 0x32, 0xFF, 2, 0x32, 0x05, 2, 0x60, 0x81,
                    0xA4, 0x01, 0x35, 2, 0x37, 0x1A, 2, 0x36, 0x13, 2, 0x8E,
                ],
            )],
            &mut globals,
        );
        assert_ne!(actor.state, State::Frozen);
        globals.audio.flush();
        let track = |track, fade| Cue::Track { track, fade };
        assert_eq!(
            globals.audio.take(),
            [
                track(0x01, true),
                track(0x34, false),
                track(0x1C, false),
                track(0x06, false),
                track(0x35, false),
                Cue::Sound(0x131A),
            ]
        );
        run(&[(0, &[2, 0x38, 0x37, 0x37, 2, 0x8E])], &mut globals);
        globals.audio.flush();
        assert_eq!(globals.audio.take(), [Cue::Sound(0x3737)]);
    }

    #[test]
    fn input_locks_deletion_jumps_and_continuations() {
        // Lock $FF50, unlock $0F00, then wait.
        let mut globals = Globals::with_events(vec![0; 512]);
        run(
            &[(0, &[2, 0x2A, 0x50, 0xFF, 2, 0x29, 0x00, 0x0F, 2, 0x8E])],
            &mut globals,
        );
        assert_eq!(globals.input_mask, 0xF050);
        // Delete when flag $30 is set, which it is.
        let mut set = Globals::with_events(vec![0; 512]);
        set.write_flag(0x8030);
        let (_, actor) = run(&[(0, &[2, 0x48, 0x30, 0x80, 2, 0x8E])], &mut set);
        assert!(actor.is_gone());
        let (_, actor) = run(&[(0, &[2, 0x48, 0x30, 0x00, 2, 0x8E])], &mut set);
        assert!(!actor.is_gone());
        // Long jump to $88:8020; BC, a body, RTL: the body runs every frame
        // from the continuation.
        let mut globals = Globals::with_events(vec![0; 512]);
        let (image, mut actor) = run(
            &[
                (0, &[2, 0x06, 0x20, 0x80, 0x88]),
                (0x20, &[2, 0xBC, 2, 0x07, 0x30, 0x80, 0x6B]),
            ],
            &mut globals,
        );
        for _ in 0..3 {
            assert_eq!(actor.pc, AT + 0x22, "RTL comes back to the continuation");
            assert_eq!(globals.events[6] & 1, 1, "and the body ran");
            globals.write_flag(0x0030);
            actor.tick(&mut around(&image, &mut globals));
        }
        // Without BC, the frame starts over where it began.
        let mut globals = Globals::with_events(vec![0; 512]);
        let (image, mut actor) = run(&[(0, &[2, 0x07, 0x30, 0x80, 0x6B])], &mut globals);
        globals.write_flag(0x0030);
        actor.tick(&mut around(&image, &mut globals));
        assert_eq!((actor.pc, globals.events[6] & 1), (AT, 1));
    }
}

#[cfg(test)]
mod scripted_leg_tests {
    use super::*;

    const AT: usize = 0x08_8000;

    /// One frame of a leg service at (56,64), on the common movement base
    /// when `admitted`, with every pose list 32 ticks long.
    fn leg(code: &[u8], admitted: bool) -> Actor {
        let mut image = vec![0; AT + 0x40];
        image[AT..AT + code.len()].copy_from_slice(code);
        let mut actor = Actor::new((56, 64), Some(0x88_8000), 0, 1);
        actor.pose_ticks = Some(vec![Some(32); 8]);
        actor.legs = admitted;
        let mut globals = Globals::with_events(vec![0; 512]);
        actor.tick(&mut Surroundings {
            image: &image,
            globals: &mut globals,
            cells: &[],
            width: 0,
            height: 0,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        });
        actor
    }

    #[test]
    fn a_leg_turns_poses_and_moves_its_first_pixel_at_once() {
        // Down to row 6: pose 3, vector $68.
        let down = leg(&[2, 0x3A, 3, 0x68, 6, 2, 0x8E], true);
        assert_eq!(
            (down.facing, down.selector, down.hflip),
            (Direction::Down, 3, false)
        );
        assert_eq!((down.position, down.walking), ((56, 65), true));
        assert_eq!(down.state, State::Waiting(31));
        // Up to row 2: pose and vector one higher.
        let up = leg(&[2, 0x3A, 3, 0x68, 2, 2, 0x8E], true);
        assert_eq!(
            (up.facing, up.selector, up.position),
            (Direction::Up, 4, (56, 63))
        );
        // Left to column 1 (x 24): mirrored, vector not incremented.
        let left = leg(&[2, 0x39, 5, 0x60, 1, 2, 0x8E], true);
        assert_eq!(
            (left.facing, left.selector, left.hflip),
            (Direction::Left, 5, true)
        );
        assert_eq!(left.position, (55, 64));
        // Right to column 5 (x 88).
        let right = leg(&[2, 0x39, 5, 0x60, 5, 2, 0x8E], true);
        assert_eq!(
            (right.facing, right.hflip, right.position),
            (Direction::Right, false, (57, 64))
        );
    }

    #[test]
    fn at_its_target_a_leg_skips_its_loop_and_bit_seven_skips_four_more() {
        // Row 4 is y 64: skip 3 operands and `8E; 3B; BRA`, landing on the pose.
        let done = leg(
            &[
                2, 0x3A, 3, 0x68, 4, 2, 0x8E, 2, 0x3B, 0x80, 0xF5, 2, 0x80, 7, 2, 0x8E,
            ],
            true,
        );
        assert_eq!((done.selector, done.position), (7, (56, 64)));
        // Bit 7 of the pose: the loop also holds a four-byte `COP 23`.
        let posed = leg(
            &[
                2, 0x3A, 0x83, 0x68, 4, 2, 0x8E, 2, 0x3B, 2, 0x23, 0, 0x80, 0x80, 0xF1, 2, 0x80, 9,
                2, 0x8E,
            ],
            true,
        );
        assert_eq!(posed.selector, 9);
    }

    #[test]
    fn cop_3b_stamps_the_cell_and_a_leg_lifts_it() {
        // At (56,64) the collision cell is (3,3).
        let stamped = leg(&[2, 0x3B, 2, 0xBC, 0x6B], true);
        assert_eq!(stamped.stamp(), Some((3, 3)));
        let walking = leg(&[2, 0x3B, 2, 0x3A, 3, 0x68, 6, 2, 0x8E], true);
        assert_eq!(walking.stamp(), None);
    }

    #[test]
    fn faster_vectors_move_one_or_two_pixels_every_frame() {
        // $70 across at one pixel a frame; $88 down at two.
        let across = leg(&[2, 0x39, 5, 0x70, 5, 2, 0x8E], true);
        assert_eq!(across.position, (57, 64));
        let down = leg(&[2, 0x3A, 3, 0x88, 8, 2, 0x8E], true);
        assert_eq!(down.position, (56, 66));
        // Up adds one to the vector: $78 becomes $79.
        let up = leg(&[2, 0x3A, 3, 0x78, 1, 2, 0x8E], true);
        assert_eq!(up.position, (56, 63));
    }

    #[test]
    fn a_leg_outside_the_audited_streams_freezes() {
        assert_eq!(
            leg(&[2, 0x3A, 3, 0x70, 6, 2, 0x8E], true).state,
            State::Frozen
        );
        assert_eq!(
            leg(&[2, 0x3A, 3, 0x68, 6, 2, 0x8E], false).state,
            State::Frozen
        );
    }
}
