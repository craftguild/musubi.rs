//! Validated bytecode templates, references, and artifact provenance.

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use sha3::Keccak256;

use crate::{Bytes, FixedBytes, ValueError};

/// A SHA-256 digest used for canonical and raw artifact identity.
pub type Sha256Digest = FixedBytes<32>;

/// A Keccak-256 digest used for EVM bytecode identity.
pub type Keccak256Digest = FixedBytes<32>;

/// Closed compiler-artifact capability class selected by a reviewed generator manifest.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// ABI plus creation bytecode, with optional deployed runtime identity.
    Deployable,
    /// ABI-only contract boundary that deliberately has no deployment surface.
    Interface,
}

/// Reports whether a compiler artifact target is a relative `source-path:Contract` identity.
///
/// Absolute paths, traversal, platform-specific separators, control characters, and malformed
/// Solidity contract names are rejected so release evidence cannot disclose build-machine paths.
#[must_use]
pub fn is_canonical_artifact_target(value: &str) -> bool {
    let Some((source, contract)) = value.rsplit_once(':') else {
        return false;
    };
    is_canonical_source_path(source)
        && !source.contains(':')
        && is_canonical_solidity_identifier(contract)
}

/// Reports whether a compiler source path is relative, normalized, and platform independent.
#[must_use]
pub fn is_canonical_source_path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.contains(['\\', ':'])
        && value.split('/').all(|component| {
            !component.is_empty()
                && !matches!(component, "." | "..")
                && !component.chars().any(char::is_control)
        })
}

/// Reports whether a Solidity compiler version has the closed release-evidence form.
///
/// Accepted identities are `major.minor.patch` with an optional lowercase eight-hex
/// `+commit.` suffix and an optional literal `solc ` prefix.
#[must_use]
pub fn is_canonical_compiler_version(value: &str) -> bool {
    if value.len() > 64 {
        return false;
    }
    let value = value.strip_prefix("solc ").unwrap_or(value);
    let (version, commit) = value
        .split_once("+commit.")
        .map_or((value, None), |(version, commit)| (version, Some(commit)));
    let mut parts = version.split('.');
    let version_valid =
        (0..3).all(|_| parts.next().is_some_and(canonical_decimal)) && parts.next().is_none();
    version_valid
        && commit.is_none_or(|commit| {
            commit.len() == 8
                && commit
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}

fn canonical_decimal(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value == "0" || !value.starts_with('0'))
}

/// Reports whether a name is a canonical Solidity identifier.
#[must_use]
pub fn is_canonical_solidity_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|first| matches!(first, '_' | '$') || first.is_ascii_alphabetic())
        && chars
            .all(|character| matches!(character, '_' | '$') || character.is_ascii_alphanumeric())
}

/// A contiguous patch location in a bytecode template.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BytecodeRange {
    /// Zero-based byte offset into the template.
    pub start: usize,
    /// Number of bytes occupied by the reference.
    pub length: usize,
}

impl BytecodeRange {
    /// Validates this range against a bytecode template length.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError::ReferenceOutOfBounds`] for an empty, overflowing, or out-of-bounds
    /// range.
    pub fn validate(&self, template_len: usize) -> Result<(), ValueError> {
        let end = self
            .start
            .checked_add(self.length)
            .ok_or(ValueError::ReferenceOutOfBounds)?;
        if self.length == 0 || end > template_len {
            return Err(ValueError::ReferenceOutOfBounds);
        }
        Ok(())
    }
}

/// One named Solidity library reference and every location where its address is linked.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkReference {
    /// Source-unit name emitted by the external compiler.
    pub source: String,
    /// Solidity library name within the source unit.
    pub library: String,
    /// Address-sized patch locations in creation or runtime bytecode.
    pub ranges: Vec<BytecodeRange>,
}

/// Runtime bytecode locations populated from constructor-time immutable values.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImmutableReference {
    /// Compiler-provided immutable identifier.
    pub id: String,
    /// Locations excluded when matching a deployed runtime template.
    pub ranges: Vec<BytecodeRange>,
}

/// Creation bytecode together with unresolved library reference locations.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreationBytecode {
    /// Compiler-emitted bytecode template.
    pub template: Bytes,
    /// Library references that require exact address coverage before deployment.
    pub links: Vec<LinkReference>,
}

impl CreationBytecode {
    /// Creates a validated creation bytecode template.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError`] if any link is not exactly an in-bounds, non-overlapping
    /// address-sized range.
    pub fn new(template: Bytes, links: Vec<LinkReference>) -> Result<Self, ValueError> {
        let bytecode = Self { template, links };
        bytecode.validate()?;
        Ok(bytecode)
    }

    /// Revalidates public or deserialized creation-bytecode fields before runtime use.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError`] for empty bytecode or malformed, out-of-bounds, or overlapping
    /// references.
    pub fn validate(&self) -> Result<(), ValueError> {
        if self.template.is_empty() {
            return Err(ValueError::EmptyCreationBytecode);
        }
        validate_references(self.template.len(), &self.links, &[])
    }

    /// Computes the Keccak-256 identity of the unlinked template.
    #[must_use]
    pub fn template_hash(&self) -> Keccak256Digest {
        keccak256(self.template.as_ref())
    }
}

/// Runtime bytecode with link and immutable masking information.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBytecode {
    /// Compiler-emitted runtime bytecode template.
    pub template: Bytes,
    /// Library address patch locations.
    pub links: Vec<LinkReference>,
    /// Constructor-populated locations ignored during template matching.
    pub immutables: Vec<ImmutableReference>,
}

impl RuntimeBytecode {
    /// Creates and range-checks a runtime bytecode template.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError`] if any link or immutable reference is outside the template or any
    /// references overlap.
    pub fn new(
        template: Bytes,
        links: Vec<LinkReference>,
        immutables: Vec<ImmutableReference>,
    ) -> Result<Self, ValueError> {
        let bytecode = Self {
            template,
            links,
            immutables,
        };
        bytecode.validate()?;
        Ok(bytecode)
    }

    /// Revalidates public or deserialized runtime-bytecode fields before runtime use.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError`] for malformed, out-of-bounds, or overlapping link and immutable
    /// references.
    pub fn validate(&self) -> Result<(), ValueError> {
        validate_references(self.template.len(), &self.links, &self.immutables)
    }

    /// Computes the Keccak-256 identity of the unmodified template.
    #[must_use]
    pub fn template_hash(&self) -> Keccak256Digest {
        keccak256(self.template.as_ref())
    }
}

/// Machine-readable statement about the relation between external inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalArtifactCoherence {
    /// The generator did not and cannot prove that ABI and bytecode share a compiler invocation.
    Unverified,
}

/// Digests and release identity embedded in every generated contract SDK.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactProvenance {
    /// SHA-256 of canonical ABI JSON.
    pub canonical_abi_sha256: Sha256Digest,
    /// SHA-256 of the exact ABI input bytes.
    pub raw_abi_sha256: Sha256Digest,
    /// Keccak-256 of the creation bytecode template.
    pub creation_template_keccak256: Keccak256Digest,
    /// Keccak-256 of runtime bytecode when supplied.
    pub runtime_template_keccak256: Option<Keccak256Digest>,
    /// SHA-256 of optional external AST input.
    pub ast_sha256: Option<Sha256Digest>,
    /// SHA-256 of optional external metadata input.
    pub metadata_sha256: Option<Sha256Digest>,
    /// SHA-256 of optional external storage layout input.
    pub storage_layout_sha256: Option<Sha256Digest>,
    /// Actual generator build observations, independent of the runtime source.
    pub generator: crate::GeneratorInfo,
    /// Explicit shared runtime dependency source; paths remain mutable.
    pub runtime: crate::RuntimeSource,
    /// Generated/runtime ABI contract version.
    pub generated_abi_version: u32,
    /// Explicitly unverified compiler-artifact coherence status.
    pub external_artifact_coherence: ExternalArtifactCoherence,
}

/// Complete artifact descriptor retained by generated SDK source.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractArtifact {
    /// Stable manifest-local contract identifier.
    pub id: String,
    /// Creation bytecode and its library references.
    pub creation: CreationBytecode,
    /// Optional runtime verification template.
    pub runtime: Option<RuntimeBytecode>,
    /// Artifact identity and generator provenance.
    pub provenance: ArtifactProvenance,
}

/// ABI-only artifact identity retained by a generated interface module.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterfaceArtifact {
    /// Stable manifest-local contract identifier.
    pub id: String,
    /// SHA-256 of canonical ABI JSON.
    pub canonical_abi_sha256: Sha256Digest,
    /// SHA-256 of the exact ABI input bytes.
    pub raw_abi_sha256: Sha256Digest,
    /// SHA-256 of optional external AST input.
    pub ast_sha256: Option<Sha256Digest>,
    /// SHA-256 of optional external metadata input.
    pub metadata_sha256: Option<Sha256Digest>,
    /// Actual generator build observations, independent of the runtime source.
    pub generator: crate::GeneratorInfo,
    /// Explicit shared runtime dependency source; paths remain mutable.
    pub runtime: crate::RuntimeSource,
    /// Generated/runtime ABI contract version.
    pub generated_abi_version: u32,
    /// Explicitly unverified compiler-artifact coherence status.
    pub external_artifact_coherence: ExternalArtifactCoherence,
}

/// Computes SHA-256 over exact bytes.
#[must_use]
pub fn sha256(bytes: &[u8]) -> Sha256Digest {
    FixedBytes::new(Sha256::digest(bytes).into())
}

/// Computes legacy EVM Keccak-256 over exact bytes.
#[must_use]
pub fn keccak256(bytes: &[u8]) -> Keccak256Digest {
    FixedBytes::new(Keccak256::digest(bytes).into())
}

fn validate_references(
    template_len: usize,
    links: &[LinkReference],
    immutables: &[ImmutableReference],
) -> Result<(), ValueError> {
    let mut ranges = Vec::new();
    for link in links {
        for range in &link.ranges {
            range.validate(template_len)?;
            if range.length != 20 {
                return Err(ValueError::ReferenceOutOfBounds);
            }
            ranges.push((range.start, range.start + range.length));
        }
    }
    for reference in immutables {
        for range in &reference.ranges {
            range.validate(template_len)?;
            ranges.push((range.start, range.start + range.length));
        }
    }
    ranges.sort_unstable();
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return Err(ValueError::OverlappingReferences);
    }
    Ok(())
}
