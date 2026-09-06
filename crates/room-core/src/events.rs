//! Bounded semantic event sequences, not original bytecode or a native scheduler.

use alloc::vec::Vec;

/// Only the semantic operations admitted by this bounded runner.
/// Source decoding and conditional selection of a sequence belong to its caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventOp {
    /// Expose one immutable page key and wait for a deliberate acknowledgement.
    ShowPage(u32),
    /// Set a bit in the source-compatible 512-bit event block.
    SetFlag(u16),
}

/// Immutable sequence; bounded linear execution cannot loop or wait on a device.
#[derive(Debug)]
pub struct EventSequence {
    ops: Vec<EventOp>,
}

/// Canonical event bits, initialized by the caller's source-qualified bootstrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventFlags([u8; 64]);
impl EventFlags {
    /// Construct from semantic startup or snapshot bytes, not reference WRAM.
    #[must_use]
    pub const fn new(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }
    /// Canonical snapshot bytes; bit numbering is low-bit-first within each byte.
    #[must_use]
    pub const fn bytes(&self) -> &[u8; 64] {
        &self.0
    }
    /// Query an admitted event index.
    /// # Errors
    /// Rejects indices outside the 512-bit event block.
    pub fn contains(&self, flag: u16) -> Result<bool, EventError> {
        let byte = self.0.get(usize::from(flag / 8)).ok_or(EventError::Flag)?;
        Ok(byte & (1 << (flag & 7)) != 0)
    }
}

/// Cursor at a page wait or completion. Program identity is owned by the game
/// snapshot's immutable data identity and active sequence ID, not this offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventCursor(u16);
impl EventCursor {
    /// Canonical offset for the owning game's versioned snapshot.
    #[must_use]
    pub const fn position(self) -> u16 {
        self.0
    }
}

/// Invalid data or acknowledgement; failed operations do not modify state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventError {
    /// Empty/oversized sequence or a sequence with no dialogue wait.
    Sequence,
    /// Event index outside 0..512.
    Flag,
    /// Cursor is neither a dialogue wait nor the canonical completed offset.
    Cursor,
    /// No page is waiting for acknowledgement.
    NotWaiting,
}
impl core::fmt::Display for EventError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "semantic event: {self:?}")
    }
}
impl core::error::Error for EventError {}

impl EventSequence {
    /// Validate a sequence with at most 256 operations and at least one page.
    /// # Errors
    /// Rejects oversized/empty sequences, absent page waits or invalid flag IDs.
    pub fn new(ops: Vec<EventOp>) -> Result<Self, EventError> {
        if ops.is_empty()
            || ops.len() > 256
            || !ops.iter().any(|op| matches!(op, EventOp::ShowPage(_)))
        {
            return Err(EventError::Sequence);
        }
        if ops
            .iter()
            .any(|op| matches!(op, EventOp::SetFlag(flag) if *flag >= 512))
        {
            return Err(EventError::Flag);
        }
        Ok(Self { ops })
    }
    /// Run immediate effects up to the first page. This takes no game tick by
    /// itself; the caller owns interaction admission, tick and movement locking.
    pub fn start(&self, flags: &mut EventFlags) -> EventCursor {
        self.run(0, flags)
    }
    /// Current page key; no font, timer or rendering state enters this runner.
    #[must_use]
    pub fn page(&self, cursor: EventCursor) -> Option<u32> {
        match self.ops.get(usize::from(cursor.0)) {
            Some(EventOp::ShowPage(key)) => Some(*key),
            _ => None,
        }
    }
    /// Acknowledge exactly one page, then execute effects up to the next wait.
    /// # Errors
    /// Rejects a cursor with no waiting page, leaving cursor and flags unchanged.
    pub fn acknowledge(
        &self,
        cursor: &mut EventCursor,
        flags: &mut EventFlags,
    ) -> Result<(), EventError> {
        if self.page(*cursor).is_none() {
            return Err(EventError::NotWaiting);
        }
        *cursor = self.run(cursor.0 + 1, flags);
        Ok(())
    }
    /// Decode only a page wait or completion. The owning game additionally
    /// validates source identity, active sequence, flags and control ownership.
    /// # Errors
    /// Rejects internal effect offsets and out-of-range offsets.
    pub fn restore_cursor(&self, position: u16) -> Result<EventCursor, EventError> {
        let cursor = EventCursor(position);
        if usize::from(position) == self.ops.len() || self.page(cursor).is_some() {
            Ok(cursor)
        } else {
            Err(EventError::Cursor)
        }
    }
    fn run(&self, mut position: u16, flags: &mut EventFlags) -> EventCursor {
        while let Some(EventOp::SetFlag(flag)) = self.ops.get(usize::from(position)) {
            // Validated at construction; no mid-sequence failure can leak effects.
            flags.0[usize::from(flag / 8)] |= 1 << (flag & 7);
            position += 1;
        }
        EventCursor(position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn acknowledgements_expose_one_page_and_apply_only_reached_effects() {
        let sequence = EventSequence::new(vec![
            EventOp::ShowPage(10),
            EventOp::ShowPage(11),
            EventOp::SetFlag(0x26),
        ])
        .unwrap();
        let mut flags = EventFlags::new([0; 64]);
        let mut cursor = sequence.start(&mut flags);
        assert_eq!(sequence.page(cursor), Some(10));
        assert!(!flags.contains(0x26).unwrap());
        sequence.acknowledge(&mut cursor, &mut flags).unwrap();
        assert_eq!(sequence.page(cursor), Some(11));
        assert!(!flags.contains(0x26).unwrap());
        sequence.acknowledge(&mut cursor, &mut flags).unwrap();
        assert_eq!(sequence.page(cursor), None);
        assert!(flags.contains(0x26).unwrap());
        let finished = (cursor, flags.clone());
        assert_eq!(
            sequence.acknowledge(&mut cursor, &mut flags),
            Err(EventError::NotWaiting)
        );
        assert_eq!((cursor, flags), finished);
    }

    #[test]
    fn sequence_validation_bounds_work_and_never_ignores_bad_operations() {
        assert!(EventSequence::new(vec![]).is_err());
        assert!(EventSequence::new(vec![EventOp::ShowPage(1); 257]).is_err());
        assert!(matches!(
            EventSequence::new(vec![EventOp::ShowPage(1), EventOp::SetFlag(512)]),
            Err(EventError::Flag)
        ));
        assert!(EventSequence::new(vec![EventOp::SetFlag(1)]).is_err());
        assert!(EventFlags::new([0; 64]).contains(512).is_err());
    }

    #[test]
    fn cursor_encoding_has_only_page_waits_and_completion_as_valid_states() {
        let sequence = EventSequence::new(vec![
            EventOp::ShowPage(7),
            EventOp::SetFlag(511),
            EventOp::ShowPage(8),
        ])
        .unwrap();
        for value in [0, 2, 3] {
            let cursor = sequence.restore_cursor(value).unwrap();
            assert_eq!(cursor.position(), value);
        }
        for value in [1, 4, u16::MAX] {
            assert_eq!(sequence.restore_cursor(value), Err(EventError::Cursor));
        }
        let mut flags = EventFlags::new([0; 64]);
        let mut cursor = sequence.start(&mut flags);
        sequence.acknowledge(&mut cursor, &mut flags).unwrap();
        let mut restored = sequence.restore_cursor(cursor.position()).unwrap();
        let mut restored_flags = EventFlags::new(*flags.bytes());
        sequence.acknowledge(&mut cursor, &mut flags).unwrap();
        sequence
            .acknowledge(&mut restored, &mut restored_flags)
            .unwrap();
        assert_eq!((cursor, flags), (restored, restored_flags));
    }

    #[test]
    fn maximum_immediate_run_finishes_at_256_and_sets_raw_source_bits() {
        let mut ops = vec![EventOp::ShowPage(1)];
        ops.extend([0, 7, 8, 511].into_iter().map(EventOp::SetFlag));
        ops.resize(256, EventOp::SetFlag(0));
        let sequence = EventSequence::new(ops).unwrap();
        let mut initial = [0; 64];
        initial[31] = 8;
        let mut flags = EventFlags::new(initial);
        let mut cursor = sequence.start(&mut flags);
        sequence.acknowledge(&mut cursor, &mut flags).unwrap();
        assert_eq!(cursor.position(), 256);
        assert_eq!(sequence.restore_cursor(256).unwrap(), cursor);
        initial[0] = 0x81;
        initial[1] = 1;
        initial[63] = 0x80;
        assert_eq!(flags.bytes(), &initial);
    }

    #[test]
    fn initial_flags_and_between_page_effects_are_preserved() {
        let mut initial = [0; 64];
        initial[31] = 0x80;
        let mut flags = EventFlags::new(initial);
        let sequence = EventSequence::new(vec![
            EventOp::SetFlag(0),
            EventOp::ShowPage(1),
            EventOp::SetFlag(511),
            EventOp::ShowPage(2),
        ])
        .unwrap();
        let mut cursor = sequence.start(&mut flags);
        assert!(flags.contains(0).unwrap());
        assert!(flags.contains(255).unwrap());
        assert!(!flags.contains(511).unwrap());
        sequence.acknowledge(&mut cursor, &mut flags).unwrap();
        assert_eq!(sequence.page(cursor), Some(2));
        assert!(flags.contains(511).unwrap());
        assert!(flags.contains(255).unwrap());
    }
}
