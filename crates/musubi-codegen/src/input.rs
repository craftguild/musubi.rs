//! Bounded loading with manifest-root path confinement.

use std::path::Path;

use musubi_core::{CreationBytecode, RuntimeBytecode, Sha256Digest, sha256};

use crate::{
    ContractConfig, Diagnostic, DiagnosticCode, Diagnostics, HardLimits, Manifest, TypedDataConfig,
    bytecode::{parse_creation, parse_runtime},
    input_fs::{InputRoot, load_optional, open_manifest, read_relative},
    input_manifest::validate_manifest,
};

/// Fully loaded and syntax-validated input for one contract.
#[derive(Clone, Debug)]
pub struct ContractInput {
    /// Manifest contract entry.
    pub config: ContractConfig,
    /// Exact ABI JSON input bytes.
    pub raw_abi: Vec<u8>,
    /// Syntax-validated ABI JSON array.
    pub abi: Vec<serde_json::Value>,
    /// Parsed creation bytecode for deployable artifacts.
    pub creation: Option<CreationBytecode>,
    /// Optional parsed runtime bytecode.
    pub runtime: Option<RuntimeBytecode>,
    /// Optional syntax-validated `SourceUnit` JSON.
    pub ast: Option<serde_json::Value>,
    /// Optional exact compiler metadata JSON bytes.
    pub metadata: Option<Vec<u8>>,
    /// Optional exact storage layout JSON bytes.
    pub storage_layout: Option<Vec<u8>>,
    /// Exact input digests for optional enrichment/provenance artifacts.
    pub auxiliary_digests: AuxiliaryDigests,
}

/// Optional input digests stored independently from source bodies.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AuxiliaryDigests {
    /// SHA-256 of the AST input.
    pub ast: Option<Sha256Digest>,
    /// SHA-256 of compiler metadata.
    pub metadata: Option<Sha256Digest>,
    /// SHA-256 of storage layout.
    pub storage_layout: Option<Sha256Digest>,
}

/// Exact closed schema and vector inputs for one typed-data authority.
#[derive(Clone, Debug)]
pub struct TypedDataInput {
    /// Manifest typed-data entry.
    pub config: TypedDataConfig,
    /// Exact typed-data schema JSON bytes.
    pub schema: Vec<u8>,
    /// Exact Solidity-linked vector bundle JSON bytes.
    pub vectors: Vec<u8>,
}

/// A closed manifest and every loaded contract input.
#[derive(Clone, Debug)]
pub struct InputBundle {
    /// Parsed manifest.
    pub manifest: Manifest,
    /// SHA-256 of the exact manifest bytes accepted by the parser.
    pub manifest_sha256: Sha256Digest,
    /// Manifest-owned output tied to the same root handle used for input reads.
    pub output: crate::OutputTarget,
    /// Effective downward-bounded limits.
    pub limits: HardLimits,
    /// Contracts sorted by stable identifier.
    pub contracts: Vec<ContractInput>,
    /// Typed-data inputs sorted by stable identifier.
    pub typed_data: Vec<TypedDataInput>,
    /// Notices and degradation warnings, including one `MSB-I001` when contracts are present.
    pub diagnostics: Diagnostics,
}

/// Redacted input-loading failure.
#[derive(Clone, Debug, thiserror::Error)]
#[error("artifact input validation failed")]
pub struct InputError {
    /// Ordered diagnostics with source-relative locations only.
    pub diagnostics: Diagnostics,
}

/// Loads and validates a closed config-v4 manifest plus all referenced files.
///
/// # Errors
///
/// Returns [`InputError`] for unsafe paths, malformed syntax, duplicate identifiers, invalid
/// bytecode references, or resource-limit violations.
pub fn load_inputs(manifest_path: &Path) -> Result<InputBundle, InputError> {
    let (root, manifest_raw) = open_manifest(manifest_path)?;
    let manifest = crate::manifest::parse(&manifest_raw).map_err(|error| {
        failure(Diagnostic::error(
            DiagnosticCode::InvalidInput,
            match error {
                crate::manifest::ParseError::Invalid => {
                    "manifest is not valid closed config-v4 TOML"
                }
                crate::manifest::ParseError::UnsupportedSchema => "manifest schema is unsupported",
            },
        ))
    })?;
    let limits = HardLimits::with_overrides(&manifest.limits).map_err(|field| {
        failure(Diagnostic::error(
            DiagnosticCode::LimitExceeded,
            format!("limit override `{field}` must lower the config-v4 maximum"),
        ))
    })?;
    validate_manifest(&manifest, limits)?;
    let mut total = 0_usize;
    let mut diagnostics = Diagnostics::new();
    if !manifest.contracts.is_empty() {
        diagnostics.push(Diagnostic::info(
            DiagnosticCode::ExternalArtifactCoherenceUnverified,
            "contract compiler-artifact invocation coherence is unverified",
        ));
    }
    let mut contracts = Vec::with_capacity(manifest.contracts.len());
    for config in &manifest.contracts {
        contracts.push(load_contract(
            &root,
            config,
            limits,
            &mut total,
            &mut diagnostics,
        )?);
    }
    contracts.sort_by(|left, right| left.config.id.cmp(&right.config.id));
    let mut typed_data = Vec::with_capacity(manifest.typed_data.len());
    for config in &manifest.typed_data {
        typed_data.push(TypedDataInput {
            config: config.clone(),
            schema: read_relative(
                &root,
                &config.schema,
                limits.typed_data_schema_bytes,
                &mut total,
                limits,
            )?,
            vectors: read_relative(
                &root,
                &config.vectors,
                limits.typed_data_vector_bytes,
                &mut total,
                limits,
            )?,
        });
    }
    typed_data.sort_by(|left, right| left.config.id.cmp(&right.config.id));
    Ok(InputBundle {
        manifest_sha256: sha256(&manifest_raw),
        output: crate::OutputTarget::new(root, manifest.generation.output.clone()),
        manifest,
        limits,
        contracts,
        typed_data,
        diagnostics,
    })
}

fn load_contract(
    root: &InputRoot,
    config: &ContractConfig,
    limits: HardLimits,
    total: &mut usize,
    diagnostics: &mut Diagnostics,
) -> Result<ContractInput, InputError> {
    let (raw_abi, abi) = load_abi(root, config, limits, total)?;
    let creation = load_optional(
        root,
        config.creation_bytecode.as_deref(),
        limits.bytecode_bytes,
        total,
        limits,
    )?
    .map(|raw| parse_creation(&raw, limits))
    .transpose()
    .map_err(|diagnostic| InputError {
        diagnostics: one(diagnostic.at(config
            .creation_bytecode
            .as_deref()
            .unwrap_or("creation_bytecode"))),
    })?;
    let runtime = load_optional(
        root,
        config.runtime_bytecode.as_deref(),
        limits.bytecode_bytes,
        total,
        limits,
    )?
    .map(|raw| parse_runtime(&raw, limits))
    .transpose()
    .map_err(|diagnostic| InputError {
        diagnostics: one(diagnostic),
    })?;
    let ast_raw = load_optional(root, config.ast.as_deref(), limits.ast_bytes, total, limits)?;
    let ast = ast_raw
        .as_ref()
        .map(|raw| {
            let value = crate::closed_json::parse(raw)?;
            if value.as_object().is_none()
                || value.get("nodeType").and_then(serde_json::Value::as_str) != Some("SourceUnit")
            {
                return Err(());
            }
            Ok(value)
        })
        .transpose()
        .map_err(|()| {
            failure_at(
                DiagnosticCode::InvalidInput,
                "AST input must be one solc SourceUnit JSON object",
                config.ast.as_deref().unwrap_or("ast"),
            )
        })?;
    if ast.is_none() {
        diagnostics.push(Diagnostic::warning(
            DiagnosticCode::AstUnavailable,
            format!("contract `{}` has no AST enrichment input", config.id),
        ));
    }
    let metadata = load_optional(
        root,
        config.metadata.as_deref(),
        limits.auxiliary_bytes,
        total,
        limits,
    )?;
    let storage_layout = load_optional(
        root,
        config.storage_layout.as_deref(),
        limits.auxiliary_bytes,
        total,
        limits,
    )?;
    validate_json_object(metadata.as_deref(), config.metadata.as_deref())?;
    validate_json_object(storage_layout.as_deref(), config.storage_layout.as_deref())?;
    let auxiliary_digests = AuxiliaryDigests {
        ast: ast_raw.as_deref().map(sha256),
        metadata: metadata.as_deref().map(sha256),
        storage_layout: storage_layout.as_deref().map(sha256),
    };
    Ok(ContractInput {
        config: config.clone(),
        raw_abi,
        abi,
        creation,
        runtime,
        ast,
        metadata,
        storage_layout,
        auxiliary_digests,
    })
}

fn load_abi(
    root: &InputRoot,
    config: &ContractConfig,
    limits: HardLimits,
    total: &mut usize,
) -> Result<(Vec<u8>, Vec<serde_json::Value>), InputError> {
    let raw = read_relative(root, &config.abi, limits.abi_bytes, total, limits)?;
    let values = crate::abi_json::parse(&raw, limits)
        .map_err(|diagnostic| failure(diagnostic.at(sanitize(&config.abi))))?;
    Ok((raw, values))
}

fn validate_json_object(raw: Option<&[u8]>, path: Option<&str>) -> Result<(), InputError> {
    if let Some(raw) = raw {
        let value = crate::closed_json::parse(raw).map_err(|()| {
            failure_at(
                DiagnosticCode::InvalidInput,
                "auxiliary input is not valid JSON",
                path.unwrap_or("auxiliary"),
            )
        })?;
        if !value.is_object() {
            return Err(failure_at(
                DiagnosticCode::InvalidInput,
                "auxiliary input must be a JSON object",
                path.unwrap_or("auxiliary"),
            ));
        }
    }
    Ok(())
}

pub(crate) fn one(diagnostic: Diagnostic) -> Diagnostics {
    let mut diagnostics = Diagnostics::new();
    diagnostics.push(diagnostic);
    diagnostics
}
pub(crate) fn failure(diagnostic: Diagnostic) -> InputError {
    InputError {
        diagnostics: one(diagnostic),
    }
}
pub(crate) fn failure_at(code: DiagnosticCode, message: &str, location: &str) -> InputError {
    failure(Diagnostic::error(code, message).at(sanitize(location)))
}
pub(crate) fn sanitize(location: &str) -> String {
    Path::new(location)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("input")
        .to_owned()
}
