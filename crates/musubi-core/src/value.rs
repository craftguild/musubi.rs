//! Dynamic owned ABI values used at codec boundaries.

use serde::{Deserialize, Serialize};

use crate::{Address, Bytes, Hash};

/// External Solidity function pointer encoded as an address and four-byte selector.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct FunctionPointer {
    /// Contract containing the external function.
    pub address: Address,
    /// Function selector.
    pub selector: [u8; 4],
}

/// Typed representation of an indexed event value.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Indexed<T> {
    /// Directly decoded static value.
    Value(T),
    /// Keccak topic for a dynamic or composite value whose preimage is unavailable.
    Hash(Hash),
}

/// Backend-independent dynamic ABI value.
///
/// Width and shape are checked against an [`crate::AbiType`] by the runtime codec. Generated APIs
/// construct variants from width-checked concrete values and do not expose this enum in typed
/// method signatures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AbiValue {
    /// Ethereum address.
    Address(Address),
    /// Boolean.
    Bool(bool),
    /// UTF-8 string.
    String(String),
    /// Dynamic bytes.
    Bytes(Bytes),
    /// Fixed-width bytes; the descriptor supplies the declared width.
    FixedBytes(Bytes),
    /// Unsigned big-endian 256-bit word; the descriptor supplies the declared width.
    Uint([u8; 32]),
    /// Sign-extended big-endian 256-bit word; the descriptor supplies the declared width.
    Int([u8; 32]),
    /// Unsigned scaled integer for a fixed-point descriptor.
    UFixed([u8; 32]),
    /// Signed scaled integer for a fixed-point descriptor.
    Fixed([u8; 32]),
    /// External function pointer.
    Function(FunctionPointer),
    /// Dynamic or fixed array values; the descriptor supplies element and length constraints.
    Array(Vec<Self>),
    /// Ordered tuple values.
    Tuple(Vec<Self>),
}
