//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, FixedBytes};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `approveHash(bytes32)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApproveHashArgs {
    /// ABI field `hashToApprove`.
    pub hash_to_approve: FixedBytes<32>,
}

impl IntoAbiValue for ApproveHashArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.hash_to_approve.into_abi_value(),
    ]) }
}
impl FromAbiValue for ApproveHashArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(ApproveHashArgs {
            hash_to_approve: <FixedBytes<32> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl ApproveHashArgs {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "approveHash".into(), signature: "approveHash(bytes32)".into(), selector: [212, 217, 189, 205], inputs: vec![AbiField { name: "hashToApprove".into(), ty: AbiType::FixedBytes(32), internal_type: Some("bytes32".into()) }], outputs: vec![], state_mutability: StateMutability::Nonpayable } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionTransaction;

impl SafeV141 {
    /// Builds `approveHash(bytes32)` as an offline typed operation.
    #[must_use]
    pub fn approve_hash(arguments: ApproveHashArgs) -> FunctionTransaction<ApproveHashArgs> { FunctionTransaction::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `approveHash(bytes32)`.
    #[must_use]
    pub fn approve_hash(&self, arguments: ApproveHashArgs) -> musubi_runtime::BoundTransaction<B, ApproveHashArgs> { SafeV141::approve_hash(arguments).at(self.address).connect(self.backend.clone()) }
}
