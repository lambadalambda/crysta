//! Canonical, revision-bound memory symbols for Terranigma.
//!
//! The built-in map is committed as JSON, deserialized into this typed model,
//! and validated before use. Addresses are canonical 24-bit SNES bus
//! addresses; direct-page symbols assume `D = $0000`.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write as _};

/// Schema version understood by this crate.
pub const SCHEMA_VERSION: u32 = 1;

/// SHA-256 of the normalized Japanese ROM revision described by the built-in map.
pub const JAPAN_ROM_SHA256: &str =
    "f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548";

const JAPAN_MAP_JSON: &str = include_str!("../data/japan-v1.json");

/// A canonical 24-bit SNES bus address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Address(u32);

impl Address {
    /// Constructs an address. Validation rejects values beyond the 24-bit bus.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the numeric bus address.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("${:06X}", self.0))
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        let digits = text
            .strip_prefix('$')
            .or_else(|| text.strip_prefix("0x"))
            .unwrap_or(&text);
        u32::from_str_radix(digits, 16)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

/// A physically distinct class of address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddressSpace {
    /// Memory-mapped SNES hardware registers in canonical bank zero.
    Hardware,
    /// Direct-page storage, interpreted with `D = $0000`.
    DirectPage,
    /// Work RAM in canonical banks `$7E-$7F`.
    Wram,
    /// Battery-backed cartridge SRAM represented conservatively in canonical
    /// `HiROM` banks `$20-$3F`, window `$6000-$7FFF`. Validation requires each
    /// region to stay within one bank. This representation does not assert the
    /// cartridge's decoded SRAM extent or which windows are physical mirrors.
    Sram,
}

/// The semantic shape of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    /// An integer or opaque scalar value.
    Scalar,
    /// An address or dispatch pointer.
    Pointer,
    /// A value whose individual bits carry named state.
    Bitfield,
    /// A contiguous byte buffer.
    Buffer,
    /// A memory-mapped hardware port.
    Port,
}

/// Permitted program access to a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    /// Read-only from the CPU's perspective.
    ReadOnly,
    /// Write-only from the CPU's perspective.
    WriteOnly,
    /// Read and write access.
    ReadWrite,
}

/// How long a value remains meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifetime {
    /// Defined by hardware rather than game state.
    Hardware,
    /// Scratch state valid only while handling an interrupt or service call.
    Interrupt,
    /// State refreshed within a frame.
    Frame,
    /// State tied to the current map or room.
    Map,
    /// State retained for the running program session.
    Session,
    /// Persistent player state that can be represented in a save.
    Persistent,
}

/// The review state of an alternate name or relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AliasStatus {
    /// The alternate name is supported by project evidence.
    Confirmed,
    /// Sources assign incompatible meanings; both claims are retained.
    Conflicting,
    /// A public-map name imported as an unverified lead.
    Imported,
    /// A former name retained to document that it was disproved.
    Rejected,
}

/// An alternate name and, when applicable, its explicit symbol relationship.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Alias {
    /// Human- or tool-facing alternate name.
    pub name: String,
    /// Review state of this alias.
    pub status: AliasStatus,
    /// ID of the separately retained symbol involved in this relationship.
    #[serde(default)]
    pub related_symbol_id: Option<String>,
    /// Explanation for the alias decision.
    pub note: String,
}

/// How strongly the canonical name and extent are supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// Imported from a public map and not independently verified.
    ImportedClaim,
    /// Corroborated by static analysis of the supported ROM.
    StaticCorroborated,
    /// Corroborated by project-owned execution traces or tests.
    TraceCorroborated,
}

/// The method by which an evidence statement was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// A claim imported from a public source and treated as a lead.
    ImportedClaim,
    /// Static analysis of code or data in the supported ROM.
    StaticAnalysis,
    /// A runtime observation recorded by this project.
    ProjectTrace,
    /// A hardware definition from project assembly or documentation.
    ProjectDocumentation,
    /// A project test that establishes the stated behavior.
    ProjectTest,
}

/// One evidence citation attached to a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// Stable ID of a record in [`MemoryMap::sources`].
    pub source_id: String,
    /// Stable location within that source, such as a line or heading.
    pub locator: String,
    /// How this evidence was obtained.
    pub provenance: Provenance,
    /// Short qualification of what the source establishes.
    pub note: String,
}

/// A stable public or project-local provenance record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// Stable source identifier used by evidence records.
    pub id: String,
    /// Inspectable source title.
    pub title: String,
    /// Public URL, if this is an external source.
    #[serde(default)]
    pub url: Option<String>,
    /// ISO 8601 retrieval date for an external source.
    #[serde(default)]
    pub retrieved_on: Option<String>,
    /// Repository-relative path, if this is a project source.
    #[serde(default)]
    pub project_path: Option<String>,
}

/// A source's competing boundary or range for a canonical symbol.
///
/// Claims remain separate from [`Symbol`] so querying an address never treats
/// an unselected boundary as canonical.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConflictingClaim {
    /// Canonical symbol whose selected range is disputed.
    pub symbol_id: String,
    /// Address space asserted by the conflicting source.
    pub claimed_space: AddressSpace,
    /// Start address asserted by the conflicting source.
    pub claimed_address: Address,
    /// Width asserted by the conflicting source.
    pub claimed_width: u32,
    /// Stable source ID making the claim.
    pub source_id: String,
    /// Neutral explanation of the disagreement and any inferred boundary.
    pub note: String,
}

/// One named region in the canonical memory map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Symbol {
    /// Stable language-neutral identifier.
    pub id: String,
    /// ca65-compatible canonical name.
    pub asm_name: String,
    /// Physical class of the address.
    pub space: AddressSpace,
    /// Canonical SNES bus address at the beginning of the region.
    pub address: Address,
    /// Region size in bytes.
    pub width: u32,
    /// Semantic kind of value.
    pub kind: SymbolKind,
    /// Program-visible access behavior.
    pub access: Access,
    /// Duration for which the value remains meaningful.
    pub lifetime: Lifetime,
    /// Explicit alternate names and relationships.
    #[serde(default)]
    pub aliases: Vec<Alias>,
    /// Strength of the canonical interpretation.
    pub confidence: Confidence,
    /// Evidence and provenance records supporting the interpretation.
    pub evidence: Vec<Evidence>,
    /// Preferred ca65 operand, which may use a proven short mirror.
    #[serde(default)]
    pub assembly_operand: Option<Address>,
}

/// A complete revision-bound memory map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryMap {
    /// Version of the JSON schema.
    pub schema_version: u32,
    /// Stable supported-revision name.
    pub revision: String,
    /// SHA-256 of the normalized ROM to which symbols apply.
    pub rom_sha256: String,
    /// Sources referenced by symbol evidence.
    pub sources: Vec<Source>,
    /// Canonical symbols and explicitly retained competing names.
    pub symbols: Vec<Symbol>,
    /// Competing source claims about canonical symbol ranges or boundaries.
    pub conflicting_claims: Vec<ConflictingClaim>,
}

impl MemoryMap {
    /// Loads and validates the committed Japanese schema-v1 map.
    ///
    /// # Errors
    ///
    /// Returns a typed parse or validation error if the committed artifact and
    /// this implementation disagree.
    pub fn built_in_japan() -> Result<Self, LoadError> {
        Self::from_json(JAPAN_MAP_JSON)
    }

    /// Deserializes and validates a canonical map.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError::Json`] for malformed JSON or
    /// [`LoadError::Validation`] when schema invariants do not hold.
    pub fn from_json(json: &str) -> Result<Self, LoadError> {
        let map: Self = serde_json::from_str(json).map_err(LoadError::Json)?;
        map.validate().map_err(LoadError::Validation)?;
        Ok(map)
    }

    /// Validates revision identity, references, ranges, and overlaps.
    ///
    /// Exact physical overlaps are accepted only when both symbols name one
    /// another with `conflicting` aliases. Direct page with `D = $0000` and
    /// canonical low WRAM are checked as the same physical bytes. Partial
    /// overlaps are always an error.
    ///
    /// # Errors
    ///
    /// Returns the first deterministic validation failure.
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_identity(self)?;
        let source_kinds = validate_sources(&self.sources)?;
        let symbol_ids = validate_symbols(&self.symbols, &source_kinds)?;
        validate_aliases(&self.symbols, &symbol_ids)?;
        validate_overlaps(&self.symbols)?;
        validate_conflicting_claims(
            &self.conflicting_claims,
            &self.symbols,
            &source_kinds,
            &symbol_ids,
        )
    }

    /// Finds a symbol by stable ID.
    #[must_use]
    pub fn lookup_id(&self, id: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|symbol| symbol.id == id)
    }

    /// Returns every symbol whose region contains `address`.
    ///
    /// Multiple values are intentional for explicitly documented aliases and
    /// competing claims. Results retain canonical JSON order.
    #[must_use]
    pub fn lookup_address(&self, address: Address) -> Vec<&Symbol> {
        self.symbols
            .iter()
            .filter(|symbol| {
                let end = symbol.address.0 + symbol.width;
                symbol.address.0 <= address.0 && address.0 < end
            })
            .collect()
    }

    /// Returns boundary/range claims attached to `symbol_id` in JSON order.
    #[must_use]
    pub fn lookup_conflicting_claims(&self, symbol_id: &str) -> Vec<&ConflictingClaim> {
        self.conflicting_claims
            .iter()
            .filter(|claim| claim.symbol_id == symbol_id)
            .collect()
    }

    /// Reads a symbol's byte region from a caller-supplied 128 KiB WRAM view.
    ///
    /// Direct-page addresses are interpreted with `D = $0000`; canonical WRAM
    /// addresses map `$7E0000-$7FFFFF` to offsets `0-$1FFFF`.
    ///
    /// # Errors
    ///
    /// Returns [`ReadError::UnsupportedAddressSpace`] for hardware or SRAM,
    /// [`ReadError::InvalidSymbolRegion`] for an invalid caller-created
    /// symbol, and [`ReadError::WramTooShort`] if the supplied slice does not
    /// cover the complete symbol.
    pub fn read_symbol<'a>(&self, symbol: &Symbol, wram: &'a [u8]) -> Result<&'a [u8], ReadError> {
        if !region_in_space(symbol.space, symbol.address.0, symbol.width) {
            return Err(ReadError::InvalidSymbolRegion);
        }
        let offset = match symbol.space {
            AddressSpace::DirectPage => symbol.address.0 as usize,
            AddressSpace::Wram => symbol
                .address
                .0
                .checked_sub(0x7E_0000)
                .ok_or(ReadError::InvalidSymbolRegion)? as usize,
            unsupported => return Err(ReadError::UnsupportedAddressSpace(unsupported)),
        };
        let needed = offset
            .checked_add(symbol.width as usize)
            .ok_or(ReadError::InvalidSymbolRegion)?;
        wram.get(offset..needed).ok_or(ReadError::WramTooShort {
            needed,
            actual: wram.len(),
        })
    }

    /// Generates a deterministic ca65 assignment include.
    ///
    /// Only symbols with an assembly operand are emitted. Names sort
    /// lexicographically; operands at most `$FFFF` use four digits and larger
    /// operands use six.
    #[must_use]
    pub fn generate_ca65_include(&self) -> String {
        let mut symbols: Vec<_> = self
            .symbols
            .iter()
            .filter_map(|symbol| symbol.assembly_operand.map(|operand| (symbol, operand)))
            .collect();
        symbols.sort_by(|(left, _), (right, _)| left.asm_name.cmp(&right.asm_name));

        let mut output = format!(
            "; Generated by memory-map schema v{} for {}. Do not edit.\n\
             ; ROM SHA-256: {}\n\n",
            self.schema_version, self.revision, self.rom_sha256
        );
        for (symbol, operand) in symbols {
            let digits = if operand.0 <= 0xFFFF { 4 } else { 6 };
            writeln!(output, "{} = ${:0digits$X}", symbol.asm_name, operand.0)
                .expect("writing to String cannot fail");
        }
        output
    }
}

fn validate_identity(map: &MemoryMap) -> Result<(), ValidationError> {
    if map.schema_version != SCHEMA_VERSION {
        return Err(ValidationError::UnsupportedSchema {
            found: map.schema_version,
        });
    }
    if map.revision != "japan" {
        return Err(ValidationError::UnsupportedRevision {
            found: map.revision.clone(),
        });
    }
    if map.rom_sha256 != JAPAN_ROM_SHA256 {
        return Err(ValidationError::UnexpectedRomHash {
            found: map.rom_sha256.clone(),
        });
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SourceKind {
    External,
    Project,
}

fn retrieval_date_is_valid(date: &str) -> bool {
    let bytes = date.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
    {
        return false;
    }
    let year: u32 = date[0..4].parse().expect("checked decimal year");
    let month: u32 = date[5..7].parse().expect("checked decimal month");
    let day: u32 = date[8..10].parse().expect("checked decimal day");
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    year != 0 && (1..=days).contains(&day)
}

fn external_url_is_valid(url: &str) -> bool {
    let remainder = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    remainder.is_some_and(|value| !value.is_empty() && !value.chars().any(char::is_whitespace))
}

fn project_path_is_valid(path: &str) -> bool {
    let mut components = std::path::Path::new(path).components();
    components.next().is_some_and(|component| {
        matches!(component, std::path::Component::Normal(_))
            && components.all(|rest| matches!(rest, std::path::Component::Normal(_)))
    })
}

fn validate_sources(sources: &[Source]) -> Result<HashMap<&str, SourceKind>, ValidationError> {
    let mut kinds = HashMap::new();
    for source in sources {
        let kind = match (&source.url, &source.retrieved_on, &source.project_path) {
            (Some(url), Some(date), None)
                if external_url_is_valid(url) && retrieval_date_is_valid(date) =>
            {
                SourceKind::External
            }
            (None, None, Some(path)) if project_path_is_valid(path) => SourceKind::Project,
            _ => {
                return Err(ValidationError::InvalidSource {
                    id: source.id.clone(),
                });
            }
        };
        if source.id.trim().is_empty() || source.title.trim().is_empty() {
            return Err(ValidationError::InvalidSource {
                id: source.id.clone(),
            });
        }
        if kinds.insert(source.id.as_str(), kind).is_some() {
            return Err(ValidationError::DuplicateSourceId {
                id: source.id.clone(),
            });
        }
    }
    Ok(kinds)
}

fn validate_symbols<'a>(
    symbols: &'a [Symbol],
    source_kinds: &HashMap<&str, SourceKind>,
) -> Result<HashSet<&'a str>, ValidationError> {
    let mut ids = HashSet::new();
    let mut asm_names = HashSet::new();
    for symbol in symbols {
        if symbol.id.trim().is_empty() {
            return Err(ValidationError::InvalidSymbolId);
        }
        if !ids.insert(symbol.id.as_str()) {
            return Err(ValidationError::DuplicateSymbolId {
                id: symbol.id.clone(),
            });
        }
        if !asm_names.insert(symbol.asm_name.as_str()) {
            return Err(ValidationError::DuplicateAsmName {
                asm_name: symbol.asm_name.clone(),
            });
        }
        validate_symbol(symbol, source_kinds)?;
    }
    Ok(ids)
}

fn validate_symbol(
    symbol: &Symbol,
    source_kinds: &HashMap<&str, SourceKind>,
) -> Result<(), ValidationError> {
    if !asm_name_is_valid(&symbol.asm_name) {
        return Err(ValidationError::InvalidAsmName {
            asm_name: symbol.asm_name.clone(),
        });
    }
    if symbol.evidence.is_empty() {
        return Err(ValidationError::MissingEvidence {
            id: symbol.id.clone(),
        });
    }
    if symbol.width == 0 {
        return Err(ValidationError::ZeroWidth {
            id: symbol.id.clone(),
        });
    }
    if !region_in_space(symbol.space, symbol.address.0, symbol.width) {
        return Err(ValidationError::AddressOutOfRange {
            id: symbol.id.clone(),
            space: symbol.space,
            address: symbol.address,
            width: symbol.width,
        });
    }
    if let Some(operand) = symbol.assembly_operand {
        if operand.0 > 0xFF_FFFF {
            return Err(ValidationError::AssemblyOperandOutOfRange {
                id: symbol.id.clone(),
            });
        }
        if !assembly_operand_matches(symbol, operand) {
            return Err(ValidationError::AssemblyOperandMismatch {
                id: symbol.id.clone(),
            });
        }
    }
    for evidence in &symbol.evidence {
        if evidence.source_id.trim().is_empty()
            || evidence.locator.trim().is_empty()
            || evidence.note.trim().is_empty()
        {
            return Err(ValidationError::InvalidEvidence {
                symbol_id: symbol.id.clone(),
            });
        }
        let Some(source_kind) = source_kinds.get(evidence.source_id.as_str()) else {
            return Err(ValidationError::UnknownEvidenceSource {
                symbol_id: symbol.id.clone(),
                source_id: evidence.source_id.clone(),
            });
        };
        let expected_kind = if evidence.provenance == Provenance::ImportedClaim {
            SourceKind::External
        } else {
            SourceKind::Project
        };
        if *source_kind != expected_kind {
            return Err(ValidationError::ProvenanceSourceMismatch {
                symbol_id: symbol.id.clone(),
                source_id: evidence.source_id.clone(),
            });
        }
    }
    let confidence_supported = match symbol.confidence {
        Confidence::ImportedClaim => symbol
            .evidence
            .iter()
            .any(|evidence| evidence.provenance == Provenance::ImportedClaim),
        Confidence::StaticCorroborated => symbol
            .evidence
            .iter()
            .any(|evidence| evidence.provenance == Provenance::StaticAnalysis),
        Confidence::TraceCorroborated => symbol.evidence.iter().any(|evidence| {
            matches!(
                evidence.provenance,
                Provenance::ProjectTrace | Provenance::ProjectTest
            )
        }),
    };
    if !confidence_supported {
        return Err(ValidationError::ConfidenceEvidenceMismatch {
            id: symbol.id.clone(),
        });
    }
    Ok(())
}

fn validate_aliases(symbols: &[Symbol], ids: &HashSet<&str>) -> Result<(), ValidationError> {
    for symbol in symbols {
        for alias in &symbol.aliases {
            if alias.name.trim().is_empty() || alias.note.trim().is_empty() {
                return Err(ValidationError::InvalidAlias {
                    symbol_id: symbol.id.clone(),
                });
            }
            if let Some(related) = &alias.related_symbol_id {
                if related == &symbol.id || !ids.contains(related.as_str()) {
                    return Err(ValidationError::InvalidAliasTarget {
                        symbol_id: symbol.id.clone(),
                        related_symbol_id: related.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}

fn validate_overlaps(symbols: &[Symbol]) -> Result<(), ValidationError> {
    for (index, left) in symbols.iter().enumerate() {
        for right in &symbols[index + 1..] {
            if regions_overlap(left, right) {
                let exact =
                    physical_start(left) == physical_start(right) && left.width == right.width;
                if !exact || !reciprocal_conflicting_aliases(left, right) {
                    return Err(ValidationError::UndocumentedOverlap {
                        first_id: left.id.clone(),
                        second_id: right.id.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}

fn validate_conflicting_claims(
    claims: &[ConflictingClaim],
    symbols: &[Symbol],
    source_kinds: &HashMap<&str, SourceKind>,
    symbol_ids: &HashSet<&str>,
) -> Result<(), ValidationError> {
    for claim in claims {
        if !symbol_ids.contains(claim.symbol_id.as_str()) {
            return Err(ValidationError::UnknownClaimSymbol {
                symbol_id: claim.symbol_id.clone(),
            });
        }
        if !source_kinds.contains_key(claim.source_id.as_str()) {
            return Err(ValidationError::UnknownClaimSource {
                source_id: claim.source_id.clone(),
            });
        }
        if claim.note.trim().is_empty()
            || !region_in_space(
                claim.claimed_space,
                claim.claimed_address.0,
                claim.claimed_width,
            )
        {
            return Err(ValidationError::InvalidConflictingClaim {
                symbol_id: claim.symbol_id.clone(),
            });
        }
        let symbol = symbols
            .iter()
            .find(|symbol| symbol.id == claim.symbol_id)
            .expect("validated symbol ID");
        if physical_address(claim.claimed_space, claim.claimed_address.0) == physical_start(symbol)
            && claim.claimed_width == symbol.width
        {
            return Err(ValidationError::NonConflictingClaim {
                symbol_id: claim.symbol_id.clone(),
            });
        }
    }
    Ok(())
}

fn asm_name_is_valid(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn assembly_operand_matches(symbol: &Symbol, operand: Address) -> bool {
    let low_wram_mirror = symbol
        .address
        .0
        .checked_add(symbol.width - 1)
        .is_some_and(|end| {
            (0x7E_0000..=0x7E_1FFF).contains(&symbol.address.0)
                && end <= 0x7E_1FFF
                && operand.0 == symbol.address.0 & 0xFFFF
        });
    operand == symbol.address || symbol.space == AddressSpace::Wram && low_wram_mirror
}

fn region_in_space(space: AddressSpace, start: u32, width: u32) -> bool {
    let Some(last_offset) = width.checked_sub(1) else {
        return false;
    };
    let Some(end) = start.checked_add(last_offset) else {
        return false;
    };
    match space {
        AddressSpace::Hardware => {
            start >> 16 == 0
                && end >> 16 == 0
                && ((0x2100..=0x21FF).contains(&start) && end <= 0x21FF
                    || (0x4200..=0x43FF).contains(&start) && end <= 0x43FF)
        }
        AddressSpace::DirectPage => start <= 0xFF && end <= 0xFF,
        AddressSpace::Wram => (0x7E_0000..=0x7F_FFFF).contains(&start) && end <= 0x7F_FFFF,
        AddressSpace::Sram => {
            let bank = start >> 16;
            (0x20..=0x3F).contains(&bank)
                && end >> 16 == bank
                && start & 0xFFFF >= 0x6000
                && end & 0xFFFF <= 0x7FFF
        }
    }
}

fn physical_address(space: AddressSpace, address: u32) -> (AddressSpace, u32) {
    match space {
        AddressSpace::DirectPage => (AddressSpace::Wram, address),
        AddressSpace::Wram => (AddressSpace::Wram, address - 0x7E_0000),
        space => (space, address),
    }
}

fn physical_start(symbol: &Symbol) -> (AddressSpace, u32) {
    physical_address(symbol.space, symbol.address.0)
}

fn regions_overlap(left: &Symbol, right: &Symbol) -> bool {
    let (left_space, left_start) = physical_start(left);
    let (right_space, right_start) = physical_start(right);
    left_space == right_space
        && left_start < right_start + right.width
        && right_start < left_start + left.width
}

fn has_conflicting_alias(symbol: &Symbol, related_id: &str) -> bool {
    symbol.aliases.iter().any(|alias| {
        alias.status == AliasStatus::Conflicting
            && alias.related_symbol_id.as_deref() == Some(related_id)
    })
}

fn reciprocal_conflicting_aliases(left: &Symbol, right: &Symbol) -> bool {
    has_conflicting_alias(left, &right.id) && has_conflicting_alias(right, &left.id)
}

/// Failure while parsing or validating a serialized map.
#[derive(Debug)]
pub enum LoadError {
    /// JSON deserialization failed.
    Json(serde_json::Error),
    /// The decoded map violated a schema invariant.
    Validation(ValidationError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "invalid memory-map JSON: {error}"),
            Self::Validation(error) => write!(formatter, "invalid memory map: {error}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// A canonical-map validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationError {
    /// The map uses an unsupported schema version.
    UnsupportedSchema {
        /// Version found in the map.
        found: u32,
    },
    /// The map identifies an unsupported ROM revision.
    UnsupportedRevision {
        /// Revision found in the map.
        found: String,
    },
    /// The map is bound to the wrong normalized ROM.
    UnexpectedRomHash {
        /// Digest found in the map.
        found: String,
    },
    /// Two source records share an ID.
    DuplicateSourceId {
        /// Duplicated ID.
        id: String,
    },
    /// A source lacks exactly one complete URL/date or project-path locator.
    InvalidSource {
        /// Invalid source ID.
        id: String,
    },
    /// A symbol has an empty stable ID.
    InvalidSymbolId,
    /// Two symbols share an ID.
    DuplicateSymbolId {
        /// Duplicated ID.
        id: String,
    },
    /// Two symbols share a ca65 name.
    DuplicateAsmName {
        /// Duplicated assembler name.
        asm_name: String,
    },
    /// A symbol name cannot be emitted safely as a ca65 identifier.
    InvalidAsmName {
        /// Invalid assembler name.
        asm_name: String,
    },
    /// A symbol has no provenance records.
    MissingEvidence {
        /// Symbol without evidence.
        id: String,
    },
    /// A symbol has an empty region.
    ZeroWidth {
        /// Invalid symbol ID.
        id: String,
    },
    /// A symbol region is outside its declared address space.
    AddressOutOfRange {
        /// Invalid symbol ID.
        id: String,
        /// Declared address space.
        space: AddressSpace,
        /// Region start.
        address: Address,
        /// Region width.
        width: u32,
    },
    /// A preferred assembly operand exceeds 24 bits.
    AssemblyOperandOutOfRange {
        /// Invalid symbol ID.
        id: String,
    },
    /// A preferred operand is neither canonical nor a supported low-WRAM mirror.
    AssemblyOperandMismatch {
        /// Invalid symbol ID.
        id: String,
    },
    /// A corroborated confidence level lacks matching project evidence.
    ConfidenceEvidenceMismatch {
        /// Invalid symbol ID.
        id: String,
    },
    /// Evidence has an empty source ID, locator, or note.
    InvalidEvidence {
        /// Symbol containing the invalid evidence.
        symbol_id: String,
    },
    /// Evidence provenance does not match an external or project source.
    ProvenanceSourceMismatch {
        /// Symbol containing the invalid evidence.
        symbol_id: String,
        /// Source whose kind does not match the provenance.
        source_id: String,
    },
    /// Evidence cites a source absent from the map.
    UnknownEvidenceSource {
        /// Symbol containing the citation.
        symbol_id: String,
        /// Missing source ID.
        source_id: String,
    },
    /// An alias has an empty name or note.
    InvalidAlias {
        /// Symbol containing the invalid alias.
        symbol_id: String,
    },
    /// An explicit alias relationship targets itself or a missing symbol.
    InvalidAliasTarget {
        /// Symbol containing the relationship.
        symbol_id: String,
        /// Invalid related symbol ID.
        related_symbol_id: String,
    },
    /// A conflicting claim names a symbol absent from the map.
    UnknownClaimSymbol {
        /// Missing canonical symbol ID.
        symbol_id: String,
    },
    /// A conflicting claim names a source absent from the map.
    UnknownClaimSource {
        /// Missing source ID.
        source_id: String,
    },
    /// A conflicting claim has empty text or an invalid region.
    InvalidConflictingClaim {
        /// Canonical symbol ID associated with the claim.
        symbol_id: String,
    },
    /// A purported conflicting claim exactly repeats the canonical region.
    NonConflictingClaim {
        /// Canonical symbol ID associated with the claim.
        symbol_id: String,
    },
    /// Two physical regions overlap without reciprocal conflicting aliases.
    UndocumentedOverlap {
        /// First overlapping symbol ID.
        first_id: String,
        /// Second overlapping symbol ID.
        second_id: String,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ValidationError {}

/// Failure to read a symbol from a WRAM byte view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReadError {
    /// The symbol does not reside in WRAM or direct page.
    UnsupportedAddressSpace(AddressSpace),
    /// The caller supplied a symbol outside its declared memory space.
    InvalidSymbolRegion,
    /// The supplied WRAM view does not cover the complete symbol.
    WramTooShort {
        /// Minimum required slice length.
        needed: usize,
        /// Actual supplied slice length.
        actual: usize,
    },
}

impl fmt::Display for ReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ReadError {}
