//! Runtime services for generated SDK crates.
//!
//! The crate defines backend interfaces and wire operations exclusively in terms of owned
//! `musubi-core` values.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod backend;
mod codec;
mod deployment;
mod descriptor;
mod event;
mod operation;
mod revert;
mod typed_data;
mod typed_data_graph;
mod typed_data_type_text;
mod typed_value;

pub use backend::{
    BackendError, BackendErrorKind, BackendFuture, ReadBackend, TransactionExecutor,
};
pub use codec::{CodecError, decode_fields, encode_fields};
pub use deployment::{
    DeploymentError, LibraryAddresses, RuntimeVerification, build_init_code, execute_deployment,
    link_creation, verify_runtime,
};
pub use event::{DecodedEventField, decode_event, encode_event};
pub use operation::{
    BoundCall, BoundTransaction, DecodeReturns, EncodeArguments, FunctionCall, FunctionTransaction,
    OperationError,
};
pub use revert::{DecodedRevert, decode_revert};
pub use typed_data::{
    TypedDataError, typed_data_digest, typed_data_struct_hash, typed_data_type_hash,
    typed_data_type_string,
};
pub use typed_data_graph::TypedDataGraph;

#[cfg(test)]
mod typed_data_edge_tests;
#[cfg(test)]
mod typed_data_tests;
pub use typed_value::{FromAbiValue, IntoAbiValue};

/// Runtime ABI version understood by this build.
pub const GENERATED_ABI_VERSION: u32 = musubi_core::GENERATED_ABI_VERSION;

/// Fails compilation when generated source and runtime ABI versions differ.
#[macro_export]
macro_rules! assert_generated_abi {
    ($version:expr) => {
        const _: [(); $crate::GENERATED_ABI_VERSION as usize] = [(); $version as usize];
    };
}

#[cfg(test)]
mod codec_tests;
#[cfg(test)]
mod deployment_tests;
#[cfg(test)]
mod event_tests;
#[cfg(test)]
mod operation_tests;
#[cfg(test)]
mod revert_tests;
