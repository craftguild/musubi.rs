//! Exact library coverage and immutable-masked runtime verification tests.

use musubi_core::{
    Address, BytecodeRange, Bytes, CreationBytecode, ImmutableReference, LinkReference,
    RuntimeBytecode, TransactionRequest,
};

use crate::deployment::validate_deployment_request;
use crate::{DeploymentError, LibraryAddresses, link_creation, verify_runtime};

fn link() -> LinkReference {
    LinkReference {
        source: "Math.sol".into(),
        library: "Math".into(),
        ranges: vec![BytecodeRange {
            start: 1,
            length: 20,
        }],
    }
}

#[test]
fn deployment_execution_accepts_only_contract_creation_requests() {
    assert!(validate_deployment_request(&TransactionRequest::default()).is_ok());
    assert_eq!(
        validate_deployment_request(&TransactionRequest {
            to: Some(Address::new([0x11; 20])),
            ..TransactionRequest::default()
        }),
        Err(DeploymentError::TransactionKind)
    );
}

#[test]
fn linking_requires_exact_source_qualified_coverage() {
    let creation = CreationBytecode::new(Bytes::new([0; 22]), vec![link()]).unwrap();
    assert_eq!(
        link_creation(&creation, &LibraryAddresses::new()),
        Err(DeploymentError::LibraryCoverage)
    );
    let mut libraries = LibraryAddresses::new();
    libraries.insert("Math.sol", "Math", Address::new([0x11; 20]));
    let linked = link_creation(&creation, &libraries).unwrap();
    assert_eq!(&linked.as_ref()[1..21], &[0x11; 20]);

    libraries.insert("Extra.sol", "Extra", Address::new([0x22; 20]));
    assert_eq!(
        link_creation(&creation, &libraries),
        Err(DeploymentError::LibraryCoverage)
    );
}

#[test]
fn runtime_verification_masks_only_declared_immutables() {
    let runtime = RuntimeBytecode::new(
        Bytes::new([1, 2, 3, 4]),
        Vec::new(),
        vec![ImmutableReference {
            id: "7".into(),
            ranges: vec![BytecodeRange {
                start: 1,
                length: 2,
            }],
        }],
    )
    .unwrap();
    let report = verify_runtime(&runtime, &LibraryAddresses::new(), &[1, 9, 9, 4]).unwrap();
    assert!(report.matches);
    let mismatch = verify_runtime(&runtime, &LibraryAddresses::new(), &[8, 9, 9, 4]).unwrap();
    assert_eq!(mismatch.first_mismatch, Some(0));
}

#[test]
fn runtime_rejects_public_structs_with_overlapping_library_slots() {
    let creation = CreationBytecode {
        template: Bytes::new([0; 30]),
        links: vec![
            link(),
            LinkReference {
                source: "Other.sol".into(),
                library: "Other".into(),
                ranges: vec![BytecodeRange {
                    start: 10,
                    length: 20,
                }],
            },
        ],
    };
    let mut libraries = LibraryAddresses::new();
    libraries.insert("Math.sol", "Math", Address::new([0x11; 20]));
    libraries.insert("Other.sol", "Other", Address::new([0x22; 20]));
    assert_eq!(
        link_creation(&creation, &libraries),
        Err(DeploymentError::Reference)
    );
}

#[test]
fn runtime_rejects_link_slots_overlapped_by_immutable_masks() {
    let runtime = RuntimeBytecode {
        template: Bytes::new([0; 22]),
        links: vec![link()],
        immutables: vec![ImmutableReference {
            id: "7".into(),
            ranges: vec![BytecodeRange {
                start: 1,
                length: 20,
            }],
        }],
    };
    let mut libraries = LibraryAddresses::new();
    libraries.insert("Math.sol", "Math", Address::new([0x11; 20]));
    let mut forged = [0_u8; 22];
    forged[1..21].copy_from_slice(&[0x22; 20]);
    assert_eq!(
        verify_runtime(&runtime, &libraries, &forged),
        Err(DeploymentError::Reference)
    );
}
