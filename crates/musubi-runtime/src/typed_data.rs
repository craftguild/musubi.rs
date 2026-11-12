//! Strict EIP-712 hashing over owned values and a prepared named type graph.

use musubi_core::{AbiValue, Hash, TypedDataStruct, TypedDataType, keccak256};
use sha3::{Digest as _, Keccak256};

use crate::{TypedDataGraph, typed_data_graph::MAX_DEPTH};

/// Closed typed-data validation or value-shape failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TypedDataError {
    /// The named type graph is duplicate, unresolved, cyclic, or otherwise invalid.
    #[error("typed-data type graph is invalid")]
    InvalidGraph,
    /// An owned value does not match the selected typed-data field shape.
    #[error("typed-data value does not match its declared type")]
    TypeMismatch,
    /// Recursive value or type processing exceeded the hard limit.
    #[error("typed-data nesting exceeds the hard limit")]
    Nesting,
    /// Canonical type text exceeded a caller-supplied byte limit.
    #[error("typed-data type text exceeds the byte limit")]
    ResourceLimit,
}

/// Produces canonical EIP-712 `encodeType` text for one named primary struct.
///
/// # Errors
///
/// Returns [`TypedDataError`] when the graph is invalid or the primary type is unknown.
pub fn typed_data_type_string(
    types: &[TypedDataStruct],
    primary: &str,
) -> Result<String, TypedDataError> {
    TypedDataGraph::new(types)?.type_string(primary)
}

/// Computes the canonical type hash for one named primary struct.
///
/// # Errors
///
/// Returns [`TypedDataError`] when the graph is invalid.
pub fn typed_data_type_hash(
    types: &[TypedDataStruct],
    primary: &str,
) -> Result<Hash, TypedDataError> {
    TypedDataGraph::new(types)?.type_hash(primary)
}

/// Computes a strict EIP-712 struct hash for one owned tuple-shaped value.
///
/// # Errors
///
/// Returns [`TypedDataError`] for invalid graphs, arity/type mismatches, or excessive nesting.
pub fn typed_data_struct_hash(
    types: &[TypedDataStruct],
    primary: &str,
    value: &AbiValue,
) -> Result<Hash, TypedDataError> {
    TypedDataGraph::new(types)?.struct_hash(primary, value)
}

/// Computes `keccak256(0x1901 || domainSeparator || structHash)`.
///
/// # Errors
///
/// Returns [`TypedDataError`] when either named value fails strict graph validation.
pub fn typed_data_digest(
    types: &[TypedDataStruct],
    domain_type: &str,
    domain: &AbiValue,
    primary: &str,
    value: &AbiValue,
) -> Result<Hash, TypedDataError> {
    TypedDataGraph::new(types)?.digest(domain_type, domain, primary, value)
}

impl TypedDataGraph<'_> {
    /// Computes a strict EIP-712 struct hash while reusing validated graph state.
    ///
    /// # Errors
    ///
    /// Returns [`TypedDataError`] for an unknown type, value mismatch, or excessive nesting.
    pub fn struct_hash(&mut self, primary: &str, value: &AbiValue) -> Result<Hash, TypedDataError> {
        struct_hash(self, primary, value, 0)
    }

    /// Computes a final EIP-712 digest while reusing validated graph state.
    ///
    /// # Errors
    ///
    /// Returns [`TypedDataError`] for an unknown type, value mismatch, or excessive nesting.
    pub fn digest(
        &mut self,
        domain_type: &str,
        domain: &AbiValue,
        primary: &str,
        value: &AbiValue,
    ) -> Result<Hash, TypedDataError> {
        let domain_hash = struct_hash(self, domain_type, domain, 0)?;
        let value_hash = struct_hash(self, primary, value, 0)?;
        let mut envelope = Vec::with_capacity(66);
        envelope.extend_from_slice(&[0x19, 0x01]);
        envelope.extend_from_slice(domain_hash.as_ref());
        envelope.extend_from_slice(value_hash.as_ref());
        Ok(keccak256(&envelope))
    }
}

fn struct_hash(
    graph: &mut TypedDataGraph<'_>,
    primary: &str,
    value: &AbiValue,
    depth: usize,
) -> Result<Hash, TypedDataError> {
    if depth > MAX_DEPTH {
        return Err(TypedDataError::Nesting);
    }
    let descriptor = graph.descriptor(primary)?;
    let AbiValue::Tuple(values) = value else {
        return Err(TypedDataError::TypeMismatch);
    };
    if descriptor.fields.len() != values.len() {
        return Err(TypedDataError::TypeMismatch);
    }
    let mut encoded = Keccak256::new();
    encoded.update(graph.type_hash(primary)?.as_ref());
    for (field, value) in descriptor.fields.iter().zip(values) {
        encoded.update(field_word(graph, &field.ty, value, depth)?);
    }
    Ok(Hash::new(encoded.finalize().into()))
}

fn field_word(
    graph: &mut TypedDataGraph<'_>,
    ty: &TypedDataType,
    value: &AbiValue,
    depth: usize,
) -> Result<[u8; 32], TypedDataError> {
    if depth > MAX_DEPTH {
        return Err(TypedDataError::Nesting);
    }
    match (ty, value) {
        (TypedDataType::Address, AbiValue::Address(value)) => {
            let mut word = [0; 32];
            word[12..].copy_from_slice(value.as_ref());
            Ok(word)
        }
        (TypedDataType::Bool, AbiValue::Bool(value)) => {
            let mut word = [0; 32];
            word[31] = u8::from(*value);
            Ok(word)
        }
        (TypedDataType::String, AbiValue::String(value)) => {
            Ok(keccak256(value.as_bytes()).into_array())
        }
        (TypedDataType::Bytes, AbiValue::Bytes(value)) => {
            Ok(keccak256(value.as_ref()).into_array())
        }
        (TypedDataType::FixedBytes(size), AbiValue::FixedBytes(value)) if value.len() == *size => {
            let mut word = [0; 32];
            word[..*size].copy_from_slice(value.as_ref());
            Ok(word)
        }
        (TypedDataType::Uint(bits), AbiValue::Uint(word)) if unsigned_valid(*bits, word) => {
            Ok(*word)
        }
        (TypedDataType::Int(bits), AbiValue::Int(word)) if signed_valid(*bits, word) => Ok(*word),
        (TypedDataType::Struct(name), AbiValue::Tuple(_)) => {
            Ok(struct_hash(graph, name, value, nested(depth)?)?.into_array())
        }
        (TypedDataType::Array(element), AbiValue::Array(values)) => {
            array_word(graph, element, values, nested(depth)?)
        }
        (TypedDataType::FixedArray(element, size), AbiValue::Array(values))
            if values.len() == *size =>
        {
            array_word(graph, element, values, nested(depth)?)
        }
        _ => Err(TypedDataError::TypeMismatch),
    }
}

fn nested(depth: usize) -> Result<usize, TypedDataError> {
    let depth = depth.checked_add(1).ok_or(TypedDataError::Nesting)?;
    if depth > MAX_DEPTH {
        Err(TypedDataError::Nesting)
    } else {
        Ok(depth)
    }
}

fn array_word(
    graph: &mut TypedDataGraph<'_>,
    element: &TypedDataType,
    values: &[AbiValue],
    depth: usize,
) -> Result<[u8; 32], TypedDataError> {
    let mut encoded = Keccak256::new();
    for value in values {
        encoded.update(field_word(graph, element, value, depth)?);
    }
    Ok(Hash::new(encoded.finalize().into()).into_array())
}

fn unsigned_valid(bits: usize, word: &[u8; 32]) -> bool {
    (8..=256).contains(&bits)
        && bits.is_multiple_of(8)
        && word[..(256 - bits) / 8].iter().all(|byte| *byte == 0)
}

fn signed_valid(bits: usize, word: &[u8; 32]) -> bool {
    if !(8..=256).contains(&bits) || !bits.is_multiple_of(8) {
        return false;
    }
    let prefix = (256 - bits) / 8;
    let expected = if word[prefix] & 0x80 == 0 { 0 } else { 0xff };
    word[..prefix].iter().all(|byte| *byte == expected)
}
