//! Semantic validation for untrusted compatibility snapshots.

use std::collections::BTreeSet;

use musubi_core::{
    ArtifactKind, FixedBytes, is_canonical_artifact_target, is_canonical_compiler_version,
};

use crate::{CompatibilitySnapshot, HardLimits, SnapshotError};

pub(crate) fn validate(snapshot: &CompatibilitySnapshot) -> Result<(), SnapshotError> {
    validate_with_limits(snapshot, HardLimits::V1)
}

pub(crate) fn validate_with_limits(
    snapshot: &CompatibilitySnapshot,
    limits: HardLimits,
) -> Result<(), SnapshotError> {
    if snapshot.schema_version != crate::snapshot::SNAPSHOT_SCHEMA_VERSION
        || snapshot.manifest_schema != crate::manifest::CONFIG_V4
        || !hash_nonzero(&snapshot.manifest_sha256)
        || snapshot.generator.validate().is_err()
        || snapshot.runtime.validate().is_err()
        || snapshot.generator_abi_version != crate::GENERATED_ABI_VERSION
        || snapshot.runtime_abi_version != musubi_core::GENERATED_ABI_VERSION
        || !crate::package_identity::valid(
            &snapshot.package_name,
            &snapshot.package_version,
            limits.identifier_bytes,
        )
        || (snapshot.contracts.is_empty() && snapshot.typed_data.is_empty())
        || snapshot.contracts.len() > limits.contracts
        || snapshot.typed_data.len() > limits.typed_data_entries
        || (!snapshot.contracts.is_empty() && !strict_contract_order(snapshot))
        || (!snapshot.typed_data.is_empty() && !strict_typed_order(snapshot))
    {
        return Err(SnapshotError::InvalidJson);
    }
    let mut ids = BTreeSet::new();
    let mut modules = BTreeSet::from(["musubi_core", "musubi_runtime"]);
    for contract in &snapshot.contracts {
        validate_identity(
            &contract.id,
            &contract.rust_module,
            &mut ids,
            &mut modules,
            limits,
        )?;
        let valid_capability = match contract.kind {
            ArtifactKind::Deployable => contract.deployment.is_some(),
            ArtifactKind::Interface => {
                contract.deployment.is_none()
                    && contract.abi.constructor.is_none()
                    && contract.provenance.storage_layout_sha256.is_none()
            }
        };
        if !valid_capability {
            return Err(SnapshotError::InvalidJson);
        }
        crate::snapshot_contract_validation::validate(contract, limits)?;
    }
    let mut evidence_bytes = 0_usize;
    for typed in &snapshot.typed_data {
        validate_identity(
            &typed.id,
            &typed.rust_module,
            &mut ids,
            &mut modules,
            limits,
        )?;
        let remaining = limits
            .generated_bytes
            .checked_sub(evidence_bytes)
            .ok_or(SnapshotError::InvalidJson)?;
        let bytes = crate::typed_data_snapshot_validation::validate(
            &typed.types,
            &typed.roots,
            &typed.root_evidence,
            limits,
            remaining,
        )
        .map_err(|_| SnapshotError::InvalidJson)?;
        evidence_bytes = evidence_bytes
            .checked_add(bytes)
            .ok_or(SnapshotError::InvalidJson)?;
        if !hash_nonzero(&typed.schema_sha256)
            || !hash_nonzero(&typed.vectors_sha256)
            || !hash_nonzero(&typed.evidence.solidity_source_sha256)
            || !hash_nonzero(&typed.evidence.artifact_sha256)
            || !hash_nonzero(&typed.evidence.compiler_settings_sha256)
            || !hash_nonzero(&typed.evidence.vector_command_sha256)
            || !is_canonical_artifact_target(&typed.evidence.artifact_target)
            || !is_canonical_compiler_version(&typed.evidence.compiler_version)
        {
            return Err(SnapshotError::InvalidJson);
        }
    }
    Ok(())
}

pub(crate) fn validate_typed_hash_text(document: &serde_json::Value) -> Result<(), SnapshotError> {
    let typed_data = document
        .get("typed_data")
        .and_then(serde_json::Value::as_array)
        .ok_or(SnapshotError::InvalidJson)?;
    for typed in typed_data {
        for pointer in ["/schema_sha256", "/vectors_sha256"] {
            if !typed.pointer(pointer).is_some_and(canonical_nonzero_hash) {
                return Err(SnapshotError::InvalidJson);
            }
        }
        validate_root_hash_text(typed)?;
        for pointer in [
            "/evidence/solidity_source_sha256",
            "/evidence/artifact_sha256",
            "/evidence/compiler_settings_sha256",
            "/evidence/vector_command_sha256",
        ] {
            if !typed.pointer(pointer).is_some_and(canonical_nonzero_hash) {
                return Err(SnapshotError::InvalidJson);
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_contract_hash_text(
    document: &serde_json::Value,
) -> Result<(), SnapshotError> {
    if !document
        .get("manifest_sha256")
        .is_some_and(canonical_nonzero_hash)
    {
        return Err(SnapshotError::InvalidJson);
    }
    let contracts = document
        .get("contracts")
        .and_then(serde_json::Value::as_array)
        .ok_or(SnapshotError::InvalidJson)?;
    for contract in contracts {
        for pointer in [
            "/provenance/canonical_abi_sha256",
            "/provenance/raw_abi_sha256",
        ] {
            if !contract
                .pointer(pointer)
                .is_some_and(canonical_nonzero_hash)
            {
                return Err(SnapshotError::InvalidJson);
            }
        }
        for pointer in [
            "/provenance/ast_sha256",
            "/provenance/metadata_sha256",
            "/provenance/storage_layout_sha256",
            "/deployment/runtime/template_keccak256",
        ] {
            if contract
                .pointer(pointer)
                .is_some_and(|value| !value.is_null() && !canonical_nonzero_hash(value))
            {
                return Err(SnapshotError::InvalidJson);
            }
        }
        if contract
            .pointer("/deployment")
            .is_some_and(|value| !value.is_null())
            && !contract
                .pointer("/deployment/creation_template_keccak256")
                .is_some_and(canonical_nonzero_hash)
        {
            return Err(SnapshotError::InvalidJson);
        }
    }
    Ok(())
}

fn validate_root_hash_text(typed: &serde_json::Value) -> Result<(), SnapshotError> {
    let roots = typed
        .get("root_evidence")
        .and_then(serde_json::Value::as_array)
        .ok_or(SnapshotError::InvalidJson)?;
    for root in roots {
        if !root.get("type_hash").is_some_and(canonical_nonzero_hash)
            || root
                .get("domain_type_hash")
                .is_some_and(|value| !value.is_null() && !canonical_nonzero_hash(value))
        {
            return Err(SnapshotError::InvalidJson);
        }
    }
    Ok(())
}

fn canonical_nonzero_hash(value: &serde_json::Value) -> bool {
    let Some(raw) = value.as_str().and_then(|value| value.strip_prefix("0x")) else {
        return false;
    };
    raw.len() == 64
        && raw
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && raw.bytes().any(|byte| byte != b'0')
}

fn validate_identity<'a>(
    id: &'a str,
    module: &'a str,
    ids: &mut BTreeSet<&'a str>,
    modules: &mut BTreeSet<&'a str>,
    limits: HardLimits,
) -> Result<(), SnapshotError> {
    if id.len() > limits.identifier_bytes
        || module.len() > limits.identifier_bytes
        || !ids.insert(id)
        || !modules.insert(module)
        || crate::names::validate_abi_name(id).is_err()
        || crate::names::module_name(Some(module), id).as_deref() != Ok(module)
    {
        return Err(SnapshotError::InvalidJson);
    }
    Ok(())
}

fn strict_contract_order(snapshot: &CompatibilitySnapshot) -> bool {
    snapshot
        .contracts
        .windows(2)
        .all(|pair| pair[0].id < pair[1].id)
}

fn strict_typed_order(snapshot: &CompatibilitySnapshot) -> bool {
    snapshot
        .typed_data
        .windows(2)
        .all(|pair| pair[0].id < pair[1].id)
}

fn hash_nonzero(value: &FixedBytes<32>) -> bool {
    value.as_ref().iter().any(|byte| *byte != 0)
}
