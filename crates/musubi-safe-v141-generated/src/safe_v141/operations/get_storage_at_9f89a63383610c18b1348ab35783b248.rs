//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Bytes, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `getStorageAt(uint256,uint256)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetStorageAtArgs {
    /// ABI field `offset`.
    pub offset: Uint<256>,
    /// ABI field `length`.
    pub length: Uint<256>,
}

impl IntoAbiValue for GetStorageAtArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.offset.into_abi_value(),
        self.length.into_abi_value(),
    ]) }
}
impl FromAbiValue for GetStorageAtArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 2 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(GetStorageAtArgs {
            offset: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            length: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl GetStorageAtArgs {
    /// Encodes this value without a function selector using Solidity ABI tuple rules.
    pub fn abi_encode(self) -> Result<musubi_core::Bytes, CodecError> {
        musubi_runtime::encode_fields(&abi::descriptor().inputs, &<Self as musubi_runtime::EncodeArguments>::into_values(self))
    }
    /// Strictly decodes a selector-free Solidity ABI tuple value.
    pub fn abi_decode(data: &[u8]) -> Result<Self, CodecError> {
        <Self as musubi_runtime::FromAbiValue>::from_abi_value(AbiValue::Tuple(musubi_runtime::decode_fields(&abi::descriptor().inputs, data)?))
    }
}

/// Return values for `getStorageAt(uint256,uint256)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetStorageAtReturn {
    /// ABI field `#0`.
    pub value_0: Bytes,
}

impl IntoAbiValue for GetStorageAtReturn {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.value_0.into_abi_value(),
    ]) }
}
impl FromAbiValue for GetStorageAtReturn {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(GetStorageAtReturn {
            value_0: <Bytes as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl musubi_runtime::DecodeReturns for GetStorageAtReturn {
    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError> {
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(GetStorageAtReturn {
            value_0: <Bytes as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl GetStorageAtReturn {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "getStorageAt".into(), signature: "getStorageAt(uint256,uint256)".into(), selector: [86, 36, 178, 91], inputs: vec![AbiField { name: "offset".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }, AbiField { name: "length".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }], outputs: vec![AbiField { name: "".into(), ty: AbiType::Bytes, internal_type: Some("bytes".into()) }], state_mutability: StateMutability::View } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionCall;

impl SafeV141 {
    /// Builds `getStorageAt(uint256,uint256)` as an offline typed operation.
    #[must_use]
    pub fn get_storage_at(arguments: GetStorageAtArgs) -> FunctionCall<GetStorageAtArgs, GetStorageAtReturn> { FunctionCall::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `getStorageAt(uint256,uint256)`.
    #[must_use]
    pub fn get_storage_at(&self, arguments: GetStorageAtArgs) -> musubi_runtime::BoundCall<B, GetStorageAtArgs, GetStorageAtReturn> { SafeV141::get_storage_at(arguments).at(self.address).connect(self.backend.clone()) }
}
