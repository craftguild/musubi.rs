//! Conformance helper self-tests using a deterministic in-memory backend.

use std::{fs, sync::Arc};

use musubi_core::{
    AbiField, AbiType, AbiValue, Address, BlockId, Bytes, CallRequest, Hash, Log, LogFilter,
    Receipt, TransactionRequest, U256, keccak256,
};
use musubi_runtime::{BackendFuture, ReadBackend, TransactionExecutor};

use crate::{
    CodecVector, ReadBackendCase, TransactionExecutorCase, check_codec_vector, check_read_backend,
    check_transaction_executor, generated_test_fixture::materialize, inspect_generated_crate,
};

#[derive(Clone)]
struct FixtureBackend {
    reads: Arc<ReadBackendCase>,
    transaction: Arc<TransactionExecutorCase>,
}

impl ReadBackend for FixtureBackend {
    fn chain_id(&self) -> BackendFuture<'_, u64> {
        Box::pin(async { Ok(self.reads.chain_id) })
    }

    fn call(&self, request: CallRequest, block: BlockId) -> BackendFuture<'_, Bytes> {
        Box::pin(async move {
            assert_eq!(
                (request, block),
                (self.reads.call.0.clone(), self.reads.call.1.clone())
            );
            Ok(self.reads.call.2.clone())
        })
    }

    fn estimate_gas(&self, request: CallRequest, block: BlockId) -> BackendFuture<'_, u64> {
        Box::pin(async move {
            assert_eq!(
                (request, block),
                (
                    self.reads.estimate_gas.0.clone(),
                    self.reads.estimate_gas.1.clone()
                )
            );
            Ok(self.reads.estimate_gas.2)
        })
    }

    fn code(&self, address: Address, block: BlockId) -> BackendFuture<'_, Bytes> {
        Box::pin(async move {
            assert_eq!(
                (address, block),
                (self.reads.code.0, self.reads.code.1.clone())
            );
            Ok(self.reads.code.2.clone())
        })
    }

    fn storage(&self, address: Address, slot: U256, block: BlockId) -> BackendFuture<'_, Hash> {
        Box::pin(async move {
            assert_eq!(
                (address, slot, block),
                (
                    self.reads.storage.0,
                    self.reads.storage.1,
                    self.reads.storage.2.clone()
                )
            );
            Ok(self.reads.storage.3)
        })
    }

    fn logs(&self, filter: LogFilter) -> BackendFuture<'_, Vec<Log>> {
        Box::pin(async move {
            assert_eq!(filter, self.reads.logs.0);
            Ok(self.reads.logs.1.clone())
        })
    }

    fn receipt(&self, transaction: Hash) -> BackendFuture<'_, Option<Receipt>> {
        Box::pin(async move {
            assert_eq!(transaction, self.reads.receipt.0);
            Ok(self.reads.receipt.1.clone())
        })
    }
}

impl TransactionExecutor for FixtureBackend {
    fn fill_transaction(
        &self,
        request: TransactionRequest,
    ) -> BackendFuture<'_, TransactionRequest> {
        Box::pin(async move {
            assert_eq!(request, self.transaction.request);
            Ok(self.transaction.filled.clone())
        })
    }

    fn sign_and_broadcast(&self, request: TransactionRequest) -> BackendFuture<'_, Hash> {
        Box::pin(async move {
            assert_eq!(request, self.transaction.filled);
            Ok(self.transaction.transaction_hash)
        })
    }

    fn wait_for_receipt(&self, transaction: Hash) -> BackendFuture<'_, Receipt> {
        Box::pin(async move {
            assert_eq!(transaction, self.transaction.transaction_hash);
            Ok(self.transaction.receipt.clone())
        })
    }
}

#[test]
fn backend_and_executor_cases_cover_the_complete_spi() {
    let transaction_hash = Hash::new([9; 32]);
    let receipt = Receipt {
        transaction_hash,
        block_hash: Some(Hash::new([8; 32])),
        block_number: Some(7),
        contract_address: None,
        gas_used: 21_000,
        success: Some(true),
        logs: Vec::new(),
    };
    let call = CallRequest {
        to: Some(Address::new([1; 20])),
        ..CallRequest::default()
    };
    let reads = ReadBackendCase {
        chain_id: 1,
        call: (
            call.clone(),
            BlockId::Latest,
            Bytes::from(b"call".as_slice()),
        ),
        estimate_gas: (call, BlockId::Pending, 40_000),
        code: (
            Address::new([2; 20]),
            BlockId::Latest,
            Bytes::from(b"code".as_slice()),
        ),
        storage: (
            Address::new([3; 20]),
            U256::ZERO,
            BlockId::Number(4),
            Hash::new([5; 32]),
        ),
        logs: (LogFilter::default(), Vec::new()),
        receipt: (transaction_hash, Some(receipt.clone())),
    };
    let transaction = TransactionExecutorCase {
        request: TransactionRequest::default(),
        filled: TransactionRequest {
            chain_id: Some(1),
            ..TransactionRequest::default()
        },
        transaction_hash,
        receipt,
    };
    let backend = FixtureBackend {
        reads: Arc::new(reads.clone()),
        transaction: Arc::new(transaction.clone()),
    };
    futures::executor::block_on(check_read_backend(&backend, &reads)).unwrap();
    futures::executor::block_on(check_transaction_executor(&backend, &transaction)).unwrap();
}

#[test]
fn codec_helper_accepts_canonical_fixture() {
    let vector = CodecVector {
        fields: vec![AbiField {
            name: "value".into(),
            ty: AbiType::Uint(256),
            internal_type: None,
        }],
        values: vec![AbiValue::Uint(U256::from_u128(42).unwrap().to_be_bytes())],
        encoded: Bytes::new(U256::from_u128(42).unwrap().to_be_bytes().to_vec()),
    };
    check_codec_vector(&vector).unwrap();
}

#[test]
fn generated_layout_helpers_accept_canonical_fixtures() {
    let root = crate::generated_test_fixture::tempdir();
    fs::write(
        root.path().join("Cargo.toml"),
        b"[package]\nname='x'\nversion='0.1.0'\nedition='2024'\n\
          rust-version='1.95'\npublish=false\n\n[workspace]\n\n[dependencies]\n\
          musubi-core={path='../crates/musubi-core'}\n\
          musubi-runtime={path='../crates/musubi-runtime'}\n",
    )
    .unwrap();
    let mut release: serde_json::Value = serde_json::from_slice(
        br#"{
          "schema_version":"musubi/release-v4",
          "typed_data": [],
          "manifest_schema": "musubi/config-v4",
          "manifest_sha256": "0x7777777777777777777777777777777777777777777777777777777777777777",
          "package":{"name":"x","version":"0.1.0"},
          "generator": {"schema_version": "musubi/generator-info-v1", "version": "0.1.0", "revision": "unknown", "git_commit": null, "dirty": null, "build": {"rustc": "unknown", "host": "test-host", "target": "test-target", "profile": "test"}},
          "generated_abi_version": 2,
          "runtime":{"source":"path","root":".."},
          "contracts":[{
            "id":"vault",
            "rust_module": "vault",
            "kind": "deployable",
            "canonical_abi_sha256":"3333333333333333333333333333333333333333333333333333333333333333",
            "raw_abi_sha256":"1111111111111111111111111111111111111111111111111111111111111111",
            "creation_template_keccak256":"2222222222222222222222222222222222222222222222222222222222222222",
            "runtime_template_keccak256":null,
            "external_artifact_coherence":"unverified"
          }]
        }"#,
    )
    .unwrap();
    materialize(root.path(), &mut release);
    let report = inspect_generated_crate(root.path()).unwrap();
    assert_eq!(report.contracts, 1);
    assert_eq!(report.generated_abi_version, 2);

    let mut release: serde_json::Value = serde_json::from_slice(
        br#"{
          "schema_version":"musubi/release-v4",
          "manifest_schema": "musubi/config-v4",
          "manifest_sha256": "0x7777777777777777777777777777777777777777777777777777777777777777",
          "package":{"name":"x","version":"0.1.0"},
          "generator": {"schema_version": "musubi/generator-info-v1", "version": "0.1.0", "revision": "unknown", "git_commit": null, "dirty": null, "build": {"rustc": "unknown", "host": "test-host", "target": "test-target", "profile": "test"}},
          "generated_abi_version": 2,
          "runtime":{"source":"path","root":".."},
          "contracts":[{
            "id":"status",
            "rust_module":"status",
            "kind":"interface",
            "canonical_abi_sha256":"3333333333333333333333333333333333333333333333333333333333333333",
            "raw_abi_sha256":"1111111111111111111111111111111111111111111111111111111111111111",
            "creation_template_keccak256":null,
            "runtime_template_keccak256":null,
            "external_artifact_coherence":"unverified"
          }],
          "typed_data":[{
            "id":"message",
            "rust_module":"message",
            "schema_sha256":"0x7777777777777777777777777777777777777777777777777777777777777777",
            "vectors_sha256":"0x1111111111111111111111111111111111111111111111111111111111111111",
            "roots":[{
              "primary_type":"Message",
              "type_string":"Message(uint256 value)",
              "type_hash":"0x2222222222222222222222222222222222222222222222222222222222222222",
              "domain_type":null,
              "domain_type_string":null,
              "domain_type_hash":null
            }],
            "evidence":{
              "solidity_source_sha256":"0x3333333333333333333333333333333333333333333333333333333333333333",
              "artifact_target":"test/Message.t.sol:MessageTest",
              "artifact_sha256":"0x4444444444444444444444444444444444444444444444444444444444444444",
              "compiler_version":"0.8.30+commit.73712a01",
              "compiler_settings_sha256":"0x5555555555555555555555555555555555555555555555555555555555555555",
              "vector_command_sha256":"0x6666666666666666666666666666666666666666666666666666666666666666"
            }
          }]
        }"#,
    )
    .unwrap();
    release["typed_data"][0]["roots"][0]["type_hash"] = serde_json::Value::String(format!(
        "0x{}",
        hex::encode(keccak256(b"Message(uint256 value)"))
    ));
    materialize(root.path(), &mut release);
    let report = inspect_generated_crate(root.path()).unwrap();
    assert_eq!(report.contracts, 1);
    assert_eq!(report.typed_data, 1);
}
