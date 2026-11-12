//! Alloy-independent Ethereum values and artifact identity.
//!
//! This crate owns every type that may cross a generated SDK public boundary. Wire codecs and
//! transports belong to sibling crates.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod abi;
mod address;
mod artifact;
mod bytes;
mod fixed;
mod generator;
mod integer;
mod runtime_source;
mod transaction;
mod typed_data;
mod value;

pub use abi::*;
pub use address::Address;
pub use artifact::*;
pub use bytes::{Bytes, FixedBytes, Hash};
pub use fixed::{Fixed, UFixed};
pub use generator::{GENERATOR_INFO_SCHEMA, GeneratorBuild, GeneratorInfo, GeneratorInfoError};
pub use integer::{Int, Uint};
pub use runtime_source::{RuntimeSource, RuntimeSourceError};
pub use transaction::*;
pub use typed_data::*;
pub use value::{AbiValue, FunctionPointer, Indexed};

#[cfg(test)]
mod artifact_tests;
#[cfg(test)]
mod fixed_tests;

/// Current ABI shared by generated source and the runtime.
pub const GENERATED_ABI_VERSION: u32 = 2;

/// Maximum cumulative fixed-array element product accepted along one ABI type path.
pub const ABI_FIXED_ARRAY_ELEMENTS_LIMIT: usize = 65_536;

/// Validation failure for an owned Ethereum value.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ValueError {
    /// A hexadecimal input had invalid syntax.
    #[error("invalid hexadecimal value")]
    InvalidHex,
    /// A fixed-width value had the wrong byte length.
    #[error("expected {expected} bytes, found {actual}")]
    InvalidLength {
        /// Required byte length.
        expected: usize,
        /// Observed byte length.
        actual: usize,
    },
    /// A const-generic integer declaration used an unsupported width.
    #[error("integer width must be between 1 and 256 bits")]
    InvalidBitWidth,
    /// An integer cannot be represented at its declared width.
    #[error("integer value is outside its declared bit width")]
    IntegerOutOfRange,
    /// Fixed-point decimal precision is not valid for its storage width.
    #[error("fixed-point decimals exceed the supported range")]
    InvalidDecimals,
    /// A bytecode reference points beyond its template.
    #[error("bytecode reference is outside the template")]
    ReferenceOutOfBounds,
    /// Two bytecode references claim at least one common byte.
    #[error("bytecode references overlap")]
    OverlappingReferences,
    /// A deployable artifact supplied no creation bytecode.
    #[error("creation bytecode must not be empty")]
    EmptyCreationBytecode,
}
