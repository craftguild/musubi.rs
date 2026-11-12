//! End-to-end CLI coverage for snapshot, diff, JSON diagnostics, and warning policy.

use std::{fs, path::Path, process::Command};

#[test]
fn snapshot_is_deterministic_and_diff_reports_additive_surface() {
    let root = tempfile::tempdir().unwrap();
    write_fixture(root.path(), single_function_abi());
    let old = root.path().join("old.json");
    let same = root.path().join("same.json");
    run(root.path(), ["snapshot", "--out", old.to_str().unwrap()]);
    run(root.path(), ["snapshot", "--out", same.to_str().unwrap()]);
    assert_eq!(fs::read(&old).unwrap(), fs::read(&same).unwrap());

    fs::write(root.path().join("abi.json"), additive_abi()).unwrap();
    let new = root.path().join("new.json");
    run(root.path(), ["snapshot", "--out", new.to_str().unwrap()]);
    let output = Command::new(env!("CARGO_BIN_EXE_musubi"))
        .args([
            "--format",
            "json",
            "diff",
            old.to_str().unwrap(),
            new.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let categories = report["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|change| change["category"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(categories.contains(&"evm_wire_additive"));
    assert!(categories.contains(&"rust_source_additive"));
}

#[test]
fn deny_warnings_promotes_ast_degradation_but_not_information() {
    let root = tempfile::tempdir().unwrap();
    write_fixture(root.path(), single_function_abi());
    let output = Command::new(env!("CARGO_BIN_EXE_musubi"))
        .args([
            "--manifest",
            root.path().join("musubi.toml").to_str().unwrap(),
            "--format",
            "json",
            "--deny-warnings",
            "snapshot",
            "--out",
            root.path().join("denied.json").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let diagnostics: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    let codes = diagnostics["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|diagnostic| diagnostic["code"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"MSB-I001"));
    assert!(codes.contains(&"MSB-W001"));
}

fn run<const N: usize>(root: &Path, arguments: [&str; N]) {
    let status = Command::new(env!("CARGO_BIN_EXE_musubi"))
        .args(["--manifest", root.join("musubi.toml").to_str().unwrap()])
        .args(arguments)
        .status()
        .unwrap();
    assert!(status.success());
}

fn write_fixture(root: &Path, abi: &[u8]) {
    fs::write(root.join("abi.json"), abi).unwrap();
    fs::write(root.join("creation.json"), b"\"0x60006000\"").unwrap();
    fs::write(
        root.join("musubi.toml"),
        r#"schema_version = "musubi/config-v4"
[runtime]
source = "path"
root = "."

[package]
name = "cli-fixture-sdk"
version = "0.1.0"
edition = "2024"

[generation]
output = "sdk"

[[contracts]]
id = "vault"
kind = "deployable"
abi = "abi.json"
creation_bytecode = "creation.json"
"#,
    )
    .unwrap();
}

fn single_function_abi() -> &'static [u8] {
    br#"[{"type":"function","name":"owner","inputs":[],"outputs":[{"name":"","type":"address"}],"stateMutability":"view"}]"#
}

fn additive_abi() -> &'static [u8] {
    br#"[
      {"type":"function","name":"owner","inputs":[],"outputs":[{"name":"","type":"address"}],"stateMutability":"view"},
      {"type":"function","name":"paused","inputs":[],"outputs":[{"name":"","type":"bool"}],"stateMutability":"view"}
    ]"#
}
