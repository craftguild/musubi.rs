//! One event's owned value, descriptor, and strict log codec.

use musubi_core::{Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed event `ChangedGuard(address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangedGuard {
    /// ABI event field `guard`.
    pub guard: Address,
}

impl ChangedGuard {
    /// Decodes one log with strict signature, topic-count, and canonical-data checks.
    pub fn decode(log: &musubi_core::Log) -> Result<Self, CodecError> {
        let values = musubi_runtime::decode_event(&descriptor(), log)?;
        let mut values = values.into_iter();
        Ok(Self {
            guard: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <Address as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
        })
    }
    /// Encodes this value as a deterministic metadata-free fixture log.
    pub fn encode_log(&self, address: musubi_core::Address) -> Result<musubi_core::Log, CodecError> {
        let values = vec![
            musubi_runtime::DecodedEventField::Value(self.guard.into_abi_value()),
        ];
        musubi_runtime::encode_event(&descriptor(), address, &values)
    }
}

pub(super) fn descriptor() -> musubi_core::EventDescriptor {
    use musubi_core::*;
EventDescriptor { name: "ChangedGuard".into(), signature: "ChangedGuard(address)".into(), topic: Some([17, 81, 17, 105, 20, 81, 91, 192, 137, 31, 249, 4, 122, 108, 179, 44, 249, 2, 84, 111, 131, 6, 100, 153, 188, 248, 186, 51, 210, 53, 63, 162]), inputs: vec![EventField { field: AbiField { name: "guard".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, indexed: true }], anonymous: false } }
