//! Current-schema release evidence; every recorded identity remains independent.

use crate::{GenerationIr, RenderError, render_source::serialize_pretty};
use musubi_core::{ArtifactKind, GeneratorInfo, RuntimeSource};
use serde::Serialize;

pub(crate) fn release_manifest(
    ir: &GenerationIr,
    generator: &GeneratorInfo,
    maximum_bytes: usize,
) -> Result<Vec<u8>, RenderError> {
    let contracts = ir.contracts.iter().map(ReleaseContract::new).collect();
    let typed_data = ir.typed_data.iter().map(ReleaseTypedData::new).collect();
    serialize_pretty(
        &ReleaseManifest {
            contracts,
            generated_abi_version: 2,
            generator,
            package: ReleasePackage {
                name: &ir.package_name,
                version: &ir.package_version,
            },
            runtime: &ir.runtime,
            schema_version: "musubi/release-v4",
            manifest_schema: crate::manifest::CONFIG_V4,
            manifest_sha256: ir.manifest_sha256,
            typed_data,
        },
        maximum_bytes,
    )
}

#[derive(Serialize)]
struct ReleaseManifest<'a> {
    contracts: Vec<ReleaseContract<'a>>,
    generated_abi_version: u32,
    generator: &'a GeneratorInfo,
    manifest_schema: &'static str,
    manifest_sha256: musubi_core::Sha256Digest,
    package: ReleasePackage<'a>,
    runtime: &'a RuntimeSource,
    schema_version: &'static str,
    typed_data: Vec<ReleaseTypedData<'a>>,
}

#[derive(Serialize)]
struct ReleasePackage<'a> {
    name: &'a str,
    version: &'a str,
}

#[derive(Serialize)]
struct ReleaseContract<'a> {
    ast_sha256: Option<String>,
    canonical_abi_sha256: String,
    creation_template_keccak256: Option<String>,
    external_artifact_coherence: &'static str,
    id: &'a str,
    kind: ArtifactKind,
    metadata_sha256: Option<String>,
    raw_abi_sha256: String,
    runtime_template_keccak256: Option<String>,
    rust_module: &'a str,
    storage_layout_sha256: Option<String>,
}

impl<'a> ReleaseContract<'a> {
    fn new(contract: &'a crate::ContractIr) -> Self {
        Self {
            ast_sha256: contract.auxiliary_digests.ast.map(hex::encode),
            canonical_abi_sha256: hex::encode(contract.canonical_abi_sha256),
            creation_template_keccak256: contract
                .creation
                .as_ref()
                .map(|value| hex::encode(value.template_hash())),
            external_artifact_coherence: "unverified",
            id: &contract.id,
            kind: contract.kind,
            metadata_sha256: contract.auxiliary_digests.metadata.map(hex::encode),
            raw_abi_sha256: hex::encode(contract.raw_abi_sha256),
            runtime_template_keccak256: contract
                .runtime
                .as_ref()
                .map(|value| hex::encode(value.template_hash())),
            rust_module: &contract.module,
            storage_layout_sha256: contract.auxiliary_digests.storage_layout.map(hex::encode),
        }
    }
}

#[derive(Serialize)]
struct ReleaseTypedData<'a> {
    evidence: ReleaseEvidence<'a>,
    id: &'a str,
    roots: Vec<ReleaseRoot<'a>>,
    rust_module: &'a str,
    schema_sha256: musubi_core::Sha256Digest,
    vectors_sha256: musubi_core::Sha256Digest,
}

impl<'a> ReleaseTypedData<'a> {
    fn new(typed: &'a crate::TypedDataIr) -> Self {
        Self {
            evidence: ReleaseEvidence::new(&typed.evidence),
            id: &typed.id,
            roots: typed.root_evidence.iter().map(ReleaseRoot::new).collect(),
            rust_module: &typed.module,
            schema_sha256: typed.schema_sha256,
            vectors_sha256: typed.vectors_sha256,
        }
    }
}

#[derive(Serialize)]
struct ReleaseRoot<'a> {
    domain_type: Option<&'a String>,
    domain_type_hash: Option<musubi_core::Hash>,
    domain_type_string: Option<&'a String>,
    primary_type: &'a str,
    type_hash: musubi_core::Hash,
    type_string: &'a str,
}

impl<'a> ReleaseRoot<'a> {
    fn new(root: &'a crate::TypedDataRootEvidence) -> Self {
        Self {
            domain_type: root.domain_type.as_ref(),
            domain_type_hash: root.domain_type_hash,
            domain_type_string: root.domain_type_string.as_ref(),
            primary_type: &root.primary_type,
            type_hash: root.type_hash,
            type_string: &root.type_string,
        }
    }
}

#[derive(Serialize)]
struct ReleaseEvidence<'a> {
    artifact_sha256: musubi_core::Sha256Digest,
    artifact_target: &'a str,
    compiler_settings_sha256: musubi_core::Sha256Digest,
    compiler_version: &'a str,
    solidity_source_sha256: musubi_core::Sha256Digest,
    vector_command_sha256: musubi_core::Sha256Digest,
}

impl<'a> ReleaseEvidence<'a> {
    fn new(evidence: &'a crate::TypedDataExternalEvidence) -> Self {
        Self {
            artifact_sha256: evidence.artifact_sha256,
            artifact_target: &evidence.artifact_target,
            compiler_settings_sha256: evidence.compiler_settings_sha256,
            compiler_version: &evidence.compiler_version,
            solidity_source_sha256: evidence.solidity_source_sha256,
            vector_command_sha256: evidence.vector_command_sha256,
        }
    }
}
