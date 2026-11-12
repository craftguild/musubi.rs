//! Object-safe asynchronous backend interfaces over owned request and result values.

use std::{future::Future, pin::Pin};

use musubi_core::{
    Address, BlockId, Bytes, CallRequest, Hash, Log, LogFilter, Receipt, TransactionRequest, U256,
};

/// Boxed future used to keep backend traits object-safe and runtime-agnostic.
pub type BackendFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, BackendError>> + Send + 'a>>;

/// Stable backend failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendErrorKind {
    /// A backend-neutral request cannot be represented by this adapter.
    InvalidRequest,
    /// Transport failed before a valid RPC response arrived.
    Transport,
    /// Remote endpoint rejected or could not execute a request.
    Rpc,
    /// Signing or transaction completion failed.
    Execution,
    /// Backend returned data that could not be represented safely.
    InvalidResponse,
}

/// Redacted backend error.
///
/// Adapters must not place endpoint credentials, authorization headers, signer material, or source
/// bodies in `message`.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{message}")]
pub struct BackendError {
    /// Stable failure category.
    pub kind: BackendErrorKind,
    /// Redacted human-readable context.
    pub message: String,
}

impl BackendError {
    /// Creates a redacted backend error.
    #[must_use]
    pub fn new(kind: BackendErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// Read-only Ethereum backend used by generated calls and log queries.
pub trait ReadBackend: Send + Sync {
    /// Returns the connected chain ID.
    fn chain_id(&self) -> BackendFuture<'_, u64>;

    /// Executes a read-only EVM call at the selected block.
    fn call(&self, request: CallRequest, block: BlockId) -> BackendFuture<'_, Bytes>;

    /// Estimates gas for a call-shaped request.
    fn estimate_gas(&self, request: CallRequest, block: BlockId) -> BackendFuture<'_, u64>;

    /// Fetches deployed code.
    fn code(&self, address: Address, block: BlockId) -> BackendFuture<'_, Bytes>;

    /// Fetches one storage word.
    fn storage(&self, address: Address, slot: U256, block: BlockId) -> BackendFuture<'_, Hash>;

    /// Fetches logs matching a backend-neutral filter.
    fn logs(&self, filter: LogFilter) -> BackendFuture<'_, Vec<Log>>;

    /// Fetches a transaction receipt when available.
    fn receipt(&self, transaction: Hash) -> BackendFuture<'_, Option<Receipt>>;
}

/// Transaction policy, signing, broadcast, and receipt completion backend.
pub trait TransactionExecutor: ReadBackend {
    /// Fills nonce, fees, gas, sender, and chain policy owned by the executor.
    fn fill_transaction(
        &self,
        request: TransactionRequest,
    ) -> BackendFuture<'_, TransactionRequest>;

    /// Signs and broadcasts one fully specified transaction.
    fn sign_and_broadcast(&self, request: TransactionRequest) -> BackendFuture<'_, Hash>;

    /// Waits according to executor finality policy and returns the mined receipt.
    fn wait_for_receipt(&self, transaction: Hash) -> BackendFuture<'_, Receipt>;
}
