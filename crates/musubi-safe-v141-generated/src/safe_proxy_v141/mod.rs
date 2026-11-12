//! Contract facade, address selection, clients, and aggregate event/revert accessors.

pub mod types;
#[path = "artifact.rs"]
mod __evm_artifact;
pub use __evm_artifact::*;

#[path = "constructor.rs"]
mod __evm_constructor;
pub use types::*;
use musubi_core::{Address, Bytes};

/// Marker for manifest contract `safe_proxy_v141`.
#[derive(Clone, Copy, Debug, Default)]
pub struct SafeProxyV141;

/// Address-selected `SafeProxyV141` without a backend.
#[derive(Clone, Copy, Debug)]
pub struct SafeProxyV141At { address: Address }

impl SafeProxyV141 {
    /// Selects a deployed contract address.
    #[must_use]
    pub fn at(address: Address) -> SafeProxyV141At { SafeProxyV141At { address } }
    /// Returns ABI-authoritative event descriptors.
    #[must_use]
    pub fn events() -> Vec<musubi_core::EventDescriptor> { vec![] }
    /// Returns ABI-authoritative custom error descriptors.
    #[must_use]
    pub fn errors() -> Vec<musubi_core::ErrorDescriptor> { vec![] }
}

impl SafeProxyV141At {
    /// Returns the selected contract address.
    #[must_use]
    pub const fn address(&self) -> Address { self.address }
}

/// Typed standard and contract-specific revert union.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SafeProxyV141Error {
    /// Standard `Error(string)` revert.
    Error(String),
    /// Standard `Panic(uint256)` revert.
    Panic([u8; 32]),
    /// Unrecognized revert bytes retained without interpretation.
    Unknown(Bytes),
}

impl SafeProxyV141 {
    /// Decodes a standard or ABI-declared custom revert.
    pub fn decode_error(data: &[u8]) -> Result<SafeProxyV141Error, musubi_runtime::CodecError> {
        match musubi_runtime::decode_revert(data, &Self::errors())? {
            musubi_runtime::DecodedRevert::Error(value) => Ok(SafeProxyV141Error::Error(value)),
            musubi_runtime::DecodedRevert::Panic(value) => Ok(SafeProxyV141Error::Panic(value)),
            musubi_runtime::DecodedRevert::Custom { .. } => Ok(SafeProxyV141Error::Unknown(Bytes::new(data.to_vec()))),
            musubi_runtime::DecodedRevert::Unknown => Ok(SafeProxyV141Error::Unknown(Bytes::new(data.to_vec()))),
        }
    }
}
