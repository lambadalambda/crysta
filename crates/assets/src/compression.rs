//! Terranigma's interleaved-control LZ packet format.
//!
//! Only the observed zero-header variant is supported. Packets must declare a
//! nonzero 16-bit output length and end with an explicit terminator at exactly
//! that length. Bytes after the terminator are left to the caller.

use std::fmt;

mod encoder;
pub use encoder::{encode, EncodeError};

/// Largest representable nonempty packet output (16-bit header length).
pub const MAX_OUTPUT_SIZE: usize = u16::MAX as usize;

/// One decoded packet and its boundary in the caller's input slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPacket {
    /// Decompressed bytes, exactly the length declared by the header.
    pub data: Vec<u8>,
    /// Bytes consumed through the terminator, including control/header bytes.
    pub consumed: usize,
}

/// A malformed or unsupported compressed packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Input ended before a requested byte or the explicit terminator.
    UnexpectedEnd {
        /// Offset of the missing byte relative to the input slice.
        offset: usize,
    },
    /// The first header byte selects an unqualified variant.
    UnsupportedHeader {
        /// Unsupported first byte.
        value: u8,
    },
    /// Zero has no qualified meaning for the header length.
    ZeroLength,
    /// The header exceeds the caller's output budget.
    OutputLimitExceeded {
        /// Length claimed by the packet.
        declared: usize,
        /// Maximum output allowed by the caller.
        limit: usize,
    },
    /// A copy would read before the beginning of the produced bytes.
    InvalidBackReference {
        /// Backward copy distance in bytes.
        distance: usize,
        /// Bytes available when the copy was requested.
        produced: usize,
    },
    /// The terminator arrived early, or a token would exceed the header length.
    LengthMismatch {
        /// Declared output length.
        expected: usize,
        /// Actual output length at termination, or attempted length on overflow.
        actual: usize,
    },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEnd { offset } => {
                write!(f, "compressed packet truncated at byte {offset}")
            }
            Self::UnsupportedHeader { value } => {
                write!(f, "unsupported packet header ${value:02X}")
            }
            Self::ZeroLength => f.write_str("zero-length compressed packets are not supported"),
            Self::OutputLimitExceeded { declared, limit } => write!(
                f,
                "packet declares {declared} output bytes, exceeding limit {limit}"
            ),
            Self::InvalidBackReference { distance, produced } => write!(
                f,
                "copy distance {distance} exceeds {produced} produced bytes"
            ),
            Self::LengthMismatch { expected, actual } => write!(
                f,
                "packet output length {actual} disagrees with declared length {expected}"
            ),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Decodes one packet from the beginning of `input` within `output_limit`.
///
/// Control bytes are consumed MSB first, fetched only when the next control bit
/// is needed. Copies may overlap the bytes being written. Terminator offsets
/// and unused final control bits are ignored, as specified by the format.
/// Allocation and work are bounded by the nonzero 16-bit header length and the
/// supplied slice; the caller's limit is checked before output allocation.
///
/// # Errors
///
/// Rejects unsupported headers, zero or over-budget lengths, truncated tokens,
/// invalid backward copies, and output lengths inconsistent with the header.
pub fn decode(input: &[u8], output_limit: usize) -> Result<DecodedPacket, DecodeError> {
    let mut reader = Reader::new(input);
    let header = reader.byte()?;
    if header != 0 {
        return Err(DecodeError::UnsupportedHeader { value: header });
    }
    let expected = usize::from(u16::from_le_bytes([reader.byte()?, reader.byte()?]));
    if expected == 0 {
        return Err(DecodeError::ZeroLength);
    }
    if expected > output_limit {
        return Err(DecodeError::OutputLimitExceeded {
            declared: expected,
            limit: output_limit,
        });
    }
    let first = reader.byte()?;
    let mut data = Vec::with_capacity(expected);
    data.push(first);
    loop {
        match reader.token()? {
            Token::Literal(byte) => {
                check_length(expected, data.len() + 1)?;
                data.push(byte);
            }
            Token::Copy { distance, length } => {
                if distance > data.len() {
                    return Err(DecodeError::InvalidBackReference {
                        distance,
                        produced: data.len(),
                    });
                }
                check_length(expected, data.len() + length)?;
                for _ in 0..length {
                    data.push(data[data.len() - distance]);
                }
            }
            Token::End => {
                if data.len() != expected {
                    return Err(DecodeError::LengthMismatch {
                        expected,
                        actual: data.len(),
                    });
                }
                return Ok(DecodedPacket {
                    data,
                    consumed: reader.position,
                });
            }
        }
    }
}

fn check_length(expected: usize, actual: usize) -> Result<(), DecodeError> {
    if actual > expected {
        Err(DecodeError::LengthMismatch { expected, actual })
    } else {
        Ok(())
    }
}

enum Token {
    Literal(u8),
    Copy { distance: usize, length: usize },
    End,
}

struct Reader<'a> {
    input: &'a [u8],
    position: usize,
    control: u8,
    remaining: u8,
}

impl<'a> Reader<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            position: 0,
            control: 0,
            remaining: 0,
        }
    }

    fn byte(&mut self) -> Result<u8, DecodeError> {
        let byte = self
            .input
            .get(self.position)
            .copied()
            .ok_or(DecodeError::UnexpectedEnd {
                offset: self.position,
            })?;
        self.position += 1;
        Ok(byte)
    }

    fn bit(&mut self) -> Result<bool, DecodeError> {
        if self.remaining == 0 {
            self.control = self.byte()?;
            self.remaining = 8;
        }
        let value = self.control & 0x80 != 0;
        self.control <<= 1;
        self.remaining -= 1;
        Ok(value)
    }

    fn token(&mut self) -> Result<Token, DecodeError> {
        if self.bit()? {
            return Ok(Token::Literal(self.byte()?));
        }
        if !self.bit()? {
            let length = 2 + usize::from(self.bit()?) * 2 + usize::from(self.bit()?);
            let distance = 256 - usize::from(self.byte()?);
            return Ok(Token::Copy { distance, length });
        }
        let word = u16::from_be_bytes([self.byte()?, self.byte()?]);
        let distance = 8192 - usize::from(word >> 3);
        let short_length = usize::from(word & 7);
        let length = if short_length == 0 {
            let extended = self.byte()?;
            if extended == 0 {
                return Ok(Token::End);
            }
            usize::from(extended) + 1
        } else {
            short_length + 2
        };
        Ok(Token::Copy { distance, length })
    }
}
