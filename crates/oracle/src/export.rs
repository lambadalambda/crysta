//! Versioned frame-state export and comparison for divergence diagnosis.
//!
//! Exports are digests over selected regions plus named semantic fields;
//! they are commit-safe (no raw memory) unless a caller deliberately stores
//! raw captures under `local/`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Export format version.
pub const EXPORT_VERSION: u32 = 1;

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
