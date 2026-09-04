//! Revision-bound ROM code/data classifications and indirect dispatch metadata.

pub use memory_map::{Confidence, Evidence, Provenance, Source};
use rom::{CanonicalRomAddress, NormalizedOffset, Revision, Rom, RuntimeRomAddress};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write as _};

/// Schema version understood by the ROM-map loader.
pub const ROM_MAP_SCHEMA_VERSION: u32 = 1;

const JAPAN_MAP_JSON: &str = include_str!("../data/rom-map/japan-v1.json");

/// Semantic kind of a data region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataKind {
    /// Bytes whose structure is not yet classified more narrowly.
    Raw,
    /// Native or emulation interrupt vectors.
    VectorTable,
    /// A packed table of callable function pointers.
    FunctionPointerTable,
    /// Records containing callback pointers and associated fields.
    CallbackRecords,
    /// A table whose entries begin or identify scripts.
    ScriptEntryTable,
}

/// Canonical classification of a ROM region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "type",
    content = "kind",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RegionClass {
    /// Executable 65C816 code.
    Code,
    /// Non-executable data with a typed role.
    Data(DataKind),
}

/// A canonical, nonoverlapping ROM range.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RomRegion {
    /// Stable language-neutral ID.
    pub id: String,
    /// Start in the normalized headerless image.
    #[serde(deserialize_with = "deserialize_normalized")]
    pub normalized: NormalizedOffset,
    /// Unique canonical `HiROM` address for the same byte.
    #[serde(deserialize_with = "deserialize_canonical")]
    pub canonical: CanonicalRomAddress,
    /// Number of bytes in the range.
    pub length: u32,
    /// Code/data classification.
    pub class: RegionClass,
    /// Strength of the classification.
    pub confidence: Confidence,
    /// Inspectable evidence supporting the classification.
    pub evidence: Vec<Evidence>,
}

/// Semantic role of a declared decode entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    /// An ordinary callable function.
    Function,
    /// An interrupt or software-interrupt handler.
    Interrupt,
    /// A script interpreter entry represented in data.
    Script,
    /// An internal code label useful as a control-flow seed.
    Internal,
}

/// Optional 65C816 state known at an entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecodeState {
    /// Emulation flag (`E`).
    #[serde(default)]
    pub e: Option<bool>,
    /// Accumulator width flag (`M`); true means 8-bit.
    #[serde(default)]
    pub m: Option<bool>,
    /// Index width flag (`X`); true means 8-bit.
    #[serde(default)]
    pub x: Option<bool>,
    /// Data-bank register.
    #[serde(default, deserialize_with = "deserialize_optional_u8")]
    pub dbr: Option<u8>,
    /// Direct-page register.
    #[serde(default, deserialize_with = "deserialize_optional_u16")]
    pub d: Option<u16>,
}

impl DecodeState {
    /// Returns true only when every state component is known.
    #[must_use]
    pub const fn is_fully_known(&self) -> bool {
        self.e.is_some()
            && self.m.is_some()
            && self.x.is_some()
            && self.dbr.is_some()
            && self.d.is_some()
    }
}

/// A declared control-flow or script entry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryPoint {
    /// Stable language-neutral ID.
    pub id: String,
    /// Optional ca65-compatible name.
    #[serde(default)]
    pub asm_name: Option<String>,
    /// Start in the normalized image.
    #[serde(deserialize_with = "deserialize_normalized")]
    pub normalized: NormalizedOffset,
    /// Canonical address for the same byte.
    #[serde(deserialize_with = "deserialize_canonical")]
    pub canonical: CanonicalRomAddress,
    /// Runtime mirror at which control is known to arrive.
    #[serde(deserialize_with = "deserialize_runtime")]
    pub runtime: RuntimeRomAddress,
    /// Semantic role.
    pub kind: EntryKind,
    /// Strength of the declaration.
    pub confidence: Confidence,
    /// Inspectable evidence supporting the declaration.
    pub evidence: Vec<Evidence>,
    /// CPU state known at this entry, if any.
    #[serde(default)]
    pub decode_state: Option<DecodeState>,
}

/// Packed layout of a ROM-backed pointer table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableLayout {
    /// Number of records in the table.
    pub entry_count: u32,
    /// Distance in bytes between adjacent records.
    pub stride: u32,
    /// Pointer field offset within each record.
    pub pointer_offset: u32,
}

/// Encoding used by an indirect pointer source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PointerEncoding {
    /// Little-endian 16-bit address interpreted in a fixed runtime bank.
    BankLocalU16 {
        /// Runtime program/data bank supplying the omitted byte.
        #[serde(deserialize_with = "deserialize_u8")]
        runtime_bank: u8,
    },
    /// Little-endian 24-bit runtime `HiROM` address.
    RuntimeU24,
    /// Little-endian 24-bit normalized image offset.
    NormalizedU24,
}

impl PointerEncoding {
    const fn width(self) -> u32 {
        match self {
            Self::BankLocalU16 { .. } => 2,
            Self::RuntimeU24 | Self::NormalizedU24 => 3,
        }
    }
}

/// A validated 24-bit pointer-field address outside ROM-backed windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MutableMemoryAddress(u32);

impl MutableMemoryAddress {
    /// Constructs a non-ROM bus address.
    ///
    /// # Errors
    ///
    /// Returns an error if the value is wider than 24 bits or maps to ROM.
    pub fn new(value: u32) -> Result<Self, &'static str> {
        if value > 0xff_ffff {
            Err("mutable address is wider than 24 bits")
        } else if RuntimeRomAddress::new(value).is_ok() {
            Err("mutable address lies in a ROM-backed window")
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the numeric 24-bit bus address.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }
}

/// Where an indirect dispatch obtains its pointer.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DispatchSource {
    /// Pointer fields stored in a typed ROM data region.
    RomTable {
        /// ID of the canonical data region containing the complete layout.
        region_id: String,
        /// Record and pointer-field layout.
        layout: TableLayout,
        /// Pointer representation.
        pointer_encoding: PointerEncoding,
    },
    /// Pointer read from mutable SNES memory and therefore not image-resolved.
    MutableMemory {
        /// 24-bit bus address of the pointer field.
        #[serde(deserialize_with = "deserialize_mutable_address")]
        address: MutableMemoryAddress,
        /// Pointer representation found at that address.
        pointer_encoding: PointerEncoding,
    },
}

/// Semantic role of an indirect dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchKind {
    /// Dispatch through a function-pointer table.
    FunctionPointerTable,
    /// Dispatch through records containing callbacks.
    CallbackRecords,
    /// Dispatch into script entries.
    ScriptTable,
    /// Dispatch through a pointer stored in mutable memory.
    MemoryCallback,
}

/// A declared selector/index and its exact entry target.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchTarget {
    /// Selector or record index.
    pub index: u32,
    /// ID of the exact target [`EntryPoint`].
    pub entry_id: String,
}

/// One indirect dispatch site and its known targets.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndirectDispatch {
    /// Stable language-neutral ID.
    pub id: String,
    /// Semantic dispatch role.
    pub kind: DispatchKind,
    /// ROM or mutable-memory pointer source.
    pub source: DispatchSource,
    /// Declared selector/index targets, in ascending index order.
    pub targets: Vec<DispatchTarget>,
    /// Strength of the declaration.
    pub confidence: Confidence,
    /// Inspectable evidence supporting the declaration.
    pub evidence: Vec<Evidence>,
}

/// A disputed range retained outside canonical address lookup.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConflictingRegionClaim {
    /// ID of the canonical region whose boundary/class is disputed.
    pub region_id: String,
    /// Claimed normalized start.
    #[serde(deserialize_with = "deserialize_normalized")]
    pub normalized: NormalizedOffset,
    /// Claimed canonical start.
    #[serde(deserialize_with = "deserialize_canonical")]
    pub canonical: CanonicalRomAddress,
    /// Claimed byte length.
    pub length: u32,
    /// Claimed classification.
    pub class: RegionClass,
    /// Source making the conflicting claim.
    pub source_id: String,
    /// Neutral explanation of the conflict.
    pub note: String,
}

/// A target decoded from a ROM-backed table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDispatchTarget {
    /// Selector or record index.
    pub index: u32,
    /// Declared target entry ID.
    pub entry_id: String,
    /// Decoded normalized target.
    pub normalized: NormalizedOffset,
    /// Decoded canonical target.
    pub canonical: CanonicalRomAddress,
    /// Decoded runtime target when the encoding carries runtime-bank semantics.
    pub runtime: Option<RuntimeRomAddress>,
}

/// A validated revision-bound ROM classification map.
#[derive(Debug, Clone)]
pub struct RomMap {
    schema_version: u32,
    revision: String,
    rom_sha256: String,
    sources: Vec<Source>,
    regions: Vec<RomRegion>,
    entry_points: Vec<EntryPoint>,
    indirect_dispatches: Vec<IndirectDispatch>,
    conflicting_region_claims: Vec<ConflictingRegionClaim>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SerializedRomMap {
    schema_version: u32,
    revision: String,
    rom_sha256: String,
    sources: Vec<Source>,
    regions: Vec<RomRegion>,
    entry_points: Vec<EntryPoint>,
    indirect_dispatches: Vec<IndirectDispatch>,
    #[serde(default)]
    conflicting_region_claims: Vec<ConflictingRegionClaim>,
}

impl RomMap {
    /// Loads and validates a ROM map from JSON.
    ///
    /// # Errors
    ///
    /// Returns a parse error for malformed JSON and a validation error for any
    /// schema invariant violation.
    pub fn from_json(json: &str) -> Result<Self, RomMapLoadError> {
        let serialized: SerializedRomMap =
            serde_json::from_str(json).map_err(RomMapLoadError::Json)?;
        let map = Self {
            schema_version: serialized.schema_version,
            revision: serialized.revision,
            rom_sha256: serialized.rom_sha256,
            sources: serialized.sources,
            regions: serialized.regions,
            entry_points: serialized.entry_points,
            indirect_dispatches: serialized.indirect_dispatches,
            conflicting_region_claims: serialized.conflicting_region_claims,
        };
        map.validate().map_err(RomMapLoadError::Validation)?;
        Ok(map)
    }

    /// Loads the committed Japanese revision map.
    ///
    /// # Errors
    ///
    /// Returns an error if the committed artifact does not satisfy this schema.
    pub fn built_in_japan() -> Result<Self, RomMapLoadError> {
        Self::from_json(JAPAN_MAP_JSON)
    }

    /// Returns the serialized schema version.
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Returns the supported ROM revision.
    #[must_use]
    pub fn revision(&self) -> Revision {
        debug_assert_eq!(self.revision, Revision::Japan.id());
        Revision::Japan
    }

    /// Returns the expected SHA-256 of the normalized image.
    #[must_use]
    pub fn rom_sha256(&self) -> [u8; 32] {
        self.revision().sha256()
    }

    /// Returns the provenance sources.
    #[must_use]
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }

    /// Returns canonical regions in ascending normalized-address order.
    #[must_use]
    pub fn regions(&self) -> &[RomRegion] {
        &self.regions
    }

    /// Returns all declared entry points.
    #[must_use]
    pub fn entry_points(&self) -> &[EntryPoint] {
        &self.entry_points
    }

    /// Returns all indirect dispatch declarations.
    #[must_use]
    pub fn indirect_dispatches(&self) -> &[IndirectDispatch] {
        &self.indirect_dispatches
    }

    /// Finds a canonical region by stable ID.
    #[must_use]
    pub fn region_by_id(&self, id: &str) -> Option<&RomRegion> {
        self.regions.iter().find(|region| region.id == id)
    }

    /// Finds the canonical region containing a normalized offset.
    #[must_use]
    pub fn region_at(&self, offset: NormalizedOffset) -> Option<&RomRegion> {
        let value = offset.value();
        self.regions.iter().find(|region| {
            value >= region.normalized.value() && value < region.normalized.value() + region.length
        })
    }

    /// Finds a canonical region through its canonical address.
    #[must_use]
    pub fn region_at_canonical(&self, address: CanonicalRomAddress) -> Option<&RomRegion> {
        self.region_at(address.normalized())
    }

    /// Finds a canonical region through any mapped runtime mirror.
    #[must_use]
    pub fn region_at_runtime(&self, address: RuntimeRomAddress) -> Option<&RomRegion> {
        self.region_at(address.normalized())
    }

    /// Finds an entry by stable ID.
    #[must_use]
    pub fn entry_by_id(&self, id: &str) -> Option<&EntryPoint> {
        self.entry_points.iter().find(|entry| entry.id == id)
    }

    /// Finds an entry whose declared start exactly matches an offset.
    #[must_use]
    pub fn entry_at(&self, offset: NormalizedOffset) -> Option<&EntryPoint> {
        self.entry_points
            .iter()
            .find(|entry| entry.normalized == offset)
    }

    /// Finds an entry through its canonical address.
    #[must_use]
    pub fn entry_at_canonical(&self, address: CanonicalRomAddress) -> Option<&EntryPoint> {
        self.entry_at(address.normalized())
    }

    /// Finds an entry through any mapped runtime mirror.
    #[must_use]
    pub fn entry_at_runtime(&self, address: RuntimeRomAddress) -> Option<&EntryPoint> {
        self.entry_at(address.normalized())
    }

    /// Returns only executable entries with a completely known decoder state.
    pub fn decoder_seeds(&self) -> impl Iterator<Item = &EntryPoint> {
        self.entry_points.iter().filter(|entry| {
            entry.kind != EntryKind::Script
                && entry
                    .decode_state
                    .as_ref()
                    .is_some_and(DecodeState::is_fully_known)
        })
    }

    /// Generates deterministic ca65 constants for named ROM entries.
    ///
    /// Every source-facing assembly name receives distinct `Canonical` and
    /// `Runtime` suffixes. Constants are sorted by their complete generated
    /// names, and the header contains only stable schema/revision identity.
    #[must_use]
    pub fn generate_ca65_include(&self) -> String {
        let mut constants = self
            .entry_points
            .iter()
            .filter_map(|entry| entry.asm_name.as_deref().map(|name| (entry, name)))
            .flat_map(|(entry, name)| {
                [
                    (format!("{name}Canonical"), entry.canonical.value()),
                    (format!("{name}Runtime"), entry.runtime.value()),
                ]
            })
            .collect::<Vec<_>>();
        constants.sort_unstable_by(|left, right| left.0.cmp(&right.0));

        let mut include = format!(
            "; Generated ROM map constants.\n; Schema version: {}\n; Revision: {}\n; ROM SHA-256: {}\n\n",
            self.schema_version, self.revision, self.rom_sha256
        );
        for (name, value) in constants {
            let _ = writeln!(include, "{name} = ${value:06X}");
        }
        include
    }

    /// Finds an indirect dispatch by stable ID.
    #[must_use]
    pub fn indirect_dispatch_by_id(&self, id: &str) -> Option<&IndirectDispatch> {
        self.indirect_dispatches
            .iter()
            .find(|dispatch| dispatch.id == id)
    }

    /// Returns disputed claims for a canonical region.
    pub fn conflicting_claims_for<'a>(
        &'a self,
        region_id: &'a str,
    ) -> impl Iterator<Item = &'a ConflictingRegionClaim> + 'a {
        self.conflicting_region_claims
            .iter()
            .filter(move |claim| claim.region_id == region_id)
    }

    /// Validates that an authenticated [`Rom`] is the image bound to this map.
    ///
    /// This deliberately compares both the map digest with
    /// [`Revision::sha256`] during map validation and with
    /// [`Rom::digests`] here, preventing metadata-only revision matches.
    ///
    /// # Errors
    ///
    /// Returns an error for a different revision or normalized-image digest.
    pub fn validate_rom(&self, rom: &Rom) -> Result<(), RomMapImageError> {
        if rom.revision() != self.revision() {
            return Err(RomMapImageError::WrongRevision {
                expected: self.revision(),
                actual: rom.revision(),
            });
        }
        let actual = rom.digests().sha256;
        let expected = self.rom_sha256();
        if actual != expected {
            return Err(RomMapImageError::DigestMismatch { expected, actual });
        }
        Ok(())
    }

    /// Resolves declared targets of one ROM-backed dispatch from image bytes.
    ///
    /// Mutable-memory dispatches are intentionally not image-resolved. This
    /// byte-slice entry point performs structural and pointer validation but no
    /// image identity check; use [`Self::resolve_dispatch`] for an authenticated
    /// [`Rom`].
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown or mutable dispatch, a truncated image,
    /// an invalid encoded pointer, or a pointer that differs from its declared
    /// exact entry start.
    pub fn resolve_dispatch_from_image(
        &self,
        dispatch_id: &str,
        image: &[u8],
    ) -> Result<Vec<ResolvedDispatchTarget>, DispatchResolveError> {
        let dispatch = self
            .indirect_dispatch_by_id(dispatch_id)
            .ok_or_else(|| DispatchResolveError::UnknownDispatch(dispatch_id.to_owned()))?;
        let DispatchSource::RomTable {
            region_id,
            layout,
            pointer_encoding,
        } = &dispatch.source
        else {
            return Err(DispatchResolveError::MutableMemorySource {
                id: dispatch.id.clone(),
            });
        };
        let region =
            self.region_by_id(region_id)
                .ok_or_else(|| DispatchResolveError::UnknownRegion {
                    id: dispatch.id.clone(),
                    region_id: region_id.clone(),
                })?;
        validate_table_image(dispatch, region, *layout, *pointer_encoding, image)?;
        dispatch
            .targets
            .iter()
            .map(|target| {
                self.resolve_table_target(
                    dispatch,
                    region,
                    *layout,
                    *pointer_encoding,
                    target,
                    image,
                )
            })
            .collect()
    }

    fn resolve_table_target(
        &self,
        dispatch: &IndirectDispatch,
        region: &RomRegion,
        layout: TableLayout,
        encoding: PointerEncoding,
        target: &DispatchTarget,
        image: &[u8],
    ) -> Result<ResolvedDispatchTarget, DispatchResolveError> {
        let relative = target
            .index
            .checked_mul(layout.stride)
            .and_then(|value| value.checked_add(layout.pointer_offset))
            .ok_or_else(|| invalid_layout(dispatch))?;
        let offset = region
            .normalized
            .value()
            .checked_add(relative)
            .ok_or_else(|| invalid_layout(dispatch))?;
        let width = encoding.width();
        let start = usize::try_from(offset).map_err(|_| invalid_layout(dispatch))?;
        let end = start
            .checked_add(usize::try_from(width).map_err(|_| invalid_layout(dispatch))?)
            .ok_or_else(|| invalid_layout(dispatch))?;
        let bytes = image
            .get(start..end)
            .ok_or(DispatchResolveError::TruncatedImage {
                id: dispatch.id.clone(),
                offset,
                width,
                image_len: image.len(),
            })?;
        let decoded = decode_pointer(encoding, bytes).map_err(|address| {
            DispatchResolveError::InvalidPointer {
                id: dispatch.id.clone(),
                index: target.index,
                address,
            }
        })?;
        let declared = self.entry_by_id(&target.entry_id).ok_or_else(|| {
            DispatchResolveError::UnknownTarget {
                id: dispatch.id.clone(),
                entry_id: target.entry_id.clone(),
            }
        })?;
        validate_decoded_target(dispatch, target, declared, decoded)?;
        Ok(ResolvedDispatchTarget {
            index: target.index,
            entry_id: target.entry_id.clone(),
            normalized: decoded.normalized,
            canonical: decoded.normalized.canonical(),
            runtime: decoded.runtime,
        })
    }

    /// Authenticates a ROM and resolves one ROM-backed dispatch from it.
    ///
    /// # Errors
    ///
    /// Returns either an image identity failure or a dispatch resolution
    /// failure.
    pub fn resolve_dispatch(
        &self,
        dispatch_id: &str,
        rom: &Rom,
    ) -> Result<Vec<ResolvedDispatchTarget>, RomDispatchError> {
        self.validate_rom(rom).map_err(RomDispatchError::Image)?;
        self.resolve_dispatch_from_image(dispatch_id, rom.image())
            .map_err(RomDispatchError::Dispatch)
    }

    fn validate(&self) -> Result<(), RomMapValidationError> {
        if self.schema_version != ROM_MAP_SCHEMA_VERSION {
            return Err(RomMapValidationError::UnsupportedSchemaVersion {
                found: self.schema_version,
            });
        }
        if self.revision != Revision::Japan.id() {
            return Err(RomMapValidationError::UnsupportedRevision {
                revision: self.revision.clone(),
            });
        }
        let digest = parse_sha256(&self.rom_sha256).ok_or_else(|| {
            RomMapValidationError::InvalidRomSha256 {
                value: self.rom_sha256.clone(),
            }
        })?;
        if digest != Revision::Japan.sha256() {
            return Err(RomMapValidationError::RevisionDigestMismatch {
                revision: self.revision.clone(),
            });
        }

        let source_kinds = validate_sources(&self.sources)?;
        self.validate_regions(&source_kinds)?;
        self.validate_entries(&source_kinds)?;
        self.validate_dispatches(&source_kinds)?;
        self.validate_conflicting_claims(&source_kinds)?;
        Ok(())
    }

    fn validate_regions(
        &self,
        source_kinds: &HashMap<&str, SourceKind>,
    ) -> Result<(), RomMapValidationError> {
        let mut ids = HashSet::new();
        let mut previous: Option<&RomRegion> = None;
        for region in &self.regions {
            validate_id(&region.id, "region")?;
            if !ids.insert(region.id.as_str()) {
                return Err(RomMapValidationError::DuplicateId {
                    category: "region",
                    id: region.id.clone(),
                });
            }
            if region.canonical.normalized() != region.normalized {
                return Err(RomMapValidationError::RegionAddressMismatch {
                    id: region.id.clone(),
                });
            }
            let end = checked_region_end(region.normalized, region.length).ok_or_else(|| {
                RomMapValidationError::InvalidRegionRange {
                    id: region.id.clone(),
                }
            })?;
            validate_evidence(
                &region.id,
                region.confidence,
                &region.evidence,
                source_kinds,
            )?;
            if let Some(left) = previous {
                let left_end = left.normalized.value() + left.length;
                if region.normalized.value() < left.normalized.value() {
                    return Err(RomMapValidationError::RegionsNotCanonical {
                        first_id: left.id.clone(),
                        second_id: region.id.clone(),
                    });
                }
                if region.normalized.value() < left_end {
                    return Err(RomMapValidationError::OverlappingRegions {
                        first_id: left.id.clone(),
                        second_id: region.id.clone(),
                    });
                }
            }
            debug_assert!(end <= NormalizedOffset::MAX);
            previous = Some(region);
        }
        Ok(())
    }

    fn validate_entries(
        &self,
        source_kinds: &HashMap<&str, SourceKind>,
    ) -> Result<(), RomMapValidationError> {
        let mut ids = HashSet::new();
        let mut starts = HashSet::new();
        for entry in &self.entry_points {
            validate_id(&entry.id, "entry point")?;
            if !ids.insert(entry.id.as_str()) {
                return Err(RomMapValidationError::DuplicateId {
                    category: "entry point",
                    id: entry.id.clone(),
                });
            }
            if !starts.insert(entry.normalized) {
                return Err(RomMapValidationError::DuplicateEntryStart {
                    id: entry.id.clone(),
                });
            }
            if entry.canonical.normalized() != entry.normalized
                || entry.runtime.normalized() != entry.normalized
            {
                return Err(RomMapValidationError::EntryAddressMismatch {
                    id: entry.id.clone(),
                });
            }
            if entry
                .asm_name
                .as_deref()
                .is_some_and(|name| !asm_name_is_valid(name))
            {
                return Err(RomMapValidationError::InvalidAsmName {
                    id: entry.id.clone(),
                });
            }
            if entry.decode_state.as_ref().is_some_and(|state| {
                state.e == Some(true) && (state.m == Some(false) || state.x == Some(false))
            }) {
                return Err(RomMapValidationError::InvalidDecodeState {
                    id: entry.id.clone(),
                });
            }
            let owner = self.region_at(entry.normalized).ok_or_else(|| {
                RomMapValidationError::EntryWithoutOwnership {
                    id: entry.id.clone(),
                }
            })?;
            let compatible = match entry.kind {
                EntryKind::Script => matches!(owner.class, RegionClass::Data(_)),
                EntryKind::Function | EntryKind::Interrupt | EntryKind::Internal => {
                    owner.class == RegionClass::Code
                }
            };
            if !compatible {
                return Err(RomMapValidationError::EntryOwnershipMismatch {
                    id: entry.id.clone(),
                });
            }
            validate_evidence(&entry.id, entry.confidence, &entry.evidence, source_kinds)?;
        }
        validate_generated_names(&self.entry_points)
    }

    fn validate_dispatches(
        &self,
        source_kinds: &HashMap<&str, SourceKind>,
    ) -> Result<(), RomMapValidationError> {
        let mut ids = HashSet::new();
        for dispatch in &self.indirect_dispatches {
            validate_id(&dispatch.id, "dispatch")?;
            if !ids.insert(dispatch.id.as_str()) {
                return Err(RomMapValidationError::DuplicateId {
                    category: "dispatch",
                    id: dispatch.id.clone(),
                });
            }
            validate_evidence(
                &dispatch.id,
                dispatch.confidence,
                &dispatch.evidence,
                source_kinds,
            )?;
            let entry_count = self.validate_dispatch_source(dispatch)?;
            self.validate_dispatch_targets(dispatch, entry_count)?;
        }
        Ok(())
    }

    fn validate_dispatch_source(
        &self,
        dispatch: &IndirectDispatch,
    ) -> Result<Option<u32>, RomMapValidationError> {
        let DispatchSource::RomTable {
            region_id,
            layout,
            pointer_encoding,
        } = &dispatch.source
        else {
            let DispatchSource::MutableMemory {
                address,
                pointer_encoding,
            } = &dispatch.source
            else {
                unreachable!();
            };
            let pointer_width = pointer_encoding.width();
            let pointer_end = address.value().checked_add(pointer_width - 1);
            if pointer_end.is_none_or(|end| end > 0xff_ffff)
                || (0..pointer_width)
                    .any(|byte| RuntimeRomAddress::new(address.value() + byte).is_ok())
            {
                return Err(RomMapValidationError::InvalidMutablePointerSource {
                    id: dispatch.id.clone(),
                });
            }
            return if dispatch.kind == DispatchKind::MemoryCallback {
                Ok(None)
            } else {
                Err(RomMapValidationError::DispatchSourceKindMismatch {
                    id: dispatch.id.clone(),
                })
            };
        };
        if dispatch.kind == DispatchKind::MemoryCallback {
            return Err(RomMapValidationError::DispatchSourceKindMismatch {
                id: dispatch.id.clone(),
            });
        }
        let region = self.region_by_id(region_id).ok_or_else(|| {
            RomMapValidationError::UnknownDispatchRegion {
                id: dispatch.id.clone(),
                region_id: region_id.clone(),
            }
        })?;
        let expected = match dispatch.kind {
            DispatchKind::FunctionPointerTable => DataKind::FunctionPointerTable,
            DispatchKind::CallbackRecords => DataKind::CallbackRecords,
            DispatchKind::ScriptTable => DataKind::ScriptEntryTable,
            DispatchKind::MemoryCallback => return Ok(None),
        };
        if region.class != RegionClass::Data(expected) {
            return Err(RomMapValidationError::DispatchRegionKindMismatch {
                id: dispatch.id.clone(),
            });
        }
        validate_layout(
            &dispatch.id,
            *layout,
            pointer_encoding.width(),
            region.length,
        )?;
        Ok(Some(layout.entry_count))
    }

    fn validate_dispatch_targets(
        &self,
        dispatch: &IndirectDispatch,
        entry_count: Option<u32>,
    ) -> Result<(), RomMapValidationError> {
        let pointer_encoding = match dispatch.source {
            DispatchSource::RomTable {
                pointer_encoding, ..
            }
            | DispatchSource::MutableMemory {
                pointer_encoding, ..
            } => pointer_encoding,
        };
        let mut previous_index = None;
        for target in &dispatch.targets {
            if previous_index.is_some_and(|previous| target.index <= previous) {
                return Err(RomMapValidationError::DispatchTargetsNotCanonical {
                    id: dispatch.id.clone(),
                });
            }
            if entry_count.is_some_and(|count| target.index >= count) {
                return Err(RomMapValidationError::DispatchIndexOutOfRange {
                    id: dispatch.id.clone(),
                    index: target.index,
                });
            }
            let entry = self.entry_by_id(&target.entry_id).ok_or_else(|| {
                RomMapValidationError::UnknownDispatchTarget {
                    id: dispatch.id.clone(),
                    entry_id: target.entry_id.clone(),
                }
            })?;
            if !dispatch_target_compatible(dispatch.kind, entry.kind) {
                return Err(RomMapValidationError::DispatchTargetKindMismatch {
                    id: dispatch.id.clone(),
                    entry_id: target.entry_id.clone(),
                });
            }
            if !pointer_can_represent(pointer_encoding, entry) {
                return Err(RomMapValidationError::PointerCannotRepresentTarget {
                    id: dispatch.id.clone(),
                    entry_id: target.entry_id.clone(),
                });
            }
            previous_index = Some(target.index);
        }
        Ok(())
    }

    fn validate_conflicting_claims(
        &self,
        source_kinds: &HashMap<&str, SourceKind>,
    ) -> Result<(), RomMapValidationError> {
        for (index, claim) in self.conflicting_region_claims.iter().enumerate() {
            let Some(canonical) = self.region_by_id(&claim.region_id) else {
                return Err(RomMapValidationError::UnknownClaimRegion {
                    region_id: claim.region_id.clone(),
                });
            };
            if claim.canonical.normalized() != claim.normalized
                || checked_region_end(claim.normalized, claim.length).is_none()
                || claim.note.trim().is_empty()
                || !ranges_overlap(
                    claim.normalized,
                    claim.length,
                    canonical.normalized,
                    canonical.length,
                )
                || self.conflicting_region_claims[..index]
                    .iter()
                    .any(|previous| {
                        previous.region_id == claim.region_id
                            && previous.normalized == claim.normalized
                            && previous.length == claim.length
                            && previous.class == claim.class
                    })
            {
                return Err(RomMapValidationError::InvalidConflictingClaim {
                    region_id: claim.region_id.clone(),
                });
            }
            if !source_kinds.contains_key(claim.source_id.as_str()) {
                return Err(RomMapValidationError::UnknownClaimSource {
                    source_id: claim.source_id.clone(),
                });
            }
            if claim.normalized == canonical.normalized
                && claim.length == canonical.length
                && claim.class == canonical.class
            {
                return Err(RomMapValidationError::NonConflictingClaim {
                    region_id: claim.region_id.clone(),
                });
            }
        }
        Ok(())
    }
}

fn invalid_layout(dispatch: &IndirectDispatch) -> DispatchResolveError {
    DispatchResolveError::InvalidLayout {
        id: dispatch.id.clone(),
    }
}

fn validate_table_image(
    dispatch: &IndirectDispatch,
    region: &RomRegion,
    layout: TableLayout,
    encoding: PointerEncoding,
    image: &[u8],
) -> Result<(), DispatchResolveError> {
    let footprint =
        table_footprint(layout, encoding.width()).ok_or_else(|| invalid_layout(dispatch))?;
    let start = usize::try_from(region.normalized.value()).map_err(|_| invalid_layout(dispatch))?;
    let end = start
        .checked_add(usize::try_from(footprint).map_err(|_| invalid_layout(dispatch))?)
        .ok_or_else(|| invalid_layout(dispatch))?;
    if image.get(start..end).is_none() {
        return Err(DispatchResolveError::TruncatedImage {
            id: dispatch.id.clone(),
            offset: region.normalized.value(),
            width: footprint,
            image_len: image.len(),
        });
    }
    Ok(())
}

fn validate_decoded_target(
    dispatch: &IndirectDispatch,
    target: &DispatchTarget,
    declared: &EntryPoint,
    decoded: DecodedPointer,
) -> Result<(), DispatchResolveError> {
    if decoded.normalized != declared.normalized {
        return Err(DispatchResolveError::TargetMismatch {
            id: dispatch.id.clone(),
            index: target.index,
            expected: declared.normalized,
            actual: decoded.normalized,
        });
    }
    if let Some(actual) = decoded.runtime {
        if actual != declared.runtime {
            return Err(DispatchResolveError::RuntimeTargetMismatch {
                id: dispatch.id.clone(),
                index: target.index,
                expected: declared.runtime,
                actual,
            });
        }
    }
    Ok(())
}

fn pointer_can_represent(encoding: PointerEncoding, entry: &EntryPoint) -> bool {
    match encoding {
        PointerEncoding::BankLocalU16 { runtime_bank } => {
            let Ok(offset) = u16::try_from(entry.canonical.value() & 0xffff) else {
                return false;
            };
            RuntimeRomAddress::from_parts(runtime_bank, offset)
                .is_ok_and(|address| address == entry.runtime)
        }
        PointerEncoding::RuntimeU24 | PointerEncoding::NormalizedU24 => true,
    }
}

fn dispatch_target_compatible(dispatch: DispatchKind, entry: EntryKind) -> bool {
    match dispatch {
        DispatchKind::ScriptTable => entry == EntryKind::Script,
        DispatchKind::FunctionPointerTable
        | DispatchKind::CallbackRecords
        | DispatchKind::MemoryCallback => {
            matches!(entry, EntryKind::Function | EntryKind::Internal)
        }
    }
}

fn validate_layout(
    id: &str,
    layout: TableLayout,
    pointer_width: u32,
    region_length: u32,
) -> Result<(), RomMapValidationError> {
    if layout.entry_count == 0
        || layout.stride == 0
        || layout
            .pointer_offset
            .checked_add(pointer_width)
            .is_none_or(|end| end > layout.stride)
    {
        return Err(RomMapValidationError::InvalidTableLayout { id: id.to_owned() });
    }
    let footprint = table_footprint(layout, pointer_width);
    if footprint.is_none_or(|length| length > region_length) {
        return Err(RomMapValidationError::TableLayoutOutsideRegion { id: id.to_owned() });
    }
    Ok(())
}

fn table_footprint(layout: TableLayout, _pointer_width: u32) -> Option<u32> {
    layout.entry_count.checked_mul(layout.stride)
}

#[derive(Debug, Clone, Copy)]
struct DecodedPointer {
    normalized: NormalizedOffset,
    runtime: Option<RuntimeRomAddress>,
}

fn decode_pointer(encoding: PointerEncoding, bytes: &[u8]) -> Result<DecodedPointer, u32> {
    match encoding {
        PointerEncoding::BankLocalU16 { runtime_bank } => {
            let offset = u16::from_le_bytes([bytes[0], bytes[1]]);
            let raw = u32::from(runtime_bank) << 16 | u32::from(offset);
            RuntimeRomAddress::from_parts(runtime_bank, offset)
                .map(|runtime| DecodedPointer {
                    normalized: runtime.normalized(),
                    runtime: Some(runtime),
                })
                .map_err(|_| raw)
        }
        PointerEncoding::RuntimeU24 => {
            let raw = little_u24(bytes);
            RuntimeRomAddress::new(raw)
                .map(|runtime| DecodedPointer {
                    normalized: runtime.normalized(),
                    runtime: Some(runtime),
                })
                .map_err(|_| raw)
        }
        PointerEncoding::NormalizedU24 => {
            let raw = little_u24(bytes);
            NormalizedOffset::new(raw)
                .map(|normalized| DecodedPointer {
                    normalized,
                    runtime: None,
                })
                .map_err(|_| raw)
        }
    }
}

fn little_u24(bytes: &[u8]) -> u32 {
    u32::from(bytes[0]) | u32::from(bytes[1]) << 8 | u32::from(bytes[2]) << 16
}

fn ranges_overlap(
    left_start: NormalizedOffset,
    left_length: u32,
    right_start: NormalizedOffset,
    right_length: u32,
) -> bool {
    left_start.value() < right_start.value() + right_length
        && right_start.value() < left_start.value() + left_length
}

fn checked_region_end(start: NormalizedOffset, length: u32) -> Option<u32> {
    let last = length.checked_sub(1)?;
    start
        .value()
        .checked_add(last)
        .filter(|end| *end <= NormalizedOffset::MAX)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceKind {
    External,
    Project,
}

fn validate_sources(
    sources: &[Source],
) -> Result<HashMap<&str, SourceKind>, RomMapValidationError> {
    let mut kinds = HashMap::new();
    for source in sources {
        validate_id(&source.id, "source")?;
        if source.title.trim().is_empty() {
            return Err(RomMapValidationError::InvalidSource {
                id: source.id.clone(),
            });
        }
        let kind = match (
            source.url.as_deref(),
            source.retrieved_on.as_deref(),
            source.project_path.as_deref(),
        ) {
            (Some(url), Some(date), None) if valid_url(url) && valid_date(date) => {
                SourceKind::External
            }
            (None, None, Some(path)) if valid_project_path(path) => SourceKind::Project,
            _ => {
                return Err(RomMapValidationError::InvalidSource {
                    id: source.id.clone(),
                });
            }
        };
        if kinds.insert(source.id.as_str(), kind).is_some() {
            return Err(RomMapValidationError::DuplicateId {
                category: "source",
                id: source.id.clone(),
            });
        }
    }
    Ok(kinds)
}

fn validate_evidence(
    owner_id: &str,
    confidence: Confidence,
    evidence: &[Evidence],
    source_kinds: &HashMap<&str, SourceKind>,
) -> Result<(), RomMapValidationError> {
    for item in evidence {
        if item.source_id.trim().is_empty()
            || item.locator.trim().is_empty()
            || item.note.trim().is_empty()
        {
            return Err(RomMapValidationError::InvalidEvidence {
                owner_id: owner_id.to_owned(),
            });
        }
        let Some(kind) = source_kinds.get(item.source_id.as_str()) else {
            return Err(RomMapValidationError::UnknownEvidenceSource {
                owner_id: owner_id.to_owned(),
                source_id: item.source_id.clone(),
            });
        };
        let expected = if item.provenance == Provenance::ImportedClaim {
            SourceKind::External
        } else {
            SourceKind::Project
        };
        if *kind != expected {
            return Err(RomMapValidationError::ProvenanceSourceMismatch {
                owner_id: owner_id.to_owned(),
                source_id: item.source_id.clone(),
            });
        }
    }
    let supported = match confidence {
        Confidence::ImportedClaim => evidence
            .iter()
            .any(|item| item.provenance == Provenance::ImportedClaim),
        Confidence::StaticCorroborated => evidence
            .iter()
            .any(|item| item.provenance == Provenance::StaticAnalysis),
        Confidence::TraceCorroborated => evidence.iter().any(|item| {
            matches!(
                item.provenance,
                Provenance::ProjectTrace | Provenance::ProjectTest
            )
        }),
    };
    if !supported {
        return Err(RomMapValidationError::ConfidenceEvidenceMismatch {
            owner_id: owner_id.to_owned(),
        });
    }
    Ok(())
}

fn validate_id(id: &str, category: &'static str) -> Result<(), RomMapValidationError> {
    let mut bytes = id.bytes();
    let valid = bytes.next().is_some_and(|first| first.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(RomMapValidationError::InvalidId {
            category,
            id: id.to_owned(),
        })
    }
}

fn validate_generated_names(entries: &[EntryPoint]) -> Result<(), RomMapValidationError> {
    let source_names = entries
        .iter()
        .filter_map(|entry| {
            entry
                .asm_name
                .as_ref()
                .map(|name| (name.as_str(), entry.id.as_str()))
        })
        .collect::<HashMap<_, _>>();
    let mut generated_names: HashMap<String, &str> = HashMap::new();

    for entry in entries {
        let Some(asm_name) = entry.asm_name.as_deref() else {
            continue;
        };
        for suffix in ["Canonical", "Runtime"] {
            let name = format!("{asm_name}{suffix}");
            if !asm_name_is_valid(&name) {
                return Err(RomMapValidationError::InvalidGeneratedName {
                    entry_id: entry.id.clone(),
                    name,
                });
            }
            if let Some(source_entry_id) = source_names.get(name.as_str()) {
                return Err(RomMapValidationError::GeneratedNameCollision {
                    name,
                    first_entry_id: entry.id.clone(),
                    second_entry_id: (*source_entry_id).to_owned(),
                });
            }
            if let Some(first_entry_id) = generated_names.insert(name.clone(), &entry.id) {
                return Err(RomMapValidationError::GeneratedNameCollision {
                    name,
                    first_entry_id: first_entry_id.to_owned(),
                    second_entry_id: entry.id.clone(),
                });
            }
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

fn valid_url(url: &str) -> bool {
    (url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://")))
    .is_some_and(|rest| !rest.is_empty() && !url.chars().any(char::is_whitespace))
}

fn valid_project_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn valid_date(date: &str) -> bool {
    let mut parts = date.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        year.parse::<u32>(),
        month.parse::<u32>(),
        day.parse::<u32>(),
    ) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=max_day).contains(&day)
}

fn parse_sha256(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut digest = [0; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_digit(pair[0])?;
        let low = hex_digit(pair[1])?;
        digest[index] = high << 4 | low;
    }
    Some(digest)
}

const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn parse_hex(text: &str) -> Result<u32, String> {
    let compact = text.replace(':', "");
    let digits = compact
        .strip_prefix('$')
        .or_else(|| compact.strip_prefix("0x"))
        .unwrap_or(&compact);
    u32::from_str_radix(digits, 16).map_err(|error| error.to_string())
}

fn deserialize_hex<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    parse_hex(&text).map_err(serde::de::Error::custom)
}

fn deserialize_normalized<'de, D>(deserializer: D) -> Result<NormalizedOffset, D::Error>
where
    D: serde::Deserializer<'de>,
{
    NormalizedOffset::new(deserialize_hex(deserializer)?).map_err(serde::de::Error::custom)
}

fn deserialize_canonical<'de, D>(deserializer: D) -> Result<CanonicalRomAddress, D::Error>
where
    D: serde::Deserializer<'de>,
{
    CanonicalRomAddress::new(deserialize_hex(deserializer)?).map_err(serde::de::Error::custom)
}

fn deserialize_runtime<'de, D>(deserializer: D) -> Result<RuntimeRomAddress, D::Error>
where
    D: serde::Deserializer<'de>,
{
    RuntimeRomAddress::new(deserialize_hex(deserializer)?).map_err(serde::de::Error::custom)
}

fn deserialize_mutable_address<'de, D>(deserializer: D) -> Result<MutableMemoryAddress, D::Error>
where
    D: serde::Deserializer<'de>,
{
    MutableMemoryAddress::new(deserialize_hex(deserializer)?).map_err(serde::de::Error::custom)
}

fn deserialize_u8<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_hex(deserializer)?
        .try_into()
        .map_err(serde::de::Error::custom)
}

fn deserialize_optional_u8<'de, D>(deserializer: D) -> Result<Option<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?
        .map(|text| parse_hex(&text).and_then(|value| value.try_into().map_err(|e| format!("{e}"))))
        .transpose()
        .map_err(serde::de::Error::custom)
}

fn deserialize_optional_u16<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?
        .map(|text| parse_hex(&text).and_then(|value| value.try_into().map_err(|e| format!("{e}"))))
        .transpose()
        .map_err(serde::de::Error::custom)
}

/// Failure while parsing or validating ROM-map JSON.
#[derive(Debug)]
pub enum RomMapLoadError {
    /// JSON deserialization failed.
    Json(serde_json::Error),
    /// A decoded map violated an invariant.
    Validation(RomMapValidationError),
}

impl RomMapLoadError {
    /// Returns the validation failure, if parsing succeeded.
    #[must_use]
    pub const fn validation_error(&self) -> Option<&RomMapValidationError> {
        match self {
            Self::Json(_) => None,
            Self::Validation(error) => Some(error),
        }
    }
}

impl fmt::Display for RomMapLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "invalid ROM-map JSON: {error}"),
            Self::Validation(error) => write!(formatter, "invalid ROM map: {error}"),
        }
    }
}

impl std::error::Error for RomMapLoadError {}

/// A ROM-map schema invariant failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RomMapValidationError {
    /// Schema version is not understood.
    UnsupportedSchemaVersion {
        /// Version found in the artifact.
        found: u32,
    },
    /// No ROM map is available for this revision.
    UnsupportedRevision {
        /// Unsupported stable revision ID.
        revision: String,
    },
    /// Digest text is not exactly 32 bytes of hexadecimal.
    InvalidRomSha256 {
        /// Invalid serialized digest.
        value: String,
    },
    /// Digest does not identify the named supported revision.
    RevisionDigestMismatch {
        /// Revision whose known digest differs.
        revision: String,
    },
    /// A stable ID is malformed.
    InvalidId {
        /// Kind of record containing the ID.
        category: &'static str,
        /// Rejected ID.
        id: String,
    },
    /// An ID is repeated in one category.
    DuplicateId {
        /// Kind of record containing the ID.
        category: &'static str,
        /// Repeated ID.
        id: String,
    },
    /// A source record is malformed.
    InvalidSource {
        /// Source ID.
        id: String,
    },
    /// Evidence has blank required fields.
    InvalidEvidence {
        /// ID of the record owning the evidence.
        owner_id: String,
    },
    /// Evidence cites an absent source.
    UnknownEvidenceSource {
        /// ID of the record owning the evidence.
        owner_id: String,
        /// Missing source ID.
        source_id: String,
    },
    /// Imported/project provenance cites the wrong kind of source.
    ProvenanceSourceMismatch {
        /// ID of the record owning the evidence.
        owner_id: String,
        /// Incompatible source ID.
        source_id: String,
    },
    /// Confidence is unsupported by the attached provenance.
    ConfidenceEvidenceMismatch {
        /// ID of the affected record.
        owner_id: String,
    },
    /// Region normalized and canonical starts differ.
    RegionAddressMismatch {
        /// Region ID.
        id: String,
    },
    /// Region is empty, overflows, or exits the normalized image.
    InvalidRegionRange {
        /// Region ID.
        id: String,
    },
    /// Canonical regions are not in ascending order.
    RegionsNotCanonical {
        /// Earlier serialized region ID.
        first_id: String,
        /// Following serialized region ID.
        second_id: String,
    },
    /// Canonical regions overlap.
    OverlappingRegions {
        /// First overlapping region ID.
        first_id: String,
        /// Second overlapping region ID.
        second_id: String,
    },
    /// Multiple entries declare one exact start.
    DuplicateEntryStart {
        /// Later entry ID.
        id: String,
    },
    /// Entry address representations do not normalize together.
    EntryAddressMismatch {
        /// Entry ID.
        id: String,
    },
    /// Optional ca65 name is malformed.
    InvalidAsmName {
        /// Entry ID.
        id: String,
    },
    /// A derived ca65 constant name is malformed.
    InvalidGeneratedName {
        /// Entry producing the name.
        entry_id: String,
        /// Rejected derived name.
        name: String,
    },
    /// A derived name collides with another derived name or source label.
    GeneratedNameCollision {
        /// Colliding generated name.
        name: String,
        /// Entry first producing or owning the name.
        first_entry_id: String,
        /// Other entry producing or owning the name.
        second_entry_id: String,
    },
    /// Decode state contradicts 65C816 emulation-mode width rules.
    InvalidDecodeState {
        /// Entry ID.
        id: String,
    },
    /// Entry is not contained in a canonical region.
    EntryWithoutOwnership {
        /// Entry ID.
        id: String,
    },
    /// Entry kind disagrees with its code/data owner.
    EntryOwnershipMismatch {
        /// Entry ID.
        id: String,
    },
    /// Dispatch source and semantic kind disagree.
    DispatchSourceKindMismatch {
        /// Dispatch ID.
        id: String,
    },
    /// Mutable pointer field crosses the bus end or a ROM-backed window.
    InvalidMutablePointerSource {
        /// Dispatch ID.
        id: String,
    },
    /// ROM-backed dispatch references an absent region.
    UnknownDispatchRegion {
        /// Dispatch ID.
        id: String,
        /// Missing region ID.
        region_id: String,
    },
    /// ROM-backed dispatch references the wrong typed data kind.
    DispatchRegionKindMismatch {
        /// Dispatch ID.
        id: String,
    },
    /// Table has zero/invalid stride or a pointer crossing its record.
    InvalidTableLayout {
        /// Dispatch ID.
        id: String,
    },
    /// Table layout overflows or exits its referenced region.
    TableLayoutOutsideRegion {
        /// Dispatch ID.
        id: String,
    },
    /// Declared target indices are repeated or unordered.
    DispatchTargetsNotCanonical {
        /// Dispatch ID.
        id: String,
    },
    /// Declared index lies beyond a ROM table.
    DispatchIndexOutOfRange {
        /// Dispatch ID.
        id: String,
        /// Invalid index.
        index: u32,
    },
    /// Declared target entry is absent.
    UnknownDispatchTarget {
        /// Dispatch ID.
        id: String,
        /// Missing entry ID.
        entry_id: String,
    },
    /// Declared target entry has an incompatible semantic kind.
    DispatchTargetKindMismatch {
        /// Dispatch ID.
        id: String,
        /// Incompatible entry ID.
        entry_id: String,
    },
    /// Declared target cannot be encoded by the selected pointer representation.
    PointerCannotRepresentTarget {
        /// Dispatch ID.
        id: String,
        /// Unrepresentable entry ID.
        entry_id: String,
    },
    /// Conflicting claim names an absent canonical region.
    UnknownClaimRegion {
        /// Missing region ID.
        region_id: String,
    },
    /// Conflicting claim has inconsistent addresses or range.
    InvalidConflictingClaim {
        /// Canonical region ID.
        region_id: String,
    },
    /// Conflicting claim cites an absent source.
    UnknownClaimSource {
        /// Missing source ID.
        source_id: String,
    },
    /// A purported conflict exactly repeats the canonical selection.
    NonConflictingClaim {
        /// Canonical region ID.
        region_id: String,
    },
}

impl fmt::Display for RomMapValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RomMapValidationError {}

/// Failure to match an authenticated ROM to a map.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RomMapImageError {
    /// The authenticated image is a different revision.
    WrongRevision {
        /// Revision required by the map.
        expected: Revision,
        /// Revision authenticated for the image.
        actual: Revision,
    },
    /// The normalized bytes do not have the map's digest.
    DigestMismatch {
        /// SHA-256 required by the map.
        expected: [u8; 32],
        /// SHA-256 computed from the image.
        actual: [u8; 32],
    },
}

impl fmt::Display for RomMapImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RomMapImageError {}

/// Failure while resolving an indirect dispatch from image bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DispatchResolveError {
    /// No dispatch has this ID.
    UnknownDispatch(String),
    /// Mutable memory cannot be resolved from immutable ROM bytes.
    MutableMemorySource {
        /// Mutable dispatch ID.
        id: String,
    },
    /// Validated dispatch unexpectedly references an absent region.
    UnknownRegion {
        /// Dispatch ID.
        id: String,
        /// Missing region ID.
        region_id: String,
    },
    /// Validated dispatch contains arithmetic that cannot be represented.
    InvalidLayout {
        /// Dispatch ID.
        id: String,
    },
    /// Validated dispatch unexpectedly references an absent target.
    UnknownTarget {
        /// Dispatch ID.
        id: String,
        /// Missing entry ID.
        entry_id: String,
    },
    /// Image ends before a required pointer field.
    TruncatedImage {
        /// Dispatch ID.
        id: String,
        /// Normalized start of the pointer field.
        offset: u32,
        /// Required pointer width.
        width: u32,
        /// Supplied image length.
        image_len: usize,
    },
    /// Encoded pointer is outside the selected ROM address space.
    InvalidPointer {
        /// Dispatch ID.
        id: String,
        /// Selector or record index.
        index: u32,
        /// Rejected encoded address.
        address: u32,
    },
    /// Encoded runtime mirror differs from the declared arrival address.
    RuntimeTargetMismatch {
        /// Dispatch ID.
        id: String,
        /// Selector or record index.
        index: u32,
        /// Declared runtime arrival address.
        expected: RuntimeRomAddress,
        /// Decoded runtime target.
        actual: RuntimeRomAddress,
    },
    /// Encoded target differs from the exact declared entry start.
    TargetMismatch {
        /// Dispatch ID.
        id: String,
        /// Selector or record index.
        index: u32,
        /// Declared normalized entry start.
        expected: NormalizedOffset,
        /// Decoded normalized target.
        actual: NormalizedOffset,
    },
}

impl fmt::Display for DispatchResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DispatchResolveError {}

/// Combined authenticated-ROM and dispatch-resolution failure.
#[derive(Debug)]
pub enum RomDispatchError {
    /// ROM identity does not match the map.
    Image(RomMapImageError),
    /// Pointer-table resolution failed.
    Dispatch(DispatchResolveError),
}

impl fmt::Display for RomDispatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RomDispatchError {}
