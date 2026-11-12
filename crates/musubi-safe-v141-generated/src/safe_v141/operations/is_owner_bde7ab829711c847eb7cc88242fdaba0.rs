//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `isOwner(address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IsOwnerArgs {
    /// ABI field `owner`.
    pub owner: Address,
}

impl IntoAbiValue for IsOwnerArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.owner.into_abi_value(),
    ]) }
}
impl FromAbiValue for IsOwnerArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(IsOwnerArgs {
            owner: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl IsOwnerArgs {
    /// Encodes this value without a function selector using Solidity ABI tuple rules.
    pub fn abi_encode(self) -> Result<musubi_core::Bytes, CodecError> {
        musubi_runtime::encode_fields(&abi::descriptor().inputs, &<Self as musubi_runtime::EncodeArguments>::into_values(self))
    }
    /// Strictly decodes a selector-free Solidity ABI tuple value.
    pub fn abi_decode(data: &[u8]) -> Result<Self, CodecError> {
        <Self as musubi_runtime::FromAbiValue>::from_abi_value(AbiValue::Tuple(musubi_runtime::decode_fields(&abi::descriptor().inputs, data)?))
    }
}

/// Return values for `isOwner(address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IsOwnerReturn {
    /// ABI field `#0`.
    pub value_0: bool,
}

impl IntoAbiValue for IsOwnerReturn {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.value_0.into_abi_value(),
    ]) }
}
impl FromAbiValue for IsOwnerReturn {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(IsOwnerReturn {
            value_0: <bool as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl musubi_runtime::DecodeReturns for IsOwnerReturn {
    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError> {
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(IsOwnerReturn {
            value_0: <bool as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl IsOwnerReturn {
    /// Encodes this value without a function selector using Solidity ABI tuple rules.
    pub fn abi_encode(self) -> Result<musubi_core::Bytes, CodecError> {
        musubi_runtime::encode_fields(&abi::descriptor().outputs, &<Self as musubi_runtime::EncodeArguments>::into_values(self))
    }
    /// Strictly decodes a selector-free Solidity ABI tuple value.
    pub fn abi_decode(data: &[u8]) -> Result<Self, CodecError> {
        <Self as musubi_runtime::FromAbiValue>::from_abi_value(AbiValue::Tuple(musubi_runtime::decode_fields(&abi::descriptor().outputs, data)?))
    }
}

mod abi {
use musubi_core::*;
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "isOwner".into(), signature: "isOwner(address)".into(), selector: [47, 84, 191, 110], inputs: vec![AbiField { name: "owner".into(), ty: AbiType::Address, internal_type: Some("address".into()) }], outputs: vec![AbiField { name: "".into(), ty: AbiType::Bool, internal_type: Some("bool".into()) }], state_mutability: StateMutability::View } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionCall;

impl SafeV141 {
    /// Builds `isOwner(address)` as an offline typed operation.
    #[must_use]
    pub fn is_owner(arguments: IsOwnerArgs) -> FunctionCall<IsOwnerArgs, IsOwnerReturn> { FunctionCall::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `isOwner(address)`.
    #[must_use]
    pub fn is_owner(&self, arguments: IsOwnerArgs) -> musubi_runtime::BoundCall<B, IsOwnerArgs, IsOwnerReturn> { SafeV141::is_owner(arguments).at(self.address).connect(self.backend.clone()) }
}
