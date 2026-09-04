//! Typed conversions among Terranigma ROM address spaces.

use std::fmt;

/// An error produced when constructing or converting a ROM address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum AddressError {
    /// A normalized file offset lies outside the 4 MiB image.
    NormalizedOffsetOutOfRange {
        /// The rejected offset.
        value: u32,
    },
    /// A canonical address lies outside `$C0:0000..=$FF:FFFF`.
    CanonicalAddressOutOfRange {
        /// The rejected address.
        value: u32,
    },
    /// A runtime address is wider than the SNES 24-bit address bus.
    RuntimeAddressOutOfRange {
        /// The rejected address.
        value: u32,
    },
    /// A 24-bit address does not lie in a ROM-backed `HiROM` window.
    UnmappedRuntimeAddress {
        /// The rejected address.
        address: u32,
    },
}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NormalizedOffsetOutOfRange { value } => {
                write!(
                    f,
                    "normalized ROM offset ${value:06X} is outside the 4 MiB image"
                )
            }
            Self::CanonicalAddressOutOfRange { value } => write!(
                f,
                "canonical ROM address ${value:06X} is outside $C00000..=$FFFFFF"
            ),
            Self::RuntimeAddressOutOfRange { value } => {
                write!(f, "runtime ROM address ${value:X} is wider than 24 bits")
            }
            Self::UnmappedRuntimeAddress { address } => write!(
                f,
                "runtime address ${:02X}:{:04X} is not in a HiROM ROM window",
                address >> 16,
                address & 0xffff
            ),
        }
    }
}

impl std::error::Error for AddressError {}

/// A byte offset in the normalized, headerless 4 MiB ROM image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NormalizedOffset(u32);

impl NormalizedOffset {
    /// The first normalized image offset.
    pub const MIN: u32 = 0x00_0000;

    /// The final normalized image offset.
    pub const MAX: u32 = 0x3f_ffff;

    /// Constructs a normalized image offset.
    ///
    /// # Errors
    ///
    /// Returns [`AddressError::NormalizedOffsetOutOfRange`] when `value` is
    /// beyond the end of the 4 MiB image.
    pub const fn new(value: u32) -> Result<Self, AddressError> {
        if value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(AddressError::NormalizedOffsetOutOfRange { value })
        }
    }

    /// Returns the zero-based byte offset.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }

    /// Converts this offset to its unique canonical `HiROM` address.
    #[must_use]
    pub const fn canonical(self) -> CanonicalRomAddress {
        CanonicalRomAddress(CanonicalRomAddress::MIN + self.0)
    }
}

impl fmt::Display for NormalizedOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${:06X}", self.0)
    }
}

impl TryFrom<u32> for NormalizedOffset {
    type Error = AddressError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<NormalizedOffset> for u32 {
    fn from(value: NormalizedOffset) -> Self {
        value.value()
    }
}

/// A ROM address in the unique canonical `HiROM` `$C0..=$FF` bank range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalRomAddress(u32);

impl CanonicalRomAddress {
    /// The first canonical ROM address.
    pub const MIN: u32 = 0xc0_0000;

    /// The final canonical ROM address.
    pub const MAX: u32 = 0xff_ffff;

    /// Constructs a canonical ROM address.
    ///
    /// # Errors
    ///
    /// Returns [`AddressError::CanonicalAddressOutOfRange`] unless `value` is
    /// in `$C00000..=$FFFFFF`.
    pub const fn new(value: u32) -> Result<Self, AddressError> {
        if value >= Self::MIN && value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(AddressError::CanonicalAddressOutOfRange { value })
        }
    }

    /// Returns the 24-bit canonical address as an integer.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }

    /// Returns the normalized image offset represented by this address.
    #[must_use]
    pub const fn normalized(self) -> NormalizedOffset {
        NormalizedOffset(self.0 - Self::MIN)
    }

    /// Returns this address as a validated runtime address.
    #[must_use]
    pub const fn runtime(self) -> RuntimeRomAddress {
        RuntimeRomAddress(self.0)
    }
}

impl fmt::Display for CanonicalRomAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${:06X}", self.0)
    }
}

impl TryFrom<u32> for CanonicalRomAddress {
    type Error = AddressError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<CanonicalRomAddress> for u32 {
    fn from(value: CanonicalRomAddress) -> Self {
        value.value()
    }
}

impl From<NormalizedOffset> for CanonicalRomAddress {
    fn from(value: NormalizedOffset) -> Self {
        value.canonical()
    }
}

impl From<CanonicalRomAddress> for NormalizedOffset {
    fn from(value: CanonicalRomAddress) -> Self {
        value.normalized()
    }
}

/// A 24-bit SNES CPU address validated to lie in a `HiROM` ROM window.
///
/// Full-bank ROM windows are `$40:0000..=$7D:FFFF` and
/// `$C0:0000..=$FF:FFFF`. Mirrored half-bank windows are
/// `$00:8000..=$3F:FFFF` and `$80:8000..=$BF:FFFF`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeRomAddress(u32);

impl RuntimeRomAddress {
    /// The largest address representable by the SNES 24-bit address bus.
    pub const MAX: u32 = 0xff_ffff;

    /// Constructs and validates a runtime `HiROM` address.
    ///
    /// # Errors
    ///
    /// Returns [`AddressError::RuntimeAddressOutOfRange`] for values wider
    /// than 24 bits, or [`AddressError::UnmappedRuntimeAddress`] when the
    /// address is not in a ROM-backed `HiROM` window.
    pub const fn new(value: u32) -> Result<Self, AddressError> {
        if value > Self::MAX {
            return Err(AddressError::RuntimeAddressOutOfRange { value });
        }

        let bytes = value.to_be_bytes();
        let bank = bytes[1];
        let offset = u16::from_be_bytes([bytes[2], bytes[3]]);
        let full_bank = (bank >= 0x40 && bank <= 0x7d) || bank >= 0xc0;
        let half_bank = (bank <= 0x3f || (bank >= 0x80 && bank <= 0xbf)) && offset >= 0x8000;
        if !full_bank && !half_bank {
            return Err(AddressError::UnmappedRuntimeAddress { address: value });
        }

        let normalized = ((bank & 0x3f) as u32) << 16 | offset as u32;
        if normalized > NormalizedOffset::MAX {
            return Err(AddressError::NormalizedOffsetOutOfRange { value: normalized });
        }

        Ok(Self(value))
    }

    /// Constructs a runtime `HiROM` address from its bank and bank offset.
    ///
    /// # Errors
    ///
    /// Returns [`AddressError::UnmappedRuntimeAddress`] when the bank and
    /// offset are not in a ROM-backed `HiROM` window.
    pub const fn from_parts(bank: u8, offset: u16) -> Result<Self, AddressError> {
        Self::new(((bank as u32) << 16) | offset as u32)
    }

    /// Returns the packed 24-bit runtime address.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }

    /// Returns the runtime bank byte.
    #[must_use]
    pub const fn bank(self) -> u8 {
        self.0.to_be_bytes()[1]
    }

    /// Returns the 16-bit offset within the runtime bank.
    #[must_use]
    pub const fn offset(self) -> u16 {
        let bytes = self.0.to_be_bytes();
        u16::from_be_bytes([bytes[2], bytes[3]])
    }

    /// Resolves all valid `HiROM` mirrors to a normalized image offset.
    #[must_use]
    pub const fn normalized(self) -> NormalizedOffset {
        NormalizedOffset(((self.bank() & 0x3f) as u32) << 16 | self.offset() as u32)
    }

    /// Resolves this runtime mirror to the unique canonical ROM address.
    #[must_use]
    pub const fn canonical(self) -> CanonicalRomAddress {
        self.normalized().canonical()
    }
}

impl fmt::Display for RuntimeRomAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${:02X}:{:04X}", self.bank(), self.offset())
    }
}

impl TryFrom<u32> for RuntimeRomAddress {
    type Error = AddressError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<RuntimeRomAddress> for u32 {
    fn from(value: RuntimeRomAddress) -> Self {
        value.value()
    }
}

impl From<NormalizedOffset> for RuntimeRomAddress {
    fn from(value: NormalizedOffset) -> Self {
        value.canonical().runtime()
    }
}

impl From<CanonicalRomAddress> for RuntimeRomAddress {
    fn from(value: CanonicalRomAddress) -> Self {
        value.runtime()
    }
}

impl From<RuntimeRomAddress> for NormalizedOffset {
    fn from(value: RuntimeRomAddress) -> Self {
        value.normalized()
    }
}

impl From<RuntimeRomAddress> for CanonicalRomAddress {
    fn from(value: RuntimeRomAddress) -> Self {
        value.canonical()
    }
}
