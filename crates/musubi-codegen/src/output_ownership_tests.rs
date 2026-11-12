//! Public retained ownership inspection rejects filesystem ambiguity before consumers see bytes.

use std::{
    fs,
    os::unix::{ffi::OsStrExt as _, fs::symlink},
    path::{Path, PathBuf},
};

use crate::{GENERATED_MARKER, GeneratedTree, HardLimits, read_owned_tree};

fn fixture() -> (tempfile::TempDir, PathBuf) {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let mut tree = GeneratedTree::default();
    tree.insert("src/lib.rs", b"// SDK bytes\n").unwrap();
    tree.seal().unwrap();
    for (path, bytes) in tree.file_entries() {
        let destination = root.join(path);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(destination, bytes).unwrap();
    }
    (temporary, root)
}

#[test]
fn owned_reader_rejects_stale_records_extra_files_and_empty_directories() {
    for change in [
        "extra",
        "empty",
        "removed",
        "changed",
        "old-marker",
        "target",
        "lock",
    ] {
        let (_temporary, root) = fixture();
        read_owned_tree(&root, HardLimits::V1).unwrap();
        match change {
            "extra" => fs::write(root.join("extra"), []).unwrap(),
            "empty" => fs::create_dir(root.join("empty")).unwrap(),
            "removed" => fs::remove_file(root.join("src/lib.rs")).unwrap(),
            "changed" => fs::write(root.join("src/lib.rs"), b"// new bytes\n").unwrap(),
            "old-marker" => {
                fs::write(root.join(GENERATED_MARKER), b"musubi/generated-v1\n").unwrap();
            }
            "target" => fs::create_dir(root.join("target")).unwrap(),
            "lock" => fs::write(root.join("Cargo.lock"), []).unwrap(),
            _ => unreachable!(),
        }
        assert!(
            read_owned_tree(&root, HardLimits::V1).is_err(),
            "accepted {change}"
        );
    }
}

#[test]
fn owned_reader_rejects_symlinks_in_every_root_component_and_leaf() {
    let (_temporary, root) = fixture();
    let outside = tempfile::tempdir().unwrap();
    let alias = outside.path().canonicalize().unwrap().join("alias");
    symlink(&root, &alias).unwrap();
    assert!(read_owned_tree(&alias, HardLimits::V1).is_err());
    assert!(read_owned_tree(&alias.join("src"), HardLimits::V1).is_err());
    fs::remove_file(root.join("src/lib.rs")).unwrap();
    symlink("missing", root.join("src/lib.rs")).unwrap();
    assert!(read_owned_tree(&root, HardLimits::V1).is_err());
}

#[test]
fn owned_reader_rejects_non_utf8_names_special_entries_and_sparse_marker() {
    for kind in ["utf8", "fifo", "marker"] {
        let (_temporary, root) = fixture();
        match kind {
            "utf8" => {
                let result = fs::write(root.join(std::ffi::OsStr::from_bytes(b"invalid-\xff")), []);
                if let Err(error) = result {
                    assert_eq!(
                        error.raw_os_error(),
                        Some(rustix::io::Errno::ILSEQ.raw_os_error())
                    );
                    eprintln!(
                        "filesystem refuses non-UTF-8 names before reader; Linux CI exercises this read case"
                    );
                    continue;
                }
            }
            "fifo" => assert!(
                std::process::Command::new("mkfifo")
                    .arg(root.join("fifo"))
                    .status()
                    .unwrap()
                    .success()
            ),
            "marker" => fs::OpenOptions::new()
                .write(true)
                .open(root.join(GENERATED_MARKER))
                .unwrap()
                .set_len(crate::MAX_MARKER_BYTES as u64 + 1)
                .unwrap(),
            _ => unreachable!(),
        }
        assert!(read_owned_tree(&root, HardLimits::V1).is_err(), "{kind}");
    }
}

#[test]
fn owned_reader_retains_root_identity_after_a_pathname_swap() {
    let (_temporary, root) = fixture();
    let parent = tempfile::tempdir().unwrap();
    let moved = parent.path().join("retained");
    let descriptor = crate::output_read::root(&root).unwrap();
    fs::rename(&root, &moved).unwrap();
    fs::create_dir(&root).unwrap();
    fs::write(root.join("human"), b"untouched").unwrap();
    let files = crate::output_fs::read_tree_from_handle(descriptor, HardLimits::V1).unwrap();
    crate::validate_owned_files(&files, HardLimits::V1).unwrap();
    assert_eq!(files[Path::new("src/lib.rs")], b"// SDK bytes\n");
    assert_eq!(fs::read(root.join("human")).unwrap(), b"untouched");
}

#[test]
fn owned_reader_detects_same_size_change_growth_and_file_pathname_swap() {
    for change in ["same-size", "growth", "swap"] {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("value");
        fs::write(&path, b"old!").unwrap();
        let file = fs::File::open(&path).unwrap();
        let before = crate::output_read::metadata(&file).unwrap();
        match change {
            "same-size" => fs::write(&path, b"new!").unwrap(),
            "growth" => fs::write(&path, b"old!more").unwrap(),
            "swap" => {
                fs::remove_file(&path).unwrap();
                symlink("never-read", &path).unwrap();
            }
            _ => unreachable!(),
        }
        let result = crate::output_read::bounded(file, &before, 4);
        // Unlink changes ctime on the retained inode. Rejection never opens the new pathname.
        assert!(result.is_err(), "{change}");
    }
}

#[test]
fn owned_reader_wide_tree_uses_only_depth_bounded_directory_handles() {
    if let Some(root) = std::env::var_os("MUSUBI_OWNERSHIP_WIDE_ROOT") {
        let mut limit = rustix::process::getrlimit(rustix::process::Resource::Nofile);
        limit.current = Some(64);
        rustix::process::setrlimit(rustix::process::Resource::Nofile, limit).unwrap();
        let tree = read_owned_tree(Path::new(&root), HardLimits::V1).unwrap();
        assert_eq!(tree.file_entries().count(), 513);
        return;
    }
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let mut tree = GeneratedTree::default();
    for index in 0..512 {
        tree.insert(format!("owner_{index}/nested/value"), [])
            .unwrap();
    }
    tree.seal().unwrap();
    for (path, bytes) in tree.file_entries() {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "output_ownership_tests::owned_reader_wide_tree_uses_only_depth_bounded_directory_handles"])
        .env("MUSUBI_OWNERSHIP_WIDE_ROOT", root).status().unwrap();
    assert!(status.success());
}
