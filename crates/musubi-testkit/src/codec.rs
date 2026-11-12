//! External golden-vector conformance for the owned ABI codec.

use musubi_core::{AbiField, AbiValue, Bytes};
use musubi_runtime::{CodecError, decode_fields, encode_fields};

/// Canonical ABI vector supplied by an implementation independent from this runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodecVector {
    /// Ordered ABI field descriptors.
    pub fields: Vec<AbiField>,
    /// Ordered owned values.
    pub values: Vec<AbiValue>,
    /// Expected canonical ABI bytes.
    pub encoded: Bytes,
}

/// Checks encoding and strict decoding against one committed vector.
///
/// # Errors
///
/// Returns [`CodecError`] when descriptor/value shapes are invalid or either direction differs
/// from the canonical vector.
pub fn check_codec_vector(vector: &CodecVector) -> Result<(), CodecError> {
    let encoded = encode_fields(&vector.fields, &vector.values)?;
    if encoded != vector.encoded {
        return Err(CodecError::Malformed);
    }
    let decoded = decode_fields(&vector.fields, vector.encoded.as_ref())?;
    if decoded != vector.values {
        return Err(CodecError::Malformed);
    }
    Ok(())
}
