//! Bounded manifest reading and cross-entry capability validation.

use std::collections::BTreeSet;

use musubi_core::ArtifactKind;

use crate::{
    Diagnostic, DiagnosticCode, HardLimits, InputError, Manifest, input::failure,
    manifest::CONFIG_V4,
};

pub(crate) fn validate_manifest(manifest: &Manifest, limits: HardLimits) -> Result<(), InputError> {
    if manifest.schema_version != CONFIG_V4
        || manifest.package.edition != "2024"
        || !crate::package_identity::valid(
            &manifest.package.name,
            &manifest.package.version,
            limits.identifier_bytes,
        )
    {
        return Err(failure(Diagnostic::error(
            DiagnosticCode::InvalidInput,
            "manifest schema or edition is unsupported",
        )));
    }
    manifest.runtime.validate().map_err(|_| {
        failure(Diagnostic::error(
            DiagnosticCode::InvalidRuntimeSource,
            "manifest runtime source is invalid",
        ))
    })?;
    if (manifest.contracts.is_empty() && manifest.typed_data.is_empty())
        || manifest.contracts.len() > limits.contracts
        || manifest.typed_data.len() > limits.typed_data_entries
    {
        return Err(failure(Diagnostic::error(
            DiagnosticCode::LimitExceeded,
            "manifest entry count is outside effective versioned limits",
        )));
    }
    let mut ids = BTreeSet::new();
    for contract in &manifest.contracts {
        validate_contract(contract, &mut ids, limits)?;
    }
    for typed in &manifest.typed_data {
        if !ids.insert(typed.id.clone()) {
            return Err(failure(Diagnostic::error(
                DiagnosticCode::InvalidInput,
                "manifest entry identifiers must be unique",
            )));
        }
        for identifier in [&typed.id, &typed.rust_module] {
            validate_identifier(
                identifier,
                "typed-data identifier length is outside config-v4 limits",
                limits.identifier_bytes,
            )?;
        }
    }
    Ok(())
}

fn validate_contract(
    contract: &crate::ContractConfig,
    ids: &mut BTreeSet<String>,
    limits: HardLimits,
) -> Result<(), InputError> {
    if !ids.insert(contract.id.clone()) {
        return Err(failure(Diagnostic::error(
            DiagnosticCode::InvalidInput,
            "contract identifiers must be unique",
        )));
    }
    for identifier in [Some(&contract.id), contract.rust_module.as_ref()]
        .into_iter()
        .flatten()
    {
        validate_identifier(
            identifier,
            "identifier length is outside effective limits",
            limits.identifier_bytes,
        )?;
    }
    match contract.kind {
        ArtifactKind::Deployable if contract.creation_bytecode.is_none() => {
            Err(failure(Diagnostic::error(
                DiagnosticCode::InvalidInput,
                "deployable contract requires creation_bytecode",
            )))
        }
        ArtifactKind::Interface
            if contract.creation_bytecode.is_some()
                || contract.runtime_bytecode.is_some()
                || contract.storage_layout.is_some() =>
        {
            Err(failure(Diagnostic::error(
                DiagnosticCode::InvalidInput,
                "interface contract forbids bytecode and storage-layout inputs",
            )))
        }
        ArtifactKind::Deployable | ArtifactKind::Interface => Ok(()),
    }
}

fn validate_identifier(identifier: &str, message: &str, maximum: usize) -> Result<(), InputError> {
    if identifier.is_empty() || identifier.len() > maximum {
        return Err(failure(Diagnostic::error(
            DiagnosticCode::LimitExceeded,
            message,
        )));
    }
    crate::names::validate_abi_name(identifier).map_err(|_| {
        failure(Diagnostic::error(
            DiagnosticCode::InvalidInput,
            "manifest entry identifier is invalid",
        ))
    })
}
