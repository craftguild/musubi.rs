//! Real TERM interruption leaves the previous SDK at its live, backup, or recovery location.

use crate::output_test_support::seal_fixture;
use crate::{
    CARGO_PROCESS_LIMITS, HardLimits, ProcessRequest, WriteOperation,
    output_fs::read_tree_with_limits,
    output_install::{Parent, ReplaceStep, SiblingDirectory, replace_with},
    run_bounded_process,
};
use std::{ffi::OsStr, fs, os::unix::process::ExitStatusExt as _, path::Path};

#[test]
fn terminated_update_child() {
    // Only the explicitly spawned subprocess receives these test-only handshakes.
    let Some(root) = std::env::var_os("MUSUBI_TEST_TERM_ROOT") else {
        return;
    };
    let step = std::env::var("MUSUBI_TEST_TERM_STEP").unwrap();
    let root = Path::new(&root);
    let parent = Parent::open(root).unwrap();
    let staging = SiblingDirectory::create(&parent, ".test-stage-", HardLimits::V1).unwrap();
    seal_fixture(&parent.child_path(staging.name()));
    replace_with(
        &parent,
        OsStr::new("sdk"),
        staging,
        HardLimits::V1,
        WriteOperation::Update,
        |current| {
            let terminate = matches!(
                (current, step.as_str()),
                (ReplaceStep::Backup, "backup")
                    | (ReplaceStep::Install, "install")
                    | (ReplaceStep::Cleanup, "cleanup")
                    | (ReplaceStep::RecoveryRemove, "recovery-remove")
            );
            if terminate {
                rustix::process::kill_process(
                    rustix::process::getpid(),
                    rustix::process::Signal::TERM,
                )
                .unwrap();
                panic!("TERM unexpectedly returned control");
            }
            false
        },
    )
    .unwrap();
    panic!("test interruption point was not reached");
}

#[test]
fn term_during_update_preserves_previous_sdk_bytes_without_touching_unrelated_paths() {
    for step in ["backup", "install", "cleanup", "recovery-remove"] {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("sdk");
        fs::create_dir_all(output.join("nested")).unwrap();
        fs::write(output.join("nested/value"), b"old bytes").unwrap();
        fs::write(root.path().join("unrelated"), b"keep").unwrap();
        seal_fixture(&output);
        let previous = read_tree_with_limits(&output, HardLimits::V1).unwrap();
        let executable = std::env::current_exe().unwrap();
        let result = run_bounded_process(
            ProcessRequest {
                program: executable.to_str().unwrap(),
                arguments: &[
                    "--exact",
                    "output_term_tests::terminated_update_child",
                    "--nocapture",
                ],
                current_dir: root.path(),
                environment: &[
                    ("MUSUBI_TEST_TERM_ROOT", root.path()),
                    ("MUSUBI_TEST_TERM_STEP", Path::new(step)),
                ],
            },
            CARGO_PROCESS_LIMITS,
        )
        .unwrap();
        assert_eq!(result.status.signal(), Some(15));
        if step == "backup" {
            assert_eq!(
                read_tree_with_limits(&output, HardLimits::V1).unwrap(),
                previous
            );
        } else if step == "recovery-remove" {
            crate::output_recovery_tests::assert_recovers(root.path(), &previous);
        } else {
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
                previous
            );
        }
        assert_eq!(fs::read(root.path().join("unrelated")).unwrap(), b"keep");
    }
}
