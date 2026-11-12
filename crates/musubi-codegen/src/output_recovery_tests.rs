//! Previous SDK bytes survive partial cleanup and the final recovery-file unlink.

use crate::output_test_support::seal_fixture;
use crate::{
    HardLimits, WriteOperation,
    output_fs::read_tree_with_limits,
    output_install::{Parent, ReplaceStep, SiblingDirectory, replace_with},
};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn assert_recovers(root: &Path, expected: &BTreeMap<PathBuf, Vec<u8>>) {
    let archive = fs::read_dir(root)
        .unwrap()
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".musubi-recovery-")
        })
        .expect("complete single-file recovery evidence")
        .path();
    let recovered = root.join("recovered");
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/extract-sdk-recovery.py");
    assert!(
        Command::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/python.sh"))
            .arg(script)
            .arg(archive.canonicalize().unwrap())
            .arg(root.canonicalize().unwrap().join("recovered"))
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(
        &read_tree_with_limits(&recovered, HardLimits::V1).unwrap(),
        expected
    );
    assert!(
        archive.exists(),
        "extraction must not consume the only recovery evidence"
    );
}

#[test]
fn recovery_preparation_cleanup_and_final_unlink_failures_preserve_prior_bytes() {
    for failure in [
        ReplaceStep::RecoveryWrite,
        ReplaceStep::Cleanup,
        ReplaceStep::RecoveryRemove,
    ] {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("sdk");
        fs::create_dir_all(output.join("nested")).unwrap();
        fs::write(output.join("nested/old"), b"original file bytes").unwrap();
        fs::write(root.path().join("unrelated"), b"untouched").unwrap();
        seal_fixture(&output);
        let expected = read_tree_with_limits(&output, HardLimits::V1).unwrap();
        if output.exists() {
            seal_fixture(&output);
        }
        let parent = Parent::open(root.path()).unwrap();
        let staged = SiblingDirectory::create(&parent, ".test-stage-", HardLimits::V1).unwrap();
        seal_fixture(&parent.child_path(staged.name()));
        let error = replace_with(
            &parent,
            OsStr::new("sdk"),
            staged,
            HardLimits::V1,
            WriteOperation::Update,
            |step| step == failure,
        )
        .unwrap_err();
        assert!(
            !error
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(root.path().to_str().unwrap()))
        );
        if failure == ReplaceStep::RecoveryWrite {
            let backup = fs::read_dir(root.path())
                .unwrap()
                .filter_map(Result::ok)
                .find(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".musubi-backup-")
                })
                .unwrap()
                .path()
                .join("previous");
            assert_eq!(
                read_tree_with_limits(&backup, HardLimits::V1).unwrap(),
                expected
            );
            assert!(
                !fs::read_dir(root.path())
                    .unwrap()
                    .filter_map(Result::ok)
                    .any(|entry| entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".musubi-recovery-"))
            );
        } else {
            assert_recovers(root.path(), &expected);
        }
        assert_eq!(
            fs::read(root.path().join("unrelated")).unwrap(),
            b"untouched"
        );
    }
}
