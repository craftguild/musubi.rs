//! Backend-neutral Safe 1.4.1 operations built from reviewed generated ABI.

use musubi_core::{Address, CallRequest, Hash, TransactionRequest, Uint};
use musubi_safe_v141_generated::safe_v141::{
    ApproveHashArgs, ExecTransactionArgs, GetTransactionHashArgs, SafeV141 as GeneratedSafeV141,
};

use crate::{PackedSignatures, SafeTransaction};

/// Maximum calldata encoded by Safe operation helpers.
pub const MAX_SAFE_CALLDATA_BYTES: usize = 16 * 1024 * 1024;
const HEAD_BYTES: usize = 10 * 32;

/// Fail-closed Safe operation encoding error.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SafeEncodingError {
    /// Dynamic calldata exceeds the public hard limit.
    #[error("Safe calldata exceeds the hard limit")]
    CalldataTooLarge,
    /// Offset or padding arithmetic overflowed.
    #[error("Safe calldata size overflow")]
    SizeOverflow,
    /// Reviewed generated ABI and its owned argument type disagree.
    #[error("generated Safe ABI encoding failed")]
    GeneratedAbi,
}

/// Builds an `approveHash(bytes32)` transaction request.
///
/// # Errors
///
/// Returns [`SafeEncodingError::GeneratedAbi`] if the reviewed generated ABI invariant fails.
pub fn approve_hash_request(
    safe: Address,
    transaction_hash: Hash,
) -> Result<TransactionRequest, SafeEncodingError> {
    GeneratedSafeV141::approve_hash(ApproveHashArgs {
        hash_to_approve: transaction_hash,
    })
    .at(safe)
    .transaction_request()
    .map_err(|_| SafeEncodingError::GeneratedAbi)
}

/// Builds an `execTransaction` request for validated packed signatures.
///
/// # Errors
///
/// Returns [`SafeEncodingError`] when dynamic calldata exceeds the hard limit, size arithmetic
/// overflows, or the generated ABI invariant fails closed.
pub fn exec_transaction_request(
    safe: Address,
    transaction: &SafeTransaction,
    signatures: &PackedSignatures,
) -> Result<TransactionRequest, SafeEncodingError> {
    let data_tail = dynamic_tail_size(transaction.data.len())?;
    let signature_tail = dynamic_tail_size(signatures.as_bytes().len())?;
    let total = 4_usize
        .checked_add(HEAD_BYTES)
        .and_then(|size| size.checked_add(data_tail))
        .and_then(|size| size.checked_add(signature_tail))
        .ok_or(SafeEncodingError::SizeOverflow)?;
    ensure_limit(total)?;
    GeneratedSafeV141::exec_transaction(ExecTransactionArgs {
        to: transaction.to,
        value: transaction.value,
        data: transaction.data.clone(),
        operation: operation(transaction),
        safe_tx_gas: transaction.safe_tx_gas,
        base_gas: transaction.base_gas,
        gas_price: transaction.gas_price,
        gas_token: transaction.gas_token,
        refund_receiver: transaction.refund_receiver,
        signatures: signatures.as_bytes().into(),
    })
    .at(safe)
    .transaction_request()
    .map_err(|_| SafeEncodingError::GeneratedAbi)
}

/// Builds a read request for Safe's canonical `getTransactionHash` implementation.
///
/// # Errors
///
/// Returns [`SafeEncodingError`] when transaction data exceeds the hard limit or the generated
/// ABI invariant fails closed.
pub fn get_transaction_hash_call(
    safe: Address,
    transaction: &SafeTransaction,
) -> Result<CallRequest, SafeEncodingError> {
    let data_tail = dynamic_tail_size(transaction.data.len())?;
    let total = 4_usize
        .checked_add(HEAD_BYTES)
        .and_then(|size| size.checked_add(data_tail))
        .ok_or(SafeEncodingError::SizeOverflow)?;
    ensure_limit(total)?;
    GeneratedSafeV141::get_transaction_hash(GetTransactionHashArgs {
        to: transaction.to,
        value: transaction.value,
        data: transaction.data.clone(),
        operation: operation(transaction),
        safe_tx_gas: transaction.safe_tx_gas,
        base_gas: transaction.base_gas,
        gas_price: transaction.gas_price,
        gas_token: transaction.gas_token,
        refund_receiver: transaction.refund_receiver,
        _nonce: transaction.nonce,
    })
    .at(safe)
    .call_request()
    .map_err(|_| SafeEncodingError::GeneratedAbi)
}

fn operation(transaction: &SafeTransaction) -> Uint<8> {
    Uint::from_u128(transaction.operation as u128)
        .expect("closed Safe operation discriminant fits uint8")
}

fn dynamic_tail_size(length: usize) -> Result<usize, SafeEncodingError> {
    ensure_limit(length)?;
    let padded = length
        .checked_add(31)
        .ok_or(SafeEncodingError::SizeOverflow)?
        / 32
        * 32;
    32_usize
        .checked_add(padded)
        .ok_or(SafeEncodingError::SizeOverflow)
}

fn ensure_limit(size: usize) -> Result<(), SafeEncodingError> {
    if size <= MAX_SAFE_CALLDATA_BYTES {
        Ok(())
    } else {
        Err(SafeEncodingError::CalldataTooLarge)
    }
}
