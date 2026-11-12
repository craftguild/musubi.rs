//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `isModuleEnabled(address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IsModuleEnabledArgs {
    /// ABI field `module`.
    pub module: Address,
}

impl IntoAbiValue for IsModuleEnabledArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.module.into_abi_value(),
    ]) }
}
impl FromAbiValue for IsModuleEnabledArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(IsModuleEnabledArgs {
            module: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl IsModuleEnabledArgs {
    /// Encodes this value without a function selector using Solidity ABI tuple rules.
    pub fn abi_encode(self) -> Result<musubi_core::Bytes, CodecError> {
        musubi_runtime::encode_fields(&abi::descriptor().inputs, &<Self as musubi_runtime::EncodeArguments>::into_values(self))
    }
    /// Strictly decodes a selector-free Solidity ABI tuple value.
    pub fn abi_decode(data: &[u8]) -> Result<Self, CodecError> {
        <Self as musubi_runtime::FromAbiValue>::from_abi_value(AbiValue::Tuple(musubi_runtime::decode_fields(&abi::descriptor().inputs, data)?))
    }
}

/// Return values for `isModuleEnabled(address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IsModuleEnabledReturn {
    /// ABI field `#0`.
    pub value_0: bool,
}

impl IntoAbiValue for IsModuleEnabledReturn {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.value_0.into_abi_value(),
    ]) }
}
impl FromAbiValue for IsModuleEnabledReturn {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(IsModuleEnabledReturn {
            value_0: <bool as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl musubi_runtime::DecodeReturns for IsModuleEnabledReturn {
    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError> {
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(IsModuleEnabledReturn {
            value_0: <bool as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl IsModuleEnabledReturn {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "isModuleEnabled".into(), signature: "isModuleEnabled(address)".into(), selector: [45, 154, 213, 61], inputs: vec![AbiField { name: "module".into(), ty: AbiType::Address, internal_type: Some("address".into()) }], outputs: vec![AbiField { name: "".into(), ty: AbiType::Bool, internal_type: Some("bool".into()) }], state_mutability: StateMutability::View } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionCall;

impl SafeV141 {
    /// Builds `isModuleEnabled(address)` as an offline typed operation.
    #[must_use]
    pub fn is_module_enabled(arguments: IsModuleEnabledArgs) -> FunctionCall<IsModuleEnabledArgs, IsModuleEnabledReturn> { FunctionCall::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `isModuleEnabled(address)`.
    #[must_use]
    pub fn is_module_enabled(&self, arguments: IsModuleEnabledArgs) -> musubi_runtime::BoundCall<B, IsModuleEnabledArgs, IsModuleEnabledReturn> { SafeV141::is_module_enabled(arguments).at(self.address).connect(self.backend.clone()) }
}
