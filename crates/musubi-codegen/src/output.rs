//! Safe staged writes, marker-gated replacement, and exact generated-tree checks.

use std::path::PathBuf;
#[cfg(unix)]
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[cfg(unix)]
use crate::{CargoValidationFailure, ProcessFailure, validate_generated_cargo};
use crate::{
    Diagnostic, DiagnosticCode, Diagnostics, GeneratedTree, OutputTarget, UNSUPPORTED_HOST_MESSAGE,
    host_capability_preflight, render::RenderError,
};

/// Staging, validation, or generated-directory replacement failure.
#[derive(Clone, Debug, thiserror::Error)]
#[error("generated output operation failed")]
pub struct GenerateError {
    /// Stable redacted diagnostics.
    pub diagnostics: Diagnostics,
}

impl From<RenderError> for GenerateError {
    fn from(error: RenderError) -> Self {
        Self {
            diagnostics: diagnostics(error.diagnostic),
        }
    }
}

/// Exact generated output drift.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CheckReport {
    /// Informational provenance differences; these do not constitute byte drift.
    pub notices: Vec<Diagnostic>,
    /// Paths expected from current inputs but absent from disk.
    pub added: Vec<PathBuf>,
    /// Existing generated paths no longer expected from current inputs.
    pub removed: Vec<PathBuf>,
    /// Paths present in both trees with different bytes or file kinds.
    pub changed: Vec<PathBuf>,
}

impl CheckReport {
    /// Reports whether generated output is byte-identical.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

/// Disjoint authority for generated SDK writes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteOperation {
    /// Install only when the configured target is absent, even if an existing SDK is owned.
    Create,
    /// Replace only an existing SDK carrying this generator's valid ownership marker.
    Update,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CargoValidation {
    Required,
    #[cfg(test)]
    Skip,
}

/// Compiles a private snapshot and stages a create-only or marker-owned update operation.
///
/// # Errors
///
/// Returns [`GenerateError`] for an unsupported secure host, unsafe targets, I/O failure, staged
/// compile failure, a marker-less existing directory, or failed replacement/rollback.
pub fn generate(
    tree: &GeneratedTree,
    output: &OutputTarget,
    operation: WriteOperation,
) -> Result<(), GenerateError> {
    write_with_validation(tree, output, operation, CargoValidation::Required)
}

pub(crate) fn write_with_validation(
    tree: &GeneratedTree,
    output: &OutputTarget,
    operation: WriteOperation,
    validation: CargoValidation,
) -> Result<(), GenerateError> {
    host_capability_preflight().map_err(|_| unsupported_host_error())?;
    tree.validate_ownership()?;
    #[cfg(unix)]
    {
        use crate::output_install::validate_operation;
        let retained = output.resolve()?;
        if let Some((parent, name)) = retained.existing() {
            validate_operation(parent, name, tree.limits(), operation)?;
        } else if operation == WriteOperation::Update {
            return Err(unsafe_error("update requires an existing marker-owned SDK"));
        }
        if validation == CargoValidation::Required {
            validate_generated_cargo(tree, &output.conceptual_path(), output.configuration_root())
                .map_err(cargo_validation_error)?;
        }
        let (parent, file_name) = retained.create_parent()?;
        validate_operation(&parent, &file_name, tree.limits(), operation)?;
        let staging = crate::output_install::SiblingDirectory::create(
            &parent,
            ".musubi-stage-",
            tree.limits(),
        )?;
        staging.write_tree(tree)?;
        crate::output_install::replace(&parent, &file_name, staging, tree.limits(), operation)
    }
    #[cfg(not(unix))]
    {
        let _ = (tree, output, operation, validation);
        Err(unsupported_host_error())
    }
}

/// Compares an expected tree with a marker-owned generated directory without writing.
///
/// # Errors
///
/// Returns [`GenerateError`] for an unsupported secure host, or if the target is unsafe,
/// marker-less, unreadable, or contains a symlink.
pub fn check(tree: &GeneratedTree, output: &OutputTarget) -> Result<CheckReport, GenerateError> {
    host_capability_preflight().map_err(|_| unsupported_host_error())?;
    tree.validate_ownership()?;
    #[cfg(unix)]
    {
        let retained = output.resolve()?;
        let Some((parent, name)) = retained.existing() else {
            return Ok(missing_report(tree));
        };
        let Some(actual) =
            crate::output_fs::read_tree_at(parent.descriptor(), name, tree.limits())?
        else {
            return Ok(missing_report(tree));
        };
        compare(tree, &actual)
    }
    #[cfg(not(unix))]
    {
        let _ = (tree, output);
        Err(unsupported_host_error())
    }
}

#[cfg(unix)]
pub(crate) fn missing_report(tree: &GeneratedTree) -> CheckReport {
    CheckReport {
        added: tree
            .file_entries()
            .map(|(path, _)| path.to_path_buf())
            .collect(),
        ..CheckReport::default()
    }
}

#[cfg(unix)]
pub(crate) fn compare(
    tree: &GeneratedTree,
    actual: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<CheckReport, GenerateError> {
    tree.validate_ownership()?;
    let expected: BTreeMap<_, _> = tree.file_entries().collect();
    let marker = actual
        .get(Path::new(crate::GENERATED_MARKER))
        .ok_or_else(|| unsafe_error("existing generation marker is missing"))?;
    // Structural validation fails closed, while a well-formed stale record still permits precise
    // added/removed/changed diagnostics. A clean comparison binds all records to expected bytes.
    crate::parse_ownership_marker(marker, tree.limits())
        .map_err(crate::output_fs::ownership_error)?;
    let expected_paths: BTreeSet<_> = expected.keys().map(|path| path.to_path_buf()).collect();
    let actual_paths: BTreeSet<_> = actual.keys().cloned().collect();
    let mut report = CheckReport {
        notices: Vec::new(),
        added: expected_paths.difference(&actual_paths).cloned().collect(),
        removed: actual_paths.difference(&expected_paths).cloned().collect(),
        changed: expected_paths
            .intersection(&actual_paths)
            .filter(|path| {
                expected.get(path.as_path()).copied() != actual.get(*path).map(Vec::as_slice)
            })
            .cloned()
            .collect(),
    };
    report.added.sort();
    report.removed.sort();
    report.changed.sort();
    Ok(report)
}

#[cfg(unix)]
fn cargo_validation_error(failure: CargoValidationFailure) -> GenerateError {
    match failure {
        CargoValidationFailure::Preparation => {
            io_error("private staged Cargo validation cannot be prepared")
        }
        CargoValidationFailure::Process(ProcessFailure::UnsupportedPlatform) => {
            process_error("staged Cargo process-tree management is unsupported on this platform")
        }
        CargoValidationFailure::Process(_) => {
            process_error("staged cargo check exceeded its process limits or failed")
        }
        CargoValidationFailure::Status => process_error("staged cargo check failed"),
    }
}

fn diagnostics(diagnostic: Diagnostic) -> Diagnostics {
    let mut diagnostics = Diagnostics::new();
    diagnostics.push(diagnostic);
    diagnostics
}

pub(crate) fn io_error(message: &str) -> GenerateError {
    GenerateError {
        diagnostics: diagnostics(Diagnostic::error(DiagnosticCode::InvalidInput, message)),
    }
}
#[cfg(unix)]
pub(crate) fn unsafe_error(message: &str) -> GenerateError {
    GenerateError {
        diagnostics: diagnostics(Diagnostic::error(DiagnosticCode::UnsafePath, message)),
    }
}
fn process_error(message: &str) -> GenerateError {
    GenerateError {
        diagnostics: diagnostics(Diagnostic::error(DiagnosticCode::ProcessFailure, message)),
    }
}
fn unsupported_host_error() -> GenerateError {
    process_error(UNSUPPORTED_HOST_MESSAGE)
}
#[cfg(unix)]
pub(crate) fn limit_error(message: &str) -> GenerateError {
    GenerateError {
        diagnostics: diagnostics(Diagnostic::error(DiagnosticCode::LimitExceeded, message)),
    }
}
#[cfg(unix)]
pub(crate) fn replacement_rollback_error() -> GenerateError {
    let mut diagnostics = Diagnostics::new();
    diagnostics.push(Diagnostic::error(
        DiagnosticCode::InvalidInput,
        "generated replacement failed",
    ));
    diagnostics.push(Diagnostic::error(
        DiagnosticCode::InvalidInput,
        "generated rollback failed; previous tree was preserved in a sibling backup directory",
    ));
    GenerateError { diagnostics }
}

#[cfg(unix)]
pub(crate) fn ownership_rollback_error(mut primary: GenerateError) -> GenerateError {
    primary.diagnostics.push(Diagnostic::error(
        DiagnosticCode::UnsafePath,
        "generated ownership validation failed and rollback failed; previous tree was preserved in a sibling backup directory",
    ));
    primary
}
