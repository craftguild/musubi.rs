//! One event's owned value, descriptor, and strict log codec.

use musubi_core::{Address, FixedBytes};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed event `ApproveHash(bytes32,address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApproveHash {
    /// ABI event field `approvedHash`.
    pub approved_hash: FixedBytes<32>,
    /// ABI event field `owner`.
    pub owner: Address,
}

impl ApproveHash {
    /// Decodes one log with strict signature, topic-count, and canonical-data checks.
    pub fn decode(log: &musubi_core::Log) -> Result<Self, CodecError> {
        let values = musubi_runtime::decode_event(&descriptor(), log)?;
        let mut values = values.into_iter();
        Ok(Self {
            approved_hash: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <FixedBytes<32> as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
            owner: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <Address as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
        })
    }
    /// Encodes this value as a deterministic metadata-free fixture log.
    pub fn encode_log(&self, address: musubi_core::Address) -> Result<musubi_core::Log, CodecError> {
        let values = vec![
            musubi_runtime::DecodedEventField::Value(self.approved_hash.into_abi_value()),
            musubi_runtime::DecodedEventField::Value(self.owner.into_abi_value()),
        ];
        musubi_runtime::encode_event(&descriptor(), address, &values)
    }
}

pub(super) fn descriptor() -> musubi_core::EventDescriptor {
    use musubi_core::*;
EventDescriptor { name: "ApproveHash".into(), signature: "ApproveHash(bytes32,address)".into(), topic: Some([242, 160, 235, 21, 100, 114, 209, 68, 2, 85, 176, 215, 193, 225, 156, 192, 113, 21, 209, 5, 31, 230, 5, 176, 220, 230, 154, 207, 236, 136, 77, 156]), inputs: vec![EventField { field: AbiField { name: "approvedHash".into(), ty: AbiType::FixedBytes(32), internal_type: Some("bytes32".into()) }, indexed: true }, EventField { field: AbiField { name: "owner".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, indexed: true }], anonymous: false } }
