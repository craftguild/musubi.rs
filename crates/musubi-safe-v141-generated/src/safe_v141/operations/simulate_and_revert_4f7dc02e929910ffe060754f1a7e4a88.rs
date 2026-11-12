//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue, Address, Bytes};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Arguments for `simulateAndRevert(address,bytes)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimulateAndRevertArgs {
    /// ABI field `targetContract`.
    pub target_contract: Address,
    /// ABI field `calldataPayload`.
    pub calldata_payload: Bytes,
}

impl IntoAbiValue for SimulateAndRevertArgs {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.target_contract.into_abi_value(),
        self.calldata_payload.into_abi_value(),
    ]) }
}
impl FromAbiValue for SimulateAndRevertArgs {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 2 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(SimulateAndRevertArgs {
            target_contract: <Address as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
            calldata_payload: <Bytes as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl SimulateAndRevertArgs {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "simulateAndRevert".into(), signature: "simulateAndRevert(address,bytes)".into(), selector: [180, 250, 186, 9], inputs: vec![AbiField { name: "targetContract".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, AbiField { name: "calldataPayload".into(), ty: AbiType::Bytes, internal_type: Some("bytes".into()) }], outputs: vec![], state_mutability: StateMutability::Nonpayable } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionTransaction;

impl SafeV141 {
    /// Builds `simulateAndRevert(address,bytes)` as an offline typed operation.
    #[must_use]
    pub fn simulate_and_revert(arguments: SimulateAndRevertArgs) -> FunctionTransaction<SimulateAndRevertArgs> { FunctionTransaction::new(abi::descriptor(), arguments) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `simulateAndRevert(address,bytes)`.
    #[must_use]
    pub fn simulate_and_revert(&self, arguments: SimulateAndRevertArgs) -> musubi_runtime::BoundTransaction<B, SimulateAndRevertArgs> { SafeV141::simulate_and_revert(arguments).at(self.address).connect(self.backend.clone()) }
}
