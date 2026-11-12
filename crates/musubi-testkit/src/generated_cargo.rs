//! Closed Cargo manifest and build-input validation for generated crates.

use musubi_core::RuntimeSource;
use serde::Deserialize;

use crate::{GeneratedCrateError, generated_tree::TreeSnapshot};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    package: Package,
    #[serde(rename = "workspace")]
    _workspace: Workspace,
    dependencies: Dependencies,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Package {
    name: String,
    version: String,
    edition: String,
    rust_version: String,
    publish: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Workspace {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Dependencies {
    musubi_core: Dependency,
    musubi_runtime: Dependency,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependency {
    path: Option<String>,
    git: Option<String>,
    rev: Option<String>,
    package: Option<String>,
}

pub(crate) fn validate(
    snapshot: &TreeSnapshot,
    package_name: &str,
    package_version: &str,
    runtime: &RuntimeSource,
) -> Result<(), GeneratedCrateError> {
    let source = std::str::from_utf8(snapshot.file("Cargo.toml")?)
        .map_err(|_| GeneratedCrateError::InvalidLayout)?;
    let manifest: Manifest = toml::from_str(source).map_err(invalid)?;
    if manifest.package.name != package_name
        || manifest.package.version != package_version
        || manifest.package.edition != "2024"
        || manifest.package.rust_version != "1.95"
        || manifest.package.publish
    {
        return Err(GeneratedCrateError::InvalidLayout);
    }
    validate_dependency(&manifest.dependencies.musubi_core, "musubi-core", runtime)?;
    validate_dependency(
        &manifest.dependencies.musubi_runtime,
        "musubi-runtime",
        runtime,
    )
}

fn validate_dependency(
    dependency: &Dependency,
    package: &str,
    runtime: &RuntimeSource,
) -> Result<(), GeneratedCrateError> {
    let valid = match runtime {
        RuntimeSource::Path { root } => {
            let expected_path = format!("{root}/crates/{package}");
            dependency.path.as_deref() == Some(expected_path.as_str())
                && dependency.git.is_none()
                && dependency.rev.is_none()
                && dependency.package.is_none()
        }
        RuntimeSource::Git { url, revision } => {
            dependency.path.is_none()
                && dependency.git.as_ref() == Some(url)
                && dependency.rev.as_ref() == Some(revision)
                && dependency.package.as_deref() == Some(package)
        }
    };
    if valid {
        Ok(())
    } else {
        Err(GeneratedCrateError::InvalidLayout)
    }
}

fn invalid<T>(_: T) -> GeneratedCrateError {
    GeneratedCrateError::InvalidLayout
}
