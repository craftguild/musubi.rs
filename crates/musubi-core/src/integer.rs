//! Width-checked owned signed and unsigned Ethereum integers.

use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use crate::ValueError;

/// An unsigned integer stored as a 256-bit big-endian word.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Uint<const BITS: usize>([u8; 32]);

impl<const BITS: usize> Uint<BITS> {
    /// Zero at the declared width.
    pub const ZERO: Self = Self([0; 32]);

    /// Constructs a width-checked integer from a big-endian word.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError`] for an invalid declaration or non-zero bits above the declared width.
    pub fn from_be_bytes(bytes: [u8; 32]) -> Result<Self, ValueError> {
        validate_width(BITS)?;
        let unused_bits = 256 - BITS;
        let full_unused_bytes = unused_bits / 8;
        let partial_unused_bits = unused_bits % 8;
        if bytes[..full_unused_bytes].iter().any(|byte| *byte != 0) {
            return Err(ValueError::IntegerOutOfRange);
        }
        if partial_unused_bits > 0 && bytes[full_unused_bytes] >> (8 - partial_unused_bits) != 0 {
            return Err(ValueError::IntegerOutOfRange);
        }
        Ok(Self(bytes))
    }

    /// Constructs a value from an unsigned 128-bit integer.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError`] when the value does not fit the declared width.
    pub fn from_u128(value: u128) -> Result<Self, ValueError> {
        let mut bytes = [0; 32];
        bytes[16..].copy_from_slice(&value.to_be_bytes());
        Self::from_be_bytes(bytes)
    }

    /// Returns the canonical 256-bit big-endian word.
    #[must_use]
    pub const fn to_be_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Returns the value as `u128` when it fits.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError::IntegerOutOfRange`] when upper bits are non-zero.
    pub fn try_into_u128(self) -> Result<u128, ValueError> {
        validate_width(BITS)?;
        if self.0[..16].iter().any(|byte| *byte != 0) {
            return Err(ValueError::IntegerOutOfRange);
        }
        Ok(u128::from_be_bytes(
            self.0[16..]
                .try_into()
                .map_err(|_| ValueError::IntegerOutOfRange)?,
        ))
    }
}

impl<const BITS: usize> Default for Uint<BITS> {
    fn default() -> Self {
        Self::ZERO
    }
}

macro_rules! impl_try_from_uint {
    ($($primitive:ty),+ $(,)?) => {
        $(
            impl<const BITS: usize> TryFrom<Uint<BITS>> for $primitive {
                type Error = ValueError;

                fn try_from(value: Uint<BITS>) -> Result<Self, Self::Error> {
                    Self::try_from(value.try_into_u128()?)
                        .map_err(|_| ValueError::IntegerOutOfRange)
                }
            }
        )+
    };
}

impl_try_from_uint!(u8, u16, u32, u64, u128);

impl<const BITS: usize> fmt::Debug for Uint<BITS> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Uint<{BITS}>(0x{})", hex::encode(self.0))
    }
}

impl<const BITS: usize> Serialize for Uint<BITS> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("0x{}", hex::encode(self.0)))
    }
}

impl<'de, const BITS: usize> Deserialize<'de> for Uint<BITS> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let raw = value.strip_prefix("0x").unwrap_or(&value);
        if raw.len() > 64 || raw.len() % 2 != 0 {
            return Err(D::Error::custom(ValueError::InvalidHex));
        }
        let mut bytes = [0; 32];
        hex::decode_to_slice(raw, &mut bytes[32 - raw.len() / 2..])
            .map_err(|_| D::Error::custom(ValueError::InvalidHex))?;
        Self::from_be_bytes(bytes).map_err(D::Error::custom)
    }
}

/// A signed two's-complement integer stored as a sign-extended 256-bit word.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Int<const BITS: usize>([u8; 32]);

impl<const BITS: usize> Int<BITS> {
    /// Zero at the declared width.
    pub const ZERO: Self = Self([0; 32]);

    /// Constructs a signed integer after checking canonical sign extension.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError`] for an invalid declaration or incorrect sign extension.
    pub fn from_be_bytes(bytes: [u8; 32]) -> Result<Self, ValueError> {
        validate_width(BITS)?;
        let sign_bit_index = 256 - BITS;
        let sign_byte = sign_bit_index / 8;
        let bit_in_byte = 7 - (sign_bit_index % 8);
        let negative = bytes[sign_byte] & (1 << bit_in_byte) != 0;
        let expected = if negative { 0xff } else { 0x00 };
        if bytes[..sign_byte].iter().any(|byte| *byte != expected) {
            return Err(ValueError::IntegerOutOfRange);
        }
        let upper_mask = if bit_in_byte == 7 {
            0
        } else {
            !((1_u8 << (bit_in_byte + 1)) - 1)
        };
        if bytes[sign_byte] & upper_mask != expected & upper_mask {
            return Err(ValueError::IntegerOutOfRange);
        }
        Ok(Self(bytes))
    }

    /// Constructs a signed value from an `i128`.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError`] when the value does not fit the declared width.
    pub fn from_i128(value: i128) -> Result<Self, ValueError> {
        let mut bytes = if value < 0 { [0xff; 32] } else { [0; 32] };
        bytes[16..].copy_from_slice(&value.to_be_bytes());
        Self::from_be_bytes(bytes)
    }

    /// Returns the sign-extended big-endian word.
    #[must_use]
    pub const fn to_be_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Returns the value as `i128` when it fits.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError::IntegerOutOfRange`] when the upper 128 bits are not the canonical
    /// sign extension of the lower 128 bits.
    pub fn try_into_i128(self) -> Result<i128, ValueError> {
        validate_width(BITS)?;
        let tail: [u8; 16] = self.0[16..]
            .try_into()
            .map_err(|_| ValueError::IntegerOutOfRange)?;
        let value = i128::from_be_bytes(tail);
        let expected = if value.is_negative() { 0xff } else { 0x00 };
        if self.0[..16].iter().any(|byte| *byte != expected) {
            return Err(ValueError::IntegerOutOfRange);
        }
        Ok(value)
    }
}

impl<const BITS: usize> Default for Int<BITS> {
    fn default() -> Self {
        Self::ZERO
    }
}

macro_rules! impl_try_from_int {
    ($($primitive:ty),+ $(,)?) => {
        $(
            impl<const BITS: usize> TryFrom<Int<BITS>> for $primitive {
                type Error = ValueError;

                fn try_from(value: Int<BITS>) -> Result<Self, Self::Error> {
                    Self::try_from(value.try_into_i128()?)
                        .map_err(|_| ValueError::IntegerOutOfRange)
                }
            }
        )+
    };
}

impl_try_from_int!(i8, i16, i32, i64, i128);

impl<const BITS: usize> fmt::Debug for Int<BITS> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Int<{BITS}>(0x{})", hex::encode(self.0))
    }
}

impl<const BITS: usize> Serialize for Int<BITS> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("0x{}", hex::encode(self.0)))
    }
}

impl<'de, const BITS: usize> Deserialize<'de> for Int<BITS> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let raw = value.strip_prefix("0x").unwrap_or(&value);
        if raw.len() != 64 {
            return Err(D::Error::custom(ValueError::InvalidLength {
                expected: 64,
                actual: raw.len(),
            }));
        }
        let mut bytes = [0; 32];
        hex::decode_to_slice(raw, &mut bytes)
            .map_err(|_| D::Error::custom(ValueError::InvalidHex))?;
        Self::from_be_bytes(bytes).map_err(D::Error::custom)
    }
}

fn validate_width(bits: usize) -> Result<(), ValueError> {
    if (1..=256).contains(&bits) {
        Ok(())
    } else {
        Err(ValueError::InvalidBitWidth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsigned_width_is_enforced() {
        assert_eq!(
            Uint::<8>::from_u128(255).unwrap().try_into_u128().unwrap(),
            255
        );
        assert_eq!(
            Uint::<8>::from_u128(256),
            Err(ValueError::IntegerOutOfRange)
        );
        assert_eq!(u8::try_from(Uint::<16>::from_u128(255).unwrap()), Ok(255));
        assert_eq!(
            u8::try_from(Uint::<16>::from_u128(256).unwrap()),
            Err(ValueError::IntegerOutOfRange)
        );
        let mut above_u128 = [0_u8; 32];
        above_u128[15] = 1;
        assert_eq!(
            u128::try_from(Uint::<256>::from_be_bytes(above_u128).unwrap()),
            Err(ValueError::IntegerOutOfRange)
        );
    }

    #[test]
    fn signed_width_is_enforced() {
        assert!(Int::<8>::from_i128(-128).is_ok());
        assert!(Int::<8>::from_i128(127).is_ok());
        assert_eq!(Int::<8>::from_i128(128), Err(ValueError::IntegerOutOfRange));
        assert_eq!(
            Int::<8>::from_i128(-129),
            Err(ValueError::IntegerOutOfRange)
        );
        assert_eq!(i8::try_from(Int::<16>::from_i128(-128).unwrap()), Ok(-128));
        assert_eq!(i8::try_from(Int::<16>::from_i128(127).unwrap()), Ok(127));
        assert_eq!(
            i8::try_from(Int::<16>::from_i128(128).unwrap()),
            Err(ValueError::IntegerOutOfRange)
        );
        assert_eq!(
            i8::try_from(Int::<16>::from_i128(-129).unwrap()),
            Err(ValueError::IntegerOutOfRange)
        );
    }

    #[test]
    fn signed_256_values_outside_i128_are_rejected_without_truncation() {
        let mut positive = [0_u8; 32];
        positive[15] = 1;
        let mut negative = [0xff_u8; 32];
        negative[15] = 0xfe;
        for value in [positive, negative] {
            let value = Int::<256>::from_be_bytes(value).unwrap();
            assert_eq!(value.try_into_i128(), Err(ValueError::IntegerOutOfRange));
            assert_eq!(i128::try_from(value), Err(ValueError::IntegerOutOfRange));
        }
    }

    #[test]
    fn primitive_conversions_reject_invalid_declared_widths() {
        assert_eq!(
            Uint::<0>::ZERO.try_into_u128(),
            Err(ValueError::InvalidBitWidth)
        );
        assert_eq!(
            u8::try_from(Uint::<257>::ZERO),
            Err(ValueError::InvalidBitWidth)
        );
        assert_eq!(
            Int::<0>::ZERO.try_into_i128(),
            Err(ValueError::InvalidBitWidth)
        );
        assert_eq!(
            i8::try_from(Int::<257>::ZERO),
            Err(ValueError::InvalidBitWidth)
        );
    }
}
