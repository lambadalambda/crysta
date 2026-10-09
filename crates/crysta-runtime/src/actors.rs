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
mod foe;
mod line;
mod motion;
mod native;
mod push;
mod routines;
mod shadowkeeper;
pub(crate) use routines::TORCHES;
mod sense;
mod walls;

pub(crate) use foe::helper;
pub use native::{
    Poke, Scratch, View, ARK_ARMOR, ARK_FLAGS, ARK_GATES, ARK_LIFE, ARK_MAX_LIFE, CAMERA,
    CURRENT_MAP, ENEMIES, FRAMES, HITS_OFF, MAP_MODE, PENDING_MAP, PLAYER_ACTION, PLAYER_X,
    PLAYER_Y, PREVIOUS_MAP, PRIME_BLUE, SAFE_X, SAFE_Y, WINDOW_BUSY,
};
use sense::probe;

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
/// Toggles the horizontal mirror (`$80:AA51`).
const TOGGLE_HFLIP: u8 = 0xB8;
/// Set, clear and toggle the vertical flip, `+$08` bit `$8000`
/// (`$80:AA15`, `AA24`, `AA60`).
const SET_VFLIP: u8 = 0xB4;
const CLEAR_VFLIP: u8 = 0xB5;
const TOGGLE_VFLIP: u8 = 0xB9;
const FLIPS: [u8; 6] = [
    CLEAR_HFLIP,
    SET_HFLIP,
    TOGGLE_HFLIP,
    SET_VFLIP,
    CLEAR_VFLIP,
    TOGGLE_VFLIP,
];
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
/// Redirects the actor's script to a long address and yields; `$80:AAE1`.
const REDIRECT_SCRIPT: u8 = 0xBF;
/// The services [`Actor::player_service`] runs.
const PLAYER_SERVICES: &[u8] = &[
    BRANCH_ON_PLAYER_NEAR,
    NEAR_BRANCH,
    AREA_BRANCH,
    TAKE_PLAYER,
    SET_CONTROL,
    TRANSFER,
    TRANSFER_BY_INDEX,
];
/// The services [`Actor::script_service`] runs.
const SCRIPT_SERVICES: &[u8] = &[
    WRITE_FLAG,
    REGISTER_CALLBACK,
    LOCK_INPUT,
    UNLOCK_INPUT,
    SET_SCRIPT,
    REDIRECT_SCRIPT,
    REDIRECT_AFTER,
    LONG_JUMP,
    CONTINUATION,
    DELETE_ON_FLAG,
    DELETE,
    WAIT_FOR_FLAG,
    OCCUPY,
];
/// As [`REDIRECT_SCRIPT`], the script resting a word of frames first
/// (`$80:AAC1`, `E+$0E`): the Guardner's vacuum, `$97:C492`.
const REDIRECT_AFTER: u8 = 0xBE;
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
/// Elle's cape, which `$11D`'s orb wants worn (`$90:A1F4`).
const CAPE: u8 = 0xBF;
/// Takes an item (`$80:9A5E`, `$8D:96A0`): Elle takes the thread.
const TAKE_ITEM: u8 = 0x55;
/// Jumps when an item would not fit (`$80:9A79`): item, target.
const NO_ROOM: u8 = 0x56;
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
/// Copies a block of map cells a row a frame (`$80:9532`,
/// `docs/block-patch.md`): `n lim sx sy dx dy wait`, with the row offset and
/// the layer in the actor's own `$7F:201A`/`201B`.
const BLOCK: u8 = 0x46;
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
/// Sets a cell's collision attribute (`$80:9393`): `a` (bit 7: absolute
/// cells, else offsets from the actor; `a << 9` the cell's high bits), then
/// column and row. The towers' gates seal their doors so (`$90:8FA4`); a
/// solid attribute marks the cell as [`STAMP`] does, an open one clears it.
const SEAL: u8 = 0x3F;
/// Spawns an actor running a long script with a flags word; `$80:A71B`.
const SPAWN: u8 = 0xA2;
/// Spawns one the same way, at the head of the actor list (`$0DFA`) rather
/// than after its parent, and without a parent link; `$80:A4B6`. Spawns all
/// run from the next frame here.
const SPAWN_LINKED: u8 = 0x99;
/// Sets up a camera move (`$80:B735`): a new camera, offsets from the
/// player, start and target in 16-pixel units, a speed index.
const PAN: u8 = 0xDD;
/// Starts the move and holds the script until it ends (`$80:B7D6`).
const PAN_WAIT: u8 = 0xDE;
/// The speed index the towers use, 2 pixels a frame (measured).
const PAN_SPEED: u8 = 0x80;
/// Starts an orbit about the actor's spawn point (`$80:B136`).
const ORBIT: u8 = 0xD0;
/// Steps the orbit a frame, and goes on once it ends (`$80:B1D1`).
const ORBIT_STEP: u8 = 0xD1;
/// The sine table, 1024 signed bytes a turn and a quarter more for the
/// cosine (`$81:F563`, in place on both ROMs).
const SINE: usize = 0x01_F563;
/// Spawns an actor running a long script at an offset from its parent, dx
/// mirrored with the parent, without a flags word; `$80:A56B`.
const SPAWN_AT: u8 = 0x9C;
/// As [`SPAWN_AT`] with a flags word (`$80:A79D`): the flyers' bullets.
const SPAWN_OFFSET: u8 = 0xA4;
/// A script at the actor, without a flags word, after its parent in the
/// list (`$80:A6F1`).
const SPAWN_AFTER: u8 = 0xA1;
/// [`SPAWN_AFTER`], [`SPAWN`] and [`SPAWN_OFFSET`] whose child joins the
/// spawner's group (`$80:B99A`, `B9A5`, `B9B0`; `7F:102E` = the root,
/// `$80:BB5A`).
const GROUP_SPAWNS: [(u8, u8); 4] = [
    (0xE6, SPAWN_AFTER),
    (0xE7, SPAWN),
    (0xE8, SPAWN_OFFSET),
    (0xEA, SPAWN_BEFORE),
];
/// A group's root deletes every other entity of the group (`$80:B9D1`).
const DELETE_GROUP: u8 = 0xEB;
/// A script before its parent in the list, without a flags word
/// (`$80:A50E`, `$80:BC54`); `9B` with one; `EA` the first in a group.
const SPAWN_BEFORE: u8 = 0x9A;
const SPAWN_BEFORE_FLAGS: u8 = 0x9B;
/// A script with a flags word at the list's end (`$80:A693`).
const SPAWN_LAST: u8 = 0xA0;

/// Where a spawned entity joins the actor list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListPlace {
    /// After its parent (`$80:BC7C`).
    After,
    /// Before its parent (`$80:BC54`).
    Before,
    /// At the list's head (`COP 99`).
    Head,
    /// At its end (`COP A0`).
    Last,
}
/// The spawns, and the group's deletion, which [`Actor::spawn`] runs.
const SPAWNS: &[u8] = &[
    SPAWN,
    SPAWN_LINKED,
    SPAWN_AT,
    SPAWN_OFFSET,
    SPAWN_AFTER,
    SPAWN_BEFORE,
    SPAWN_BEFORE_FLAGS,
    SPAWN_LAST,
    0xE6,
    0xE7,
    0xE8,
    0xEA,
    DELETE_GROUP,
];
/// The continents' door's parchment set-up and where it goes on, Japanese
/// (`$90:A4CE`, `$90:A565`) and European (`$97:BDCF`, `$97:BE66`); skipped
/// until `meta/issues/continent-door-parchment.md`.
const PARCHMENT: [(usize, usize); 2] = [(0x10_A4CE, 0x10_A565), (0x17_BDCF, 0x17_BE66)];
/// The enemies' death script (`$85:E27B`), which a script may jump to
/// (`COP 06`) or go on in (`COP BF`, the show's controller, `$97:CD07`).
const DEATH: usize = 0x05_E27B;

fn is_death(image: &[u8], target: usize) -> bool {
    assets::layout::offset(image, DEATH) == Some(target)
}
/// The own word that holds the group's root (`7F:102E`).
const GROUP: u16 = 0x102E;
/// The own words of an enemy's life (`7F:102A`) and its struck callback
/// (`7F:1016`).
const LIFE: u16 = 0x102A;
const STRUCK: u16 = 0x1016;
/// Frames a hit leaves the target unhittable (`$7F:1020 = $10`).
const HIT_COOLDOWN: u16 = 16;
/// Services for the display alone: the spinning window (`6A`, shape 0
/// only), PPU register writes (`76`), both into [`crate::display`], and the
/// hit profile (`D9`, `$7F:1022` from `$8D:BDFA`, stepped over).
const COSMETIC: [(u8, usize); 2] = [(SPIN, 2), (PPU_WRITE, 2)];
/// Sets the combat profile (`$80:B501`): `$8D:BDFA[n & $7F]`, and without
/// bit 7 the life too.
const PROFILE: u8 = 0xD9;
/// `COP 6A shape speed` (`$80:9DD6`).
const SPIN: u8 = 0x6A;
/// `COP 76 register value` (`$80:A127`): `$21rr = value` at the next NMI.
const PPU_WRITE: u8 = 0x76;
/// Jumps through a table of words on the spawn parameter (entity `+$26`):
/// operands the lowest and highest value, then a target per value; above
/// the highest, on past the table (`$80:8CD8`). Below the lowest the
/// handler's index wraps; no slice script reaches that, and here it goes
/// on past the table too.
const SWITCH: u8 = 0x22;
/// Picks the movement resource base (`$7F:0022`, `$80:A975`): `$4000 +
/// n << 12`, or with `FF` a word and a bank, which is not modelled
/// (`meta/issues/partial-cop-services.md`).
const SPEED: u8 = 0xB0;
/// Selects one of Ark's direct animation resources and starts a movement
/// selector through the following pose wait (`$80:A200..A2D8`). Operands:
/// pose, movement selector and resource-table index.
const PLAYER_POSE_MOVING: u8 = 0x84;
/// Ark's pose list of a resource, a count of times (`$80:A28B`).
const ARK_POSE_REPEAT: u8 = 0x89;
/// Selects a pose and starts the movement streams of the same selector
/// (`$80:A1B9`).
const POSE_MOVING: u8 = 0x81;
/// As [`POSE_MOVING`] with another selector (`$80:A1DE`): pose, selector.
const POSE_SELECTOR: u8 = 0x82;
/// Sets the next `COP 8F`'s repetitions, then [`POSE_MOVING`]
/// (`$80:A1AF`): count, pose.
const REPEAT_POSE_MOVING: u8 = 0x86;
/// Sets the next `COP 8F`'s repetitions, a pose, and another selector's
/// movement streams (`$80:A1D4`).
const REPEAT_MOVING: u8 = 0x87;
/// The common movement resource, at `$7F:6000` from `$AB:F037` (European
/// `$AE:8000`).
const COMMON_SOURCE: usize = 0x2B_F037;
const COMMON_SIZE: usize = 0x1A0C;

/// The first `frames` moves of a common movement selector (`$7E:6000`),
/// mirrored or not, its streams looping: the hurt pushes' `$36`..`$38`.
pub(crate) fn common_moves(
    image: &[u8],
    selector: u8,
    hflip: bool,
    frames: usize,
) -> Vec<(i16, i16)> {
    let resource = assets::layout::offset(image, COMMON_SOURCE)
        .and_then(|source| assets::compression::decode(image.get(source..)?, COMMON_SIZE).ok())
        .map(|packet| motion::Resource::wram(0x6000, packet.data.into()));
    let Some(mut motion) =
        resource.and_then(|resource| motion::Motion::start(resource, selector, false, None))
    else {
        return Vec::new();
    };
    (0..frames)
        .map_while(|_| motion.step((hflip, false)))
        .collect()
}

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
    /// A ROM bank's table (`COP B0 FF word bank`): the bank's offset and
    /// the table's address in it.
    Rom(usize, u16),
    /// Another private resource (`COP B0 n`, n not 0 or 2): no movement is
    /// modelled (`meta/issues/partial-cop-services.md`).
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
const SHOP_SPAWNER: u32 = 0x92_CC8A;

/// Whether `script` is the shops' spawner in `image`'s revision
/// (European `$92:E2FC`).
pub(crate) fn is_shop_spawner(image: &[u8], script: u32) -> bool {
    assets::layout::at(image, SHOP_SPAWNER) == Some(script)
}
const SOUND_PORT2: u8 = 0x37;
const SOUND_WORD: u8 = 0x38;
/// Branches on the player inside a rectangle of cells around the actor;
/// `$80:87C2`. Operands: facing, four signed cell offsets, target.
const NEAR_BRANCH: u8 = 0x0D;
/// As [`NEAR_BRANCH`] with the rectangle in map cells (`$80:876C`): the
/// tower tops' doors wait for Ark in front of them (`$90:9418`).
const AREA_BRANCH: u8 = 0x0C;
/// Takes the player's script once no forced action runs (`$097C & $0810`);
/// `$80:B827`. Operand: the long script.
const TAKE_PLAYER: u8 = 0xDF;
/// Sets Ark's control script (`$80:ADBD`, entity `$0DEE`): a byte, the long
/// script. The pad's own, [`PAD_CONTROL`], ends the script holding him;
/// another takes him as `COP DF` does.
const SET_CONTROL: u8 = 0xCB;
/// Ark's pad control script (`$84:87C1`, the same in both revisions).
const PAD_CONTROL: usize = 0x04_87C1;
/// Plays the pose and ends the script's frame as an `RTL` does; `$80:A395`.
const ANIMATE_AND_END: u8 = 0x91;
/// Calls a long subroutine, keeping one return (`$7F:0004`); `$80:8592`.
const CALL: u8 = 0x00;
/// Returns from it, or goes on when none is kept; `$80:85B8`.
const RETURN: u8 = 0x01;
/// The save screen `COP 00` calls from the desk (`$87:8590`, in place on
/// both ROMs): it clears flags `$FB` and `$FC` (`COP 07`), then runs a
/// screen of its own, which the world models.
const RECORDS: usize = 0x07_8590;
const RECORDS_CLEARS: [u16; 2] = [0x00FB, 0x00FC];
/// Starts an eased move: pose, then x and y offsets; `$80:9F4C`.
const EASE_START: u8 = 0xED;
/// Steps it by a speed each frame until it ends; `$80:9F93`.
const EASE_STEP: u8 = 0xEE;
/// Queues a map transfer: map, mode, selector, x, y; `$80:8A23`.
const TRANSFER: u8 = 0x14;
/// As [`TRANSFER`], the record from a table by the actor's `7F:101E`
/// (`$80:8A58`): the Guardners send Ark back down their tower.
const TRANSFER_BY_INDEX: u8 = 0x15;
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
/// `+$04` bits enemies set and clear: `$0001` a projectile (shields block
/// it), `$0002` and `$0020` not a target of hits, `$0010` not attacking
/// (the hit scan `$85:D281`).
const GUARD_04: u16 = 0x0033;
/// The `+$04` bits scripts may set and clear: those modelled, and some
/// accepted and not modelled (`meta/issues/partial-cop-services.md`). The guide clears and sets 12 around the
/// freezing's whitening (`$88:B507`, `$88:B53F`) and clears 8, the
/// dispatcher's target bit, before it leaves (`$88:AF1A`). Bit 13 lets a
/// hidden body move (`$80:C967`), as every body does here (the Guardner's
/// dive, `$97:C573`). Bit 7, out of play, is the runtime's own: a copy
/// clears it when struck (`$97:C9C5`).
const MODELLED_04: u16 = 0x8000 | 0x2000 | 0x1000 | 0x0200 | 0x0100 | 0x0080 | WALLS_04 | GUARD_04;
/// `+$04` bit `$0004`: the walls stop the enemy (with `$0002` clear, which
/// the scripts that set it leave clear: `$90:9C27`, a dropped Hiball).
const WALLS_04: u16 = 0x0004;
/// `+$06` bits: `$0010` the script handles knockback (none), `$0020` takes
/// no damage.
const GUARD_06: u16 = 0x0030;
/// `+$06` bits 11 to 14: the draw pass's depth key (`$80:E9FB`,
/// [`crate::residents::Resident::draw_depth`]).
const DEPTH_06: u16 = 0x7800;
/// `+$06` bit 6: a pose's change keeps the movement streams (`$80:ED54`),
/// and they run on every frame (`$80:D0CF`) until their own end:
/// Shadowkeeper's shots fall on after their pose.
const KEEP_STREAMS: u16 = 0x0040;
/// `LDA $0016,Y; CMP $0016,X`, Y Ark's entity (`LDY $0DEA` before it, or
/// left so by `COP 59`, `$97:B99A`): Ark on the actor's layer. The runtime
/// keeps one layer for both, so the test always holds.
const SAME_LAYER: [u8; 6] = [0xB9, 0x16, 0x00, 0xDD, 0x16, 0x00];
/// `LDY $0DEA`.
const LOAD_PLAYER_Y: [u8; 3] = [0xAC, 0xEA, 0x0D];
/// `LDA $0004,X; BIT #$4000`: the actor off screen.
const OFF_SCREEN_TEST: [u8; 6] = [0xBD, 0x04, 0x00, 0x89, 0x00, 0x40];
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
/// Adds a word to the actor's y (`$80:A9D7`).
const MOVE_Y: u8 = 0xB2;
/// Writes a byte into `+$08` bits 8–15 (`$80:AA6F`): bits 12–13 are the
/// OBJ priority.
const OBJ_PRIORITY: u8 = 0xBA;
/// The palette field.
const PALETTE: u8 = 0xBB;
/// The circle window around the caller (`$80:9CA5`): the shape, then
/// `$0474`, the radius ([`crate::world::Circle`]).
const CIRCLE: u8 = 0x63;
/// `$0474`: the circle window's radius.
pub(crate) const CIRCLE_RADIUS: u16 = 0x0474;
/// Colours from the ROM into CGRAM (`$80:9AEB`): bank, word, index,
/// count; the OBJ ones go to [`crate::colours::ObjColours`].
const LOAD_COLOURS: u8 = 0x5A;
/// Player tests against Ark's probe (x, y - 8) (`docs/enemy-scripts.md`):
/// jump when it is within a distance on both axes (`$80:B474`).
const PLAYER_NEAR: u8 = 0xD6;
/// Jump to the vertical or the horizontal target by the larger offset
/// (`$80:B4B1`).
const PLAYER_AXIS: u8 = 0xD7;
/// Left, even or right of the actor beyond a dead zone (`$80:B362`).
const PLAYER_SIDE: u8 = 0xD3;
/// Above, even or below, by Ark's feet (`$80:B38A`).
const PLAYER_HEIGHT: u8 = 0xD4;
/// Jumps when Ark is busy, down or out of play, or `$097E & m1`, or
/// `$097C & m2`; else goes on (`$80:A016`): m1, m2, the target.
const ARK_BUSY: u8 = 0x71;
/// Clears the cells under the actor's box (`$80:93D4`): mode, column, row;
/// a pushed block before it moves (`$90:FC6E`). Its marks go.
const UNSEAL: u8 = 0x40;
/// Patches a map cell (`$80:9486`): column, row, tile word.
const PATCH_AT: u8 = 0x43;
/// Jumps when the cell beyond the box is solid, Up, Down, Left, Right
/// (`$80:AB2E..AB88`): the target.
const BLOCKED_UP: u8 = 0xC2;
const BLOCKED_RIGHT: u8 = 0xC5;
/// Jumps while the pad holds every bit of a mask (`$80:9007`): mask,
/// target.
const HELD: u8 = 0x2B;
/// The pad bits a push holds (as `COP 2A $FFF0`).
const PAD_HELD: u16 = 0xFFF0;
/// Starts a line move toward `$7F:2004/2006,X` (`$80:AE38`): legs, pose,
/// speed, frame limit, selector (`FF` none).
const LINE_START: u8 = 0xCC;
/// One frame of the line move, until it ends (`$80:AF22`).
const LINE_STEP: u8 = 0xCD;
/// Ark's probe in the box in front, the facings admitted (`$80:B26E`).
const PLAYER_FRONT: u8 = 0xD2;
/// While off screen (entity `+$04` bit 14, which the draw pass keeps), sleep
/// n frames and try again (`$80:9AC7`, `docs/enemy-scripts.md`).
const SLEEP_OFF_SCREEN: u8 = 0x59;
/// One step of the random generator (`$86:8236`, `$80:8E33`).
const RANDOMIZE: u8 = 0x25;
/// x += word, negated for a mirrored actor (`$80:A9B9`).
const MOVE_X: u8 = 0xB1;
/// The same, then y += a second word (`$80:A9EA`).
const MOVE_XY: u8 = 0xB3;
/// The services [`Actor::body`] runs.
const BODY: [u8; 8] = [
    MOVE_X,
    MOVE_Y,
    MOVE_XY,
    SET_PACKET,
    OBJ_PRIORITY,
    PALETTE,
    ORBIT,
    ORBIT_STEP,
];
/// Points the actor at another art packet (`$80:B4DF`): its display lists,
/// which a `COP 8E` wait plays, are the new packet's.
const SET_PACKET: u8 = 0xD8;
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
/// Starts a counted loop; `$80:85DF`. Operand: a two-byte count. The count
/// and the address after the operand are the slot's single loop level.
const LOOP_START: u8 = 0x02;
/// Ends a pass of the counted loop; `$80:85F8`. Decrements the count; while
/// it is nonzero, jumps to the loop start and yields one frame.
const LOOP_END: u8 = 0x03;
/// As [`LOOP_END`] without the yield (`$80:8613`): Shadowkeeper's tail
/// spawns its segments in one frame.
const LOOP_AGAIN: u8 = 0x04;
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
    /// The save screen, closed.
    Records,
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

impl Surroundings<'_> {
    /// The map's first layer as the wall tests see it.
    fn layer(&self) -> walls::Layer<'_> {
        walls::Layer {
            cells: self.cells,
            width: self.width,
            height: self.height,
            gaps: self.globals.tower_floor(),
        }
    }
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
    /// Vertical flip in force (`+$08` bit `$8000`).
    pub vflip: bool,
    /// Frames since the pose changed or a qualified action restarted it.
    pub pose_age: u32,
    /// Whether a step is under way.
    pub walking: bool,
    /// `+$04` bit 12: it runs in the nested frame of a transfer's fade
    /// (`$80:C85E`); the main loop runs every actor.
    nested: bool,
    /// Entity `+$04` bit 15: not drawn, not animated, not moved.
    pub hidden: bool,
    /// OBJ priority (entity `+$08` bits 12–13): 2 unless `COP BA` set it.
    pub priority: u8,
    /// `+$06`'s depth bits ([`DEPTH_06`]).
    pub depth: u16,
    /// `+$06` bit 6 ([`KEEP_STREAMS`]).
    keeps_streams: bool,
    /// Whether this frame's streams moved it already.
    streamed: bool,
    /// Palette field (entity `+$08` bits 9–11, `COP BB`): added to each
    /// frame's OBJ palette, modulo 8.
    pub palette: u8,
    /// The player's pose a `COP 84` selected: Ark's resource and list.
    pub player_pose: Option<(u8, u8)>,
    /// Its own bytes in bank `$7F`, as native runs keep them.
    own: native::Own,
    /// A native run paused in a nested frame, to go on next frame.
    paused: Option<native::Paused>,
    /// Where a script's `JSR` returns to (`RTS`).
    subroutine: Option<usize>,
    /// `COP D0`'s orbit, while `COP D1` steps it.
    orbit: Option<Orbit>,
    /// An enemy's combat state, from its descriptor's profile.
    pub(crate) foe: Option<foe::Foe>,
    /// The boxes of its poses.
    boxes: Option<foe::Boxes>,
    /// Died this frame, for the world's EXP.
    pub(crate) died: bool,
    /// The helper art's list drawn instead of the body.
    pub(crate) overlay: Option<(u32, u8)>,
    /// A spawn's art turned to the helper art's (`COP D8`, Shadowkeeper's
    /// wisps `$8F:80C5`): its lists are drawn in place of its parent's.
    helper_art: Option<u32>,
    /// A sleep `COP 46` or a native store leaves for the next yield
    /// (`E+$0E`).
    sleep: u16,
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
    /// Combat bits scripts write (`docs/enemy-scripts.md` §4), kept as
    /// written: `+$04` [`GUARD_04`] and `+$06` [`GUARD_06`].
    guard: (u16, u16),
    /// `+$04 & $0006 == $0004`: moves stop at walls.
    walls: bool,
    /// The frame's move, applied at its end against the walls.
    pending: (i16, i16),
    /// A line move under way (`COP CC`).
    line: Option<line::Line>,
    /// Spawned by another actor's script.
    spawned: bool,
    /// The parent as it was at the spawn, for `$7F:001E,X` reads.
    parent: Option<native::View>,
    /// The entity before it in the list (`+$2C`), which the world keeps.
    pub(crate) previous: Option<u16>,
    /// Where a spawn joins the list ([`ListPlace`]).
    pub(crate) place: ListPlace,
    /// The id other actors' runs know it by (`views`, `LDY $0026,X`).
    pub(crate) id: u16,
    /// It heads a group (`7F:001E = $FFFF`, `$80:BB60`).
    root: bool,
    /// A hit that did not kill wakes its struck callback next frame.
    pub(crate) struck: bool,
    /// It headed a group and died: the group goes with it.
    root_died: bool,
    /// `COP C2`-`C5` that found a wall this frame.
    blocked_tests: u8,
    /// Ark's own script (`COP DF`), and the frames of the list his pose
    /// service named (`COP 84`, `89`), which `COP 8E`/`8F` wait out.
    ark: bool,
    ark_list: Option<u16>,
    /// The resource and list `ark_pose` showed, and whether `COP 89`
    /// repeats it: Ark's own art.
    ark_shown: Option<(u8, u8, bool)>,
    /// The child the last `COP 99` spawned, for the run after it.
    linked: Option<u16>,
    /// A a run left at a jump, and the jump's target, for a run there.
    carried: Option<(usize, u16)>,
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
    pub(crate) parameter: u8,
    /// Entry of the one fully authenticated map `$21` frozen-return stream.
    /// Other player scripts keep the prior generic skipped-service behavior.
    frozen_return: Option<usize>,
    /// The movement resource the streams read: the descriptor's
    /// (`$80:FAAF`) until `COP B0` picks another.
    base: Base,
    /// The descriptor the actor is built from, whose movement pointer
    /// fills its own base.
    descriptor: Option<usize>,
    /// A pose's movement streams (`COP 81`/`87`), through the next wait.
    motion: Option<motion::Motion>,
    /// The common and the own movement resource, once read.
    resources: [Option<motion::Resource>; 3],
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
            vflip: false,
            pose_age: 0,
            walking: false,
            nested: false,
            hidden: false,
            priority: 2,
            depth: 0,
            keeps_streams: false,
            streamed: false,
            palette: 0,
            player_pose: None,
            own: native::Own::new(),
            paused: None,
            subroutine: None,
            orbit: None,
            foe: None,
            boxes: None,
            died: false,
            overlay: None,
            helper_art: None,
            sleep: 0,
            walked: false,
            contact: None,
            ease: None,
            call: None,
            touchable: true,
            guard: (0, 0),
            walls: false,
            pending: (0, 0),
            line: None,
            spawned: false,
            parent: None,
            previous: None,
            place: ListPlace::After,
            id: 0,
            root: false,
            struck: false,
            root_died: false,
            blocked_tests: 0,
            ark: false,
            ark_list: None,
            ark_shown: None,
            linked: None,
            carried: None,
            parameter: 0,
            frozen_return: None,
            base: Base::Common,
            descriptor: None,
            motion: None,
            resources: [None, None, None],
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

    /// Builds the temporary actor installed by `COP DF`. Only the exact map
    /// `$21` guide invocation and frozen-return entry receive Ark's direct
    /// animation-resource profile; a changed target or profile freezes at the
    /// target entry instead of falling through generic service skipping.
    pub(crate) fn for_player(
        image: &[u8],
        map: u16,
        position: (u16, u16),
        script: Option<u32>,
        source: Option<usize>,
    ) -> Self {
        let mut actor = Self::new(position, script, 0, 1);
        actor.map = map;
        actor.ark = true;
        let candidate = map == 0x21 && source == cadence::frozen_return_guide(image);
        if candidate {
            let entry = cadence::frozen_return_start(image);
            if entry == Some(actor.pc) && cadence::frozen_return_profile(image) == entry {
                actor.frozen_return = entry;
                actor.legs = true;
            } else {
                actor.state = State::Frozen;
                actor.frozen_at = Some(actor.pc);
            }
        }
        actor
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
        // The shops' spawner ends at once; the world keeps its targets.
        if resident
            .script
            .is_some_and(|script| is_shop_spawner(image, script))
        {
            (actor.pc, actor.state) = (0, State::Gone);
        }
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
        let mode4 = resident
            .descriptor
            .and_then(|descriptor| image.get(descriptor + 3))
            == Some(&4);
        // An enemy: hittable (header `+$04` bit `$0200`), its descriptor's
        // byte 4 naming its profile (`$80:FACF`). The hit scans read the
        // header and the boxes, not the art: `$110`'s dart launchers, whose
        // packet the art decoder refuses, attack as well.
        let header = resident
            .script
            .and_then(|script| usize::try_from(script & 0x3F_FFFF).ok())
            .and_then(|script| image.get(script.checked_sub(4)?..script.checked_sub(2)?))
            .map_or(0, |word| u16::from_le_bytes([word[0], word[1]]));
        let hittable = header & 0x0200 != 0;
        actor.pose_ticks = resident
            .descriptor
            .filter(|_| resident.body || mode4 || hittable)
            .and_then(|descriptor| cadence::pose_ticks(image, descriptor));
        // Wall collision (`docs/enemy-scripts.md` §6) for enemies' bodies.
        actor.walls = resident.body && hittable && header & 0x0006 == 0x0004;
        actor.nested = header & 0x1000 != 0;
        actor.foe = resident
            .descriptor
            .filter(|_| hittable)
            .and_then(|descriptor| image.get(descriptor + 4))
            .and_then(|&index| crate::combat::profile(image, index))
            .map(|profile| {
                // A spawn parameter with bit 7 counts in `$0498` (`$80:F94C`).
                let counted = image
                    .get(resident.record + 3)
                    .is_some_and(|&byte| byte & 0x80 != 0);
                foe::Foe::new(profile, counted)
            });
        // `+$26` holds the spawn parameter; an enemy's set-up clears it
        // (`$80:F974`).
        if actor.foe.is_none() && actor.parameter != 0 {
            actor.own.insert(0x26, actor.parameter);
        }
        // A 16-byte record's fields (`$80:F56F`): `7F:1018` (word), `101A`,
        // `101C` (word) and `101E`, the Guardners' transfer index.
        let record = image.get(resident.record..resident.record + 16);
        if let Some(&[0 | 1, _, _, flags, .., a, b, c, d, e, f]) = record {
            if flags & 0xC0 == 0xC0 {
                let fields = [0x1018, 0x1019, 0x101A, 0x101C, 0x101D, 0x101E];
                actor.own.extend(fields.into_iter().zip([a, b, c, d, e, f]));
            }
        }
        if actor.foe.is_some() {
            // The header's `+$04` (`$80:F9xx`): hidden, out of the hit scan.
            actor.hidden |= header & 0x8000 != 0;
            actor.guard.0 = header & GUARD_04;
        }
        // Enemies' boxes for the hit scans; the towers' mode-`$04` objects'
        // for their push tests (`docs/tower-two.md`).
        if actor.foe.is_some() || mode4 {
            actor.boxes = resident
                .descriptor
                .and_then(|descriptor| cadence::pose_boxes(image, descriptor))
                .map(std::rc::Rc::new);
        }
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
        // The hit replaces whatever ran: a paused native run, a JSR.
        (self.paused, self.subroutine) = (None, None);
        self.state = State::Running;
        self.stream = None;
        self.motion = None;
        true
    }

    /// Runs one frame.
    pub fn tick(&mut self, around: &mut Surroundings<'_>) {
        let start = self.position;
        if !self.foe_frame(around.image, &mut around.globals.random) {
            self.frame(around);
        }
        self.settle(&around.layer());
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
        self.streamed = false;
        self.cooldown = self.cooldown.saturating_sub(1);
        self.pose_age = self.pose_age.saturating_add(1);
        self.wake_struck();
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
                if self.keeps_streams && self.state == State::Running && !self.streamed {
                    self.apply_stream();
                }
            }
        }
        if self.state == State::Frozen && self.frozen_at.is_none() {
            self.frozen_at = Some(self.pc);
        }
        self.sync_life();
    }

    /// Where the script stopped at something the interpreter does not model,
    /// for diagnostics.
    #[must_use]
    pub const fn frozen_at(&self) -> Option<usize> {
        self.frozen_at
    }

    /// Whether an admitted player-only pose movement currently owns Ark's
    /// displacement. The player actor may outlive this state at `COP BC`/`RTL`.
    #[must_use]
    pub(crate) fn admitted_player_motion_active(&self) -> bool {
        self.frozen_return.is_some() && self.motion.is_some()
    }

    /// Refuses a player displacement the world cannot qualify, retaining the
    /// script offset through the existing player diagnostic.
    pub(crate) fn refuse_player_motion(&mut self, position: (u16, u16)) {
        self.position = position;
        self.motion = None;
        self.stream = None;
        self.walking = false;
        self.state = State::Frozen;
        if self.frozen_at.is_none() {
            self.frozen_at = Some(self.pc);
        }
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
    /// - [`NO_DAMAGE`], [`PLAYER_POSE_TEST`], and runs on script scratch
    ///   words and the display ([`native::run`]).
    fn native_idiom(
        &mut self,
        image: &[u8],
        bank: usize,
        around: &mut Surroundings<'_>,
    ) -> Option<usize> {
        let at = self.pc;
        if let Some((set, cleared)) = image.get(at..).and_then(native::flags_04) {
            self.write_04(set, cleared);
            return Some(at + 9);
        }
        // `EOR #$8000` on `+$04`: a blink (the Cadet, `$97:C016`).
        if image.get(at..at + 9) == Some(&[0xBD, 0x04, 0x00, 0x49, 0x00, 0x80, 0x9D, 0x04, 0x00]) {
            self.hidden = !self.hidden;
            return Some(at + 9);
        }
        let layer_test = if image.get(at..at + 3) == Some(&LOAD_PLAYER_Y) {
            at + 3
        } else {
            at
        };
        if image.get(layer_test..layer_test + SAME_LAYER.len()) == Some(&SAME_LAYER) {
            return tested_branch(image, layer_test + SAME_LAYER.len(), true);
        }
        // `PHX; TYX; COP A7; PLX` with Y the child kept in `+$26` (the
        // block watchers, `$90:96CC`): the child goes.
        if image.get(at..at + 5) == Some(&[0xDA, 0xBB, 0x02, 0xA7, 0xFA]) {
            let id = |at| u16::from(self.own.get(&at).copied().unwrap_or(0));
            around.globals.deletions.push(id(0x26) | id(0x27) << 8);
            return Some(at + 5);
        }
        if image.get(at..at + OFF_SCREEN_TEST.len()) == Some(&OFF_SCREEN_TEST) {
            let on_screen = !self.off_screen(around.globals.view);
            return tested_branch(image, at + OFF_SCREEN_TEST.len(), on_screen);
        }
        // `+$06`'s interaction bits (`$0200` any side, `$0100` facing), as
        // the figure in `$21` sets them before registering its callback
        // (`$88:D33D`).
        if let Some(&[0xBD, 0x06, 0x00, op, low, high, 0x9D, 0x06, 0x00]) = image.get(at..at + 9) {
            const INTERACTION: u16 = INTERACT_ANY_SIDE | INTERACT_FACING;
            // Bits 11 to 14 order the draw ([`DEPTH_06`]); bit 6 keeps the
            // streams ([`KEEP_STREAMS`]).
            const ACCEPTED: u16 = INTERACTION | GUARD_06 | DEPTH_06 | KEEP_STREAMS;
            let value = u16::from_le_bytes([low, high]);
            match op {
                0x09 if value & !ACCEPTED == 0 => {
                    self.interaction |= value & INTERACTION;
                    self.guard.1 |= value & GUARD_06;
                    self.depth |= value & DEPTH_06;
                    self.keeps_streams |= value & KEEP_STREAMS != 0;
                }
                0x29 if !value & !(INTERACTION | GUARD_06 | DEPTH_06 | KEEP_STREAMS) == 0 => {
                    self.interaction &= value;
                    self.guard.1 &= value;
                    self.depth &= value;
                    self.keeps_streams &= value & KEEP_STREAMS != 0;
                }
                _ => return None,
            }
            return Some(at + 9);
        }
        // `LDA #0; STA $0004,X` / `$0006,X`: a cleared entity, as the
        // desk's book starts (`$88:D641`).
        if let Some(&[0xA9, 0, 0, 0x9D, field @ (4 | 6), 0]) = image.get(at..at + 6) {
            if field == 4 {
                (self.hidden, self.touchable, self.guard.0) = (false, false, 0);
            } else {
                (self.interaction, self.guard.1) = (0, 0);
            }
            return Some(at + 6);
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
        if let Some(next) = player_pose_mismatch(image, at) {
            return Some(next);
        }
        // The continents' door after its reload (`docs/underworld-end.md`
        // §2): the parchment's set-up (the picture's DMA, the palette, the
        // colour math) is not drawn here; the door, hidden, goes on at its
        // text.
        let parchment = assets::layout::per_revision(image, PARCHMENT[0], PARCHMENT[1]);
        if at == parchment.0 {
            return Some(parchment.1);
        }
        let player = (around.player, around.facing);
        let ran = native::run(image, at, &mut self.memory(around.globals, player))?;
        Some(self.ran(ran))
    }

    /// What a native run may change.
    fn memory<'m>(
        &'m mut self,
        globals: &'m mut Globals,
        (player, facing): ((u16, u16), Direction),
    ) -> native::Memory<'m> {
        let probe = probe(player);
        let published = globals.scratch.get(&ARK_FLAGS).copied().unwrap_or(0);
        let player = native::View {
            id: 0,
            flags: globals.ark_flags | published,
            word26: 0,
            index: 0,
            x: player.0,
            y: player.1,
            facing: u16::from(sense::code(facing)),
            previous: None,
        };
        // `+$14` reads as the facing of the record shown (`$97:B65A`).
        let facing = self.facing_code();
        self.own.insert(0x14, facing);
        self.own.insert(0x15, 0);
        native::Memory {
            words: &mut globals.scratch,
            own: &mut self.own,
            display: &mut globals.display,
            random: globals.random.word(),
            probe,
            events: &mut globals.events,
            sleep: &mut self.sleep,
            position: &mut self.position,
            player,
            parent: self.parent,
            linked: self.linked.take(),
            previous: self.previous,
            views: &globals.views,
            carried: &mut self.carried,
            pokes: &mut globals.pokes,
            bank: u8::try_from(self.pc >> 16 & 0x3F).unwrap_or(0) | 0x80,
        }
    }

    /// Where a native run goes on: past it, or, paused for a frame, at the
    /// script position it started from.
    fn ran(&mut self, ran: native::Ran) -> usize {
        match ran {
            native::Ran::Next(next) => next,
            native::Ran::Frame(paused) => {
                self.paused = Some(paused);
                self.pc
            }
        }
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

    /// The descriptor its art and boxes come from, its parent's for a
    /// spawned child.
    #[must_use]
    /// The helper art's list drawn in place of the body: an explosion, a
    /// gem, or a spawn's own pose in the helper art.
    pub(crate) fn shown_overlay(&self) -> Option<(u32, u8)> {
        self.overlay
            .or_else(|| self.helper_art.map(|helper| (helper, self.selector)))
    }

    pub(crate) const fn descriptor(&self) -> Option<usize> {
        self.descriptor
    }

    /// Where the registered interaction callback runs (normalized), if any.
    #[must_use]
    pub(crate) fn callback_at(&self) -> Option<usize> {
        Some((self.pc & 0xFF_0000) | usize::from(self.callback?))
    }

    /// Leaves the map (`COP A7`), as a pickup taken.
    pub(crate) fn remove(&mut self) {
        self.state = State::Gone;
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

    /// `COP B4`..`B9`: sets, clears or toggles the mirror or the vertical
    /// flip.
    fn flip(&mut self, service: u8) {
        match service {
            CLEAR_HFLIP => self.set_pose(self.selector, false),
            SET_HFLIP => self.set_pose(self.selector, true),
            TOGGLE_HFLIP => self.set_pose(self.selector, !self.hflip),
            SET_VFLIP => self.vflip = true,
            CLEAR_VFLIP => self.vflip = false,
            _ => self.vflip = !self.vflip,
        }
    }

    fn set_pose(&mut self, selector: u8, hflip: bool) {
        self.ark_list = None;
        self.ark_shown = None;
        if self.selector != selector || self.hflip != hflip {
            self.selector = selector;
            self.hflip = hflip;
            self.pose_age = 0;
        }
    }

    fn run(&mut self, around: &mut Surroundings<'_>) -> Run {
        let image = around.image;
        // A native run paused in a nested frame goes on first; a callback
        // runs its own code and leaves it paused.
        if let Some(paused) = self.paused.take_if(|_| self.outer.is_none()) {
            match native::resume(
                image,
                paused,
                &mut self.memory(around.globals, (around.player, around.facing)),
            ) {
                Some(ran) => self.pc = self.ran(ran),
                None => self.state = State::Frozen,
            }
            if self.paused.is_some() || self.state == State::Frozen {
                return Run::Yielded;
            }
        }
        // `+$0A` at entry: where an `RTL` comes back to next frame unless
        // `COP BC`/`C0` point it elsewhere first.
        let entry = self.pc;
        self.continuation = None;
        self.blocked_tests = 0;
        for _ in 0..BUDGET {
            // Per step: a long jump or call may have changed it.
            let bank = self.pc & 0xFF_0000;
            let Some(window) = image.get(self.pc..self.pc + 2) else {
                self.state = State::Frozen;
                return Run::Yielded;
            };
            if self.model_routine(around) {
                continue;
            }
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
                // The towers' shared push routines, evaluated here.
                0x20 if self.push_routine(bank, around).is_some() => continue,
                // JSR into the bank and its RTS, one level deep (the
                // freeze's crystals, `$88:B60F`).
                0x20 if self.subroutine.is_none() => {
                    let Some(&[low, high]) = image.get(self.pc + 1..self.pc + 3) else {
                        self.state = State::Frozen;
                        return Run::Yielded;
                    };
                    let target = u16::from_le_bytes([low, high]);
                    if target < 0x8000 {
                        self.state = State::Frozen;
                        return Run::Yielded;
                    }
                    self.subroutine = Some(self.pc + 3);
                    self.pc = bank | usize::from(target);
                    continue;
                }
                0x60 => {
                    let Some(caller) = self.subroutine.take() else {
                        self.state = State::Frozen;
                        return Run::Yielded;
                    };
                    self.pc = caller;
                    continue;
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
                    if let Some(next) = self.native_idiom(image, bank, around) {
                        self.pc = next;
                        if self.paused.is_some() {
                            return Run::Yielded;
                        }
                        continue;
                    }
                    self.state = State::Frozen;
                    return Run::Yielded;
                }
            }
            // Y holds a spawned child only up to the next service.
            self.linked = None;
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
            _ if SCRIPT_SERVICES.contains(&service) => {
                return self.script_service(service, operands, around)
            }
            EASE_START | EASE_STEP => return self.ease_service(service, operands, image),
            SWITCH | SPEED => return self.parameter_service(service, operands, bank, image),
            PLAYER_POSE_MOVING | ARK_POSE_REPEAT if self.ark => {
                return self.ark_pose(service, operands, image)
            }
            POSE_MOVING | POSE_SELECTOR | REPEAT_POSE_MOVING | REPEAT_MOVING => {
                return self.moving_pose(service, operands, image)
            }
            PLAY_TRACK | FADE_TO_TRACK | PLAY_SELECTION | SOUND_PORT3 | SOUND_PORT2
            | SOUND_WORD => return self.audio_service(service, operands, around),
            CALL => return self.call_service(operands, around),
            PLAYER_NEAR | PLAYER_AXIS | PLAYER_SIDE | PLAYER_HEIGHT | PLAYER_FRONT => {
                return self.player_test(service, operands, bank, around)
            }
            RANDOMIZE | SLEEP_OFF_SCREEN => return self.engine_service(service, operands, around),
            RETURN => self.pc = self.call.take().unwrap_or(operands),
            GIVE_ITEM | GRANT_ITEM | TAKE_ITEM | NO_ROOM => {
                return self.item_service(service, operands, bank, around)
            }
            PLACE | DELETE_ON_MAP | REPEAT_POSE | COUNT | YIELD => {
                return self.stage_service(service, operands, around)
            }
            TILE_BRANCH | PATCH => return self.tile_service(service, operands, bank, around),
            STAMP
            | UNSTAMP
            | SEAL
            | BLOCK
            | UNSEAL
            | PATCH_AT
            | HELD
            | BLOCKED_UP..=BLOCKED_RIGHT => {
                return self.cell_service(service, operands, bank, around)
            }
            HIT_TARGET | HIT_RETURN | COUNT_BRANCH | HELD_BRANCH | MUSIC_WAIT | 0x6A | 0x76 => {
                return self.door_service(service, operands, bank, around)
            }
            _ if SPAWNS.contains(&service) => return self.spawn(service, operands, around),
            WALK_TO_ROW | WALK_TO_COLUMN => return self.walk_toward(service, operands, image),
            SELECT_POSE => return self.select_pose(operands, image),
            LINE_START | LINE_STEP => return self.line_service(service, operands, image),
            PROFILE => return self.profile_service(operands, image),
            ARK_BUSY => return self.ark_busy(operands, bank, around),
            _ if FLIPS.contains(&service) => {
                self.flip(service);
                self.pc = operands;
            }
            WAIT => return self.wait_for_pose(operands),
            WAIT_STEP => return self.wait_step(operands),
            RANDOM_STEP => return self.random_step_service(operands, around),
            _ if PLAYER_SERVICES.contains(&service) => {
                return self.player_service(service, operands, bank, around)
            }
            BRANCH_ON_GLOBAL => self.pc = operands + 4,
            _ if BODY.contains(&service) => {
                return self.body(service, operands, image, around.player)
            }
            PAN | PAN_WAIT => return self.pan(service, operands, around),
            LOOP_START => return self.loop_start(operands, image),
            LOOP_END => return self.loop_end(operands),
            LOOP_AGAIN => {
                self.loop_end(operands);
                return true;
            }
            BRANCH_ON_MAP => return self.branch_on_map(operands, bank, image),
            TIMED_WAIT => return self.timed_wait(operands, image),
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
            LOAD_COLOURS | CIRCLE => return self.display_service(service, operands, around),
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

    /// `COP 5A`, the OBJ colours into [`crate::colours::ObjColours`], and
    /// `COP 63`, the circle window around this actor with its radius.
    /// Returns whether execution continues.
    fn display_service(
        &mut self,
        service: u8,
        operands: usize,
        around: &mut Surroundings<'_>,
    ) -> bool {
        self.cadence = None;
        let (done, length) = if service == LOAD_COLOURS {
            (around.globals.obj_colours.load(around.image, operands), 5)
        } else if let Some(&radius) = around.image.get(operands + 1) {
            around.globals.circle = Some(self.id);
            let words = &mut around.globals.scratch;
            words.insert(CIRCLE_RADIUS, u16::from(radius));
            (true, 2)
        } else {
            (false, 2)
        };
        if !done {
            self.state = State::Frozen;
            return false;
        }
        self.pc = operands + length;
        true
    }

    /// Ticks of the display list a selector names, when the packet is known.
    fn pose_list(&self, selector: u8) -> Option<u16> {
        if self.ark_list.is_some() {
            return self.ark_list;
        }
        *self.pose_ticks.as_ref()?.get(usize::from(selector))?
    }

    /// The pose of Ark's own art his script shows: once, or looped while
    /// `COP 89` repeats it ([`crate::world::ArkPose`]).
    pub(crate) fn ark_shown(&self) -> Option<crate::world::ArkPose> {
        let (resource, list, repeat) = self.ark_shown?;
        let age = u16::try_from(self.pose_age).unwrap_or(u16::MAX);
        Some(crate::world::ArkPose::new(
            resource, list, self.hflip, age, !repeat,
        ))
    }

    /// `COP 84 sel list resource` and `COP 89 count list sel resource` on
    /// Ark's own script outside the frozen return (`$80:A200`, `A28B`): the
    /// list of his resource, which `COP 8E` waits out, and `COP 8F` that
    /// many times (the Guardner's sleep, `$97:C5BB`), shown as his art
    /// ([`Self::ark_shown`]).
    fn ark_pose(&mut self, service: u8, operands: usize, image: &[u8]) -> bool {
        if self.frozen_return.is_some() {
            return if service == PLAYER_POSE_MOVING {
                self.frozen_return_pose(operands, image)
            } else {
                self.state = State::Frozen;
                false
            };
        }
        // `COP 84 list sel resource`; `COP 89` has the count (`+$22`) first.
        let repeat = service == ARK_POSE_REPEAT;
        let length = if repeat { 4 } else { 3 };
        let Some(bytes) = image.get(operands..operands + length) else {
            self.state = State::Frozen;
            return false;
        };
        let (count, list, resource) = if repeat {
            (Some(bytes[0]), bytes[1], bytes[3])
        } else {
            (None, bytes[0], bytes[2])
        };
        let Ok(records) = assets::sprites::boxes::ark_list(image, resource, list) else {
            self.state = State::Frozen;
            return false;
        };
        let ticks: u16 = records
            .iter()
            .map(|record| u16::from(record.duration) + 1)
            .sum();
        self.ark_list = Some(ticks.max(1));
        self.ark_shown = Some((resource, list, repeat));
        self.pose_age = 0;
        self.repeats = count.map(u16::from);
        self.pc = operands + length;
        true
    }

    /// The actor's body: its x and y (`COP B1`, `B2`, `B3`), its art packet (`COP D8`), its
    /// OBJ priority (`COP BA`), palette field (`COP BB`) and orbit (`COP D0`,
    /// `D1`). Returns whether execution continues.
    fn body(&mut self, service: u8, operands: usize, image: &[u8], player: (u16, u16)) -> bool {
        match service {
            ORBIT => return self.start_orbit(operands, image, player),
            ORBIT_STEP => return self.step_orbit(operands, image, player),
            MOVE_Y => {
                let Some(dy) = cadence::word(image, operands) else {
                    self.state = State::Frozen;
                    return false;
                };
                self.position.1 = self.position.1.wrapping_add(dy);
                self.pc = operands + 2;
            }
            MOVE_X | MOVE_XY => {
                let words = if service == MOVE_X { 1 } else { 2 };
                let (Some(dx), Some(dy)) = (
                    cadence::word(image, operands),
                    cadence::word(image, operands + 2)
                        .filter(|_| words == 2)
                        .or(Some(0)),
                ) else {
                    self.state = State::Frozen;
                    return false;
                };
                let dx = if self.hflip { dx.wrapping_neg() } else { dx };
                self.position = (
                    self.position.0.wrapping_add(dx),
                    self.position.1.wrapping_add(dy),
                );
                self.pc = operands + 2 * words;
            }
            SET_PACKET => {
                let Some(pointer) = image.get(operands..operands + 3) else {
                    self.state = State::Frozen;
                    return false;
                };
                if let Some(ticks) = cadence::packet_ticks(image, pointer) {
                    self.pose_ticks = Some(ticks);
                }
                // An enemy's or a bullet's boxes follow its art.
                if self.foe.is_some() || self.boxes.is_some() || self.spawned {
                    self.boxes = cadence::packet_boxes(image, pointer).map(std::rc::Rc::new);
                }
                let art = u32::from_le_bytes([pointer[0], pointer[1], pointer[2], 0]);
                let helper = foe::helper(image);
                self.helper_art = (self.spawned && art == helper).then_some(helper);
                self.pc = operands + 3;
            }
            OBJ_PRIORITY => {
                let Some(&byte) = image.get(operands) else {
                    self.state = State::Frozen;
                    return false;
                };
                self.priority = (byte >> 4) & 3;
                self.pc = operands + 1;
            }
            // +$08 = (+$08 & $F1FF) | operand << 8.
            PALETTE => {
                let Some(&byte) = image.get(operands) else {
                    self.state = State::Frozen;
                    return false;
                };
                self.palette = (byte >> 1) & 7;
                self.priority |= (byte >> 4) & 3;
                // From $40 it sets the flip bit too, which the timing
                // derivation does not follow.
                if byte >= 0x40 {
                    self.cadence = None;
                }
                self.pc = operands + 1;
            }
            _ => self.state = State::Frozen,
        }
        self.state != State::Frozen
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

    /// Admits only one of the five source-pinned `COP 84` instructions in the
    /// authenticated frozen-return stream.
    fn frozen_return_pose(&mut self, operands: usize, image: &[u8]) -> bool {
        let instruction = operands.saturating_sub(2);
        if cadence::frozen_return_selection(
            image,
            self.frozen_return.expect("checked frozen-return profile"),
            instruction,
        ) {
            return self.player_moving_pose(operands, image);
        }
        self.state = State::Frozen;
        if self.frozen_at.is_none() {
            self.frozen_at = Some(self.pc);
        }
        false
    }

    /// Player-only `COP 84 pose movement resource`: select one of Ark's
    /// direct pose tables and run a common movement selector through `COP 8E`.
    /// A resident reaching the same service remains on the generic skipped-
    /// service path; changed player resources freeze at their source offset.
    fn player_moving_pose(&mut self, operands: usize, image: &[u8]) -> bool {
        let Some(&[pose, selector, resource]) = image.get(operands..operands + 3) else {
            self.state = State::Frozen;
            return false;
        };
        let Some(ticks) = cadence::player_pose_ticks(image, resource) else {
            self.state = State::Frozen;
            return false;
        };
        self.pose_ticks = Some(ticks);
        let Some(list) = self.pose_list(pose) else {
            self.state = State::Frozen;
            return false;
        };
        let expected = match (pose, selector, resource) {
            (0x17, 0x0F, 1) => Some(36),
            (0x09, 0x1B, 0) => Some(16),
            (0 | 1, 0, 0) => Some(1),
            _ => None,
        };
        if expected.is_some_and(|expected| list != expected) {
            self.state = State::Frozen;
            return false;
        }
        self.base = Base::Common;
        let hflip = self.hflip;
        let Some(motion) = self
            .legs
            .then(|| self.movement(image))
            .flatten()
            .and_then(|movement| motion::Motion::start(movement, selector, false, Some(list)))
        else {
            self.state = State::Frozen;
            return false;
        };
        self.set_pose(pose, hflip);
        self.player_pose = Some((resource, pose));
        self.pose_age = 0;
        self.stream = None;
        self.motion = Some(motion);
        self.continuation = Some(operands + 3);
        self.pc = operands + 3;
        true
    }

    /// `COP 81 pose`, `82 pose selector`, `86 count pose` and `87 count
    /// pose selector`: a pose that moves by its movement streams through
    /// the next wait. Returns whether execution continues this frame.
    fn moving_pose(&mut self, service: u8, operands: usize, image: &[u8]) -> bool {
        let length = match service {
            POSE_MOVING => 1,
            REPEAT_MOVING => 3,
            _ => 2,
        };
        let (pose, selector) = match (service, image.get(operands..operands + length)) {
            (_, Some(&[pose])) => (pose, pose),
            (POSE_SELECTOR, Some(&[pose, selector])) => (pose, selector),
            (_, Some(&[count, pose])) => {
                self.repeats = Some(u16::from(count));
                (pose, pose)
            }
            (_, Some(&[count, pose, selector])) => {
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
            motion::Motion::start(resource, selector, self.interaction & 0x80 != 0, list)
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
            Base::Rom(bank, table) => {
                if self.resources[2].is_none() {
                    let bytes = image.get(bank..(bank + 0x1_0000).min(image.len()))?;
                    self.resources[2] = Some(motion::Resource::rom(bytes.into(), table));
                }
                return self.resources[2].clone();
            }
            Base::Unknown => return None,
        };
        if self.resources[slot].is_none() {
            let (source, size) = if slot == 0 {
                (assets::layout::offset(image, COMMON_SOURCE)?, COMMON_SIZE)
            } else {
                let pointer = image.get(self.descriptor? + 5..self.descriptor? + 8)?;
                (assets::maps::actors::rom_offset(pointer)?, 0x2000)
            };
            let packet = assets::compression::decode(image.get(source..)?, size).ok()?;
            self.resources[slot] = Some(motion::Resource::wram(
                if slot == 0 { 0x6000 } else { 0x4000 },
                packet.data.into(),
            ));
        }
        self.resources[slot].clone()
    }

    /// A `JSR` at the script position into one of the towers' push
    /// routines ([`push::routine`]), run here, and the `BCS`/`BCC` after it
    /// on its carry. `None` when it is not one.
    fn push_routine(&mut self, bank: usize, around: &mut Surroundings<'_>) -> Option<()> {
        let image = around.image;
        let &[0x20, low, high] = image.get(self.pc..self.pc + 3)? else {
            return None;
        };
        let routine = push::routine(image, bank | usize::from(u16::from_le_bytes([low, high])))?;
        let busy = around.globals.ark_busy
            || around
                .globals
                .scratch
                .get(&native::PLAYER_ACTION)
                .is_some_and(|&action| action != 0);
        let carry = match routine {
            push::Routine::Test(direction) => {
                let shape = self
                    .boxes
                    .as_ref()
                    .and_then(|boxes| boxes.get(usize::from(self.selector))?.first())
                    .map_or([-8, 16, -16, 16], |record| record.sprite);
                let ark = (
                    around.player,
                    sense::code(around.facing),
                    around.globals.pad,
                );
                !busy && push::pushes(direction, ark, self.position, shape)
            }
            push::Routine::Take => {
                around.globals.input_mask |= PAD_HELD;
                false
            }
            push::Routine::Give => {
                if !busy {
                    around.globals.input_mask &= !PAD_HELD;
                }
                busy
            }
        };
        self.pc += 3;
        if let Some(&[branch @ (0x90 | 0xB0), offset]) = image.get(self.pc..self.pc + 2) {
            self.pc += 2;
            if (branch == 0xB0) == carry {
                self.pc = self
                    .pc
                    .wrapping_add_signed(isize::from(i8::from_ne_bytes([offset])));
            }
        }
        Some(())
    }

    /// `COP 80 pose`. Returns whether execution continues this frame.
    fn select_pose(&mut self, operands: usize, image: &[u8]) -> bool {
        let Some(selector) = image.get(operands).copied() else {
            self.state = State::Frozen;
            return false;
        };
        let hflip = self.hflip;
        self.set_pose(selector, hflip);
        // `$80:A18C` restarts the list even for the same pose. Only where
        // its length is known, or a one-frame fallback wait would hold the
        // raster on its first frame.
        if self.pose_list(selector).is_some() {
            self.pose_age = 0;
        }
        // `$80:A1A8` writes `+$0A`: an RTL comes back here.
        self.continuation = Some(operands + 1);
        self.pc = operands + 1;
        true
    }

    /// `COP 71 m1 m2 t`: jumps when Ark is busy or down, or `$097E & m1`,
    /// or `$097C & m2`; else goes on. Returns whether execution continues
    /// this frame.
    fn ark_busy(&mut self, operands: usize, bank: usize, around: &Surroundings<'_>) -> bool {
        let image = around.image;
        let (Some(m1), Some(m2), Some(target)) = (
            cadence::word(image, operands),
            cadence::word(image, operands + 2),
            cadence::word(image, operands + 4),
        ) else {
            self.state = State::Frozen;
            return false;
        };
        let word = |at| around.globals.scratch.get(&at).copied().unwrap_or(0);
        let (gates, action) = (word(native::ARK_GATES), word(native::PLAYER_ACTION));
        if around.globals.ark_busy || gates & m1 != 0 || action & m2 != 0 {
            return self.jump(bank, target);
        }
        self.pc = operands + 6;
        true
    }

    /// `COP D9 n`: the profile, and without bit 7 the life too; a spawned
    /// child (a bullet) becomes an enemy that counts for no room.
    fn profile_service(&mut self, operands: usize, image: &[u8]) -> bool {
        let Some(&index) = image.get(operands) else {
            self.state = State::Frozen;
            return false;
        };
        if let Some(profile) = crate::combat::profile(image, index) {
            match &mut self.foe {
                Some(foe) => {
                    foe.profile = profile;
                    if index & 0x80 == 0 {
                        foe.life = profile.life;
                    }
                }
                // A bullet's: it strikes Ark; it counts for no room.
                None if self.spawned => self.foe = Some(foe::Foe::new(profile, false)),
                None => {}
            }
        }
        self.pc = operands + 1;
        true
    }

    /// `COP CC a p c d s` and `COP CD` (`docs/enemy-scripts.md` §6): the
    /// target is the script's `$7F:2004/2006,X`; `CC` goes on in the same
    /// frame, `CD` steps once a frame and goes on the frame after the end.
    /// A selector besides `FF`, which no chapter 1 script uses, freezes.
    fn line_service(&mut self, service: u8, operands: usize, image: &[u8]) -> bool {
        if service == LINE_STEP {
            // A knockback ends the move (`$80:B04C`).
            let Some(line) = &mut self.line else {
                self.pc = operands;
                return false;
            };
            match line.frame() {
                line::Frame::Moving(dx, dy) => self.displace((dx, dy)),
                line::Frame::Ended(dx, dy) => {
                    self.displace((dx, dy));
                    self.line = None;
                    self.continuation = Some(operands);
                    self.pc = operands;
                }
            }
            return false;
        }
        let Some(&[legs, pose, speed, limit, 0xFF]) = image.get(operands..operands + 5) else {
            self.state = State::Frozen;
            return false;
        };
        let word = |at: u16| {
            u16::from_le_bytes([
                self.own.get(&at).copied().unwrap_or(0),
                self.own.get(&(at + 1)).copied().unwrap_or(0),
            ])
        };
        let target = (word(0x2004), word(0x2006));
        self.line = Some(line::Line::start(self.position, target, legs, speed, limit));
        let hflip = self.hflip;
        self.set_pose(pose, hflip);
        self.pose_age = 0;
        (self.motion, self.stream) = (None, None);
        self.continuation = Some(operands + 5);
        self.pc = operands + 5;
        true
    }

    /// Moves by `delta`; an actor that collides with walls keeps it for
    /// the frame's end ([`Self::settle`]), as `7F:0018`/`001A` wait for
    /// `$80:D0CF`.
    fn displace(&mut self, (dx, dy): (i16, i16)) {
        if self.walls {
            self.pending = (
                self.pending.0.wrapping_add(dx),
                self.pending.1.wrapping_add(dy),
            );
        } else {
            self.position = (
                self.position.0.wrapping_add_signed(dx),
                self.position.1.wrapping_add_signed(dy),
            );
        }
    }

    /// The frame's pending move against the walls (`$80:D101`): the box is
    /// the first record's of the pose, a blocked axis is clamped and its
    /// stream stops.
    fn settle(&mut self, layer: &walls::Layer<'_>) {
        let delta = std::mem::take(&mut self.pending);
        if delta == (0, 0) {
            return;
        }
        let shape = self
            .boxes
            .as_ref()
            .and_then(|boxes| boxes.get(usize::from(self.selector))?.first())
            .map(|record| record.sprite);
        let Some(shape) = shape else {
            self.position = (
                self.position.0.wrapping_add_signed(delta.0),
                self.position.1.wrapping_add_signed(delta.1),
            );
            return;
        };
        let (position, blocked) = walls::step(self.position, delta, shape, layer);
        self.position = position;
        if let Some(motion) = &mut self.motion {
            motion.stop(blocked);
        }
        if blocked.0 || blocked.1 {
            self.stream = None;
        }
    }

    /// One frame of a scripted leg or a pose's movement; cleared once the
    /// pose wait is over.
    fn apply_stream(&mut self) {
        self.streamed = true;
        let (dx, dy) = if let Some(motion) = &mut self.motion {
            motion.step((self.hflip, self.vflip)).unwrap_or((0, 0))
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
        self.displace((dx, dy));
        self.walking = self.stream.is_some() || (dx, dy) != (0, 0);
        let ends = !self.keeps_streams
            && match self.state {
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
            MUSIC_WAIT => return self.hold(operands, 3),
            cosmetic => {
                let Some(&(_, length)) = COSMETIC.iter().find(|&&(service, _)| service == cosmetic)
                else {
                    self.state = State::Frozen;
                    return false;
                };
                if let Some(&[first, second]) = around.image.get(operands..operands + 2) {
                    let display = &mut around.globals.display;
                    match cosmetic {
                        PPU_WRITE => {
                            display.write(0x2100 | u16::from(first), second);
                        }
                        SPIN if first == 0 => display.spin(self.position, second),
                        _ => {}
                    }
                }
                self.pc = operands + length;
            }
        }
        true
    }

    /// `COP D0 pose target dx dy angle radius turn reach end`: an orbit
    /// about the actor's own place (where its parent spawned it; target 0)
    /// or Ark (`$0DEA`) plus (dx, dy). Natively `COP D1` reads the parent's place each frame;
    /// here it is fixed when the orbit starts, as the crystals' Elle stands
    /// still. Returns whether execution continues.
    fn start_orbit(&mut self, operands: usize, image: &[u8], player: (u16, u16)) -> bool {
        let word = |at: usize| cadence::word(image, operands + at);
        let byte = |at: usize| {
            image
                .get(operands + at)
                .map(|&b| i16::from(i8::from_ne_bytes([b])))
        };
        let (
            Some(pose),
            Some(target @ (0 | 0x0DEA)),
            Some(dx),
            Some(dy),
            Some(angle),
            Some(radius),
        ) = (word(0), word(2), word(4), word(6), word(8), word(10))
        else {
            self.state = State::Frozen;
            return false;
        };
        let (Some(turn), Some(reach), Some(end)) = (byte(12), byte(13), word(14)) else {
            self.state = State::Frozen;
            return false;
        };
        let about_player = (target == 0x0DEA).then_some((dx, dy));
        let base = if about_player.is_some() {
            player
        } else {
            self.position
        };
        let centre = (base.0.wrapping_add(dx), base.1.wrapping_add(dy));
        self.set_pose(u8::try_from(pose).unwrap_or(0), self.hflip);
        self.orbit = Some(Orbit {
            centre,
            about_player,
            angle,
            radius,
            turn: turn * 2,
            reach,
            end,
        });
        self.pc = operands + 16;
        true
    }

    /// `COP D1`: the orbit's frame. It ends the frame until the orbit ends
    /// (its count, or its radius reached), then goes on.
    fn step_orbit(&mut self, operands: usize, image: &[u8], player: (u16, u16)) -> bool {
        let Some(mut orbit) = self.orbit else {
            self.state = State::Frozen;
            return false;
        };
        if let Some((dx, dy)) = orbit.about_player {
            orbit.centre = (player.0.wrapping_add(dx), player.1.wrapping_add(dy));
        }
        orbit.angle = orbit.angle.wrapping_add_signed(orbit.turn) & 0x3FF;
        orbit.radius = orbit.radius.wrapping_add_signed(orbit.reach) & 0x1FF;
        let Some(position) = orbit.position(image) else {
            self.state = State::Frozen;
            return false;
        };
        self.position = position;
        let done = if orbit.end & 0x8000 == 0 {
            orbit.end = orbit.end.wrapping_sub(1);
            orbit.end == 0
        } else {
            let target = orbit.end & 0x1FF;
            if orbit.reach >= 0 {
                orbit.radius >= target
            } else {
                target >= orbit.radius
            }
        };
        self.orbit = (!done).then_some(orbit);
        self.pc = if done { operands } else { operands - 2 };
        done
    }

    /// `COP 25` (a random step) and `COP 59 n` (sleep while off screen).
    /// Returns whether execution continues.
    fn engine_service(
        &mut self,
        service: u8,
        operands: usize,
        around: &mut Surroundings<'_>,
    ) -> bool {
        if service == RANDOMIZE {
            around.globals.random.step();
            self.pc = operands;
            return true;
        }
        let Some(&ticks) = around.image.get(operands) else {
            self.state = State::Frozen;
            return false;
        };
        if self.off_screen(around.globals.view) {
            self.pc = operands - 2;
            self.state = State::Waiting(u16::from(ticks));
            return false;
        }
        self.pc = operands + 1;
        true
    }

    /// Whether the actor is off the screen `view` (`$80:E9A2`, by the
    /// sprite box; here a 32x32 box standing on the actor's place).
    fn off_screen(&self, view: Option<(u16, u16, u16, u16)>) -> bool {
        let Some((left, top, right, bottom)) = view else {
            return false;
        };
        let (x, y) = self.position;
        x.saturating_add(16) < left
            || x.saturating_sub(16) >= right
            || y < top
            || y.saturating_sub(32) >= bottom
    }

    /// `COP D6 d t`, `D7 h v`, `D3 w l e r` and `D4 w u e d`: branches on
    /// Ark's place. Returns whether execution continues.
    fn player_test(
        &mut self,
        service: u8,
        operands: usize,
        bank: usize,
        around: &Surroundings<'_>,
    ) -> bool {
        let image = around.image;
        let word = |at: usize| cadence::word(image, operands + at);
        let (x, y) = (i32::from(self.position.0), i32::from(self.position.1));
        let probe = (i32::from(around.player.0), i32::from(around.player.1) - 8);
        let (dx, dy) = (probe.0 - x, probe.1 - y);
        let target = match service {
            PLAYER_FRONT => {
                let (Some(&[w, l, mode]), Some(target)) =
                    (image.get(operands..operands + 3), word(3))
                else {
                    self.state = State::Frozen;
                    return false;
                };
                let facing = self.facing_code();
                let (left, top, right, bottom) = sense::front(facing, self.position, w, l);
                let inside = (left..=right).contains(&probe.0) && (top..=bottom).contains(&probe.1);
                if !inside || !sense::admits(mode, facing, sense::code(around.facing)) {
                    self.pc = operands + 5;
                    return true;
                }
                Some(target)
            }
            PLAYER_NEAR => {
                let (Some(&distance), Some(target)) = (image.get(operands), word(1)) else {
                    self.state = State::Frozen;
                    return false;
                };
                let distance = i32::from(distance);
                if dx.abs() > distance || dy.abs() > distance {
                    self.pc = operands + 3;
                    return true;
                }
                Some(target)
            }
            PLAYER_AXIS => {
                if dy.abs() >= dx.abs() {
                    word(2)
                } else {
                    word(0)
                }
            }
            _ => {
                // `D4` measures from Ark's feet: the probe plus 8.
                let offset = if service == PLAYER_SIDE { dx } else { dy + 8 };
                let Some(zone) = word(0).map(i32::from) else {
                    self.state = State::Frozen;
                    return false;
                };
                match offset {
                    0 => word(4),
                    offset if offset < 0 && -offset > zone => word(2),
                    offset if offset > 0 && offset > zone => word(6),
                    _ => word(4),
                }
            }
        };
        let Some(target) = target else {
            self.state = State::Frozen;
            return false;
        };
        self.jump(bank, target)
    }

    /// `COP 00`: a long call, keeping one return; the desk's save screen
    /// blocks for the world instead. Returns whether execution continues.
    fn call_service(&mut self, operands: usize, around: &mut Surroundings<'_>) -> bool {
        let Some(target) = around.image.get(operands..operands + 3).and_then(long) else {
            self.state = State::Frozen;
            return false;
        };
        if target == RECORDS {
            for flag in RECORDS_CLEARS {
                around.globals.write_flag(flag);
            }
            self.pc = operands + 3;
            self.state = State::Blocked(Wait::Records);
            return false;
        }
        self.call = Some(operands + 3);
        self.pc = target;
        true
    }

    /// `COP DD new relative sx sy tx ty speed` (a new camera, offsets from
    /// the player, at the towers' speed only) and `COP DE speed`, which
    /// starts the move and ends the frame until it arrives; its speed
    /// operand is not read. Returns whether execution continues.
    fn pan(&mut self, service: u8, operands: usize, around: &mut Surroundings<'_>) -> bool {
        let pan = &mut around.globals.pan;
        let units = |byte: u8| i16::from(i8::from_ne_bytes([byte])) * 16;
        match (service, around.image.get(operands..operands + 7), *pan) {
            (PAN, Some(&[0, 0, sx, sy, tx, ty, PAN_SPEED]), _) => {
                *pan = Some(crate::scene::Pan {
                    offset: (units(sx), units(sy)),
                    target: (units(tx), units(ty)),
                    moving: false,
                });
                self.pc = operands + 7;
                true
            }
            (PAN_WAIT, _, Some(started)) if started.arrived() && started.moving => {
                *pan = None;
                self.pc = operands + 1;
                true
            }
            (PAN_WAIT, _, Some(mut started)) => {
                started.moving = true;
                *pan = Some(started);
                self.pc = operands - 2;
                false
            }
            // Other moves are stepped over, as before they were modelled.
            (PAN, ..) | (PAN_WAIT, _, None) => {
                self.cadence = None;
                self.pc = operands + if service == PAN { 7 } else { 1 };
                true
            }
            _ => {
                self.state = State::Frozen;
                false
            }
        }
    }

    /// `COP A2` and `COP 99` (a script and a flags word, at the actor),
    /// `COP A1` (a script at the actor), `COP 9C` and `A4` (a script at an
    /// offset), the group spawns (`E6`-`E8`) and the group's deletion
    /// (`EB`). Returns whether execution continues.
    fn spawn(&mut self, service: u8, operands: usize, around: &mut Surroundings<'_>) -> bool {
        if service == DELETE_GROUP {
            if self.root {
                let spawned = around.globals.spawns.len();
                around.globals.group_deletions.push((self.id, spawned));
            }
            self.pc = operands;
            return true;
        }
        let image = around.image;
        let script = image.get(operands..operands + 3).and_then(long);
        let word = |at: usize| cadence::word(image, operands + at);
        let grouped = GROUP_SPAWNS.iter().find(|&&(group, _)| group == service);
        let copied = (self.flags_04() | 0x8000) & !0x1000;
        let kind = grouped.map_or(service, |&(_, as_)| as_);
        let (script, flags, offset, length) = match kind {
            SPAWN | SPAWN_LINKED | SPAWN_BEFORE_FLAGS | SPAWN_LAST => {
                (script, word(3), Some((0, 0)), 5)
            }
            // No flags word: the child keeps the parent's `+$04`, hidden
            // and out of the nested frame (`$80:BCD2`: `ORA #$8000`, `AND
            // #$EFFF`).
            SPAWN_AFTER | SPAWN_BEFORE => (script, Some(copied), Some((0, 0)), 3),
            SPAWN_AT => (script, Some(copied), word(3).zip(word(5)), 7),
            _ => (script, word(7), word(3).zip(word(5)), 9),
        };
        let (Some(script), Some(flags), Some((dx, dy))) = (script, flags, offset) else {
            self.state = State::Frozen;
            return false;
        };
        let dx = if self.hflip { dx.wrapping_neg() } else { dx };
        let at = (
            self.position.0.wrapping_add(dx),
            self.position.1.wrapping_add(dy),
        );
        let mut child = self.child(script, flags, at);
        child.place = match kind {
            SPAWN_BEFORE | SPAWN_BEFORE_FLAGS => ListPlace::Before,
            SPAWN_LINKED => ListPlace::Head,
            SPAWN_LAST => ListPlace::Last,
            _ => ListPlace::After,
        };
        child.id = around.globals.next_id;
        around.globals.next_id = around.globals.next_id.wrapping_add(1).max(0x4000);
        if grouped.is_some() {
            let root = self.group_of_spawns();
            self.root |= root == self.id;
            child.set_own_word(GROUP, root);
            // A hittable group child (`+$04` bit `$0200`) is an enemy of its
            // descriptor's profile, as a record is: the High Cadet's copies.
            if flags & 0x0200 != 0 {
                child.arm(image);
            }
        } else if flags & 0x0200 != 0 {
            // An attacker of its parent's profile (`7F:1022`, `$80:BCC9`):
            // Shadowkeeper's shots and head.
            if let Some(foe) = &self.foe {
                child.foe = Some(foe::Foe::new(foe.profile, false));
            }
        }
        // Y holds the child after the service (`$97:C6AD`).
        self.linked = Some(child.id);
        // The child is an entity at once: runs this frame see it.
        around.globals.views.push((child.id, child.view()));
        around.globals.spawns.push((script, child));
        self.pc = operands + length;
        true
    }

    /// Makes a spawned child an enemy of its descriptor's profile (byte 4),
    /// counted in no room, with the descriptor's boxes.
    fn arm(&mut self, image: &[u8]) {
        let Some(descriptor) = self.descriptor else {
            return;
        };
        let Some(profile) = image
            .get(descriptor + 4)
            .and_then(|&index| crate::combat::profile(image, index))
        else {
            return;
        };
        self.foe = Some(foe::Foe::new(profile, false));
        if self.boxes.is_none() {
            self.boxes = cadence::pose_boxes(image, descriptor).map(std::rc::Rc::new);
        }
    }

    /// An enemy's life, which its script keeps in `7F:102A` (the High
    /// Cadet's copies, `$97:C9D1`): a write there sets it; a hit writes it
    /// back ([`Self::mirror_life`]).
    fn sync_life(&mut self) {
        if !self.own.contains_key(&LIFE) {
            return;
        }
        let life = self.own_word(LIFE);
        if let Some(foe) = &mut self.foe {
            foe.life = life;
        }
    }

    /// The life after a hit, where the script reads it.
    pub(crate) fn mirror_life(&mut self) {
        let life = self.foe.as_ref().map(|foe| foe.life);
        if let Some(life) = life.filter(|_| self.own.contains_key(&LIFE)) {
            self.set_own_word(LIFE, life);
        }
    }

    /// The struck wake (`7F:201E` bit `$0800`, `$80:C967`): the script goes
    /// on from `7F:1016`, in the bank of `7F:2020`, or its own.
    fn wake_struck(&mut self) {
        let target = self.own_word(STRUCK);
        let idle = matches!(self.state, State::Frozen | State::Gone | State::Blocked(_));
        if !std::mem::take(&mut self.struck) || target < 0x8000 || idle {
            return;
        }
        let bank = self
            .own
            .get(&0x2020)
            .map_or(self.pc & 0x3F_0000, |&bank| usize::from(bank & 0x3F) << 16);
        self.pc = bank | usize::from(target);
        self.state = State::Running;
        self.call = None;
        self.paused = None;
    }

    /// The group it is in: its root's id (`7F:102E`), if any.
    pub(crate) fn group(&self) -> Option<u16> {
        Some(self.own_word(GROUP)).filter(|&root| root != 0)
    }

    /// A word of its own bytes (0 where unwritten).
    fn own_word(&self, at: u16) -> u16 {
        let byte = |at| self.own.get(&at).copied().unwrap_or(0);
        u16::from_le_bytes([byte(at), byte(at + 1)])
    }

    fn set_own_word(&mut self, at: u16, value: u16) {
        let [low, high] = value.to_le_bytes();
        self.own.insert(at, low);
        self.own.insert(at + 1, high);
    }

    /// `+$04` as the runtime keeps it, for a spawn that copies it.
    fn flags_04(&self) -> u16 {
        u16::from(self.hidden) << 15
            | u16::from(self.nested) << 12
            | u16::from(self.touchable) << 9
            | if self.walls { WALLS_04 } else { 0 }
            | self.guard.0
    }

    /// The root its group spawns join (`$80:BB5A`): itself when it has no
    /// parent or already heads a group, else its own group's root.
    fn group_of_spawns(&self) -> u16 {
        if self.parent.is_none() || self.root {
            self.id
        } else {
            self.group().unwrap_or(self.id)
        }
    }

    /// `+$04` bits set and cleared, as far as they are modelled.
    fn write_04(&mut self, set: u16, cleared: u16) {
        self.hidden = (self.hidden || set & 0x8000 != 0) && cleared & 0x8000 == 0;
        self.touchable = (self.touchable || set & 0x0200 != 0) && cleared & 0x0200 == 0;
        self.guard.0 = (self.guard.0 | set & GUARD_04) & !cleared;
        self.walls = (self.walls || set & WALLS_04 != 0) && cleared & WALLS_04 == 0;
        self.nested = (self.nested || set & 0x1000 != 0) && cleared & 0x1000 == 0;
    }

    /// Whether it runs in the nested frame of a transfer's fade.
    pub(crate) const fn runs_nested(&self) -> bool {
        self.nested
    }

    /// The id of the parent it was spawned after, if any.
    pub(crate) fn parent_id(&self) -> Option<u16> {
        self.parent.map(|parent| parent.id).filter(|&id| id != 0)
    }

    /// Where its script is, an image offset.
    pub(crate) const fn script_place(&self) -> usize {
        self.pc
    }

    /// Takes a write another actor's run made into it.
    pub(crate) fn take_poke(&mut self, poke: Poke) {
        match poke {
            Poke::Word { at, value, .. } => {
                self.set_own_word(at, value);
                self.sync_life();
            }
            Poke::Flags { set, cleared, .. } => self.write_04(set, cleared),
            Poke::Priority { priority, .. } => self.priority = priority,
            Poke::Place { at, .. } => self.position = at,
            // `+$0A` alone: a sleep (`+$0E`, a timed wait) stays.
            Poke::Script { pc, .. } => {
                if matches!(self.state, State::Gone | State::Frozen) {
                    return;
                }
                self.pc = pc;
                if !matches!(self.state, State::Waiting(_)) {
                    self.state = State::Running;
                }
                (self.subroutine, self.continuation, self.call) = (None, None, None);
            }
            // The world keeps Ark's.
            Poke::Ark { .. } | Poke::ArkAt { .. } | Poke::ArkPush { .. } => {}
        }
    }

    /// What other actors' runs read of it through `,Y`.
    pub(crate) fn view(&self) -> native::View {
        let out = self
            .foe
            .as_ref()
            .is_some_and(|foe| foe.exploding.is_some() || foe.dead);
        native::View {
            id: self.id,
            flags: u16::from(self.hidden) << 15 | u16::from(out) << 7,
            word26: self.own_word(0x26),
            index: self.own_word(0x101E) & 0xFF,
            x: self.position.0,
            y: self.position.1,
            facing: u16::from(self.facing_code()),
            previous: self.previous,
        }
    }

    /// A child as the spawns make it (`$80:BCA4`): the parent's mirror,
    /// palette, priority, art, boxes and movement base, `+$04` from `flags` (bit 15
    /// hidden, `$0006` walls).
    fn child(&self, script: usize, flags: u16, at: (u16, u16)) -> Self {
        let runtime = u32::try_from(script).map_or(0, |script| 0x80_0000 | script);
        let mut child = Self::new(at, Some(runtime), 0, 1);
        (child.hflip, child.vflip) = (self.hflip, self.vflip);
        child.palette = self.palette;
        child.priority = self.priority;
        child.base = self.base;
        child.resources.clone_from(&self.resources);
        child.descriptor = self.descriptor;
        child.boxes.clone_from(&self.boxes);
        child.pose_ticks.clone_from(&self.pose_ticks);
        child.legs = self.legs;
        child.hidden = flags & 0x8000 != 0;
        child.nested = flags & 0x1000 != 0;
        child.walls = flags & 0x0006 == 0x0004;
        child.guard.0 = flags & GUARD_04;
        child.spawned = true;
        child.parent = Some(self.view());
        child
    }

    /// `COP 3D`/`3E` mark and unmark a cell beside the actor; `COP 3F` sets
    /// one's collision attribute; `COP 46` copies cells. Returns whether
    /// execution continues.
    fn cell_service(
        &mut self,
        service: u8,
        operands: usize,
        bank: usize,
        around: &mut Surroundings<'_>,
    ) -> bool {
        let image = around.image;
        match service {
            BLOCK => return self.block_service(operands, around),
            UNSEAL => {
                // `$80:C012`: the cells under the box, as `COP 3F` set them.
                let Some(&[mode, dx, dy]) = image.get(operands..operands + 3) else {
                    self.state = State::Frozen;
                    return false;
                };
                let signed = |by: u8| i16::from(i8::from_ne_bytes([by])) * 16;
                let base = if mode == 0 { self.position } else { (0, 0) };
                let at = (
                    base.0.wrapping_add_signed(signed(dx)),
                    base.1.wrapping_add_signed(signed(dy)),
                );
                for cell in self.cells_under(at) {
                    self.stamps.retain(|&stamped| stamped != cell);
                }
                self.pc = operands + 3;
            }
            PATCH_AT => {
                let Some(&[column, row, low, high]) = image.get(operands..operands + 4) else {
                    self.state = State::Frozen;
                    return false;
                };
                let tile = u16::from_le_bytes([low, high]) & 0x1FF;
                around
                    .globals
                    .patches
                    .push((u16::from(column), u16::from(row), tile));
                self.pc = operands + 4;
            }
            HELD => {
                let (Some(mask), Some(target)) = (
                    cadence::word(image, operands),
                    cadence::word(image, operands + 2),
                ) else {
                    self.state = State::Frozen;
                    return false;
                };
                // Bit 0 asks for a tower floor instead (`$80:902D`).
                let floor = mask & 1 == 0 || around.globals.tower_floor();
                let mask = mask & !1;
                if floor && around.globals.pad & mask == mask {
                    return self.jump(bank, target);
                }
                self.pc = operands + 4;
            }
            BLOCKED_UP..=BLOCKED_RIGHT => {
                return self.blocked_test(service, operands, bank, around)
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
            _ => {
                let Some(&[mode, dx, dy]) = image.get(operands..operands + 3) else {
                    self.state = State::Frozen;
                    return false;
                };
                let signed = |by: u8| i16::from(i8::from_ne_bytes([by])) * 16;
                let base = if mode & 0x80 == 0 {
                    self.position
                } else {
                    (0, 0)
                };
                let at = (
                    base.0.wrapping_add_signed(signed(dx)),
                    base.1.wrapping_add_signed(signed(dy)),
                );
                let attribute = u16::from(mode & 0x3F);
                for cell in self.cells_under(at) {
                    self.stamps.retain(|&stamped| stamped != cell);
                    if matches!(attribute & 0x1F, 0 | 1 | 2 | 8 | 17 | 20 | 22) {
                        // Floors, pits and lips (the Hole's rim, `$90:808C`)
                        // the walker knows go in the map.
                        around.globals.attributes.push((cell.0, cell.1, attribute));
                    } else {
                        self.stamps.push(cell);
                    }
                }
                self.pc = operands + 3;
            }
        }
        true
    }

    /// `COP C2`-`C5`: jumps when the cells beyond the box that way stop the
    /// actor. Returns whether execution continues this frame.
    fn blocked_test(
        &mut self,
        service: u8,
        operands: usize,
        bank: usize,
        around: &Surroundings<'_>,
    ) -> bool {
        let Some(target) = cadence::word(around.image, operands) else {
            self.state = State::Frozen;
            return false;
        };
        if !self.blocked_beyond(service - BLOCKED_UP, around) {
            self.pc = operands + 2;
            return true;
        }
        // `$0868` bit 7, a world map, probes the plane (`$80:C164`) instead;
        // the room's cells stand in (`meta/issues/partial-cop-services.md`).
        // Blocked every way, a wall-follower would test round and round
        // without a yield, which hangs the game natively: it waits a frame.
        self.blocked_tests += 1;
        self.blocked_tests <= 4 && self.jump(bank, target)
    }

    /// The box (x offset, width, y offset, height) of the pose shown, from
    /// its first record (`7F:0028..002E`); a body's 16 pixels without one.
    fn box_shape(&self) -> [i8; 4] {
        self.boxes
            .as_ref()
            .and_then(|boxes| boxes.get(usize::from(self.selector))?.first())
            .map_or([-8, 16, -16, 16], |record| record.sprite)
    }

    /// The cells `COP 3F` sets for a body at `at` (`$80:BF8E`): a box of
    /// fewer than three cells across and down marks the one under (x - 8,
    /// y - 16); a larger one every cell under the box (`$80:BFD3`, tower 2's
    /// 32-pixel blocks).
    fn cells_under(&self, at: (u16, u16)) -> Vec<(u16, u16)> {
        let [bx, bw, by, bh] = self.box_shape().map(i16::from);
        let (columns, rows) = (bw.max(0) >> 4, bh.max(0) >> 4);
        if columns + rows < 3 {
            return vec![(at.0.wrapping_sub(8) / 16, at.1.wrapping_sub(16) / 16)];
        }
        let (left, top) = (
            at.0.wrapping_add_signed(bx) / 16,
            at.1.wrapping_add_signed(by) / 16,
        );
        (0..rows.cast_unsigned())
            .flat_map(|row| {
                (0..columns.cast_unsigned()).map(move |column| (left + column, top + row))
            })
            .collect()
    }

    /// Whether the cells beyond the actor's box toward `direction` (0 Up,
    /// 1 Down, 2 Left, 3 Right) stop a blocked test ([`walls::beyond`],
    /// [`walls::stops_a_test`]): off the map, or a body on them, the bit-15
    /// mark the runtime keeps as occupied cells.
    fn blocked_beyond(&self, direction: u8, around: &Surroundings<'_>) -> bool {
        let layer = around.layer();
        walls::beyond(direction, self.position, self.box_shape())
            .into_iter()
            .any(|(column, row)| {
                layer.cell(column, row).is_none_or(walls::stops_a_test)
                    || around
                        .occupied
                        .iter()
                        .any(|&(c, r)| (i32::from(c), i32::from(r)) == (column, row))
            })
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

    /// `COP 46 n lim sx sy dx dy wait`: while the row offset `r` (own
    /// `$7F:201A`) is at most `lim`, copies row `r / 16` of the `n + 1` cells
    /// from (sx, sy) to (dx, dy), adds 16 and yields; then sleeps `wait` at
    /// the next yield and goes on. Layer 0 (own `$7F:201B`) is the first,
    /// with its collision; layers 1 to `$7F`, the second, whose copies the
    /// world resolves (picture only). Returns whether execution continues.
    fn block_service(&mut self, operands: usize, around: &mut Surroundings<'_>) -> bool {
        let Some(&[n, limit, sx, sy, dx, dy, wait]) = around.image.get(operands..operands + 7)
        else {
            self.state = State::Frozen;
            return false;
        };
        let row = self.own.get(&0x201A).copied().unwrap_or(0);
        if limit < row {
            self.sleep = u16::from(wait);
            self.pc = operands + 7;
            return true;
        }
        let layer = self.own.get(&0x201B).copied().unwrap_or(0);
        if (1..0x80).contains(&layer) {
            let line = u16::from(row / 16);
            around
                .globals
                .second_copies
                .extend((0..=u16::from(n)).map(|i| {
                    (
                        (u16::from(sx) + i, u16::from(sy) + line),
                        (u16::from(dx) + i, u16::from(dy) + line),
                    )
                }));
        }
        if layer == 0 && around.width > 0 {
            let width = around.width;
            for i in 0..=u16::from(n) {
                let from = (
                    (u16::from(sx) + i) % width,
                    u16::from(sy) + u16::from(row / 16),
                );
                let at = usize::from(from.1) * usize::from(width) + usize::from(from.0);
                let Some(&cell) = around.cells.get(at) else {
                    continue;
                };
                let to = (
                    (u16::from(dx) + i) % width,
                    u16::from(dy) + u16::from(row / 16),
                );
                around.globals.patches.push((to.0, to.1, cell & 0x1FF));
            }
        }
        self.own.insert(0x201A, row.wrapping_add(16));
        self.pc = operands - 2;
        // A row a frame: the step path leaves the sleep (`E+$0E`) alone.
        if self.sleep > 0 {
            self.state = State::Waiting(std::mem::take(&mut self.sleep));
        }
        false
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
                let Some(skip) = around.globals.count(op, word) else {
                    self.state = State::Frozen;
                    return false;
                };
                self.pc = operands + skip;
            }
            YIELD => {
                self.pc = operands;
                if self.sleep > 0 {
                    self.state = State::Waiting(std::mem::take(&mut self.sleep));
                }
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
            REDIRECT_SCRIPT | REDIRECT_AFTER => {
                return self.redirect_script(service, operands, image)
            }
            SET_SCRIPT | LONG_JUMP => {
                let Some(target) = image.get(operands..operands + 3).and_then(long) else {
                    self.state = State::Frozen;
                    return false;
                };
                if service == LONG_JUMP && is_death(image, target) {
                    self.die(image);
                    return false;
                } else if service == LONG_JUMP {
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

    /// `COP BF` writes the actor's script pointer, clears its countdown and
    /// exits the scheduler (`$80:AAE1`). The target runs on the next tick.
    fn redirect_script(&mut self, service: u8, operands: usize, image: &[u8]) -> bool {
        let delay = if service == REDIRECT_AFTER {
            let Some(delay) = cadence::word(image, operands + 3) else {
                self.state = State::Frozen;
                return false;
            };
            delay
        } else {
            0
        };
        let Some(target) = image
            .get(operands..operands + 3)
            .and_then(long)
            .filter(|&target| image.get(target..target + 2).is_some())
        else {
            self.state = State::Frozen;
            return false;
        };
        if is_death(image, target) {
            self.die(image);
        } else {
            self.pc = target;
            // `E+$0E`: the rest before the target runs, as a yield's.
            if delay > 0 {
                self.state = State::Waiting(delay);
            }
        }
        false
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
            TAKE_ITEM => {
                let Some(&item) = image.get(operands) else {
                    self.state = State::Frozen;
                    return false;
                };
                around.globals.take_item(item);
                self.pc = operands + 1;
            }
            NO_ROOM => {
                let (Some(&item), Some(target)) =
                    (image.get(operands), cadence::word(image, operands + 1))
                else {
                    self.state = State::Frozen;
                    return false;
                };
                if !around.globals.inventory.has_room(item) {
                    return self.jump(bank, target);
                }
                self.pc = operands + 3;
            }
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
                let kept = around.globals.inventory.add(item);
                // Elle's cape goes on at once, a stand-in for the armor door
                // of the menu, which is not ported (`docs/tower-five.md`,
                // `meta/issues/cape-through-armor-door.md`).
                if kept && item == CAPE && around.globals.slot.armor().is_none() {
                    around.globals.slot.set_armor(item);
                }
                around.globals.audio.fanfare(track, frames);
                around.globals.presentation = Some(crate::scene::Presentation {
                    item,
                    frames,
                    age: 0,
                });
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
            image.get(operands).and_then(|&base| {
                self.base = match base {
                    2 => Base::Common,
                    0 => Base::Own,
                    0xFF => {
                        let &[low, high, bank] = image.get(operands + 1..operands + 4)? else {
                            return None;
                        };
                        self.resources[2] = None;
                        // A bank outside the ROM: nothing to read.
                        assets::maps::actors::rom_offset(&[0, 0x80, bank])
                            .map_or(Base::Unknown, |bank| {
                                Base::Rom(bank & !0xFFFF, u16::from_le_bytes([low, high]))
                            })
                    }
                    _ => Base::Unknown,
                };
                Some(operands + if base == 0xFF { 4 } else { 1 })
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
        let (low, high) = (
            u16::from(*image.get(operands)?),
            u16::from(*image.get(operands + 1)?),
        );
        let table = operands + 2;
        // `+$26`: the spawn parameter, or what the script stored there
        // since (`$80:8CE9`; Shadowkeeper's phases, `$93:DD0F`).
        let value = self.own_word(0x26);
        if (low..=high).contains(&value) {
            let target = cadence::word(image, table + usize::from(value - low) * 2)?;
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
            NEAR_BRANCH | AREA_BRANCH => {
                return self.branch_in_cells(operands, bank, around, service == NEAR_BRANCH)
            }
            TAKE_PLAYER => {
                if around.globals.player_action {
                    // `$80:B87B` retries the COP next frame.
                    return false;
                }
                let Some(script) = image.get(operands..operands + 3).and_then(long) else {
                    self.state = State::Frozen;
                    return false;
                };
                around.globals.player_script = Some(script);
                around.globals.player_script_source = Some(self.pc);
                self.pc = operands + 3;
            }
            SET_CONTROL => {
                let (Some(&kind), Some(script)) = (
                    image.get(operands),
                    image.get(operands + 1..operands + 4).and_then(long),
                ) else {
                    self.state = State::Frozen;
                    return false;
                };
                // The last word in a frame stands: the pad's ends the
                // script holding Ark, another takes him. A first byte with
                // bit 0 clear (`$80:ADCD`: `$0980`, `$85:D1F8`) is not
                // modelled and goes by, as before.
                if script == PAD_CONTROL {
                    around.globals.release_player = true;
                    around.globals.player_script = None;
                } else if kind & 1 != 0 {
                    around.globals.release_player = false;
                    around.globals.player_script = Some(script);
                    around.globals.player_script_source = Some(self.pc);
                }
                self.pc = operands + 4;
            }
            TRANSFER | TRANSFER_BY_INDEX => {
                // The table of `$80:8A58` (`$8D:BA41`, European `$8D:B90A`).
                let index = self.own.get(&0x101E).copied().unwrap_or(0);
                let record = if service == TRANSFER {
                    operands
                } else if index == 0 {
                    self.pc = operands;
                    return true;
                } else {
                    assets::layout::per_revision(image, 0x0D_BA41, 0x0D_B90A)
                        + usize::from(index) * 8
                };
                let (Some(map), Some(&mode), Some(x), Some(y)) = (
                    cadence::word(image, record),
                    image.get(record + 2),
                    cadence::word(image, record + 4),
                    cadence::word(image, record + 6),
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
                self.pc = operands + if service == TRANSFER { 8 } else { 0 };
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
    /// inclusive; negative near corners clamp to 0. `COP 0C` (`relative`
    /// false) takes the corners as map cells. Returns whether execution
    /// continues this frame.
    fn branch_in_cells(
        &mut self,
        operands: usize,
        bank: usize,
        around: &Surroundings<'_>,
        relative: bool,
    ) -> bool {
        let Some(bytes) = around.image.get(operands..operands + 7) else {
            self.state = State::Frozen;
            return false;
        };
        let id = bytes[0];
        let corner = |base: u16, byte: u8, less: i32| {
            i32::from(base) + i32::from(i8::from_ne_bytes([byte])) * 16 - less
        };
        // `COP 0C` counts from the map's corner, without the 8 above.
        let (x, y, less) = if relative {
            (self.position.0, self.position.1, 8)
        } else {
            (0, 0, 0)
        };
        let (left, top) = (
            corner(x, bytes[1], 0).max(0),
            corner(y, bytes[2], less).max(0),
        );
        let (right, bottom) = (corner(x, bytes[3], 0), corner(y, bytes[4], less));
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
        Wait::Text | Wait::Records => Some(pc),
        Wait::Choice(table) => {
            let target = cadence::word(image, table + 2 * usize::from(answer))?;
            (target >= 0x8000).then_some((table & 0xFF_0000) | usize::from(target))
        }
    }
}

/// Where the `BEQ` or `BNE` at `at` goes after a test that left Z as
/// `zero`.
fn tested_branch(image: &[u8], at: usize, zero: bool) -> Option<usize> {
    let (&opcode, &offset) = (image.get(at)?, image.get(at + 1)?);
    let taken = match opcode {
        0xF0 => zero,
        0xD0 => !zero,
        _ => return None,
    };
    let next = at + 2;
    Some(if taken {
        (at & 0xFF_0000)
            | (next.wrapping_add_signed(isize::from(i8::from_ne_bytes([offset]))) & 0xFFFF)
    } else {
        next
    })
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

/// `COP D0`'s orbit (`$7F:2000..2013`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Orbit {
    /// The point it turns about.
    centre: (u16, u16),
    /// About Ark (target `$0DEA`, the Cadet's ring `$97:C0AC`): the offset
    /// from him, the centre followed each frame.
    about_player: Option<(u16, u16)>,
    /// 1024 a turn.
    angle: u16,
    /// In units of 1/128 of the table's 127.
    radius: u16,
    /// Added to the angle each frame.
    turn: i16,
    /// Added to the radius each frame.
    reach: i16,
    /// Frames left, or with bit 15 the radius it ends at.
    end: u16,
}

impl Orbit {
    /// The position on it (`$87:C701`, `$87:C72F`): the centre plus
    /// `radius * table / 128` toward the cosine and the sine.
    fn position(&self, image: &[u8]) -> Option<(u16, u16)> {
        // `STA $4202` takes the radius's low byte only.
        let [radius, _] = self.radius.to_le_bytes();
        let along = |index: usize, base: u16| {
            let table = i8::from_ne_bytes([*image.get(SINE + index)?]);
            let length = (i32::from(table.unsigned_abs()) * i32::from(radius) * 2) >> 8;
            let length = if table < 0 { -length } else { length };
            Some(base.wrapping_add_signed(i16::try_from(length).ok()?))
        };
        let angle = usize::from(self.angle);
        let clamp = |at: u16| if at & 0x8000 != 0 { 0 } else { at };
        Some((
            along(angle + 256, clamp(self.centre.0))?,
            along(angle, clamp(self.centre.1))?,
        ))
    }
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
    fn changed_frozen_return_guide_target_freezes_the_player_actor() {
        let mut image = vec![0; 0x09_0000];
        let guide = cadence::frozen_return_guide(&image).unwrap();
        let changed_target = cadence::frozen_return_start(&image).unwrap() + 1;
        let runtime = 0x80_0000 | u32::try_from(changed_target).unwrap();
        let pointer = runtime.to_le_bytes();
        image[guide..guide + 5].copy_from_slice(&[
            2,
            TAKE_PLAYER,
            pointer[0],
            pointer[1],
            pointer[2],
        ]);
        let guide_runtime = 0x80_0000 | u32::try_from(guide).unwrap();
        let mut guide_actor = Actor::new((136, 368), Some(guide_runtime), 0, 1);
        let mut globals = Globals::with_events(vec![0; 512]);
        guide_actor.tick(&mut surroundings(&image, &[], &mut globals));
        assert_eq!(globals.player_script, Some(changed_target));
        assert_eq!(globals.player_script_source, Some(guide));

        let actor = Actor::for_player(
            &image,
            0x21,
            (136, 368),
            Some(runtime),
            globals.player_script_source,
        );

        assert!(matches!(actor.state, State::Frozen));
        assert_eq!(actor.frozen_at(), Some(changed_target));
        assert!(!actor.admitted_player_motion_active());
    }

    #[test]
    fn generic_player_pose_motion_never_owns_ark() {
        let image = [0];
        let mut actor = Actor::for_player(&image, 0, (40, 48), Some(0x80_8000), None);
        actor.pose_ticks = Some(vec![Some(2)]);
        let words: [u16; 7] = [0, 0x6004, 0, 0, 1, 0xFFFF, 0x6004];
        actor.resources[0] = Some(motion::Resource::wram(
            0x6000,
            words.iter().flat_map(|word| word.to_le_bytes()).collect(),
        ));

        assert!(actor.moving_pose(POSE_MOVING, 0, &image));
        assert!(
            actor.motion.is_some(),
            "generic COP 81 may retain actor motion"
        );
        assert!(
            !actor.admitted_player_motion_active(),
            "an unauthenticated player script cannot acquire Ark ownership"
        );
        actor.apply_stream();
        assert_eq!(
            actor.position,
            (40, 49),
            "the temporary actor may still move"
        );
        assert!(!actor.admitted_player_motion_active());
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
    fn cop_2b_with_bit_0_jumps_only_on_a_tower_floor() {
        // COP 2B $0081 (A and bit 0) to $8020; pose 7; ...; $8020: pose 9.
        let mut code = vec![2, 0x2B, 0x81, 0x00, 0x20, 0x80, 2, 0x80, 7, 2, 0xBD];
        code.resize(0x20, 0);
        code.extend_from_slice(&[2, 0x80, 9, 2, 0xBD]);
        for (mode, selector) in [(0, 7), (0x8000, 9)] {
            let (image, mut actor) = actor_running(&code);
            let mut globals = Globals::with_events(vec![0; 512]);
            globals.pad = 0x0080;
            globals.scratch.insert(MAP_MODE, mode);
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
            assert_eq!(actor.selector, selector, "mode {mode:#06x}");
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
    fn cop_0c_waits_for_the_player_in_a_rectangle_of_map_cells() {
        // `$90:9418`: COP 0C 7F (31,19)-(33,20) -> pose 7; else pose 9.
        let [low, high] = u16::try_from((AT + 14) & 0xFFFF).unwrap().to_le_bytes();
        let code = [
            2, 0x0C, 0x7F, 0x1F, 0x13, 0x21, 0x14, low, high, 2, 0x80, 9, 2, 0xBD, 2, 0x80, 7, 2,
            0xBD,
        ];
        for (player, pose) in [((512, 320), 7), ((512, 344), 9)] {
            let (image, mut actor) = actor_running(&code);
            let mut globals = Globals::with_events(vec![0; 512]);
            tick_at(&mut actor, &image, &mut globals, player);
            assert_eq!(actor.selector, pose, "{player:?}");
        }
    }

    #[test]
    fn cop_cb_hands_ark_back_to_the_pad() {
        // COP CB 01 $84:87C1; yield.
        let (image, mut actor) = actor_running(&[2, 0xCB, 1, 0xC1, 0x87, 0x84, 2, 0xBD]);
        let mut globals = Globals::with_events(vec![0; 512]);
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        assert!(globals.release_player && globals.player_script.is_none());
    }

    #[test]
    fn a_dropped_ball_takes_the_walls_by_its_04_bit() {
        // `$90:9C27`: LDA $0004,X; ORA #$0004; STA $0004,X; yield; then the
        // bit cleared again.
        let set = [
            0xBD, 0x04, 0x00, 0x09, 0x04, 0x00, 0x9D, 0x04, 0x00, 2, 0xBD,
        ];
        let (image, mut actor) = actor_running(&set);
        tick(&mut actor, &image);
        assert!(actor.walls && actor.frozen_at().is_none());
        let clear = [
            0xBD, 0x04, 0x00, 0x29, 0xFB, 0xFF, 0x9D, 0x04, 0x00, 2, 0xBD,
        ];
        let (image, _) = actor_running(&clear);
        actor.pc = AT;
        tick(&mut actor, &image);
        assert!(!actor.walls);
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
        assert_eq!(
            globals.player_script, None,
            "a forced action defers the handoff"
        );
        assert_eq!(globals.player_script_source, None);
        globals.player_action = false;
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        assert_eq!(actor.selector, 7);
        assert_eq!(globals.player_script, Some(0x08_8ea6));
        assert_eq!(globals.player_script_source, Some(AT));
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
    fn cop_bf_redirects_the_actor_next_tick_without_running_fallthrough() {
        // COP BF $88:8010; pose 7 belongs to a different script. The target
        // sets pose 9, then yields. A same-frame jump would set it on tick 1.
        let mut code = vec![2, 0xBF, 0x10, 0x80, 0x88, 2, 0x80, 7, 2, 0xBD];
        code.resize(0x10, 0);
        code.extend_from_slice(&[2, 0x80, 9, 2, 0xBD]);
        let (image, mut actor) = actor_running(&code);
        tick(&mut actor, &image);
        assert_eq!(
            (actor.pc, actor.selector, actor.state),
            (AT + 0x10, 0, State::Running)
        );
        tick(&mut actor, &image);
        assert_eq!((actor.pc, actor.selector), (AT + 0x15, 9));
        assert_eq!(actor.frozen_at(), None);
    }

    #[test]
    fn cop_bf_refuses_truncated_ram_and_out_of_image_targets() {
        for code in [
            vec![2, 0xBF, 0x10],             // missing address/bank bytes
            vec![2, 0xBF, 0x00, 0x40, 0x7E], // RAM
            vec![2, 0xBF, 0x00, 0x90, 0x88], // beyond the image
        ] {
            let mut image = vec![0; AT + code.len()];
            image[AT..].copy_from_slice(&code);
            let mut actor = Actor::new((56, 64), Some(0x88_8000), 0, 1);
            tick(&mut actor, &image);
            assert_eq!(actor.state, State::Frozen, "{code:02X?}");
            assert_eq!(actor.frozen_at(), Some(AT), "{code:02X?}");
        }
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
    fn a_call_to_the_records_screen_clears_its_flags_and_waits_for_it() {
        // COP00 $87:8590; pose 7; wait.
        let (image, mut actor) = actor_running(&[2, 0x00, 0x90, 0x85, 0x87, 2, 0x80, 7, 2, 0x8E]);
        let mut events = vec![0; 512];
        events[0xFB / 8] = 0x18;
        let mut globals = Globals::with_events(events);
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
        actor.tick(&mut around);
        assert_eq!(actor.blocked(), Some(Wait::Records));
        assert_eq!(
            around.globals.events[0xFB / 8],
            0,
            "`$FB` and `$FC` cleared"
        );
        assert_ne!(actor.selector, 7);
        actor.resume(0, &mut around);
        assert_eq!(actor.selector, 7, "the screen returns past the call");
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
    fn cop_b2_moves_y_and_cop_ba_sets_the_obj_priority() {
        // COP B2 $FFF8; COP BA $30; yield.
        let (image, mut actor) = actor_running(&[2, 0xB2, 0xF8, 0xFF, 2, 0xBA, 0x30, 2, 0xBD]);
        let y = actor.position.1;
        tick(&mut actor, &image);
        assert_eq!((actor.position.1, actor.priority), (y - 8, 3));
    }

    #[test]
    fn cop_b0_ff_moves_by_a_rom_resource() {
        // COP B0 FF $9000 $B2; COP 81 0; COP 8E. At $B2:9000 the table, X's
        // stream at +$10: 3 a frame, looping.
        let mut image = vec![0; 0x33_0000];
        image[AT..AT + 11].copy_from_slice(&[2, 0xB0, 0xFF, 0x00, 0x90, 0xB2, 2, 0x81, 0, 2, 0x8E]);
        for (at, word) in [
            (0, 0x10u16),
            (0x12, 1),
            (0x14, 3),
            (0x16, 0xFFFF),
            (0x18, 0x12),
        ] {
            let at = 0x32_9000 + at;
            image[at..at + 2].copy_from_slice(&word.to_le_bytes());
        }
        let mut actor = Actor::new((56, 64), Some(0x88_8000), 0, 1);
        tick(&mut actor, &image);
        tick(&mut actor, &image);
        assert_eq!(actor.position.0, 56 + 6);
        assert!(actor.frozen_at().is_none());
    }

    #[test]
    fn cop_46_on_the_second_layer_records_its_copies() {
        // Layer 2 (own `$7F:201B`): `COP 46 1 $10 7 5 7 4 8` copies row 0
        // of two cells, (7,5)-(8,5) to (7,4)-(8,4), then row 1 next frame.
        let code = [2, 0x46, 1, 0x10, 7, 5, 7, 4, 8, 2, 0xBD];
        let (image, mut actor) = actor_running(&code);
        actor.own.insert(0x201B, 2);
        let mut globals = Globals::with_events(vec![0; 512]);
        for _ in 0..2 {
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
        }
        assert_eq!(
            globals.second_copies,
            [
                ((7, 5), (7, 4)),
                ((8, 5), (8, 4)),
                ((7, 6), (7, 5)),
                ((8, 6), (8, 5))
            ]
        );
        assert!(globals.patches.is_empty(), "the first layer is untouched");
    }

    #[test]
    fn cop_b4_b5_and_b9_set_clear_and_toggle_the_vertical_flip() {
        // COP B4; yield; COP B5; yield; COP B9; yield; COP B9; yield.
        let code = [
            2, 0xB4, 2, 0xBD, 2, 0xB5, 2, 0xBD, 2, 0xB9, 2, 0xBD, 2, 0xB9, 2, 0xBD,
        ];
        let (image, mut actor) = actor_running(&code);
        let flips: Vec<bool> = (0..4)
            .map(|_| {
                tick(&mut actor, &image);
                actor.vflip
            })
            .collect();
        assert_eq!(flips, [true, false, true, false]);
        assert!(actor.frozen_at().is_none());
    }

    #[test]
    fn cop_bb_sets_the_palette_field() {
        // COP BB $0E (`$80:AA8A`: +$08 bits 9-11 = 7); yield.
        let (image, mut actor) = actor_running(&[2, 0xBB, 0x0E, 2, 0xBD]);
        tick(&mut actor, &image);
        assert_eq!((actor.palette, actor.priority), (7, 2));
        assert!(actor.frozen_at().is_none());
    }

    #[test]
    fn cop_63_draws_the_circle_around_its_caller() {
        // COP 63 01 05 (`$80:9CA5`): `$0476` = the caller, `$0474` = 5.
        let (image, mut actor) = actor_running(&[2, 0x63, 1, 5, 2, 0xBD]);
        let mut globals = Globals::with_events(vec![0; 512]);
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        assert_eq!(globals.circle, Some(actor.id));
        assert_eq!(globals.scratch.get(&CIRCLE_RADIUS), Some(&5));
        assert!(actor.frozen_at().is_none());
    }

    #[test]
    fn cop_5a_loads_obj_colours() {
        // COP 5A $88 $8009 $90 1 (`$80:9AEB`): one colour to CGRAM `$90`;
        // yield; the colour.
        let code = [2, 0x5A, 0x88, 0x09, 0x80, 0x90, 1, 2, 0xBD, 0x34, 0x12];
        let (image, mut actor) = actor_running(&code);
        let mut globals = Globals::with_events(vec![0; 512]);
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        let colour = assets::graphics::Bgr555::new(0x1234);
        assert_eq!(globals.obj_colours.get(0x90), Some(colour));
        assert!(actor.frozen_at().is_none());
    }

    #[test]
    fn cop_99_spawns_as_cop_a2_does() {
        let code = [2, 0x99, 0x29, 0x80, 0x88, 0x00, 0x40, 2, 0xBD];
        let (image, mut actor) = actor_running(&code);
        let mut globals = Globals::with_events(vec![0; 512]);
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        let [(script, child)] = &globals.spawns[..] else {
            panic!("one spawn");
        };
        assert_eq!((*script, child.position), (0x08_8029, actor.position));
    }

    #[test]
    fn cop_a1_spawns_at_the_actor() {
        // COP A1 $88:8029; yield.
        let code = [2, 0xA1, 0x29, 0x80, 0x88, 2, 0xBD];
        let (image, mut actor) = actor_running(&code);
        let mut globals = Globals::with_events(vec![0; 512]);
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        let [(script, child)] = &globals.spawns[..] else {
            panic!("one spawn");
        };
        assert_eq!((*script, child.position), (0x08_8029, actor.position));
        // Without a flags word the child takes the parent's, hidden and out
        // of the nested frame (`$80:BCD2`), until its script shows it.
        assert!(child.hidden && !actor.hidden);
        let (image, mut actor) = actor_running(&code);
        actor.write_04(0x1000, 0);
        let mut globals = Globals::with_events(vec![0; 512]);
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        assert!(actor.runs_nested() && !globals.spawns[0].1.runs_nested());
    }

    #[test]
    fn cop_e8_spawns_into_the_group_and_eb_deletes_it() {
        // COP E8 $88:8029 (+8,-4) $2230; COP E8 again; yield; COP EB; yield.
        let spawn = [2, 0xE8, 0x29, 0x80, 0x88, 8, 0, 0xFC, 0xFF, 0x30, 0x22];
        let mut code = [spawn, spawn].concat();
        code.extend_from_slice(&[2, 0xBD, 2, 0xEB, 2, 0xBD]);
        let (image, mut actor) = actor_running(&code);
        actor.id = 0x0105;
        let mut globals = Globals::with_events(vec![0; 512]);
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        let roots: Vec<_> = globals
            .spawns
            .iter()
            .map(|(_, child)| (child.position, child.group()))
            .collect();
        assert_eq!(roots, [((64, 60), Some(0x0105)); 2]);
        // A child spawning into the group passes the root on.
        let (_, child) = &globals.spawns[0];
        assert_eq!(child.group_of_spawns(), 0x0105);
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        assert_eq!(globals.group_deletions, [(0x0105, 2)]);
    }

    fn profile(life: u16) -> crate::combat::Profile {
        crate::combat::Profile {
            level: 1,
            life,
            ..crate::combat::Profile::default()
        }
    }

    #[test]
    fn a_wall_follower_blocked_every_way_waits_a_frame() {
        // `$97:B445`-like: C5 -> C3 -> C4 -> C2 -> C5 ..., all blocked (no
        // cells: off the map).
        let at = |offset: usize| u16::try_from((AT + offset) & 0xFFFF).unwrap().to_le_bytes();
        let mut code = vec![];
        for (service, next) in [(0xC5, 4), (0xC3, 8), (0xC4, 12), (0xC2, 0)] {
            let [low, high] = at(next);
            code.extend_from_slice(&[2, service, low, high]);
        }
        let (image, mut actor) = actor_running(&code);
        tick(&mut actor, &image);
        assert_eq!(actor.frozen_at(), None);
    }

    #[test]
    fn cop_15_transfers_by_the_actors_index() {
        // `7F:101E = 1`: the table's first record, tower 3's `$10F` at raw
        // (376,912); the loader adds (8,16).
        let mut code = vec![2, 0x15, 2, 0xBD];
        code.resize(0x8_0000, 0);
        let (mut image, mut actor) = actor_running(&code);
        image[0x0D_BA49..0x0D_BA51].copy_from_slice(&[0x0F, 0x01, 0, 6, 0x78, 0x01, 0x90, 0x03]);
        actor.own.insert(0x101E, 1);
        let mut globals = Globals::with_events(vec![0; 512]);
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        let transfer = globals.transfer.expect("a transfer");
        assert_eq!((transfer.map, transfer.position), (0x010F, (384, 928)));
    }

    #[test]
    fn cop_be_goes_on_elsewhere_after_a_rest() {
        // COP BE $88:8010 3; at +$10: pose 9, yield.
        let mut code = vec![2, 0xBE, 0x10, 0x80, 0x88, 3, 0];
        code.resize(0x10, 0);
        code.extend_from_slice(&[2, 0x80, 9, 2, 0xBD]);
        let (image, mut actor) = actor_running(&code);
        for _ in 0..3 {
            tick(&mut actor, &image);
            assert_ne!(actor.selector, 9, "resting");
        }
        for _ in 0..3 {
            tick(&mut actor, &image);
        }
        assert_eq!(actor.selector, 9);
    }

    #[test]
    fn cop_55_takes_an_item_and_56_tests_for_room() {
        // COP 56 $32 -> $8010 (no room); COP 55 $32; yield. At $8010: pose 9.
        let mut code = vec![2, 0x56, 0x32, 0x10, 0x80, 2, 0x55, 0x32, 2, 0xBD];
        code.resize(0x10, 0);
        code.extend_from_slice(&[2, 0x80, 9, 2, 0xBD]);
        let (image, mut actor) = actor_running(&code);
        let mut globals = Globals::with_events(vec![0; 512]);
        globals.inventory.add(0x32);
        tick_at(&mut actor, &image, &mut globals, (0, 0));
        assert_eq!(globals.inventory.count(0x32), 0, "Elle takes the thread");
        assert_ne!(actor.selector, 9);
    }

    #[test]
    fn a_struck_callback_takes_the_script_after_a_hit_that_did_not_kill() {
        // Pose 7, yield; at +$10 the callback: pose 9, yield. The script
        // keeps its life in `7F:102A` ($6000) and handles knockback itself.
        let mut code = vec![2, 0x80, 7, 2, 0xBD];
        code.resize(0x10, 0);
        code.extend_from_slice(&[2, 0x80, 9, 2, 0xBD]);
        let (image, mut actor) = actor_running(&code);
        actor.foe = Some(foe::Foe::new(profile(20), false));
        actor.guard.1 = 0x0010;
        let [low, high] = u16::try_from((AT + 0x10) & 0xFFFF).unwrap().to_le_bytes();
        actor
            .own
            .extend([(0x1016, low), (0x1017, high), (0x102A, 0), (0x102B, 0x60)]);
        tick(&mut actor, &image);
        assert_eq!(actor.selector, 7);
        assert_eq!(actor.foe.as_ref().map(|foe| foe.life), Some(0x6000));
        actor.take_hit(5, Direction::Down, &image);
        tick(&mut actor, &image);
        assert_eq!(actor.selector, 9);
    }

    #[test]
    fn cop_04_loops_back_within_the_frame() {
        // COP 02 3 0; COP B1 1 0 (x + 1); COP 04; COP BD.
        let code = [2, 0x02, 3, 0, 2, 0xB1, 1, 0, 2, 0x04, 2, 0xBD];
        let (image, mut actor) = actor_running(&code);
        let x = actor.position.0;
        tick(&mut actor, &image);
        assert_eq!(actor.position.0, x + 3);
    }

    #[test]
    fn an_idle_foe_ages_its_pose_a_frame_a_frame() {
        // Pose 7, then yields in a loop: a foe's pose ages as anyone's.
        let code = [2, 0x80, 7, 2, 0xBD, 0x80, 0xFC];
        let (image, mut plain) = actor_running(&code);
        let (_, mut foe) = actor_running(&code);
        foe.foe = Some(foe::Foe::new(profile(20), false));
        for _ in 0..10 {
            tick(&mut plain, &image);
            tick(&mut foe, &image);
        }
        assert_eq!(foe.pose_age, plain.pose_age);
    }

    #[test]
    fn a_jump_to_the_death_script_ends_the_actor() {
        // COP 06 $85:E27B: an enemy explodes; anything else goes. COP BF
        // too.
        let code = [2, 0x06, 0x7B, 0xE2, 0x85];
        let (image, mut actor) = actor_running(&code);
        tick(&mut actor, &image);
        assert!(actor.is_gone());
        let mut image = vec![0; AT + 5];
        image[AT..AT + 5].copy_from_slice(&[2, 0xBF, 0x7B, 0xE2, 0x85]);
        let mut actor = Actor::new((56, 64), Some(0x88_8000), 0, 1);
        tick(&mut actor, &image);
        assert!(actor.is_gone());
        let (image, mut actor) = actor_running(&code);
        actor.foe = Some(foe::Foe::new(profile(20), false));
        tick(&mut actor, &image);
        assert!(actor
            .foe
            .as_ref()
            .is_some_and(|foe| foe.exploding.is_some()));
        // A group's root takes its group along.
        let (image, mut actor) = actor_running(&code);
        actor.root = true;
        tick(&mut actor, &image);
        assert!(actor.take_root_death());
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
    fn enemies_test_the_layer_and_keep_their_combat_bits() {
        // Same layer: BNE +3 falls through to COP 80 7; the flyer's
        // `ORA #$0030` on `+$04` and `ORA #$0010` on `+$06`; yield.
        let code = [
            0xAC, 0xEA, 0x0D, 0xB9, 0x16, 0x00, 0xDD, 0x16, 0x00, 0xD0, 0x03, 2, 0x80, 7, 0xBD,
            0x04, 0x00, 0x09, 0x30, 0x00, 0x9D, 0x04, 0x00, 0xBD, 0x06, 0x00, 0x09, 0x10, 0x00,
            0x9D, 0x06, 0x00, 2, 0xBD,
        ];
        let (image, mut actor) = actor_running(&code);
        tick(&mut actor, &image);
        assert_eq!(
            (actor.frozen_at(), actor.selector, actor.guard),
            (None, 7, (0x30, 0x10))
        );
    }

    #[test]
    fn cop_cc_and_cd_walk_a_line_toward_the_target() {
        // The flyer's chase: COP CC 00 08 01 18 FF; COP CD; COP 80 7; yield.
        let code = [2, 0xCC, 0, 8, 1, 0x18, 0xFF, 2, 0xCD, 2, 0x80, 7, 2, 0xBD];
        let (image, mut actor) = actor_running(&code);
        for (at, byte) in [(0x2004, 96), (0x2005, 0), (0x2006, 94), (0x2007, 0)] {
            actor.own.insert(at, byte);
        }
        for _ in 0..24 {
            tick(&mut actor, &image);
        }
        // 24 frames, the last moving: (+23,+17), the script not yet on.
        assert_eq!((actor.position, actor.selector), ((79, 81), 8));
        tick(&mut actor, &image);
        assert_eq!((actor.position, actor.selector), ((79, 81), 7));
    }

    #[test]
    fn cop_b8_toggles_the_mirror() {
        // COP B7; COP B8; yield; COP B8; yield.
        let (image, mut actor) = actor_running(&[2, 0xB7, 2, 0xB8, 2, 0xBD, 2, 0xB8, 2, 0xBD]);
        tick(&mut actor, &image);
        assert!(!actor.hflip);
        tick(&mut actor, &image);
        assert!(actor.hflip);
    }

    #[test]
    fn cop_82_and_86_select_a_pose_with_a_selector_or_a_count() {
        // COP 82 5 9; yield.
        let (image, mut actor) = actor_running(&[2, 0x82, 5, 9, 2, 0xBD]);
        tick(&mut actor, &image);
        assert_eq!((actor.frozen_at(), actor.selector), (None, 5));
        // COP 86 3 4; yield.
        let (image, mut actor) = actor_running(&[2, 0x86, 3, 4, 2, 0xBD]);
        tick(&mut actor, &image);
        assert_eq!(
            (actor.frozen_at(), actor.selector, actor.repeats),
            (None, 4, Some(3))
        );
    }

    #[test]
    fn cop_25_steps_the_random_and_native_code_dispatches_on_it() {
        // COP 25; COP 25; LDA $0408; AND #7; STA $0026,X; DEC; STA $0024,X;
        // LDA $0026,X; CMP $0024,X; COP BD.
        let (image, mut actor) = actor_running(&[
            2, 0x25, 2, 0x25, 0xAD, 0x08, 0x04, 0x29, 0x07, 0x00, 0x9D, 0x26, 0x00, 0x3A, 0x9D,
            0x24, 0x00, 0xBD, 0x26, 0x00, 0xDD, 0x24, 0x00, 2, 0xBD,
        ]);
        tick(&mut actor, &image);
        assert!(actor.frozen_at().is_none());
        // Two steps from zero: $0408 = $0101, & 7 = 1.
        let word = |at: u16| {
            u16::from_le_bytes([
                actor.own[&at],
                actor.own.get(&(at + 1)).copied().unwrap_or(0),
            ])
        };
        assert_eq!((word(0x26), word(0x24)), (1, 0));
    }

    #[test]
    fn cop_d6_d7_d3_d4_test_the_players_place() {
        // The actor stands at (56,64); Ark's probe is (x, y - 8).
        let site = |offset: u16| {
            u16::try_from(AT & 0xFFFF)
                .unwrap()
                .wrapping_add(offset)
                .to_le_bytes()
        };
        let run = |code: &[u8], player: (u16, u16)| {
            let (image, mut actor) = actor_running(code);
            actor.tick(&mut Surroundings {
                image: &image,
                globals: &mut Globals::with_events(vec![0; 512]),
                cells: &[],
                width: 0,
                height: 0,
                occupied: &[],
                player,
                facing: Direction::Down,
            });
            actor.selector
        };
        // D6 $20 →near: pose 1 there, else pose 2.
        let [n0, n1] = site(10);
        let near = [
            2, 0xD6, 0x20, n0, n1, 2, 0x80, 2, 2, 0xBD, 2, 0x80, 1, 2, 0xBD,
        ];
        assert_eq!(run(&near, (80, 96)), 1, "dx 24, dy 24");
        assert_eq!(run(&near, (89, 72)), 2, "dx 33");
        // D7 →h →v: pose 3 horizontal, 4 vertical.
        let [h0, h1] = site(6);
        let [v0, v1] = site(11);
        let axis = [
            2, 0xD7, h0, h1, v0, v1, 2, 0x80, 3, 2, 0xBD, 2, 0x80, 4, 2, 0xBD,
        ];
        assert_eq!(run(&axis, (100, 72)), 3);
        assert_eq!(run(&axis, (60, 120)), 4);
        // D3 4 →left →even →right: poses 5, 6, 7.
        let [l0, l1] = site(10);
        let [e0, e1] = site(13);
        let [r0, r1] = site(18);
        let sides = [
            2, 0xD3, 4, 0, l0, l1, e0, e1, r0, r1, 2, 0x80, 5, 2, 0x80, 6, 2, 0xBD, 2, 0x80, 7, 2,
            0xBD,
        ];
        assert_eq!(run(&sides, (40, 72)), 6, "left, then falls into even");
        assert_eq!(run(&sides, (58, 72)), 6, "within the dead zone");
        assert_eq!(run(&sides, (70, 72)), 7);
        // D4 compares Ark's feet: dy = y + 8 - 8 - 64.
        let mut heights = sides;
        heights[1] = 0xD4;
        assert_eq!(run(&heights, (56, 40)), 6, "above, then even");
        assert_eq!(run(&heights, (56, 90)), 7, "below");
    }

    #[test]
    fn cop_59_sleeps_off_screen_and_retries() {
        // COP 59 3; pose 1; COP BD. The actor stands at (56,64).
        let run = |view, frames| {
            let (image, mut actor) = actor_running(&[2, 0x59, 3, 2, 0x80, 1, 2, 0xBD]);
            let mut globals = Globals::with_events(vec![0; 512]);
            globals.view = view;
            for _ in 0..frames {
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
            }
            actor.selector
        };
        assert_eq!(run(Some((0, 0, 256, 224)), 1), 1, "on screen: on");
        assert_eq!(run(Some((0, 300, 256, 524)), 4), 0, "off screen: asleep");
        assert_eq!(run(None, 1), 1, "no view: on");
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
        // Bit 6 subtracts in BCD down to 0 (`$80:979D`), and reads its
        // word without skipping it: the script goes on after the op byte.
        assert_eq!(globals.count(0x42, 0x0999), Some(1));
        assert_eq!(globals.counter(2), 0x9000);
        assert_eq!(globals.count(0x42, 0x9001), Some(1));
        assert_eq!(globals.counter(2), 0);
        assert_eq!(globals.count(0x02, 7), Some(3));
    }

    #[test]
    fn cop_4b_with_bit_6_goes_on_after_its_op_byte() {
        // COP 4B $40 with the word's bytes `2, $80` (a `COP 80`); the
        // script runs them next: `COP 80 3`, and BCD 9000 - 8002 = 0998.
        let mut globals = Globals::with_events(vec![0; 512]);
        globals.count(0x00, 0x9000);
        let (_, actor) = run(&[(0, &[2, 0x4B, 0x40, 2, 0x80, 3, 2, 0xBD])], &mut globals);
        assert_eq!((globals.counter(0), actor.selector), (0x0998, 3));
    }

    #[test]
    fn cop_22_switches_on_the_spawn_parameter_and_b0_sets_a_speed() {
        // COP B0 02 (a speed), then COP 22 01 02 with a table of two
        // targets: parameter 1 jumps to $8030, 2 to $8040, anything else
        // goes on past the table to COP 8E.
        let script: &[u8] = &[
            2, 0xB0, 0x02, 2, 0x22, 0x01, 0x02, 0x30, 0x80, 0x40, 0x80, 2, 0x8E,
        ];
        for (parameter, expected) in [
            (1u16, AT + 0x30),
            (2, AT + 0x40),
            (0, AT + 11),
            (3, AT + 11),
        ] {
            let mut image = vec![0; AT + 0x200];
            image[AT..AT + script.len()].copy_from_slice(script);
            for at in [0x30, 0x40] {
                image[AT + at..AT + at + 2].copy_from_slice(&[2, 0x8E]);
            }
            let mut globals = Globals::with_events(vec![0; 512]);
            let mut actor = Actor::new((56, 64), Some(0x88_8000), 0, 1);
            actor.set_own_word(0x26, parameter);
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
