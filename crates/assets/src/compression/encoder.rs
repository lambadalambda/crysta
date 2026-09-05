//! Greedy packet encoder with the original zero-padded match selection rule.

use super::MAX_OUTPUT_SIZE;
use std::fmt;

const WINDOW: usize = 8192;
const MAX_MATCH: usize = 256;

/// Input that cannot be represented by a qualified compressed packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// A packet always includes a first output byte; empty inputs are invalid.
    EmptyInput,
    /// The nonzero 16-bit header cannot represent this input length.
    InputTooLarge {
        /// Caller-provided length in bytes.
        length: usize,
    },
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => f.write_str("cannot encode an empty compressed packet"),
            Self::InputTooLarge { length } => write!(
                f,
                "input length {length} exceeds packet maximum {MAX_OUTPUT_SIZE}"
            ),
        }
    }
}

impl std::error::Error for EncodeError {}

/// Encodes a nonempty input using the documented original greedy algorithm.
///
/// Searches the preceding 8 KiB for the longest match (up to 256 bytes), breaks
/// ties by nearest distance, and considers virtual zero padding before clamping
/// a final match to the real input. Copies may overlap. Emits zero unused control
/// bits and a zero-offset terminator. This is deterministic, not an optimal-size
/// compressor or a promise to preserve every possible source tokenization.
///
/// # Errors
///
/// Rejects empty inputs and inputs longer than [`MAX_OUTPUT_SIZE`].
pub fn encode(input: &[u8]) -> Result<Vec<u8>, EncodeError> {
    if input.is_empty() {
        return Err(EncodeError::EmptyInput);
    }
    let size = u16::try_from(input.len()).map_err(|_| EncodeError::InputTooLarge {
        length: input.len(),
    })?;
    let mut writer = Writer::new(size, input[0]);
    let previous = previous_equal_bytes(input);
    let mut position = 1;
    while position < input.len() {
        let (distance, length) = best_match(input, &previous, position);
        if length >= 3 || (length == 2 && distance <= 256) {
            writer.copy(distance, length);
            position += length;
        } else {
            writer.bit(true);
            writer.bytes.push(input[position]);
            position += 1;
        }
    }
    writer.bit(false);
    writer.bit(true);
    writer.bytes.extend_from_slice(&[0, 0, 0]);
    Ok(writer.bytes)
}

// A predecessor chain skips positions that cannot even match the first byte.
// Precompute all positions, including those later swallowed by copy tokens.
fn previous_equal_bytes(input: &[u8]) -> Vec<Option<usize>> {
    let mut last = [None; 256];
    input
        .iter()
        .enumerate()
        .map(|(position, &byte)| {
            let previous = last[usize::from(byte)];
            last[usize::from(byte)] = Some(position);
            previous
        })
        .collect()
}

fn best_match(input: &[u8], previous: &[Option<usize>], position: usize) -> (usize, usize) {
    let mut candidate = previous[position];
    let lower_bound = position.saturating_sub(WINDOW);
    let mut best_length = 0;
    let mut distance = 0;
    // Read-only virtual padding follows the wiki's original compressor rule;
    // padded bytes affect match selection but are never output.
    let byte = |index| input.get(index).copied().unwrap_or(0);
    while let Some(start) = candidate.filter(|&start| start >= lower_bound) {
        let length = (0..MAX_MATCH)
            .take_while(|&offset| byte(start + offset) == byte(position + offset))
            .count();
        if length > best_length {
            best_length = length;
            distance = position - start;
            if length == MAX_MATCH {
                break;
            }
        }
        candidate = previous[start];
    }
    (distance, best_length.min(input.len() - position))
}

struct Writer {
    bytes: Vec<u8>,
    control_position: usize,
    remaining: u8,
}

impl Writer {
    fn new(size: u16, first: u8) -> Self {
        let length = size.to_le_bytes();
        Self {
            bytes: vec![0, length[0], length[1], first],
            control_position: 0,
            remaining: 0,
        }
    }

    fn bit(&mut self, value: bool) {
        if self.remaining == 0 {
            self.control_position = self.bytes.len();
            self.bytes.push(0);
            self.remaining = 8;
        }
        self.remaining -= 1;
        self.bytes[self.control_position] |= u8::from(value) << self.remaining;
    }

    fn copy(&mut self, distance: usize, length: usize) {
        self.bit(false);
        if (2..=5).contains(&length) && distance <= 256 {
            self.bit(false);
            self.bit((length - 2) & 2 != 0);
            self.bit((length - 2) & 1 != 0);
            self.bytes
                .push(u8::try_from(256 - distance).expect("short distance is 1..=256"));
        } else {
            self.bit(true);
            let offset = u16::try_from(WINDOW - distance).expect("distance is 1..=8192");
            let short_length = if length <= 9 { length - 2 } else { 0 };
            let word = (offset << 3) | u16::try_from(short_length).expect("length field is 0..=7");
            self.bytes.extend_from_slice(&word.to_be_bytes());
            if short_length == 0 {
                self.bytes
                    .push(u8::try_from(length - 1).expect("extended length is 2..=256"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{best_match, previous_equal_bytes, WINDOW};

    #[test]
    fn matching_includes_the_window_edge_but_not_older_bytes() {
        for distance in [WINDOW, WINDOW + 1] {
            let mut input = vec![0xFF; distance + 3];
            input[..3].copy_from_slice(b"ABC");
            input[distance..].copy_from_slice(b"ABC");
            let found = best_match(&input, &previous_equal_bytes(&input), distance);
            assert_eq!(
                found,
                if distance == WINDOW {
                    (WINDOW, 3)
                } else {
                    (0, 0)
                }
            );
        }
    }
}
