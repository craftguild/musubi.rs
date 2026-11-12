//! One event's owned value, descriptor, and strict log codec.

use musubi_core::{Address};
use musubi_runtime::{CodecError, FromAbiValue, IntoAbiValue};

/// Typed event `EnabledModule(address)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnabledModule {
    /// ABI event field `module`.
    pub module: Address,
}

impl EnabledModule {
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
EventDescriptor { name: "EnabledModule".into(), signature: "EnabledModule(address)".into(), topic: Some([236, 223, 58, 62, 255, 234, 87, 131, 163, 196, 194, 20, 14, 103, 117, 119, 102, 100, 40, 212, 78, 217, 212, 116, 160, 179, 164, 201, 148, 63, 132, 64]), inputs: vec![EventField { field: AbiField { name: "module".into(), ty: AbiType::Address, internal_type: Some("address".into()) }, indexed: true }], anonymous: false } }
