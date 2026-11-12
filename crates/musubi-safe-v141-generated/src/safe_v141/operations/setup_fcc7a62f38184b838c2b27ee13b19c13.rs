//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Address, Bytes, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `setup(address[],uint256,address,bytes,address,address,uint256,address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupArgs {
    /// ABI field `_owners`.
    pub _owners: Vec<Address>,
    /// ABI field `_threshold`.
    pub _threshold: Uint<256>,
    /// ABI field `to`.
    pub to: Address,
    /// ABI field `data`.
    pub data: Bytes,
    /// ABI field `fallbackHandler`.
    pub fallback_handler: Address,
    /// ABI field `paymentToken`.
    pub payment_token: Address,
    /// ABI field `payment`.
    pub payment: Uint<256>,
    /// ABI field `paymentReceiver`.
    pub payment_receiver: Address,
}

impl IntoAbiValue for SetupArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self._owners.into_abi_value(),
        self._threshold.into_abi_value(),
        self.to.into_abi_value(),
        self.data.into_abi_value(),
        self.fallback_handler.into_abi_value(),
        self.payment_token.into_abi_value(),
        self.payment.into_abi_value(),
        self.payment_receiver.into_abi_value(),
    ]) }
}
impl FromAbiValue for SetupArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 8 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(SetupArgs {
            _owners: <Vec<Address> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            _threshold: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            to: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            data: <Bytes as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            fallback_handler: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            payment_token: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            payment: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            payment_receiver: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl SetupArgs {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "setup".into(), signature: "setup(address[],uint256,address,bytes,address,address,uint256,address)".into(), selector: [182, 62, 128, 13], inputs: vec![AbiField { name: "_owners".into(), ty: AbiType::Array(Box::new(AbiType::Address)), internal_type: Some("address[]".into()) }, AbiField { name: "_threshold".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }, AbiField { name: "to".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, AbiField { name: "data".into(), ty: AbiType::Bytes, internal_type: Some("bytes".into()) }, AbiField { name: "fallbackHandler".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, AbiField { name: "paymentToken".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, AbiField { name: "payment".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }, AbiField { name: "paymentReceiver".into(), ty: AbiType::Address, internal_type: Some("address payable".into()) }], outputs: vec![], state_mutability: StateMutability::Nonpayable } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionTransaction;

impl SafeV141 {
    /// Builds `setup(address[],uint256,address,bytes,address,address,uint256,address)` as an offline typed operation.
    #[must_use]
    pub fn setup(arguments: SetupArgs) -> FunctionTransaction<SetupArgs> { FunctionTransaction::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `setup(address[],uint256,address,bytes,address,address,uint256,address)`.
    #[must_use]
    pub fn setup(&self, arguments: SetupArgs) -> musubi_runtime::BoundTransaction<B, SetupArgs> { SafeV141::setup(arguments).at(self.address).connect(self.backend.clone()) }
}
