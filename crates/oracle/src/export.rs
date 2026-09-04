//! Versioned frame-state export and comparison for divergence diagnosis.
//!
//! Exports are digests over selected regions plus named semantic fields;
//! they are commit-safe (no raw memory) unless a caller deliberately stores
//! raw captures under `local/`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// Export format version.
pub const EXPORT_VERSION: u32 = 1;

/// Canonical symbol-trace format version.
pub const SYMBOL_TRACE_FORMAT_VERSION: u32 = 1;

/// A named region digest (e.g. WRAM range, VRAM, CGRAM, OAM).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionDigest {
    /// Human-readable region name, e.g. `"wram"`, `"vram"`, `"cgram"`, `"oam"`.
    pub name: String,
    /// SHA-256 over the region bytes at this frame.
    pub sha256: String,
}

/// A named semantic field (symbol) with its scalar value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticField {
    /// Symbol name, e.g. `"current_map"`.
    pub name: String,
    /// Raw little-endian bytes of the field.
    pub value: Vec<u8>,
}

/// Failure to export canonical symbols from a WRAM image or construct a symbol trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolExportError {
    /// The trace's format version is not supported by this implementation.
    UnsupportedTraceVersion {
        /// Unsupported version found in the trace.
        found: u32,
    },
    /// The supplied memory map did not pass its schema validation.
    InvalidMemoryMap {
        /// Underlying map validation failure.
        source: memory_map::ValidationError,
    },
    /// The memory map's normalized-ROM digest was not 64 hexadecimal digits.
    InvalidMapRomSha256(String),
    /// The runtime and memory-map normalized ROM identities differ.
    RomIdentityMismatch {
        /// Digest of the normalized ROM used by the emulator.
        runtime_rom_sha256: [u8; 32],
        /// Digest declared by the memory map.
        map_rom_sha256: [u8; 32],
    },
    /// The requested canonical symbol ID does not exist.
    UnknownSymbol(String),
    /// The symbol cannot be sampled from a WRAM image.
    UnsupportedSymbol {
        /// Unsupported canonical symbol ID.
        id: String,
        /// Address space declared by the memory-map schema.
        space: memory_map::AddressSpace,
    },
    /// A count or byte-string length cannot be represented by the format's `u32` fields.
    LengthOverflow {
        /// Name of the value whose length overflowed.
        field: &'static str,
        /// Length supplied by the caller.
        len: usize,
    },
    /// The symbol cannot be read from the supplied WRAM image.
    Read {
        /// Canonical symbol ID that failed.
        id: String,
        /// Underlying map/read failure.
        source: memory_map::ReadError,
    },
}

impl fmt::Display for SymbolExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTraceVersion { found } => write!(
                formatter,
                "symbol-trace version {found} is not supported (expected {SYMBOL_TRACE_FORMAT_VERSION})"
            ),
            Self::InvalidMemoryMap { source } => write!(formatter, "invalid memory map: {source}"),
            Self::InvalidMapRomSha256(value) => {
                write!(formatter, "invalid memory-map ROM SHA-256 {value:?}")
            }
            Self::RomIdentityMismatch {
                runtime_rom_sha256,
                map_rom_sha256,
            } => write!(
                formatter,
                "runtime ROM SHA-256 {} does not match memory-map ROM SHA-256 {}",
                bytes_to_hex(runtime_rom_sha256),
                bytes_to_hex(map_rom_sha256)
            ),
            Self::UnknownSymbol(id) => write!(formatter, "unknown canonical symbol {id}"),
            Self::UnsupportedSymbol { id, space } => {
                write!(
                    formatter,
                    "canonical symbol {id} uses unsupported address space {space:?}"
                )
            }
            Self::LengthOverflow { field, len } => {
                write!(
                    formatter,
                    "{field} length {len} exceeds the symbol-trace format limit"
                )
            }
            Self::Read { id, source } => {
                write!(formatter, "cannot read canonical symbol {id}: {source}")
            }
        }
    }
}

impl std::error::Error for SymbolExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidMemoryMap { source } => Some(source),
            Self::Read { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Reads canonical symbols from WRAM as semantic export fields.
///
/// Output preserves `ids` order and field values preserve their schema widths.
///
/// # Errors
///
/// Returns an error for an unknown ID, unsupported address space, or a WRAM
/// image too short to contain a requested field.
pub fn semantic_fields_from_wram(
    map: &memory_map::MemoryMap,
    wram: &[u8],
    ids: &[&str],
) -> Result<Vec<SemanticField>, SymbolExportError> {
    ids.iter()
        .map(|id| {
            let symbol = map
                .lookup_id(id)
                .ok_or_else(|| SymbolExportError::UnknownSymbol((*id).to_owned()))?;
            let value =
                map.read_symbol(symbol, wram)
                    .map_err(|source| SymbolExportError::Read {
                        id: (*id).to_owned(),
                        source,
                    })?;
            Ok(SemanticField {
                name: (*id).to_owned(),
                value: value.to_vec(),
            })
        })
        .collect()
}

/// Identity metadata bound into a [`SymbolTrace`] digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolTraceMetadata {
    /// SHA-256 of the normalized ROM used by the emulator.
    pub normalized_rom_sha256: [u8; 32],
    /// Version of the canonical memory-map schema.
    pub memory_map_schema_version: u32,
    /// SHA-256 of the normalized ROM declared by the memory map.
    pub memory_map_rom_sha256: [u8; 32],
    /// Stable ID of the scenario that produced this trace.
    pub scenario_id: String,
}

/// One selected canonical symbol definition, in trace order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceSymbol {
    /// Stable language-neutral memory-map symbol ID.
    pub id: String,
    /// Canonical 24-bit SNES bus address.
    pub address: u32,
    /// Symbol width in bytes.
    pub width: u32,
}

/// Values sampled at one named scenario checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolCheckpoint {
    /// Scenario-local frame label; this is not an emulator frame index.
    pub scenario_label: u32,
    /// Actual emulator frame counter when this checkpoint was sampled.
    pub actual_frame: u32,
    /// Raw symbol values in the trace's selected-symbol order.
    pub values: Vec<Vec<u8>>,
}

/// Versioned, identity-bound trace of ordered canonical symbol values.
///
/// This format is separate from [`FrameExport`]: a scenario-local checkpoint
/// label identifies a test step, while [`SymbolCheckpoint::actual_frame`]
/// records the emulator's independent frame counter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolTrace {
    /// Trace format version; equals [`SYMBOL_TRACE_FORMAT_VERSION`] on construction.
    pub version: u32,
    /// ROM, memory-map, and scenario identity.
    pub metadata: SymbolTraceMetadata,
    /// Selected definitions in caller-specified order.
    pub symbols: Vec<TraceSymbol>,
    /// Sampled checkpoints in append order.
    pub checkpoints: Vec<SymbolCheckpoint>,
}

impl SymbolTrace {
    /// Constructs an empty trace from a validated map and selected symbol IDs.
    ///
    /// Selection order is retained. Only direct-page and WRAM symbols can be
    /// sampled from the WRAM image accepted by [`Self::append_checkpoint`].
    ///
    /// # Errors
    ///
    /// Returns an error if the map is invalid, its ROM digest is malformed or
    /// differs from `normalized_rom_sha256`, a selected ID is unknown or not
    /// WRAM-backed, or a format length exceeds `u32::MAX`.
    pub fn new(
        map: &memory_map::MemoryMap,
        normalized_rom_sha256: [u8; 32],
        scenario_id: impl Into<String>,
        selected_symbol_ids: &[&str],
    ) -> Result<Self, SymbolExportError> {
        let map_rom_sha256 = decode_sha256(&map.rom_sha256)
            .ok_or_else(|| SymbolExportError::InvalidMapRomSha256(map.rom_sha256.clone()))?;
        if normalized_rom_sha256 != map_rom_sha256 {
            return Err(SymbolExportError::RomIdentityMismatch {
                runtime_rom_sha256: normalized_rom_sha256,
                map_rom_sha256,
            });
        }
        map.validate()
            .map_err(|source| SymbolExportError::InvalidMemoryMap { source })?;

        let scenario_id = scenario_id.into();
        checked_u32_len("scenario ID", scenario_id.len())?;
        checked_u32_len("selected symbol count", selected_symbol_ids.len())?;
        let symbols = selected_symbol_ids
            .iter()
            .map(|id| {
                let symbol = map
                    .lookup_id(id)
                    .ok_or_else(|| SymbolExportError::UnknownSymbol((*id).to_owned()))?;
                if !matches!(
                    symbol.space,
                    memory_map::AddressSpace::DirectPage | memory_map::AddressSpace::Wram
                ) {
                    return Err(SymbolExportError::UnsupportedSymbol {
                        id: (*id).to_owned(),
                        space: symbol.space,
                    });
                }
                checked_u32_len("symbol ID", symbol.id.len())?;
                Ok(TraceSymbol {
                    id: symbol.id.clone(),
                    address: symbol.address.value(),
                    width: symbol.width,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            version: SYMBOL_TRACE_FORMAT_VERSION,
            metadata: SymbolTraceMetadata {
                normalized_rom_sha256,
                memory_map_schema_version: map.schema_version,
                memory_map_rom_sha256: map_rom_sha256,
                scenario_id,
            },
            symbols,
            checkpoints: Vec::new(),
        })
    }

    /// Samples all selected symbols from `wram` at one checkpoint.
    ///
    /// Values preserve selected-symbol order and exact schema widths. The
    /// checkpoint is appended only after every symbol read succeeds.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported trace version or checkpoint-list
    /// overflow, a definition that is no longer a supported canonical WRAM
    /// region, or short `wram`.
    pub fn append_checkpoint(
        &mut self,
        actual_frame: u32,
        scenario_label: u32,
        wram: &[u8],
    ) -> Result<(), SymbolExportError> {
        self.ensure_supported_version()?;
        let checkpoint_count =
            self.checkpoints
                .len()
                .checked_add(1)
                .ok_or(SymbolExportError::LengthOverflow {
                    field: "checkpoint count",
                    len: usize::MAX,
                })?;
        checked_u32_len("checkpoint count", checkpoint_count)?;
        checked_u32_len("checkpoint value count", self.symbols.len())?;

        let values = self
            .symbols
            .iter()
            .map(|symbol| {
                read_trace_symbol(symbol, wram)
                    .map(<[u8]>::to_vec)
                    .map_err(|source| SymbolExportError::Read {
                        id: symbol.id.clone(),
                        source,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.checkpoints.push(SymbolCheckpoint {
            scenario_label,
            actual_frame,
            values,
        });
        Ok(())
    }

    fn ensure_supported_version(&self) -> Result<(), SymbolExportError> {
        if self.version == SYMBOL_TRACE_FORMAT_VERSION {
            Ok(())
        } else {
            Err(SymbolExportError::UnsupportedTraceVersion {
                found: self.version,
            })
        }
    }

    /// Computes the lowercase SHA-256 of the canonical binary trace encoding.
    ///
    /// The fixed encoding is, in order: domain separator
    /// `terranigma.symbol-trace\0`; format version; normalized ROM SHA-256;
    /// memory-map schema version and ROM SHA-256; length-prefixed scenario ID;
    /// symbol count followed by each length-prefixed ID, canonical address,
    /// and width; checkpoint count followed by each scenario frame label,
    /// actual frame counter, value count, and length-prefixed raw value. All
    /// versions, counts, lengths, addresses, widths, scenario labels, and frame
    /// counters are unsigned 32-bit little-endian integers. SHA-256 fields are
    /// their raw 32 bytes; strings are their exact UTF-8 bytes prefixed by the
    /// byte length, with no text normalization. Digests never depend on serde
    /// or JSON representation.
    ///
    /// # Errors
    ///
    /// Returns an error if the trace version is unsupported or caller-mutated
    /// public vectors or strings exceed a length representable by the format's
    /// `u32` fields.
    pub fn digest_hex(&self) -> Result<String, SymbolExportError> {
        self.ensure_supported_version()?;
        let mut digest = Sha256::new();
        digest.update(b"terranigma.symbol-trace\0");
        update_u32(&mut digest, self.version);
        digest.update(self.metadata.normalized_rom_sha256);
        update_u32(&mut digest, self.metadata.memory_map_schema_version);
        digest.update(self.metadata.memory_map_rom_sha256);
        update_bytes(
            &mut digest,
            "scenario ID",
            self.metadata.scenario_id.as_bytes(),
        )?;

        update_len(&mut digest, "symbol count", self.symbols.len())?;
        for symbol in &self.symbols {
            update_bytes(&mut digest, "symbol ID", symbol.id.as_bytes())?;
            update_u32(&mut digest, symbol.address);
            update_u32(&mut digest, symbol.width);
        }

        update_len(&mut digest, "checkpoint count", self.checkpoints.len())?;
        for checkpoint in &self.checkpoints {
            update_u32(&mut digest, checkpoint.scenario_label);
            update_u32(&mut digest, checkpoint.actual_frame);
            update_len(
                &mut digest,
                "checkpoint value count",
                checkpoint.values.len(),
            )?;
            for value in &checkpoint.values {
                update_bytes(&mut digest, "symbol value", value)?;
            }
        }

        Ok(bytes_to_hex(&digest.finalize()))
    }
}

fn checked_u32_len(field: &'static str, len: usize) -> Result<u32, SymbolExportError> {
    u32::try_from(len).map_err(|_| SymbolExportError::LengthOverflow { field, len })
}

fn update_u32(digest: &mut Sha256, value: u32) {
    digest.update(value.to_le_bytes());
}

fn update_len(
    digest: &mut Sha256,
    field: &'static str,
    len: usize,
) -> Result<(), SymbolExportError> {
    update_u32(digest, checked_u32_len(field, len)?);
    Ok(())
}

fn update_bytes(
    digest: &mut Sha256,
    field: &'static str,
    bytes: &[u8],
) -> Result<(), SymbolExportError> {
    update_len(digest, field, bytes.len())?;
    digest.update(bytes);
    Ok(())
}

fn decode_sha256(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 {
        return None;
    }
    let mut digest = [0; 32];
    for (output, pair) in digest.iter_mut().zip(hex.as_bytes().chunks_exact(2)) {
        let [high, low] = pair else {
            return None;
        };
        *output = decode_hex_nibble(*high)? << 4 | decode_hex_nibble(*low)?;
    }
    Some(digest)
}

fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn read_trace_symbol<'a>(
    symbol: &TraceSymbol,
    wram: &'a [u8],
) -> Result<&'a [u8], memory_map::ReadError> {
    let Some(last_offset) = symbol.width.checked_sub(1) else {
        return Err(memory_map::ReadError::InvalidSymbolRegion);
    };
    let Some(end) = symbol.address.checked_add(last_offset) else {
        return Err(memory_map::ReadError::InvalidSymbolRegion);
    };
    let offset = if symbol.address <= 0xFF && end <= 0xFF {
        usize::try_from(symbol.address).map_err(|_| memory_map::ReadError::InvalidSymbolRegion)?
    } else if (0x7E_0000..=0x7F_FFFF).contains(&symbol.address) && end <= 0x7F_FFFF {
        usize::try_from(symbol.address - 0x7E_0000)
            .map_err(|_| memory_map::ReadError::InvalidSymbolRegion)?
    } else {
        return Err(memory_map::ReadError::InvalidSymbolRegion);
    };
    let width =
        usize::try_from(symbol.width).map_err(|_| memory_map::ReadError::InvalidSymbolRegion)?;
    let needed = offset
        .checked_add(width)
        .ok_or(memory_map::ReadError::InvalidSymbolRegion)?;
    wram.get(offset..needed)
        .ok_or(memory_map::ReadError::WramTooShort {
            needed,
            actual: wram.len(),
        })
}

/// One frame's export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameExport {
    /// Export format version; must equal [`EXPORT_VERSION`].
    pub version: u32,
    /// Frame index within the replay.
    pub frame: u32,
    /// Region digests.
    pub regions: Vec<RegionDigest>,
    /// Named semantic fields.
    pub fields: Vec<SemanticField>,
    /// Excluded byte ranges, by region name, with justification.
    pub exclusions: Vec<Exclusion>,
}

/// A documented exclusion of unstable bytes from comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exclusion {
    /// Region the exclusion applies to.
    pub region: String,
    /// First excluded offset.
    pub start: u32,
    /// One-past-last excluded offset.
    pub end: u32,
    /// Why these bytes are excluded.
    pub reason: String,
}

/// Lowercase hex encoding.
pub(crate) fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0F) as usize] as char);
    }
    s
}

/// Computes a hex SHA-256 over `bytes`.
#[must_use]
pub fn digest_hex(bytes: &[u8]) -> String {
    let d = Sha256::digest(bytes);
    bytes_to_hex(&d)
}

/// Difference kinds found by [`compare_exports`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Difference {
    /// A region digest differs.
    Region {
        /// Region name.
        name: String,
        /// Digest on the left (reference) side.
        left: String,
        /// Digest on the right (candidate) side.
        right: String,
    },
    /// A semantic field differs.
    Field {
        /// Field name.
        name: String,
        /// Value on the left side, hex.
        left: String,
        /// Value on the right side, hex.
        right: String,
    },
    /// A field or region exists on only one side.
    Missing {
        /// Item name.
        name: String,
        /// Which side is missing it: `"left"` or `"right"`.
        side: String,
    },
}

/// The first divergence between two export sequences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DivergenceReport {
    /// Export format version the comparison ran under.
    pub version: u32,
    /// First frame index whose export differs.
    pub frame: u32,
    /// Differences at that frame, in report order.
    pub differences: Vec<Difference>,
}

/// Compares two export sequences and returns the first divergent frame.
///
/// Sequences of different lengths compare only up to the shorter one's
/// length; a length mismatch with equal prefixes is reported at the first
/// frame only one side has (as [`Difference::Missing`]).
#[must_use]
pub fn compare_exports(left: &[FrameExport], right: &[FrameExport]) -> Option<DivergenceReport> {
    let n = left.len().min(right.len());
    for idx in 0..n {
        let (l, r) = (&left[idx], &right[idx]);
        if l.version != r.version || l.version != EXPORT_VERSION {
            // Version disagreement is a hard error surfaced as a difference
            // on the metadata itself.
            return Some(DivergenceReport {
                version: EXPORT_VERSION,
                frame: l.frame,
                differences: vec![Difference::Field {
                    name: "version".into(),
                    left: l.version.to_string(),
                    right: r.version.to_string(),
                }],
            });
        }
        let mut diffs = Vec::new();
        for lr in &l.regions {
            match r.regions.iter().find(|x| x.name == lr.name) {
                Some(rr) if rr.sha256 != lr.sha256 => diffs.push(Difference::Region {
                    name: lr.name.clone(),
                    left: lr.sha256.clone(),
                    right: rr.sha256.clone(),
                }),
                Some(_) => {}
                None => diffs.push(Difference::Missing {
                    name: lr.name.clone(),
                    side: "right".into(),
                }),
            }
        }
        for rr in &r.regions {
            if !l.regions.iter().any(|x| x.name == rr.name) {
                diffs.push(Difference::Missing {
                    name: rr.name.clone(),
                    side: "left".into(),
                });
            }
        }
        for lf in &l.fields {
            match r.fields.iter().find(|x| x.name == lf.name) {
                Some(rf) if rf.value != lf.value => diffs.push(Difference::Field {
                    name: lf.name.clone(),
                    left: bytes_to_hex(&lf.value),
                    right: bytes_to_hex(&rf.value),
                }),
                Some(_) => {}
                None => diffs.push(Difference::Missing {
                    name: lf.name.clone(),
                    side: "right".into(),
                }),
            }
        }
        for rf in &r.fields {
            if !l.fields.iter().any(|x| x.name == rf.name) {
                diffs.push(Difference::Missing {
                    name: rf.name.clone(),
                    side: "left".into(),
                });
            }
        }
        if !diffs.is_empty() {
            return Some(DivergenceReport {
                version: EXPORT_VERSION,
                frame: l.frame,
                differences: diffs,
            });
        }
    }
    if left.len() != right.len() {
        // Report the first frame only the longer side has, so the frame
        // number points at the actual missing content.
        let (longer, side) = if left.len() > right.len() {
            (left, "right")
        } else {
            (right, "left")
        };
        if let Some(first_missing) = longer.get(left.len().min(right.len())) {
            return Some(DivergenceReport {
                version: EXPORT_VERSION,
                frame: first_missing.frame,
                differences: vec![Difference::Missing {
                    name: "<subsequent frames>".into(),
                    side: side.into(),
                }],
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn export(frame: u32, wram: &str, map: u8) -> FrameExport {
        FrameExport {
            version: EXPORT_VERSION,
            frame,
            regions: vec![RegionDigest {
                name: "wram".into(),
                sha256: wram.into(),
            }],
            fields: vec![SemanticField {
                name: "current_map".into(),
                value: vec![map],
            }],
            exclusions: vec![],
        }
    }

    #[test]
    fn canonical_symbols_export_from_wram_in_requested_order() {
        let map = memory_map::MemoryMap::built_in_japan().unwrap();
        let mut wram = vec![0; 0x20_000];
        wram[0x047E..0x0480].copy_from_slice(&0x0129_u16.to_le_bytes());
        wram[0x1000..0x1002].copy_from_slice(&304_u16.to_le_bytes());

        assert_eq!(
            semantic_fields_from_wram(&map, &wram, &["player_x", "current_map"]).unwrap(),
            vec![
                SemanticField {
                    name: "player_x".into(),
                    value: vec![0x30, 0x01],
                },
                SemanticField {
                    name: "current_map".into(),
                    value: vec![0x29, 0x01],
                },
            ]
        );
        assert!(matches!(
            semantic_fields_from_wram(&map, &wram, &["missing"]),
            Err(SymbolExportError::UnknownSymbol(id)) if id == "missing"
        ));
    }

    #[test]
    fn symbol_trace_rejects_rom_identity_mismatch() {
        let map = memory_map::MemoryMap::built_in_japan().unwrap();
        let error = SymbolTrace::new(&map, [0xFF; 32], "opening", &["current_map"])
            .expect_err("runtime and map ROM identities differ");

        assert!(matches!(
            error,
            SymbolExportError::RomIdentityMismatch {
                runtime_rom_sha256,
                ..
            } if runtime_rom_sha256 == [0xFF; 32]
        ));
    }

    #[test]
    fn symbol_trace_rejects_unknown_and_unsupported_symbols() {
        let map = memory_map::MemoryMap::built_in_japan().unwrap();
        let rom_sha256 = rom::Revision::Japan.sha256();

        assert!(matches!(
            SymbolTrace::new(&map, rom_sha256, "opening", &["missing"]),
            Err(SymbolExportError::UnknownSymbol(id)) if id == "missing"
        ));
        assert!(matches!(
            SymbolTrace::new(&map, rom_sha256, "opening", &["cgram_address"]),
            Err(SymbolExportError::UnsupportedSymbol { id, .. }) if id == "cgram_address"
        ));
    }

    #[test]
    fn symbol_trace_extracts_ordered_values_and_distinguishes_frames() {
        let map = memory_map::MemoryMap::built_in_japan().unwrap();
        let mut trace = SymbolTrace::new(
            &map,
            rom::Revision::Japan.sha256(),
            "opening",
            &["player_x", "current_map"],
        )
        .unwrap();
        let mut wram = vec![0; 0x20_000];
        wram[0x047E..0x0480].copy_from_slice(&0x0129_u16.to_le_bytes());
        wram[0x1000..0x1002].copy_from_slice(&304_u16.to_le_bytes());

        trace.append_checkpoint(1_537, 1_499, &wram).unwrap();

        assert_eq!(
            trace.symbols,
            [
                TraceSymbol {
                    id: "player_x".into(),
                    address: 0x7E_1000,
                    width: 2,
                },
                TraceSymbol {
                    id: "current_map".into(),
                    address: 0x7E_047E,
                    width: 2,
                },
            ]
        );
        assert_eq!(trace.checkpoints[0].scenario_label, 1_499);
        assert_eq!(trace.checkpoints[0].actual_frame, 1_537);
        assert_eq!(
            trace.checkpoints[0].values,
            [vec![0x30, 0x01], vec![0x29, 0x01]]
        );
    }

    #[test]
    fn symbol_trace_reports_short_wram() {
        let map = memory_map::MemoryMap::built_in_japan().unwrap();
        let mut trace = SymbolTrace::new(
            &map,
            rom::Revision::Japan.sha256(),
            "opening",
            &["player_x"],
        )
        .unwrap();

        assert!(matches!(
            trace.append_checkpoint(1, 0, &[0; 16]),
            Err(SymbolExportError::Read {
                id,
                source: memory_map::ReadError::WramTooShort { .. },
            }) if id == "player_x"
        ));
        assert!(trace.checkpoints.is_empty());
    }

    fn digest_fixture() -> SymbolTrace {
        SymbolTrace {
            version: SYMBOL_TRACE_FORMAT_VERSION,
            metadata: SymbolTraceMetadata {
                normalized_rom_sha256: [0x11; 32],
                memory_map_schema_version: 7,
                memory_map_rom_sha256: [0x22; 32],
                scenario_id: "opening".into(),
            },
            symbols: vec![TraceSymbol {
                id: "map".into(),
                address: 0x7E_047E,
                width: 2,
            }],
            checkpoints: vec![SymbolCheckpoint {
                scenario_label: 1_499,
                actual_frame: 1_523,
                values: vec![vec![4, 0]],
            }],
        }
    }

    #[test]
    fn symbol_trace_rejects_an_unsupported_format_version() {
        let mut trace = digest_fixture();
        trace.version += 1;

        assert!(matches!(
            trace.digest_hex(),
            Err(SymbolExportError::UnsupportedTraceVersion { found: 2 })
        ));
    }

    #[test]
    fn symbol_trace_digest_has_exact_canonical_fixture() {
        assert_eq!(
            digest_fixture().digest_hex().unwrap(),
            "e15defce65c53947bfdba83573bc8b24ac17352fc67c6b5ffb066691abd0a449"
        );
    }

    #[test]
    fn symbol_trace_digest_covers_definitions_values_and_metadata() {
        let original = digest_fixture();
        let original_digest = original.digest_hex().unwrap();
        let mut variants = Vec::new();

        let mut changed = original.clone();
        changed.symbols[0].address += 1;
        variants.push(changed);
        let mut changed = original.clone();
        changed.symbols[0].width += 1;
        variants.push(changed);
        let mut changed = original.clone();
        changed.checkpoints[0].values[0][0] ^= 1;
        variants.push(changed);
        let mut changed = original.clone();
        changed.metadata.scenario_id.push_str("-changed");
        variants.push(changed);
        let mut changed = original.clone();
        changed.metadata.normalized_rom_sha256[0] ^= 1;
        variants.push(changed);
        let mut changed = original.clone();
        changed.metadata.memory_map_schema_version += 1;
        variants.push(changed);
        let mut changed = original.clone();
        changed.metadata.memory_map_rom_sha256[0] ^= 1;
        variants.push(changed);
        let mut changed = original.clone();
        changed.symbols[0].id.push_str("-changed");
        variants.push(changed);
        let mut changed = original.clone();
        changed.checkpoints[0].scenario_label += 1;
        variants.push(changed);
        let mut changed = original.clone();
        changed.checkpoints[0].actual_frame += 1;
        variants.push(changed);

        for changed in variants {
            assert_ne!(changed.digest_hex().unwrap(), original_digest);
        }
    }

    #[test]
    fn identical_sequences_report_no_divergence() {
        let a = vec![export(0, "aa", 1), export(1, "bb", 2)];
        let b = a.clone();
        assert_eq!(compare_exports(&a, &b), None);
    }

    #[test]
    fn reports_first_divergent_frame_and_field() {
        let a = vec![export(0, "aa", 1), export(1, "bb", 2), export(2, "cc", 3)];
        let mut b = a.clone();
        b[1].fields[0].value = vec![9];
        b[2].regions[0].sha256 = "zz".into();
        let report = compare_exports(&a, &b).expect("divergence found");
        assert_eq!(report.frame, 1);
        assert_eq!(report.differences.len(), 1);
        assert_eq!(
            report.differences[0],
            Difference::Field {
                name: "current_map".into(),
                left: "02".into(),
                right: "09".into(),
            }
        );
    }

    #[test]
    fn reports_missing_regions_and_length_mismatch() {
        let a = vec![export(0, "aa", 1)];
        let mut b = a.clone();
        b[0].regions.clear();
        let report = compare_exports(&a, &b).expect("divergence");
        assert!(report
            .differences
            .iter()
            .any(|d| matches!(d, Difference::Missing { side, .. } if side == "right")));

        let longer = vec![export(0, "aa", 1), export(1, "bb", 2)];
        let report = compare_exports(&a, &longer).expect("length divergence");
        assert_eq!(report.frame, 1);
    }

    #[test]
    fn json_round_trip() {
        let e = export(7, "ff", 0x22);
        let s = serde_json::to_string(&e).expect("ser");
        let back: FrameExport = serde_json::from_str(&s).expect("de");
        assert_eq!(e, back);
    }
}
