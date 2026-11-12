//! Manifest-owned lifecycle authority, read-only checking, and write-failure containment.

use crate::{
    GeneratedTree, OutputTarget, WriteOperation, check, generate, load_inputs,
    output::{CargoValidation, write_with_validation},
};
use std::{fs, os::unix::fs::symlink, path::Path};

fn inputs(root: &Path, output: &str) -> OutputTarget {
    fs::write(root.join("abi.json"), b"[]").unwrap();
    fs::write(root.join("musubi.toml"), format!(
        "schema_version = \"musubi/config-v4\"\n[runtime]\nsource = \"path\"\nroot = \"../..\"\n[package]\nname = \"test-sdk\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[generation]\noutput = {output:?}\n[[contracts]]\nid = \"test\"\nkind = \"interface\"\nabi = \"abi.json\"\n"
    )).unwrap();
    load_inputs(&root.join("musubi.toml")).unwrap().output
}

fn tree(value: &[u8]) -> GeneratedTree {
    let mut tree = GeneratedTree::default();
    tree.insert("value", value).unwrap();
    tree.seal().unwrap();
    tree
}

fn write(
    tree: &GeneratedTree,
    output: &OutputTarget,
    operation: WriteOperation,
) -> Result<(), crate::GenerateError> {
    write_with_validation(tree, output, operation, CargoValidation::Skip)
}

#[test]
fn create_and_update_have_disjoint_authority_and_never_modify_the_manifest() {
    let root = tempfile::tempdir().unwrap();
    let output = inputs(root.path(), "generated/sdk");
    let manifest = fs::read(root.path().join("musubi.toml")).unwrap();
    assert!(write(&tree(b"first"), &output, WriteOperation::Update).is_err());
    assert!(!root.path().join("generated").exists());
    write(&tree(b"first"), &output, WriteOperation::Create).unwrap();
    assert!(write(&tree(b"second"), &output, WriteOperation::Create).is_err());
    assert_eq!(
        fs::read(root.path().join("generated/sdk/value")).unwrap(),
        b"first"
    );
    write(&tree(b"second"), &output, WriteOperation::Update).unwrap();
    assert!(check(&tree(b"second"), &output).unwrap().is_clean());
    assert_eq!(fs::read(root.path().join("musubi.toml")).unwrap(), manifest);
    assert_eq!(
        fs::read_dir(root.path().join("generated")).unwrap().count(),
        1
    );
}

#[test]
fn missing_parent_creation_retains_the_deepest_existing_directory_across_a_swap() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("prefix")).unwrap();
    let output = inputs(root.path(), "prefix/missing/sdk");
    let retained = output.resolve().unwrap();
    assert!(retained.existing().is_none());
    fs::rename(root.path().join("prefix"), root.path().join("retained")).unwrap();
    symlink(outside.path(), root.path().join("prefix")).unwrap();
    let (parent, name) = retained.create_parent().unwrap();
    let tree = tree(b"owned");
    let staging = crate::output_install::SiblingDirectory::create(
        &parent,
        ".test-stage-",
        crate::HardLimits::V1,
    )
    .unwrap();
    staging.write_tree(&tree).unwrap();
    crate::output_install::replace(
        &parent,
        &name,
        staging,
        crate::HardLimits::V1,
        WriteOperation::Create,
    )
    .unwrap();
    assert_eq!(
        fs::read(root.path().join("retained/missing/sdk/value")).unwrap(),
        b"owned"
    );
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    assert!(check(&tree, &output).is_err());
}

#[test]
fn every_foreign_target_is_rejected_by_both_write_operations_and_check() {
    for kind in ["directory", "file", "symlink", "fifo"] {
        let root = tempfile::tempdir().unwrap();
        let output = inputs(root.path(), "sdk");
        let path = root.path().join("sdk");
        fs::write(root.path().join("unrelated"), b"preserve").unwrap();
        match kind {
            "directory" => {
                fs::create_dir(&path).unwrap();
                fs::write(path.join("human"), b"preserve").unwrap();
            }
            "file" => fs::write(&path, b"preserve").unwrap(),
            "symlink" => symlink(root.path().join("unrelated"), &path).unwrap(),
            "fifo" => assert!(
                std::process::Command::new("mkfifo")
                    .arg(&path)
                    .status()
                    .unwrap()
                    .success()
            ),
            _ => unreachable!(),
        }
        let before = fs::symlink_metadata(&path).unwrap().file_type();
        for operation in [WriteOperation::Create, WriteOperation::Update] {
            assert!(
                write(&tree(b"new"), &output, operation).is_err(),
                "accepted {kind}"
            );
        }
        assert!(check(&tree(b"new"), &output).is_err());
        assert_eq!(fs::symlink_metadata(&path).unwrap().file_type(), before);
        if kind == "directory" {
            assert_eq!(fs::read(path.join("human")).unwrap(), b"preserve");
        }
        if kind == "file" {
            assert_eq!(fs::read(&path).unwrap(), b"preserve");
        }
        assert_eq!(
            fs::read(root.path().join("unrelated")).unwrap(),
            b"preserve"
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 4);
    }
}

#[test]
fn symlink_parents_are_rejected_before_output_mutation() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let output = inputs(root.path(), "linked/nested/sdk");
    symlink(outside.path(), root.path().join("linked")).unwrap();
    for operation in [WriteOperation::Create, WriteOperation::Update] {
        assert!(write(&tree(b"new"), &output, operation).is_err());
    }
    assert!(check(&tree(b"new"), &output).is_err());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}

#[test]
fn output_uses_the_same_manifest_root_handle_after_a_pathname_swap() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("original");
    let retained = root.path().join("retained");
    fs::create_dir(&original).unwrap();
    let output = inputs(&original, "nested/sdk");
    fs::rename(&original, &retained).unwrap();
    fs::create_dir(&original).unwrap();
    write(&tree(b"retained"), &output, WriteOperation::Create).unwrap();
    assert_eq!(
        fs::read(retained.join("nested/sdk/value")).unwrap(),
        b"retained"
    );
    assert!(check(&tree(b"retained"), &output).unwrap().is_clean());
    assert_eq!(fs::read_dir(&original).unwrap().count(), 0);
}

#[test]
fn validation_and_conflicting_tree_failures_preserve_the_previous_sdk() {
    for operation in [WriteOperation::Create, WriteOperation::Update] {
        let root = tempfile::tempdir().unwrap();
        let output = inputs(root.path(), "sdk");
        if operation == WriteOperation::Update {
            write(&tree(b"old"), &output, WriteOperation::Create).unwrap();
        }
        // No Cargo manifest: fails private validation preparation, before installing any bytes.
        assert!(generate(&tree(b"new"), &output, operation).is_err());
        let mut conflict = tree(b"new");
        conflict.insert("collision", b"file").unwrap();
        assert!(
            conflict
                .insert("collision/child", b"cannot-create")
                .is_err()
        );
        assert!(write(&conflict, &output, operation).is_err());
        if operation == WriteOperation::Update {
            assert!(check(&tree(b"old"), &output).unwrap().is_clean());
        } else {
            assert!(!root.path().join("sdk").exists());
        }
        assert_eq!(
            fs::read_dir(root.path()).unwrap().count(),
            if operation == WriteOperation::Update {
                3
            } else {
                2
            }
        );
    }
}

#[test]
fn control_bearing_drift_names_are_rejected_without_exposing_them() {
    for name in ["SECRET\nvalue", "SECRET\u{0085}value", "SECRET\\value"] {
        let root = tempfile::tempdir().unwrap();
        let output = inputs(root.path(), "sdk");
        let expected = tree(b"old");
        write(&expected, &output, WriteOperation::Create).unwrap();
        let extra = root.path().join("sdk").join(name);
        fs::write(&extra, b"preserve").unwrap();
        for result in [
            check(&expected, &output).map(|_| ()),
            write(&tree(b"new"), &output, WriteOperation::Update),
        ] {
            let error = result.unwrap_err();
            assert!(error.diagnostics.iter().all(|diagnostic| {
                diagnostic.location.is_none() && !diagnostic.message.contains("SECRET")
            }));
        }
        assert_eq!(fs::read(extra).unwrap(), b"preserve");
        assert_eq!(fs::read(root.path().join("sdk/value")).unwrap(), b"old");
    }
}

#[test]
fn exact_check_reports_each_change_without_rewriting_any_file() {
    let root = tempfile::tempdir().unwrap();
    let output = inputs(root.path(), "sdk");
    let mut expected = tree(b"old");
    expected.insert("missing", b"expected").unwrap();
    expected.seal().unwrap();
    write(&expected, &output, WriteOperation::Create).unwrap();
    fs::write(root.path().join("sdk/value"), b"changed").unwrap();
    fs::remove_file(root.path().join("sdk/missing")).unwrap();
    fs::write(root.path().join("sdk/extra"), b"extra").unwrap();
    for _ in 0..2 {
        let report = check(&expected, &output).unwrap();
        assert_eq!(report.added, vec![Path::new("missing")]);
        assert_eq!(report.removed, vec![Path::new("extra")]);
        assert_eq!(report.changed, vec![Path::new("value")]);
    }
    assert_eq!(fs::read(root.path().join("sdk/value")).unwrap(), b"changed");
    assert_eq!(fs::read(root.path().join("sdk/extra")).unwrap(), b"extra");
    assert!(!root.path().join("sdk/missing").exists());
}

#[test]
fn input_only_changes_can_update_but_unknown_or_edited_old_bytes_cannot() {
    let root = tempfile::tempdir().unwrap();
    let output = inputs(root.path(), "sdk");
    write(&tree(b"old"), &output, WriteOperation::Create).unwrap();
    fs::write(root.path().join("abi.json"), b"[ ]").unwrap();
    let manifest = root.path().join("musubi.toml");
    let mut updated = fs::read(&manifest).unwrap();
    updated.extend_from_slice(b"# reviewed input change\n");
    fs::write(&manifest, &updated).unwrap();
    let new_output = load_inputs(&manifest).unwrap().output;
    write(&tree(b"new"), &new_output, WriteOperation::Update).unwrap();
    for path in ["unknown", "value"] {
        fs::write(root.path().join("sdk").join(path), b"edited").unwrap();
        assert!(write(&tree(b"next"), &new_output, WriteOperation::Update).is_err());
        assert_eq!(
            fs::read(root.path().join("sdk").join(path)).unwrap(),
            b"edited"
        );
        if path == "unknown" {
            fs::remove_file(root.path().join("sdk/unknown")).unwrap();
        }
    }
    assert_eq!(fs::read(&manifest).unwrap(), updated);
}
