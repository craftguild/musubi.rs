//! One function's owned arguments, returns, codecs, descriptor, and operations.

use musubi_core::{AbiValue};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Return values for `VERSION()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VERSIONReturn {
    /// ABI field `#0`.
    pub value_0: String,
}

impl IntoAbiValue for VERSIONReturn {
    fn into_abi_value(self) -> AbiValue { AbiValue::Tuple(vec![
        self.value_0.into_abi_value(),
    ]) }
}
impl FromAbiValue for VERSIONReturn {
    fn from_abi_value(value: AbiValue) -> Result<Self, CodecError> {
        let AbiValue::Tuple(values) = value else { return Err(CodecError::TypeMismatch); };
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(VERSIONReturn {
            value_0: <String as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl musubi_runtime::DecodeReturns for VERSIONReturn {
    fn from_values(values: Vec<AbiValue>) -> Result<Self, CodecError> {
        if values.len() != 1 { return Err(CodecError::Arity); }
        let mut values = values.into_iter();
        Ok(VERSIONReturn {
            value_0: <String as FromAbiValue>::from_abi_value(values.next().ok_or(CodecError::Arity)?)?,
        })
    }
}

impl VERSIONReturn {
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
pub(super) fn descriptor() -> FunctionDescriptor { FunctionDescriptor { name: "VERSION".into(), signature: "VERSION()".into(), selector: [255, 161, 173, 116], inputs: vec![], outputs: vec![AbiField { name: "".into(), ty: AbiType::String, internal_type: Some("string".into()) }], state_mutability: StateMutability::View } }
}
use super::{SafeV141, SafeV141Client};
use musubi_runtime::FunctionCall;

impl SafeV141 {
    /// Builds `VERSION()` as an offline typed operation.
    #[must_use]
    pub fn v_e_r_s_i_o_n() -> FunctionCall<(), VERSIONReturn> { FunctionCall::new(abi::descriptor(), ()) }
}

impl<B: Clone> SafeV141Client<B> {
    /// Builds backend-bound `VERSION()`.
    #[must_use]
    pub fn v_e_r_s_i_o_n(&self, ) -> musubi_runtime::BoundCall<B, (), VERSIONReturn> { SafeV141::v_e_r_s_i_o_n().at(self.address).connect(self.backend.clone()) }
}
