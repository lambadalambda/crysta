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
    /// Terminal choice; return a continuation key without executing its sequence.
    Choose {
        /// Immutable choice catalog key, validated by the caller.
        catalog: u16,
        /// Continuation sequence keys for selections 0, 1 and 2.
        branches: [u32; 3],
    },
}

/// Current semantic wait; presentation and continuation lookup belong to the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventWait {
    /// Immutable page key awaiting acknowledgement.
    Page(u32),
    /// Terminal choice awaiting one of three selections.
    Choice {
        /// Immutable choice catalog key.
        catalog: u16,
        /// Continuation sequence keys for selections 0, 1 and 2.
        branches: [u32; 3],
    },
}

/// Immutable sequence; bounded linear execution cannot loop or wait on a device.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Cursor at a page/choice wait or completion. Program identity is owned by the game
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
    /// Empty/oversized sequence, no wait, or a nonterminal choice.
    Sequence,
    /// Event index outside 0..512.
    Flag,
    /// Cursor is neither a page/choice wait nor the canonical completed offset.
    Cursor,
    /// The requested operation has no matching page or choice wait.
    NotWaiting,
    /// Choice selection outside 0..3.
    Selection,
}
impl core::fmt::Display for EventError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "semantic event: {self:?}")
    }
}
impl core::error::Error for EventError {}

impl EventSequence {
    /// Validate at most 256 operations with at least one page or choice wait.
    /// A choice must be last; catalog and continuation keys are caller-validated.
    /// # Errors
    /// Rejects oversized/empty sequences, absent waits, nonterminal choices or
    /// invalid flag IDs.
    pub fn new(ops: Vec<EventOp>) -> Result<Self, EventError> {
        if ops.is_empty()
            || ops.len() > 256
            || !ops
                .iter()
                .any(|op| matches!(op, EventOp::ShowPage(_) | EventOp::Choose { .. }))
            || ops
                .iter()
                .enumerate()
                .any(|(index, op)| matches!(op, EventOp::Choose { .. }) && index + 1 != ops.len())
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
    /// Immutable operations for caller-owned structural validation.
    #[must_use]
    pub fn ops(&self) -> &[EventOp] {
        &self.ops
    }
    /// Run immediate effects up to the first wait. This takes no game tick by
    /// itself; the caller owns interaction admission, tick and movement locking.
    pub fn start(&self, flags: &mut EventFlags) -> EventCursor {
        self.run(0, flags)
    }
    /// Current wait, or none at completion or an invalid cursor.
    #[must_use]
    pub fn wait(&self, cursor: EventCursor) -> Option<EventWait> {
        match self.ops.get(usize::from(cursor.0)) {
            Some(EventOp::ShowPage(key)) => Some(EventWait::Page(*key)),
            Some(EventOp::Choose { catalog, branches }) => Some(EventWait::Choice {
                catalog: *catalog,
                branches: *branches,
            }),
            _ => None,
        }
    }
    /// Current page key; no font, timer or rendering state enters this runner.
    #[must_use]
    pub fn page(&self, cursor: EventCursor) -> Option<u32> {
        match self.wait(cursor) {
            Some(EventWait::Page(key)) => Some(key),
            _ => None,
        }
    }
    /// Complete a terminal choice and return its continuation key, not an offset.
    /// The caller decides whether and when to start the continuation sequence.
    /// # Errors
    /// Rejects a cursor with no waiting choice, then selections outside 0..3.
    /// Failures leave the cursor unchanged.
    pub fn choose(&self, cursor: &mut EventCursor, selection: u8) -> Result<u32, EventError> {
        let Some(EventWait::Choice { branches, .. }) = self.wait(*cursor) else {
            return Err(EventError::NotWaiting);
        };
        let key = *branches
            .get(usize::from(selection))
            .ok_or(EventError::Selection)?;
        // Construction guarantees that a choice is last and the length is <= 256.
        cursor.0 += 1;
        Ok(key)
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
    /// Decode only a page/choice wait or completion. The owning game additionally
    /// validates source identity, active sequence, flags and control ownership.
    /// # Errors
    /// Rejects internal effect offsets and out-of-range offsets.
    pub fn restore_cursor(&self, position: u16) -> Result<EventCursor, EventError> {
        let cursor = EventCursor(position);
        if usize::from(position) == self.ops.len() || self.wait(cursor).is_some() {
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

    const CHOICE: EventOp = EventOp::Choose {
        catalog: 42,
        branches: [10, 20, 30],
    };

    #[test]
    fn choice_only_returns_each_continuation_and_completes_without_acknowledgement() {
        let sequence = EventSequence::new(vec![CHOICE]).unwrap();
        assert_eq!(sequence.ops(), &[CHOICE]);
        assert_eq!(sequence.clone(), sequence);
        for selection in 0..3 {
            let mut flags = EventFlags::new([0x80; 64]);
            let mut cursor = sequence.start(&mut flags);
            assert_eq!(cursor.position(), 0);
            assert_eq!(sequence.page(cursor), None);
            assert_eq!(
                sequence.wait(cursor),
                Some(EventWait::Choice {
                    catalog: 42,
                    branches: [10, 20, 30],
                })
            );
            assert_eq!(
                sequence.choose(&mut cursor, selection),
                Ok(10 * (u32::from(selection) + 1))
            );
            assert_eq!(cursor, sequence.restore_cursor(1).unwrap());
            assert_eq!(sequence.wait(cursor), None);
            assert_eq!(flags.bytes(), &[0x80; 64]);
            assert_eq!(
                sequence.choose(&mut cursor, selection),
                Err(EventError::NotWaiting)
            );
            assert_eq!(cursor.position(), 1);
        }
    }

    #[test]
    fn choices_must_be_terminal_and_keep_existing_validation_bounds() {
        for suffix in [EventOp::ShowPage(1), EventOp::SetFlag(0), CHOICE] {
            assert_eq!(
                EventSequence::new(vec![CHOICE, suffix]),
                Err(EventError::Sequence)
            );
        }
        assert_eq!(
            EventSequence::new(vec![EventOp::SetFlag(512), CHOICE]),
            Err(EventError::Flag)
        );
        let mut ops = vec![EventOp::SetFlag(0); 256];
        ops.push(CHOICE);
        assert_eq!(EventSequence::new(ops), Err(EventError::Sequence));
    }

    #[test]
    fn page_to_choice_applies_only_reached_flags_and_restores_waits() {
        let sequence = EventSequence::new(vec![
            EventOp::SetFlag(0),
            EventOp::ShowPage(7),
            EventOp::SetFlag(511),
            CHOICE,
        ])
        .unwrap();
        let mut flags = EventFlags::new([0; 64]);
        let mut cursor = sequence.start(&mut flags);
        assert_eq!(sequence.wait(cursor), Some(EventWait::Page(7)));
        assert!(flags.contains(0).unwrap());
        assert!(!flags.contains(511).unwrap());
        sequence.acknowledge(&mut cursor, &mut flags).unwrap();
        assert_eq!(cursor.position(), 3);
        assert!(flags.contains(511).unwrap());
        let mut restored = sequence.restore_cursor(3).unwrap();
        let before = flags.clone();
        assert_eq!(sequence.choose(&mut cursor, 2), Ok(30));
        assert_eq!(sequence.choose(&mut restored, 2), Ok(30));
        assert_eq!(cursor, restored);
        assert_eq!(cursor, sequence.restore_cursor(4).unwrap());
        assert_eq!(flags, before);
        for position in [0, 2, 5, u16::MAX] {
            assert_eq!(sequence.restore_cursor(position), Err(EventError::Cursor));
        }
        assert_eq!(sequence.restore_cursor(1).unwrap().position(), 1);
    }

    #[test]
    fn wrong_waits_invalid_selections_and_invalid_cursors_fail_atomically() {
        let sequence =
            EventSequence::new(vec![EventOp::SetFlag(0), EventOp::ShowPage(7), CHOICE]).unwrap();
        let mut flags = EventFlags::new([0x80; 64]);
        for position in [0, 1, 3, 4, u16::MAX] {
            let mut cursor = EventCursor(position);
            let before = (cursor, flags.clone());
            assert_eq!(sequence.choose(&mut cursor, 0), Err(EventError::NotWaiting));
            assert_eq!((cursor, flags.clone()), before);
            if position != 1 {
                assert_eq!(sequence.wait(cursor), None);
                assert_eq!(
                    sequence.acknowledge(&mut cursor, &mut flags),
                    Err(EventError::NotWaiting)
                );
                assert_eq!((cursor, flags.clone()), before);
            }
        }
        let mut cursor = sequence.restore_cursor(2).unwrap();
        let before = (cursor, flags.clone());
        assert_eq!(
            sequence.acknowledge(&mut cursor, &mut flags),
            Err(EventError::NotWaiting)
        );
        assert_eq!((cursor, flags.clone()), before);
        for selection in 3..=u8::MAX {
            assert_eq!(
                sequence.choose(&mut cursor, selection),
                Err(EventError::Selection)
            );
            assert_eq!((cursor, flags.clone()), before);
        }
    }

    #[test]
    fn maximum_choice_sequence_completes_at_256() {
        let mut ops = vec![EventOp::SetFlag(511); 255];
        ops.push(CHOICE);
        let sequence = EventSequence::new(ops).unwrap();
        let mut flags = EventFlags::new([0; 64]);
        let mut cursor = sequence.start(&mut flags);
        assert_eq!(cursor.position(), 255);
        assert!(flags.contains(511).unwrap());
        assert_eq!(sequence.choose(&mut cursor, 0), Ok(10));
        assert_eq!(cursor, sequence.restore_cursor(256).unwrap());
    }

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
