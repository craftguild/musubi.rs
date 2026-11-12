//! Closed config-v4 product manifest and typed generation output path.

use std::path::Path;

use musubi_core::ArtifactKind;
use serde::Deserialize;

use crate::LimitOverrides;

/// Sole supported product manifest schema identifier.
pub const CONFIG_V4: &str = "musubi/config-v4";

/// Closed generator manifest.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Exact supported closed schema identifier.
    pub schema_version: String,
    /// Generated Cargo package identity.
    pub package: PackageConfig,
    /// Manifest-owned generated output configuration.
    pub generation: GenerationConfig,
    /// Explicit shared core/runtime source, relative to this manifest for paths.
    pub runtime: musubi_core::RuntimeSource,
    /// Logical contracts included in the generated crate.
    #[serde(default)]
    pub contracts: Vec<ContractConfig>,
    /// Explicit typed-data authorities included in the generated crate.
    #[serde(default)]
    pub typed_data: Vec<TypedDataConfig>,
    /// Optional downward-only hard-limit overrides.
    #[serde(default)]
    pub limits: LimitOverrides,
}

/// Generated Cargo package metadata.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageConfig {
    /// Cargo package name.
    pub name: String,
    /// Cargo package version.
    pub version: String,
    /// Rust edition; config-v4 accepts only `2024`.
    pub edition: String,
}

/// Manifest-owned generation settings.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationConfig {
    /// Manifest-root-relative generated crate location.
    pub output: GenerationOutput,
}

/// A non-empty manifest-root-relative output made only of normal path components.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct GenerationOutput(String);

impl GenerationOutput {
    /// Returns the validated relative output as a platform path.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }

    /// Returns the exact validated slash-separated representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for GenerationOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if valid_output(&value) {
            Ok(Self(value))
        } else {
            Err(serde::de::Error::custom(
                "generation.output must contain only normal relative path components",
            ))
        }
    }
}

fn valid_output(value: &str) -> bool {
    let bytes = value.as_bytes();
    !value.is_empty()
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.contains('\\')
        && !value.chars().any(char::is_control)
        && !(bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
        && value
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

/// External inputs for one logical contract.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractConfig {
    /// Stable unique contract identifier.
    pub id: String,
    /// Explicit artifact capability class.
    pub kind: ArtifactKind,
    /// Manifest-relative ABI JSON path.
    pub abi: String,
    /// Optional manifest-relative creation bytecode JSON path; required for deployable artifacts.
    pub creation_bytecode: Option<String>,
    /// Optional manifest-relative runtime bytecode JSON path.
    pub runtime_bytecode: Option<String>,
    /// Optional manifest-relative solc `SourceUnit` AST path.
    pub ast: Option<String>,
    /// Optional compiler metadata JSON path.
    pub metadata: Option<String>,
    /// Optional storage layout JSON path.
    pub storage_layout: Option<String>,
    /// Optional generated Rust module name.
    pub rust_module: Option<String>,
}

/// External inputs for one closed typed-data authority.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedDataConfig {
    /// Stable unique typed-data identifier.
    pub id: String,
    /// Unique generated Rust module name.
    pub rust_module: String,
    /// Closed authority mode; no ABI or AST inference is accepted.
    pub authority: TypedDataAuthority,
    /// Manifest-relative typed-data schema JSON path.
    pub schema: String,
    /// Manifest-relative Solidity-linked vector bundle JSON path.
    pub vectors: String,
}

/// Closed typed-data authority modes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
pub enum TypedDataAuthority {
    /// Versioned reviewed schema plus mandatory independently produced vectors.
    #[serde(rename = "closed-schema-v1")]
    ClosedSchemaV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ParseError {
    Invalid,
    UnsupportedSchema,
}

#[derive(Deserialize)]
struct SchemaProbe {
    schema_version: String,
}

pub(crate) fn parse(raw: &[u8]) -> Result<Manifest, ParseError> {
    let schema: SchemaProbe = toml::from_slice(raw).map_err(|_| ParseError::Invalid)?;
    if schema.schema_version != CONFIG_V4 {
        return Err(ParseError::UnsupportedSchema);
    }
    toml::from_slice(raw).map_err(|_| ParseError::Invalid)
}
