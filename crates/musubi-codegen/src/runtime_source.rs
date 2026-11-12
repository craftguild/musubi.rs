//! Manifest runtime rebasing and paired Cargo/Rust source rendering; no binary defaults.

use musubi_core::RuntimeSource;

pub(crate) fn rebase(source: &RuntimeSource, output: &crate::GenerationOutput) -> RuntimeSource {
    match source {
        RuntimeSource::Git { .. } => source.clone(),
        RuntimeSource::Path { root } => RuntimeSource::Path {
            root: format!(
                "{}{root}",
                "../".repeat(output.as_path().components().count())
            ),
        },
    }
}

pub(crate) fn cargo_dependency(source: &RuntimeSource, package: &str) -> String {
    match source {
        RuntimeSource::Path { root } => {
            let path = format!("{root}/crates/{package}");
            // TOML and Rust have different escaping rules. Delegate TOML strings to its serializer.
            format!("{package} = {{ path = {} }}", toml::Value::String(path))
        }
        RuntimeSource::Git { url, revision } => format!(
            "{package} = {{ git = {}, rev = {}, package = {} }}",
            toml::Value::String(url.clone()),
            toml::Value::String(revision.clone()),
            toml::Value::String(package.into())
        ),
    }
}

pub(crate) fn core_expression(source: &RuntimeSource) -> String {
    match source {
        RuntimeSource::Path { root } => {
            format!("musubi_core::RuntimeSource::Path {{ root: {root:?}.into() }}")
        }
        RuntimeSource::Git { url, revision } => format!(
            "musubi_core::RuntimeSource::Git {{ url: {url:?}.into(), revision: {revision:?}.into() }}"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_runtime_rebases_both_packages_without_generator_version() {
        for root in [".", "../lab02", "../../my lab02"] {
            let output: crate::GenerationConfig = toml::from_str("output = 'nested/sdk'").unwrap();
            let source = rebase(&RuntimeSource::Path { root: root.into() }, &output.output);
            assert_eq!(
                source,
                RuntimeSource::Path {
                    root: format!("../../{root}")
                }
            );
            for package in ["musubi-core", "musubi-runtime"] {
                let manifest: toml::Value =
                    toml::from_str(&cargo_dependency(&source, package)).unwrap();
                assert_eq!(
                    manifest[package]["path"].as_str(),
                    Some(format!("../../{root}/crates/{package}").as_str())
                );
                assert!(manifest[package].get("version").is_none());
            }
        }
    }

    #[test]
    fn explicit_runtime_passes_exact_ssh_or_https_to_cargo() {
        for url in ["ssh://git@host/repo.git", "https://host/repo.git"] {
            let source = RuntimeSource::Git {
                url: url.into(),
                revision: "1234567890abcdef1234567890abcdef12345678".into(),
            };
            for package in ["musubi-core", "musubi-runtime"] {
                let manifest: toml::Value =
                    toml::from_str(&cargo_dependency(&source, package)).unwrap();
                assert_eq!(manifest[package]["git"].as_str(), Some(url));
                assert_eq!(
                    manifest[package]["rev"].as_str(),
                    Some("1234567890abcdef1234567890abcdef12345678")
                );
                assert!(manifest[package].get("version").is_none());
                assert!(manifest[package].get("path").is_none());
            }
        }
    }
}
