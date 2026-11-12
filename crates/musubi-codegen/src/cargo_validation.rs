//! Private generated sources and lockfiles validated with ordinary Cargo settings and caches.

use std::{fs, path::Path, process::Command};

pub(crate) use crate::cargo_paths::require_external_cache;
use crate::cargo_paths::{cargo_home, default_target, rebind_runtime_paths};

use crate::{
    CARGO_PROCESS_LIMITS, GeneratedTree, ProcessFailure, ProcessOutput, ProcessRequest,
    run_bounded_process,
};

/// Private generated-tree validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoValidationFailure {
    /// The private source, manifest, or reusable Cargo target could not be prepared.
    Preparation,
    /// A bounded Cargo child failed to start, finish, or stay within its limits.
    Process(ProcessFailure),
    /// Cargo completed and rejected the generated crate.
    Status,
}

/// Cargo check command and its private source/lockfile lifetime.
///
/// Keep the guard alive until the caller's bounded process tree has finished. Cargo caches are
/// independently owned and remain available after the source guard is dropped.
#[derive(Debug)]
pub struct PreparedCargoCheck {
    command: Command,
    _source: tempfile::TempDir,
}

impl PreparedCargoCheck {
    /// Returns the command for caller-owned bounded execution.
    pub fn command_mut(&mut self) -> &mut Command {
        &mut self.command
    }

    fn run(&self) -> Result<(), CargoValidationFailure> {
        let arguments = self
            .command
            .get_args()
            .map(|arg| arg.to_str().ok_or(CargoValidationFailure::Preparation))
            .collect::<Result<Vec<_>, _>>()?;
        let environment = self
            .command
            .get_envs()
            .map(|(name, value)| {
                Ok((
                    name.to_str().ok_or(CargoValidationFailure::Preparation)?,
                    Path::new(value.ok_or(CargoValidationFailure::Preparation)?),
                ))
            })
            .collect::<Result<Vec<_>, CargoValidationFailure>>()?;
        run_cargo(
            &arguments,
            self.command
                .get_current_dir()
                .ok_or(CargoValidationFailure::Preparation)?,
            &environment,
        )
        .map(|_| ())
    }
}

/// Prepares bounded lockfile/metadata resolution and a private Cargo check command.
///
/// Configuration and caches follow ordinary Cargo settings. Relative runtime paths are rebound
/// using the original conceptual SDK root. No original source or lockfile is reopened or changed.
///
/// # Errors
///
/// Returns a redacted preparation or bounded-process failure, including cache/SDK overlap.
pub fn prepare_cargo_check(
    tree: &GeneratedTree,
    conceptual_root: &Path,
    configuration_root: &Path,
) -> Result<PreparedCargoCheck, CargoValidationFailure> {
    crate::host_capability_preflight().map_err(CargoValidationFailure::Process)?;
    tree.validate_ownership()
        .map_err(|_| CargoValidationFailure::Preparation)?;
    let temporary = tempfile::tempdir().map_err(|_| CargoValidationFailure::Preparation)?;
    prepare_in_workspace(tree, conceptual_root, configuration_root, temporary)
}

/// Compiles the exact generated sources from a private snapshot.
///
/// Explicit relative dependencies are rebound only inside the private copy, using `conceptual_root`
/// as their original SDK location. Cargo discovers trusted configuration from `configuration_root`
/// and inherits the caller's Cargo home, toolchain, wrappers and flags. Its target cache is reused;
/// source and lockfile changes remain private.
///
/// # Errors
///
/// Returns [`CargoValidationFailure`] for preparation, bounded-process, or compile failures.
pub fn validate_generated_cargo(
    tree: &GeneratedTree,
    conceptual_root: &Path,
    configuration_root: &Path,
) -> Result<(), CargoValidationFailure> {
    crate::host_capability_preflight().map_err(CargoValidationFailure::Process)?;
    let prepared = prepare_cargo_check(tree, conceptual_root, configuration_root)?;
    prepared.run()
}

/// Validates a private generated snapshot using the manifest-owned location for path rebinding.
///
/// The configured output is never created, opened, or changed by this check.
///
/// # Errors
///
/// Returns [`CargoValidationFailure`] when private preparation or bounded Cargo validation fails.
pub fn doctor(
    tree: &GeneratedTree,
    output: &crate::OutputTarget,
) -> Result<(), CargoValidationFailure> {
    crate::host_capability_preflight().map_err(CargoValidationFailure::Process)?;
    validate_generated_cargo(tree, &output.conceptual_path(), output.configuration_root())
}

fn prepare_in_workspace(
    tree: &GeneratedTree,
    conceptual_root: &Path,
    configuration_root: &Path,
    temporary: tempfile::TempDir,
) -> Result<PreparedCargoCheck, CargoValidationFailure> {
    tree.validate_ownership()
        .map_err(|_| CargoValidationFailure::Preparation)?;
    require_external_cache(&cargo_home()?, conceptual_root, configuration_root)?;
    if let Some(path) = std::env::var_os("CARGO_TARGET_DIR") {
        require_external_cache(Path::new(&path), conceptual_root, configuration_root)?;
    }
    let source = temporary.path().join("source");
    fs::create_dir(&source).map_err(|_| CargoValidationFailure::Preparation)?;
    materialize(tree, &source)?;
    rebind_runtime_paths(&source.join("Cargo.toml"), conceptual_root)?;
    let manifest = source.join("Cargo.toml");
    let manifest = manifest
        .to_str()
        .ok_or(CargoValidationFailure::Preparation)?;
    if !source.join("Cargo.lock").exists() {
        run_cargo(
            &["generate-lockfile", "--manifest-path", manifest],
            configuration_root,
            &[],
        )?;
    }
    // Let Cargo interpret its own configuration (including build.target-dir). Only its default
    // private-source target is replaced with a reusable cache outside the generated tree.
    let metadata = run_cargo(
        &[
            "metadata",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
            manifest,
        ],
        configuration_root,
        &[],
    )?;
    let metadata: serde_json::Value = serde_json::from_slice(&metadata.stdout)
        .map_err(|_| CargoValidationFailure::Preparation)?;
    let target = metadata
        .get("target_directory")
        .and_then(serde_json::Value::as_str)
        .ok_or(CargoValidationFailure::Preparation)?;
    let fallback = if std::env::var_os("CARGO_TARGET_DIR").is_none()
        && Path::new(target) == source.join("target")
    {
        Some(default_target(configuration_root)?)
    } else {
        None
    };
    let environment = fallback
        .as_ref()
        .map(|path| ("CARGO_TARGET_DIR", path.as_path()))
        .into_iter()
        .collect::<Vec<_>>();
    require_external_cache(
        fallback.as_deref().unwrap_or_else(|| Path::new(target)),
        conceptual_root,
        configuration_root,
    )?;
    let mut command = Command::new("cargo");
    command
        .args([
            "check",
            "--quiet",
            "--locked",
            "--all-targets",
            "--manifest-path",
            manifest,
        ])
        .current_dir(configuration_root);
    for (name, value) in environment {
        command.env(name, value);
    }
    Ok(PreparedCargoCheck {
        command,
        _source: temporary,
    })
}

#[cfg(all(test, unix))]
pub(crate) fn validate_in_parent(
    tree: &GeneratedTree,
    conceptual_root: &Path,
    configuration_root: &Path,
    temporary_parent: &Path,
) -> Result<(), CargoValidationFailure> {
    let temporary =
        tempfile::tempdir_in(temporary_parent).map_err(|_| CargoValidationFailure::Preparation)?;
    prepare_in_workspace(tree, conceptual_root, configuration_root, temporary)?.run()
}

fn materialize(tree: &GeneratedTree, root: &Path) -> Result<(), CargoValidationFailure> {
    for (path, contents) in tree.file_entries() {
        let output = root.join(path);
        let parent = output.parent().ok_or(CargoValidationFailure::Preparation)?;
        fs::create_dir_all(parent).map_err(|_| CargoValidationFailure::Preparation)?;
        fs::write(output, contents).map_err(|_| CargoValidationFailure::Preparation)?;
    }
    Ok(())
}

fn run_cargo(
    arguments: &[&str],
    configuration_root: &Path,
    environment: &[(&str, &Path)],
) -> Result<ProcessOutput, CargoValidationFailure> {
    let output = run_bounded_process(
        ProcessRequest {
            program: "cargo",
            arguments,
            current_dir: configuration_root,
            environment,
        },
        CARGO_PROCESS_LIMITS,
    )
    .map_err(CargoValidationFailure::Process)?;
    let _captured_bytes = output.stdout.len().saturating_add(output.stderr.len());
    if output.status.success() {
        Ok(output)
    } else {
        Err(CargoValidationFailure::Status)
    }
}

#[cfg(all(test, not(unix)))]
mod tests {
    use super::*;

    #[test]
    fn unsupported_host_fails_before_doctor_creates_its_temporary_directory() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("missing");
        let output = crate::output_target::for_test(&missing);
        assert_eq!(
            doctor(&GeneratedTree::default(), &output),
            Err(CargoValidationFailure::Process(
                ProcessFailure::UnsupportedPlatform
            ))
        );
        assert!(!missing.exists());
    }
}
