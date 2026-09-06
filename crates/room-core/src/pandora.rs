//! Fixed Pandora request identities and validated immutable text data.
use crate::slice::SliceError;
use alloc::vec::Vec;

/// Event invocation, not a text resource or an executable ROM address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Invocation {
    /// Source-qualified `ResidentFirst` invocation.
    ResidentFirst,
    /// Source-qualified `ResidentGrant` invocation.
    ResidentGrant,
    /// Source-qualified `CEntry` invocation.
    CEntry,
    /// Source-qualified `CApproach` invocation.
    CApproach,
    /// Source-qualified `CChoice` invocation.
    CChoice,
    /// Source-qualified `CDirect` invocation.
    CDirect,
    /// Source-qualified `FirstHit` invocation.
    FirstHit,
    /// Source-qualified `SecondHit` invocation.
    SecondHit,
    /// Source-qualified `ReactionSpeaker` invocation.
    ReactionSpeaker,
    /// Source-qualified `ReactionRequest` invocation.
    ReactionRequest,
    /// Source-qualified `ReactionRight` invocation.
    ReactionRight,
    /// Source-qualified `ReactionLeft` invocation.
    ReactionLeft,
    /// Source-qualified `ReactionFinal` invocation.
    ReactionFinal,
    /// Source-qualified `BoxEntry` invocation.
    BoxEntry,
    /// Source-qualified `BoxWarning` invocation.
    BoxWarning,
    /// Source-qualified `OpeningFirst` invocation.
    OpeningFirst,
    /// Source-qualified `OpeningSecond` invocation.
    OpeningSecond,
    /// Source-qualified `OpeningThird` invocation.
    OpeningThird,
    /// Source-qualified `OpeningFourth` invocation.
    OpeningFourth,
    /// Source-qualified `TourIntro` invocation.
    TourIntro,
    /// Source-qualified `TourOne` invocation.
    TourOne,
    /// Source-qualified `TourTwo` invocation.
    TourTwo,
    /// Source-qualified `TourThree` invocation.
    TourThree,
    /// Source-qualified `TourFour` invocation.
    TourFour,
    /// Source-qualified `TourFive` invocation.
    TourFive,
    /// Source-qualified `TourSix` invocation.
    TourSix,
    /// Source-qualified `TourLeave41` invocation.
    TourLeave41,
    /// Source-qualified `Tour44` invocation.
    Tour44,
    /// Source-qualified `TourLeave44` invocation.
    TourLeave44,
    /// Source-qualified `Tour42` invocation.
    Tour42,
    /// Source-qualified `TourLeave42` invocation.
    TourLeave42,
    /// Source-qualified `Tour43` invocation.
    Tour43,
    /// Source-qualified `TourLeave43` invocation.
    TourLeave43,
    /// Source-qualified `TourFinal` invocation.
    TourFinal,
    /// Map13 retained retry context.
    ResidentRetry,
    /// Map13 cancel/result2 response.
    ResidentRefusal,
}
impl Invocation {
    /// Direct-route invocation order; repeated resources retain separate IDs.
    pub const ALL: [Self; 34] = [
        Self::ResidentFirst,
        Self::ResidentGrant,
        Self::CEntry,
        Self::CApproach,
        Self::CChoice,
        Self::CDirect,
        Self::FirstHit,
        Self::SecondHit,
        Self::ReactionSpeaker,
        Self::ReactionRequest,
        Self::ReactionRight,
        Self::ReactionLeft,
        Self::ReactionFinal,
        Self::BoxEntry,
        Self::BoxWarning,
        Self::OpeningFirst,
        Self::OpeningSecond,
        Self::OpeningThird,
        Self::OpeningFourth,
        Self::TourIntro,
        Self::TourOne,
        Self::TourTwo,
        Self::TourThree,
        Self::TourFour,
        Self::TourFive,
        Self::TourSix,
        Self::TourLeave41,
        Self::Tour44,
        Self::TourLeave44,
        Self::Tour42,
        Self::TourLeave42,
        Self::Tour43,
        Self::TourLeave43,
        Self::TourFinal,
    ];
    /// Immutable text resource identity, never dereferenced by the runtime.
    #[must_use]
    #[allow(clippy::match_same_arms)] // Keep authenticated invocation order visible.
    pub const fn source(self) -> u32 {
        match self {
            Self::ResidentFirst => 0x88_b6c7,
            Self::ResidentGrant => 0x88_b758,
            Self::CEntry => 0x88_9e9c,
            Self::CApproach => 0x88_9ee3,
            Self::CChoice => 0x88_9efc,
            Self::CDirect => 0x88_9fa9,
            Self::FirstHit => 0x88_a15f,
            Self::SecondHit => 0x88_a17b,
            Self::ReactionSpeaker => 0x88_9dc3,
            Self::ReactionRequest => 0x88_9deb,
            Self::ReactionRight => 0x88_a295,
            Self::ReactionLeft => 0x88_a420,
            Self::ReactionFinal => 0x88_9e0a,
            Self::BoxEntry => 0x88_adcb,
            Self::BoxWarning => 0x88_adf2,
            Self::OpeningFirst => 0x88_afa6,
            Self::OpeningSecond => 0x88_afe1,
            Self::OpeningThird => 0x88_b064,
            Self::OpeningFourth => 0x88_b0fb,
            Self::TourIntro => 0x89_d50f,
            Self::TourOne => 0x89_d548,
            Self::TourTwo => 0x89_d5e5,
            Self::TourThree => 0x89_d622,
            Self::TourFour => 0x89_d66d,
            Self::TourFive => 0x89_d6a7,
            Self::TourSix => 0x89_d6dd,
            Self::TourLeave41 => 0x89_d720,
            Self::Tour44 => 0x89_d7c1,
            Self::TourLeave44 => 0x89_d720,
            Self::Tour42 => 0x89_d92d,
            Self::TourLeave42 => 0x89_d720,
            Self::Tour43 => 0x89_d991,
            Self::TourLeave43 => 0x89_d720,
            Self::TourFinal => 0x89_d735,
            Self::ResidentRetry => 0x88_b722,
            Self::ResidentRefusal => 0x88_b7e3,
        }
    }
    /// Exact source page count including the non-acknowledged choice tail.
    #[must_use]
    #[allow(clippy::match_same_arms)] // Keep authenticated resource order visible.
    pub const fn request_shape(source: u32) -> Option<(u8, bool)> {
        match source {
            0x88_b6c7 => Some((2, true)),
            0x88_b758 => Some((4, false)),
            0x88_9e9c => Some((2, false)),
            0x88_9ee3 => Some((1, false)),
            0x88_9efc => Some((4, true)),
            0x88_9fa9 => Some((2, false)),
            0x88_a15f => Some((1, false)),
            0x88_a17b => Some((1, false)),
            0x88_9dc3 => Some((1, false)),
            0x88_9deb => Some((1, false)),
            0x88_a295 => Some((1, false)),
            0x88_a420 => Some((1, false)),
            0x88_9e0a => Some((1, false)),
            0x88_adcb => Some((1, false)),
            0x88_adf2 => Some((2, false)),
            0x88_afa6 => Some((2, false)),
            0x88_afe1 => Some((4, false)),
            0x88_b064 => Some((4, false)),
            0x88_b0fb => Some((4, false)),
            0x89_d50f => Some((1, false)),
            0x89_d548 => Some((4, false)),
            0x89_d5e5 => Some((2, false)),
            0x89_d622 => Some((2, false)),
            0x89_d66d => Some((2, false)),
            0x89_d6a7 => Some((2, false)),
            0x89_d6dd => Some((2, false)),
            0x89_d720 => Some((1, false)),
            0x89_d7c1 => Some((8, false)),
            0x89_d92d => Some((3, false)),
            0x89_d991 => Some((3, false)),
            0x89_d735 => Some((4, false)),
            0x88_b722 => Some((1, true)),
            0x88_b7e3 => Some((2, false)),
            _ => None,
        }
    }
}

/// One decoded resource. Values are opaque raster keys, not text or ROM pointers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestPages {
    /// Source resource identity.
    pub source: u32,
    /// Exact ordered logical pages; the last may be retained choice context.
    pub pages: Vec<u32>,
    /// True only for the three catalog1 contexts.
    pub choice: bool,
}
/// Immutable fixed text catalog. Does not accept instructions or branch programs.
#[derive(Debug)]
pub struct PandoraText {
    requests: Vec<RequestPages>,
}
impl PandoraText {
    /// Validate all 33 resources in first-use order, then retry and refusal.
    /// # Errors
    /// Rejects unknown/reordered/duplicate resources, wrong counts or aliased keys.
    pub fn new(requests: Vec<RequestPages>) -> Result<Self, SliceError> {
        let mut sources = Vec::new();
        for source in Invocation::ALL
            .iter()
            .map(|i| i.source())
            .chain([0x88_b722, 0x88_b7e3])
        {
            if !sources.contains(&source) {
                sources.push(source);
            }
        }
        if requests.len() != sources.len() {
            return Err(SliceError::Data);
        }
        let mut keys = Vec::new();
        for (request, source) in requests.iter().zip(sources) {
            let (count, choice) = Invocation::request_shape(source).ok_or(SliceError::Data)?;
            if request.source != source
                || request.pages.len() != usize::from(count)
                || request.choice != choice
            {
                return Err(SliceError::Data);
            }
            keys.extend_from_slice(&request.pages);
        }
        keys.sort_unstable();
        if keys.windows(2).any(|w| w[0] == w[1]) {
            return Err(SliceError::Data);
        }
        Ok(Self { requests })
    }
    /// Ordered raster keys for a validated invocation.
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // Construction proves every enum resource exists.
    pub fn pages(&self, invocation: Invocation) -> &[u32] {
        &self
            .requests
            .iter()
            .find(|r| r.source == invocation.source())
            .expect("validated catalog")
            .pages
    }
}
