//! One event's owned value, descriptor, and strict log codec.

use musubi_core::{Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed event `AddedOwner(address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddedOwner {
    /// ABI event field `owner`.
    pub owner: Address,
}

impl AddedOwner {
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
EventDescriptor { name: "AddedOwner".into(), signature: "AddedOwner(address)".into(), topic: Some([148, 101, 250, 12, 150, 44, 199, 105, 88, 230, 55, 58, 153, 51, 38, 64, 12, 28, 148, 248, 190, 47, 227, 169, 82, 173, 250, 127, 96, 178, 234, 38]), inputs: vec![EventField { field: AbiField { name: "owner".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, indexed: true }], anonymous: false } }
