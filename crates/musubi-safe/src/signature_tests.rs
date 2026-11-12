//! Focused Safe 1.4.1 signature packing and malformed-layout tests.

use musubi_core::{Address, Bytes, Hash};

use crate::{EoaRecoveryId, SafeV141Signature, SignatureError, pack_signatures};

use crate::signature::validate_packed_signature_layout;

fn address(last: u8) -> Address {
    let mut bytes = [0_u8; 20];
    bytes[19] = last;
    Address::new(bytes)
}

#[test]
fn packer_sorts_owners_and_builds_canonical_contract_tail() {
    let packed = pack_signatures(
        vec![
            SafeV141Signature::Contract {
                owner: address(2),
                signature: Bytes::from(b"abc".as_slice()),
            },
            SafeV141Signature::ApprovedHash { owner: address(1) },
        ],
        2,
    )
    .unwrap();
    let bytes = packed.as_bytes();
    assert_eq!(packed.signature_count(), 2);
    assert_eq!(&bytes[12..32], address(1).as_ref());
    assert_eq!(bytes[64], 1);
    assert_eq!(&bytes[65 + 12..65 + 32], address(2).as_ref());
    assert_eq!(bytes[65 + 63], 130);
    assert_eq!(bytes[65 + 64], 0);
    assert_eq!(bytes[130 + 31], 3);
    assert_eq!(&bytes[162..165], b"abc");
    assert_eq!(bytes.len(), 194);
}

#[test]
fn packer_sorts_reversed_approved_and_contract_owners() {
    for signatures in [
        vec![
            SafeV141Signature::ApprovedHash { owner: address(2) },
            SafeV141Signature::ApprovedHash { owner: address(1) },
        ],
        vec![
            SafeV141Signature::Contract {
                owner: address(2),
                signature: Bytes::from(b"two".as_slice()),
            },
            SafeV141Signature::Contract {
                owner: address(1),
                signature: Bytes::from(b"one".as_slice()),
            },
        ],
    ] {
        let packed = pack_signatures(signatures, 2).unwrap();
        assert_eq!(&packed.as_bytes()[12..32], address(1).as_ref());
        assert_eq!(&packed.as_bytes()[65 + 12..65 + 32], address(2).as_ref());
    }
}

#[test]
fn eoa_markers_distinguish_eip712_and_eth_sign() {
    let packed = pack_signatures(
        vec![
            SafeV141Signature::EthSign {
                owner: address(2),
                r: Hash::new([2; 32]),
                s: Hash::new([3; 32]),
                recovery_id: EoaRecoveryId::V28,
            },
            SafeV141Signature::Eip712 {
                owner: address(1),
                r: Hash::new([4; 32]),
                s: Hash::new([5; 32]),
                recovery_id: EoaRecoveryId::V27,
            },
        ],
        2,
    )
    .unwrap();
    assert_eq!(packed.as_bytes()[64], 27);
    assert_eq!(packed.as_bytes()[129], 32);
}

#[test]
fn contract_and_approved_hash_static_words_are_canonical() {
    let contract = pack_signatures(
        vec![SafeV141Signature::Contract {
            owner: address(1),
            signature: Bytes::from(b"a".as_slice()),
        }],
        1,
    )
    .unwrap();
    let approved = pack_signatures(
        vec![SafeV141Signature::ApprovedHash { owner: address(1) }],
        1,
    )
    .unwrap();
    for mut bytes in [contract.as_bytes().to_vec(), approved.as_bytes().to_vec()] {
        bytes[0] = 1;
        assert_eq!(
            validate_packed_signature_layout(&bytes, 1),
            Err(SignatureError::NonCanonicalStaticRecord { index: 0 })
        );
    }

    let mut ignored_s = approved.as_bytes().to_vec();
    ignored_s[63] = 1;
    assert_eq!(
        validate_packed_signature_layout(&ignored_s, 1),
        Err(SignatureError::NonCanonicalStaticRecord { index: 0 })
    );
}

#[test]
fn excessive_static_signature_count_fails_before_duplicate_scanning() {
    let signatures = vec![
        SafeV141Signature::ApprovedHash { owner: address(1) };
        crate::signature::MAX_SAFE_SIGNATURE_BYTES / 65 + 1
    ];
    let count = signatures.len();
    assert_eq!(
        pack_signatures(signatures, count),
        Err(SignatureError::BlobTooLarge)
    );
}

#[test]
fn threshold_duplicates_and_dynamic_bounds_fail_closed() {
    assert_eq!(
        pack_signatures(Vec::new(), 0),
        Err(SignatureError::ZeroThreshold)
    );
    assert_eq!(
        validate_packed_signature_layout(&[], 0),
        Err(SignatureError::ZeroThreshold)
    );
    assert_eq!(
        pack_signatures(
            vec![SafeV141Signature::ApprovedHash { owner: address(1) }],
            2
        ),
        Err(SignatureError::ThresholdMismatch {
            expected: 2,
            actual: 1,
        })
    );
    assert_eq!(
        pack_signatures(
            vec![
                SafeV141Signature::ApprovedHash { owner: address(1) },
                SafeV141Signature::ApprovedHash { owner: address(1) },
            ],
            2,
        ),
        Err(SignatureError::DuplicateOwner { owner: address(1) })
    );
    for signatures in [
        vec![
            SafeV141Signature::Contract {
                owner: address(1),
                signature: Bytes::from(b"one".as_slice()),
            },
            SafeV141Signature::Contract {
                owner: address(1),
                signature: Bytes::from(b"two".as_slice()),
            },
        ],
        vec![
            SafeV141Signature::ApprovedHash { owner: address(1) },
            SafeV141Signature::Contract {
                owner: address(1),
                signature: Bytes::from(b"contract".as_slice()),
            },
        ],
        vec![
            SafeV141Signature::Eip712 {
                owner: address(1),
                r: Hash::new([2; 32]),
                s: Hash::new([3; 32]),
                recovery_id: EoaRecoveryId::V27,
            },
            SafeV141Signature::ApprovedHash { owner: address(1) },
        ],
    ] {
        assert_eq!(
            pack_signatures(signatures, 2),
            Err(SignatureError::DuplicateOwner { owner: address(1) })
        );
    }

    let valid = pack_signatures(
        vec![SafeV141Signature::Contract {
            owner: address(1),
            signature: Bytes::from(b"a".as_slice()),
        }],
        1,
    )
    .unwrap();
    let mut invalid_offset = valid.as_bytes().to_vec();
    invalid_offset[63] = 64;
    assert_eq!(
        validate_packed_signature_layout(&invalid_offset, 1),
        Err(SignatureError::InvalidContractOffset { index: 0 })
    );
    let mut invalid_padding = valid.as_bytes().to_vec();
    *invalid_padding.last_mut().unwrap() = 1;
    assert_eq!(
        validate_packed_signature_layout(&invalid_padding, 1),
        Err(SignatureError::NonCanonicalPadding { index: 0 })
    );
    let mut unsupported = vec![0_u8; 65];
    unsupported[64] = 2;
    assert_eq!(
        validate_packed_signature_layout(&unsupported, 1),
        Err(SignatureError::UnsupportedMarker {
            index: 0,
            marker: 2,
        })
    );
}
