//! Explicit Path generated-crate runtime source regressions.

use std::fs;

use serde_json::{Value, json};

use crate::{generated_test_fixture::materialize, inspect_generated_crate};

fn path_manifest(runtime_path: &str) -> String {
    format!(
        "[package]\nname='fixture'\nversion='0.1.0'\nedition='2024'\n\
         rust-version='1.95'\npublish=false\n\n[workspace]\n\n[dependencies]\n\
         musubi-core={{path='../crates/musubi-core'}}\n\
         musubi-runtime={{path={runtime_path:?}}}\n"
    )
}

fn path_release() -> Value {
    json!({
        "schema_version": "musubi/release-v4",
        "typed_data": [],
        "manifest_schema": "musubi/config-v4",
        "manifest_sha256": "0x7777777777777777777777777777777777777777777777777777777777777777",
        "package": {"name": "fixture", "version": "0.1.0"},
        "generator": {"schema_version": "musubi/generator-info-v1", "version": "0.1.0", "revision": "unknown", "git_commit": null, "dirty": null, "build": {"rustc": "unknown", "host": "test-host", "target": "test-target", "profile": "test"}},
        "generated_abi_version": 2,
        "runtime": {"source": "path", "root": ".."},
        "contracts": [{
            "id": "alpha",
            "rust_module": "alpha",
            "kind": "deployable",
            "canonical_abi_sha256": "11".repeat(32),
            "raw_abi_sha256": "22".repeat(32),
            "creation_template_keccak256": "33".repeat(32),
            "runtime_template_keccak256": null,
            "external_artifact_coherence": "unverified"
        }]
    })
}

#[test]
fn path_source_requires_fixed_core_and_runtime_under_the_same_root() {
    let root = crate::generated_test_fixture::tempdir();
    fs::write(
        root.path().join("Cargo.toml"),
        path_manifest("../crates/musubi-runtime"),
    )
    .unwrap();
    let mut release = path_release();
    materialize(root.path(), &mut release);
    assert!(inspect_generated_crate(root.path()).is_ok());

    fs::write(
        root.path().join("Cargo.toml"),
        path_manifest("../other-runtime"),
    )
    .unwrap();
    crate::generated_test_fixture::reseal(root.path());
    assert!(inspect_generated_crate(root.path()).is_err());
}
