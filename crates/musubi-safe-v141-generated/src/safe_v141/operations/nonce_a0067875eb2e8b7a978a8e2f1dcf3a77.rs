//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Return values for `nonce()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NonceReturn {
    /// ABI field `#0`.
    pub value_0: Uint<256>,
}

impl IntoAbiValue for NonceReturn {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.value_0.into_abi_value(),
    ]) }
}
impl FromAbiValue for NonceReturn {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(NonceReturn {
            value_0: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl musubi_runtime::DecodeReturns for NonceReturn {
    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError> {
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(NonceReturn {
            value_0: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl NonceReturn {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "nonce".into(), signature: "nonce()".into(), selector: [175, 254, 208, 224], inputs: vec![], outputs: vec![AbiField { name: "".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }], state_mutability: StateMutability::View } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionCall;

impl SafeV141 {
    /// Builds `nonce()` as an offline typed operation.
    #[must_use]
    pub fn nonce() -> FunctionCall<(), NonceReturn> { FunctionCall::new(abi::descriptor(), ()) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `nonce()`.
    #[must_use]
    pub fn nonce(&self, ) -> musubi_runtime::BoundCall<B, (), NonceReturn> { SafeV141::nonce().at(self.address).connect(self.backend.clone()) }
}
