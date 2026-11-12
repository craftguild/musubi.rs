//! Deterministic SDK generation from closed manifests and external compiler artifacts.
//!
//! ABI is the only wire authority. Optional compiler AST data can enrich names and Rust type
//! identity but cannot alter selectors, topics, or encoding.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod abi_json;
mod abi_parse;
mod abi_type;
mod ast;
mod ast_internal;
mod ast_type;
mod ast_type_parse;
mod bytecode;
mod canonical;
mod cargo_paths;
mod cargo_validation;
mod check_generated;
mod closed_json;
mod compatibility;
mod compatibility_surface;
mod compatibility_typed_data;
mod diagnostic;
mod generated_tree;
mod init;
mod input;
mod input_fs;
mod input_manifest;
mod ir;
mod limits;
mod manifest;
mod name_validation;
mod names;
mod output;
#[cfg(unix)]
mod output_cleanup;
#[cfg(unix)]
mod output_fs;
#[cfg(unix)]
mod output_install;
#[cfg(unix)]
mod output_read;
#[cfg(unix)]
mod output_recovery;
mod output_target;
#[cfg(unix)]
mod output_write;
mod owned_encoding;
mod owned_identity;
mod ownership;
#[cfg(test)]
mod ownership_conformance_tests;
mod ownership_io;
mod ownership_path;
#[cfg(test)]
mod ownership_tests;
mod package_identity;
mod process;
#[cfg(unix)]
mod process_cleanup;
mod render;
mod render_abi;
mod render_artifact;
mod render_constructor;
mod render_contract;
mod render_contract_layout;
mod render_contract_tree;
mod render_generator;
mod render_layout;
mod render_names;
mod render_operation;
mod render_owned_tuple;
mod render_release;
mod render_source;
mod render_type_imports;
mod render_typed_data;
mod render_types;
mod render_validation;
mod runtime_source;
mod semantic_inventory;
mod snapshot;
mod snapshot_bytecode_validation;
mod snapshot_contract_validation;
mod snapshot_validation;
mod typed_data_evidence;
mod typed_data_hash_text;
mod typed_data_schema;
mod typed_data_snapshot_validation;
mod typed_data_vectors;

pub use cargo_validation::{
    CargoValidationFailure, PreparedCargoCheck, doctor, prepare_cargo_check,
    validate_generated_cargo,
};
pub use check_generated::check_generated;
pub use compatibility::{
    CompatibilityCategory, CompatibilityChange, CompatibilityReport, diff_snapshots,
};
pub use diagnostic::{Diagnostic, DiagnosticCode, Diagnostics, Severity};
pub use generated_tree::{GeneratedFile, GeneratedTree};
pub use init::{INITIAL_MANIFEST, init_manifest};
pub use input::{ContractInput, InputBundle, InputError, TypedDataInput, load_inputs};
pub use ir::{
    ContractIr, Enrichment, EnrichmentKind, GenerationIr, IrError, TypedDataExternalEvidence,
    TypedDataIr, TypedDataRootEvidence, build_ir,
};
pub use limits::{HardLimits, LimitOverrides};
pub use manifest::{
    ContractConfig, GenerationConfig, GenerationOutput, Manifest, PackageConfig,
    TypedDataAuthority, TypedDataConfig,
};
pub use musubi_core::RuntimeSource;
pub use output::{CheckReport, GenerateError, WriteOperation, check, generate};
pub use output_target::OutputTarget;
pub use ownership::{
    GENERATED_MARKER, MAX_MARKER_BYTES, OWNERSHIP_SCHEMA, OwnedFile, OwnershipError,
    OwnershipManifest, SEMANTIC_LAYOUT, parse_ownership_marker, validate_owned_files,
};
pub use ownership_io::read_owned_tree;
pub use process::{
    CARGO_PROCESS_LIMITS, ProcessFailure, ProcessLimits, ProcessOutput, ProcessRequest,
    UNSUPPORTED_HOST_MESSAGE, host_capability_preflight, run_bounded_process,
};
pub use render::{RenderError, render};
pub use render_layout::SemanticOwner;
pub use semantic_inventory::{semantic_abi_modules, semantic_owner};
pub use snapshot::{
    CompatibilitySnapshot, ContractSnapshot, DeploymentSnapshot, ProvenanceSnapshot,
    RuntimeDeploymentSnapshot, SnapshotError, TypedDataSnapshot, build_snapshot, parse_snapshot,
    snapshot_json,
};

/// Stable generator ABI used in release manifests.
pub const GENERATED_ABI_VERSION: u32 = musubi_core::GENERATED_ABI_VERSION;

#[cfg(test)]
mod abi_event_tests;
#[cfg(test)]
mod abi_tests;
#[cfg(test)]
mod abi_type_tests;
#[cfg(test)]
mod artifact_kind_tests;
#[cfg(test)]
mod ast_tests;
#[cfg(test)]
mod bytecode_tests;
#[cfg(test)]
mod canonical_abi_budget_tests;
#[cfg(all(test, unix))]
mod cargo_validation_tests;
#[cfg(test)]
mod compatibility_tests;
#[cfg(test)]
mod compatibility_typed_data_tests;
#[cfg(all(test, unix))]
mod constructor_generation_tests;
#[cfg(test)]
mod event_evidence_tests;
#[cfg(test)]
mod fixed_point_tests;
#[cfg(all(test, unix))]
mod generated_event_wire_tests;
#[cfg(test)]
mod generation_tests;
#[cfg(all(test, unix))]
mod generator_namespace_tests;
#[cfg(all(test, unix))]
mod init_tests;
#[cfg(test)]
mod input_tests;
#[cfg(all(test, unix))]
mod lifecycle_tests;
#[cfg(test)]
mod manifest_provenance_tests;
#[cfg(test)]
mod manifest_tests;
#[cfg(all(test, unix))]
mod output_cleanup_tests;
#[cfg(all(test, unix))]
mod output_install_tests;
#[cfg(all(test, unix))]
mod output_ownership_tests;
#[cfg(all(test, unix))]
mod output_recovery_tests;
#[cfg(all(test, unix))]
mod output_term_tests;
#[cfg(test)]
mod output_test_support;
#[cfg(test)]
mod output_tests;
#[cfg(test)]
mod owned_identity_tests;
#[cfg(test)]
mod owned_tuple_generation_tests;
#[cfg(all(test, unix))]
mod owned_tuple_wire_tests;
#[cfg(all(test, unix))]
mod process_tests;
#[cfg(test)]
mod render_validation_tests;
#[cfg(test)]
mod snapshot_descriptor_tests;
#[cfg(test)]
mod snapshot_error_tests;
#[cfg(test)]
mod snapshot_schema_tests;
#[cfg(test)]
mod snapshot_validation_tests;
#[cfg(all(test, unix))]
mod sparse_generation_tests;
#[cfg(test)]
mod typed_data_array_evidence_tests;
#[cfg(test)]
mod typed_data_codegen_tests;
#[cfg(test)]
mod typed_data_edge_evidence_tests;
#[cfg(test)]
mod typed_data_limit_tests;

#[cfg(test)]
mod test_generator;

#[cfg(all(test, unix))]
mod check_generated_tests;

#[cfg(test)]
mod semantic_layout_tests;
