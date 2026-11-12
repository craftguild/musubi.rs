//! One event's owned value, descriptor, and strict log codec.

use musubi_core::{Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed event `ChangedThreshold(uint256)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangedThreshold {
    /// ABI event field `threshold`.
    pub threshold: Uint<256>,
}

impl ChangedThreshold {
    /// Decodes one log with strict signature, topic-count, and canonical-data checks.
    pub fn decode(log: &musubi_core::Log) -> Result<Self, CodecError> {
        let values = musubi_runtime::decode_event(&descriptor(), log)?;
        let mut values = values.into_iter();
        Ok(Self {
            threshold: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <Uint<256> as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
        })
    }
    /// Encodes this value as a deterministic metadata-free fixture log.
    pub fn encode_log(&self, address: musubi_core::Address) -> Result<musubi_core::Log, CodecError> {
        let values = vec![
            musubi_runtime::DecodedEventField::Value(self.threshold.into_abi_value()),
        ];
        musubi_runtime::encode_event(&descriptor(), address, &values)
    }
}

pub(super) fn descriptor() -> musubi_core::EventDescriptor {
    use musubi_core::*;
EventDescriptor { name: "ChangedThreshold".into(), signature: "ChangedThreshold(uint256)".into(), topic: Some([97, 15, 127, 242, 179, 4, 174, 137, 3, 195, 222, 116, 198, 12, 106, 177, 247, 214, 34, 107, 63, 82, 197, 22, 25, 5, 187, 90, 212, 3, 156, 147]), inputs: vec![EventField { field: AbiField { name: "threshold".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }, indexed: false }], anonymous: false } }
