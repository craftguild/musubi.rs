//! Alloy-independent EIP-712 type graph descriptors.

use serde::{Deserialize, Serialize};

/// One closed EIP-712 field type.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TypedDataType {
    /// Ethereum address encoded as a left-padded 20-byte value.
    Address,
    /// Boolean encoded as zero or one.
    Bool,
    /// UTF-8 string hashed before inclusion in a struct.
    String,
    /// Dynamic bytes hashed before inclusion in a struct.
    Bytes,
    /// Right-padded fixed bytes with a width from one through 32.
    FixedBytes(usize),
    /// Unsigned integer with an explicit eight-bit-aligned width.
    Uint(usize),
    /// Signed integer with an explicit eight-bit-aligned width.
    Int(usize),
    /// Reference to a named EIP-712 struct.
    Struct(String),
    /// Dynamic array whose encoded element words are hashed in order.
    Array(Box<Self>),
    /// Fixed array whose encoded element words are hashed in order.
    FixedArray(Box<Self>, usize),
}

/// One source-ordered field in a named EIP-712 struct.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedDataField {
    /// Solidity field name.
    pub name: String,
    /// Closed field type.
    pub ty: TypedDataType,
}

/// One named EIP-712 struct descriptor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedDataStruct {
    /// Solidity struct name.
    pub name: String,
    /// Fields in declared source order.
    pub fields: Vec<TypedDataField>,
}

/// One root and its optional explicit EIP-712 domain struct.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TypedDataRoot {
    /// Root exposes a struct hash without a final domain-bound digest.
    StructHash {
        /// Named primary struct.
        r#type: String,
    },
    /// Root exposes a final digest bound to the named domain struct.
    TypedData {
        /// Named primary struct.
        r#type: String,
        /// Named domain struct.
        domain: String,
    },
}

impl TypedDataRoot {
    /// Returns the primary struct name.
    #[must_use]
    pub fn primary_type(&self) -> &str {
        match self {
            Self::StructHash { r#type } | Self::TypedData { r#type, .. } => r#type,
        }
    }

    /// Returns the explicit domain struct name when the root exposes a final digest.
    #[must_use]
    pub fn domain_type(&self) -> Option<&str> {
        match self {
            Self::StructHash { .. } => None,
            Self::TypedData { domain, .. } => Some(domain),
        }
    }
}
