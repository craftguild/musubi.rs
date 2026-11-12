//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `changeThreshold(uint256)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeThresholdArgs {
    /// ABI field `_threshold`.
    pub _threshold: Uint<256>,
}

impl IntoAbiValue for ChangeThresholdArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self._threshold.into_abi_value(),
    ]) }
}
impl FromAbiValue for ChangeThresholdArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(ChangeThresholdArgs {
            _threshold: <Uint<256> as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl ChangeThresholdArgs {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "changeThreshold".into(), signature: "changeThreshold(uint256)".into(), selector: [105, 78, 128, 195], inputs: vec![AbiField { name: "_threshold".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }], outputs: vec![], state_mutability: StateMutability::Nonpayable } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionTransaction;

impl SafeV141 {
    /// Builds `changeThreshold(uint256)` as an offline typed operation.
    #[must_use]
    pub fn change_threshold(arguments: ChangeThresholdArgs) -> FunctionTransaction<ChangeThresholdArgs> { FunctionTransaction::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `changeThreshold(uint256)`.
    #[must_use]
    pub fn change_threshold(&self, arguments: ChangeThresholdArgs) -> musubi_runtime::BoundTransaction<B, ChangeThresholdArgs> { SafeV141::change_threshold(arguments).at(self.address).connect(self.backend.clone()) }
}
