//! Adversarial tests for bounded reads through retained input handles.

use std::{fs, io::Write as _, os::unix::fs::symlink};

use tempfile::TempDir;

use super::*;

fn root(temp: &TempDir) -> InputRoot {
    fs::write(temp.path().join("musubi.toml"), b"manifest").unwrap();
    open_manifest(&temp.path().join("musubi.toml")).unwrap().0
}

#[test]
fn pathname_swap_after_open_reads_the_opened_file() {
    let temp = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let input = temp.path().join("input.json");
    fs::write(&input, b"inside").unwrap();
    fs::write(outside.path().join("input.json"), b"outside").unwrap();
    let root = root(&temp);
    let mut total = 0;
    let bytes =
        read_relative_with_hook(&root, "input.json", 32, &mut total, HardLimits::V1, || {
            fs::remove_file(&input).unwrap();
            symlink(outside.path().join("input.json"), &input).unwrap();
        })
        .unwrap();
    assert_eq!(bytes, b"inside");
}

#[test]
fn growth_after_metadata_check_is_bounded_on_the_same_handle() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.json");
    fs::write(&input, b"1234").unwrap();
    let root = root(&temp);
    let mut total = 0;
    let error = read_relative_with_hook(&root, "input.json", 4, &mut total, HardLimits::V1, || {
        let mut file = fs::OpenOptions::new().append(true).open(&input).unwrap();
        file.write_all(b"5").unwrap();
    })
    .unwrap_err();
    assert_eq!(
        error.diagnostics.iter().next().unwrap().code,
        DiagnosticCode::LimitExceeded
    );
}

#[test]
fn fifo_is_rejected_without_blocking() {
    let temp = TempDir::new().unwrap();
    let root = root(&temp);
    assert!(
        std::process::Command::new("mkfifo")
            .arg(temp.path().join("input.json"))
            .status()
            .unwrap()
            .success()
    );
    let mut total = 0;
    let error = read_relative(&root, "input.json", 32, &mut total, HardLimits::V1).unwrap_err();
    assert_eq!(
        error.diagnostics.iter().next().unwrap().code,
        DiagnosticCode::UnsafePath
    );
}

#[test]
fn noncanonical_relative_spellings_are_rejected_before_open() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("a")).unwrap();
    fs::write(temp.path().join("a/input.json"), b"input").unwrap();
    fs::write(temp.path().join("input.json"), b"input").unwrap();
    let root = root(&temp);
    for path in [
        "",
        "/input.json",
        "./input.json",
        "a/./input.json",
        "a/../input.json",
        "a//input.json",
        "input.json/",
        "a\\input.json",
    ] {
        let mut total = 0;
        let error = read_relative(&root, path, 32, &mut total, HardLimits::V1).unwrap_err();
        assert_eq!(
            error.diagnostics.iter().next().unwrap().code,
            DiagnosticCode::UnsafePath,
            "accepted {path:?}"
        );
    }
}
