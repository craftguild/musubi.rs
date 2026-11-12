//! Exact release-to-file inventory, asset identity, and compiler input validation.

use std::{collections::BTreeSet, fmt::Write as _};

use musubi_core::{ArtifactKind, keccak256, sha256};

use crate::{
    GeneratedCrateError, generated_identity::valid_rust_module, generated_release::ValidRelease,
    generated_tree::TreeSnapshot,
};

const ROOT_PREAMBLE: &str = "//! Deterministically generated typed EVM contract SDK.\n\n\
#![forbid(unsafe_code)]\n#![warn(missing_docs)]\n\n\
musubi_runtime::assert_generated_abi!(2);\n\n";

pub(crate) fn validate(
    snapshot: &TreeSnapshot,
    release: &ValidRelease,
) -> Result<(), GeneratedCrateError> {
    validate_root_source(snapshot, release)?;
    validate_assets(snapshot, release)?;
    crate::generated_layout::validate(snapshot, release)
}

fn validate_root_source(
    snapshot: &TreeSnapshot,
    release: &ValidRelease,
) -> Result<(), GeneratedCrateError> {
    let mut expected = String::from(ROOT_PREAMBLE);
    expected.push_str(&crate::generated_provenance::root_source(
        &release.manifest_schema,
        &release.manifest_sha256,
        &release.runtime,
        &release.generator,
    ));
    let mut modules = BTreeSet::new();
    for contract in &release.contracts {
        let module = contract.rust_module.as_str();
        if !valid_rust_module(module) || !modules.insert(module) {
            return Err(GeneratedCrateError::InvalidLayout);
        }
        writeln!(
            expected,
            "/// Typed SDK for manifest contract `{}`.\n#[rustfmt::skip]\npub mod {module};",
            contract.id
        )
        .expect("String formatting cannot fail");
    }
    for typed in &release.typed_data {
        if !valid_rust_module(&typed.rust_module) || !modules.insert(&typed.rust_module) {
            return Err(GeneratedCrateError::InvalidLayout);
        }
        writeln!(
            expected,
            "/// Owned EIP-712 authority `{}`.\n#[rustfmt::skip]\npub mod {};",
            typed.id, typed.rust_module
        )
        .expect("String formatting cannot fail");
    }
    if snapshot.file("src/lib.rs")? != expected.as_bytes() {
        return Err(GeneratedCrateError::InvalidLayout);
    }
    syn::parse_file(&expected).map_err(invalid)?;
    Ok(())
}

fn validate_assets(
    snapshot: &TreeSnapshot,
    release: &ValidRelease,
) -> Result<(), GeneratedCrateError> {
    for contract in &release.contracts {
        let base = format!("assets/{}", contract.id);
        if hex::encode(sha256(snapshot.file(&format!("{base}/abi.json"))?))
            != contract.canonical_abi_sha256
        {
            return Err(GeneratedCrateError::InvalidLayout);
        }
        validate_bytecode(
            snapshot,
            &base,
            "creation.bin",
            contract.creation_template_keccak256.as_deref(),
        )?;
        validate_bytecode(
            snapshot,
            &base,
            "runtime.bin",
            contract.runtime_template_keccak256.as_deref(),
        )?;
        if contract.kind == ArtifactKind::Interface
            && (contract.creation_template_keccak256.is_some()
                || contract.runtime_template_keccak256.is_some())
        {
            return Err(GeneratedCrateError::InvalidLayout);
        }
    }
    Ok(())
}

fn validate_bytecode(
    snapshot: &TreeSnapshot,
    base: &str,
    file: &str,
    expected: Option<&str>,
) -> Result<(), GeneratedCrateError> {
    if let Some(expected) = expected
        && hex::encode(keccak256(snapshot.file(&format!("{base}/{file}"))?)) != expected
    {
        return Err(GeneratedCrateError::InvalidLayout);
    }
    Ok(())
}

fn invalid<T>(_: T) -> GeneratedCrateError {
    GeneratedCrateError::InvalidLayout
}
