//! Read-only Alloy provider adapter.

use std::sync::Arc;

use alloy_network::Ethereum;
use alloy_provider::{Provider, RootProvider};
use musubi_core::{Address, BlockId, Bytes, CallRequest, Hash, Log, LogFilter, Receipt, U256};
use musubi_runtime::{BackendFuture, ReadBackend};
use url::Url;

use crate::convert;

/// Read-only adapter around an arbitrary Alloy Ethereum provider stack.
pub struct AlloyRead<P> {
    provider: Arc<P>,
}

impl<P> Clone for AlloyRead<P> {
    fn clone(&self) -> Self {
        Self {
            provider: Arc::clone(&self.provider),
        }
    }
}

impl<P> AlloyRead<P> {
    /// Wraps an owned provider stack.
    #[must_use]
    pub fn new(provider: P) -> Self {
        Self {
            provider: Arc::new(provider),
        }
    }

    /// Wraps an already shared provider stack.
    #[must_use]
    pub const fn from_arc(provider: Arc<P>) -> Self {
        Self { provider }
    }

    /// Borrows the underlying Alloy provider.
    #[must_use]
    pub fn provider(&self) -> &P {
        self.provider.as_ref()
    }

    /// Clones the shared provider handle.
    #[must_use]
    pub fn provider_arc(&self) -> Arc<P> {
        Arc::clone(&self.provider)
    }
}

impl AlloyRead<RootProvider<Ethereum>> {
    /// Creates a read-only HTTP provider without retaining the endpoint in adapter errors.
    #[must_use]
    pub fn http(endpoint: Url) -> Self {
        Self::new(RootProvider::new_http(endpoint))
    }
}

impl<P> ReadBackend for AlloyRead<P>
where
    P: Provider<Ethereum>,
{
    fn chain_id(&self) -> BackendFuture<'_, u64> {
        Box::pin(async move {
            self.provider
                .get_chain_id()
                .await
                .map_err(|_| convert::rpc_error("eth_chainId"))
        })
    }

    fn call(&self, request: CallRequest, block: BlockId) -> BackendFuture<'_, Bytes> {
        Box::pin(async move {
            let result = self
                .provider
                .call(convert::call_request(&request))
                .block(convert::block(&block))
                .await
                .map_err(|_| convert::rpc_error("eth_call"))?;
            Ok(Bytes::new(result.to_vec()))
        })
    }

    fn estimate_gas(&self, request: CallRequest, block: BlockId) -> BackendFuture<'_, u64> {
        Box::pin(async move {
            self.provider
                .estimate_gas(convert::call_request(&request))
                .block(convert::block(&block))
                .await
                .map_err(|_| convert::rpc_error("eth_estimateGas"))
        })
    }

    fn code(&self, address: Address, block: BlockId) -> BackendFuture<'_, Bytes> {
        Box::pin(async move {
            let result = self
                .provider
                .get_code_at(convert::address(address))
                .block_id(convert::block(&block))
                .await
                .map_err(|_| convert::rpc_error("eth_getCode"))?;
            Ok(Bytes::new(result.to_vec()))
        })
    }

    fn storage(&self, address: Address, slot: U256, block: BlockId) -> BackendFuture<'_, Hash> {
        Box::pin(async move {
            let result = self
                .provider
                .get_storage_at(convert::address(address), convert::uint(slot))
                .block_id(convert::block(&block))
                .await
                .map_err(|_| convert::rpc_error("eth_getStorageAt"))?;
            Ok(Hash::new(result.to_be_bytes::<32>()))
        })
    }

    fn logs(&self, filter: LogFilter) -> BackendFuture<'_, Vec<Log>> {
        Box::pin(async move {
            let filter = convert::log_filter(filter)?;
            let logs = self
                .provider
                .get_logs(&filter)
                .await
                .map_err(|_| convert::rpc_error("eth_getLogs"))?;
            Ok(logs.iter().map(convert::log).collect())
        })
    }

    fn receipt(&self, transaction: Hash) -> BackendFuture<'_, Option<Receipt>> {
        Box::pin(async move {
            self.provider
                .get_transaction_receipt(convert::hash(transaction))
                .await
                .map(|receipt| receipt.as_ref().map(convert::receipt))
                .map_err(|_| convert::rpc_error("eth_getTransactionReceipt"))
        })
    }
}
