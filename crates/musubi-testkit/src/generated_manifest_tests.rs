//! Closed current-schema generated release validation and exact SSH identity tests.

use crate::{generated_test_fixture::materialize, inspect_generated_crate};
use std::fs;

#[test]
fn generated_release_manifest_rejects_unknown_nested_fields() {
    let root = crate::generated_test_fixture::tempdir();
    fs::create_dir(root.path().join("src")).unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        b"[package]\nname='x'\nversion='0.1.0'\nedition='2024'\n\
          rust-version='1.95'\npublish=false\n\n[workspace]\n\n[dependencies]\n\
          musubi-core={path='../crates/musubi-core'}\n\
          musubi-runtime={path='../crates/musubi-runtime'}\n",
    )
    .unwrap();
    fs::write(root.path().join("src/lib.rs"), b"//! fixture\n").unwrap();
    let base = serde_json::json!({
        "schema_version":"musubi/release-v4",
        "typed_data": [],
        "manifest_schema": "musubi/config-v4",
        "manifest_sha256": "0x7777777777777777777777777777777777777777777777777777777777777777",
        "package":{"name":"x","version":"0.1.0"},
        "generator": {"schema_version": "musubi/generator-info-v1", "version": "0.1.0", "revision": "unknown", "git_commit": null, "dirty": null, "build": {"rustc": "unknown", "host": "test-host", "target": "test-target", "profile": "test"}},
        "generated_abi_version": 2,
        "runtime":{"source":"path","root":".."},
        "contracts":[{
            "id":"vault",
            "rust_module": "vault",
            "kind": "deployable",
            "canonical_abi_sha256":"33".repeat(32),
            "raw_abi_sha256":"11".repeat(32),
            "creation_template_keccak256":"22".repeat(32),
            "runtime_template_keccak256":null,
            "external_artifact_coherence":"unverified"
        }]
    });
    for pointer in ["/package", "/runtime", "/contracts/0"] {
        let mut mutated = base.clone();
        mutated
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown".into(), serde_json::Value::Bool(true));
        fs::write(
            root.path().join("musubi-release.json"),
            serde_json::to_vec(&mutated).unwrap(),
        )
        .unwrap();
        crate::generated_test_fixture::reseal(root.path());
        assert!(inspect_generated_crate(root.path()).is_err());
    }
}

#[test]
fn generated_release_manifest_accepts_only_canonical_ssh_identity() {
    let root = crate::generated_test_fixture::tempdir();
    fs::write(
        root.path().join("Cargo.toml"),
        b"[package]\nname='x'\nversion='0.1.0'\nedition='2024'\n\
          rust-version='1.95'\npublish=false\n\n[workspace]\n\n[dependencies]\n\
          musubi-core={git='ssh://git@github.com/craftguild/musubi.rs.git',rev='87553a40b014754de7991878c8e994490bad5939',package='musubi-core'}\n\
          musubi-runtime={git='ssh://git@github.com/craftguild/musubi.rs.git',rev='87553a40b014754de7991878c8e994490bad5939',package='musubi-runtime'}\n",
    )
    .unwrap();
    let mut release = serde_json::json!({
        "schema_version":"musubi/release-v4",
        "typed_data": [],
        "manifest_schema": "musubi/config-v4",
        "manifest_sha256": "0x7777777777777777777777777777777777777777777777777777777777777777",
        "package":{"name":"x","version":"0.1.0"},
        "generator": {"schema_version": "musubi/generator-info-v1", "version": "0.1.0", "revision": "unknown", "git_commit": null, "dirty": null, "build": {"rustc": "unknown", "host": "test-host", "target": "test-target", "profile": "test"}},
        "generated_abi_version": 2,
        "runtime":{
            "source":"git",
            "url":"ssh://git@github.com/craftguild/musubi.rs.git",
            "revision":"87553a40b014754de7991878c8e994490bad5939"
        },
        "contracts":[{
            "id":"vault",
            "rust_module": "vault",
            "kind": "deployable",
            "canonical_abi_sha256":"33".repeat(32),
            "raw_abi_sha256":"11".repeat(32),
            "creation_template_keccak256":"22".repeat(32),
            "runtime_template_keccak256":null,
            "external_artifact_coherence":"unverified"
        }]
    });
    materialize(root.path(), &mut release);
    assert!(inspect_generated_crate(root.path()).is_ok());

    release["runtime"]["url"] =
        serde_json::Value::String("git@github.com:craftguild/musubi.rs.git".into());
    fs::write(
        root.path().join("musubi-release.json"),
        serde_json::to_vec(&release).unwrap(),
    )
    .unwrap();
    crate::generated_test_fixture::reseal(root.path());
    assert!(inspect_generated_crate(root.path()).is_err());
}
