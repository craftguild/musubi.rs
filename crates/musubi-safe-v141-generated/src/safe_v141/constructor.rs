//! Constructor arguments, descriptor, linking, and deployment operations.

use musubi_core::{AbiValue};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed constructor arguments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstructorArgs {
}

impl IntoAbiValue for ConstructorArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
    ]) }
}
impl FromAbiValue for ConstructorArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if !values.is_empty() { return Err(CodecError::Arity); }
        Ok(ConstructorArgs {})
    }
}

impl ConstructorArgs {
    /// Encodes this value without a function selector using Solidity ABI tuple rules.
    pub fn abi_encode(self) -> Result<musubi_core::Bytes, CodecError> {
        musubi_runtime::encode_fields(&constructor_inputs(), &<Self as musubi_runtime::EncodeArguments>::into_values(self))
    }
    /// Strictly decodes a selector-free Solidity ABI tuple value.
    pub fn abi_decode(data: &[u8]) -> Result<Self, CodecError> {
        <Self as musubi_runtime::FromAbiValue>::from_abi_value(AbiValue::Tuple(musubi_runtime::decode_fields(&constructor_inputs(), data)?))
    }
}

use super::{SafeV141, artifact};
fn constructor_inputs() -> Vec<musubi_core::AbiField> {
vec![] }
impl SafeV141 {
    /// Builds linked constructor init code with exact library coverage.
    pub fn init_code(libraries: &musubi_runtime::LibraryAddresses) -> Result<musubi_core::Bytes, musubi_runtime::DeploymentError> {
        musubi_runtime::build_init_code(&artifact().creation, libraries, &constructor_inputs(), &Vec::new())
    }
    /// Builds an unsigned contract-creation transaction after enforcing constructor payability.
    pub fn deployment(libraries: &musubi_runtime::LibraryAddresses, value: musubi_core::U256) -> Result<musubi_core::TransactionRequest, musubi_runtime::DeploymentError> {
        if value != musubi_core::U256::ZERO { return Err(musubi_runtime::CodecError::UnexpectedValue.into()); }
        let input = Self::init_code(libraries)?; Ok(musubi_core::TransactionRequest { to: None, input, value, ..musubi_core::TransactionRequest::default() })
    }
}
