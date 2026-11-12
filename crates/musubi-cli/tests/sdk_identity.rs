//! Ordinary CLI Path generation and compiled SDK generator information equality.

use std::{fs, path::Path, process::Command};

use musubi_codegen::{CARGO_PROCESS_LIMITS, ProcessRequest, run_bounded_process};

const BINARY: &str = env!("CARGO_BIN_EXE_musubi");

fn cli(root: &Path, arguments: &[&str]) -> std::process::Output {
    let output = Command::new(BINARY)
        .current_dir(root)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn fixture(root: &Path) {
    fs::write(root.join("abi.json"), b"[]").unwrap();
    fs::write(
        root.join("musubi.toml"),
        r#"schema_version = "musubi/config-v4"
[runtime]
source = "path"
root = "../.."
[package]
name = "ordinary-identity-sdk"
version = "0.8.0"
edition = "2024"
[generation]
output = "nested/sdk"
[[contracts]]
id = "api"
kind = "interface"
abi = "abi.json"
"#,
    )
    .unwrap();
}

#[test]
fn ordinary_cli_preserves_identity_in_json_rust_and_all_lifecycle_operations() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let target = workspace.join("target");
    fs::create_dir_all(&target).unwrap();
    let first = tempfile::tempdir_in(&target).unwrap();
    let second = tempfile::tempdir_in(&target).unwrap();
    let version: serde_json::Value =
        serde_json::from_slice(&cli(first.path(), &["--format", "json", "--version"]).stdout)
            .unwrap();
    for root in [first.path(), second.path()] {
        fixture(root);
        cli(root, &["doctor"]);
        assert!(!root.join("nested").exists());
        cli(root, &["generate"]);
        cli(root, &["check"]);
    }
    let sdk = first.path().join("nested/sdk");
    for file in [
        "Cargo.toml",
        "src/lib.rs",
        "musubi-release.json",
        "src/api/mod.rs",
    ] {
        assert_eq!(
            fs::read(sdk.join(file)).unwrap(),
            fs::read(second.path().join("nested/sdk").join(file)).unwrap()
        );
    }
    let release_before = fs::read(sdk.join("musubi-release.json")).unwrap();
    let release: serde_json::Value = serde_json::from_slice(&release_before).unwrap();
    assert_eq!(release["generator"], version);
    assert_eq!(
        release["runtime"],
        serde_json::json!({"source":"path", "root":"../../../.."})
    );
    let consumer = first.path().join("consumer");
    fs::create_dir_all(consumer.join("src")).unwrap();
    fs::write(consumer.join("Cargo.toml"), "[package]\nname='identity-consumer'\nversion='0.1.0'\nedition='2024'\n[workspace]\n[dependencies]\nsdk={package='ordinary-identity-sdk',path='../nested/sdk'}\nserde_json='1'\n").unwrap();
    fs::write(consumer.join("src/main.rs"), "fn main() { println!(\"{}\", serde_json::to_string(sdk::generator()).unwrap()); assert_eq!(sdk::api::interface_artifact().generator, *sdk::generator()); }\n").unwrap();
    let manifest = consumer.join("Cargo.toml");
    let output = run_bounded_process(
        ProcessRequest {
            program: "cargo",
            arguments: &[
                "run",
                "--quiet",
                "--manifest-path",
                manifest.to_str().unwrap(),
            ],
            current_dir: &workspace,
            environment: &[("CARGO_TARGET_DIR", &target)],
        },
        CARGO_PROCESS_LIMITS,
    )
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        version
    );
    cli(first.path(), &["update"]);
    cli(first.path(), &["check"]);
    assert_eq!(
        fs::read(sdk.join("musubi-release.json")).unwrap(),
        release_before
    );
    assert!(!sdk.join("Cargo.lock").exists());
    assert!(!sdk.join("target").exists());
}
