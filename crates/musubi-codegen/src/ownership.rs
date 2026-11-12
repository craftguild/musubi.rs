//! Closed v2 ownership records bind final file bytes; they do not authenticate a generator.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use std::io::Write as _;

use serde::{Deserialize, Serialize};

use crate::{
    HardLimits,
    ownership_path::{self, Paths},
};

/// The only generated SDK ownership-marker basename.
pub const GENERATED_MARKER: &str = ".musubi-generated";
/// Maximum serialized ownership-marker bytes, also charged to the aggregate SDK budget.
pub const MAX_MARKER_BYTES: usize = 16 * 1024 * 1024;
/// Sole supported ownership schema; literal earlier markers are rejected.
pub const OWNERSHIP_SCHEMA: &str = "musubi/generated-v2";
/// Private semantic layout identifier independent of wire and generated-runtime ABI versions.
pub const SEMANTIC_LAYOUT: &str = "musubi/semantic-layout-v1";

/// Exact final bytes owned at one root-relative path.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedFile {
    /// Normal slash-separated UTF-8 path, excluding the marker itself.
    pub path: String,
    /// Exact nonnegative byte length.
    pub bytes: u64,
    /// SHA-256 as 64 lowercase hexadecimal digits without a prefix.
    pub sha256: String,
}

/// Validated closed marker records. Mutation must be revalidated before use.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnershipManifest {
    /// Must equal [`OWNERSHIP_SCHEMA`].
    pub schema_version: String,
    /// Must equal [`SEMANTIC_LAYOUT`].
    pub layout: String,
    /// Unique records in lexicographic path order; ancestors cannot collide with files.
    #[serde(deserialize_with = "bounded_records")]
    pub files: Vec<OwnedFile>,
}

/// Bounded ownership validation failure, without untrusted path or file contents.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum OwnershipError {
    /// Closed schema, record ordering, field representation, or marker presence is invalid.
    #[error("generated ownership marker is invalid")]
    InvalidMarker,
    /// A path is non-normal, colliding, or not UTF-8.
    #[error("generated ownership path is invalid")]
    InvalidPath,
    /// The marker, file count, directory depth/count, or aggregate byte budget is exceeded.
    #[error("generated ownership exceeds effective limits")]
    LimitExceeded,
    /// File set, size, or hash differs from its ownership record.
    #[error("generated ownership inventory differs from file bytes")]
    InventoryMismatch,
}

/// Parses and validates the sole closed ownership schema within effective and hard limits.
///
/// This validates records only. Use [`validate_owned_files`] to bind them to actual bytes.
///
/// # Errors
/// Rejects old/unknown schemas, unknown/duplicate keys, invalid paths/hashes/sizes/order, and limits.
pub fn parse_ownership_marker(
    bytes: &[u8],
    limits: HardLimits,
) -> Result<OwnershipManifest, OwnershipError> {
    let limits = bounded_limits(limits);
    if bytes.len() > MAX_MARKER_BYTES || bytes.len() > limits.generated_bytes {
        return Err(OwnershipError::LimitExceeded);
    }
    // Typed deserialization rejects duplicate fields at both closed object levels, without a
    // second JSON Value tree or any permissive map reader.
    let marker: OwnershipManifest =
        serde_json::from_slice(bytes).map_err(|_| OwnershipError::InvalidMarker)?;
    if marker.schema_version != OWNERSHIP_SCHEMA || marker.layout != SEMANTIC_LAYOUT {
        return Err(OwnershipError::InvalidMarker);
    }
    if marker.files.len() >= limits.generated_files {
        return Err(OwnershipError::LimitExceeded);
    }
    let mut total = bytes.len() as u64;
    let mut previous: Option<&str> = None;
    let mut paths = Paths::default();
    paths.insert(GENERATED_MARKER, false, limits.generated_files)?;
    for file in &marker.files {
        ownership_path::validate(&file.path, limits)?;
        if file.path == GENERATED_MARKER
            || previous.is_some_and(|old| old >= file.path.as_str())
            || file.sha256.len() != 64
            || !file
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(OwnershipError::InvalidMarker);
        }
        paths.insert(&file.path, false, limits.generated_files)?;
        total = total
            .checked_add(file.bytes)
            .ok_or(OwnershipError::LimitExceeded)?;
        if total > limits.generated_bytes as u64 {
            return Err(OwnershipError::LimitExceeded);
        }
        previous = Some(&file.path);
    }
    if paths.directories().count() > limits.generated_files {
        return Err(OwnershipError::LimitExceeded);
    }
    Ok(marker)
}

/// Verifies every retained file against the bounded marker and exact owned file set.
///
/// Filesystem callers must first retain regular-file bytes with no-follow, bounded I/O and verify
/// that directories are exactly their ancestors. This pure validator performs no I/O or compilation.
/// Ownership is self-consistency, not authenticity or an exact regeneration from current inputs.
///
/// # Errors
/// Rejects malformed markers, missing/extra files, size/hash drift, unsafe paths, and resource limits.
pub fn validate_owned_files(
    files: &BTreeMap<PathBuf, Vec<u8>>,
    limits: HardLimits,
) -> Result<OwnershipManifest, OwnershipError> {
    let limits = bounded_limits(limits);
    if files.len() > limits.generated_files {
        return Err(OwnershipError::LimitExceeded);
    }
    for path in files.keys() {
        ownership_path::validate(path.to_str().ok_or(OwnershipError::InvalidPath)?, limits)?;
    }
    let bytes = files
        .get(Path::new(GENERATED_MARKER))
        .ok_or(OwnershipError::InvalidMarker)?;
    let marker = parse_ownership_marker(bytes, limits)?;
    if files.len() != marker.files.len() + 1 {
        return Err(OwnershipError::InventoryMismatch);
    }
    for file in &marker.files {
        let contents = files
            .get(Path::new(&file.path))
            .ok_or(OwnershipError::InventoryMismatch)?;
        if contents.len() as u64 != file.bytes
            || hex::encode(musubi_core::sha256(contents)) != file.sha256
        {
            return Err(OwnershipError::InventoryMismatch);
        }
    }
    Ok(marker)
}

/// Serializes final normalized output records. Allocation is bounded before and while writing.
pub(crate) fn render_marker(
    files: &BTreeMap<PathBuf, Vec<u8>>,
    maximum: usize,
) -> Result<Vec<u8>, crate::RenderError> {
    let mut records = files
        .iter()
        .filter(|(path, _)| path.as_path() != Path::new(GENERATED_MARKER))
        .map(|(path, contents)| {
            Ok(OwnedFile {
                path: path
                    .to_str()
                    .ok_or_else(|| crate::render::render_error("generated path is not UTF-8"))?
                    .into(),
                bytes: contents.len() as u64,
                sha256: hex::encode(musubi_core::sha256(contents)),
            })
        })
        .collect::<Result<Vec<_>, crate::RenderError>>()?;
    // PathBuf compares components; the wire inventory orders complete UTF-8 strings. For example,
    // assets/foo$/abi.json must precede assets/foo/abi.json even though `foo` is a shorter component.
    records.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    let mut output = crate::render_source::BoundedBytes::new(maximum.min(MAX_MARKER_BYTES));
    serde_json::to_writer(
        &mut output,
        &OwnershipManifest {
            schema_version: OWNERSHIP_SCHEMA.into(),
            layout: SEMANTIC_LAYOUT.into(),
            files: records,
        },
    )
    .map_err(|_| crate::render::render_error("ownership marker exceeds its byte budget"))?;
    output
        .write_all(b"\n")
        .map_err(|_| crate::render::render_error("ownership marker exceeds its byte budget"))?;
    Ok(output.finish())
}

pub(crate) fn bounded_limits(mut limits: HardLimits) -> HardLimits {
    limits.generated_files = limits.generated_files.min(HardLimits::V1.generated_files);
    limits.generated_bytes = limits.generated_bytes.min(HardLimits::V1.generated_bytes);
    limits.nesting = limits.nesting.min(HardLimits::V1.nesting);
    limits
}

fn bounded_records<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<OwnedFile>, D::Error> {
    struct Records;
    impl<'de> serde::de::Visitor<'de> for Records {
        type Value = Vec<OwnedFile>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a bounded generated file inventory")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            let mut records = Vec::new();
            while let Some(record) = sequence.next_element()? {
                if records.len() >= HardLimits::V1.generated_files - 1 {
                    return Err(serde::de::Error::custom("ownership file limit"));
                }
                records.push(record);
            }
            Ok(records)
        }
    }
    deserializer.deserialize_seq(Records)
}

pub(crate) fn empty_marker_bytes() -> usize {
    // Exact compact schema/layout framing including the final newline.
    b"{\"schema_version\":\"musubi/generated-v2\",\"layout\":\"musubi/semantic-layout-v1\",\"files\":[]}\n".len()
}

pub(crate) fn record_bytes(path: &str, bytes: usize) -> Result<usize, crate::RenderError> {
    serde_json::to_vec(&OwnedFile {
        path: path.into(),
        bytes: bytes as u64,
        sha256: "0".repeat(64),
    })
    .map(|bytes| bytes.len())
    .map_err(|_| crate::render::render_error("ownership record cannot be serialized"))
}
