//! Alloy-independent canonical ABI descriptors used by codecs and generated operations.

use serde::{Deserialize, Serialize};

/// Every Solidity ABI wire type, including recursively nested arrays and tuples.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AbiType {
    /// Ethereum address.
    Address,
    /// Boolean encoded as a 256-bit word.
    Bool,
    /// UTF-8 Solidity string.
    String,
    /// Arbitrary byte string.
    Bytes,
    /// Fixed-width byte string from one through 32 bytes.
    FixedBytes(usize),
    /// Unsigned integer with the declared bit width.
    Uint(usize),
    /// Signed integer with the declared bit width.
    Int(usize),
    /// Unsigned fixed point with `(bits, decimals)`.
    UFixed(usize, usize),
    /// Signed fixed point with `(bits, decimals)`.
    Fixed(usize, usize),
    /// External function pointer encoded as address plus selector.
    Function,
    /// Dynamic array.
    Array(Box<Self>),
    /// Fixed-length array.
    FixedArray(Box<Self>, usize),
    /// Ordered tuple components.
    Tuple(Vec<AbiField>),
}

impl AbiType {
    /// Returns whether this type is represented indirectly in standard ABI encoding.
    #[must_use]
    pub fn is_dynamic(&self) -> bool {
        match self {
            Self::String | Self::Bytes | Self::Array(_) => true,
            Self::FixedArray(element, _) => element.is_dynamic(),
            Self::Tuple(fields) => fields.iter().any(|field| field.ty.is_dynamic()),
            _ => false,
        }
    }
}

/// A named or anonymous ABI tuple/function/event component.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbiField {
    /// External compiler name, which may be empty for anonymous components.
    pub name: String,
    /// Canonical wire type.
    pub ty: AbiType,
    /// Optional Solidity internal type used only for enrichment identity.
    pub internal_type: Option<String>,
}

/// Function state-mutability classification from ABI JSON.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateMutability {
    /// Reads chain state without mutation.
    View,
    /// Reads no chain state.
    Pure,
    /// May mutate chain state and rejects attached value.
    Nonpayable,
    /// May mutate chain state and accepts attached value.
    Payable,
}

/// Canonical function ABI information.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionDescriptor {
    /// ABI function name.
    pub name: String,
    /// Canonical Solidity signature.
    pub signature: String,
    /// First four bytes of Keccak-256 of the canonical signature.
    pub selector: [u8; 4],
    /// Ordered input fields.
    pub inputs: Vec<AbiField>,
    /// Ordered output fields.
    pub outputs: Vec<AbiField>,
    /// State mutability.
    pub state_mutability: StateMutability,
}

/// Canonical event ABI information.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventDescriptor {
    /// ABI event name.
    pub name: String,
    /// Canonical Solidity signature.
    pub signature: String,
    /// Keccak-256 canonical signature topic, absent for anonymous events.
    pub topic: Option<[u8; 32]>,
    /// Ordered event parameters.
    pub inputs: Vec<EventField>,
    /// Whether topic zero omits the signature hash.
    pub anonymous: bool,
}

/// An event parameter and whether it is stored in a log topic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventField {
    /// Parameter descriptor.
    pub field: AbiField,
    /// Whether the parameter is indexed.
    pub indexed: bool,
}

/// Canonical custom error ABI information.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorDescriptor {
    /// ABI error name.
    pub name: String,
    /// Canonical Solidity signature.
    pub signature: String,
    /// First four bytes of Keccak-256 of the canonical signature.
    pub selector: [u8; 4],
    /// Ordered error arguments.
    pub inputs: Vec<AbiField>,
}

/// Constructor ABI information.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstructorDescriptor {
    /// Ordered constructor arguments.
    pub inputs: Vec<AbiField>,
    /// Constructor state mutability.
    pub state_mutability: StateMutability,
}

/// Complete canonical contract ABI.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractAbi {
    /// Optional constructor; absence means a no-argument nonpayable constructor.
    pub constructor: Option<ConstructorDescriptor>,
    /// Canonically sorted functions.
    pub functions: Vec<FunctionDescriptor>,
    /// Canonically sorted events.
    pub events: Vec<EventDescriptor>,
    /// Canonically sorted custom errors.
    pub errors: Vec<ErrorDescriptor>,
    /// Whether fallback entry is present.
    pub fallback: Option<StateMutability>,
    /// Whether receive entry is present.
    pub receive: Option<StateMutability>,
}
