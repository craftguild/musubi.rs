//! Fixed-point wrappers whose scale is carried in the Rust type.

use crate::{Int, Uint, ValueError};

/// An unsigned fixed-point value with `DECIMALS` decimal places.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct UFixed<const BITS: usize, const DECIMALS: usize>(Uint<BITS>);

impl<const BITS: usize, const DECIMALS: usize> UFixed<BITS, DECIMALS> {
    /// Wraps a scaled integer after validating Solidity fixed-point parameters.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError::InvalidDecimals`] for an invalid width or decimal count.
    pub fn from_scaled(value: Uint<BITS>) -> Result<Self, ValueError> {
        validate_decimals(BITS, DECIMALS)?;
        Ok(Self(value))
    }

    /// Returns the integer scaled by ten to `DECIMALS`.
    #[must_use]
    pub const fn into_scaled(self) -> Uint<BITS> {
        self.0
    }
}

/// A signed fixed-point value with `DECIMALS` decimal places.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Fixed<const BITS: usize, const DECIMALS: usize>(Int<BITS>);

impl<const BITS: usize, const DECIMALS: usize> Fixed<BITS, DECIMALS> {
    /// Wraps a scaled integer after validating Solidity fixed-point parameters.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError::InvalidDecimals`] for an invalid width or decimal count.
    pub fn from_scaled(value: Int<BITS>) -> Result<Self, ValueError> {
        validate_decimals(BITS, DECIMALS)?;
        Ok(Self(value))
    }

    /// Returns the integer scaled by ten to `DECIMALS`.
    #[must_use]
    pub const fn into_scaled(self) -> Int<BITS> {
        self.0
    }
}

fn validate_decimals(bits: usize, decimals: usize) -> Result<(), ValueError> {
    if (8..=256).contains(&bits) && bits.is_multiple_of(8) && decimals <= 80 {
        Ok(())
    } else {
        Err(ValueError::InvalidDecimals)
    }
}
