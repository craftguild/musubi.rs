//! Closed generated release-manifest parsing and validation.

use std::collections::BTreeSet;

use musubi_core::{
    ArtifactKind, ExternalArtifactCoherence, RuntimeSource, is_canonical_artifact_target,
    is_canonical_compiler_version, is_canonical_solidity_identifier, keccak256,
};
use serde::Deserialize;

use crate::{
    GeneratedCrateError,
    generated_identity::{valid_entry_id, valid_rust_module},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReleaseManifest {
    schema_version: String,
    manifest_schema: String,
    manifest_sha256: String,
    package: ReleasePackage,
    generator: musubi_core::GeneratorInfo,
    generated_abi_version: u32,
    runtime: RuntimeSource,
    contracts: Vec<ReleaseContract>,
    typed_data: Vec<ReleaseTypedData>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReleasePackage {
    pub(crate) name: String,
    pub(crate) version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseContract {
    id: String,
    rust_module: String,
    kind: ArtifactKind,
    canonical_abi_sha256: String,
    raw_abi_sha256: String,
    creation_template_keccak256: Option<String>,
    runtime_template_keccak256: Option<String>,
    ast_sha256: Option<String>,
    metadata_sha256: Option<String>,
    storage_layout_sha256: Option<String>,
    external_artifact_coherence: ExternalArtifactCoherence,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseTypedData {
    id: String,
    rust_module: String,
    schema_sha256: String,
    vectors_sha256: String,
    roots: Vec<ReleaseTypedDataRoot>,
    evidence: ReleaseTypedDataEvidence,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseTypedDataRoot {
    primary_type: String,
    type_string: String,
    type_hash: String,
    domain_type: Option<String>,
    domain_type_string: Option<String>,
    domain_type_hash: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseTypedDataEvidence {
    solidity_source_sha256: String,
    artifact_target: String,
    artifact_sha256: String,
    compiler_version: String,
    compiler_settings_sha256: String,
    vector_command_sha256: String,
}

impl ReleaseContract {
    fn valid(&self) -> bool {
        let kind_valid = match self.kind {
            ArtifactKind::Deployable => self.creation_template_keccak256.is_some(),
            ArtifactKind::Interface => {
                self.creation_template_keccak256.is_none()
                    && self.runtime_template_keccak256.is_none()
                    && self.storage_layout_sha256.is_none()
            }
        };
        kind_valid
            && valid_entry_id(&self.id)
            && valid_rust_module(&self.rust_module)
            && valid_nonzero_digest(&self.canonical_abi_sha256)
            && valid_nonzero_digest(&self.raw_abi_sha256)
            && self
                .creation_template_keccak256
                .as_deref()
                .is_none_or(valid_nonzero_digest)
            && self
                .runtime_template_keccak256
                .as_deref()
                .is_none_or(valid_nonzero_digest)
            && self.ast_sha256.as_deref().is_none_or(valid_nonzero_digest)
            && self
                .metadata_sha256
                .as_deref()
                .is_none_or(valid_nonzero_digest)
            && self
                .storage_layout_sha256
                .as_deref()
                .is_none_or(valid_nonzero_digest)
            && self.external_artifact_coherence == ExternalArtifactCoherence::Unverified
    }
}

impl ReleaseTypedData {
    fn valid(&self) -> bool {
        valid_entry_id(&self.id)
            && valid_rust_module(&self.rust_module)
            && valid_prefixed_nonzero_digest(&self.schema_sha256)
            && valid_prefixed_nonzero_digest(&self.vectors_sha256)
            && !self.roots.is_empty()
            && is_canonical_artifact_target(&self.evidence.artifact_target)
            && is_canonical_compiler_version(&self.evidence.compiler_version)
            && valid_prefixed_nonzero_digest(&self.evidence.solidity_source_sha256)
            && valid_prefixed_nonzero_digest(&self.evidence.artifact_sha256)
            && valid_prefixed_nonzero_digest(&self.evidence.compiler_settings_sha256)
            && valid_prefixed_nonzero_digest(&self.evidence.vector_command_sha256)
            && self.roots.iter().all(ReleaseTypedDataRoot::valid)
            && self.roots.windows(2).all(|pair| {
                (&pair[0].primary_type, &pair[0].domain_type)
                    < (&pair[1].primary_type, &pair[1].domain_type)
            })
    }
}

impl ReleaseTypedDataRoot {
    fn valid(&self) -> bool {
        let domain_valid = match (
            self.domain_type.as_deref(),
            self.domain_type_string.as_deref(),
            self.domain_type_hash.as_deref(),
        ) {
            (None, None, None) => true,
            (Some(name), Some(type_string), Some(type_hash)) => {
                valid_type_evidence(name, type_string, type_hash)
            }
            _ => false,
        };
        valid_type_evidence(&self.primary_type, &self.type_string, &self.type_hash) && domain_valid
    }
}

fn valid_type_evidence(name: &str, type_string: &str, type_hash: &str) -> bool {
    is_canonical_solidity_identifier(name)
        && crate::generated_type_string::canonical(name, type_string)
        && valid_prefixed_nonzero_digest(type_hash)
        && type_hash
            .strip_prefix("0x")
            .is_some_and(|expected| hex::encode(keccak256(type_string.as_bytes())) == expected)
}

pub(crate) struct ValidRelease {
    pub(crate) manifest_schema: String,
    pub(crate) manifest_sha256: musubi_core::Sha256Digest,
    pub(crate) package: ReleasePackage,
    pub(crate) generator: musubi_core::GeneratorInfo,
    pub(crate) generated_abi_version: u32,
    pub(crate) runtime: RuntimeSource,
    pub(crate) contracts: Vec<ValidContract>,
    pub(crate) typed_data: Vec<ValidTypedData>,
}

pub(crate) struct ValidContract {
    pub(crate) id: String,
    pub(crate) rust_module: String,
    pub(crate) kind: ArtifactKind,
    pub(crate) canonical_abi_sha256: String,
    pub(crate) creation_template_keccak256: Option<String>,
    pub(crate) runtime_template_keccak256: Option<String>,
}

pub(crate) struct ValidTypedData {
    pub(crate) id: String,
    pub(crate) rust_module: String,
    pub(crate) required_types: BTreeSet<String>,
}

impl ReleaseManifest {
    pub(crate) fn validate(self) -> Result<ValidRelease, GeneratedCrateError> {
        if self.schema_version != "musubi/release-v4"
            || self.manifest_schema != "musubi/config-v4"
            || !valid_prefixed_nonzero_digest(&self.manifest_sha256)
            || (self.contracts.is_empty() && self.typed_data.is_empty())
            || !self.contracts.iter().all(ReleaseContract::valid)
            || !self.typed_data.iter().all(ReleaseTypedData::valid)
            || !self
                .contracts
                .windows(2)
                .all(|pair| pair[0].id < pair[1].id)
            || !self
                .typed_data
                .windows(2)
                .all(|pair| pair[0].id < pair[1].id)
            || !unique_modules(&self.contracts, &self.typed_data)
        {
            return Err(GeneratedCrateError::InvalidLayout);
        }
        let mut digest = [0; 32];
        hex::decode_to_slice(&self.manifest_sha256[2..], &mut digest)
            .map_err(|_| GeneratedCrateError::InvalidLayout)?;
        Ok(ValidRelease {
            manifest_schema: self.manifest_schema,
            manifest_sha256: musubi_core::Sha256Digest::new(digest),
            package: self.package,
            generator: self.generator,
            generated_abi_version: self.generated_abi_version,
            runtime: self.runtime,
            contracts: self
                .contracts
                .into_iter()
                .map(|contract| ValidContract {
                    id: contract.id,
                    rust_module: contract.rust_module,
                    kind: contract.kind,
                    canonical_abi_sha256: contract.canonical_abi_sha256,
                    creation_template_keccak256: contract.creation_template_keccak256,
                    runtime_template_keccak256: contract.runtime_template_keccak256,
                })
                .collect(),
            typed_data: self
                .typed_data
                .into_iter()
                .map(|typed| ValidTypedData {
                    id: typed.id,
                    rust_module: typed.rust_module,
                    required_types: typed
                        .roots
                        .into_iter()
                        .flat_map(|root| std::iter::once(root.primary_type).chain(root.domain_type))
                        .collect(),
                })
                .collect(),
        })
    }
}

fn unique_modules(contracts: &[ReleaseContract], typed_data: &[ReleaseTypedData]) -> bool {
    let mut modules = BTreeSet::new();
    contracts
        .iter()
        .map(|entry| entry.rust_module.as_str())
        .chain(typed_data.iter().map(|entry| entry.rust_module.as_str()))
        .all(|module| modules.insert(module))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_nonzero_digest(value: &str) -> bool {
    valid_digest(value) && value.bytes().any(|byte| byte != b'0')
}

fn valid_prefixed_nonzero_digest(value: &str) -> bool {
    let Some(raw) = value.strip_prefix("0x") else {
        return false;
    };
    valid_nonzero_digest(raw)
}
