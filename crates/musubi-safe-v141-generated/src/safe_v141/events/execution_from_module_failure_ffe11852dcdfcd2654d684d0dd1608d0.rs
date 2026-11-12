//! One event's owned value, descriptor, and strict log codec.

use musubi_core::{Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed event `ExecutionFromModuleFailure(address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionFromModuleFailure {
    /// ABI event field `module`.
    pub module: Address,
}

impl ExecutionFromModuleFailure {
    /// Decodes one log with strict signature, topic-count, and canonical-data checks.
    pub fn decode(log: &musubi_core::Log) -> Result<Self, CodecError> {
        let values = musubi_runtime::decode_event(&descriptor(), log)?;
        let mut values = values.into_iter();
        Ok(Self {
            module: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <Address as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
        })
    }
    /// Encodes this value as a deterministic metadata-free fixture log.
    pub fn encode_log(&self, address: musubi_core::Address) -> Result<musubi_core::Log, CodecError> {
        let values = vec![
            musubi_runtime::DecodedEventField::Value(self.module.into_abi_value()),
        ];
        musubi_runtime::encode_event(&descriptor(), address, &values)
    }
}

pub(super) fn descriptor() -> musubi_core::EventDescriptor {
    use musubi_core::*;
EventDescriptor { name: "ExecutionFromModuleFailure".into(), signature: "ExecutionFromModuleFailure(address)".into(), topic: Some([172, 210, 200, 112, 40, 4, 18, 143, 219, 13, 178, 187, 73, 246, 209, 39, 221, 1, 129, 193, 63, 212, 93, 191, 225, 109, 224, 147, 14, 43, 211, 117]), inputs: vec![EventField { field: AbiField { name: "module".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, indexed: true }], anonymous: false } }
