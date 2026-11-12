//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Address, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `getModulesPaginated(address,uint256)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetModulesPaginatedArgs {
    /// ABI field `start`.
    pub start: Address,
    /// ABI field `pageSize`.
    pub page_size: Uint<256>,
}

impl IntoAbiValue for GetModulesPaginatedArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.start.into_abi_value(),
        self.page_size.into_abi_value(),
    ]) }
}
impl FromAbiValue for GetModulesPaginatedArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 2 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(GetModulesPaginatedArgs {
            start: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            page_size: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl GetModulesPaginatedArgs {
    /// Encodes this value without a function selector using Solidity ABI tuple rules.
    pub fn abi_encode(self) -> Result<musubi_core::Bytes, CodecError> {
        musubi_runtime::encode_fields(&abi::descriptor().inputs, &<Self as musubi_runtime::EncodeArguments>::into_values(self))
    }
    /// Strictly decodes a selector-free Solidity ABI tuple value.
    pub fn abi_decode(data: &[u8]) -> Result<Self, CodecError> {
        <Self as musubi_runtime::FromAbiValue>::from_abi_value(AbiValue::Tuple(musubi_runtime::decode_fields(&abi::descriptor().inputs, data)?))
    }
}

/// Return values for `getModulesPaginated(address,uint256)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetModulesPaginatedReturn {
    /// ABI field `array`.
    pub array: Vec<Address>,
    /// ABI field `next`.
    pub next: Address,
}

impl IntoAbiValue for GetModulesPaginatedReturn {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.array.into_abi_value(),
        self.next.into_abi_value(),
    ]) }
}
impl FromAbiValue for GetModulesPaginatedReturn {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 2 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(GetModulesPaginatedReturn {
            array: <Vec<Address> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            next: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl musubi_runtime::DecodeReturns for GetModulesPaginatedReturn {
    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError> {
        if values.len() != 2 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(GetModulesPaginatedReturn {
            array: <Vec<Address> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            next: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl GetModulesPaginatedReturn {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "getModulesPaginated".into(), signature: "getModulesPaginated(address,uint256)".into(), selector: [204, 47, 132, 82], inputs: vec![AbiField { name: "start".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, AbiField { name: "pageSize".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }], outputs: vec![AbiField { name: "array".into(), ty: AbiType::Array(Box::new(AbiType::Address)), internal_type: Some("address[]".into()) }, AbiField { name: "next".into(), ty: AbiType::Address, internal_type: Some("address".into()) }], state_mutability: StateMutability::View } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionCall;

impl SafeV141 {
    /// Builds `getModulesPaginated(address,uint256)` as an offline typed operation.
    #[must_use]
    pub fn get_modules_paginated(arguments: GetModulesPaginatedArgs) -> FunctionCall<GetModulesPaginatedArgs, GetModulesPaginatedReturn> { FunctionCall::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `getModulesPaginated(address,uint256)`.
    #[must_use]
    pub fn get_modules_paginated(&self, arguments: GetModulesPaginatedArgs) -> musubi_runtime::BoundCall<B, GetModulesPaginatedArgs, GetModulesPaginatedReturn> { SafeV141::get_modules_paginated(arguments).at(self.address).connect(self.backend.clone()) }
}
