//! Retained external Solidity evidence for the `RebalanceExecuted` emitted log.

use std::{fs, path::PathBuf};

use musubi_core::{keccak256, sha256};

#[test]
fn retained_event_artifact_binds_the_emitter_source_and_vector() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../external-acceptance/event-rebalance-v1");
    let source = fs::read(fixture.join("solidity/test/RebalanceEventReference.t.sol")).unwrap();
    let artifact_bytes = fs::read(
        fixture.join("solidity/out/RebalanceEventReference.t.sol/RebalanceEventReferenceTest.json"),
    )
    .unwrap();
    let artifact: serde_json::Value = serde_json::from_slice(&artifact_bytes).unwrap();
    let vector: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.join("vector.json")).unwrap()).unwrap();

    assert_eq!(
        artifact["metadata"]["sources"]["test/RebalanceEventReference.t.sol"]["keccak256"],
        format!("0x{}", hex::encode(keccak256(&source).as_ref()))
    );
    assert_eq!(
        vector["evidence"]["solidity_source_sha256"],
        format!("0x{}", hex::encode(sha256(&source).as_ref()))
    );
    assert_eq!(
        vector["evidence"]["artifact_sha256"],
        format!("0x{}", hex::encode(sha256(&artifact_bytes).as_ref()))
    );
    let mut settings = serde_json::to_vec(&canonical(&artifact["metadata"]["settings"])).unwrap();
    settings.push(b'\n');
    assert_eq!(
        vector["evidence"]["compiler_settings_sha256"],
        format!("0x{}", hex::encode(sha256(&settings).as_ref()))
    );
    assert_eq!(
        artifact["metadata"]["compiler"]["version"],
        vector["evidence"]["compiler_version"]
    );
    assert_eq!(
        artifact["metadata"]["settings"]["compilationTarget"]["test/RebalanceEventReference.t.sol"],
        "RebalanceEventReferenceTest"
    );
    assert_eq!(
        vector["event"]["topics"][0],
        format!(
            "0x{}",
            hex::encode(keccak256(
                vector["event"]["signature"].as_str().unwrap().as_bytes()
            ))
        )
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
