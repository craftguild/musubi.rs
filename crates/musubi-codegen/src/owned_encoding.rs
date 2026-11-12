//! Streaming canonical encoding primitives for bounded owned type identities.

use musubi_core::AbiType;
use sha3::{Digest, Keccak256};

use crate::abi_type::canonical_type;

const DOMAIN: &[u8] = b"owned-tuple-surface/v1";

/// Full Keccak-256 identity of one canonical owned-type encoding.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct OwnedTypeIdentity(pub(crate) [u8; 32]);

/// Incremental encoder that never retains a recursively flattened identity tree.
pub(crate) struct IdentityEncoder(Keccak256);

impl IdentityEncoder {
    /// Starts a domain-separated canonical node with its byte tag.
    pub(crate) fn new(tag: u8) -> Self {
        let mut encoder = Self(Keccak256::new());
        encoder.bytes(DOMAIN);
        encoder.byte(tag);
        encoder
    }

    /// Appends one byte tag.
    pub(crate) fn byte(&mut self, value: u8) {
        self.0.update([value]);
    }

    /// Appends an eight-byte length followed by exact bytes.
    pub(crate) fn bytes(&mut self, value: &[u8]) {
        self.usize(value.len());
        self.0.update(value);
    }

    /// Appends one fixed-size child identity as a length-prefixed value.
    pub(crate) fn identity(&mut self, identity: OwnedTypeIdentity) {
        self.bytes(&identity.0);
    }

    /// Appends a bounded unsigned integer in canonical big-endian form.
    pub(crate) fn usize(&mut self, value: usize) {
        self.0.update((value as u64).to_be_bytes());
    }

    /// Hashes the small node encoding into its full recursive identity.
    pub(crate) fn finish(self) -> OwnedTypeIdentity {
        OwnedTypeIdentity(self.0.finalize().into())
    }
}

/// Returns the canonical identity of a primitive wire type.
pub(crate) fn primitive_identity(ty: &AbiType) -> OwnedTypeIdentity {
    let mut encoder = IdentityEncoder::new(1);
    encoder.bytes(canonical_type(ty).as_bytes());
    encoder.finish()
}

/// Returns the canonical identity of one dynamic or fixed array node.
pub(crate) fn array_identity(
    tag: u8,
    length: Option<usize>,
    child: OwnedTypeIdentity,
) -> OwnedTypeIdentity {
    let mut encoder = IdentityEncoder::new(tag);
    if let Some(length) = length {
        encoder.usize(length);
    }
    encoder.identity(child);
    encoder.finish()
}

/// Returns the generated Rust spelling for one primitive ABI type.
pub(crate) fn primitive_rust_type(ty: &AbiType) -> String {
    match ty {
        AbiType::Address => "Address".into(),
        AbiType::Bool => "bool".into(),
        AbiType::String => "String".into(),
        AbiType::Bytes => "Bytes".into(),
        AbiType::FixedBytes(size) => format!("FixedBytes<{size}>"),
        AbiType::Uint(bits) => format!("Uint<{bits}>"),
        AbiType::Int(bits) => format!("Int<{bits}>"),
        AbiType::UFixed(bits, decimals) => format!("UFixed<{bits}, {decimals}>"),
        AbiType::Fixed(bits, decimals) => format!("Fixed<{bits}, {decimals}>"),
        AbiType::Function => "FunctionPointer".into(),
        AbiType::Array(_) | AbiType::FixedArray(_, _) | AbiType::Tuple(_) => String::new(),
    }
}
