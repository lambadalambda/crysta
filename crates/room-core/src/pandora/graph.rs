//! Private fixed continuation. Geometry and pacing are deliberately absent.
#[allow(clippy::enum_glob_use)] // A single fixed graph, not arbitrary dispatch.
use super::Invocation::*;
use super::{Invocation, PandoraText};
use crate::conversation::{DialogueOutput, DialogueWait};
use crate::events::StoryFlags;
use crate::slice::SliceError;

/// A qualified presentation completion, not a frontend acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    /// Return/presentation following this exact invocation (including repeated D720).
    Returned(Invocation),
    /// Source second-hit patches/temporary occupancy precede COP07 $8292.
    SecondHitPatched,
    /// Source box opening sets $22 before a same-map reload.
    BoxReload,
    /// Masked COPDF handoff wait; completion certifies script readiness, before grant22.
    BoxAcquireControl,
    /// Cooperating color-math worker writes local4 before speaker request.
    ReactionColorMath,
    /// Worker writes local6 after the speaker has requested local5.
    ReactionColorReturn,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Node {
    Control,
    Request(Invocation, u8),
    Cue(Cue),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Runtime {
    pub node: Node,
    pub counter: u8,
}
fn has(flags: &StoryFlags, bit: u16) -> bool {
    flags.contains(bit) == Ok(true)
}
fn set(flags: &mut StoryFlags, bit: u16, value: bool) {
    let mut bytes = *flags.bytes();
    if value {
        bytes[usize::from(bit / 8)] |= 1 << (bit % 8);
    } else {
        bytes[usize::from(bit / 8)] &= !(1 << (bit % 8));
    }
    *flags = StoryFlags::new(bytes);
}
impl Runtime {
    pub const fn new() -> Self {
        Self {
            node: Node::Control,
            counter: 0,
        }
    }
    fn request(&mut self, invocation: Invocation) {
        self.node = Node::Request(invocation, 0);
    }
    /// Every reconstruction, even same-map, resets exactly locals0..31 + counter.
    pub fn reset_visit(&mut self, flags: &mut StoryFlags) {
        let mut bytes = *flags.bytes();
        bytes[..4].fill(0);
        *flags = StoryFlags::new(bytes);
        self.counter = 0;
    }
    pub fn load(&mut self, map: u16, flags: &mut StoryFlags) {
        self.reset_visit(flags);
        if map == 0xc && has(flags, 0x28) && !has(flags, 0x27) {
            set(flags, 0x27, true);
            self.request(Invocation::CEntry);
        } else if map == 0x21 && !has(flags, 0x22) {
            self.request(Invocation::BoxEntry);
        }
    }
    pub fn resident(&mut self, map: u16, flags: &StoryFlags) -> Result<(), SliceError> {
        if self.node != Node::Control
            || map != 0x13
            || !has(flags, 0x26)
            || [0x28, 0x27, 0x74, 0x101, 0x109]
                .iter()
                .any(|b| has(flags, *b))
        {
            return Err(SliceError::Interaction);
        }
        self.request(if has(flags, 1) {
            Invocation::ResidentRetry
        } else {
            Invocation::ResidentFirst
        });
        Ok(())
    }
    pub fn dialogue(self, text: &PandoraText) -> Option<DialogueOutput> {
        let Node::Request(invocation, page) = self.node else {
            return None;
        };
        let pages = text.pages(invocation);
        let choice = Invocation::request_shape(invocation.source())
            .expect("fixed invocation")
            .1;
        Some(DialogueOutput {
            request: invocation.source(),
            cursor: u16::from(page),
            wait: if choice && usize::from(page) + 1 == pages.len() {
                DialogueWait::Choice {
                    catalog: 1,
                    key: pages[usize::from(page)],
                }
            } else {
                DialogueWait::Page(pages[usize::from(page)])
            },
        })
    }
    pub fn acknowledge(
        &mut self,
        text: &PandoraText,
        flags: &mut StoryFlags,
    ) -> Result<(), SliceError> {
        if !matches!(
            self.dialogue(text).map(|d| d.wait),
            Some(DialogueWait::Page(_))
        ) {
            return Err(SliceError::Interaction);
        }
        let Node::Request(invocation, page) = self.node else {
            unreachable!()
        };
        if usize::from(page) + 1 < text.pages(invocation).len() {
            self.node = Node::Request(invocation, page + 1);
        } else {
            use Invocation::{BoxEntry, CDirect, ResidentGrant, ResidentRefusal, TourFinal};
            match invocation {
                ResidentGrant => {
                    set(flags, 0x28, true);
                    self.node = Node::Control;
                }
                ResidentRefusal => {
                    set(flags, 1, true);
                    self.node = Node::Control;
                }
                CDirect => {
                    set(flags, 0x2e, true);
                    self.node = Node::Control;
                }
                TourFinal => {
                    set(flags, 0x244, true);
                    self.node = Node::Control;
                }
                BoxEntry => self.node = Node::Control,
                _ => self.node = Node::Cue(Cue::Returned(invocation)),
            }
        }
        Ok(())
    }
    pub fn choose(&mut self, text: &PandoraText, choice: u8) -> Result<(), SliceError> {
        if choice > 2
            || !matches!(
                self.dialogue(text).map(|d| d.wait),
                Some(DialogueWait::Choice { .. })
            )
        {
            return Err(SliceError::Interaction);
        }
        let Node::Request(invocation, _) = self.node else {
            unreachable!()
        };
        let next = match (invocation, choice) {
            (ResidentFirst | ResidentRetry, 1) => ResidentGrant,
            (ResidentFirst | ResidentRetry, 0 | 2) => ResidentRefusal,
            (CChoice, 1) => CDirect,
            _ => return Err(SliceError::Interaction), // C cancel/result2 is NOT direct or 2F.
        };
        self.request(next);
        Ok(())
    }
    /// Called only for an actual `PotState` output, never consumption or an A attempt.
    pub fn hit(&mut self, flags: &mut StoryFlags) -> Result<(), SliceError> {
        if self.node != Node::Control
            || !has(flags, 0x2e)
            || !has(flags, 0x28)
            || has(flags, 0x292)
            || self.counter >= 2
        {
            return Err(SliceError::Interaction);
        }
        self.counter += 1;
        if self.counter == 1 {
            set(flags, 1, true);
            self.request(Invocation::FirstHit);
        } else {
            set(flags, 2, true);
            self.node = Node::Cue(Cue::SecondHitPatched);
        }
        Ok(())
    }
    /// Admission itself is qualified by aggregate collision/contact data.
    pub fn contact(&mut self, flags: &mut StoryFlags) -> Result<(), SliceError> {
        if self.node != Node::Control || has(flags, 0x22) || !has(flags, 0x292) {
            return Err(SliceError::Interaction);
        }
        if has(flags, 1) || has(flags, 2) {
            return Err(SliceError::Interaction);
        }
        set(flags, 1, true);
        self.request(Invocation::BoxWarning);
        Ok(())
    }
    /// Spatial polling is admitted separately by the immutable raw-coordinate gate.
    /// Taking control does not prove COPDF success and cannot grant22 yet.
    pub fn poll_opening(&mut self, flags: &StoryFlags) -> bool {
        if self.node != Node::Control
            || !has(flags, 1)
            || !has(flags, 2)
            || !has(flags, 0x292)
            || has(flags, 0x22)
        {
            return false;
        }
        self.node = Node::Cue(Cue::BoxAcquireControl);
        true
    }
    /// A typed boundary can only complete the currently owned source continuation.
    /// The aggregate performs any required load/pose sequence before invoking this.
    pub fn complete(&mut self, cue: Cue, flags: &mut StoryFlags) -> Result<(), SliceError> {
        if self.node != Node::Cue(cue) {
            return Err(SliceError::Interaction);
        }
        let next = match cue {
            Cue::SecondHitPatched => {
                set(flags, 0x292, true);
                Some(SecondHit)
            }
            Cue::BoxAcquireControl => {
                set(flags, 0x22, true);
                self.node = Node::Cue(Cue::BoxReload);
                return Ok(());
            }
            Cue::BoxReload => Some(OpeningFirst),
            Cue::ReactionColorMath => {
                set(flags, 4, true);
                Some(ReactionSpeaker)
            }
            Cue::ReactionColorReturn => {
                set(flags, 6, true);
                Some(ReactionRequest)
            }
            Cue::Returned(i) => match i {
                CEntry => Some(CApproach),
                CApproach => Some(CChoice),
                FirstHit => {
                    set(flags, 1, false);
                    None
                }
                SecondHit => {
                    set(flags, 2, false);
                    self.node = Node::Cue(Cue::ReactionColorMath);
                    return Ok(());
                }
                ReactionSpeaker => {
                    set(flags, 5, true);
                    self.node = Node::Cue(Cue::ReactionColorReturn);
                    return Ok(());
                }
                ReactionRequest => {
                    set(flags, 7, true);
                    Some(ReactionRight)
                }
                ReactionRight => {
                    set(flags, 8, true);
                    Some(ReactionLeft)
                }
                ReactionLeft => {
                    set(flags, 9, true);
                    Some(ReactionFinal)
                }
                ReactionFinal => None,
                BoxWarning => {
                    set(flags, 2, true);
                    None
                }
                OpeningFirst => {
                    set(flags, 10, true);
                    Some(OpeningSecond)
                }
                OpeningSecond => {
                    set(flags, 10, false);
                    Some(OpeningThird)
                }
                OpeningThird => {
                    set(flags, 10, true);
                    Some(OpeningFourth)
                }
                OpeningFourth => Some(TourIntro),
                TourIntro => Some(TourOne),
                TourOne => Some(TourTwo),
                TourTwo => Some(TourThree),
                TourThree => Some(TourFour),
                TourFour => Some(TourFive),
                TourFive => Some(TourSix),
                TourSix => Some(TourLeave41),
                TourLeave41 => Some(Tour44),
                Tour44 => Some(TourLeave44),
                TourLeave44 => Some(Tour42),
                Tour42 => Some(TourLeave42),
                TourLeave42 => Some(Tour43),
                Tour43 => Some(TourLeave43),
                TourLeave43 => {
                    set(flags, 0x243, true);
                    Some(TourFinal)
                }
                _ => return Err(SliceError::Interaction),
            },
        };
        if let Some(i) = next {
            self.request(i);
        } else {
            self.node = Node::Control;
        }
        Ok(())
    }
}

impl Runtime {
    pub fn encode(self) -> [u8; 4] {
        let (tag, id, page) = match self.node {
            Node::Control => (0, 0, 0),
            Node::Request(i, p) => (1, i as u8, p),
            Node::Cue(Cue::Returned(i)) => (2, i as u8, 0),
            Node::Cue(Cue::SecondHitPatched) => (3, 0, 0),
            Node::Cue(Cue::BoxReload) => (4, 0, 0),
            Node::Cue(Cue::ReactionColorMath) => (5, 0, 0),
            Node::Cue(Cue::ReactionColorReturn) => (6, 0, 0),
            Node::Cue(Cue::BoxAcquireControl) => (7, 0, 0),
        };
        [tag, id, page, self.counter]
    }
    pub fn decode(bytes: [u8; 4], map: u16, flags: &StoryFlags) -> Result<Self, SliceError> {
        let invocation = |id| {
            Invocation::ALL
                .into_iter()
                .chain([Invocation::ResidentRetry, Invocation::ResidentRefusal])
                .find(|i| *i as u8 == id)
                .ok_or(SliceError::Snapshot)
        };
        let node = match bytes {
            [0, 0, 0, _] => Node::Control,
            [1, id, page, _] => Node::Request(invocation(id)?, page),
            [2, id, 0, _] => Node::Cue(Cue::Returned(invocation(id)?)),
            [3, 0, 0, _] => Node::Cue(Cue::SecondHitPatched),
            [4, 0, 0, _] => Node::Cue(Cue::BoxReload),
            [5, 0, 0, _] => Node::Cue(Cue::ReactionColorMath),
            [6, 0, 0, _] => Node::Cue(Cue::ReactionColorReturn),
            [7, 0, 0, _] => Node::Cue(Cue::BoxAcquireControl),
            _ => return Err(SliceError::Snapshot),
        };
        let state = Self {
            node,
            counter: bytes[3],
        };
        if !state.validate(map, flags) {
            return Err(SliceError::Snapshot);
        }
        Ok(state)
    }
    #[allow(clippy::too_many_lines)] // Keep coupled stage/flag/local validation together.
    pub fn validate(self, map: u16, flags: &StoryFlags) -> bool {
        let allowed = [
            0x20, 0xfb, 0x26, 0x28, 0x27, 0x2e, 0x292, 0x22, 0x243, 0x244,
        ];
        if !has(flags, 0x20)
            || !has(flags, 0xfb)
            || self.counter > 2
            || (map != 0xc && self.counter != 0)
        {
            return false;
        }
        for bit in 32..1024 {
            if has(flags, bit) && !allowed.contains(&bit) {
                return false;
            }
        }
        let grants = [0x26, 0x28, 0x27, 0x2e, 0x292, 0x22, 0x243, 0x244];
        if grants
            .windows(2)
            .any(|p| has(flags, p[1]) && !has(flags, p[0]))
        {
            return false;
        }
        if has(flags, 0x244) && (map != 0x41 || self.node != Node::Control) {
            return false;
        }
        let locals = u32::from_le_bytes(flags.bytes()[..4].try_into().expect("four bytes"));
        let invocation = match self.node {
            Node::Request(i, page) => {
                if page
                    >= Invocation::request_shape(i.source())
                        .expect("fixed shape")
                        .0
                {
                    return false;
                }
                Some(i)
            }
            Node::Cue(Cue::Returned(i)) => {
                if matches!(
                    i,
                    Invocation::ResidentFirst
                        | Invocation::ResidentRetry
                        | Invocation::ResidentRefusal
                        | Invocation::ResidentGrant
                        | Invocation::CChoice
                        | Invocation::CDirect
                        | Invocation::BoxEntry
                        | Invocation::TourFinal
                ) {
                    return false;
                }
                Some(i)
            }
            Node::Cue(Cue::SecondHitPatched) => {
                return map == 0xc
                    && self.counter == 2
                    && locals == 4
                    && has(flags, 0x2e)
                    && !has(flags, 0x292)
            }
            Node::Cue(Cue::BoxAcquireControl) => {
                return map == 0x21 && locals == 6 && has(flags, 0x292) && !has(flags, 0x22);
            }
            Node::Cue(Cue::BoxReload) => {
                return map == 0x21
                    && matches!(locals, 0 | 6)
                    && has(flags, 0x22)
                    && !has(flags, 0x243)
            }
            Node::Cue(Cue::ReactionColorMath | Cue::ReactionColorReturn) => {
                return map == 0xc
                    && self.counter == 2
                    && locals
                        == if self.node == Node::Cue(Cue::ReactionColorMath) {
                            0
                        } else {
                            0x30
                        }
                    && has(flags, 0x292)
                    && !has(flags, 0x22)
            }
            Node::Control => None,
        };
        if let Some(i) = invocation {
            let (expected_map, prefix, expected_locals) = match i {
                ResidentFirst => (0x13, 1, 0),
                ResidentRetry => (0x13, 1, 2),
                ResidentGrant | ResidentRefusal => (0x13, 1, locals & 2),
                CEntry | CApproach | CChoice | CDirect => (0xc, 3, 0),
                FirstHit => (0xc, 4, 2),
                SecondHit => (0xc, 5, 4),
                ReactionSpeaker => (0xc, 5, 0x10),
                ReactionRequest => (0xc, 5, 0x70),
                ReactionRight => (0xc, 5, 0xf0),
                ReactionLeft => (0xc, 5, 0x1f0),
                ReactionFinal => (0xc, 5, 0x3f0),
                BoxEntry => (0x21, 5, 0),
                BoxWarning => (0x21, 5, 2),
                OpeningFirst | OpeningThird => (0x21, 6, 0),
                OpeningSecond | OpeningFourth => (0x21, 6, 0x400),
                TourIntro | TourOne | TourTwo | TourThree | TourFour | TourFive | TourSix
                | TourLeave41 => (0x41, 6, 0),
                Tour44 | TourLeave44 => (0x44, 6, 0),
                Tour42 | TourLeave42 => (0x42, 6, 0),
                Tour43 | TourLeave43 => (0x43, 6, 0),
                TourFinal => (0x41, 7, 0),
            };
            let transition_target = match self.node {
                Node::Cue(Cue::Returned(OpeningFourth | TourLeave43)) => Some(0x41),
                Node::Cue(Cue::Returned(TourLeave41)) => Some(0x44),
                Node::Cue(Cue::Returned(TourLeave44)) => Some(0x42),
                Node::Cue(Cue::Returned(TourLeave42)) => Some(0x43),
                _ => None,
            };
            let loaded = transition_target == Some(map);
            if (map != expected_map && !loaded)
                || locals != if loaded { 0 } else { expected_locals }
            {
                return false;
            }
            if grants
                .iter()
                .enumerate()
                .any(|(n, b)| has(flags, *b) != (n < prefix))
            {
                return false;
            }
            let counter = match i {
                FirstHit => 1,
                SecondHit | ReactionSpeaker | ReactionRequest | ReactionRight | ReactionLeft
                | ReactionFinal => 2,
                _ => 0,
            };
            return self.counter == counter;
        }
        if has(flags, 0x22) && !has(flags, 0x244) {
            return false;
        }
        match map {
            0x13 => matches!(locals, 0 | 2) && self.counter == 0 && has(flags, 0x26),
            0xc if has(flags, 0x292) => matches!((self.counter, locals), (0, 0) | (2, 0x3f0)),
            0xc => {
                (!has(flags, 0x28) || has(flags, 0x2e))
                    && locals == 0
                    && self.counter <= 1
                    && (self.counter == 0 || has(flags, 0x2e))
            }
            0x21 => matches!(locals, 0 | 6) && has(flags, 0x292) && !has(flags, 0x22),
            0x41 => locals == 0 && has(flags, 0x244),
            0xe | 0x20 => locals == 0 && has(flags, 0x292) && !has(flags, 0x22),
            0xa | 0xb | 0xd | 0xf | 0x10 | 0x11 => locals == 0 && !has(flags, 0x22),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pandora::RequestPages;
    use alloc::vec::Vec;
    fn text() -> PandoraText {
        let mut requests = Vec::new();
        for i in Invocation::ALL
            .into_iter()
            .chain([Invocation::ResidentRetry, Invocation::ResidentRefusal])
        {
            if requests
                .iter()
                .any(|r: &RequestPages| r.source == i.source())
            {
                continue;
            }
            let (count, choice) = Invocation::request_shape(i.source()).unwrap();
            requests.push(RequestPages {
                source: i.source(),
                pages: (0..count).map(|p| i.source() * 16 + u32::from(p)).collect(),
                choice,
            });
        }
        PandoraText::new(requests).unwrap()
    }
    fn flags() -> StoryFlags {
        let mut f = StoryFlags::new([0; 128]);
        for b in [0x20, 0xfb, 0x26] {
            set(&mut f, b, true);
        }
        f
    }
    fn ack_all(r: &mut Runtime, text: &PandoraText, f: &mut StoryFlags) {
        while matches!(
            r.dialogue(text).map(|d| d.wait),
            Some(DialogueWait::Page(_))
        ) {
            let map = match r.node {
                Node::Request(
                    Invocation::ResidentFirst
                    | Invocation::ResidentRetry
                    | Invocation::ResidentRefusal
                    | Invocation::ResidentGrant,
                    _,
                ) => 0x13,
                Node::Request(i, _) if (i as u8) <= Invocation::ReactionFinal as u8 => 0xc,
                Node::Request(i, _) if (i as u8) <= Invocation::OpeningFourth as u8 => 0x21,
                Node::Request(Invocation::Tour44 | Invocation::TourLeave44, _) => 0x44,
                Node::Request(Invocation::Tour42 | Invocation::TourLeave42, _) => 0x42,
                Node::Request(Invocation::Tour43 | Invocation::TourLeave43, _) => 0x43,
                _ => 0x41,
            };
            let mut restored = Runtime::decode(r.encode(), map, f).unwrap();
            let mut copy_flags = f.clone();
            r.acknowledge(text, f).unwrap();
            restored.acknowledge(text, &mut copy_flags).unwrap();
            assert_eq!(*r, restored);
            assert_eq!(*f, copy_flags);
        }
    }
    #[test]
    fn refusal_retry_and_four_page_grant_then_c_direct_only() {
        let t = text();
        for selection in [0, 1, 2] {
            let mut r = Runtime::new();
            let mut f = flags();
            r.load(0x13, &mut f);
            r.resident(0x13, &f).unwrap();
            r.acknowledge(&t, &mut f).unwrap();
            r.choose(&t, selection).unwrap();
            if selection != 1 {
                ack_all(&mut r, &t, &mut f);
                assert!(has(&f, 1));
                assert!(!has(&f, 0x28));
                r.resident(0x13, &f).unwrap();
                assert_eq!(r.node, Node::Request(Invocation::ResidentRetry, 0));
                r.choose(&t, 1).unwrap();
            }
            for _ in 0..3 {
                r.acknowledge(&t, &mut f).unwrap();
                assert!(!has(&f, 0x28));
            }
            r.acknowledge(&t, &mut f).unwrap();
            assert!(has(&f, 0x28));
            r.load(0xc, &mut f);
            assert!(has(&f, 0x27));
            assert!(!has(&f, 1));
            for i in [Invocation::CEntry, Invocation::CApproach] {
                ack_all(&mut r, &t, &mut f);
                r.complete(Cue::Returned(i), &mut f).unwrap();
            }
            ack_all(&mut r, &t, &mut f);
            let before = r;
            for bad in [0, 2, 3] {
                assert!(r.choose(&t, bad).is_err());
                assert_eq!(r, before);
            }
            r.choose(&t, 1).unwrap();
            r.acknowledge(&t, &mut f).unwrap();
            assert!(!has(&f, 0x2e));
            r.acknowledge(&t, &mut f).unwrap();
            assert!(has(&f, 0x2e));
            assert!(!has(&f, 0x2f));
            assert!(r.validate(0xc, &f));
        }
    }
    #[test]
    fn warning_is_two_pages_delay_then_poll_handoff_and_same_map_reset() {
        let t = text();
        let mut r = Runtime::new();
        let mut f = flags();
        for b in [0x28, 0x27, 0x2e, 0x292] {
            set(&mut f, b, true);
        }
        r.load(0x21, &mut f);
        ack_all(&mut r, &t, &mut f);
        r.contact(&mut f).unwrap();
        r.acknowledge(&t, &mut f).unwrap();
        assert!(r.contact(&mut f).is_err());
        assert!(!has(&f, 2));
        r.acknowledge(&t, &mut f).unwrap();
        assert!(!has(&f, 2));
        assert!(!has(&f, 0x22));
        r.complete(Cue::Returned(Invocation::BoxWarning), &mut f)
            .unwrap();
        assert!(has(&f, 2));
        assert!(!has(&f, 0x22));
        assert!(r.poll_opening(&f));
        assert!(!has(&f, 0x22));
        r.complete(Cue::BoxAcquireControl, &mut f).unwrap();
        assert!(has(&f, 0x22));
        assert_ne!(r.node, Node::Control);
        r.load(0x21, &mut f);
        assert_eq!(&f.bytes()[..4], &[0; 4]);
        assert_eq!(r.counter, 0);
        assert!(has(&f, 0x292));
        r.complete(Cue::BoxReload, &mut f).unwrap();
        assert!(r.validate(0x21, &f));
    }
    #[test]
    fn actual_hits_and_distinct_second_patch_grant() {
        let t = text();
        let mut r = Runtime::new();
        let mut f = flags();
        for b in [0x28, 0x27, 0x2e] {
            set(&mut f, b, true);
        }
        r.hit(&mut f).unwrap();
        assert_eq!(r.counter, 1);
        assert!(has(&f, 1));
        ack_all(&mut r, &t, &mut f);
        r.complete(Cue::Returned(Invocation::FirstHit), &mut f)
            .unwrap();
        assert!(!has(&f, 1));
        r.hit(&mut f).unwrap();
        assert_eq!(r.counter, 2);
        assert!(!has(&f, 0x292));
        r.complete(Cue::SecondHitPatched, &mut f).unwrap();
        assert!(has(&f, 0x292));
        assert!(r.hit(&mut f).is_err());
    }
    #[test]
    fn forced_tour_never_releases_control_before_four_page_final_return() {
        let t = text();
        let mut r = Runtime::new();
        let mut f = flags();
        for b in [0x28, 0x27, 0x2e, 0x292, 0x22] {
            set(&mut f, b, true);
        }
        r.node = Node::Cue(Cue::BoxReload);
        r.complete(Cue::BoxReload, &mut f).unwrap();
        let mut map = 0x21;
        loop {
            assert_ne!(r.node, Node::Control);
            assert!(!has(&f, 0x244));
            let Node::Request(i, _) = r.node else {
                panic!("request");
            };
            if i == Invocation::TourFinal {
                break;
            }
            ack_all(&mut r, &t, &mut f);
            let load = match i {
                Invocation::OpeningFourth | Invocation::TourLeave43 => Some(0x41),
                Invocation::TourLeave41 => Some(0x44),
                Invocation::TourLeave44 => Some(0x42),
                Invocation::TourLeave42 => Some(0x43),
                _ => None,
            };
            if let Some(next) = load {
                map = next;
                r.load(map, &mut f);
            }
            r.complete(Cue::Returned(i), &mut f).unwrap();
            assert_eq!(has(&f, 0x243), i == Invocation::TourLeave43);
        }
        assert_eq!(map, 0x41);
        assert!(has(&f, 0x243));
        for _ in 0..3 {
            r.acknowledge(&t, &mut f).unwrap();
            assert!(!has(&f, 0x244));
        }
        r.acknowledge(&t, &mut f).unwrap();
        assert!(has(&f, 0x244));
        assert_eq!(r.node, Node::Control);
        assert!(r.validate(map, &f));
        set(&mut f, 0x243, false);
        assert!(!r.validate(map, &f));
    }
}

#[cfg(test)]
mod malformed_tests {
    use super::*;
    #[test]
    fn forced_ownership_cannot_restore_as_house_or_resident_control() {
        let mut flags = StoryFlags::new([0; 128]);
        for bit in [0x20, 0xfb, 0x26, 0x28, 0x27, 0x2e, 0x292, 0x22] {
            set(&mut flags, bit, true);
        }
        for final_return in [false, true] {
            set(&mut flags, 0x243, final_return);
            for map in [0xc, 0x13] {
                assert!(Runtime::decode([0; 4], map, &flags).is_err());
            }
        }
    }
    #[test]
    fn cursor_stage_flags_and_locals_are_not_independent_fields() {
        let mut flags = StoryFlags::new([0; 128]);
        for bit in [0x20, 0xfb, 0x26, 0x28, 0x27] {
            set(&mut flags, bit, true);
        }
        let state = Runtime {
            node: Node::Request(Invocation::CChoice, 3),
            counter: 0,
        };
        assert_eq!(Runtime::decode(state.encode(), 0xc, &flags).unwrap(), state);
        for bytes in [
            [0, 0, 0, 0],
            [1, Invocation::CChoice as u8, 4, 0],
            [1, 255, 0, 0],
            [2, Invocation::CChoice as u8, 0, 0],
            [1, Invocation::CChoice as u8, 3, 1],
        ] {
            assert!(Runtime::decode(bytes, 0xc, &flags).is_err());
        }
        for bit in [0, 1, 31, 0x2e, 0x2f, 0x292, 0x244, 1023] {
            let mut invalid = flags.clone();
            set(&mut invalid, bit, true);
            assert!(
                Runtime::decode(state.encode(), 0xc, &invalid).is_err(),
                "bit {bit}"
            );
        }
    }
}

#[cfg(test)]
mod opening_predicate_tests {
    use super::*;
    #[test]
    fn local1_and_local2_are_required_and_neither_contact_nor_poll_grants22() {
        for locals in [0, 2, 4, 6] {
            let mut flags = StoryFlags::new([0; 128]);
            for bit in [0x20, 0xfb, 0x26, 0x28, 0x27, 0x2e, 0x292] {
                set(&mut flags, bit, true);
            }
            set(&mut flags, 1, locals & 2 != 0);
            set(&mut flags, 2, locals & 4 != 0);
            let mut r = Runtime::new();
            assert_eq!(r.poll_opening(&flags), locals == 6);
            assert!(!has(&flags, 0x22));
            if locals != 0 {
                assert!(r.contact(&mut flags).is_err());
            }
        }
    }
}
