//! Owned dynamic and fixed-width byte values.

use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use crate::ValueError;

/// An owned arbitrary-length byte string.
#[derive(Clone, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Bytes(Vec<u8>);

impl Bytes {
    /// Creates an owned byte string.
    #[must_use]
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    /// Parses an even-length, optionally `0x`-prefixed hexadecimal string.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError::InvalidHex`] for odd length or non-hexadecimal input.
    pub fn from_hex(value: &str) -> Result<Self, ValueError> {
        let value = value.strip_prefix("0x").unwrap_or(value);
        if !value.len().is_multiple_of(2) {
            return Err(ValueError::InvalidHex);
        }
        hex::decode(value)
            .map(Self)
            .map_err(|_| ValueError::InvalidHex)
    }

    /// Returns the byte length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Reports whether the value contains no bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Consumes the wrapper and returns the underlying vector.
    #[must_use]
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<&[u8]> for Bytes {
    fn from(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }
}

impl fmt::Debug for Bytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "0x{}", hex::encode(&self.0))
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("0x{}", hex::encode(&self.0)))
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_hex(&value).map_err(D::Error::custom)
    }
}

/// An owned fixed-width byte string.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FixedBytes<const N: usize>([u8; N]);

impl<const N: usize> FixedBytes<N> {
    /// Creates a fixed-width byte string.
    #[must_use]
    pub const fn new(bytes: [u8; N]) -> Self {
        Self(bytes)
    }

    /// Returns the underlying array.
    #[must_use]
    pub const fn into_array(self) -> [u8; N] {
        self.0
    }

    /// Borrows the underlying array.
    #[must_use]
    pub const fn as_array(&self) -> &[u8; N] {
        &self.0
    }

    /// Copies a byte slice of the exact declared width.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError::InvalidLength`] when the slice length differs from `N`.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, ValueError> {
        let bytes: [u8; N] = bytes.try_into().map_err(|_| ValueError::InvalidLength {
            expected: N,
            actual: bytes.len(),
        })?;
        Ok(Self(bytes))
    }
}

impl<const N: usize> Default for FixedBytes<N> {
    fn default() -> Self {
        Self([0; N])
    }
}

impl<const N: usize> AsRef<[u8]> for FixedBytes<N> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<const N: usize> fmt::Debug for FixedBytes<N> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "0x{}", hex::encode(self.0))
    }
}

impl<const N: usize> Serialize for FixedBytes<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("0x{}", hex::encode(self.0)))
    }
}

impl<'de, const N: usize> Deserialize<'de> for FixedBytes<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Bytes::deserialize(deserializer)?;
        Self::from_slice(value.as_ref()).map_err(D::Error::custom)
    }
}

/// A 32-byte digest, topic, or transaction hash.
pub type Hash = FixedBytes<32>;
