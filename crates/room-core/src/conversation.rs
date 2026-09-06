//! Fixed source-qualified B request graph, independent of font/raster decoding.
use crate::events::{EventCursor, EventFlags, EventOp, EventWait, FlagBlock, FlagSequence, StoryFlags};
use crate::slice::SliceError;
use alloc::{vec, vec::Vec};

/// First progression request, not room entry text.
pub const FIRST: u32 = 0x88_8ff0;
/// Repeat prompt, with no page acknowledgement before its choice.
pub const REPEAT: u32 = 0x88_9156;
const FIRST_ONE: u32 = 0x88_90d9;
const FIRST_TWO: u32 = 0x88_905a;
const REPEAT_ONE: u32 = 0x88_918c;
const REPEAT_TWO: u32 = 0x88_91d6;
const REQUESTS: [u32; 6] = [FIRST, REPEAT, FIRST_ONE, FIRST_TWO, REPEAT_ONE, REPEAT_TWO];

/// Caller-assigned immutable raster keys, in qualified source page order.
/// The two auto-completing tails are choice contexts, never acknowledged pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversationPages {
    /// $888FF0 page0 (D5).
    pub first: u32,
    /// $888FF0 page1 (D4), retained for catalog0.
    pub first_choice: u32,
    /// $889156 page0 (D4), retained for catalog1.
    pub repeat_choice: u32,
    /// $8890D9 pages0..2, final D3.
    pub first_option1: [u32; 3],
    /// $88905A pages0..2, final D3; also cancellation.
    pub first_option2: [u32; 3],
    /// $88918C pages0..1, final D3.
    pub repeat_option1: [u32; 2],
    /// $8891D6 pages0..1, final D3; also cancellation.
    pub repeat_option2: [u32; 2],
}

/// Existing B conversation API, using the 512-bit house projection.
pub type ConversationSpec = FlagConversationSpec<64>;
/// B conversation over the wider story projection; not a new runtime capability.
pub type StoryConversationSpec = FlagConversationSpec<128>;

/// Immutable six-request graph. Only its first request can change an event.
#[derive(Debug)]
pub struct FlagConversationSpec<const BYTES: usize> {
    sequences: [FlagSequence<BYTES>; 6],
    choices: [u32; 2],
}
/// Current presentation/control boundary; no frontend selection is simulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogueWait {
    /// Deliberate acknowledgement required for this raster key.
    Page(u32),
    /// Explicit result 0=cancel, 1/2=options; key is retained context, not a page wait.
    Choice {
        /// Source choice catalog (0 or 1).
        catalog: u16,
        /// Immutable raster key.
        key: u32,
    },
}
/// Stable event-level output, separate from the walking animation output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogueOutput {
    /// Source request identity, including selected follow-up continuation.
    pub request: u32,
    /// Canonical event-operation offset, not a text byte cursor.
    pub cursor: u16,
    /// Owner's next admissible action.
    pub wait: DialogueWait,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Active {
    pub request: u32,
    pub cursor: EventCursor,
}

pub(crate) fn initial_flags(granted: bool) -> EventFlags {
    let mut bytes = [0; 64];
    bytes[4] = 1 | if granted { 0x40 } else { 0 }; // $0020, optionally $0026
    bytes[31] = 8; // $00FB
    EventFlags::new(bytes)
}
pub(crate) fn initial_story_flags(granted: bool) -> StoryFlags {
    let mut bytes = [0; 128];
    bytes[..64].copy_from_slice(initial_flags(granted).bytes());
    StoryFlags::new(bytes)
}

impl ConversationSpec {
    /// Widen validated immutable operations once when attaching progression data.
    pub(crate) fn into_story(self) -> StoryConversationSpec {
        StoryConversationSpec {
            sequences: self.sequences.map(|sequence| {
                FlagSequence::new(sequence.ops().to_vec()).expect("house ops fit story storage")
            }),
            choices: self.choices,
        }
    }
}

impl<const BYTES: usize> FlagConversationSpec<BYTES> {
    /// Compile the fixed source graph, rejecting aliased raster identities.
    /// The caller binds these ordered keys and the decoded source into content identity.
    /// # Errors
    /// Rejects duplicate keys or storage too small for $26; opaque key values
    /// (including zero) are unrestricted.
    pub fn new(pages: ConversationPages) -> Result<Self, SliceError> {
        let mut keys = vec![pages.first, pages.first_choice, pages.repeat_choice];
        keys.extend(pages.first_option1);
        keys.extend(pages.first_option2);
        keys.extend(pages.repeat_option1);
        keys.extend(pages.repeat_option2);
        keys.sort_unstable();
        if keys.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(SliceError::Data);
        }
        let followup = |keys: &[u32]| {
            FlagSequence::new(
                keys.iter()
                    .copied()
                    .map(EventOp::ShowPage)
                    .collect::<Vec<_>>(),
            )
            .map_err(|_| SliceError::Data)
        };
        let first = FlagSequence::new(vec![
            EventOp::ShowPage(pages.first),
            EventOp::SetFlag(0x26),
            EventOp::Choose {
                catalog: 0,
                branches: [FIRST_TWO, FIRST_ONE, FIRST_TWO],
            },
        ])
        .map_err(|_| SliceError::Data)?;
        let repeat = FlagSequence::new(vec![EventOp::Choose {
            catalog: 1,
            branches: [REPEAT_TWO, REPEAT_ONE, REPEAT_TWO],
        }])
        .map_err(|_| SliceError::Data)?;
        Ok(Self {
            sequences: [
                first,
                repeat,
                followup(&pages.first_option1)?,
                followup(&pages.first_option2)?,
                followup(&pages.repeat_option1)?,
                followup(&pages.repeat_option2)?,
            ],
            choices: [pages.first_choice, pages.repeat_choice],
        })
    }
    fn sequence(&self, request: u32) -> Result<&FlagSequence<BYTES>, SliceError> {
        REQUESTS
            .iter()
            .position(|key| *key == request)
            .map(|i| &self.sequences[i])
            .ok_or(SliceError::Snapshot)
    }
    pub(crate) fn begin(&self, flags: &mut FlagBlock<BYTES>) -> Active {
        let request = if flags.contains(0x26).expect("bounded flag") {
            REPEAT
        } else {
            FIRST
        };
        Active {
            request,
            cursor: self.sequence(request).expect("fixed request").start(flags),
        }
    }
    pub(crate) fn output(&self, active: Active) -> Result<DialogueOutput, SliceError> {
        let wait = match self.sequence(active.request)?.wait(active.cursor) {
            Some(EventWait::Page(key)) => DialogueWait::Page(key),
            Some(EventWait::Choice { catalog, .. }) => DialogueWait::Choice {
                catalog,
                key: self.choices[usize::from(catalog)],
            },
            None => return Err(SliceError::Snapshot),
        };
        Ok(DialogueOutput {
            request: active.request,
            cursor: active.cursor.position(),
            wait,
        })
    }
    pub(crate) fn acknowledge(
        &self,
        mut active: Active,
        flags: &mut FlagBlock<BYTES>,
    ) -> Result<Option<Active>, SliceError> {
        let sequence = self.sequence(active.request)?;
        sequence
            .acknowledge(&mut active.cursor, flags)
            .map_err(|_| SliceError::Interaction)?;
        Ok(sequence.wait(active.cursor).map(|_| active))
    }
    pub(crate) fn choose(
        &self,
        mut active: Active,
        selection: u8,
        flags: &mut FlagBlock<BYTES>,
    ) -> Result<Active, SliceError> {
        let request = self
            .sequence(active.request)?
            .choose(&mut active.cursor, selection)
            .map_err(|_| SliceError::Interaction)?;
        Ok(Active {
            request,
            cursor: self.sequence(request)?.start(flags),
        })
    }
    pub(crate) fn restore(
        &self,
        request: u32,
        position: u16,
        flags: &FlagBlock<BYTES>,
    ) -> Result<Active, SliceError> {
        let sequence = self.sequence(request)?;
        let cursor = sequence
            .restore_cursor(position)
            .map_err(|_| SliceError::Snapshot)?;
        let active = Active { request, cursor };
        self.output(active)?; // completion never owns dialogue
        let granted = request != FIRST
            || sequence.ops()[..usize::from(position)].contains(&EventOp::SetFlag(0x26));
        // B owns only $26 consistency; full profile admission belongs to GameState.
        if flags.contains(0x26) != Ok(granted) {
            return Err(SliceError::Snapshot);
        }
        Ok(active)
    }
}
