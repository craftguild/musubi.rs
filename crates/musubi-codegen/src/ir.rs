//! Canonically ordered generator IR combining ABI authority with optional enrichment.

use musubi_core::{
    AbiField, AbiType, ArtifactKind, ContractAbi, CreationBytecode, RuntimeBytecode, Sha256Digest,
    TypedDataRoot, TypedDataStruct, sha256,
};
use serde::{Deserialize, Serialize};

use crate::{
    Diagnostic, Diagnostics, InputBundle,
    abi_parse::parse_abi,
    ast::enrich,
    input::{AuxiliaryDigests, one},
    name_validation::validate_generated_names,
    names::module_name,
    owned_identity::OwnedTypeRegistry,
};

/// One safe AST-derived developer-facing type identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enrichment {
    /// Solidity `internalType` identity matched to the AST.
    pub internal_type: String,
    /// Deterministic generated Rust type name.
    pub rust_name: String,
    /// Enrichment-specific shape validated against ABI.
    pub kind: EnrichmentKind,
}

/// Closed AST enrichment variants.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EnrichmentKind {
    /// Closed Solidity enum.
    Enum {
        /// AST-declared variant names in discriminant order.
        variants: Vec<String>,
        /// ABI-authoritative integer representation.
        underlying: AbiType,
    },
    /// Transparent user-defined value type.
    UserDefinedValueType {
        /// ABI-authoritative underlying type.
        underlying: AbiType,
    },
    /// Named Solidity struct represented by an ABI tuple.
    Struct {
        /// ABI-authoritative tuple fields.
        fields: Vec<AbiField>,
    },
}

/// Canonical contract-level generation input.
#[derive(Clone, Debug)]
pub struct ContractIr {
    /// Stable logical identifier.
    pub id: String,
    /// Validated Rust module name.
    pub module: String,
    /// Explicit compiler-artifact capability class.
    pub kind: ArtifactKind,
    /// ABI-authoritative wire descriptor.
    pub abi: ContractAbi,
    /// Canonical standard ABI JSON bytes stored as a generated asset.
    pub canonical_abi: Vec<u8>,
    /// Canonical serialized ABI descriptor hash.
    pub canonical_abi_sha256: Sha256Digest,
    /// Exact raw ABI input hash.
    pub raw_abi_sha256: Sha256Digest,
    /// Creation bytecode template for deployable artifacts.
    pub creation: Option<CreationBytecode>,
    /// Optional runtime verification template.
    pub runtime: Option<RuntimeBytecode>,
    /// AST enrichment identities that passed ABI compatibility checks.
    pub enrichments: Vec<Enrichment>,
    /// Pre-resolved owned identities and anonymous tuple declarations.
    pub(crate) owned_types: OwnedTypeRegistry,
    /// Digests for optional external inputs.
    pub auxiliary_digests: AuxiliaryDigests,
}

/// Canonical typed-data generation input and reviewed evidence identity.
#[derive(Clone, Debug)]
pub struct TypedDataIr {
    /// Stable logical identifier.
    pub id: String,
    /// Validated unique Rust module name.
    pub module: String,
    /// Closed named type graph.
    pub types: Vec<TypedDataStruct>,
    /// Declared roots with optional explicit domains.
    pub roots: Vec<TypedDataRoot>,
    /// Canonical root strings and hashes.
    pub root_evidence: Vec<TypedDataRootEvidence>,
    /// SHA-256 of exact schema bytes.
    pub schema_sha256: Sha256Digest,
    /// SHA-256 of exact vector-bundle bytes.
    pub vectors_sha256: Sha256Digest,
    /// Reviewed external Solidity evidence.
    pub evidence: TypedDataExternalEvidence,
}

/// Canonical computed identity for one typed-data root and optional domain.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedDataRootEvidence {
    /// Primary struct name.
    pub primary_type: String,
    /// Canonical primary encodeType string.
    pub type_string: String,
    /// Keccak-256 of the primary type string.
    pub type_hash: musubi_core::Hash,
    /// Explicit domain struct name when final digest is enabled.
    pub domain_type: Option<String>,
    /// Canonical domain encodeType string.
    pub domain_type_string: Option<String>,
    /// Keccak-256 of the domain type string.
    pub domain_type_hash: Option<musubi_core::Hash>,
}

/// Digested provenance linking typed-data inputs to reviewed Solidity output.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedDataExternalEvidence {
    /// SHA-256 of reviewed Solidity source bytes.
    pub solidity_source_sha256: Sha256Digest,
    /// Reviewed Foundry artifact target identifier.
    pub artifact_target: String,
    /// SHA-256 of the reviewed compiler artifact.
    pub artifact_sha256: Sha256Digest,
    /// Exact compiler version identity.
    pub compiler_version: String,
    /// SHA-256 of canonical compiler settings evidence.
    pub compiler_settings_sha256: Sha256Digest,
    /// SHA-256 of the stable vector-generation command identity.
    pub vector_command_sha256: Sha256Digest,
}

/// Canonical workspace generation IR.
#[derive(Clone, Debug)]
pub struct GenerationIr {
    /// SHA-256 of exact manifest input bytes, independent of every artifact digest.
    pub manifest_sha256: Sha256Digest,
    /// Generated Cargo package name.
    pub package_name: String,
    /// Generated Cargo package version.
    pub package_version: String,
    /// Explicit core/runtime source; path roots are relative to the generated SDK.
    pub runtime: musubi_core::RuntimeSource,
    /// Contracts sorted by logical ID.
    pub contracts: Vec<ContractIr>,
    /// Typed-data authorities sorted by logical ID.
    pub typed_data: Vec<TypedDataIr>,
    /// Trust-boundary notices and enrichment degradation warnings.
    pub diagnostics: Diagnostics,
    /// Effective downward-only resource limits from the manifest.
    pub limits: crate::HardLimits,
}

/// Redacted ABI/AST IR construction failure.
#[derive(Clone, Debug, thiserror::Error)]
#[error("ABI or AST validation failed")]
pub struct IrError {
    /// Ordered stable diagnostics.
    pub diagnostics: Diagnostics,
}

/// Builds deterministic wire IR and optional AST enrichment.
///
/// # Errors
///
/// Returns [`IrError`] for malformed ABI types, selector/topic/name collisions, AST resource
/// exhaustion, or an ABI/AST underlying-type contradiction.
pub fn build_ir(bundle: InputBundle) -> Result<GenerationIr, IrError> {
    let mut diagnostics = bundle.diagnostics;
    let mut contracts = Vec::with_capacity(bundle.contracts.len());
    for input in bundle.contracts {
        let abi = parse_abi(&input.abi, bundle.limits).map_err(ir_failure)?;
        if input.config.kind == ArtifactKind::Interface && abi.constructor.is_some() {
            return Err(ir_failure(Diagnostic::error(
                crate::DiagnosticCode::InconsistentArtifact,
                "interface ABI must not declare a constructor",
            )));
        }
        let enrichments = enrich(&abi, input.ast.as_ref(), bundle.limits, &mut diagnostics)
            .map_err(ir_failure)?;
        let owned_types =
            OwnedTypeRegistry::build(&abi, &enrichments, bundle.limits).map_err(ir_failure)?;
        validate_generated_names(
            &input.config.id,
            input.config.kind,
            &abi,
            &enrichments,
            &owned_types,
        )
        .map_err(ir_failure)?;
        let canonical = crate::canonical::canonical_abi(&input.abi).map_err(|_| {
            ir_failure(Diagnostic::error(
                crate::DiagnosticCode::InconsistentArtifact,
                "canonical ABI descriptor cannot be serialized",
            ))
        })?;
        contracts.push(ContractIr {
            id: input.config.id.clone(),
            module: module_name(input.config.rust_module.as_deref(), &input.config.id)
                .map_err(ir_failure)?,
            kind: input.config.kind,
            abi,
            canonical_abi: canonical.clone(),
            canonical_abi_sha256: sha256(&canonical),
            raw_abi_sha256: sha256(&input.raw_abi),
            creation: input.creation,
            runtime: input.runtime,
            enrichments,
            owned_types,
            auxiliary_digests: input.auxiliary_digests,
        });
    }
    contracts.sort_by(|left, right| left.id.cmp(&right.id));
    let mut typed_data = Vec::with_capacity(bundle.typed_data.len());
    let mut evidence_bytes = 0_usize;
    for input in bundle.typed_data {
        let remaining = bundle
            .limits
            .generated_bytes
            .checked_sub(evidence_bytes)
            .ok_or_else(|| {
                ir_failure(Diagnostic::error(
                    crate::DiagnosticCode::LimitExceeded,
                    "typed-data evidence exceeds the aggregate generated-byte limit",
                ))
            })?;
        let (typed, bytes) = crate::typed_data_schema::build(input, bundle.limits, remaining)?;
        evidence_bytes = evidence_bytes.checked_add(bytes).ok_or_else(|| {
            ir_failure(Diagnostic::error(
                crate::DiagnosticCode::LimitExceeded,
                "typed-data evidence byte count overflowed",
            ))
        })?;
        typed_data.push(typed);
    }
    typed_data.sort_by(|left, right| left.id.cmp(&right.id));
    let mut modules =
        std::collections::BTreeSet::from(["musubi_core".to_owned(), "musubi_runtime".to_owned()]);
    if contracts
        .iter()
        .any(|contract| !modules.insert(contract.module.clone()))
    {
        return Err(ir_failure(Diagnostic::error(
            crate::DiagnosticCode::InconsistentArtifact,
            "contract identifiers collide in the generated Rust module namespace",
        )));
    }
    for typed in &typed_data {
        if !modules.insert(typed.module.clone()) {
            return Err(ir_failure(Diagnostic::error(
                crate::DiagnosticCode::InconsistentArtifact,
                "typed-data identifiers collide in the generated Rust module namespace",
            )));
        }
    }
    Ok(GenerationIr {
        runtime: crate::runtime_source::rebase(
            &bundle.manifest.runtime,
            &bundle.manifest.generation.output,
        ),
        manifest_sha256: bundle.manifest_sha256,
        package_name: bundle.manifest.package.name,
        package_version: bundle.manifest.package.version,
        contracts,
        typed_data,
        diagnostics,
        limits: bundle.limits,
    })
}

fn ir_failure(diagnostic: Diagnostic) -> IrError {
    IrError {
        diagnostics: one(diagnostic),
    }
}
