//! Residents placed from spawn lists, and the dialogue they carry.
//!
//! A spawn record says where someone stands and points at the script that runs
//! when the player talks to them. [`assets::maps::actors`] resolves the first
//! and [`assets::maps::actor_script`] walks the second; this joins them into
//! something a runtime can put in a room.

use assets::maps::actor_script::{self, ScriptEffects};
use assets::maps::actors::{ResolveError, SpawnList};
use assets::maps::scripts::EventFlags;
use assets::sprites::{HouseActor, ResidentPose};
use assets::text::{DialoguePage, HouseDialogue};

/// Someone standing in a map.
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
    /// Choice prompts are the case in the slice: the documented resident's
    /// first-visit line at `$88:95B3` is one. Distinguished from silence
    /// because the script *did* reach text.
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
    let present = SpawnList::resolve(image, map, events)?;
    // Bodies are decided over the whole list, present or not, because a
    // record may reuse the resource of the record before it.
    let list = SpawnList::from_rom(image, map).map_err(ResolveError::Decode)?;
    let decoded = HouseActor::from_records(image, map, list.records(), |record| {
        ResidentPose::from_script(image, record, events).unwrap_or_default()
    });
    let is_body = |offset: usize| {
        list.records()
            .iter()
            .position(|record| record.offset() == offset)
            .is_some_and(|index| decoded[index].is_ok())
    };
    Ok(present
        .iter()
        .map(|record| Resident {
            position: record.origin(),
            record: record.offset(),
            script: record.script(),
            body: is_body(record.offset()),
        })
        .filter(|resident| !despawns(image, resident, events))
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
