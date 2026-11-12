//! Safe 1.4.1 hashing through the reviewed generated typed-data authority.

use musubi_core::{Address, Hash, Uint};
use musubi_safe_v141_generated::safe_v141_typed_data::{
    EIP712Domain as GeneratedDomain, SafeTx as GeneratedSafeTransaction,
};

use crate::{SafeDomain, SafeTransaction};

pub(crate) fn transaction_hash(domain: SafeDomain, transaction: &SafeTransaction) -> Hash {
    generated_transaction(transaction)
        .digest(&GeneratedDomain {
            chain_id: domain.chain_id,
            verifying_contract: domain.verifying_contract,
        })
        .expect("generated Safe typed-data graph is closed and validated")
}

pub(crate) fn address_word(address: Address) -> [u8; 32] {
    let mut word = [0_u8; 32];
    word[12..].copy_from_slice(address.as_ref());
    word
}

fn generated_transaction(transaction: &SafeTransaction) -> GeneratedSafeTransaction {
    GeneratedSafeTransaction {
        to: transaction.to,
        value: transaction.value,
        data: transaction.data.clone(),
        operation: Uint::from_u128(transaction.operation as u128)
            .expect("closed Safe operation discriminant fits uint8"),
        safe_tx_gas: transaction.safe_tx_gas,
        base_gas: transaction.base_gas,
        gas_price: transaction.gas_price,
        gas_token: transaction.gas_token,
        refund_receiver: transaction.refund_receiver,
        nonce: transaction.nonce,
    }
}
