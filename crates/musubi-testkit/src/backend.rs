//! Reusable read-backend and transaction-executor conformance scenarios.

use musubi_core::{
    Address, BlockId, Bytes, CallRequest, Hash, Log, LogFilter, Receipt, TransactionRequest, U256,
};
use musubi_runtime::{BackendError, ReadBackend, TransactionExecutor};

/// Expected request/result pairs covering every read-backend method.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadBackendCase {
    /// Expected chain ID.
    pub chain_id: u64,
    /// Call request, block selection, and expected bytes.
    pub call: (CallRequest, BlockId, Bytes),
    /// Gas-estimation request, block selection, and expected quantity.
    pub estimate_gas: (CallRequest, BlockId, u64),
    /// Code address, block selection, and expected code.
    pub code: (Address, BlockId, Bytes),
    /// Storage address, slot, block selection, and expected word.
    pub storage: (Address, U256, BlockId, Hash),
    /// Log filter and expected ordered logs.
    pub logs: (LogFilter, Vec<Log>),
    /// Transaction hash and expected optional receipt.
    pub receipt: (Hash, Option<Receipt>),
}

/// Expected transaction-executor lifecycle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionExecutorCase {
    /// Unsigned policy-incomplete request passed to the executor.
    pub request: TransactionRequest,
    /// Expected filled transaction.
    pub filled: TransactionRequest,
    /// Expected broadcast transaction hash.
    pub transaction_hash: Hash,
    /// Expected final receipt.
    pub receipt: Receipt,
}

/// Backend conformance failure preserving only adapter-redacted errors.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BackendConformanceError {
    /// A backend method returned an error.
    #[error("backend conformance operation `{operation}` failed: {error}")]
    Backend {
        /// Stable operation name.
        operation: &'static str,
        /// Adapter-redacted backend error.
        error: BackendError,
    },
    /// A backend method returned a value different from the fixture.
    #[error("backend conformance operation `{operation}` returned an unexpected value")]
    Mismatch {
        /// Stable operation name.
        operation: &'static str,
    },
}

/// Exercises every read-backend method against one deterministic case.
///
/// # Errors
///
/// Returns [`BackendConformanceError`] on the first backend error or result mismatch.
pub async fn check_read_backend(
    backend: &dyn ReadBackend,
    case: &ReadBackendCase,
) -> Result<(), BackendConformanceError> {
    compare("chain_id", backend.chain_id().await, &case.chain_id)?;
    compare(
        "call",
        backend.call(case.call.0.clone(), case.call.1.clone()).await,
        &case.call.2,
    )?;
    compare(
        "estimate_gas",
        backend
            .estimate_gas(case.estimate_gas.0.clone(), case.estimate_gas.1.clone())
            .await,
        &case.estimate_gas.2,
    )?;
    compare(
        "code",
        backend.code(case.code.0, case.code.1.clone()).await,
        &case.code.2,
    )?;
    compare(
        "storage",
        backend
            .storage(case.storage.0, case.storage.1, case.storage.2.clone())
            .await,
        &case.storage.3,
    )?;
    compare(
        "logs",
        backend.logs(case.logs.0.clone()).await,
        &case.logs.1,
    )?;
    compare(
        "receipt",
        backend.receipt(case.receipt.0).await,
        &case.receipt.1,
    )
}

/// Exercises fill, broadcast, and receipt completion in protocol order.
///
/// # Errors
///
/// Returns [`BackendConformanceError`] on the first backend error or result mismatch.
pub async fn check_transaction_executor(
    executor: &dyn TransactionExecutor,
    case: &TransactionExecutorCase,
) -> Result<(), BackendConformanceError> {
    let filled = executor
        .fill_transaction(case.request.clone())
        .await
        .map_err(|error| backend_error("fill_transaction", error))?;
    if filled != case.filled {
        return Err(mismatch("fill_transaction"));
    }
    let hash = executor
        .sign_and_broadcast(filled)
        .await
        .map_err(|error| backend_error("sign_and_broadcast", error))?;
    if hash != case.transaction_hash {
        return Err(mismatch("sign_and_broadcast"));
    }
    let receipt = executor
        .wait_for_receipt(hash)
        .await
        .map_err(|error| backend_error("wait_for_receipt", error))?;
    if receipt != case.receipt {
        return Err(mismatch("wait_for_receipt"));
    }
    Ok(())
}

fn compare<T: Eq>(
    operation: &'static str,
    actual: Result<T, BackendError>,
    expected: &T,
) -> Result<(), BackendConformanceError> {
    let actual = actual.map_err(|error| backend_error(operation, error))?;
    if actual == *expected {
        Ok(())
    } else {
        Err(mismatch(operation))
    }
}

const fn backend_error(operation: &'static str, error: BackendError) -> BackendConformanceError {
    BackendConformanceError::Backend { operation, error }
}

const fn mismatch(operation: &'static str) -> BackendConformanceError {
    BackendConformanceError::Mismatch { operation }
}
