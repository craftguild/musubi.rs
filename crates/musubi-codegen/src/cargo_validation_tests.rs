//! Ordinary Cargo configuration/cache observations without reopening configured SDK source.

use std::{fs, os::unix::fs::PermissionsExt as _, path::Path, process::Command};

use crate::{GeneratedTree, cargo_validation::validate_in_parent};

#[test]
fn private_validation_preserves_cargo_configuration_and_reuses_cache() {
    if let Ok(mode) = std::env::var("MUSUBI_CARGO_TEST_CHILD") {
        observe_validation(&mode);
        return;
    }
    // Test-only child environments avoid mutating this multithreaded test process's environment.
    for mode in ["configured", "environment", "default"] {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", "cargo_validation_tests::private_validation_preserves_cargo_configuration_and_reuses_cache", "--nocapture"])
            .env("MUSUBI_CARGO_TEST_CHILD", mode)
            .env_remove("CARGO_TARGET_DIR")
            .env_remove("RUSTC_WRAPPER")
            .env_remove("RUSTC_WORKSPACE_WRAPPER");
        if mode == "environment" {
            command.env(
                "CARGO_TARGET_DIR",
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../target/cargo-validation-cache")
                    .canonicalize()
                    .unwrap(),
            );
        }
        let status = command.status().unwrap();
        assert!(status.success(), "{mode} Cargo environment failed");
    }
}

fn observe_validation(mode: &str) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let root = tempfile::tempdir_in(workspace.join("target")).unwrap();
    let temporary_parent = tempfile::tempdir().unwrap();
    let (log, target) = prepare_configuration(root.path(), &workspace, mode);
    // Configuration beside private source is never the trusted lookup root.
    fs::create_dir(temporary_parent.path().join(".cargo")).unwrap();
    fs::write(
        temporary_parent.path().join(".cargo/config.toml"),
        "invalid TOML [[[",
    )
    .unwrap();
    let consumer_lock = root.path().join("Cargo.lock");
    fs::write(&consumer_lock, "consumer-owned lockfile\n").unwrap();
    let mut tree = GeneratedTree::default();
    tree.insert(
        "Cargo.toml",
        b"[package]\nname='cargo-validation-fixture'\nversion='0.0.0'\nedition='2024'\n\
        [workspace]\n[dependencies]\n\
        musubi-core={path='../../../../crates/musubi-core'}\n\
        musubi-runtime={path='../../../../crates/musubi-runtime'}\n"
            .to_vec(),
    )
    .unwrap();
    tree.insert("src/lib.rs", b"pub fn valid() {}\n".to_vec())
        .unwrap();
    tree.seal().unwrap();
    let conceptual_root = root.path().join("nested/sdk");
    for iteration in 0..2 {
        fs::write(&log, "").unwrap();
        validate_in_parent(
            &tree,
            &conceptual_root,
            root.path(),
            temporary_parent.path(),
        )
        .unwrap();
        let observations = fs::read_to_string(&log).unwrap();
        assert!(
            !observations.is_empty(),
            "trusted wrapper/config was not used"
        );
        assert!(!observations.contains(conceptual_root.to_str().unwrap()));
        assert!(observations.lines().any(|line| line.starts_with("source=")
            && line.contains(temporary_parent.path().to_str().unwrap())));
        if iteration == 1 {
            assert!(!observations.contains("--crate-name musubi_core"));
            assert!(!observations.contains("--crate-name musubi_runtime"));
        }
        assert_eq!(
            fs::read_to_string(&consumer_lock).unwrap(),
            "consumer-owned lockfile\n"
        );
        assert!(!conceptual_root.exists());
    }
    if mode == "configured" {
        assert!(target.join("debug/deps").is_dir());
    }
    // A failed compilation still cannot install or mutate the conceptual SDK.
    // Replace the source through a fresh rendered fixture to model a real renderer failure.
    let mut broken = GeneratedTree::default();
    for file in tree.files() {
        let bytes = if file.path == Path::new("src/lib.rs") {
            b"compile_error!(\"rejected fixture\");\n".to_vec()
        } else {
            file.contents
        };
        broken.insert(file.path, bytes).unwrap();
    }
    broken.seal().unwrap();
    assert!(
        validate_in_parent(
            &broken,
            &conceptual_root,
            root.path(),
            temporary_parent.path()
        )
        .is_err()
    );
    assert!(!conceptual_root.exists());
    assert_eq!(
        fs::read_to_string(consumer_lock).unwrap(),
        "consumer-owned lockfile\n"
    );
}

#[test]
fn cache_paths_cannot_mutate_the_configured_sdk_even_through_aliases() {
    use crate::cargo_validation::require_external_cache;
    let root = tempfile::tempdir().unwrap();
    let sdk = root.path().join("sdk");
    assert!(require_external_cache(root.path(), &sdk, root.path()).is_err());
    for cache in [
        sdk.clone(),
        sdk.join("target"),
        root.path().join("alias/../sdk/target"),
    ] {
        assert!(require_external_cache(&cache, &sdk, root.path()).is_err());
    }
    assert!(!sdk.exists());
    fs::create_dir(&sdk).unwrap();
    let alias = root.path().join("alias");
    std::os::unix::fs::symlink(&sdk, &alias).unwrap();
    assert!(require_external_cache(&alias.join("target"), &sdk, root.path()).is_err());
    assert!(require_external_cache(&root.path().join("cache"), &sdk, root.path()).is_ok());
    assert_eq!(sdk.read_dir().unwrap().count(), 0);
}

#[allow(
    clippy::unnecessary_debug_formatting,
    reason = "TOML paths require quoted and escaped string literals"
)]
fn prepare_configuration(
    root: &Path,
    workspace: &Path,
    mode: &str,
) -> (std::path::PathBuf, std::path::PathBuf) {
    let config = root.join(".cargo");
    fs::create_dir(&config).unwrap();
    let wrapper = root.join("wrapper.sh");
    let log = root.join("observations");
    fs::write(
        &wrapper,
        r#"#!/bin/sh
set -eu
test "$MUSUBI_CARGO_PROBE" = ordinary-config
printf 'source=%s\nargs=%s\n' "${CARGO_MANIFEST_DIR:-}" "$*" >> "$MUSUBI_CARGO_LOG"
exec "$@"
"#,
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    let target = workspace.join("target/cargo-validation-cache");
    let target_setting = if mode == "configured" {
        format!("target-dir = {target:?}\n")
    } else {
        String::new()
    };
    fs::write(config.join("config.toml"), format!(
        "[build]\nrustc-wrapper = {wrapper:?}\n{target_setting}\n[env]\nMUSUBI_CARGO_PROBE = 'ordinary-config'\nMUSUBI_CARGO_LOG = {log:?}\n"
    )).unwrap();
    (log, target)
}

#[test]
fn effective_cargo_home_is_checked_before_any_cargo_command() {
    if let Some(root) = std::env::var_os("MUSUBI_CARGO_HOME_TEST_ROOT") {
        let root = Path::new(&root);
        let mut tree = GeneratedTree::default();
        tree.insert(
            "Cargo.toml",
            b"[package]\nname='home-fixture'\nversion='0.0.0'\n\
            [workspace]\n[dependencies]\nmusubi-core={version='0.1'}\n\
            musubi-runtime={version='0.1'}\n"
                .to_vec(),
        )
        .unwrap();
        tree.insert("src/lib.rs", b"pub fn valid() {}\n".to_vec())
            .unwrap();
        tree.seal().unwrap();
        assert_eq!(
            crate::validate_generated_cargo(&tree, &root.join("sdk"), root),
            Err(crate::CargoValidationFailure::Preparation)
        );
        assert!(!root.join("sdk").exists());
        return;
    }
    for explicit in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "cargo_validation_tests::effective_cargo_home_is_checked_before_any_cargo_command",
            ])
            .env("MUSUBI_CARGO_HOME_TEST_ROOT", root.path())
            .env("CARGO_TARGET_DIR", root.path().join("external-cache"))
            .env_remove("CARGO_HOME");
        if explicit {
            command.env("CARGO_HOME", root.path().join("sdk/cache"));
        } else {
            command.env("HOME", root.path().join("sdk"));
        }
        assert!(command.status().unwrap().success());
        assert!(!root.path().join("sdk").exists());
    }
}
