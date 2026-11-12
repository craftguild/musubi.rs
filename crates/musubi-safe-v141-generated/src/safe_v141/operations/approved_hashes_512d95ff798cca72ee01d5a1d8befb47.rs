//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Address, FixedBytes, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `approvedHashes(address,bytes32)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovedHashesArgs {
    /// ABI field `#0`.
    pub value_0: Address,
    /// ABI field `#1`.
    pub value_1: FixedBytes<32>,
}

impl IntoAbiValue for ApprovedHashesArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.value_0.into_abi_value(),
        self.value_1.into_abi_value(),
    ]) }
}
impl FromAbiValue for ApprovedHashesArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 2 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(ApprovedHashesArgs {
            value_0: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            value_1: <FixedBytes<32> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl ApprovedHashesArgs {
    /// Encodes this value without a function selector using Solidity ABI tuple rules.
    pub fn abi_encode(self) -> Result<musubi_core::Bytes, CodecError> {
        musubi_runtime::encode_fields(&abi::descriptor().inputs, &<Self as musubi_runtime::EncodeArguments>::into_values(self))
    }
    /// Strictly decodes a selector-free Solidity ABI tuple value.
    pub fn abi_decode(data: &[u8]) -> Result<Self, CodecError> {
        <Self as musubi_runtime::FromAbiValue>::from_abi_value(AbiValue::Tuple(musubi_runtime::decode_fields(&abi::descriptor().inputs, data)?))
    }
}

/// Return values for `approvedHashes(address,bytes32)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovedHashesReturn {
    /// ABI field `#0`.
    pub value_0: Uint<256>,
}

impl IntoAbiValue for ApprovedHashesReturn {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.value_0.into_abi_value(),
    ]) }
}
impl FromAbiValue for ApprovedHashesReturn {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(ApprovedHashesReturn {
            value_0: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl musubi_runtime::DecodeReturns for ApprovedHashesReturn {
    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError> {
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(ApprovedHashesReturn {
            value_0: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl ApprovedHashesReturn {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "approvedHashes".into(), signature: "approvedHashes(address,bytes32)".into(), selector: [125, 131, 41, 116], inputs: vec![AbiField { name: "".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, AbiField { name: "".into(), ty: AbiType::FixedBytes(32), internal_type: Some("bytes32".into()) }], outputs: vec![AbiField { name: "".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }], state_mutability: StateMutability::View } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionCall;

impl SafeV141 {
    /// Builds `approvedHashes(address,bytes32)` as an offline typed operation.
    #[must_use]
    pub fn approved_hashes(arguments: ApprovedHashesArgs) -> FunctionCall<ApprovedHashesArgs, ApprovedHashesReturn> { FunctionCall::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `approvedHashes(address,bytes32)`.
    #[must_use]
    pub fn approved_hashes(&self, arguments: ApprovedHashesArgs) -> musubi_runtime::BoundCall<B, ApprovedHashesArgs, ApprovedHashesReturn> { SafeV141::approved_hashes(arguments).at(self.address).connect(self.backend.clone()) }
}
