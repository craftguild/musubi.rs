//! Closed schema/vector and generated owned EIP-712 API tests.

#[cfg(unix)]
use crate::output_test_support::{CargoValidation, generate};

use std::{fs, path::Path};

#[cfg(unix)]
use crate::{
    CARGO_PROCESS_LIMITS, ProcessRequest, build_snapshot, parse_snapshot, run_bounded_process,
    snapshot_json,
};
use crate::{build_ir, load_inputs, render};

fn write_fixture(root: &Path) {
    fs::write(
        root.join("musubi.toml"),
        r#"schema_version = "musubi/config-v4"

[runtime]
source = "path"
root = "../.."

[generation]
output = "sdk"

[package]
name = "generated-safe-hash-sdk"
version = "0.1.0"
edition = "2024"

[[typed_data]]
id = "safe_hash"
rust_module = "safe_hash"
authority = "closed-schema-v1"
schema = "safe.schema.json"
vectors = "safe.vectors.json"
"#,
    )
    .unwrap();
    fs::write(
        root.join("safe.schema.json"),
        r#"{
          "schema_version":"musubi/typed-data-schema-v1",
          "types":[
            {"name":"EIP712Domain","fields":[
              {"name":"chainId","type":"uint256"},
              {"name":"verifyingContract","type":"address"}
            ]},
            {"name":"SafeTx","fields":[
              {"name":"to","type":"address"},
              {"name":"value","type":"uint256"},
              {"name":"data","type":"bytes"},
              {"name":"operation","type":"uint8"},
              {"name":"safeTxGas","type":"uint256"},
              {"name":"baseGas","type":"uint256"},
              {"name":"gasPrice","type":"uint256"},
              {"name":"gasToken","type":"address"},
              {"name":"refundReceiver","type":"address"},
              {"name":"nonce","type":"uint256"}
            ]}
          ],
          "roots":[{"kind":"typed_data","type":"SafeTx","domain":"EIP712Domain"}]
        }"#,
    )
    .unwrap();
    fs::write(root.join("safe.vectors.json"), vector_bytes()).unwrap();
}

fn vector_bytes() -> &'static [u8] {
    br#"{
      "schema_version":"musubi/typed-data-vectors-v1",
      "evidence":{
        "solidity_source_path":"test/SafeHash.t.sol",
        "solidity_source_sha256":"0x0000000000000000000000000000000000000000000000000000000000000001",
        "artifact_target":"test/SafeHash.t.sol:SafeHashTest",
        "artifact_sha256":"0x0000000000000000000000000000000000000000000000000000000000000002",
        "compiler_version":"solc 0.8.26",
        "compiler_settings_sha256":"0x0000000000000000000000000000000000000000000000000000000000000003",
        "vector_command":"forge test --match-test testSafeHashVector"
      },
      "vectors":[{
        "root":"SafeTx",
        "domain_type":"EIP712Domain",
        "value":{
          "to":"0xabcdefabcdefabcdefabcdefabcdefabcdefabcd",
          "value":"0",
          "data":"0xdeadbeef",
          "operation":"0",
          "safeTxGas":"0",
          "baseGas":"0",
          "gasPrice":"0",
          "gasToken":"0x0000000000000000000000000000000000000000",
          "refundReceiver":"0x0000000000000000000000000000000000000000",
          "nonce":"42"
        },
        "domain":{
          "chainId":"11155111",
          "verifyingContract":"0x1234567890abcdef1234567890abcdef12345678"
        },
        "type_string":"SafeTx(address to,uint256 value,bytes data,uint8 operation,uint256 safeTxGas,uint256 baseGas,uint256 gasPrice,address gasToken,address refundReceiver,uint256 nonce)",
        "type_hash":"0xbb8310d486368db6bd6f849402fdd73ad53d316b5a4b2644ad6efe0f941286d8",
        "struct_hash":"0xded7dce09e5387a4fa1b08a0e258e2aedd1ea2ecd52c874693d7a522ee05b152",
        "domain_type_string":"EIP712Domain(uint256 chainId,address verifyingContract)",
        "domain_type_hash":"0x47e79534a245952e8b16893a336b85a3d9ea9fa8c573f3d803afb92a79469218",
        "domain_separator":"0x5f9246b4dfb61b8c1856ebf915f25401aaa441edd3e618dfa685abb7bc11c9ff",
        "digest":"0x1f0cb86e5d37170b03e06c8ea3dc8faa9d3c5d13a2480c6db788ac6276e34de3"
      }]
    }"#
}

#[test]
#[cfg(unix)]
fn safe_vector_generates_owned_api_and_release_evidence() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let root = tempfile::tempdir_in(workspace.join("target")).unwrap();
    write_fixture(root.path());
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    let ir = build_ir(inputs).unwrap();
    let snapshot = parse_snapshot(
        &snapshot_json(&build_snapshot(&ir, &crate::test_generator::info())).unwrap(),
    )
    .unwrap();
    assert_eq!(snapshot.typed_data[0].id, "safe_hash");
    let tree = render(&ir, &crate::test_generator::info()).unwrap();
    assert_eq!(tree, render(&ir, &crate::test_generator::info()).unwrap());
    let files = tree
        .files()
        .map(|file| (file.path, file.contents))
        .collect::<std::collections::BTreeMap<_, _>>();
    let source = crate::output_test_support::rust_sources(&tree, "src/safe_hash/types");
    assert!(source.contains("pub struct SafeTx"));
    assert!(source.contains("pub fn digest(&self, domain: &EIP712Domain)"));
    assert!(source.contains("pub const TYPE_STRING"));
    assert!(!source.contains("alloy"));
    let release: serde_json::Value =
        serde_json::from_slice(&files[Path::new("musubi-release.json")]).unwrap();
    assert_eq!(release["typed_data"][0]["id"], "safe_hash");
    assert_eq!(
        release["typed_data"][0]["roots"][0]["type_hash"],
        "0xbb8310d486368db6bd6f849402fdd73ad53d316b5a4b2644ad6efe0f941286d8"
    );
    assert!(
        release["typed_data"][0]["evidence"]
            .get("solidity_source_path")
            .is_none()
    );
    assert!(
        release["typed_data"][0]["evidence"]
            .get("vector_command")
            .is_none()
    );

    let output = root.path().join("sdk");
    generate(&tree, &output, CargoValidation::Required).unwrap();
    let private = crate::output_test_support::private_copy(&tree, root.path());
    let target = crate::output_test_support::cargo_target();
    let commands: &[&[&str]] = &[
        &["check", "--quiet"],
        &["test", "--quiet"],
        &["clippy", "--all-targets", "--", "-D", "warnings"],
        &["rustdoc", "--lib", "--", "-D", "warnings"],
    ];
    for arguments in commands {
        let result = run_bounded_process(
            ProcessRequest {
                program: "cargo",
                arguments,
                current_dir: &private,
                environment: &[("CARGO_TARGET_DIR", target.as_path())],
            },
            CARGO_PROCESS_LIMITS,
        )
        .unwrap();
        assert!(
            result.status.success(),
            "generated typed-data command {arguments:?} failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    assert!(
        crate::output_test_support::check(&tree, &output)
            .unwrap()
            .is_clean()
    );
}

#[test]
fn vectors_fail_closed_on_hash_field_order_and_domain_drift() {
    let mutations = [
        (
            "/vectors/0/digest",
            serde_json::Value::String(format!("0x{}", "00".repeat(32))),
        ),
        (
            "/vectors/0/domain/chainId",
            serde_json::Value::String("1".into()),
        ),
        (
            "/vectors/0/type_string",
            serde_json::Value::String("SafeTx(uint256 value,address to)".into()),
        ),
        (
            "/vectors/0/value/operation",
            serde_json::Value::String("256".into()),
        ),
        (
            "/vectors/0/value/to",
            serde_json::Value::String("0xABCDEFabcdefabcdefabcdefabcdefabcdefabcd".into()),
        ),
        (
            "/vectors/0/value/data",
            serde_json::Value::String("0x0".into()),
        ),
        (
            "/vectors/0/value",
            serde_json::json!({"to":"0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"}),
        ),
        (
            "/vectors/0/type_hash",
            serde_json::Value::String(
                "0xBB8310D486368DB6BD6F849402FDD73AD53D316B5A4B2644AD6EFE0F941286D8".into(),
            ),
        ),
        (
            "/vectors/0/struct_hash",
            serde_json::Value::String(
                "ded7dce09e5387a4fa1b08a0e258e2aedd1ea2ecd52c874693d7a522ee05b152".into(),
            ),
        ),
        (
            "/evidence/artifact_sha256",
            serde_json::Value::String(format!("0x{}", "00".repeat(32))),
        ),
        (
            "/evidence/artifact_target",
            serde_json::Value::String("/private/build/Message.json:Message".into()),
        ),
        (
            "/evidence/solidity_source_path",
            serde_json::Value::String(r"C:\Users\secret\Hash.sol".into()),
        ),
        (
            "/evidence/compiler_version",
            serde_json::Value::String("/private/toolchains/solc\nsecret".into()),
        ),
    ];
    for (pointer, replacement) in mutations {
        let root = tempfile::tempdir().unwrap();
        write_fixture(root.path());
        let mut vectors: serde_json::Value = serde_json::from_slice(vector_bytes()).unwrap();
        *vectors.pointer_mut(pointer).unwrap() = replacement;
        fs::write(
            root.path().join("safe.vectors.json"),
            serde_json::to_vec(&vectors).unwrap(),
        )
        .unwrap();
        let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
        assert!(
            build_ir(inputs).is_err(),
            "vector drift accepted at {pointer}"
        );
    }
}

#[test]
fn vectors_reject_duplicate_fields_recursively() {
    let root = tempfile::tempdir().unwrap();
    write_fixture(root.path());
    let duplicated = String::from_utf8(vector_bytes().to_vec())
        .unwrap()
        .replacen(
            "\"operation\":\"0\",",
            "\"operation\":\"1\",\n          \"operation\":\"0\",",
            1,
        );
    fs::write(root.path().join("safe.vectors.json"), duplicated).unwrap();
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    assert!(build_ir(inputs).is_err());
}

#[test]
fn schema_rejects_duplicate_fields_recursively() {
    let root = tempfile::tempdir().unwrap();
    write_fixture(root.path());
    let schema_path = root.path().join("safe.schema.json");
    let duplicated = fs::read_to_string(&schema_path).unwrap().replacen(
        "\"name\":\"operation\",",
        "\"name\":\"shadow\",\n              \"name\":\"operation\",",
        1,
    );
    fs::write(schema_path, duplicated).unwrap();
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    assert!(build_ir(inputs).is_err());
}

#[test]
fn schema_rejects_ambiguous_widths_cycles_and_field_order_drift() {
    let mutations = [
        ("/types/1/name", "Box"),
        ("/types/1/fields/1/type", "uint"),
        ("/types/1/fields/1/type", "uint08"),
        ("/types/1/fields/1/type", "uint7"),
        ("/types/1/fields/1/type", "uint256[0]"),
        ("/types/1/fields/0/type", "SafeTx"),
    ];
    for (pointer, replacement) in mutations {
        let root = tempfile::tempdir().unwrap();
        write_fixture(root.path());
        let mut schema: serde_json::Value =
            serde_json::from_slice(&fs::read(root.path().join("safe.schema.json")).unwrap())
                .unwrap();
        *schema.pointer_mut(pointer).unwrap() = replacement.into();
        fs::write(
            root.path().join("safe.schema.json"),
            serde_json::to_vec(&schema).unwrap(),
        )
        .unwrap();
        assert!(build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).is_err());
    }

    let root = tempfile::tempdir().unwrap();
    write_fixture(root.path());
    let schema_path = root.path().join("safe.schema.json");
    let mut schema: serde_json::Value =
        serde_json::from_slice(&fs::read(&schema_path).unwrap()).unwrap();
    schema["types"][1]["fields"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    fs::write(schema_path, serde_json::to_vec(&schema).unwrap()).unwrap();
    assert!(build_ir(load_inputs(&root.path().join("musubi.toml")).unwrap()).is_err());
}

#[test]
fn schema_rejects_named_struct_chains_beyond_the_nesting_limit() {
    let root = tempfile::tempdir().unwrap();
    write_fixture(root.path());
    let mut types = vec![serde_json::json!({
        "name": "Node0",
        "fields": [{"name": "value", "type": "uint256"}]
    })];
    types.extend((1..=65).map(|index| {
        serde_json::json!({
            "name": format!("Node{index}"),
            "fields": [{"name": "child", "type": format!("Node{}", index - 1)}]
        })
    }));
    let schema = serde_json::json!({
        "schema_version": "musubi/typed-data-schema-v1",
        "types": types,
        "roots": [{"kind": "struct_hash", "type": "Node65"}]
    });
    fs::write(
        root.path().join("safe.schema.json"),
        serde_json::to_vec(&schema).unwrap(),
    )
    .unwrap();
    let inputs = load_inputs(&root.path().join("musubi.toml")).unwrap();
    let error = build_ir(inputs).unwrap_err();
    assert!(error.diagnostics.iter().any(|diagnostic| {
        diagnostic.message == "typed-data graph is invalid or too deeply nested"
    }));
}

#[test]
fn release_fixture_generates_with_typed_data_evidence() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let manifest = workspace.join("release-fixture/musubi.toml");
    let ir = build_ir(load_inputs(&manifest).unwrap()).unwrap();
    let tree = render(&ir, &crate::test_generator::info()).unwrap();
    let release = tree
        .files()
        .find(|file| file.path == Path::new("musubi-release.json"))
        .unwrap();
    let release: serde_json::Value = serde_json::from_slice(&release.contents).unwrap();
    assert_eq!(release["contracts"][0]["kind"], "deployable");
    assert_eq!(release["typed_data"][0]["id"], "release_safe_v141_hash");

    crate::validate_generated_cargo(
        &tree,
        &workspace.join("release-fixture/sdk"),
        &workspace.join("release-fixture"),
    )
    .unwrap();
}
