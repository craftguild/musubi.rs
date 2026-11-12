//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `disableModule(address,address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisableModuleArgs {
    /// ABI field `prevModule`.
    pub prev_module: Address,
    /// ABI field `module`.
    pub module: Address,
}

impl IntoAbiValue for DisableModuleArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.prev_module.into_abi_value(),
        self.module.into_abi_value(),
    ]) }
}
impl FromAbiValue for DisableModuleArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 2 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(DisableModuleArgs {
            prev_module: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            module: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl DisableModuleArgs {
    /// Encodes this value without a function selector using Solidity ABI tuple rules.
    pub fn abi_encode(self) -> Result<musubi_core::Bytes, CodecError> {
        musubi_runtime::encode_fields(&abi::descriptor().inputs, &<Self as musubi_runtime::EncodeArguments>::into_values(self))
    }
    /// Strictly decodes a selector-free Solidity ABI tuple value.
    pub fn abi_decode(data: &[u8]) -> Result<Self, CodecError> {
        <Self as musubi_runtime::FromAbiValue>::from_abi_value(AbiValue::Tuple(musubi_runtime::decode_fields(&abi::descriptor().inputs, data)?))
    }
}

mod abi {
use musubi_core::*;
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "disableModule".into(), signature: "disableModule(address,address)".into(), selector: [224, 9, 207, 222], inputs: vec![AbiField { name: "prevModule".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, AbiField { name: "module".into(), ty: AbiType::Address, internal_type: Some("address".into()) }], outputs: vec![], state_mutability: StateMutability::Nonpayable } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionTransaction;

impl SafeV141 {
    /// Builds `disableModule(address,address)` as an offline typed operation.
    #[must_use]
    pub fn disable_module(arguments: DisableModuleArgs) -> FunctionTransaction<DisableModuleArgs> { FunctionTransaction::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `disableModule(address,address)`.
    #[must_use]
    pub fn disable_module(&self, arguments: DisableModuleArgs) -> musubi_runtime::BoundTransaction<B, DisableModuleArgs> { SafeV141::disable_module(arguments).at(self.address).connect(self.backend.clone()) }
}
