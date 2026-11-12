//! Retained external Solidity evidence for atomic and fixed-array EIP-712 edges.

use std::{fs, path::PathBuf};

use musubi_core::{keccak256, sha256};

#[test]
fn retained_edge_artifact_binds_the_reviewed_source_and_vector_bundle() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../external-acceptance/typed-data-edge-v1");
    let source = fs::read(fixture.join("solidity/test/TypedDataEdgeReference.t.sol")).unwrap();
    let artifact_bytes = fs::read(
        fixture.join("solidity/out/TypedDataEdgeReference.t.sol/TypedDataEdgeReferenceTest.json"),
    )
    .unwrap();
    let artifact: serde_json::Value = serde_json::from_slice(&artifact_bytes).unwrap();
    let vectors: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.join("vectors.json")).unwrap()).unwrap();

    assert_eq!(
        artifact["metadata"]["sources"]["test/TypedDataEdgeReference.t.sol"]["keccak256"],
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
    assert_eq!(
        artifact["metadata"]["compiler"]["version"],
        vectors["evidence"]["compiler_version"]
    );
    assert_eq!(
        artifact["metadata"]["settings"]["compilationTarget"]["test/TypedDataEdgeReference.t.sol"],
        "TypedDataEdgeReferenceTest"
    );
}
