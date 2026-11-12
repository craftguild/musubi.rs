//! One event's owned value, descriptor, and strict log codec.

use musubi_core::{Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed event `RemovedOwner(address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemovedOwner {
    /// ABI event field `owner`.
    pub owner: Address,
}

impl RemovedOwner {
    /// Decodes one log with strict signature, topic-count, and canonical-data checks.
    pub fn decode(log: &musubi_core::Log) -> Result<Self, CodecError> {
        let values = musubi_runtime::decode_event(&descriptor(), log)?;
        let mut values = values.into_iter();
        Ok(Self {
            owner: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <Address as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
        })
    }
    /// Encodes this value as a deterministic metadata-free fixture log.
    pub fn encode_log(&self, address: musubi_core::Address) -> Result<musubi_core::Log, CodecError> {
        let values = vec![
            musubi_runtime::DecodedEventField::Value(self.owner.into_abi_value()),
        ];
        musubi_runtime::encode_event(&descriptor(), address, &values)
    }
}

pub(super) fn descriptor() -> musubi_core::EventDescriptor {
    use musubi_core::*;
EventDescriptor { name: "RemovedOwner".into(), signature: "RemovedOwner(address)".into(), topic: Some([248, 212, 159, 197, 41, 129, 46, 154, 124, 92, 80, 230, 156, 32, 240, 220, 204, 13, 184, 250, 149, 201, 139, 197, 140, 201, 164, 241, 193, 41, 158, 175]), inputs: vec![EventField { field: AbiField { name: "owner".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, indexed: true }], anonymous: false } }
