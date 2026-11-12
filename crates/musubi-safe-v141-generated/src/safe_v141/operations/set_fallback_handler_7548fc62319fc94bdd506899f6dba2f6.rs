//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `setFallbackHandler(address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetFallbackHandlerArgs {
    /// ABI field `handler`.
    pub handler: Address,
}

impl IntoAbiValue for SetFallbackHandlerArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.handler.into_abi_value(),
    ]) }
}
impl FromAbiValue for SetFallbackHandlerArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(SetFallbackHandlerArgs {
            handler: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl SetFallbackHandlerArgs {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "setFallbackHandler".into(), signature: "setFallbackHandler(address)".into(), selector: [240, 138, 3, 35], inputs: vec![AbiField { name: "handler".into(), ty: AbiType::Address, internal_type: Some("address".into()) }], outputs: vec![], state_mutability: StateMutability::Nonpayable } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionTransaction;

impl SafeV141 {
    /// Builds `setFallbackHandler(address)` as an offline typed operation.
    #[must_use]
    pub fn set_fallback_handler(arguments: SetFallbackHandlerArgs) -> FunctionTransaction<SetFallbackHandlerArgs> { FunctionTransaction::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `setFallbackHandler(address)`.
    #[must_use]
    pub fn set_fallback_handler(&self, arguments: SetFallbackHandlerArgs) -> musubi_runtime::BoundTransaction<B, SetFallbackHandlerArgs> { SafeV141::set_fallback_handler(arguments).at(self.address).connect(self.backend.clone()) }
}
