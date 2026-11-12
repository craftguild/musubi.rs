//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Address, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `addOwnerWithThreshold(address,uint256)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddOwnerWithThresholdArgs {
    /// ABI field `owner`.
    pub owner: Address,
    /// ABI field `_threshold`.
    pub _threshold: Uint<256>,
}

impl IntoAbiValue for AddOwnerWithThresholdArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.owner.into_abi_value(),
        self._threshold.into_abi_value(),
    ]) }
}
impl FromAbiValue for AddOwnerWithThresholdArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 2 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(AddOwnerWithThresholdArgs {
            owner: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            _threshold: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl AddOwnerWithThresholdArgs {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "addOwnerWithThreshold".into(), signature: "addOwnerWithThreshold(address,uint256)".into(), selector: [13, 88, 47, 19], inputs: vec![AbiField { name: "owner".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, AbiField { name: "_threshold".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }], outputs: vec![], state_mutability: StateMutability::Nonpayable } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionTransaction;

impl SafeV141 {
    /// Builds `addOwnerWithThreshold(address,uint256)` as an offline typed operation.
    #[must_use]
    pub fn add_owner_with_threshold(arguments: AddOwnerWithThresholdArgs) -> FunctionTransaction<AddOwnerWithThresholdArgs> { FunctionTransaction::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `addOwnerWithThreshold(address,uint256)`.
    #[must_use]
    pub fn add_owner_with_threshold(&self, arguments: AddOwnerWithThresholdArgs) -> musubi_runtime::BoundTransaction<B, AddOwnerWithThresholdArgs> { SafeV141::add_owner_with_threshold(arguments).at(self.address).connect(self.backend.clone()) }
}
