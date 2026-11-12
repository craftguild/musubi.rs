//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Address, Bytes, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `execTransaction(address,uint256,bytes,uint8,uint256,uint256,uint256,address,address,bytes)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecTransactionArgs {
    /// ABI field `to`.
    pub to: Address,
    /// ABI field `value`.
    pub value: Uint<256>,
    /// ABI field `data`.
    pub data: Bytes,
    /// ABI field `operation`.
    pub operation: Uint<8>,
    /// ABI field `safeTxGas`.
    pub safe_tx_gas: Uint<256>,
    /// ABI field `baseGas`.
    pub base_gas: Uint<256>,
    /// ABI field `gasPrice`.
    pub gas_price: Uint<256>,
    /// ABI field `gasToken`.
    pub gas_token: Address,
    /// ABI field `refundReceiver`.
    pub refund_receiver: Address,
    /// ABI field `signatures`.
    pub signatures: Bytes,
}

impl IntoAbiValue for ExecTransactionArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.to.into_abi_value(),
        self.value.into_abi_value(),
        self.data.into_abi_value(),
        self.operation.into_abi_value(),
        self.safe_tx_gas.into_abi_value(),
        self.base_gas.into_abi_value(),
        self.gas_price.into_abi_value(),
        self.gas_token.into_abi_value(),
        self.refund_receiver.into_abi_value(),
        self.signatures.into_abi_value(),
    ]) }
}
impl FromAbiValue for ExecTransactionArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 10 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(ExecTransactionArgs {
            to: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            value: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            data: <Bytes as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            operation: <Uint<8> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            safe_tx_gas: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            base_gas: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            gas_price: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            gas_token: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            refund_receiver: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            signatures: <Bytes as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl ExecTransactionArgs {
    /// Encodes this value without a function selector using Solidity ABI tuple rules.
    pub fn abi_encode(self) -> Result<musubi_core::Bytes, CodecError> {
        musubi_runtime::encode_fields(&abi::descriptor().inputs, &<Self as musubi_runtime::EncodeArguments>::into_values(self))
    }
    /// Strictly decodes a selector-free Solidity ABI tuple value.
    pub fn abi_decode(data: &[u8]) -> Result<Self, CodecError> {
        <Self as musubi_runtime::FromAbiValue>::from_abi_value(AbiValue::Tuple(musubi_runtime::decode_fields(&abi::descriptor().inputs, data)?))
    }
}

/// Return values for `execTransaction(address,uint256,bytes,uint8,uint256,uint256,uint256,address,address,bytes)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecTransactionReturn {
    /// ABI field `success`.
    pub success: bool,
}

impl IntoAbiValue for ExecTransactionReturn {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.success.into_abi_value(),
    ]) }
}
impl FromAbiValue for ExecTransactionReturn {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(ExecTransactionReturn {
            success: <bool as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl musubi_runtime::DecodeReturns for ExecTransactionReturn {
    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError> {
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(ExecTransactionReturn {
            success: <bool as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl ExecTransactionReturn {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "execTransaction".into(), signature: "execTransaction(address,uint256,bytes,uint8,uint256,uint256,uint256,address,address,bytes)".into(), selector: [106, 118, 18, 2], inputs: vec![AbiField { name: "to".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, AbiField { name: "value".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }, AbiField { name: "data".into(), ty: AbiType::Bytes, internal_type: Some("bytes".into()) }, AbiField { name: "operation".into(), ty: AbiType::Uint(8), internal_type: Some("enum Enum.Operation".into()) }, AbiField { name: "safeTxGas".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }, AbiField { name: "baseGas".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }, AbiField { name: "gasPrice".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }, AbiField { name: "gasToken".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, AbiField { name: "refundReceiver".into(), ty: AbiType::Address, internal_type: Some("address payable".into()) }, AbiField { name: "signatures".into(), ty: AbiType::Bytes, internal_type: Some("bytes".into()) }], outputs: vec![AbiField { name: "success".into(), ty: AbiType::Bool, internal_type: Some("bool".into()) }], state_mutability: StateMutability::Payable } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionTransaction;

impl SafeV141 {
    /// Builds `execTransaction(address,uint256,bytes,uint8,uint256,uint256,uint256,address,address,bytes)` as an offline typed operation.
    #[must_use]
    pub fn exec_transaction(arguments: ExecTransactionArgs) -> FunctionTransaction<ExecTransactionArgs> { FunctionTransaction::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `execTransaction(address,uint256,bytes,uint8,uint256,uint256,uint256,address,address,bytes)`.
    #[must_use]
    pub fn exec_transaction(&self, arguments: ExecTransactionArgs) -> musubi_runtime::BoundTransaction<B, ExecTransactionArgs> { SafeV141::exec_transaction(arguments).at(self.address).connect(self.backend.clone()) }
}
