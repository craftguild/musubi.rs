//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Bytes, FixedBytes};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `checkSignatures(bytes32,bytes,bytes)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckSignaturesArgs {
    /// ABI field `dataHash`.
    pub data_hash: FixedBytes<32>,
    /// ABI field `data`.
    pub data: Bytes,
    /// ABI field `signatures`.
    pub signatures: Bytes,
}

impl IntoAbiValue for CheckSignaturesArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.data_hash.into_abi_value(),
        self.data.into_abi_value(),
        self.signatures.into_abi_value(),
    ]) }
}
impl FromAbiValue for CheckSignaturesArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 3 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(CheckSignaturesArgs {
            data_hash: <FixedBytes<32> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            data: <Bytes as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            signatures: <Bytes as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl CheckSignaturesArgs {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "checkSignatures".into(), signature: "checkSignatures(bytes32,bytes,bytes)".into(), selector: [147, 79, 58, 17], inputs: vec![AbiField { name: "dataHash".into(), ty: AbiType::FixedBytes(32), internal_type: Some("bytes32".into()) }, AbiField { name: "data".into(), ty: AbiType::Bytes, internal_type: Some("bytes".into()) }, AbiField { name: "signatures".into(), ty: AbiType::Bytes, internal_type: Some("bytes".into()) }], outputs: vec![], state_mutability: StateMutability::View } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionCall;

impl SafeV141 {
    /// Builds `checkSignatures(bytes32,bytes,bytes)` as an offline typed operation.
    #[must_use]
    pub fn check_signatures(arguments: CheckSignaturesArgs) -> FunctionCall<CheckSignaturesArgs, ()> { FunctionCall::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `checkSignatures(bytes32,bytes,bytes)`.
    #[must_use]
    pub fn check_signatures(&self, arguments: CheckSignaturesArgs) -> musubi_runtime::BoundCall<B, CheckSignaturesArgs, ()> { SafeV141::check_signatures(arguments).at(self.address).connect(self.backend.clone()) }
}
