//! End-to-end Alloy adapter coverage against an externally installed Anvil node.

use std::{
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use alloy_network::EthereumWallet;
use alloy_provider::ProviderBuilder;
use alloy_signer_local::PrivateKeySigner;
use musubi_alloy::{AlloyExecutor, ReceiptPolicy};
use musubi_core::{Address, BlockId, Bytes, CallRequest, LogFilter, TransactionRequest, U256};
use musubi_runtime::{ReadBackend, TransactionExecutor};

const ANVIL_PRIVATE_KEY: &str = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const RUNTIME: &str = "361560175760003560005560005460005260206000a0005b60005460005260206000f3";
const INIT_CODE: &str = "6023600c60003960236000f3361560175760003560005560005460005260206000a0005b60005460005260206000f3";

struct Anvil {
    child: Child,
    endpoint: url::Url,
}

impl Anvil {
    fn spawn() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("reserve Anvil port");
        let port = listener.local_addr().expect("read reserved port").port();
        drop(listener);
        let binary = std::env::var_os("MUSUBI_ANVIL").unwrap_or_else(|| "anvil".into());
        let child = Command::new(binary)
            .args([
                "--port",
                &port.to_string(),
                "--chain-id",
                "31337",
                "--silent",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Anvil is required; set MUSUBI_ANVIL when it is not on PATH");
        let deadline = Instant::now() + Duration::from_secs(10);
        while TcpStream::connect(("127.0.0.1", port)).is_err() {
            assert!(Instant::now() < deadline, "Anvil did not become ready");
            thread::sleep(Duration::from_millis(25));
        }
        Self {
            child,
            endpoint: format!("http://127.0.0.1:{port}")
                .parse()
                .expect("local endpoint is valid"),
        }
    }
}

impl Drop for Anvil {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[tokio::test]
#[ignore = "requires externally provisioned Anvil; run in the anvil-integration CI job"]
async fn deploy_read_write_event_and_receipt_round_trip() {
    let anvil = Anvil::spawn();
    let signer: PrivateKeySigner = ANVIL_PRIVATE_KEY.parse().expect("fixture key is valid");
    let sender = Address::new(signer.address().into_array());
    let provider = ProviderBuilder::new()
        .wallet(EthereumWallet::from(signer))
        .connect_http(anvil.endpoint.clone());
    let executor = AlloyExecutor::new(provider, sender, ReceiptPolicy::default());

    assert_eq!(executor.chain_id().await.expect("chain ID"), 31_337);
    let deployment = transact(
        &executor,
        TransactionRequest {
            input: Bytes::from_hex(INIT_CODE).expect("init code fixture"),
            ..TransactionRequest::default()
        },
    )
    .await;
    let contract = deployment.contract_address.expect("deployment address");
    assert_eq!(
        executor
            .code(contract, BlockId::Latest)
            .await
            .expect("runtime code"),
        Bytes::from_hex(RUNTIME).expect("runtime fixture")
    );

    let initial = call_value(&executor, contract).await;
    assert_eq!(initial, U256::ZERO);
    let mut encoded = [0_u8; 32];
    encoded[31] = 42;
    let write_receipt = transact(
        &executor,
        TransactionRequest {
            to: Some(contract),
            input: Bytes::from(encoded.as_slice()),
            ..TransactionRequest::default()
        },
    )
    .await;
    assert_eq!(write_receipt.success, Some(true));
    assert_eq!(write_receipt.logs.len(), 1);
    assert_eq!(write_receipt.logs[0].data, Bytes::from(encoded.as_slice()));

    let queried = executor
        .logs(LogFilter {
            addresses: vec![contract],
            ..LogFilter::default()
        })
        .await
        .expect("event query");
    assert_eq!(queried, write_receipt.logs);
    assert_eq!(
        call_value(&executor, contract)
            .await
            .try_into_u128()
            .unwrap(),
        42
    );
    assert_eq!(
        executor
            .storage(contract, U256::ZERO, BlockId::Latest)
            .await
            .expect("storage word")
            .into_array(),
        encoded
    );
    assert_eq!(
        executor
            .receipt(write_receipt.transaction_hash)
            .await
            .expect("receipt query"),
        Some(write_receipt)
    );
}

async fn transact<P>(
    executor: &AlloyExecutor<P>,
    request: TransactionRequest,
) -> musubi_core::Receipt
where
    P: alloy_provider::Provider,
{
    let request = executor
        .fill_transaction(request)
        .await
        .expect("fill transaction");
    let hash = executor
        .sign_and_broadcast(request)
        .await
        .expect("sign and broadcast");
    executor
        .wait_for_receipt(hash)
        .await
        .expect("mined receipt")
}

async fn call_value<P>(executor: &AlloyExecutor<P>, contract: Address) -> U256
where
    P: alloy_provider::Provider,
{
    let output = executor
        .call(
            CallRequest {
                to: Some(contract),
                ..CallRequest::default()
            },
            BlockId::Latest,
        )
        .await
        .expect("read call");
    U256::from_be_bytes(output.as_ref().try_into().expect("one ABI word")).expect("valid uint256")
}
