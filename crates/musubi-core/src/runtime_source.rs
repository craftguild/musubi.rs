//! Explicit Cargo dependency sources, independent of generator build information and wire ABI.

use serde::{Deserialize, Serialize};

/// One explicit source shared by generated core and runtime dependencies.
///
/// Git records an exact URL and commit; ordinary Cargo/Git configuration controls acquisition.
/// Path records a mutable, placement-dependent source and proves neither content identity nor
/// authenticity. A path is relative to the manifest that owns it (configuration manifest on input,
/// generated Cargo manifest in SDK provenance). No source or version fallback is implied.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeSource {
    /// An explicit SSH or HTTPS Git dependency pinned to a complete commit.
    Git {
        /// Exact credential-free URL passed to Cargo for both packages.
        url: String,
        /// Nonzero lowercase 40-digit Git commit, never a branch, tag or display revision.
        revision: String,
    },
    /// A mutable local musubi.rs source root authorized by the manifest owner.
    Path {
        /// Relative musubi.rs source root; packages are fixed under `crates/musubi-{core,runtime}`.
        root: String,
    },
}

/// Redacted failure for a malformed or incomplete explicit runtime source.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("runtime source is invalid")]
pub struct RuntimeSourceError;

impl RuntimeSource {
    /// Validates the closed Git or relative Path source without acquiring dependencies.
    ///
    /// # Errors
    ///
    /// Rejects credentials, unsupported URL schemes, non-complete revisions, and unsafe path forms.
    /// `.` and `..` are allowed only as ordinary relative dependency components; this does not relax
    /// artifact or output confinement.
    pub fn validate(&self) -> Result<(), RuntimeSourceError> {
        let valid = match self {
            Self::Git { url, revision } => {
                valid_url(url) && crate::generator::full_commit(revision)
            }
            Self::Path { root } => valid_root(root),
        };
        if valid {
            Ok(())
        } else {
            Err(RuntimeSourceError)
        }
    }
}

fn valid_root(root: &str) -> bool {
    !(root.is_empty()
        || root.len() > 4096
        || root.starts_with('/')
        || root.contains('\\')
        || root.chars().any(char::is_control)
        || (root.as_bytes().get(1) == Some(&b':') && root.as_bytes()[0].is_ascii_alphabetic()))
}

fn valid_url(value: &str) -> bool {
    if value.len() > 4096
        || value.contains('\\')
        || value.chars().any(|c| c.is_control() || c.is_whitespace())
        || encoded_control(value)
    {
        return false;
    }
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    matches!(url.scheme(), "ssh" | "https")
        && (value.starts_with("ssh://") || value.starts_with("https://"))
        && url.host_str().is_some_and(|host| !host.is_empty())
        && !url.cannot_be_a_base()
        && url.path().len() > 1
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && (url.scheme() == "ssh" || url.username().is_empty())
}

fn encoded_control(value: &str) -> bool {
    value.as_bytes().windows(3).any(|bytes| {
        bytes[0] == b'%'
            && std::str::from_utf8(&bytes[1..])
                .ok()
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
                .is_some_and(|byte| byte.is_ascii_control())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const COMMIT: &str = "046477cb8a19e8db4d93d7adb8b82e2ecec77577";

    #[test]
    fn explicit_sources_allow_ordinary_ssh_https_and_relative_paths() {
        for url in [
            "ssh://git@github.com/craftguild/musubi.rs.git",
            "https://github.com/craftguild/musubi.rs.git",
            "ssh://builder@private.test:2222/source",
            "https://example.test:8443/source",
        ] {
            RuntimeSource::Git {
                url: url.into(),
                revision: COMMIT.into(),
            }
            .validate()
            .unwrap();
        }
        for root in [".", "..", "../lab02", "vendor/lab02", "../my source/lab02"] {
            RuntimeSource::Path { root: root.into() }
                .validate()
                .unwrap();
        }
    }

    #[test]
    fn explicit_sources_reject_credentials_ambiguous_paths_and_display_revisions() {
        for url in [
            "file:///repo",
            "git@host:repo",
            "http://host/repo",
            "https://token@host/repo",
            "ssh://git:secret@host/repo",
            "ssh://git@host/repo?token=secret",
            "https://host/repo#main",
            "ssh://git@host/repo\n",
            "ssh://git@host/repo%0a",
            "https://host/",
            "../repo",
        ] {
            assert!(
                RuntimeSource::Git {
                    url: url.into(),
                    revision: COMMIT.into()
                }
                .validate()
                .is_err()
            );
        }
        for revision in [
            "main",
            "v1.0.0",
            "046477c",
            "unknown",
            "046477cb8a19e8db4d93d7adb8b82e2ecec77577-dirty",
            "0000000000000000000000000000000000000000",
        ] {
            assert!(
                RuntimeSource::Git {
                    url: "https://host/repo".into(),
                    revision: revision.into()
                }
                .validate()
                .is_err()
            );
        }
        for root in [
            "",
            "/repo",
            "C:/repo",
            "C:\\repo",
            "repo\\other",
            "../repo\n",
        ] {
            assert!(
                RuntimeSource::Path { root: root.into() }
                    .validate()
                    .is_err()
            );
        }
    }

    #[test]
    fn runtime_union_rejects_mixed_missing_unknown_and_old_fields() {
        for value in [
            serde_json::json!({"source":"git","url":"https://host/repo","revision":COMMIT,"root":"."}),
            serde_json::json!({"source":"path","root":".","revision":COMMIT}),
            serde_json::json!({"source":"path"}),
            serde_json::json!({"source":"git","url":"https://host/repo"}),
            serde_json::json!({"source":"path","root":".","unknown":true}),
            serde_json::json!({"distribution":"bundled"}),
        ] {
            assert!(serde_json::from_value::<RuntimeSource>(value).is_err());
        }
    }
}
