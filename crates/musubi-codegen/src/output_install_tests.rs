//! Atomic create/update, bounded replacement, and recoverable rollback tests.

use crate::output_test_support::seal_fixture;
use crate::{
    DiagnosticCode, HardLimits,
    output_fs::MARKER,
    output_install::{Parent, ReplaceStep, SiblingDirectory, replace_with},
};
use std::fs;

fn first_code(error: &crate::GenerateError) -> DiagnosticCode {
    error.diagnostics.iter().next().unwrap().code
}

#[test]
fn failed_rollback_preserves_the_previous_tree_and_reports_both_failures() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("sdk");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("previous"), b"preserve").unwrap();
    if output.exists() {
        seal_fixture(&output);
    }
    let parent = Parent::open(root.path()).unwrap();
    let staged = SiblingDirectory::create(&parent, ".test-stage-", HardLimits::V1).unwrap();
    let error = replace_with(
        &parent,
        std::ffi::OsStr::new("sdk"),
        staged,
        HardLimits::V1,
        crate::WriteOperation::Update,
        |step| matches!(step, ReplaceStep::Install | ReplaceStep::Rollback),
    )
    .unwrap_err();
    assert_eq!(error.diagnostics.iter().count(), 2);
    let backups = fs::read_dir(root.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".musubi-backup-")
        })
        .collect::<Vec<_>>();
    assert_eq!(backups.len(), 1);
    assert_eq!(
        fs::read(backups[0].path().join("previous/previous")).unwrap(),
        b"preserve"
    );
}

#[test]
fn backup_or_install_failure_restores_the_complete_previous_sdk() {
    for failure in [ReplaceStep::Backup, ReplaceStep::Install] {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("sdk");
        fs::create_dir_all(output.join("nested")).unwrap();
        fs::write(output.join("nested/value"), b"preserve").unwrap();
        fs::write(root.path().join("unrelated"), b"keep").unwrap();
        seal_fixture(&output);
        let before = crate::output_fs::read_tree_with_limits(&output, HardLimits::V1).unwrap();
        if output.exists() {
            seal_fixture(&output);
        }
        let parent = Parent::open(root.path()).unwrap();
        let staged = SiblingDirectory::create(&parent, ".test-stage-", HardLimits::V1).unwrap();
        seal_fixture(&parent.child_path(staged.name()));
        assert!(
            replace_with(
                &parent,
                std::ffi::OsStr::new("sdk"),
                staged,
                HardLimits::V1,
                crate::WriteOperation::Update,
                |step| step == failure
            )
            .is_err()
        );
        assert_eq!(
            crate::output_fs::read_tree_with_limits(&output, HardLimits::V1).unwrap(),
            before
        );
        assert_eq!(fs::read(root.path().join("unrelated")).unwrap(), b"keep");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    }
}

#[test]
fn replacement_revalidates_marker_after_the_backup_rename() {
    for remove_marker in [false, true] {
        for fail_rollback in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let output = root.path().join("sdk");
            fs::create_dir(&output).unwrap();
            fs::write(output.join("human"), b"preserve").unwrap();
            fs::write(root.path().join("unrelated"), b"keep").unwrap();
            if output.exists() {
                seal_fixture(&output);
            }
            let parent = Parent::open(root.path()).unwrap();
            let staged = SiblingDirectory::create(&parent, ".test-stage-", HardLimits::V1).unwrap();
            let mut changed = false;
            let mut rollback_attempted = false;
            let error = replace_with(
                &parent,
                std::ffi::OsStr::new("sdk"),
                staged,
                HardLimits::V1,
                crate::WriteOperation::Update,
                |step| {
                    if step == ReplaceStep::Backup {
                        if remove_marker {
                            fs::remove_file(output.join(MARKER)).unwrap();
                        } else {
                            fs::write(output.join(MARKER), b"musubi/generated-v2\n").unwrap();
                        }
                        changed = true;
                    }
                    if step == ReplaceStep::Rollback {
                        rollback_attempted = true;
                        return fail_rollback;
                    }
                    false
                },
            )
            .unwrap_err();
            assert!(changed && rollback_attempted);
            assert_eq!(first_code(&error), DiagnosticCode::UnsafePath);
            assert_eq!(
                error.diagnostics.iter().next().unwrap().message,
                "backed-up generation marker is invalid"
            );
            let previous = if fail_rollback {
                assert!(!output.exists());
                let backup = fs::read_dir(root.path())
                    .unwrap()
                    .filter_map(Result::ok)
                    .find(|entry| {
                        entry
                            .file_name()
                            .to_string_lossy()
                            .starts_with(".musubi-backup-")
                    })
                    .unwrap();
                assert_eq!(error.diagnostics.iter().count(), 2);
                backup.path().join("previous")
            } else {
                output
            };
            assert_eq!(fs::read(previous.join("human")).unwrap(), b"preserve");
            assert_eq!(fs::read(root.path().join("unrelated")).unwrap(), b"keep");
            assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
        }
    }
}

#[test]
fn replacement_stays_with_the_retained_parent_after_path_swap() {
    let root = tempfile::tempdir().unwrap();
    let parent_path = root.path().join("parent");
    let retained_path = root.path().join("retained");
    let replacement_path = root.path().join("replacement");
    fs::create_dir(&parent_path).unwrap();
    fs::create_dir(&replacement_path).unwrap();
    let output = parent_path.join("sdk");
    fs::create_dir(&output).unwrap();
    if output.exists() {
        seal_fixture(&output);
    }
    let parent = Parent::open(&parent_path).unwrap();
    let staged = SiblingDirectory::create(&parent, ".test-stage-", HardLimits::V1).unwrap();
    seal_fixture(&parent.child_path(staged.name()));
    let mut swapped = false;
    replace_with(
        &parent,
        std::ffi::OsStr::new("sdk"),
        staged,
        HardLimits::V1,
        crate::WriteOperation::Update,
        |step| {
            if step == ReplaceStep::Backup && !swapped {
                fs::rename(&parent_path, &retained_path).unwrap();
                fs::rename(&replacement_path, &parent_path).unwrap();
                swapped = true;
            }
            false
        },
    )
    .unwrap();
    assert!(retained_path.join("sdk").is_dir());
    assert!(!parent_path.join("sdk").exists());
}

#[test]
fn missing_target_install_never_replaces_a_racing_markerless_directory() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("sdk");
    if output.exists() {
        seal_fixture(&output);
    }
    let parent = Parent::open(root.path()).unwrap();
    let staged = SiblingDirectory::create(&parent, ".test-stage-", HardLimits::V1).unwrap();
    seal_fixture(&parent.child_path(staged.name()));
    let mut raced = false;
    let error = replace_with(
        &parent,
        std::ffi::OsStr::new("sdk"),
        staged,
        HardLimits::V1,
        crate::WriteOperation::Create,
        |step| {
            if step == ReplaceStep::Install && !raced {
                fs::create_dir(&output).unwrap();
                fs::write(output.join("human"), b"preserve").unwrap();
                raced = true;
            }
            false
        },
    )
    .unwrap_err();
    assert_eq!(first_code(&error), DiagnosticCode::InvalidInput);
    assert_eq!(fs::read(output.join("human")).unwrap(), b"preserve");
}

#[test]
fn replacement_rejects_existing_trees_beyond_effective_count_and_depth_limits() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("sdk");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("one"), []).unwrap();
    fs::write(output.join("two"), []).unwrap();
    if output.exists() {
        seal_fixture(&output);
    }
    let parent = Parent::open(root.path()).unwrap();
    let limits = HardLimits {
        generated_files: 2,
        ..HardLimits::V1
    };
    let staged = SiblingDirectory::create(&parent, ".test-stage-", limits).unwrap();
    let error = replace_with(
        &parent,
        std::ffi::OsStr::new("sdk"),
        staged,
        limits,
        crate::WriteOperation::Update,
        |_| false,
    )
    .unwrap_err();
    assert_eq!(first_code(&error), DiagnosticCode::LimitExceeded);
    assert!(output.join("two").exists());

    fs::remove_file(output.join("one")).unwrap();
    fs::remove_file(output.join("two")).unwrap();
    fs::create_dir_all(output.join("level/too-deep")).unwrap();
    let limits = HardLimits {
        nesting: 1,
        ..HardLimits::V1
    };
    let staged = SiblingDirectory::create(&parent, ".test-stage-", limits).unwrap();
    let error = replace_with(
        &parent,
        std::ffi::OsStr::new("sdk"),
        staged,
        limits,
        crate::WriteOperation::Update,
        |_| false,
    )
    .unwrap_err();
    assert_eq!(first_code(&error), DiagnosticCode::LimitExceeded);
    assert!(output.join("level/too-deep").is_dir());
}

#[test]
fn replacement_reports_bounded_backup_cleanup_failure() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("sdk");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("previous"), b"old").unwrap();
    seal_fixture(&output);
    let expected = crate::output_fs::read_tree_with_limits(&output, HardLimits::V1).unwrap();
    let limits = HardLimits {
        generated_files: 2,
        ..HardLimits::V1
    };
    if output.exists() {
        seal_fixture(&output);
    }
    let parent = Parent::open(root.path()).unwrap();
    let staged = SiblingDirectory::create(&parent, ".test-stage-", limits).unwrap();
    seal_fixture(&parent.child_path(staged.name()));
    fs::write(parent.child_path(staged.name()).join("fresh"), b"new").unwrap();
    seal_fixture(&parent.child_path(staged.name()));
    let error = replace_with(
        &parent,
        std::ffi::OsStr::new("sdk"),
        staged,
        limits,
        crate::WriteOperation::Update,
        |step| {
            if step == ReplaceStep::Install {
                let backup = fs::read_dir(root.path())
                    .unwrap()
                    .filter_map(Result::ok)
                    .find(|entry| {
                        entry
                            .file_name()
                            .to_string_lossy()
                            .starts_with(".musubi-backup-")
                    })
                    .unwrap();
                fs::write(backup.path().join("previous/overflow"), []).unwrap();
            }
            false
        },
    )
    .unwrap_err();
    assert_eq!(first_code(&error), DiagnosticCode::LimitExceeded);
    assert_eq!(fs::read(output.join("fresh")).unwrap(), b"new");
    crate::output_recovery_tests::assert_recovers(root.path(), &expected);
    assert!(
        fs::read_dir(root.path())
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with(".musubi-backup-"))
    );
}
