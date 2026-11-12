//! Conversion between generated owned Rust fields and dynamic ABI boundary values.

use musubi_core::{
    AbiValue, Address, Bytes, Fixed, FixedBytes, FunctionPointer, Int, UFixed, Uint,
};

use crate::CodecError;

/// Converts one generated typed field into its dynamic codec-boundary representation.
pub trait IntoAbiValue {
    /// Performs an ownership-preserving conversion without changing wire identity.
    fn into_abi_value(self) -> AbiValue;
}

/// Converts one dynamic codec-boundary value into a generated typed field.
pub trait FromAbiValue: Sized {
    /// Performs a width- and variant-checked conversion.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::TypeMismatch`] if the dynamic value has a different ABI shape.
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError>;
}

impl<T: Clone + IntoAbiValue> IntoAbiValue for &T {
    fn into_abi_value(self) -> AbiValue {
        self.clone().into_abi_value()
    }
}

impl IntoAbiValue for Address {
    fn into_abi_value(self) -> AbiValue {
        AbiValue::Address(self)
    }
}

impl FromAbiValue for Address {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        match value {
            AbiValue::Address(value) => Ok(value),
            _ => Err(CodecError::TypeMismatch),
        }
    }
}

impl IntoAbiValue for bool {
    fn into_abi_value(self) -> AbiValue {
        AbiValue::Bool(self)
    }
}

impl FromAbiValue for bool {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        match value {
            AbiValue::Bool(value) => Ok(value),
            _ => Err(CodecError::TypeMismatch),
        }
    }
}

impl IntoAbiValue for String {
    fn into_abi_value(self) -> AbiValue {
        AbiValue::String(self)
    }
}

impl FromAbiValue for String {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        match value {
            AbiValue::String(value) => Ok(value),
            _ => Err(CodecError::TypeMismatch),
        }
    }
}

impl IntoAbiValue for Bytes {
    fn into_abi_value(self) -> AbiValue {
        AbiValue::Bytes(self)
    }
}

impl FromAbiValue for Bytes {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        match value {
            AbiValue::Bytes(value) => Ok(value),
            _ => Err(CodecError::TypeMismatch),
        }
    }
}

impl<const N: usize> IntoAbiValue for FixedBytes<N> {
    fn into_abi_value(self) -> AbiValue {
        AbiValue::FixedBytes(Bytes::new(self.as_ref().to_vec()))
    }
}

impl<const N: usize> FromAbiValue for FixedBytes<N> {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        match value {
            AbiValue::FixedBytes(value) => {
                Self::from_slice(value.as_ref()).map_err(|_| CodecError::TypeMismatch)
            }
            _ => Err(CodecError::TypeMismatch),
        }
    }
}

impl<const BITS: usize> IntoAbiValue for Uint<BITS> {
    fn into_abi_value(self) -> AbiValue {
        AbiValue::Uint(self.to_be_bytes())
    }
}

impl<const BITS: usize> FromAbiValue for Uint<BITS> {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        match value {
            AbiValue::Uint(value) => {
                Self::from_be_bytes(value).map_err(|_| CodecError::TypeMismatch)
            }
            _ => Err(CodecError::TypeMismatch),
        }
    }
}

impl<const BITS: usize> IntoAbiValue for Int<BITS> {
    fn into_abi_value(self) -> AbiValue {
        AbiValue::Int(self.to_be_bytes())
    }
}

impl<const BITS: usize> FromAbiValue for Int<BITS> {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        match value {
            AbiValue::Int(value) => {
                Self::from_be_bytes(value).map_err(|_| CodecError::TypeMismatch)
            }
            _ => Err(CodecError::TypeMismatch),
        }
    }
}

impl<const BITS: usize, const DECIMALS: usize> IntoAbiValue for UFixed<BITS, DECIMALS> {
    fn into_abi_value(self) -> AbiValue {
        AbiValue::UFixed(self.into_scaled().to_be_bytes())
    }
}

impl<const BITS: usize, const DECIMALS: usize> FromAbiValue for UFixed<BITS, DECIMALS> {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        match value {
            AbiValue::UFixed(value) => Uint::from_be_bytes(value)
                .and_then(Self::from_scaled)
                .map_err(|_| CodecError::TypeMismatch),
            _ => Err(CodecError::TypeMismatch),
        }
    }
}

impl<const BITS: usize, const DECIMALS: usize> IntoAbiValue for Fixed<BITS, DECIMALS> {
    fn into_abi_value(self) -> AbiValue {
        AbiValue::Fixed(self.into_scaled().to_be_bytes())
    }
}

impl<const BITS: usize, const DECIMALS: usize> FromAbiValue for Fixed<BITS, DECIMALS> {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        match value {
            AbiValue::Fixed(value) => Int::from_be_bytes(value)
                .and_then(Self::from_scaled)
                .map_err(|_| CodecError::TypeMismatch),
            _ => Err(CodecError::TypeMismatch),
        }
    }
}

impl IntoAbiValue for FunctionPointer {
    fn into_abi_value(self) -> AbiValue {
        AbiValue::Function(self)
    }
}

impl FromAbiValue for FunctionPointer {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        match value {
            AbiValue::Function(value) => Ok(value),
            _ => Err(CodecError::TypeMismatch),
        }
    }
}

impl<T: IntoAbiValue> IntoAbiValue for Vec<T> {
    fn into_abi_value(self) -> AbiValue {
        AbiValue::Array(self.into_iter().map(IntoAbiValue::into_abi_value).collect())
    }
}

impl<T: FromAbiValue> FromAbiValue for Vec<T> {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        match value {
            AbiValue::Array(values) => values.into_iter().map(T::from_abi_value).collect(),
            _ => Err(CodecError::TypeMismatch),
        }
    }
}

impl<T: IntoAbiValue, const N: usize> IntoAbiValue for [T; N] {
    fn into_abi_value(self) -> AbiValue {
        AbiValue::Array(self.into_iter().map(IntoAbiValue::into_abi_value).collect())
    }
}

impl<T: FromAbiValue, const N: usize> FromAbiValue for [T; N] {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let values = Vec::<T>::from_abi_value(value)?;
        values.try_into().map_err(|_| CodecError::TypeMismatch)
    }
}
