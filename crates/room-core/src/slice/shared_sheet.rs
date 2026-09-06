//! The one finite AFCBB3 cache; scene occupancy always comes from a fresh profile.
use super::{
    pandora_runtime::State, Cue, GameData, GameState, PandoraData, Room, SliceError, StoryFlags,
};
use crate::pots::PotState;

/// Persistent cellar tile patch, independent of the per-load hit counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CellarDoorPatch {
    /// Original source door words.
    Closed,
    /// First-hit cracked upper cell.
    Damaged,
    /// Second-hit opened cells (scene occupancy remains separate).
    Open,
}
impl CellarDoorPatch {
    pub(super) fn decode(byte: u8) -> Result<Self, SliceError> {
        match byte {
            0 => Ok(Self::Closed),
            1 => Ok(Self::Damaged),
            2 => Ok(Self::Open),
            _ => Err(SliceError::Snapshot),
        }
    }
    pub(super) const fn words(self) -> [u16; 2] {
        match self {
            Self::Closed => [0x1d80, 0x0b81],
            Self::Damaged => [0x1da7, 0x0b81],
            Self::Open => [0x1cf6, 0x3acb],
        }
    }
}
/// Read-only resource-cache projection, not a room/actor cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedSheetOutput {
    /// Whether AFCBB3 is the resident first-layer source.
    pub resident: bool,
    /// Persistent cellar door patch.
    pub cellar: CellarDoorPatch,
    /// Source object catalog bitset, valid throughout this sheet's lifetime.
    pub consumed: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Sheet {
    pub resident: bool,
    pub cellar: CellarDoorPatch,
    // Zero while PotState owns the ledger. Never two mutable authorities.
    pub parked_consumed: u64,
}
impl Sheet {
    pub const fn new(resident: bool) -> Self {
        Self {
            resident,
            cellar: CellarDoorPatch::Closed,
            parked_consumed: 0,
        }
    }
}
// B/C/D/F/10/11 reach the identical $988410 layer instruction. E/20's
// $988458/$9885ED carry the same packed B3 CB 0B operand -> $AFCBB3.
pub(super) const fn shared(map: u16) -> bool {
    matches!(map, 0xb..=0x11 | 0x20)
}
pub(super) fn patch(room: &mut Room, cell: usize, word: u16) {
    // Do not carry the departing scene's actors into a different scene.
    let occupancy = room.cells()[cell] & 0x8000;
    room.replace_cell(cell, (word & 0x7fff) | occupancy);
}
pub(super) fn wood(room: &mut Room, open: bool) {
    let words = if open {
        [0x1cf6, 0x00f7]
    } else {
        [0x1cf2, 0x1cf3]
    };
    for (cell, word) in [19 * 32 + 8, 20 * 32 + 8].into_iter().zip(words) {
        patch(room, cell, word);
    }
}
impl State {
    pub fn consumed(self) -> u64 {
        self.pot
            .map_or(self.sheet.parked_consumed, PotState::ledger)
    }
    pub fn load(
        &mut self,
        map: u16,
        flags: &mut StoryFlags,
        wooden: &mut bool,
    ) -> Result<(), SliceError> {
        let resident = shared(map);
        // No qualified source load operation has yet been supplied to reapply
        // event292's tiles after a sheet replacement. Do not invent one.
        if map == 0xc
            && flags.contains(0x292) == Ok(true)
            && (!self.sheet.resident || self.sheet.cellar != CellarDoorPatch::Open)
        {
            return Err(SliceError::Exit);
        }
        self.sheet.parked_consumed = self.consumed();
        self.pot = None;
        self.frozen = None;
        if !resident || !self.sheet.resident {
            self.sheet = Sheet::new(resident);
            *wooden = false;
        }
        self.town_open = 0;
        self.visit_consumed = self.sheet.parked_consumed;
        self.visit_cellar = self.sheet.cellar;
        self.graph.load(map, flags);
        Ok(())
    }
    pub fn validate_sheet(
        self,
        spec: &PandoraData,
        map: u16,
        flags: &StoryFlags,
        wooden: bool,
    ) -> Result<(), SliceError> {
        let consumed = self.consumed();
        let current_hit = self.pot.is_some_and(PotState::contact_reached);
        let prior_hits = self
            .graph
            .counter
            .checked_sub(u8::from(current_hit))
            .ok_or(SliceError::Snapshot)?;
        let current_object = self
            .pot
            .is_some_and(|p| p.phase() != crate::pots::Phase::Empty);
        let required = u32::from(prior_hits) + u32::from(current_object);
        let expected = match self.graph.counter {
            0 => self.visit_cellar,
            1 => CellarDoorPatch::Damaged,
            2 => CellarDoorPatch::Open,
            _ => return Err(SliceError::Snapshot),
        };
        if self.sheet.resident != shared(map)
            || (!self.sheet.resident && (self.sheet != Sheet::new(false) || wooden))
            || (spec.objects.len() < 64 && consumed >> spec.objects.len() != 0)
            || (self.pot.is_some() && self.sheet.parked_consumed != 0)
            || self.visit_consumed & !consumed != 0
            || (consumed & !self.visit_consumed).count_ones() < required
            || self.visit_consumed.count_ones() < u32::from(self.visit_cellar as u8)
            || (self.graph.counter != 0 && (map != 0xc || self.pot.is_none()))
            || (map != 0xc
                && (self.visit_consumed != consumed || self.visit_cellar != self.sheet.cellar))
            || consumed.count_ones() < u32::from(self.sheet.cellar as u8)
            || self.sheet.cellar != expected
            || (self.visit_cellar == CellarDoorPatch::Open && self.graph.counter != 0)
            || (map == 0xc
                && flags.contains(0x292) == Ok(true)
                && self.sheet.cellar != CellarDoorPatch::Open)
            || (self.sheet.cellar == CellarDoorPatch::Open
                && flags.contains(0x292) != Ok(true)
                && self.graph.node != crate::pandora::Node::Cue(Cue::SecondHitPatched))
            || (self.sheet.cellar != CellarDoorPatch::Closed && flags.contains(0x2e) != Ok(true))
        {
            return Err(SliceError::Snapshot);
        }
        Ok(())
    }
    pub fn apply_sheet(self, room: &mut Room, spec: &PandoraData, wooden: bool) {
        if !self.sheet.resident {
            if let Some(nav) = &spec.navigation {
                for door in &nav.doors {
                    if self.town_open & door.door.mask() != 0 {
                        for p in door.patches {
                            patch(room, usize::from(p.cell), p.open);
                        }
                    }
                }
            }
            return;
        }
        wood(room, wooden);
        for (cell, word) in [20 * 32 + 11, 21 * 32 + 11]
            .into_iter()
            .zip(self.sheet.cellar.words())
        {
            patch(room, cell, word);
        }
        for (i, object) in spec.objects.iter().enumerate() {
            let word = if self.consumed() & (1 << i) != 0 {
                object.replacement
            } else {
                object.raw
            };
            patch(room, usize::from(object.cell), word);
        }
    }
}
impl GameState {
    /// Owned effective scene geometry, overlaid with the finite resident sheet patches.
    /// Unlike `current_room`, this includes consumed pots and retained door words;
    /// immutable source occupancy/halos are rebuilt for the selected scene.
    /// Profile9 returns a clone of its unchanged borrowed room.
    /// # Errors
    /// Rejects incompatible data or unavailable profiles.
    pub fn effective_room(&self, data: &GameData) -> Result<Room, SliceError> {
        let mut room = self.current_room(data)?.clone();
        if let Some(state) = self.pandora {
            state.apply_sheet(
                &mut room,
                data.pandora.as_ref().ok_or(SliceError::Data)?,
                self.wooden_door_open,
            );
        }
        Ok(room)
    }
}
