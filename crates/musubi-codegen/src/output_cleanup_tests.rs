//! Boundary tests for generated backup cleanup budgets.

use crate::output_test_support::seal_fixture;
use std::{ffi::OsStr, fs};

use crate::{
    HardLimits,
    output_fs::MARKER,
    output_install::{Parent, SiblingDirectory, replace},
};

#[test]
fn backup_wrapper_does_not_consume_existing_tree_count_or_depth_budget() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("sdk");
    fs::create_dir_all(output.join("one/two")).unwrap();
    fs::write(output.join("one/two/value"), []).unwrap();
    let limits = HardLimits {
        generated_files: 2,
        nesting: 2,
        ..HardLimits::V1
    };
    if output.exists() {
        seal_fixture(&output);
    }
    let parent = Parent::open(root.path()).unwrap();
    let staged = SiblingDirectory::create(&parent, ".test-stage-", limits).unwrap();
    seal_fixture(&parent.child_path(staged.name()));
    replace(
        &parent,
        OsStr::new("sdk"),
        staged,
        limits,
        crate::WriteOperation::Update,
    )
    .unwrap();
    assert!(output.join(MARKER).is_file());
    assert!(
        !fs::read_dir(root.path())
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with(".musubi-backup-"))
    );
}
