//! Residents placed from spawn lists, and the dialogue they carry.
//!
//! A spawn record says where someone stands and points at the script that runs
//! when the player talks to them. [`assets::maps::actors`] resolves the first
//! and [`assets::maps::actor_script`] walks the second; this joins them into
//! something a runtime can put in a room.

use assets::maps::actor_script::{self, ScriptEffects};
use assets::maps::actors::{descriptor_owner, ResolveError, SpawnList};
use assets::maps::scripts::EventFlags;
use assets::sprites::{HouseActor, ResidentPose};
use assets::text::{DialoguePage, HouseDialogue};

/// Someone standing in a map.
#[allow(clippy::struct_excessive_bools)] // independent actor bits, not a state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resident {
    /// Pixel position, as the running game reports it.
    pub position: (u16, u16),
    /// Normalized ROM offset of the spawn record.
    pub record: usize,
    /// Runtime address of the script the record installs, when it has one.
    pub script: Option<u32>,
    /// Whether the record decodes to a body: a descriptor whose art the
    /// loader accepts. A script-only record and a refused one are not
    /// bodies, and do not block movement.
    pub body: bool,
    /// The header's initial selector, where the script starts.
    pub initial: u8,
    /// Sequence the running script has selected.
    pub selector: u8,
    /// Horizontal mirror in force.
    pub hflip: bool,
    /// Frames since the pose changed or a qualified action restarted it.
    pub pose_age: u32,
    /// Whether a step is under way.
    pub walking: bool,
    /// Normalized offset of the resource descriptor the actor is built
    /// from: the record's own, or the one a zero pointer reuses.
    pub descriptor: Option<usize>,
    /// Hidden by its script (entity `+$04` bit 15): not drawn, not blocking.
    pub hidden: bool,
}

impl Resident {
    /// Cell the resident stands in, as a reader of the map would name it.
    #[must_use]
    pub const fn cell(&self) -> (u16, u16) {
        (self.position.0 / 16, self.position.1 / 16)
    }

    /// Cell the resident occupies in the *collision* grid.
    ///
    /// Movement samples the grid at `(x - 8, y - 16)`, so a body standing at
    /// `origin()` blocks the cell that reference names, one row above the cell
    /// it visually stands in. Blocking the visual cell instead walls off the
    /// row below the resident, which is where doorway approaches sit.
    #[must_use]
    pub const fn collision_cell(&self) -> (u16, u16) {
        (
            self.position.0.saturating_sub(8) / 16,
            self.position.1.saturating_sub(16) / 16,
        )
    }
}

/// What talking to a resident produced.
#[derive(Debug, Clone)]
pub enum Conversation {
    /// Pages to show, and the flags the script writes.
    Speaks {
        /// Decoded dialogue pages, in order.
        pages: Vec<DialoguePage>,
        /// Event-flag operands written, retaining bit 15.
        flags: Vec<u16>,
    },
    /// The resident has a script, but it reaches no dialogue.
    Silent,
    /// The script reached dialogue the text decoder cannot render.
    ///
    /// One record in the slice, in map `$1D`, hands the text service an
    /// address holding native code rather than text. Distinguished from
    /// silence because the script *did* reach a text service.
    Unsupported {
        /// Bank-relative address of the text that would not decode.
        source: u16,
    },
    /// The script stopped at a service whose length is unaccounted for.
    ///
    /// Reported rather than presented as silence: 12 of the slice's 115 record
    /// scripts stop this way, and a resident who *appears* to have nothing to
    /// say is indistinguishable from one the walker could not follow.
    Unaccounted {
        /// The `COP` signature that stopped the walk.
        service: u8,
    },
}

/// The player's own `FD` record (`$84:A129`, entry `$84:A12E`): the host
/// owns the player (`docs/house-scene.md`).
const PLAYER: u32 = 0x84_A12E;

/// `FB` compact services that loop over the engine's per-frame tile and
/// palette animation routines (`$8D:93xx`, `docs/house-scene.md`'s **C**):
/// display work, not scripts. Other compact actors, such as the town's
/// scene `$88:84EF`, are scripts and run.
const SERVICES: [u32; 2] = [0x87_98C2, 0x87_98EB];

/// Residents a map installs for a given event-flag state.
///
/// The spawn list's own `$FA` conditions choose which records are installed.
/// A record's entry script can then remove the actor again: `COP 47` despawns
/// when its condition chain holds, and the documented resident's script opens
/// with one. Such a record is left out, because the room the game shows does
/// not contain them.
///
/// # Errors
/// Propagates a spawn stream the decoder refuses.
pub fn residents(
    image: &[u8],
    map: u16,
    events: EventFlags<'_>,
) -> Result<Vec<Resident>, ResolveError> {
    // In executed order, which decides what a zero descriptor reuses.
    let present = SpawnList::resolve(image, map, events)?;
    let decoded = HouseActor::from_records(image, map, &present, |record| {
        ResidentPose::from_script(image, record, events).unwrap_or_default()
    });
    Ok(present
        .iter()
        .enumerate()
        .map(|(index, record)| {
            let actor = decoded[index].as_ref().ok();
            let initial = actor.map_or(0, HouseActor::initial);
            Resident {
                position: record.origin(),
                record: record.offset(),
                script: record.script(),
                body: actor.is_some(),
                initial,
                selector: initial,
                hflip: false,
                pose_age: 0,
                walking: false,
                descriptor: descriptor_owner(&present, index)
                    .and_then(|owner| present[owner].descriptor_offset()),
                hidden: false,
            }
        })
        .filter(|resident| {
            resident
                .script
                .is_none_or(|script| script != PLAYER && !SERVICES.contains(&script))
                && !despawns(image, resident, events)
        })
        .collect())
}

/// Whether the resident's entry script removes them under these flags.
///
/// A script the walker cannot follow is kept: a refusal is not evidence of
/// absence.
fn despawns(image: &[u8], resident: &Resident, events: EventFlags<'_>) -> bool {
    resident.script.is_some_and(|script| {
        actor_script::walk_with_events(image, script, events)
            .is_ok_and(|walked| matches!(walked.stop, actor_script::Stop::Despawned { .. }))
    })
}

/// Walks a resident's interaction and decodes the dialogue it reaches.
///
/// The entry script is what the actor does when it spawns; `$21` registers the
/// **interaction** callback, and that is what talking runs. Only the callbacks
/// are collected, so a resident's ambient text is not mistaken for their
/// conversation -- merging the two gives the documented resident three pages
/// where the chain has two.
#[must_use]
pub fn talk_to(image: &[u8], resident: &Resident, events: EventFlags<'_>) -> Conversation {
    let Some(script) = resident.script else {
        return Conversation::Silent;
    };
    let Ok(entry) = actor_script::walk_with_events(image, script, events) else {
        return Conversation::Silent;
    };
    let bank = script & 0xFF_0000;
    if entry.callbacks.is_empty() {
        return match entry.stop {
            actor_script::Stop::Unaccounted { service, .. } => {
                Conversation::Unaccounted { service }
            }
            _ => Conversation::Silent,
        };
    }
    let effects: Vec<_> = entry
        .callbacks
        .iter()
        .filter_map(|callback| {
            actor_script::walk_with_events(image, bank | u32::from(*callback), events).ok()
        })
        .collect();
    collect(image, bank, &effects)
}

fn collect(image: &[u8], bank: u32, effects: &[ScriptEffects]) -> Conversation {
    let mut pages = Vec::new();
    let mut flags = Vec::new();
    let mut undecodable = None;
    for walked in effects {
        for source in &walked.text {
            match HouseDialogue::decode_at(image, bank | u32::from(*source)) {
                Ok(decoded) => pages.extend(decoded),
                Err(_) => undecodable = undecodable.or(Some(*source)),
            }
        }
        flags.extend(walked.flags.iter().copied());
    }
    if !pages.is_empty() {
        return Conversation::Speaks { pages, flags };
    }
    if let Some(source) = undecodable {
        return Conversation::Unsupported { source };
    }
    // Nothing to say, and a refusal is the likelier reason than silence.
    for walked in effects {
        if let actor_script::Stop::Unaccounted { service, .. } = walked.stop {
            return Conversation::Unaccounted { service };
        }
    }
    Conversation::Silent
}
