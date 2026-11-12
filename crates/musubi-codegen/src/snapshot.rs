//! Canonical compatibility snapshot model and strict serialization boundary.

use musubi_core::{
    ArtifactKind, ContractAbi, ExternalArtifactCoherence, ImmutableReference, Keccak256Digest,
    LinkReference, Sha256Digest, TypedDataRoot, TypedDataStruct,
};
use serde::{Deserialize, Serialize};

use crate::{Enrichment, GenerationIr, TypedDataExternalEvidence, TypedDataRootEvidence};

/// Current compatibility snapshot schema.
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 4;

/// Complete canonical compatibility snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilitySnapshot {
    /// Closed snapshot schema version.
    pub schema_version: u32,
    /// Exact manifest schema authority.
    pub manifest_schema: String,
    /// SHA-256 of exact manifest bytes, independent of the artifact identities.
    pub manifest_sha256: Sha256Digest,
    /// Generator build information that created this snapshot.
    pub generator: musubi_core::GeneratorInfo,
    /// Explicit runtime source, relative to the generated SDK for paths.
    pub runtime: musubi_core::RuntimeSource,
    /// ABI contract emitted by the generator.
    pub generator_abi_version: u32,
    /// ABI contract expected by the runtime.
    pub runtime_abi_version: u32,
    /// Generated package name.
    pub package_name: String,
    /// Generated package version.
    pub package_version: String,
    /// Contracts sorted by stable manifest identifier.
    pub contracts: Vec<ContractSnapshot>,
    /// Typed-data authorities sorted by stable manifest identifier.
    pub typed_data: Vec<TypedDataSnapshot>,
}

/// Wire, Rust, deployment, and provenance surface for one contract.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractSnapshot {
    /// Stable manifest-local contract identifier.
    pub id: String,
    /// Generated Rust module name.
    pub rust_module: String,
    /// Explicit compiler-artifact capability class.
    pub kind: ArtifactKind,
    /// ABI-authoritative descriptor, including source-facing names.
    pub abi: ContractAbi,
    /// AST-derived Rust type identities.
    pub enrichments: Vec<Enrichment>,
    /// Bytecode identity and reference layout.
    pub deployment: Option<DeploymentSnapshot>,
    /// Input identity and explicit trust-boundary status.
    pub provenance: ProvenanceSnapshot,
}

/// Bytecode template identity relevant to deployment compatibility.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentSnapshot {
    /// Keccak-256 of the unlinked creation template.
    pub creation_template_keccak256: Keccak256Digest,
    /// Exact nonzero creation-template length.
    pub creation_template_bytes: usize,
    /// Source-qualified creation-bytecode link locations.
    pub creation_links: Vec<LinkReference>,
    /// Optional runtime verification identity.
    pub runtime: Option<RuntimeDeploymentSnapshot>,
}

/// Runtime template identity, link references, and immutable masks.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeDeploymentSnapshot {
    /// Keccak-256 of the runtime template.
    pub template_keccak256: Keccak256Digest,
    /// Exact nonzero runtime-template length.
    pub template_bytes: usize,
    /// Source-qualified runtime link locations.
    pub links: Vec<LinkReference>,
    /// Constructor-populated runtime locations excluded during verification.
    pub immutables: Vec<ImmutableReference>,
}

/// Exact external-input identity without source bodies or paths.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceSnapshot {
    /// SHA-256 of canonical ABI JSON.
    pub canonical_abi_sha256: Sha256Digest,
    /// SHA-256 of exact external ABI bytes.
    pub raw_abi_sha256: Sha256Digest,
    /// SHA-256 of optional AST bytes.
    pub ast_sha256: Option<Sha256Digest>,
    /// SHA-256 of optional compiler metadata bytes.
    pub metadata_sha256: Option<Sha256Digest>,
    /// SHA-256 of optional storage-layout bytes.
    pub storage_layout_sha256: Option<Sha256Digest>,
    /// Explicit statement that cross-artifact compiler coherence is not proven.
    pub external_artifact_coherence: ExternalArtifactCoherence,
}

/// Closed typed-data schema, computed roots, and reviewed evidence identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedDataSnapshot {
    /// Stable manifest-local typed-data identifier.
    pub id: String,
    /// Generated Rust module name.
    pub rust_module: String,
    /// Named EIP-712 type graph.
    pub types: Vec<TypedDataStruct>,
    /// Declared primary roots and optional domains.
    pub roots: Vec<TypedDataRoot>,
    /// Canonical strings and hashes for every root.
    pub root_evidence: Vec<TypedDataRootEvidence>,
    /// SHA-256 of exact schema bytes.
    pub schema_sha256: Sha256Digest,
    /// SHA-256 of exact vector bundle bytes.
    pub vectors_sha256: Sha256Digest,
    /// Digested reviewed Solidity provenance.
    pub evidence: TypedDataExternalEvidence,
}

/// Strict compatibility snapshot parse or serialization error.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SnapshotError {
    /// JSON syntax or the closed schema is invalid.
    #[error("compatibility snapshot is not valid closed-schema JSON")]
    InvalidJson,
    /// Snapshot schema is not supported by this build.
    #[error("compatibility snapshot schema is unsupported")]
    UnsupportedSchema,
    /// Canonical serialization failed.
    #[error("compatibility snapshot serialization failed")]
    Serialization,
}

/// Builds a deterministic snapshot from validated generation IR.
#[must_use]
pub fn build_snapshot(
    ir: &GenerationIr,
    generator: &musubi_core::GeneratorInfo,
) -> CompatibilitySnapshot {
    CompatibilitySnapshot {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        manifest_schema: crate::manifest::CONFIG_V4.into(),
        manifest_sha256: ir.manifest_sha256,
        generator: generator.clone(),
        runtime: ir.runtime.clone(),
        generator_abi_version: crate::GENERATED_ABI_VERSION,
        runtime_abi_version: musubi_core::GENERATED_ABI_VERSION,
        package_name: ir.package_name.clone(),
        package_version: ir.package_version.clone(),
        contracts: ir.contracts.iter().map(contract_snapshot).collect(),
        typed_data: ir
            .typed_data
            .iter()
            .map(|typed| TypedDataSnapshot {
                id: typed.id.clone(),
                rust_module: typed.module.clone(),
                types: typed.types.clone(),
                roots: typed.roots.clone(),
                root_evidence: typed.root_evidence.clone(),
                schema_sha256: typed.schema_sha256,
                vectors_sha256: typed.vectors_sha256,
                evidence: typed.evidence.clone(),
            })
            .collect(),
    }
}

/// Serializes a snapshot as stable pretty JSON ending in one newline.
///
/// # Errors
///
/// Returns [`SnapshotError::Serialization`] if the in-memory model cannot be encoded.
pub fn snapshot_json(snapshot: &CompatibilitySnapshot) -> Result<Vec<u8>, SnapshotError> {
    let mut bytes =
        serde_json::to_vec_pretty(snapshot).map_err(|_| SnapshotError::Serialization)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Parses a closed-schema snapshot and rejects unknown schema versions.
///
/// # Errors
///
/// Returns [`SnapshotError`] for malformed JSON, unknown fields, or unsupported schema versions.
pub fn parse_snapshot(bytes: &[u8]) -> Result<CompatibilitySnapshot, SnapshotError> {
    if bytes.len() > crate::HardLimits::V1.generated_bytes {
        return Err(SnapshotError::InvalidJson);
    }
    let document = crate::closed_json::parse(bytes).map_err(|()| SnapshotError::InvalidJson)?;
    let probe: SnapshotSchemaProbe =
        serde_json::from_slice(bytes).map_err(|_| SnapshotError::InvalidJson)?;
    if probe.schema_version != SNAPSHOT_SCHEMA_VERSION {
        return Err(SnapshotError::UnsupportedSchema);
    }
    crate::snapshot_validation::validate_contract_hash_text(&document)?;
    crate::snapshot_validation::validate_typed_hash_text(&document)?;
    let snapshot = serde_json::from_slice(bytes).map_err(|_| SnapshotError::InvalidJson)?;
    crate::snapshot_validation::validate(&snapshot)?;
    Ok(snapshot)
}

fn contract_snapshot(contract: &crate::ContractIr) -> ContractSnapshot {
    ContractSnapshot {
        id: contract.id.clone(),
        rust_module: contract.module.clone(),
        kind: contract.kind,
        abi: contract.abi.clone(),
        enrichments: contract.enrichments.clone(),
        deployment: contract
            .creation
            .as_ref()
            .map(|creation| DeploymentSnapshot {
                creation_template_keccak256: creation.template_hash(),
                creation_template_bytes: creation.template.len(),
                creation_links: creation.links.clone(),
                runtime: contract
                    .runtime
                    .as_ref()
                    .map(|runtime| RuntimeDeploymentSnapshot {
                        template_keccak256: runtime.template_hash(),
                        template_bytes: runtime.template.len(),
                        links: runtime.links.clone(),
                        immutables: runtime.immutables.clone(),
                    }),
            }),
        provenance: ProvenanceSnapshot {
            canonical_abi_sha256: contract.canonical_abi_sha256,
            raw_abi_sha256: contract.raw_abi_sha256,
            ast_sha256: contract.auxiliary_digests.ast,
            metadata_sha256: contract.auxiliary_digests.metadata,
            storage_layout_sha256: contract.auxiliary_digests.storage_layout,
            external_artifact_coherence: ExternalArtifactCoherence::Unverified,
        },
    }
}

#[derive(Deserialize)]
struct SnapshotSchemaProbe {
    schema_version: u32,
}
