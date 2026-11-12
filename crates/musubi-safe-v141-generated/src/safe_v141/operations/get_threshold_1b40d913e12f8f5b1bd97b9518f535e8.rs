//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Return values for `getThreshold()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetThresholdReturn {
    /// ABI field `#0`.
    pub value_0: Uint<256>,
}

impl IntoAbiValue for GetThresholdReturn {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.value_0.into_abi_value(),
    ]) }
}
impl FromAbiValue for GetThresholdReturn {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(GetThresholdReturn {
            value_0: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl musubi_runtime::DecodeReturns for GetThresholdReturn {
    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError> {
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(GetThresholdReturn {
            value_0: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl GetThresholdReturn {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "getThreshold".into(), signature: "getThreshold()".into(), selector: [231, 82, 53, 184], inputs: vec![], outputs: vec![AbiField { name: "".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }], state_mutability: StateMutability::View } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionCall;

impl SafeV141 {
    /// Builds `getThreshold()` as an offline typed operation.
    #[must_use]
    pub fn get_threshold() -> FunctionCall<(), GetThresholdReturn> { FunctionCall::new(abi::descriptor(), ()) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `getThreshold()`.
    #[must_use]
    pub fn get_threshold(&self, ) -> musubi_runtime::BoundCall<B, (), GetThresholdReturn> { SafeV141::get_threshold().at(self.address).connect(self.backend.clone()) }
}
