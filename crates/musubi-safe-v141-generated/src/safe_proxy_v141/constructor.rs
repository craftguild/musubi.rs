//! Constructor arguments, descriptor, linking, and deployment operations.

use musubi_core::{AbiValue, Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed constructor arguments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstructorArgs {
    /// ABI field `_singleton`.
    pub _singleton: Address,
}

impl IntoAbiValue for ConstructorArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self._singleton.into_abi_value(),
    ]) }
}
impl FromAbiValue for ConstructorArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(ConstructorArgs {
            _singleton: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
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

use super::{SafeProxyV141, artifact};
fn constructor_inputs() -> Vec<musubi_core::AbiField> {
    use musubi_core::*;
vec![AbiField { name: "_singleton".into(), ty: AbiType::Address, internal_type: Some("address".into()) }] }
impl SafeProxyV141 {
    /// Builds linked constructor init code with exact library coverage.
    pub fn init_code(arguments: ConstructorArgs, libraries: &musubi_runtime::LibraryAddresses) -> Result<musubi_core::Bytes, musubi_runtime::DeploymentError> {
        use musubi_runtime::EncodeArguments as _;
        musubi_runtime::build_init_code(&artifact().creation, libraries, &constructor_inputs(), &arguments.into_values())
    }
    /// Builds an unsigned contract-creation transaction after enforcing constructor payability.
    pub fn deployment(arguments: ConstructorArgs, libraries: &musubi_runtime::LibraryAddresses, value: musubi_core::U256) -> Result<musubi_core::TransactionRequest, musubi_runtime::DeploymentError> {
        if value != musubi_core::U256::ZERO { return Err(musubi_runtime::CodecError::UnexpectedValue.into()); }
        let input = Self::init_code(arguments, libraries)?; Ok(musubi_core::TransactionRequest { to: None, input, value, ..musubi_core::TransactionRequest::default() })
    }
}
