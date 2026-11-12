//! Exclusive manifest lifecycle, failure cleanup, and pathname-race regression tests.

use std::{
    fs,
    os::unix::{fs::symlink, net::UnixListener},
    path::Path,
};

use crate::{
    INITIAL_MANIFEST,
    init::{InitStep, initialize_with},
    init_manifest,
};

#[test]
fn init_writes_deterministic_editable_defaults_and_never_an_sdk() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    for root in [first.path(), second.path()] {
        let path = root.join("musubi.toml");
        init_manifest(&path).unwrap();
        let bytes = fs::read(&path).unwrap();
        assert_eq!(bytes, INITIAL_MANIFEST.as_bytes());
        assert!(bytes.ends_with(b"\n"));
        assert!(!bytes.ends_with(b"\n\n"));
        assert!(crate::manifest::parse(&bytes).is_err());
        let editable = String::from_utf8(bytes.clone()).unwrap()
            + "\n[runtime]\nsource = \"path\"\nroot = \".\"\n";
        let manifest = crate::manifest::parse(editable.as_bytes()).unwrap();
        assert_eq!(manifest.schema_version, "musubi/config-v4");
        assert_eq!(manifest.package.name, "generated-contract-sdk");
        assert_eq!(manifest.package.version, "0.1.0");
        assert_eq!(manifest.package.edition, "2024");
        assert_eq!(
            manifest.generation.output.as_str(),
            "generated/contract-sdk"
        );
        assert!(manifest.contracts.is_empty() && manifest.typed_data.is_empty());
        assert!(crate::load_inputs(&path).is_err());
        assert_eq!(fs::read_dir(root).unwrap().count(), 1);
    }
    assert_eq!(
        fs::read(first.path().join("musubi.toml")).unwrap(),
        fs::read(second.path().join("musubi.toml")).unwrap()
    );
}

fn create_target(path: &Path, kind: &str) {
    match kind {
        "file" => fs::write(path, b"preserve").unwrap(),
        "directory" => {
            fs::create_dir(path).unwrap();
            fs::write(path.join("human"), b"preserve").unwrap();
        }
        "symlink" => symlink("missing-external-target", path).unwrap(),
        "fifo" => {
            assert!(
                std::process::Command::new("mkfifo")
                    .arg(path)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        "socket" => {
            let _listener = UnixListener::bind(path).unwrap();
        }
        _ => unreachable!(),
    }
}

fn assert_preserved(path: &Path, kind: &str) {
    use std::os::unix::fs::FileTypeExt as _;
    let metadata = fs::symlink_metadata(path).unwrap();
    match kind {
        "file" => assert_eq!(fs::read(path).unwrap(), b"preserve"),
        "directory" => assert_eq!(fs::read(path.join("human")).unwrap(), b"preserve"),
        "symlink" => assert_eq!(
            fs::read_link(path).unwrap(),
            Path::new("missing-external-target")
        ),
        "fifo" => assert!(metadata.file_type().is_fifo()),
        "socket" => assert!(metadata.file_type().is_socket()),
        _ => unreachable!(),
    }
}

#[test]
fn init_rejects_every_existing_and_raced_entry_without_mutation() {
    for kind in ["file", "directory", "symlink", "fifo", "socket"] {
        for raced in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("musubi.toml");
            if !raced {
                create_target(&path, kind);
            }
            let result = initialize_with(&path, |step| {
                if raced && step == InitStep::Install {
                    create_target(&path, kind);
                }
                false
            });
            assert!(result.is_err(), "accepted {kind} raced={raced}");
            assert_preserved(&path, kind);
            assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
        }
    }
}

#[test]
fn init_cleans_private_partial_writes_and_install_failures() {
    for failure in [
        InitStep::TargetCheck,
        InitStep::Write,
        InitStep::Sync,
        InitStep::Install,
    ] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("unrelated"), b"preserve").unwrap();
        assert!(initialize_with(&root.path().join("musubi.toml"), |step| step == failure).is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
        assert_eq!(
            fs::read(root.path().join("unrelated")).unwrap(),
            b"preserve"
        );
    }
}

#[test]
fn init_retains_the_opened_parent_across_a_pathname_swap() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("original");
    let retained = root.path().join("retained");
    fs::create_dir(&original).unwrap();
    initialize_with(&original.join("musubi.toml"), |step| {
        if step == InitStep::TargetCheck {
            fs::rename(&original, &retained).unwrap();
            fs::create_dir(&original).unwrap();
            fs::write(original.join("musubi.toml"), b"foreign").unwrap();
        }
        false
    })
    .unwrap();
    assert_eq!(fs::read(original.join("musubi.toml")).unwrap(), b"foreign");
    assert_eq!(
        fs::read(retained.join("musubi.toml")).unwrap(),
        INITIAL_MANIFEST.as_bytes()
    );
}

#[test]
fn init_rejects_missing_and_symlink_parents() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), root.path().join("link")).unwrap();
    for parent in ["missing", "link"] {
        assert!(init_manifest(&root.path().join(parent).join("musubi.toml")).is_err());
    }
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    assert!(!root.path().join("missing").exists());
}
