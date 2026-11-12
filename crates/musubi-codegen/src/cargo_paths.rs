//! Rebinds private Cargo runtime dependencies and prevents cache overlap with configured SDKs.

use crate::CargoValidationFailure;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

pub(crate) fn rebind_runtime_paths(
    manifest_path: &Path,
    conceptual_root: &Path,
) -> Result<(), CargoValidationFailure> {
    let source =
        fs::read_to_string(manifest_path).map_err(|_| CargoValidationFailure::Preparation)?;
    let mut manifest: toml::Value =
        toml::from_str(&source).map_err(|_| CargoValidationFailure::Preparation)?;
    let dependencies = manifest
        .get_mut("dependencies")
        .and_then(toml::Value::as_table_mut)
        .ok_or(CargoValidationFailure::Preparation)?;
    for name in ["musubi-core", "musubi-runtime"] {
        let Some(dependency) = dependencies
            .get_mut(name)
            .and_then(toml::Value::as_table_mut)
        else {
            return Err(CargoValidationFailure::Preparation);
        };
        let Some(relative) = dependency.get("path").and_then(toml::Value::as_str) else {
            continue;
        };
        let rebound = lexical_join(conceptual_root, Path::new(relative))
            .canonicalize()
            .map_err(|_| CargoValidationFailure::Preparation)?;
        dependency.insert(
            "path".to_owned(),
            toml::Value::String(rebound.to_string_lossy().into_owned()),
        );
    }
    let rebound = toml::to_string(&manifest).map_err(|_| CargoValidationFailure::Preparation)?;
    fs::write(manifest_path, rebound).map_err(|_| CargoValidationFailure::Preparation)
}

fn lexical_join(base: &Path, relative: &Path) -> PathBuf {
    let mut output = base.to_path_buf();
    for component in relative.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                output.pop();
            }
            Component::Normal(value) => output.push(value),
            Component::RootDir | Component::Prefix(_) => return relative.to_path_buf(),
        }
    }
    output
}

pub(crate) fn default_target(configuration_root: &Path) -> Result<PathBuf, CargoValidationFailure> {
    Ok(configuration_root.join(cargo_home()?).join("musubi/target"))
}

pub(crate) fn cargo_home() -> Result<PathBuf, CargoValidationFailure> {
    std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
        .ok_or(CargoValidationFailure::Preparation)
}

pub(crate) fn require_external_cache(
    cache: &Path,
    sdk: &Path,
    configuration_root: &Path,
) -> Result<(), CargoValidationFailure> {
    let cache = resolve_existing_ancestor(&configuration_root.join(cache))?;
    let sdk = resolve_existing_ancestor(&configuration_root.join(sdk))?;
    if cache.starts_with(&sdk) || sdk.starts_with(cache) {
        Err(CargoValidationFailure::Preparation)
    } else {
        Ok(())
    }
}

// Resolve existing symlink aliases without creating an absent SDK/cache. Configuration paths are
// trusted Cargo inputs; this check only prevents their writes from entering the configured SDK.
fn resolve_existing_ancestor(path: &Path) -> Result<PathBuf, CargoValidationFailure> {
    let mut ancestor = path;
    loop {
        if let Ok(resolved) = ancestor.canonicalize() {
            let suffix = path
                .strip_prefix(ancestor)
                .map_err(|_| CargoValidationFailure::Preparation)?;
            return Ok(lexical_join(&resolved, suffix));
        }
        ancestor = ancestor
            .parent()
            .ok_or(CargoValidationFailure::Preparation)?;
    }
}
