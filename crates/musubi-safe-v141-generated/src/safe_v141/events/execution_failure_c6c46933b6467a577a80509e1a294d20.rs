//! One event's owned value, descriptor, and strict log codec.

use musubi_core::{FixedBytes, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed event `ExecutionFailure(bytes32,uint256)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionFailure {
    /// ABI event field `txHash`.
    pub tx_hash: FixedBytes<32>,
    /// ABI event field `payment`.
    pub payment: Uint<256>,
}

impl ExecutionFailure {
    /// Decodes one log with strict signature, topic-count, and canonical-data checks.
    pub fn decode(log: &musubi_core::Log) -> Result<Self, CodecError> {
        let values = musubi_runtime::decode_event(&descriptor(), log)?;
        let mut values = values.into_iter();
        Ok(Self {
            tx_hash: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <FixedBytes<32> as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
            payment: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <Uint<256> as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
        })
    }
    /// Encodes this value as a deterministic metadata-free fixture log.
    pub fn encode_log(&self, address: musubi_core::Address) -> Result<musubi_core::Log, CodecError> {
        let values = vec![
            musubi_runtime::DecodedEventField::Value(self.tx_hash.into_abi_value()),
            musubi_runtime::DecodedEventField::Value(self.payment.into_abi_value()),
        ];
        musubi_runtime::encode_event(&descriptor(), address, &values)
    }
}

pub(super) fn descriptor() -> musubi_core::EventDescriptor {
    use musubi_core::*;
EventDescriptor { name: "ExecutionFailure".into(), signature: "ExecutionFailure(bytes32,uint256)".into(), topic: Some([35, 66, 139, 24, 172, 251, 62, 166, 75, 8, 220, 12, 29, 41, 110, 169, 192, 151, 2, 192, 144, 131, 202, 82, 114, 230, 77, 17, 91, 104, 125, 35]), inputs: vec![EventField { field: AbiField { name: "txHash".into(), ty: AbiType::FixedBytes(32), internal_type: Some("bytes32".into()) }, indexed: true }, EventField { field: AbiField { name: "payment".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }, indexed: false }], anonymous: false } }
