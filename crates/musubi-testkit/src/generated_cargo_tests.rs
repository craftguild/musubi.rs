//! Ordinary Cargo preparation preserves a relocated snapshot's conceptual paths and source guard.

use std::{fs, path::Path};

use crate::{cargo_check_command, generated_test_fixture::materialize};

#[test]
fn cargo_command_rebases_a_guarded_snapshot_and_retains_ordinary_settings() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let conceptual = workspace.join("target/testkit-relocated/sdk");
    let copy = crate::generated_test_fixture::tempdir();
    let manifest = "[package]\nname='fixture'\nversion='0.1.0'\nedition='2024'\nrust-version='1.95'\npublish=false\n[workspace]\n[dependencies]\nmusubi-core={path='../../../crates/musubi-core'}\nmusubi-runtime={path='../../../crates/musubi-runtime'}\n";
    fs::write(copy.path().join("Cargo.toml"), manifest).unwrap();
    let mut release = serde_json::json!({
        "schema_version":"musubi/release-v4", "manifest_schema":"musubi/config-v4",
        "manifest_sha256":format!("0x{}", "77".repeat(32)),
        "package":{"name":"fixture","version":"0.1.0"},
        "generator": {"schema_version":"musubi/generator-info-v1", "version":"0.1.0", "revision":"unknown", "git_commit":null, "dirty":null, "build":{"rustc":"unknown", "host":"test-host", "target":"test-target", "profile":"test"}},
        "generated_abi_version":2, "runtime":{"source":"path","root":"../../.."},
        "typed_data":[], "contracts":[{"id":"api", "rust_module":"api", "kind":"interface", "canonical_abi_sha256":"11".repeat(32), "raw_abi_sha256":"22".repeat(32), "creation_template_keccak256":null, "runtime_template_keccak256":null, "external_artifact_coherence":"unverified"}]
    });
    materialize(copy.path(), &mut release);
    let mut prepared = cargo_check_command(copy.path(), &conceptual, &workspace).unwrap();
    fs::write(copy.path().join("Cargo.toml"), b"mutated original").unwrap();
    let command = prepared.command_mut();
    assert_eq!(command.get_current_dir().unwrap(), workspace);
    let args = command.get_args().collect::<Vec<_>>();
    let index = args
        .iter()
        .position(|value| *value == "--manifest-path")
        .unwrap();
    let private_manifest = Path::new(args[index + 1]);
    assert_ne!(private_manifest.parent().unwrap(), copy.path());
    let rebound: toml::Value =
        toml::from_str(&fs::read_to_string(private_manifest).unwrap()).unwrap();
    for name in ["musubi-core", "musubi-runtime"] {
        assert_eq!(
            Path::new(rebound["dependencies"][name]["path"].as_str().unwrap()),
            workspace.join("crates").join(name)
        );
    }
    assert!(
        private_manifest
            .parent()
            .unwrap()
            .join("Cargo.lock")
            .is_file()
    );
    assert!(!copy.path().join("Cargo.lock").exists());
    for (name, value) in command.get_envs() {
        assert!(value.is_some());
        assert_eq!(name, "CARGO_TARGET_DIR");
        assert!(!Path::new(value.unwrap()).starts_with(private_manifest.parent().unwrap()));
    }
    let private_lock = fs::read(private_manifest.parent().unwrap().join("Cargo.lock")).unwrap();
    let private_root = private_manifest.parent().unwrap().to_owned();
    drop(prepared);
    assert!(!private_root.exists());
    assert_eq!(
        fs::read(copy.path().join("Cargo.toml")).unwrap(),
        b"mutated original"
    );
    fs::write(copy.path().join("Cargo.toml"), manifest).unwrap();
    fs::write(copy.path().join("Cargo.lock"), &private_lock).unwrap();
    assert!(cargo_check_command(copy.path(), &conceptual, &workspace).is_err());
    assert_eq!(
        fs::read(copy.path().join("Cargo.lock")).unwrap(),
        private_lock
    );
    fs::remove_file(copy.path().join("Cargo.lock")).unwrap();
    let mut second = cargo_check_command(copy.path(), &conceptual, &workspace).unwrap();
    let arguments = second.command_mut().get_args().collect::<Vec<_>>();
    let manifest_index = arguments
        .iter()
        .position(|argument| *argument == "--manifest-path")
        .unwrap();
    let second_root = Path::new(arguments[manifest_index + 1])
        .parent()
        .unwrap()
        .to_owned();
    assert_eq!(
        fs::read(second_root.join("Cargo.lock")).unwrap(),
        private_lock
    );
    // A caller can make the private lock stale; --locked must still refuse implicit repair.
    let stale = b"version = 4\n";
    fs::write(second_root.join("Cargo.lock"), stale).unwrap();
    let failed = execute(&mut second);
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("--locked"));
    assert_eq!(fs::read(second_root.join("Cargo.lock")).unwrap(), stale);
    assert!(!copy.path().join("Cargo.lock").exists());
}

fn execute(prepared: &mut crate::GeneratedCargoCheck) -> musubi_codegen::ProcessOutput {
    let command = prepared.command_mut();
    let arguments = command
        .get_args()
        .map(|argument| argument.to_str().unwrap())
        .collect::<Vec<_>>();
    let environment = command
        .get_envs()
        .map(|(name, value)| (name.to_str().unwrap(), Path::new(value.unwrap())))
        .collect::<Vec<_>>();
    musubi_codegen::run_bounded_process(
        musubi_codegen::ProcessRequest {
            program: "cargo",
            arguments: &arguments,
            current_dir: command.get_current_dir().unwrap(),
            environment: &environment,
        },
        musubi_codegen::CARGO_PROCESS_LIMITS,
    )
    .unwrap()
}
