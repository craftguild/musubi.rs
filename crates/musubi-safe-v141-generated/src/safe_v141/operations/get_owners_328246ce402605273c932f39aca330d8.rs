//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Return values for `getOwners()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetOwnersReturn {
    /// ABI field `#0`.
    pub value_0: Vec<Address>,
}

impl IntoAbiValue for GetOwnersReturn {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.value_0.into_abi_value(),
    ]) }
}
impl FromAbiValue for GetOwnersReturn {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(GetOwnersReturn {
            value_0: <Vec<Address> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl musubi_runtime::DecodeReturns for GetOwnersReturn {
    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError> {
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(GetOwnersReturn {
            value_0: <Vec<Address> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl GetOwnersReturn {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "getOwners".into(), signature: "getOwners()".into(), selector: [160, 230, 126, 43], inputs: vec![], outputs: vec![AbiField { name: "".into(), ty: AbiType::Array(Box::new(AbiType::Address)), internal_type: Some("address[]".into()) }], state_mutability: StateMutability::View } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionCall;

impl SafeV141 {
    /// Builds `getOwners()` as an offline typed operation.
    #[must_use]
    pub fn get_owners() -> FunctionCall<(), GetOwnersReturn> { FunctionCall::new(abi::descriptor(), ()) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `getOwners()`.
    #[must_use]
    pub fn get_owners(&self, ) -> musubi_runtime::BoundCall<B, (), GetOwnersReturn> { SafeV141::get_owners().at(self.address).connect(self.backend.clone()) }
}
