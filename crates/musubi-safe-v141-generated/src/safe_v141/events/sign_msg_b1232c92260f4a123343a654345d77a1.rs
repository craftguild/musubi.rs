//! One event's owned value, descriptor, and strict log codec.

use musubi_core::{FixedBytes};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed event `SignMsg(bytes32)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignMsg {
    /// ABI event field `msgHash`.
    pub msg_hash: FixedBytes<32>,
}

impl SignMsg {
    /// Decodes one log with strict signature, topic-count, and canonical-data checks.
    pub fn decode(log: &musubi_core::Log) -> Result<Self, CodecError> {
        let values = musubi_runtime::decode_event(&descriptor(), log)?;
        let mut values = values.into_iter();
        Ok(Self {
            msg_hash: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <FixedBytes<32> as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
        })
    }
    /// Encodes this value as a deterministic metadata-free fixture log.
    pub fn encode_log(&self, address: musubi_core::Address) -> Result<musubi_core::Log, CodecError> {
        let values = vec![
            musubi_runtime::DecodedEventField::Value(self.msg_hash.into_abi_value()),
        ];
        musubi_runtime::encode_event(&descriptor(), address, &values)
    }
}

pub(super) fn descriptor() -> musubi_core::EventDescriptor {
    use musubi_core::*;
EventDescriptor { name: "SignMsg".into(), signature: "SignMsg(bytes32)".into(), topic: Some([231, 244, 103, 80, 56, 244, 246, 3, 77, 252, 187, 178, 76, 77, 192, 142, 78, 191, 16, 235, 157, 37, 125, 61, 2, 192, 243, 141, 18, 42, 198, 228]), inputs: vec![EventField { field: AbiField { name: "msgHash".into(), ty: AbiType::FixedBytes(32), internal_type: Some("bytes32".into()) }, indexed: true }], anonymous: false } }
