//! Immutable source-authenticated dialogue for the bounded direct Pandora route.
//!
//! No event execution, custom names, Unicode transcription or native window engine.
//! See `docs/pandora-dialogue.md` for source-only pages and presentation limits.
use super::{decode_choice, invalid, DialogueChoice, DialoguePage, TextError};

#[cfg(test)]
mod tests;

/// Ordered `(event COP1B site, text source)` invocations on the direct route.
/// Repeated D720 requests deliberately share resources, not invocation identity.
pub const DIRECT_INVOCATIONS: [(u32, u32); 34] = [
    (0x88_b673, 0x88_b6c7),
    (0x88_b69c, 0x88_b758),
    (0x88_9ae9, 0x88_9e9c),
    (0x88_9afb, 0x88_9ee3),
    (0x88_9b11, 0x88_9efc),
    (0x88_9da9, 0x88_9fa9),
    (0x88_abbc, 0x88_a15f),
    (0x88_9b9f, 0x88_a17b),
    (0x88_9bc2, 0x88_9dc3),
    (0x88_9bef, 0x88_9deb),
    (0x88_a241, 0x88_a295),
    (0x88_a3d9, 0x88_a420),
    (0x88_9c10, 0x88_9e0a),
    (0x88_ad95, 0x88_adcb),
    (0x88_adb7, 0x88_adf2),
    (0x88_ae77, 0x88_afa6),
    (0x88_ae85, 0x88_afe1),
    (0x88_ae93, 0x88_b064),
    (0x88_aea1, 0x88_b0fb),
    (0x89_d3e0, 0x89_d50f),
    (0x89_d3f4, 0x89_d548),
    (0x89_d408, 0x89_d5e5),
    (0x89_d41c, 0x89_d622),
    (0x89_d430, 0x89_d66d),
    (0x89_d444, 0x89_d6a7),
    (0x89_d458, 0x89_d6dd),
    (0x89_d470, 0x89_d720),
    (0x89_d4a4, 0x89_d7c1),
    (0x89_d4ae, 0x89_d720),
    (0x89_d4c9, 0x89_d92d),
    (0x89_d4d3, 0x89_d720),
    (0x89_d4ee, 0x89_d991),
    (0x89_d4f8, 0x89_d720),
    (0x89_d48b, 0x89_d735),
];

/// One source request, with fixed ordered logical pages and optional retained choice.
#[derive(Debug)]
pub struct DialogueRequest {
    source: u32,
    pages: Vec<DialoguePage>,
    choice_catalog: Option<u8>,
}
impl DialogueRequest {
    fn compile(image: &[u8], source: u32, choice_catalog: Option<u8>) -> Result<Self, TextError> {
        let pages = super::decode_profile(image, source, true)?;
        if pages.len() > 15 {
            return Err(invalid(source, "Pandora request exceeds 15 logical pages"));
        }
        if choice_catalog.is_some()
            && (choice_catalog != Some(1)
                || pages.last().map(DialoguePage::acknowledgement)
                    != Some(super::Acknowledgement::None))
        {
            return Err(invalid(
                source,
                "choice requires admitted catalog and retained D4 tail",
            ));
        }
        Ok(Self {
            source,
            pages,
            choice_catalog,
        })
    }

    /// ROM text entry address, not the event instruction issuing the request.
    #[must_use]
    pub const fn source(&self) -> u32 {
        self.source
    }
    /// Ordered immutable content. Respect each page's D3/D5/D4 boundary.
    #[must_use]
    pub fn pages(&self) -> &[DialoguePage] {
        &self.pages
    }
    /// Host-compatible logical identity: `(source << 4) | zero_based_index`.
    /// Out-of-range indices never alias another request.
    #[must_use]
    pub fn page_id(&self, index: usize) -> Option<u32> {
        if index >= self.pages.len() {
            return None;
        }
        Some((self.source << 4) | u32::try_from(index).ok()?)
    }
    /// Catalog entered by the direct-route event after this retained D4 tail.
    /// Selection/branch effects remain entirely caller-owned.
    #[must_use]
    pub const fn choice_catalog(&self) -> Option<u8> {
        self.choice_catalog
    }
}

/// Bounded direct-route and map13 retry-path resources; `HouseDialogue` is unchanged.
#[derive(Debug)]
pub struct PandoraDialogue {
    requests: Vec<DialogueRequest>,
    choice: DialogueChoice,
}
impl PandoraDialogue {
    /// Compile from an authenticated normalized Japanese image, without CPU execution.
    ///
    /// # Errors
    /// Rejects any other image, unsupported controls/layout, or a request with more
    /// than 15 pages (explicit admission guard for the four-bit host page index).
    pub fn from_rom(image: &[u8]) -> Result<Self, TextError> {
        if image.len() != 0x40_0000 || rom::digests(image).sha256 != rom::Revision::Japan.sha256() {
            return Err(invalid(0, "expected authenticated normalized Japanese ROM"));
        }
        Ok(Self {
            requests: {
                let mut requests = Vec::new();
                for source in DIRECT_INVOCATIONS
                    .iter()
                    .map(|(_, source)| *source)
                    .chain([0x88_b722, 0x88_b7e3])
                {
                    if requests
                        .iter()
                        .any(|r: &DialogueRequest| r.source == source)
                    {
                        continue;
                    }
                    let choice = [0x88_b6c7, 0x88_9efc, 0x88_b722]
                        .contains(&source)
                        .then_some(1);
                    requests.push(DialogueRequest::compile(image, source, choice)?);
                }
                requests
            },
            choice: decode_choice(image, 1)?,
        })
    }
    /// Distinct resources in first direct-use order, followed by the retry choice
    /// context and its map13 refusal resource. Use `DIRECT_INVOCATIONS` for direct
    /// execution order including repeats; enumeration is not retry-path playback.
    #[must_use]
    pub fn requests(&self) -> &[DialogueRequest] {
        &self.requests
    }
    /// Lookup by authenticated text-source address. Unknown sources are not decoded.
    #[must_use]
    pub fn request(&self, source: u32) -> Option<&DialogueRequest> {
        self.requests.iter().find(|r| r.source == source)
    }
    /// Compatibility-shaped source lookup for a parent asset compiler.
    #[must_use]
    pub fn pages(&self, source: u32) -> Option<&[DialoguePage]> {
        self.request(source).map(DialogueRequest::pages)
    }
    /// The admitted source catalog (1); initial result 1, cancel 0, confirm 1/2.
    #[must_use]
    pub fn choice(&self, catalog: u8) -> Option<&DialogueChoice> {
        (catalog == 1).then_some(&self.choice)
    }
}

// $8596BE compares assignments in address order. $85BEF7 resets the default
// controller configuration; read its immediate stores, not captured button RAM.
pub(super) fn default_button_source(image: &[u8], mask: u16) -> Result<u32, TextError> {
    let mut assignments = [0_u16; 6];
    for (index, target) in [0x634, 0x636, 0x638, 0x63a, 0x63e, 0x63c]
        .into_iter()
        .enumerate()
    {
        // European `$85:BF8F`, the same six stores.
        let start = crate::layout::at(image, 0x85_bef7)
            .ok_or_else(|| invalid(0x85_bef7, "unrecorded default controller initialization"))?;
        let source = start + u32::try_from(index).unwrap() * 6;
        let code = super::bytes(image, source, 6)?;
        if code[0] != 0xa9 || code[3] != 0x8d || u16::from_le_bytes([code[4], code[5]]) != target {
            return Err(invalid(source, "changed default controller initialization"));
        }
        assignments[usize::from((target - 0x634) / 2)] = u16::from_le_bytes([code[1], code[2]]);
    }
    let index = assignments
        .iter()
        .position(|value| *value == mask)
        .ok_or_else(|| invalid(0x85_96be, "unsupported default controller label"))?;
    // European `$85:975D`, the operand of `LDA $85975D,X` at `$85:974A`.
    let labels = crate::layout::at(image, 0x85_970d)
        .ok_or_else(|| invalid(0x85_970d, "unrecorded controller label table"))?;
    let source = labels + u32::try_from(index).unwrap() * 2;
    let pointer = super::bytes(image, source, 2)?;
    Ok(0x85_0000 | u32::from(u16::from_le_bytes([pointer[0], pointer[1]])))
}
