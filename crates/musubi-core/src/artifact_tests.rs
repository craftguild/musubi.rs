//! Compiler artifact identity and bytecode-reference validation tests.

use crate::{
    BytecodeRange, Bytes, CreationBytecode, ImmutableReference, LinkReference, RuntimeBytecode,
    ValueError, is_canonical_artifact_target, is_canonical_compiler_version,
    is_canonical_solidity_identifier,
};

fn link(start: usize) -> LinkReference {
    LinkReference {
        source: "Library.sol".into(),
        library: format!("Library{start}"),
        ranges: vec![BytecodeRange { start, length: 20 }],
    }
}

#[test]
fn rejects_invalid_and_overlapping_bytecode_references() {
    assert_eq!(
        CreationBytecode::new(Bytes::new(vec![0; 21]), vec![link(2)]),
        Err(ValueError::ReferenceOutOfBounds)
    );
    assert_eq!(
        CreationBytecode::new(Bytes::new(vec![0; 40]), vec![link(0), link(10)]),
        Err(ValueError::OverlappingReferences)
    );
    assert_eq!(
        RuntimeBytecode::new(
            Bytes::new(vec![0; 40]),
            vec![link(0)],
            vec![ImmutableReference {
                id: "0".into(),
                ranges: vec![BytecodeRange {
                    start: 10,
                    length: 20,
                }],
            }],
        ),
        Err(ValueError::OverlappingReferences)
    );
}

#[test]
fn rejects_empty_creation_bytecode() {
    assert_eq!(
        CreationBytecode::new(Bytes::default(), Vec::new()),
        Err(ValueError::EmptyCreationBytecode)
    );
}

#[test]
fn compiler_evidence_identifiers_are_closed_and_path_safe() {
    for valid in ["0.8.26", "solc 0.7.6+commit.7338295f"] {
        assert!(is_canonical_compiler_version(valid));
    }
    for invalid in [
        "01.8.26",
        "0.8",
        "0.8.26+commit.DEADBEEF",
        "/private/solc",
        "0.8.26\nsecret",
    ] {
        assert!(!is_canonical_compiler_version(invalid));
    }
    assert!(is_canonical_artifact_target("src/Hash.sol:Hash"));
    assert!(!is_canonical_artifact_target("../Hash.sol:Hash"));
    for valid in ["Message", "_Message", "$Message", "M0"] {
        assert!(is_canonical_solidity_identifier(valid));
    }
    for invalid in ["", "0Message", "Message.Name", "Message\n"] {
        assert!(!is_canonical_solidity_identifier(invalid));
    }
}
