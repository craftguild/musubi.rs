//! Fail-closed tests for typed-data release evidence text.

use std::fs;

use serde_json::{Value, json};

use musubi_core::keccak256;

use crate::{generated_test_fixture::materialize, inspect_generated_crate};

pub(crate) fn fixture_root(release: &mut Value) -> tempfile::TempDir {
    let root = crate::generated_test_fixture::tempdir();
    fs::write(root.path().join("Cargo.toml"), development_manifest("x")).unwrap();
    materialize(root.path(), release);
    root
}

fn development_manifest(name: &str) -> String {
    format!(
        "[package]\nname={name:?}\nversion='0.1.0'\nedition='2024'\n\
         rust-version='1.95'\npublish=false\n\n[workspace]\n\n[dependencies]\n\
         musubi-core={{path='../crates/musubi-core'}}\n\
         musubi-runtime={{path='../crates/musubi-runtime'}}\n"
    )
}

pub(crate) fn valid_release() -> Value {
    let type_string = "Message(uint256 value)";
    let domain_type_string = "EIP712Domain(uint256 chainId)";
    json!({
        "schema_version": "musubi/release-v4",
        "manifest_schema": "musubi/config-v4",
        "manifest_sha256": "0x7777777777777777777777777777777777777777777777777777777777777777",
        "package": {"name": "x", "version": "0.1.0"},
        "generator": {"schema_version": "musubi/generator-info-v1", "version": "0.1.0", "revision": "unknown", "git_commit": null, "dirty": null, "build": {"rustc": "unknown", "host": "test-host", "target": "test-target", "profile": "test"}},
        "generated_abi_version": 2,
        "runtime": {"source": "path", "root": ".."},
        "contracts": [],
        "typed_data": [{
            "id": "message",
            "rust_module": "message",
            "schema_sha256": format!("0x{}", "11".repeat(32)),
            "vectors_sha256": format!("0x{}", "22".repeat(32)),
            "roots": [{
                "primary_type": "Message",
                "type_string": type_string,
                "type_hash": type_hash(type_string),
                "domain_type": "EIP712Domain",
                "domain_type_string": domain_type_string,
                "domain_type_hash": type_hash(domain_type_string)
            }],
            "evidence": {
                "solidity_source_sha256": format!("0x{}", "55".repeat(32)),
                "artifact_target": "contracts/Message.sol:Message",
                "artifact_sha256": format!("0x{}", "66".repeat(32)),
                "compiler_version": "0.8.30+commit.73712a01",
                "compiler_settings_sha256": format!("0x{}", "77".repeat(32)),
                "vector_command_sha256": format!("0x{}", "88".repeat(32))
            }
        }]
    })
}

fn type_hash(type_string: &str) -> String {
    format!("0x{}", hex::encode(keccak256(type_string.as_bytes())))
}

#[test]
fn typed_data_release_hashes_are_prefixed_lowercase_and_nonzero() {
    let mut valid = valid_release();
    let root = fixture_root(&mut valid);
    let release_path = root.path().join("musubi-release.json");
    assert!(inspect_generated_crate(root.path()).is_ok());

    for (pointer, invalid) in [
        (
            "/typed_data/0/schema_sha256",
            format!("0x{}", "00".repeat(32)),
        ),
        ("/typed_data/0/vectors_sha256", "22".repeat(32)),
        (
            "/typed_data/0/roots/0/type_hash",
            format!("0x{}", "AA".repeat(32)),
        ),
        (
            "/typed_data/0/roots/0/domain_type_hash",
            format!("0x{}", "00".repeat(32)),
        ),
        (
            "/typed_data/0/evidence/artifact_sha256",
            format!("0x{}", "00".repeat(32)),
        ),
        (
            "/typed_data/0/evidence/artifact_target",
            "/private/out.json:Message".into(),
        ),
        (
            "/typed_data/0/evidence/compiler_version",
            "/private/solc\nsecret".into(),
        ),
    ] {
        let mut mutated = valid.clone();
        *mutated.pointer_mut(pointer).unwrap() = Value::String(invalid);
        fs::write(&release_path, serde_json::to_vec(&mutated).unwrap()).unwrap();
        crate::generated_test_fixture::reseal(root.path());
        assert!(
            inspect_generated_crate(root.path()).is_err(),
            "accepted {pointer}"
        );
    }
}

#[test]
fn typed_data_root_names_and_hashes_are_self_consistent() {
    let valid = valid_release();
    let mut fixture = valid.clone();
    let root = fixture_root(&mut fixture);
    let release_path = root.path().join("musubi-release.json");

    let mut mismatched_hash = valid.clone();
    mismatched_hash["typed_data"][0]["roots"][0]["type_hash"] =
        Value::String(format!("0x{}", "33".repeat(32)));
    assert_rejected(&release_path, root.path(), &mismatched_hash);

    let mut invalid_name = valid.clone();
    invalid_name["typed_data"][0]["roots"][0]["primary_type"] = json!("9Message");
    assert_rejected(&release_path, root.path(), &invalid_name);

    let mut wrong_definition = valid.clone();
    let shadow = "Shadow(uint256 value)";
    wrong_definition["typed_data"][0]["roots"][0]["type_string"] = json!(shadow);
    wrong_definition["typed_data"][0]["roots"][0]["type_hash"] = json!(type_hash(shadow));
    assert_rejected(&release_path, root.path(), &wrong_definition);

    for malformed in [
        "Message(",
        "Message()trailing",
        "Message(uint value)",
        "Message(uint256 value)Unused(uint256 value)",
    ] {
        let mut malformed_release = valid.clone();
        malformed_release["typed_data"][0]["roots"][0]["type_string"] = json!(malformed);
        malformed_release["typed_data"][0]["roots"][0]["type_hash"] = json!(type_hash(malformed));
        assert_rejected(&release_path, root.path(), &malformed_release);
    }

    let mut invalid_domain = valid.clone();
    invalid_domain["typed_data"][0]["roots"][0]["domain_type"] = json!("Domain.Name");
    assert_rejected(&release_path, root.path(), &invalid_domain);

    let mut mismatched_domain_hash = valid;
    mismatched_domain_hash["typed_data"][0]["roots"][0]["domain_type_hash"] =
        Value::String(format!("0x{}", "44".repeat(32)));
    assert_rejected(&release_path, root.path(), &mismatched_domain_hash);
}

fn assert_rejected(release_path: &std::path::Path, root: &std::path::Path, release: &Value) {
    fs::write(release_path, serde_json::to_vec(release).unwrap()).unwrap();
    crate::generated_test_fixture::reseal(root);
    assert!(inspect_generated_crate(root).is_err());
}

#[test]
fn interface_release_rejects_storage_layout_provenance() {
    let mut release = valid_release();
    release["contracts"] = json!([{
        "id": "status",
        "rust_module": "status",
        "kind": "interface",
        "canonical_abi_sha256": "11".repeat(32),
        "raw_abi_sha256": "22".repeat(32),
        "creation_template_keccak256": null,
        "runtime_template_keccak256": null,
        "ast_sha256": null,
        "metadata_sha256": null,
        "storage_layout_sha256": null,
        "external_artifact_coherence": "unverified"
    }]);
    let root = fixture_root(&mut release);
    let release_path = root.path().join("musubi-release.json");
    let canonical_hash = release["contracts"][0]["canonical_abi_sha256"].clone();
    assert!(inspect_generated_crate(root.path()).is_ok());

    release["contracts"][0]["canonical_abi_sha256"] = Value::String("00".repeat(32));
    fs::write(&release_path, serde_json::to_vec(&release).unwrap()).unwrap();
    crate::generated_test_fixture::reseal(root.path());
    assert!(inspect_generated_crate(root.path()).is_err());

    release["contracts"][0]["canonical_abi_sha256"] = canonical_hash;
    release["contracts"][0]["storage_layout_sha256"] = Value::String("33".repeat(32));
    fs::write(&release_path, serde_json::to_vec(&release).unwrap()).unwrap();
    crate::generated_test_fixture::reseal(root.path());
    assert!(inspect_generated_crate(root.path()).is_err());
}

#[test]
fn release_rejects_duplicate_nested_json_fields() {
    let mut valid = valid_release();
    let root = fixture_root(&mut valid);
    let release = serde_json::to_string(&valid).unwrap().replacen(
        "\"primary_type\":\"Message\"",
        "\"primary_type\":\"Shadow\",\"primary_type\":\"Message\"",
        1,
    );
    fs::write(root.path().join("musubi-release.json"), release).unwrap();
    crate::generated_test_fixture::reseal(root.path());
    assert!(inspect_generated_crate(root.path()).is_err());
}

#[test]
fn release_rejects_oversized_marker_before_loading_it() {
    let mut valid = valid_release();
    let root = fixture_root(&mut valid);
    fs::write(
        root.path().join(".musubi-generated"),
        vec![b'x'; 1024 * 1024],
    )
    .unwrap();
    assert!(inspect_generated_crate(root.path()).is_err());
}

#[test]
fn release_identity_requires_cargo_name_and_semantic_versions() {
    let mut valid = valid_release();
    let root = fixture_root(&mut valid);
    let release_path = root.path().join("musubi-release.json");
    for (pointer, replacement) in [
        ("/package/name", "invalid package"),
        ("/package/version", "not-semver"),
        ("/generator/version", "not-semver"),
    ] {
        let mut mutated = valid.clone();
        *mutated.pointer_mut(pointer).unwrap() = Value::String(replacement.into());
        fs::write(&release_path, serde_json::to_vec(&mutated).unwrap()).unwrap();
        crate::generated_test_fixture::reseal(root.path());
        assert!(
            inspect_generated_crate(root.path()).is_err(),
            "accepted {pointer}"
        );
    }
}

#[test]
fn release_entries_and_roots_are_unique_and_canonically_ordered() {
    let mut valid = valid_release();
    let root = fixture_root(&mut valid);
    let release_path = root.path().join("musubi-release.json");

    let mut duplicate_typed_id = valid.clone();
    duplicate_typed_id["typed_data"] = json!([
        valid["typed_data"][0].clone(),
        valid["typed_data"][0].clone()
    ]);
    fs::write(
        &release_path,
        serde_json::to_vec(&duplicate_typed_id).unwrap(),
    )
    .unwrap();
    crate::generated_test_fixture::reseal(root.path());
    assert!(inspect_generated_crate(root.path()).is_err());

    let mut duplicate_module = valid.clone();
    let mut second = valid["typed_data"][0].clone();
    second["id"] = json!("second");
    duplicate_module["typed_data"] = json!([valid["typed_data"][0].clone(), second]);
    fs::write(
        &release_path,
        serde_json::to_vec(&duplicate_module).unwrap(),
    )
    .unwrap();
    crate::generated_test_fixture::reseal(root.path());
    assert!(inspect_generated_crate(root.path()).is_err());

    let mut canonical_typed = valid.clone();
    let mut second = valid["typed_data"][0].clone();
    second["id"] = json!("second");
    second["rust_module"] = json!("second");
    canonical_typed["typed_data"] = json!([valid["typed_data"][0].clone(), second]);
    materialize(root.path(), &mut canonical_typed);
    assert!(inspect_generated_crate(root.path()).is_ok());
    let mut unsorted_typed = canonical_typed;
    unsorted_typed["typed_data"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    fs::write(&release_path, serde_json::to_vec(&unsorted_typed).unwrap()).unwrap();
    crate::generated_test_fixture::reseal(root.path());
    assert!(inspect_generated_crate(root.path()).is_err());

    let contract = json!({
        "id": "alpha",
        "rust_module": "alpha",
        "kind": "interface",
        "canonical_abi_sha256": "11".repeat(32),
        "raw_abi_sha256": "22".repeat(32),
        "creation_template_keccak256": null,
        "runtime_template_keccak256": null,
        "ast_sha256": null,
        "metadata_sha256": null,
        "storage_layout_sha256": null,
        "external_artifact_coherence": "unverified"
    });
    let mut contract_release = valid.clone();
    let mut second_contract = contract.clone();
    second_contract["id"] = json!("beta");
    second_contract["rust_module"] = json!("beta");
    contract_release["contracts"] = json!([contract.clone(), second_contract]);
    materialize(root.path(), &mut contract_release);
    assert!(inspect_generated_crate(root.path()).is_ok());
    contract_release["contracts"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    fs::write(
        &release_path,
        serde_json::to_vec(&contract_release).unwrap(),
    )
    .unwrap();
    crate::generated_test_fixture::reseal(root.path());
    assert!(inspect_generated_crate(root.path()).is_err());
    contract_release["contracts"] = json!([contract.clone(), contract]);
    fs::write(
        &release_path,
        serde_json::to_vec(&contract_release).unwrap(),
    )
    .unwrap();
    crate::generated_test_fixture::reseal(root.path());
    assert!(inspect_generated_crate(root.path()).is_err());

    let mut duplicate_root = valid.clone();
    let root_value = valid["typed_data"][0]["roots"][0].clone();
    duplicate_root["typed_data"][0]["roots"] = json!([root_value.clone(), root_value]);
    fs::write(&release_path, serde_json::to_vec(&duplicate_root).unwrap()).unwrap();
    crate::generated_test_fixture::reseal(root.path());
    assert!(inspect_generated_crate(root.path()).is_err());

    let mut unsorted_roots = valid.clone();
    let mut alpha = valid["typed_data"][0]["roots"][0].clone();
    alpha["primary_type"] = json!("Alpha");
    unsorted_roots["typed_data"][0]["roots"] =
        json!([valid["typed_data"][0]["roots"][0].clone(), alpha]);
    fs::write(&release_path, serde_json::to_vec(&unsorted_roots).unwrap()).unwrap();
    crate::generated_test_fixture::reseal(root.path());
    assert!(inspect_generated_crate(root.path()).is_err());
}
