//! Retained external Solidity evidence for the dynamic struct-array EIP-712 vector.

use std::{fs, path::PathBuf};

use musubi_core::{keccak256, sha256};

#[test]
fn retained_array_artifact_binds_the_reviewed_source_and_vector_bundle() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../external-acceptance/typed-data-array-v1");
    let source = fs::read(fixture.join("solidity/test/TypedDataArrayReference.t.sol")).unwrap();
    let artifact_bytes = fs::read(
        fixture.join("solidity/out/TypedDataArrayReference.t.sol/TypedDataArrayReferenceTest.json"),
    )
    .unwrap();
    let artifact: serde_json::Value = serde_json::from_slice(&artifact_bytes).unwrap();
    let vectors: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.join("vectors.json")).unwrap()).unwrap();

    assert_eq!(
        artifact["metadata"]["sources"]["test/TypedDataArrayReference.t.sol"]["keccak256"],
        format!("0x{}", hex::encode(keccak256(&source).as_ref()))
    );
    assert_eq!(
        vectors["evidence"]["solidity_source_sha256"],
        format!("0x{}", hex::encode(sha256(&source).as_ref()))
    );
    assert_eq!(
        vectors["evidence"]["artifact_sha256"],
        format!("0x{}", hex::encode(sha256(&artifact_bytes).as_ref()))
    );
    let mut settings = serde_json::to_vec(&canonical(&artifact["metadata"]["settings"])).unwrap();
    settings.push(b'\n');
    assert_eq!(
        vectors["evidence"]["compiler_settings_sha256"],
        format!("0x{}", hex::encode(sha256(&settings).as_ref()))
    );
    assert_eq!(
        artifact["metadata"]["compiler"]["version"],
        vectors["evidence"]["compiler_version"]
    );
    assert_eq!(
        artifact["metadata"]["settings"]["compilationTarget"]["test/TypedDataArrayReference.t.sol"],
        "TypedDataArrayReferenceTest"
    );
}

fn canonical(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), canonical(value)))
                .collect(),
        ),
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(canonical).collect())
        }
        value => value.clone(),
    }
}
