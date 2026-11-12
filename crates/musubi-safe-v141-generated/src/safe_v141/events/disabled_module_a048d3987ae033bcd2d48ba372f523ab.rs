//! One event's owned value, descriptor, and strict log codec.

use musubi_core::{Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed event `DisabledModule(address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisabledModule {
    /// ABI event field `module`.
    pub module: Address,
}

impl DisabledModule {
    /// Decodes one log with strict signature, topic-count, and canonical-data checks.
    pub fn decode(log: &musubi_core::Log) -> Result<Self, CodecError> {
        let values = musubi_runtime::decode_event(&descriptor(), log)?;
        let mut values = values.into_iter();
        Ok(Self {
            module: match values.next().ok_or(CodecError::Arity)? { musubi_runtime::DecodedEventField::Value(value) => <Address as FromAbiValue>::from_abi_value(value)?, _ => return Err(CodecError::TypeMismatch) },
        })
    }
    /// Encodes this value as a deterministic metadata-free fixture log.
    pub fn encode_log(&self, address: musubi_core::Address) -> Result<musubi_core::Log, CodecError> {
        let values = vec![
            musubi_runtime::DecodedEventField::Value(self.module.into_abi_value()),
        ];
        musubi_runtime::encode_event(&descriptor(), address, &values)
    }
}

pub(super) fn descriptor() -> musubi_core::EventDescriptor {
    use musubi_core::*;
EventDescriptor { name: "DisabledModule".into(), signature: "DisabledModule(address)".into(), topic: Some([170, 180, 250, 43, 70, 63, 88, 27, 43, 50, 203, 59, 126, 59, 112, 75, 156, 227, 124, 194, 9, 181, 251, 77, 119, 229, 147, 172, 228, 5, 66, 118]), inputs: vec![EventField { field: AbiField { name: "module".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, indexed: true }], anonymous: false } }
