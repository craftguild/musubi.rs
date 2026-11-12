//! Closed, serializable generator build information independent of runtime dependencies.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

/// Sole generator information schema understood by this API.
pub const GENERATOR_INFO_SCHEMA: &str = "musubi/generator-info-v1";

/// Build-time observations preserved literally by a generated SDK.
///
/// This is diagnostic provenance, not a signature or proof of authenticity. A `hash-dirty`
/// revision does not uniquely identify uncommitted contents: different dirty builds may have the
/// same value. `unknown` provides no source identity. Runtime source and ABI compatibility are
/// separate contracts. Borrowed strings support static generated literals; deserialization owns
/// its strings and never borrows from an input buffer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorInfo {
    /// Exact version of this closed metadata structure.
    pub schema_version: Cow<'static, str>,
    /// Generator package version, independent of runtime package versions.
    pub version: Cow<'static, str>,
    /// Complete commit, complete commit followed by `-dirty`, or `unknown`.
    pub revision: Cow<'static, str>,
    /// Complete lowercase Git commit; null if either HEAD or status was unavailable.
    #[serde(deserialize_with = "required_option")]
    pub git_commit: Option<Cow<'static, str>>,
    /// Repository state including staged, unstaged, untracked, and submodule changes.
    #[serde(deserialize_with = "required_option")]
    pub dirty: Option<bool>,
    /// Compiler and target observations captured when the generator was built.
    pub build: GeneratorBuild,
}

/// Non-secret compiler and platform information from the generator build.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorBuild {
    /// Rust compiler version banner, or `unknown` if unavailable.
    pub rustc: Cow<'static, str>,
    /// Build host target triple.
    pub host: Cow<'static, str>,
    /// Target triple of the generator binary.
    pub target: Cow<'static, str>,
    /// Cargo profile such as `debug` or `release`.
    pub profile: Cow<'static, str>,
}

/// Metadata is incomplete, inconsistent, or outside the closed schema's bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("generator build information is invalid")]
pub struct GeneratorInfoError;

impl GeneratorInfo {
    /// Validates bounds, textual shapes, and the revision/commit/dirty invariant.
    ///
    /// # Errors
    ///
    /// Returns a redacted error for invalid metadata; does not authenticate recorded values.
    pub fn validate(&self) -> Result<(), GeneratorInfoError> {
        let revision_valid = match (&self.git_commit, self.dirty) {
            (None, None) => self.revision == "unknown",
            (Some(commit), Some(dirty)) if full_commit(commit) => {
                if dirty {
                    self.revision == format!("{commit}-dirty")
                } else {
                    self.revision == *commit
                }
            }
            _ => false,
        };
        if self.schema_version != GENERATOR_INFO_SCHEMA
            || self.version.len() > 256
            || semver::Version::parse(&self.version).is_err()
            || !revision_valid
            || !valid_rustc(&self.build.rustc)
            || !token(&self.build.host)
            || !token(&self.build.target)
            || !token(&self.build.profile)
        {
            return Err(GeneratorInfoError);
        }
        Ok(())
    }
}

fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

pub(crate) fn full_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && value.bytes().any(|byte| byte != b'0')
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
}

fn valid_rustc(value: &str) -> bool {
    if value == "unknown" {
        return true;
    }
    if value.len() > 256 {
        return false;
    }
    let Some(banner) = value.strip_prefix("rustc ") else {
        return false;
    };
    let Some((version, details)) = banner.split_once(" (") else {
        return false;
    };
    let Some(details) = details.strip_suffix(')') else {
        return false;
    };
    let Some((hash, date)) = details.split_once(' ') else {
        return false;
    };
    semver::Version::parse(version).is_ok()
        && (7..=40).contains(&hash.len())
        && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        && date.len() == 10
        && date.bytes().enumerate().all(|(index, byte)| {
            if index == 4 || index == 7 {
                byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
}

#[cfg(test)]
mod tests;
