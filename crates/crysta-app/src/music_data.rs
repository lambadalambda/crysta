//! Source-only Japanese music and sound effect extraction; no captured
//! machine state.
//!
//! Selection 3 (track 4) is qualified byte for byte; other tracks share its
//! framing through the track table. Transfer protocol and evidence are
//! documented in `tools/native-music-qualification/README.md`.

use std::ops::Range;

use rom::{Revision, Rom, RuntimeRomAddress};

const MAX_BLOCKS: usize = 16;
const BOOTSTRAP: usize = 0x06_ac42;
const SAMPLE_POOL: usize = 0x38_8000;
/// The highest sample ID a selection may name; the pool is scanned to it.
const LAST_SAMPLE: u8 = 0x7f;

/// One ROM-derived payload to transfer through APU ports, never by RAM injection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transfer {
    /// Destination in SPC address space.
    pub destination: u16,
    /// Ordered normalized ROM ranges; a sample can cross a bank's skipped half.
    pub source_ranges: Vec<Range<usize>>,
    /// Concatenation of `source_ranges`, without headers or synthesized state.
    pub data: Vec<u8>,
}

/// An IPL-compatible transfer session, including its zero-length terminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferGroup {
    /// Blocks in original host order.
    pub blocks: Vec<Transfer>,
    /// Bootstrap execution entry; later groups merely pass this to the receiver.
    /// The resident driver returns to its loop rather than executing this address.
    pub terminal_destination: u16,
}

/// Source-derived uploads for one track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upload {
    /// Sequence, instruments, and directory metadata.
    pub sequence: TransferGroup,
    /// Sample payloads in the tail's order.
    pub samples: TransferGroup,
}

/// The driver and what it keeps loaded under every track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Driver {
    /// Initial physical-IPL upload; terminal destination is the driver entry.
    pub bootstrap: TransferGroup,
    /// The sound effect bank (`$C6:2191`): effect sequences and samples 0-B
    /// below `$76AA`, where every track's sequence starts.
    pub sounds: Upload,
}

/// One entry of the track table `$96:F2A0`. A map's `08 FC` selection `n`
/// plays track `n + 1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// The table index, as `COP 30` names it.
    pub index: u8,
    /// Parameter for the host's `F0` and `F1` commands before this track.
    pub stop_parameter: u8,
    /// The track's uploads.
    pub upload: Upload,
}

/// Extraction fails closed on revision, framing, source, or destination errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MusicDataError {
    /// Requires the built-in authenticated Japanese revision, not a caller label.
    UnsupportedRom,
    /// A source range is truncated or its arithmetic overflowed.
    SourceBounds {
        /// Normalized ROM offset requested.
        offset: usize,
        /// Number of bytes requested.
        length: usize,
    },
    /// The requested data falls outside the qualified framing.
    Invalid(&'static str),
}

impl std::fmt::Display for MusicDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedRom => f.write_str("music requires the authenticated Japanese ROM"),
            Self::SourceBounds { offset, length } => write!(
                f,
                "music ROM range {offset:#x}+{length:#x} is out of bounds"
            ),
            Self::Invalid(reason) => write!(f, "unsupported music framing: {reason}"),
        }
    }
}
impl std::error::Error for MusicDataError {}

type Result<T> = std::result::Result<T, MusicDataError>;

/// The track table `$96:F2A0`: a long pointer, then the parameter's low
/// nibble (`$86:AC26`).
const TRACKS: usize = 0x16_f2a0;
/// The last entry that points at a track; later ones are other data.
const LAST_TRACK: u8 = 0x3b;
/// The sound effect bank the host uploads before any track.
const SOUND_BANK: usize = 0x06_2191;

fn authenticated(rom: &Rom) -> Result<&[u8]> {
    let digest = rom.digests();
    if rom.revision() != Revision::Japan
        || digest.sha256 != Revision::Japan.sha256()
        || digest.crc32 != Revision::Japan.crc32()
    {
        return Err(MusicDataError::UnsupportedRom);
    }
    Ok(rom.image())
}

/// Extract the driver and its sound effect bank.
///
/// # Errors
/// Rejects unauthenticated/non-Japanese images and invalid bounded framing.
pub fn extract_driver(rom: &Rom) -> Result<Driver> {
    let image = authenticated(rom)?;
    let (bootstrap, _) = inline_group(image, BOOTSTRAP)?;
    Ok(Driver {
        bootstrap,
        sounds: upload_at(image, SOUND_BANK)?,
    })
}

/// Extract a track of the table.
///
/// # Errors
/// As [`extract_driver`], and rejects an index the table does not hold.
pub fn extract_track(rom: &Rom, track: u8) -> Result<Track> {
    let image = authenticated(rom)?;
    let (source, stop_parameter) = track_source(image, track)?;
    Ok(Track {
        index: track,
        stop_parameter,
        upload: upload_at(image, source)?,
    })
}

fn track_source(image: &[u8], track: u8) -> Result<(usize, u8)> {
    if !(1..=LAST_TRACK).contains(&track) {
        return Err(MusicDataError::Invalid("track outside the table"));
    }
    let entry = bytes(image, TRACKS + usize::from(track) * 4, 4)?;
    let pointer = RuntimeRomAddress::new(u32::from_le_bytes([entry[0], entry[1], entry[2], 0]))
        .map_err(|_| MusicDataError::Invalid("track pointer"))?;
    Ok((pointer.normalized().value() as usize, entry[3] & 15))
}

/// An inline group, then the sample IDs after its terminator
/// (`$86:AAA9`).
fn upload_at(image: &[u8], source: usize) -> Result<Upload> {
    let (sequence, tail) = inline_group(image, source)?;
    let count = usize::from(bytes(image, tail, 1)?[0]);
    let last = bytes(image, tail + 1, count)?
        .iter()
        .copied()
        .max()
        .unwrap_or(0);
    let pool = sample_pool(image, SAMPLE_POOL, last)?;
    let samples = sample_group(image, tail, sequence.terminal_destination, &pool)?;
    Ok(Upload { sequence, samples })
}

fn bytes(image: &[u8], offset: usize, length: usize) -> Result<&[u8]> {
    let fail = || MusicDataError::SourceBounds { offset, length };
    let end = offset.checked_add(length).ok_or_else(fail)?;
    image.get(offset..end).ok_or_else(fail)
}

fn word(image: &[u8], offset: usize) -> Result<u16> {
    let b = bytes(image, offset, 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn check_destination(destination: u16, length: usize) -> Result<()> {
    if length == 0 || usize::from(destination) + length > 0x10000 {
        return Err(MusicDataError::Invalid(
            "empty payload or SPC destination wrap",
        ));
    }
    Ok(())
}

fn inline_group(image: &[u8], mut cursor: usize) -> Result<(TransferGroup, usize)> {
    let mut blocks = Vec::new();
    let mut total = 0;
    loop {
        bytes(image, cursor, 4)?;
        let length = usize::from(word(image, cursor)?);
        let destination = word(image, cursor + 2)?;
        cursor += 4;
        if length == 0 {
            return Ok((
                TransferGroup {
                    blocks,
                    terminal_destination: destination,
                },
                cursor,
            ));
        }
        check_destination(destination, length)?;
        total += length;
        if blocks.len() == MAX_BLOCKS || total > 0x10000 {
            return Err(MusicDataError::Invalid("inline group budget"));
        }
        let data = bytes(image, cursor, length)?.to_vec();
        blocks.push(Transfer {
            destination,
            source_ranges: std::iter::once(cursor..cursor + length).collect(),
            data,
        });
        cursor += length;
    }
}

// $86:AAEA restarts this length-prefixed pool scan for each sample ID. Reading
// it once is equivalent; both skipping and copying advance FFFF -> next:8000.
fn sample_pool(image: &[u8], mut cursor: usize, last: u8) -> Result<Vec<Transfer>> {
    if last > LAST_SAMPLE {
        return Err(MusicDataError::Invalid(
            "sample index exceeds qualified pool",
        ));
    }
    let mut pool = Vec::new();
    for _ in 0..=last {
        let length = usize::from(word(image, cursor)?);
        if length == 0 {
            return Err(MusicDataError::Invalid("zero sample length"));
        }
        cursor += 2;
        let mut data = Vec::with_capacity(length);
        let mut source_ranges = Vec::new();
        while data.len() < length {
            if cursor.is_multiple_of(0x10000) {
                cursor += 0x8000;
            }
            let size = (length - data.len()).min(0x10000 - (cursor & 0xffff));
            data.extend_from_slice(bytes(image, cursor, size)?);
            source_ranges.push(cursor..cursor + size);
            cursor += size;
        }
        if cursor.is_multiple_of(0x10000) {
            cursor += 0x8000;
        }
        pool.push(Transfer {
            destination: 0,
            source_ranges,
            data,
        });
    }
    Ok(pool)
}

fn sample_group(image: &[u8], tail: usize, base: u16, pool: &[Transfer]) -> Result<TransferGroup> {
    let count = usize::from(bytes(image, tail, 1)?[0]);
    if count == 0 || count > MAX_BLOCKS {
        return Err(MusicDataError::Invalid("sample count"));
    }
    let mut destination = usize::from(base);
    let mut blocks = Vec::new();
    for &id in bytes(image, tail + 1, count)? {
        let mut block = pool
            .get(usize::from(id))
            .ok_or(MusicDataError::Invalid("unqualified sample ID"))?
            .clone();
        block.destination = u16::try_from(destination)
            .map_err(|_| MusicDataError::Invalid("sample destination overflow"))?;
        check_destination(block.destination, block.data.len())?;
        destination += block.data.len();
        blocks.push(block);
    }
    let terminal_destination = blocks.last().expect("nonzero count").destination;
    Ok(TransferGroup {
        blocks,
        terminal_destination,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_blocks_keep_destination_source_and_terminator() {
        let image = [2, 0, 0, 3, 11, 22, 0, 0, 0, 3];
        let (group, end) = inline_group(&image, 0).unwrap();
        assert_eq!(end, image.len());
        assert_eq!(group.terminal_destination, 0x300);
        assert_eq!(group.blocks.len(), 1);
        assert_eq!(group.blocks[0].destination, 0x300);
        assert_eq!(group.blocks[0].data, [11, 22]);
        assert_eq!(group.blocks[0].source_ranges.len(), 1);
        assert_eq!(group.blocks[0].source_ranges[0], 4..6);
    }

    #[test]
    fn inline_rejects_truncation_missing_terminator_and_apu_wrap() {
        for image in [
            vec![],
            vec![0, 0, 0],
            vec![2, 0, 0, 3, 11],
            vec![1, 0, 0, 3, 11],
            vec![2, 0, 255, 255, 11, 22, 0, 0, 0, 3],
        ] {
            assert!(inline_group(&image, 0).is_err(), "{image:?}");
        }
        assert!(inline_group(&[0; 4], usize::MAX).is_err());
    }

    #[test]
    fn inline_block_count_is_bounded() {
        let mut image = [1, 0, 0, 3, 11].repeat(MAX_BLOCKS + 1);
        image.extend([0, 0, 0, 3]);
        assert!(inline_group(&image, 0).is_err());
    }

    #[test]
    fn pool_payload_skips_lower_half_bank_and_preserves_provenance() {
        let mut image = vec![0; 0x18002];
        image[0xfffc..0x10000].copy_from_slice(&[4, 0, 11, 22]);
        image[0x18000..].copy_from_slice(&[33, 44]);
        let blocks = sample_pool(&image, 0xfffc, 0).unwrap();
        assert_eq!(blocks[0].data, [11, 22, 33, 44]);
        assert_eq!(blocks[0].source_ranges, [0xfffe..0x10000, 0x18000..0x18002]);
        assert!(sample_pool(&image[..0x18001], 0xfffc, 0).is_err());
        assert!(sample_pool(&[0; 4], 0, 0).is_err());
    }

    #[test]
    fn a_track_entry_is_a_long_pointer_and_the_parameters_low_nibble() {
        let mut image = vec![0; TRACKS + 0x100];
        image[TRACKS + 4 * 4..TRACKS + 4 * 5].copy_from_slice(&[0x82, 0x8c, 0xaa, 0x27]);
        assert_eq!(track_source(&image, 4), Ok((0x2a_8c82, 7)));
        for outside in [0, LAST_TRACK + 1] {
            assert!(track_source(&image, outside).is_err());
        }
    }

    #[test]
    fn pool_next_header_skips_lower_half_when_payload_ends_at_bank_boundary() {
        let mut image = vec![0; 0x18004];
        image[0xfffc..0x10000].copy_from_slice(&[2, 0, 11, 22]);
        image[0x18000..].copy_from_slice(&[2, 0, 33, 44]);
        let blocks = sample_pool(&image, 0xfffc, 1).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].data, [11, 22]);
        assert_eq!(blocks[1].data, [33, 44]);
        for (block, expected) in blocks.iter().zip([0xfffe..0x10000, 0x18002..0x18004]) {
            assert_eq!(block.source_ranges.len(), 1);
            assert_eq!(block.source_ranges[0], expected);
        }
    }

    #[test]
    fn sample_tail_preserves_order_and_rejects_unqualified_ids_and_overflow() {
        let pool = vec![Transfer {
            destination: 0,
            data: vec![11, 22],
            source_ranges: std::iter::once(2..4).collect(),
        }];
        let group = sample_group(&[2, 0, 0], 0, 0x1000, &pool).unwrap();
        assert_eq!(
            group
                .blocks
                .iter()
                .map(|b| b.destination)
                .collect::<Vec<_>>(),
            [0x1000, 0x1002]
        );
        assert_eq!(group.terminal_destination, 0x1002);
        for tail in [&[0][..], &[17, 0], &[1], &[1, 0x80], &[1, 1]] {
            assert!(sample_group(tail, 0, 0x1000, &pool).is_err());
        }
        assert!(sample_group(&[1, 0], 0, 0xffff, &pool).is_err());
    }

    #[test]
    fn caller_assigned_japan_revision_is_not_authentication() {
        let bytes = vec![0; Rom::IMAGE_SIZE];
        let digest = rom::digests(&bytes);
        let fake = Rom::load_with_known(
            &bytes,
            &[rom::KnownRom {
                revision: Revision::Japan,
                sha256: digest.sha256,
                crc32: digest.crc32,
            }],
        )
        .unwrap();
        assert!(extract_driver(&fake).is_err());
        assert!(extract_track(&fake, 4).is_err());
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn the_slices_tracks_and_the_sound_bank_extract() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").expect("CRYSTA_JP_ROM")).unwrap();
        let rom = Rom::load(&bytes).unwrap();
        // Map selections 0, 1, 3, 5, $1B, and the scenes' $31 and $34.
        for track in [1, 2, 4, 6, 0x1c, 0x31, 0x34] {
            let music = extract_track(&rom, track).unwrap();
            assert!(!music.upload.samples.blocks.is_empty(), "{track:#x}");
            assert!(music
                .upload
                .sequence
                .blocks
                .iter()
                .any(|block| block.destination == 0x76aa));
        }
        let sounds = extract_driver(&rom).unwrap().sounds;
        let destinations = |group: &TransferGroup| -> Vec<(u16, usize)> {
            group
                .blocks
                .iter()
                .map(|b| (b.destination, b.data.len()))
                .collect()
        };
        assert_eq!(
            destinations(&sounds.sequence),
            [
                (0x1000, 0x48),
                (0xfe8, 0x18),
                (0x1100, 0xa30),
                (0xf00, 0x30)
            ]
        );
        let samples = &sounds.samples.blocks;
        assert_eq!((samples.len(), samples[0].destination), (12, 0x1b30));
        let last = samples.last().unwrap();
        assert_eq!(
            usize::from(last.destination) + last.data.len(),
            0x76aa,
            "the bank ends where tracks start"
        );
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn authenticated_local_rom_has_qualified_groups() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").expect("CRYSTA_JP_ROM")).unwrap();
        let rom = Rom::load(&bytes).unwrap();
        // Selection 3, the bedroom's, is track 4.
        let music = extract_track(&rom, 4).unwrap();
        let bootstrap = extract_driver(&rom).unwrap().bootstrap;
        assert_eq!(music.stop_parameter, 7);
        assert_eq!(bootstrap.terminal_destination, 0x300);
        assert_eq!(music.upload.sequence.terminal_destination, 0x821c);
        assert_eq!(music.upload.samples.terminal_destination, 0xbdaa);
        let one = |range| std::iter::once(range).collect::<Vec<_>>();
        for (group, expected, expected_ranges) in [
            (
                &bootstrap,
                vec![(0x1e0, 0x20), (0x300, 0xbf5), (0xff16, 0x6f)],
                vec![
                    one(0x06_ac46..0x06_ac66),
                    one(0x06_ac6a..0x06_b85f),
                    one(0x06_b863..0x06_b8d2),
                ],
            ),
            (
                &music.upload.sequence,
                vec![
                    (0x1048, 0x30),
                    (0xfe8, 0x18),
                    (0x76aa, 0xb72),
                    (0xf30, 0x20),
                    (0x10fc, 2),
                    (0x10fe, 2),
                    (0xffab, 8),
                ],
                vec![
                    one(0x2a_8c86..0x2a_8cb6),
                    one(0x2a_8cba..0x2a_8cd2),
                    one(0x2a_8cd6..0x2a_9848),
                    one(0x2a_984c..0x2a_986c),
                    one(0x2a_9870..0x2a_9872),
                    one(0x2a_9876..0x2a_9878),
                    one(0x2a_987c..0x2a_9884),
                ],
            ),
            (
                &music.upload.samples,
                vec![
                    (0x821c, 0x32a),
                    (0x8546, 0x27f),
                    (0x87c5, 0xfa5),
                    (0x976a, 0x36),
                    (0x97a0, 0x20bb),
                    (0xb85b, 0x51),
                    (0xb8ac, 0x4fe),
                    (0xbdaa, 0x3d5),
                ],
                vec![
                    one(0x39_a5fd..0x39_a927),
                    one(0x39_cf4f..0x39_d1ce),
                    one(0x39_d1d0..0x39_e175),
                    one(0x39_e177..0x39_e1ad),
                    // Sample $0E continues at the next bank's upper half.
                    vec![0x38_ef5a..0x39_0000, 0x39_8000..0x39_9015],
                    one(0x39_e1af..0x39_e200),
                    one(0x39_e202..0x39_e700),
                    one(0x39_e702..0x39_ead7),
                ],
            ),
        ] {
            assert_eq!(
                group
                    .blocks
                    .iter()
                    .map(|b| (b.destination, b.data.len()))
                    .collect::<Vec<_>>(),
                expected
            );
            for (block, ranges) in group.blocks.iter().zip(expected_ranges) {
                assert_eq!(block.source_ranges, ranges);
                let source: Vec<_> = block
                    .source_ranges
                    .iter()
                    .flat_map(|r| &rom.image()[r.clone()])
                    .copied()
                    .collect();
                assert_eq!(block.data, source);
            }
        }
    }
}
