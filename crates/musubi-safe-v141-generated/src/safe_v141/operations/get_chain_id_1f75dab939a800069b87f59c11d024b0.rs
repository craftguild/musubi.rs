//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Return values for `getChainId()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetChainIdReturn {
    /// ABI field `#0`.
    pub value_0: Uint<256>,
}

impl IntoAbiValue for GetChainIdReturn {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.value_0.into_abi_value(),
    ]) }
}
impl FromAbiValue for GetChainIdReturn {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(GetChainIdReturn {
            value_0: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl musubi_runtime::DecodeReturns for GetChainIdReturn {
    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError> {
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(GetChainIdReturn {
            value_0: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl GetChainIdReturn {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "getChainId".into(), signature: "getChainId()".into(), selector: [52, 8, 228, 112], inputs: vec![], outputs: vec![AbiField { name: "".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }], state_mutability: StateMutability::View } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionCall;

impl SafeV141 {
    /// Builds `getChainId()` as an offline typed operation.
    #[must_use]
    pub fn get_chain_id() -> FunctionCall<(), GetChainIdReturn> { FunctionCall::new(abi::descriptor(), ()) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `getChainId()`.
    #[must_use]
    pub fn get_chain_id(&self, ) -> musubi_runtime::BoundCall<B, (), GetChainIdReturn> { SafeV141::get_chain_id().at(self.address).connect(self.backend.clone()) }
}
