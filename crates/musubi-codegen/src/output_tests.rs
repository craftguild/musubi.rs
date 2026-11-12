//! Adversarial generated-output inspection and read-only check tests.

use crate::output_test_support::check;
#[cfg(not(unix))]
use crate::output_test_support::{CargoValidation, generate};

#[cfg(unix)]
use std::{fs, path::Path, process::Command};

#[cfg(not(unix))]
use crate::UNSUPPORTED_HOST_MESSAGE;
use crate::{DiagnosticCode, GeneratedTree};
#[cfg(unix)]
use crate::{
    HardLimits,
    output_fs::{MARKER, read_tree_with_limits},
};

#[cfg(not(unix))]
#[test]
fn unsupported_host_fails_before_generate_or_check_can_write() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("missing/target");

    let generate_error =
        generate(&GeneratedTree::default(), &output, CargoValidation::Skip).unwrap_err();
    let check_error = check(&GeneratedTree::default(), &output).unwrap_err();

    for error in [generate_error, check_error] {
        let diagnostic = error.diagnostics.iter().next().unwrap();
        assert_eq!(diagnostic.code, DiagnosticCode::ProcessFailure);
        assert_eq!(diagnostic.message, UNSUPPORTED_HOST_MESSAGE);
    }
    assert!(!root.path().join("missing").exists());
}

#[cfg(unix)]
fn first_code(error: &crate::GenerateError) -> DiagnosticCode {
    error.diagnostics.iter().next().unwrap().code
}

#[cfg(unix)]
#[test]
fn check_rejects_oversized_marker_without_unbounded_read() {
    let root = tempfile::tempdir().unwrap();
    fs::File::create(root.path().join(MARKER))
        .unwrap()
        .set_len(crate::MAX_MARKER_BYTES as u64 + 1)
        .unwrap();
    let mut tree = GeneratedTree::default();
    tree.seal().unwrap();
    let error = check(&tree, root.path()).unwrap_err();
    assert_eq!(first_code(&error), DiagnosticCode::LimitExceeded);
}

#[cfg(unix)]
#[test]
fn actual_tree_enforces_file_and_sparse_byte_limits() {
    assert_eq!(HardLimits::V1.generated_files, 10_000);
    assert_eq!(HardLimits::V1.generated_bytes, 256 * 1024 * 1024);

    let files = tempfile::tempdir().unwrap();
    for name in ["one", "two", "three"] {
        fs::write(files.path().join(name), []).unwrap();
    }
    let file_limits = HardLimits {
        generated_files: 2,
        ..HardLimits::V1
    };
    let error = read_tree_with_limits(files.path(), file_limits).unwrap_err();
    assert_eq!(first_code(&error), DiagnosticCode::LimitExceeded);

    let bytes = tempfile::tempdir().unwrap();
    let oversized = fs::File::create(bytes.path().join("sparse")).unwrap();
    oversized.set_len(9).unwrap();
    let byte_limits = HardLimits {
        generated_bytes: 8,
        ..HardLimits::V1
    };
    let error = read_tree_with_limits(bytes.path(), byte_limits).unwrap_err();
    assert_eq!(first_code(&error), DiagnosticCode::LimitExceeded);
}

#[cfg(unix)]
#[test]
fn actual_tree_enforces_directory_depth_iteratively() {
    let root = tempfile::tempdir().unwrap();
    let mut directory = root.path().to_path_buf();
    for index in 0..HardLimits::V1.nesting {
        directory.push(format!("level-{index}"));
        fs::create_dir(&directory).unwrap();
    }
    fs::write(directory.join("value"), []).unwrap();
    assert!(read_tree_with_limits(root.path(), HardLimits::V1).is_ok());
    directory.push("too-deep");
    fs::create_dir(&directory).unwrap();
    let error = read_tree_with_limits(root.path(), HardLimits::V1).unwrap_err();
    assert_eq!(first_code(&error), DiagnosticCode::LimitExceeded);
}

#[cfg(unix)]
#[test]
fn actual_tree_rejects_fifo_entries() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("nested")).unwrap();
    let fifo = root.path().join("nested/pipe");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let error = read_tree_with_limits(root.path(), HardLimits::V1).unwrap_err();
    assert_eq!(first_code(&error), DiagnosticCode::UnsafePath);
}

#[cfg(unix)]
#[test]
fn actual_tree_rejects_symlinks_below_target_without_following_them() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("nested")).unwrap();
    symlink("outside", root.path().join("nested/link")).unwrap();
    let error = read_tree_with_limits(root.path(), HardLimits::V1).unwrap_err();
    assert_eq!(first_code(&error), DiagnosticCode::UnsafePath);
}

#[cfg(unix)]
#[test]
fn check_applies_effective_file_and_byte_limits_to_actual_tree() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("expected"), b"x").unwrap();
    crate::output_test_support::seal_fixture(root.path());
    fs::create_dir(root.path().join("nested")).unwrap();
    fs::write(root.path().join("nested/unexpected"), b"unexpected").unwrap();

    let file_limits = HardLimits {
        generated_files: 2,
        ..HardLimits::V1
    };
    let mut tree = GeneratedTree::with_limits(file_limits);
    tree.insert("expected", b"x").unwrap();
    tree.seal().unwrap();
    let error = check(&tree, root.path()).unwrap_err();
    assert_eq!(first_code(&error), DiagnosticCode::LimitExceeded);

    let mut default_tree = GeneratedTree::default();
    default_tree.insert("expected", b"x").unwrap();
    default_tree.seal().unwrap();
    assert_eq!(
        check(&default_tree, root.path()).unwrap().removed,
        vec![std::path::PathBuf::from("nested/unexpected")]
    );

    let byte_limits = HardLimits {
        generated_bytes: default_tree
            .file_entries()
            .map(|(_, bytes)| bytes.len())
            .sum(),
        ..HardLimits::V1
    };
    let mut tree = GeneratedTree::with_limits(byte_limits);
    tree.insert("expected", b"x").unwrap();
    tree.seal().unwrap();
    let error = check(&tree, root.path()).unwrap_err();
    assert_eq!(first_code(&error), DiagnosticCode::LimitExceeded);
}

#[cfg(unix)]
#[test]
fn check_does_not_create_a_missing_parent() {
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join("missing").join("nested");
    let output = parent.join("sdk");
    let mut tree = GeneratedTree::default();
    tree.seal().unwrap();
    assert_eq!(
        check(&tree, &output).unwrap().added,
        vec![std::path::PathBuf::from(crate::GENERATED_MARKER)]
    );
    assert!(!Path::new(&parent).exists());
}
