//! Public generated-client compatibility without requiring a clonable provider stack.

use alloy_provider::{Provider, RootProvider};
use musubi_alloy::{AlloyExecutor, ReceiptPolicy};
use musubi_core::{Address, FixedBytes};
use musubi_safe_v141_generated::safe_v141::{ApproveHashArgs, SafeV141};

// Type-check both execution paths for every Provider, with no P: Clone bound. Dropping the
// futures keeps this compile regression independent of RPC availability.
fn connect<P: Provider>(executor: AlloyExecutor<P>) {
    let client = SafeV141::at(Address::ZERO).connect(executor);
    drop(client.nonce().call());
    drop(
        client
            .approve_hash(ApproveHashArgs {
                hash_to_approve: FixedBytes::default(),
            })
            .send(),
    );
}

#[test]
fn generated_client_accepts_the_official_executor() {
    let provider = RootProvider::new_http("http://127.0.0.1:1".parse().unwrap());
    connect(AlloyExecutor::new(
        provider,
        Address::ZERO,
        ReceiptPolicy::default(),
    ));
}
