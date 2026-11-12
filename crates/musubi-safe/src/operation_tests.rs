//! Focused Safe hash, request encoding, and reviewed-registry tests.

use musubi_core::{Address, Bytes, Hash, U256};

use crate::{
    SMART_ACCOUNT_REVISION, SMART_ACCOUNT_SAFE_SOURCE_SHA256, SafeDomain, SafeOperation,
    SafeTransaction, SafeV141, SafeV141Signature, approve_hash_request, exec_transaction_request,
    get_transaction_hash_call, pack_signatures, safe_v141_registry,
};

#[test]
fn operation_selectors_and_dynamic_offsets_are_canonical() {
    let safe = Address::new([0x11; 20]);
    let hash = Hash::new([0x22; 32]);
    let approve = approve_hash_request(safe, hash).unwrap();
    assert_eq!(&approve.input.as_ref()[..4], &[0xd4, 0xd9, 0xbd, 0xcd]);
    assert_eq!(&approve.input.as_ref()[4..], hash.as_ref());

    let transaction = SafeTransaction {
        to: Address::new([0x33; 20]),
        data: Bytes::from([1_u8, 2, 3].as_slice()),
        nonce: U256::from_u128(7).unwrap(),
        ..SafeTransaction::default()
    };
    let signatures = pack_signatures(
        vec![SafeV141Signature::ApprovedHash {
            owner: Address::new([0x44; 20]),
        }],
        1,
    )
    .unwrap();
    let execute = exec_transaction_request(safe, &transaction, &signatures).unwrap();
    assert_eq!(&execute.input.as_ref()[..4], &[0x6a, 0x76, 0x12, 0x02]);
    assert_eq!(
        &execute.input.as_ref()[4 + 2 * 32 + 30..4 + 3 * 32],
        &[1, 64]
    );
    assert_eq!(
        &execute.input.as_ref()[4 + 9 * 32 + 30..4 + 10 * 32],
        &[1, 128]
    );

    let hash_call = get_transaction_hash_call(safe, &transaction).unwrap();
    assert_eq!(&hash_call.input.as_ref()[..4], &[0xd8, 0xd1, 0x1f, 0x78]);
    assert_eq!(hash_call.to, Some(safe));
}

#[test]
fn reviewed_registry_matches_pinned_canonical_identities() {
    let registry = safe_v141_registry().unwrap();
    assert_eq!(registry.safe_version, "1.4.1");
    assert_eq!(registry.smart_account_revision, SMART_ACCOUNT_REVISION);
    assert_eq!(
        registry.smart_account_safe_source_sha256,
        SMART_ACCOUNT_SAFE_SOURCE_SHA256
    );
    assert_eq!(
        registry.contracts.singleton.address.to_string(),
        "0x41675c099f32341bf84bfc5382af534df5c7461a"
    );
    assert_eq!(
        registry.contracts.singleton.runtime_code_hash,
        serde_json::from_str::<Hash>(
            "\"0x1fe2df852ba3299d6534ef416eefa406e56ced995bca886ab7a553e6d0c5e1c4\"",
        )
        .unwrap()
    );
    assert_eq!(
        registry.contracts.proxy_factory.address.to_string(),
        "0x4e1dcf7ad4e460cfd30791ccc4f9c8a4f820ec67"
    );
}

#[test]
fn transaction_hash_is_sensitive_to_domain_nonce_and_operation() {
    let domain = SafeDomain {
        chain_id: U256::from_u128(1).unwrap(),
        verifying_contract: Address::new([0x55; 20]),
    };
    let transaction = SafeTransaction {
        to: Address::new([0x66; 20]),
        data: Bytes::from(b"payload".as_slice()),
        ..SafeTransaction::default()
    };
    let base = SafeV141::transaction_hash(domain, &transaction);
    let mut changed = transaction.clone();
    changed.nonce = U256::from_u128(1).unwrap();
    assert_ne!(base, SafeV141::transaction_hash(domain, &changed));
    changed = transaction.clone();
    changed.operation = SafeOperation::DelegateCall;
    assert_ne!(base, SafeV141::transaction_hash(domain, &changed));
    assert_ne!(
        base,
        SafeV141::transaction_hash(
            SafeDomain {
                chain_id: U256::from_u128(2).unwrap(),
                ..domain
            },
            &transaction,
        )
    );
    assert_ne!(
        base,
        SafeV141::transaction_hash(
            SafeDomain {
                verifying_contract: Address::new([0x56; 20]),
                ..domain
            },
            &transaction,
        )
    );
}

#[test]
fn transaction_hash_matches_actual_safe_v141_golden_vector() {
    let bundle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../safe-v141-sdk-input/safe-v1.4.1.typed-data-vectors.json"
    ))
    .unwrap();
    let vector = &bundle["vectors"][0];
    let domain = SafeDomain {
        chain_id: U256::from_u128(
            vector["domain"]["chainId"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
        )
        .unwrap(),
        verifying_contract: vector["domain"]["verifyingContract"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
    };
    let transaction = SafeTransaction {
        to: vector["value"]["to"].as_str().unwrap().parse().unwrap(),
        data: Bytes::from_hex(vector["value"]["data"].as_str().unwrap()).unwrap(),
        operation: {
            assert_eq!(vector["value"]["operation"], "0");
            SafeOperation::Call
        },
        nonce: U256::from_u128(vector["value"]["nonce"].as_str().unwrap().parse().unwrap())
            .unwrap(),
        ..SafeTransaction::default()
    };
    let expected: Hash = serde_json::from_value(vector["digest"].clone()).unwrap();
    assert_eq!(SafeV141::transaction_hash(domain, &transaction), expected);
}
