//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, FixedBytes, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `signedMessages(bytes32)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedMessagesArgs {
    /// ABI field `#0`.
    pub value_0: FixedBytes<32>,
}

impl IntoAbiValue for SignedMessagesArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.value_0.into_abi_value(),
    ]) }
}
impl FromAbiValue for SignedMessagesArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(SignedMessagesArgs {
            value_0: <FixedBytes<32> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl SignedMessagesArgs {
    /// Encodes this value without a function selector using Solidity ABI tuple rules.
    pub fn abi_encode(self) -> Result<musubi_core::Bytes, CodecError> {
        musubi_runtime::encode_fields(&abi::descriptor().inputs, &<Self as musubi_runtime::EncodeArguments>::into_values(self))
    }
    /// Strictly decodes a selector-free Solidity ABI tuple value.
    pub fn abi_decode(data: &[u8]) -> Result<Self, CodecError> {
        <Self as musubi_runtime::FromAbiValue>::from_abi_value(AbiValue::Tuple(musubi_runtime::decode_fields(&abi::descriptor().inputs, data)?))
    }
}

/// Return values for `signedMessages(bytes32)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedMessagesReturn {
    /// ABI field `#0`.
    pub value_0: Uint<256>,
}

impl IntoAbiValue for SignedMessagesReturn {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.value_0.into_abi_value(),
    ]) }
}
impl FromAbiValue for SignedMessagesReturn {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(SignedMessagesReturn {
            value_0: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl musubi_runtime::DecodeReturns for SignedMessagesReturn {
    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError> {
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(SignedMessagesReturn {
            value_0: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl SignedMessagesReturn {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "signedMessages".into(), signature: "signedMessages(bytes32)".into(), selector: [90, 230, 189, 55], inputs: vec![AbiField { name: "".into(), ty: AbiType::FixedBytes(32), internal_type: Some("bytes32".into()) }], outputs: vec![AbiField { name: "".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }], state_mutability: StateMutability::View } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionCall;

impl SafeV141 {
    /// Builds `signedMessages(bytes32)` as an offline typed operation.
    #[must_use]
    pub fn signed_messages(arguments: SignedMessagesArgs) -> FunctionCall<SignedMessagesArgs, SignedMessagesReturn> { FunctionCall::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `signedMessages(bytes32)`.
    #[must_use]
    pub fn signed_messages(&self, arguments: SignedMessagesArgs) -> musubi_runtime::BoundCall<B, SignedMessagesArgs, SignedMessagesReturn> { SafeV141::signed_messages(arguments).at(self.address).connect(self.backend.clone()) }
}
