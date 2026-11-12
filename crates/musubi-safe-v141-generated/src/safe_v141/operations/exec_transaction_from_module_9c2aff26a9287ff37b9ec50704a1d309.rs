//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Address, Bytes, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `execTransactionFromModule(address,uint256,bytes,uint8)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecTransactionFromModuleArgs {
    /// ABI field `to`.
    pub to: Address,
    /// ABI field `value`.
    pub value: Uint<256>,
    /// ABI field `data`.
    pub data: Bytes,
    /// ABI field `operation`.
    pub operation: Uint<8>,
}

impl IntoAbiValue for ExecTransactionFromModuleArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.to.into_abi_value(),
        self.value.into_abi_value(),
        self.data.into_abi_value(),
        self.operation.into_abi_value(),
    ]) }
}
impl FromAbiValue for ExecTransactionFromModuleArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 4 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(ExecTransactionFromModuleArgs {
            to: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            value: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            data: <Bytes as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            operation: <Uint<8> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl ExecTransactionFromModuleArgs {
    /// Encodes this value without a function selector using Solidity ABI tuple rules.
    pub fn abi_encode(self) -> Result<musubi_core::Bytes, CodecError> {
        musubi_runtime::encode_fields(&abi::descriptor().inputs, &<Self as musubi_runtime::EncodeArguments>::into_values(self))
    }
    /// Strictly decodes a selector-free Solidity ABI tuple value.
    pub fn abi_decode(data: &[u8]) -> Result<Self, CodecError> {
        <Self as musubi_runtime::FromAbiValue>::from_abi_value(AbiValue::Tuple(musubi_runtime::decode_fields(&abi::descriptor().inputs, data)?))
    }
}

/// Return values for `execTransactionFromModule(address,uint256,bytes,uint8)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecTransactionFromModuleReturn {
    /// ABI field `success`.
    pub success: bool,
}

impl IntoAbiValue for ExecTransactionFromModuleReturn {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.success.into_abi_value(),
    ]) }
}
impl FromAbiValue for ExecTransactionFromModuleReturn {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(ExecTransactionFromModuleReturn {
            success: <bool as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl musubi_runtime::DecodeReturns for ExecTransactionFromModuleReturn {
    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError> {
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(ExecTransactionFromModuleReturn {
            success: <bool as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl ExecTransactionFromModuleReturn {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "execTransactionFromModule".into(), signature: "execTransactionFromModule(address,uint256,bytes,uint8)".into(), selector: [70, 135, 33, 167], inputs: vec![AbiField { name: "to".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, AbiField { name: "value".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }, AbiField { name: "data".into(), ty: AbiType::Bytes, internal_type: Some("bytes".into()) }, AbiField { name: "operation".into(), ty: AbiType::Uint(8), internal_type: Some("enum Enum.Operation".into()) }], outputs: vec![AbiField { name: "success".into(), ty: AbiType::Bool, internal_type: Some("bool".into()) }], state_mutability: StateMutability::Nonpayable } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionTransaction;

impl SafeV141 {
    /// Builds `execTransactionFromModule(address,uint256,bytes,uint8)` as an offline typed operation.
    #[must_use]
    pub fn exec_transaction_from_module(arguments: ExecTransactionFromModuleArgs) -> FunctionTransaction<ExecTransactionFromModuleArgs> { FunctionTransaction::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `execTransactionFromModule(address,uint256,bytes,uint8)`.
    #[must_use]
    pub fn exec_transaction_from_module(&self, arguments: ExecTransactionFromModuleArgs) -> musubi_runtime::BoundTransaction<B, ExecTransactionFromModuleArgs> { SafeV141::exec_transaction_from_module(arguments).at(self.address).connect(self.backend.clone()) }
}
