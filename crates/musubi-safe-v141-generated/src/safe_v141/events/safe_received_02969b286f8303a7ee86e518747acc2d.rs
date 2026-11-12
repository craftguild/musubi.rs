//! One event's owned value, descriptor, and strict log codec.

use musubi_core::{Address, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed event `SafeReceived(address,uint256)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeReceived {
    /// ABI event field `sender`.
    pub sender: Address,
    /// ABI event field `value`.
    pub value: Uint<256>,
}

impl SafeReceived {
    /// Decodes one log with strict signature, topic-count, and canonical-data checks.
    pub fn decode(log: &musubi_core::Log) -> Result<Self, CodecError> {
        let values = musubi_runtime::decode_event(&descriptor(), log)?;
        let mut values = values.into_iter();
        Ok(Self {
            sender: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <Address as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
            value: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <Uint<256> as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
        })
    }
    /// Encodes this value as a deterministic metadata-free fixture log.
    pub fn encode_log(&self, address: musubi_core::Address) -> Result<musubi_core::Log, CodecError> {
        let values = vec![
            musubi_runtime::DecodedEventField::Value(self.sender.into_abi_value()),
            musubi_runtime::DecodedEventField::Value(self.value.into_abi_value()),
        ];
        musubi_runtime::encode_event(&descriptor(), address, &values)
    }
}

pub(super) fn descriptor() -> musubi_core::EventDescriptor {
    use musubi_core::*;
EventDescriptor { name: "SafeReceived".into(), signature: "SafeReceived(address,uint256)".into(), topic: Some([61, 12, 233, 191, 195, 237, 125, 104, 98, 219, 178, 139, 45, 234, 148, 86, 31, 231, 20, 161, 180, 208, 25, 170, 138, 243, 151, 48, 209, 173, 124, 61]), inputs: vec![EventField { field: AbiField { name: "sender".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, indexed: true }, EventField { field: AbiField { name: "value".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }, indexed: false }], anonymous: false } }
