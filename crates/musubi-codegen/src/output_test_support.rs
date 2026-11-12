//! Test-only adapters that bind temporary fixture paths to output capabilities.

use std::path::Path;
#[cfg(unix)]
use std::{fs, path::PathBuf};

pub(crate) use crate::output::CargoValidation;
use crate::{
    CheckReport, GenerateError, GeneratedTree, WriteOperation, output::write_with_validation,
    output_target::for_test,
};

pub(crate) fn generate(
    tree: &GeneratedTree,
    path: &Path,
    validation: CargoValidation,
) -> Result<(), GenerateError> {
    write_with_validation(tree, &for_test(path), WriteOperation::Create, validation)
}

#[cfg(unix)]
pub(crate) fn update(
    tree: &GeneratedTree,
    path: &Path,
    validation: CargoValidation,
) -> Result<(), GenerateError> {
    write_with_validation(tree, &for_test(path), WriteOperation::Update, validation)
}

pub(crate) fn check(tree: &GeneratedTree, path: &Path) -> Result<CheckReport, GenerateError> {
    crate::check(tree, &for_test(path))
}

/// Reuses the caller's cache or the private validation cache across standalone test crates.
#[cfg(unix)]
pub(crate) fn cargo_target() -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(
        || crate::cargo_paths::default_target(&workspace).unwrap(),
        |value| workspace.join(value),
    );
    fs::create_dir_all(&target).unwrap();
    target.canonicalize().unwrap()
}

/// Materializes a disposable sibling so Cargo and injected tests cannot alter the owned SDK.
#[cfg(unix)]
pub(crate) fn private_copy(tree: &GeneratedTree, fixture_root: &Path) -> PathBuf {
    tree.validate_ownership().unwrap();
    let root = fixture_root.join("standalone-sdk");
    fs::create_dir(&root).unwrap();
    for (path, contents) in tree.file_entries() {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    root
}

/// Explicitly seals synthetic test bytes before a test begins its adversarial mutation.
#[cfg(unix)]
pub(crate) fn seal_fixture(root: &Path) {
    let files = crate::output_fs::read_tree_with_limits(root, crate::HardLimits::V1).unwrap();
    let marker = crate::ownership::render_marker(&files, crate::MAX_MARKER_BYTES).unwrap();
    std::fs::write(root.join(crate::GENERATED_MARKER), marker).unwrap();
}

/// Reads all generated Rust below a semantic owner directory for cross-file surface assertions.
pub(crate) fn rust_sources(tree: &crate::GeneratedTree, prefix: &str) -> String {
    let files = tree
        .files()
        .filter(|file| {
            file.path.starts_with(prefix)
                && file
                    .path
                    .extension()
                    .is_some_and(|extension| extension == "rs")
        })
        .map(|file| String::from_utf8(file.contents).unwrap())
        .collect::<Vec<_>>();
    assert!(!files.is_empty(), "missing Rust source below {prefix}");
    files.join("\n")
}
