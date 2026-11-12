//! One event's owned value, descriptor, and strict log codec.

use musubi_core::{Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed event `ExecutionFromModuleSuccess(address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionFromModuleSuccess {
    /// ABI event field `module`.
    pub module: Address,
}

impl ExecutionFromModuleSuccess {
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
EventDescriptor { name: "ExecutionFromModuleSuccess".into(), signature: "ExecutionFromModuleSuccess(address)".into(), topic: Some([104, 149, 193, 54, 100, 170, 79, 103, 40, 139, 37, 215, 162, 29, 122, 170, 52, 145, 110, 53, 95, 185, 182, 250, 224, 161, 57, 169, 8, 91, 236, 184]), inputs: vec![EventField { field: AbiField { name: "module".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, indexed: true }], anonymous: false } }
