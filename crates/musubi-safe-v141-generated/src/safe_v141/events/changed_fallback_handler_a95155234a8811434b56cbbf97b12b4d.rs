//! One event's owned value, descriptor, and strict log codec.

use musubi_core::{Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed event `ChangedFallbackHandler(address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangedFallbackHandler {
    /// ABI event field `handler`.
    pub handler: Address,
}

impl ChangedFallbackHandler {
    /// Decodes one log with strict signature, topic-count, and canonical-data checks.
    pub fn decode(log: &musubi_core::Log) -> Result<Self, CodecError> {
        let values = musubi_runtime::decode_event(&descriptor(), log)?;
        let mut values = values.into_iter();
        Ok(Self {
            handler: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <Address as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
        })
    }
    /// Encodes this value as a deterministic metadata-free fixture log.
    pub fn encode_log(&self, address: musubi_core::Address) -> Result<musubi_core::Log, CodecError> {
        let values = vec![
            musubi_runtime::DecodedEventField::Value(self.handler.into_abi_value()),
        ];
        musubi_runtime::encode_event(&descriptor(), address, &values)
    }
}

pub(super) fn descriptor() -> musubi_core::EventDescriptor {
    use musubi_core::*;
EventDescriptor { name: "ChangedFallbackHandler".into(), signature: "ChangedFallbackHandler(address)".into(), topic: Some([90, 198, 196, 108, 147, 200, 208, 229, 55, 20, 186, 59, 83, 219, 62, 124, 4, 109, 169, 148, 49, 61, 126, 208, 209, 146, 2, 139, 199, 194, 40, 176]), inputs: vec![EventField { field: AbiField { name: "handler".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, indexed: true }], anonymous: false } }
