//! Actual Safe 1.4.1 and EIP-1271 integration using committed official compiler artifacts.

mod support;

use alloy_network::EthereumWallet;
use alloy_primitives::{B256, Signature};
use alloy_provider::ProviderBuilder;
use alloy_signer::{Signer, SignerSync};
use alloy_signer_local::PrivateKeySigner;
use musubi_alloy::{AlloyExecutor, ReceiptPolicy};
use musubi_core::{Address, BlockId, Bytes, Hash, TransactionRequest, U256};
use musubi_runtime::{LibraryAddresses, ReadBackend, TransactionExecutor};
use musubi_safe::{
    EoaRecoveryId, SafeDomain, SafeTransaction, SafeV141, SafeV141Signature,
    exec_transaction_request, get_transaction_hash_call, pack_signatures, safe_v141_registry,
};
use musubi_safe_v141_generated::safe_proxy_v141::{
    ConstructorArgs as SafeProxyConstructorArgs, SafeProxyV141,
};
use musubi_safe_v141_generated::safe_v141::{
    ApproveHashArgs, ExecutionSuccess, GetTransactionHashReturn, SafeV141 as GeneratedSafeV141,
    SetupArgs, artifact as generated_safe_artifact,
};

use support::Anvil;

const OWNER_KEY: &str = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const RELAY_KEY: &str = "59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";
const EIP1271_INIT: &str = "6010600c60003960106000f36320c13b0b60e01b60005260206000f3";

#[tokio::test]
#[ignore = "requires externally provisioned Anvil; run in the anvil-integration CI job"]
async fn all_v141_signature_forms_execute_against_actual_safe() {
    let anvil = Anvil::spawn();
    let owner_signer: PrivateKeySigner = OWNER_KEY.parse().unwrap();
    let relay_signer: PrivateKeySigner = RELAY_KEY.parse().unwrap();
    let owner = owned_address(owner_signer.address());
    let relay = owned_address(relay_signer.address());
    let owner_executor = executor(&anvil, owner_signer.clone());
    let relay_executor = executor(&anvil, relay_signer);
    let libraries = LibraryAddresses::new();

    let contract_owner = deploy(&relay_executor, Bytes::from_hex(EIP1271_INIT).unwrap()).await;
    let singleton = transact(
        &relay_executor,
        GeneratedSafeV141::deployment(&libraries, U256::ZERO).unwrap(),
    )
    .await
    .contract_address
    .unwrap();
    let deployed_code = relay_executor
        .code(singleton, BlockId::Latest)
        .await
        .unwrap();
    let generated_runtime = generated_safe_artifact().runtime.unwrap();
    assert_eq!(deployed_code, generated_runtime.template);
    assert_eq!(
        generated_runtime.template_hash(),
        safe_v141_registry()
            .unwrap()
            .contracts
            .singleton
            .runtime_code_hash
    );

    let safe = transact(
        &relay_executor,
        SafeProxyV141::deployment(
            SafeProxyConstructorArgs {
                _singleton: singleton,
            },
            &libraries,
            U256::ZERO,
        )
        .unwrap(),
    )
    .await
    .contract_address
    .unwrap();
    let vector_bundle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../safe-v141-sdk-input/safe-v1.4.1.typed-data-vectors.json"
    ))
    .unwrap();
    let vector = &vector_bundle["vectors"][0];
    assert_eq!(
        safe,
        vector["domain"]["verifyingContract"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap()
    );
    assert_eq!(
        relay,
        vector["value"]["to"].as_str().unwrap().parse().unwrap()
    );
    transact(
        &relay_executor,
        setup_request(safe, &[owner, contract_owner], 1),
    )
    .await;
    let domain = SafeDomain {
        chain_id: U256::from_u128(31_337).unwrap(),
        verifying_contract: safe,
    };
    let vector_digest = serde_json::from_value(vector["digest"].clone()).unwrap();

    exercise_eoa_signatures(
        &relay_executor,
        &owner_signer,
        domain,
        safe,
        owner,
        relay,
        vector_digest,
    )
    .await;
    exercise_approved_and_contract(
        &owner_executor,
        &relay_executor,
        domain,
        safe,
        owner,
        contract_owner,
        relay,
    )
    .await;
    assert_eq!(read_nonce(&relay_executor, safe).await, 4);
}

async fn exercise_eoa_signatures<P: alloy_provider::Provider>(
    relay_executor: &AlloyExecutor<P>,
    owner_signer: &PrivateKeySigner,
    domain: SafeDomain,
    safe: Address,
    owner: Address,
    relay: Address,
    vector_digest: Hash,
) {
    let tx0 = safe_transaction(relay, 0);
    let hash0 = assert_onchain_hash(relay_executor, domain, safe, &tx0).await;
    assert_eq!(hash0, vector_digest);
    let (r, s, recovery_id) = owned_signature(
        owner_signer
            .sign_hash_sync(&B256::from(hash0.into_array()))
            .unwrap(),
    );
    execute_safe(
        relay_executor,
        safe,
        &tx0,
        hash0,
        SafeV141Signature::Eip712 {
            owner,
            r,
            s,
            recovery_id,
        },
    )
    .await;

    let tx1 = safe_transaction(relay, 1);
    let hash1 = assert_onchain_hash(relay_executor, domain, safe, &tx1).await;
    let (r, s, recovery_id) =
        owned_signature(owner_signer.sign_message(hash1.as_ref()).await.unwrap());
    execute_safe(
        relay_executor,
        safe,
        &tx1,
        hash1,
        SafeV141Signature::EthSign {
            owner,
            r,
            s,
            recovery_id,
        },
    )
    .await;
}

async fn exercise_approved_and_contract<P: alloy_provider::Provider>(
    owner_executor: &AlloyExecutor<P>,
    relay_executor: &AlloyExecutor<P>,
    domain: SafeDomain,
    safe: Address,
    owner: Address,
    contract_owner: Address,
    relay: Address,
) {
    let tx2 = safe_transaction(relay, 2);
    let hash2 = assert_onchain_hash(relay_executor, domain, safe, &tx2).await;
    let client = GeneratedSafeV141::at(safe).connect(owner_executor.clone());
    let receipt = client
        .approve_hash(ApproveHashArgs {
            hash_to_approve: hash2,
        })
        .send()
        .await
        .expect("generated client approveHash");
    assert_eq!(receipt.success, Some(true));
    execute_safe(
        relay_executor,
        safe,
        &tx2,
        hash2,
        SafeV141Signature::ApprovedHash { owner },
    )
    .await;

    let tx3 = safe_transaction(relay, 3);
    let hash3 = assert_onchain_hash(relay_executor, domain, safe, &tx3).await;
    execute_safe(
        relay_executor,
        safe,
        &tx3,
        hash3,
        SafeV141Signature::Contract {
            owner: contract_owner,
            signature: Bytes::from(b"reviewed-eip1271-fixture".as_slice()),
        },
    )
    .await;
}

fn executor(
    anvil: &Anvil,
    signer: PrivateKeySigner,
) -> AlloyExecutor<impl alloy_provider::Provider> {
    let sender = owned_address(signer.address());
    let provider = ProviderBuilder::new()
        .wallet(EthereumWallet::from(signer))
        .connect_http(anvil.endpoint.clone());
    AlloyExecutor::new(provider, sender, ReceiptPolicy::default())
}

async fn deploy<P: alloy_provider::Provider>(executor: &AlloyExecutor<P>, code: Bytes) -> Address {
    transact(
        executor,
        TransactionRequest {
            input: code,
            ..TransactionRequest::default()
        },
    )
    .await
    .contract_address
    .expect("deployment address")
}

async fn transact<P: alloy_provider::Provider>(
    executor: &AlloyExecutor<P>,
    request: TransactionRequest,
) -> musubi_core::Receipt {
    let request = executor
        .fill_transaction(request)
        .await
        .expect("fill transaction");
    let hash = executor
        .sign_and_broadcast(request)
        .await
        .expect("broadcast");
    let receipt = executor.wait_for_receipt(hash).await.expect("receipt");
    assert_eq!(receipt.success, Some(true));
    receipt
}

async fn execute_safe<P: alloy_provider::Provider>(
    executor: &AlloyExecutor<P>,
    safe: Address,
    transaction: &SafeTransaction,
    expected_hash: Hash,
    signature: SafeV141Signature,
) {
    let packed = pack_signatures(vec![signature], 1).unwrap();
    let receipt = transact(
        executor,
        exec_transaction_request(safe, transaction, &packed).unwrap(),
    )
    .await;
    let mut events = receipt
        .logs
        .iter()
        .filter_map(|log| ExecutionSuccess::decode(log).ok());
    let event = events.next().expect("generated ExecutionSuccess event");
    assert!(events.next().is_none());
    assert_eq!(event.tx_hash, expected_hash);
    assert_eq!(event.payment, U256::ZERO);
}

async fn assert_onchain_hash<P: alloy_provider::Provider>(
    executor: &AlloyExecutor<P>,
    domain: SafeDomain,
    safe: Address,
    transaction: &SafeTransaction,
) -> Hash {
    let offline = SafeV141::transaction_hash(domain, transaction);
    let output = executor
        .call(
            get_transaction_hash_call(safe, transaction).unwrap(),
            BlockId::Latest,
        )
        .await
        .expect("getTransactionHash");
    assert_eq!(
        GetTransactionHashReturn::abi_decode(output.as_ref())
            .unwrap()
            .value_0,
        offline
    );
    offline
}

fn safe_transaction(to: Address, nonce: u128) -> SafeTransaction {
    SafeTransaction {
        to,
        data: Bytes::from(b"".as_slice()),
        nonce: U256::from_u128(nonce).unwrap(),
        ..SafeTransaction::default()
    }
}

fn owned_signature(signature: Signature) -> (Hash, Hash, EoaRecoveryId) {
    (
        Hash::new(signature.r().to_be_bytes::<32>()),
        Hash::new(signature.s().to_be_bytes::<32>()),
        if signature.v() {
            EoaRecoveryId::V28
        } else {
            EoaRecoveryId::V27
        },
    )
}

fn setup_request(safe: Address, owners: &[Address], threshold: u128) -> TransactionRequest {
    GeneratedSafeV141::setup(SetupArgs {
        _owners: owners.to_vec(),
        _threshold: U256::from_u128(threshold).unwrap(),
        to: Address::ZERO,
        data: Bytes::default(),
        fallback_handler: Address::ZERO,
        payment_token: Address::ZERO,
        payment: U256::ZERO,
        payment_receiver: Address::ZERO,
    })
    .at(safe)
    .transaction_request()
    .unwrap()
}

async fn read_nonce<P: alloy_provider::Provider>(
    executor: &AlloyExecutor<P>,
    safe: Address,
) -> u128 {
    GeneratedSafeV141::at(safe)
        .connect(executor.clone())
        .nonce()
        .call()
        .await
        .expect("generated client nonce")
        .value_0
        .try_into_u128()
        .unwrap()
}

fn owned_address(address: alloy_primitives::Address) -> Address {
    Address::new(address.into_array())
}
