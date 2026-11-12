//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Bytes, FixedBytes, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `checkNSignatures(bytes32,bytes,bytes,uint256)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckNSignaturesArgs {
    /// ABI field `dataHash`.
    pub data_hash: FixedBytes<32>,
    /// ABI field `data`.
    pub data: Bytes,
    /// ABI field `signatures`.
    pub signatures: Bytes,
    /// ABI field `requiredSignatures`.
    pub required_signatures: Uint<256>,
}

impl IntoAbiValue for CheckNSignaturesArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.data_hash.into_abi_value(),
        self.data.into_abi_value(),
        self.signatures.into_abi_value(),
        self.required_signatures.into_abi_value(),
    ]) }
}
impl FromAbiValue for CheckNSignaturesArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 4 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(CheckNSignaturesArgs {
            data_hash: <FixedBytes<32> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            data: <Bytes as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            signatures: <Bytes as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            required_signatures: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl CheckNSignaturesArgs {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "checkNSignatures".into(), signature: "checkNSignatures(bytes32,bytes,bytes,uint256)".into(), selector: [18, 251, 104, 224], inputs: vec![AbiField { name: "dataHash".into(), ty: AbiType::FixedBytes(32), internal_type: Some("bytes32".into()) }, AbiField { name: "data".into(), ty: AbiType::Bytes, internal_type: Some("bytes".into()) }, AbiField { name: "signatures".into(), ty: AbiType::Bytes, internal_type: Some("bytes".into()) }, AbiField { name: "requiredSignatures".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }], outputs: vec![], state_mutability: StateMutability::View } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionCall;

impl SafeV141 {
    /// Builds `checkNSignatures(bytes32,bytes,bytes,uint256)` as an offline typed operation.
    #[must_use]
    pub fn check_n_signatures(arguments: CheckNSignaturesArgs) -> FunctionCall<CheckNSignaturesArgs, ()> { FunctionCall::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `checkNSignatures(bytes32,bytes,bytes,uint256)`.
    #[must_use]
    pub fn check_n_signatures(&self, arguments: CheckNSignaturesArgs) -> musubi_runtime::BoundCall<B, CheckNSignaturesArgs, ()> { SafeV141::check_n_signatures(arguments).at(self.address).connect(self.backend.clone()) }
}
