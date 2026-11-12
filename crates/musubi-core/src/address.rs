//! Owned 20-byte Ethereum address.

use core::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use crate::ValueError;

/// A 20-byte Ethereum account or contract address.
#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Address([u8; 20]);

impl Address {
    /// The all-zero address.
    pub const ZERO: Self = Self([0; 20]);

    /// Constructs an address from exactly 20 bytes.
    #[must_use]
    pub const fn new(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    /// Returns the underlying big-endian bytes.
    #[must_use]
    pub const fn into_array(self) -> [u8; 20] {
        self.0
    }

    /// Borrows the underlying big-endian bytes.
    #[must_use]
    pub const fn as_array(&self) -> &[u8; 20] {
        &self.0
    }

    /// Parses a `0x`-prefixed or unprefixed 40-digit hexadecimal address.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError`] when length or hexadecimal syntax is invalid.
    pub fn from_hex(value: &str) -> Result<Self, ValueError> {
        let value = value.strip_prefix("0x").unwrap_or(value);
        if value.len() != 40 {
            return Err(ValueError::InvalidLength {
                expected: 40,
                actual: value.len(),
            });
        }
        let mut bytes = [0; 20];
        hex::decode_to_slice(value, &mut bytes).map_err(|_| ValueError::InvalidHex)?;
        Ok(Self(bytes))
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "0x{}", hex::encode(self.0))
    }
}

impl FromStr for Address {
    type Err = ValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_hex(value)
    }
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_hex(&value).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_hex_round_trip() {
        let address = Address::from_hex("0x00000000000000000000000000000000000000aB").unwrap();
        assert_eq!(
            address.to_string(),
            "0x00000000000000000000000000000000000000ab"
        );
        assert_eq!(
            serde_json::to_string(&address).unwrap(),
            format!("\"{address}\"")
        );
    }
}
