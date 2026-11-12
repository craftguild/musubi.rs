//! One event's owned value, descriptor, and strict log codec.

use musubi_core::{Address, Uint};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed event `SafeSetup(address,address[],uint256,address,address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeSetup {
    /// ABI event field `initiator`.
    pub initiator: Address,
    /// ABI event field `owners`.
    pub owners: Vec<Address>,
    /// ABI event field `threshold`.
    pub threshold: Uint<256>,
    /// ABI event field `initializer`.
    pub initializer: Address,
    /// ABI event field `fallbackHandler`.
    pub fallback_handler: Address,
}

impl SafeSetup {
    /// Decodes one log with strict signature, topic-count, and canonical-data checks.
    pub fn decode(log: &musubi_core::Log) -> Result<Self, CodecError> {
        let values = musubi_runtime::decode_event(&descriptor(), log)?;
        let mut values = values.into_iter();
        Ok(Self {
            initiator: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <Address as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
            owners: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <Vec<Address> as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
            threshold: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <Uint<256> as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
            initializer: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <Address as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
            fallback_handler: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <Address as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
        })
    }
    /// Encodes this value as a deterministic metadata-free fixture log.
    pub fn encode_log(&self, address: musubi_core::Address) -> Result<musubi_core::Log, CodecError> {
        let values = vec![
            musubi_runtime::DecodedEventField::Value(self.initiator.into_abi_value()),
            musubi_runtime::DecodedEventField::Value(IntoAbiValue::into_abi_value(&self.owners)),
            musubi_runtime::DecodedEventField::Value(self.threshold.into_abi_value()),
            musubi_runtime::DecodedEventField::Value(self.initializer.into_abi_value()),
            musubi_runtime::DecodedEventField::Value(self.fallback_handler.into_abi_value()),
        ];
        musubi_runtime::encode_event(&descriptor(), address, &values)
    }
}

pub(super) fn descriptor() -> musubi_core::EventDescriptor {
    use musubi_core::*;
EventDescriptor { name: "SafeSetup".into(), signature: "SafeSetup(address,address[],uint256,address,address)".into(), topic: Some([20, 29, 248, 104, 166, 51, 26, 245, 40, 227, 140, 131, 183, 170, 3, 237, 193, 155, 230, 110, 55, 174, 103, 249, 40, 91, 244, 248, 227, 198, 161, 168]), inputs: vec![EventField { field: AbiField { name: "initiator".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, indexed: true }, EventField { field: AbiField { name: "owners".into(), ty: AbiType::Array(Box::new(AbiType::Address)), internal_type: Some("address[]".into()) }, indexed: false }, EventField { field: AbiField { name: "threshold".into(), ty: AbiType::Uint(256), internal_type: Some("uint256".into()) }, indexed: false }, EventField { field: AbiField { name: "initializer".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, indexed: false }, EventField { field: AbiField { name: "fallbackHandler".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, indexed: false }], anonymous: false } }
