//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `setGuard(address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetGuardArgs {
    /// ABI field `guard`.
    pub guard: Address,
}

impl IntoAbiValue for SetGuardArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.guard.into_abi_value(),
    ]) }
}
impl FromAbiValue for SetGuardArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(SetGuardArgs {
            guard: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl SetGuardArgs {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "setGuard".into(), signature: "setGuard(address)".into(), selector: [225, 154, 157, 217], inputs: vec![AbiField { name: "guard".into(), ty: AbiType::Address, internal_type: Some("address".into()) }], outputs: vec![], state_mutability: StateMutability::Nonpayable } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionTransaction;

impl SafeV141 {
    /// Builds `setGuard(address)` as an offline typed operation.
    #[must_use]
    pub fn set_guard(arguments: SetGuardArgs) -> FunctionTransaction<SetGuardArgs> { FunctionTransaction::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `setGuard(address)`.
    #[must_use]
    pub fn set_guard(&self, arguments: SetGuardArgs) -> musubi_runtime::BoundTransaction<B, SetGuardArgs> { SafeV141::set_guard(arguments).at(self.address).connect(self.backend.clone()) }
}
