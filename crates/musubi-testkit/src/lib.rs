//! Conformance helpers for runtime backends, codecs, and generated crates.
//!
//! The crate contains test-only building blocks and never introduces production fallbacks.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod backend;
mod closed_json;
mod codec;
mod generated;
mod generated_cargo;
mod generated_generator;
mod generated_identity;
mod generated_inventory;
mod generated_layout;
mod generated_provenance;
mod generated_release;
mod generated_source;
#[cfg(test)]
mod generated_test_fixture;
mod generated_tree;
mod generated_type_string;
mod generated_types;

pub use backend::{
    BackendConformanceError, ReadBackendCase, TransactionExecutorCase, check_read_backend,
    check_transaction_executor,
};
pub use codec::{CodecVector, check_codec_vector};
pub use generated::{
    GeneratedCargoCheck, GeneratedCrateError, GeneratedCrateReport, cargo_check_command,
    inspect_generated_crate,
};

#[cfg(test)]
mod generated_inventory_tests;
#[cfg(test)]
mod generated_manifest_tests;
#[cfg(test)]
mod generated_path_tests;
#[cfg(test)]
mod generated_provenance_tests;
#[cfg(test)]
mod generated_release_tests;
#[cfg(test)]
mod generated_semantic_tests;
#[cfg(test)]
mod tests;

#[cfg(all(test, unix))]
mod generated_cargo_tests;
