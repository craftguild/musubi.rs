//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `enableModule(address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnableModuleArgs {
    /// ABI field `module`.
    pub module: Address,
}

impl IntoAbiValue for EnableModuleArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.module.into_abi_value(),
    ]) }
}
impl FromAbiValue for EnableModuleArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(EnableModuleArgs {
            module: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl EnableModuleArgs {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "enableModule".into(), signature: "enableModule(address)".into(), selector: [97, 11, 89, 37], inputs: vec![AbiField { name: "module".into(), ty: AbiType::Address, internal_type: Some("address".into()) }], outputs: vec![], state_mutability: StateMutability::Nonpayable } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionTransaction;

impl SafeV141 {
    /// Builds `enableModule(address)` as an offline typed operation.
    #[must_use]
    pub fn enable_module(arguments: EnableModuleArgs) -> FunctionTransaction<EnableModuleArgs> { FunctionTransaction::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `enableModule(address)`.
    #[must_use]
    pub fn enable_module(&self, arguments: EnableModuleArgs) -> musubi_runtime::BoundTransaction<B, EnableModuleArgs> { SafeV141::enable_module(arguments).at(self.address).connect(self.backend.clone()) }
}
