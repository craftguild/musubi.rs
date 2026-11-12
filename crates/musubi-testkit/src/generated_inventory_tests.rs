//! Adversarial release-to-tree binding tests.

use std::fs;

use serde_json::{Value, json};

use crate::{generated_test_fixture::materialize, inspect_generated_crate};

pub(crate) fn fixture() -> (tempfile::TempDir, Value) {
    let root = crate::generated_test_fixture::tempdir();
    fs::write(
        root.path().join("Cargo.toml"),
        b"[package]\nname='x'\nversion='0.1.0'\nedition='2024'\n\
          rust-version='1.95'\npublish=false\n\n[workspace]\n\n[dependencies]\n\
          musubi-core={path='../crates/musubi-core'}\n\
          musubi-runtime={path='../crates/musubi-runtime'}\n",
    )
    .unwrap();
    let mut release = json!({
        "schema_version": "musubi/release-v4",
        "manifest_schema": "musubi/config-v4",
        "manifest_sha256": "0x7777777777777777777777777777777777777777777777777777777777777777",
        "package": {"name": "x", "version": "0.1.0"},
        "generator": {"schema_version": "musubi/generator-info-v1", "version": "0.1.0", "revision": "unknown", "git_commit": null, "dirty": null, "build": {"rustc": "unknown", "host": "test-host", "target": "test-target", "profile": "test"}},
        "generated_abi_version": 2,
        "runtime": {"source": "path", "root": ".."},
        "contracts": [{
            "id": "vault",
            "rust_module": "vault_sdk",
            "kind": "deployable",
            "canonical_abi_sha256": "11".repeat(32),
            "raw_abi_sha256": "22".repeat(32),
            "creation_template_keccak256": "33".repeat(32),
            "runtime_template_keccak256": "44".repeat(32),
            "ast_sha256": null,
            "metadata_sha256": null,
            "storage_layout_sha256": null,
            "external_artifact_coherence": "unverified"
        }],
        "typed_data": []
    });
    materialize(root.path(), &mut release);
    (root, release)
}

#[test]
fn exact_inventory_rejects_extra_and_missing_generated_files() {
    let (root, _) = fixture();
    assert!(inspect_generated_crate(root.path()).is_ok());
    fs::create_dir(root.path().join("src/extra")).unwrap();
    fs::write(root.path().join("src/extra/mod.rs"), b"//! extra\n").unwrap();
    assert!(inspect_generated_crate(root.path()).is_err());

    let (root, _) = fixture();
    fs::remove_file(root.path().join("assets/vault/runtime.bin")).unwrap();
    assert!(inspect_generated_crate(root.path()).is_err());
}

#[test]
fn manifest_modules_root_and_assets_are_one_identity() {
    let (root, mut release) = fixture();
    release["contracts"][0]["rust_module"] = json!("different");
    fs::write(
        root.path().join("musubi-release.json"),
        serde_json::to_vec(&release).unwrap(),
    )
    .unwrap();
    assert!(inspect_generated_crate(root.path()).is_err());

    let (root, _) = fixture();
    fs::write(root.path().join("assets/vault/abi.json"), b"[ ]").unwrap();
    assert!(inspect_generated_crate(root.path()).is_err());

    let (root, _) = fixture();
    fs::write(root.path().join("assets/vault/creation.bin"), b"changed").unwrap();
    assert!(inspect_generated_crate(root.path()).is_err());
}

#[test]
fn cargo_check_source_rejects_unreviewed_file_inputs() {
    for injected in [
        "const LEAK: &[u8] = include_bytes!(\"/etc/passwd\");\n",
        "const LEAK: &str = include_str!(\"/etc/passwd\");\n",
        "const LEAK: &str = env!(\"HOME\");\n",
        "#[path = \"/etc/passwd\"] mod leak;\n",
        "#[cfg_attr(all(), path = \"/etc/passwd\")] mod leak;\n",
        "const LEAK: &str = concat!(include_str!(\"/etc/passwd\"));\n",
        "macro_rules! leak { () => { include_bytes!(\"/etc/passwd\") } }\n",
        "const LEAK: &[u8] = r#include_bytes!(\"/etc/passwd\");\n",
        "const LEAK: &str = r#include_str!(\"/etc/passwd\");\n",
        "const LEAK: &str = r#env!(\"HOME\");\n",
        "const LEAK: Option<&str> = r#option_env!(\"HOME\");\n",
        "#[r#path = \"/etc/passwd\"] mod leak;\n",
    ] {
        let (root, _) = fixture();
        let path = root.path().join("src/vault_sdk/types.rs");
        let mut source = fs::read_to_string(&path).unwrap();
        source.push_str(injected);
        fs::write(path, source).unwrap();
        crate::generated_test_fixture::reseal(root.path());
        assert!(
            inspect_generated_crate(root.path()).is_err(),
            "accepted {injected}"
        );
    }
}

#[test]
fn release_rejects_keyword_modules_and_control_character_ids() {
    for (pointer, value) in [
        ("/contracts/0/rust_module", "type"),
        ("/contracts/0/id", "vault\n#[path=\"/etc/passwd\"]"),
    ] {
        let (root, mut release) = fixture();
        *release.pointer_mut(pointer).unwrap() = json!(value);
        fs::write(
            root.path().join("musubi-release.json"),
            serde_json::to_vec(&release).unwrap(),
        )
        .unwrap();
        assert!(inspect_generated_crate(root.path()).is_err());
    }
}

#[test]
fn expected_asset_include_must_be_present_exactly_once() {
    let (root, _) = fixture();
    let path = root.path().join("src/vault_sdk/artifact.rs");
    let source = fs::read_to_string(&path).unwrap().replace(
        "../../assets/vault/runtime.bin",
        "../../assets/vault/abi.json",
    );
    fs::write(path, source).unwrap();
    crate::generated_test_fixture::reseal(root.path());
    assert!(inspect_generated_crate(root.path()).is_err());
}
