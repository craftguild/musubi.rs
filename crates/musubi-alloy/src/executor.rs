//! Wallet-agnostic transaction execution through an Alloy provider stack.

use std::time::Duration;

use alloy_network::Ethereum;
use alloy_provider::{PendingTransactionBuilder, Provider};
use musubi_core::{
    Address, BlockId, Bytes, CallRequest, Hash, Log, LogFilter, Receipt, TransactionRequest, U256,
};
use musubi_runtime::{BackendError, BackendFuture, ReadBackend, TransactionExecutor};

use crate::{AlloyRead, convert};

/// Confirmation and timeout policy applied while awaiting receipts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReceiptPolicy {
    confirmations: u64,
    timeout: Duration,
}

impl ReceiptPolicy {
    /// Creates a validated receipt policy.
    ///
    /// # Errors
    ///
    /// Returns an invalid-request error when either value is zero.
    pub fn new(confirmations: u64, timeout: Duration) -> Result<Self, BackendError> {
        if confirmations == 0 {
            return Err(convert::invalid_request(
                "receipt confirmations must be non-zero",
            ));
        }
        if timeout.is_zero() {
            return Err(convert::invalid_request("receipt timeout must be non-zero"));
        }
        Ok(Self {
            confirmations,
            timeout,
        })
    }

    /// Returns required block confirmations.
    #[must_use]
    pub const fn confirmations(self) -> u64 {
        self.confirmations
    }

    /// Returns the receipt timeout.
    #[must_use]
    pub const fn timeout(self) -> Duration {
        self.timeout
    }
}

impl Default for ReceiptPolicy {
    fn default() -> Self {
        Self {
            confirmations: 1,
            timeout: Duration::from_mins(2),
        }
    }
}

/// Transaction executor over a user-supplied Alloy provider and wallet/filler stack.
///
/// The provider must be capable of signing or forwarding transactions. The adapter never owns
/// private key bytes and delegates signing to the provider assembled by the caller.
/// Cloning shares the underlying provider stack and copies sender and receipt policy without
/// requiring the provider itself to implement `Clone`.
pub struct AlloyExecutor<P> {
    read: AlloyRead<P>,
    sender: Address,
    receipt_policy: ReceiptPolicy,
}

impl<P> Clone for AlloyExecutor<P> {
    fn clone(&self) -> Self {
        Self {
            read: self.read.clone(),
            sender: self.sender,
            receipt_policy: self.receipt_policy,
        }
    }
}

impl<P> AlloyExecutor<P> {
    /// Wraps a provider stack with its default sender and receipt policy.
    #[must_use]
    pub fn new(provider: P, sender: Address, receipt_policy: ReceiptPolicy) -> Self {
        Self::from_read(AlloyRead::new(provider), sender, receipt_policy)
    }

    /// Adds execution policy to an existing read adapter.
    #[must_use]
    pub const fn from_read(
        read: AlloyRead<P>,
        sender: Address,
        receipt_policy: ReceiptPolicy,
    ) -> Self {
        Self {
            read,
            sender,
            receipt_policy,
        }
    }

    /// Borrows the read-only view of this provider.
    #[must_use]
    pub const fn read(&self) -> &AlloyRead<P> {
        &self.read
    }

    /// Returns the default transaction sender.
    #[must_use]
    pub const fn sender(&self) -> Address {
        self.sender
    }
}

impl<P> ReadBackend for AlloyExecutor<P>
where
    P: Provider<Ethereum>,
{
    fn chain_id(&self) -> BackendFuture<'_, u64> {
        self.read.chain_id()
    }

    fn call(&self, request: CallRequest, block: BlockId) -> BackendFuture<'_, Bytes> {
        self.read.call(request, block)
    }

    fn estimate_gas(&self, request: CallRequest, block: BlockId) -> BackendFuture<'_, u64> {
        self.read.estimate_gas(request, block)
    }

    fn code(&self, address: Address, block: BlockId) -> BackendFuture<'_, Bytes> {
        self.read.code(address, block)
    }

    fn storage(&self, address: Address, slot: U256, block: BlockId) -> BackendFuture<'_, Hash> {
        self.read.storage(address, slot, block)
    }

    fn logs(&self, filter: LogFilter) -> BackendFuture<'_, Vec<Log>> {
        self.read.logs(filter)
    }

    fn receipt(&self, transaction: Hash) -> BackendFuture<'_, Option<Receipt>> {
        self.read.receipt(transaction)
    }
}

impl<P> TransactionExecutor for AlloyExecutor<P>
where
    P: Provider<Ethereum>,
{
    fn fill_transaction(
        &self,
        mut request: TransactionRequest,
    ) -> BackendFuture<'_, TransactionRequest> {
        Box::pin(async move {
            request.from.get_or_insert(self.sender);
            if request.chain_id.is_none() {
                request.chain_id = Some(
                    self.read
                        .provider()
                        .get_chain_id()
                        .await
                        .map_err(|_| convert::rpc_error("eth_chainId"))?,
                );
            }
            if request.nonce.is_none() {
                let sender = convert::address(request.from.expect("sender was inserted above"));
                request.nonce = Some(
                    self.read
                        .provider()
                        .get_transaction_count(sender)
                        .pending()
                        .await
                        .map_err(|_| convert::rpc_error("eth_getTransactionCount"))?,
                );
            }
            fill_fees(self.read.provider(), &mut request).await?;
            if request.gas.is_none() {
                request.gas = Some(
                    self.read
                        .provider()
                        .estimate_gas(convert::transaction_request(&request)?)
                        .await
                        .map_err(|_| convert::rpc_error("eth_estimateGas"))?,
                );
            }
            Ok(request)
        })
    }

    fn sign_and_broadcast(&self, request: TransactionRequest) -> BackendFuture<'_, Hash> {
        Box::pin(async move {
            let pending = self
                .read
                .provider()
                .send_transaction(convert::transaction_request(&request)?)
                .await
                .map_err(|_| convert::rpc_error("eth_sendRawTransaction"))?;
            Ok(Hash::new(pending.tx_hash().0))
        })
    }

    fn wait_for_receipt(&self, transaction: Hash) -> BackendFuture<'_, Receipt> {
        Box::pin(async move {
            let receipt = PendingTransactionBuilder::<Ethereum>::new(
                self.read.provider().root().clone(),
                convert::hash(transaction),
            )
            .with_required_confirmations(self.receipt_policy.confirmations)
            .with_timeout(Some(self.receipt_policy.timeout))
            .get_receipt()
            .await
            .map_err(|_| convert::rpc_error("eth_getTransactionReceipt"))?;
            Ok(convert::receipt(&receipt))
        })
    }
}

async fn fill_fees<P: Provider<Ethereum>>(
    provider: &P,
    request: &mut TransactionRequest,
) -> Result<(), BackendError> {
    if request.gas_price.is_some()
        && (request.max_fee_per_gas.is_some() || request.max_priority_fee_per_gas.is_some())
    {
        return Err(convert::invalid_request(
            "legacy and EIP-1559 fee fields are mutually exclusive",
        ));
    }
    if request.gas_price.is_some()
        || (request.max_fee_per_gas.is_some() && request.max_priority_fee_per_gas.is_some())
    {
        return Ok(());
    }
    let estimate = provider
        .estimate_eip1559_fees()
        .await
        .map_err(|_| convert::rpc_error("eth_feeHistory"))?;
    request
        .max_fee_per_gas
        .get_or_insert(U256::from_u128(estimate.max_fee_per_gas).map_err(|_| {
            convert::invalid_request("estimated maximum fee exceeds owned integer width")
        })?);
    request.max_priority_fee_per_gas.get_or_insert(
        U256::from_u128(estimate.max_priority_fee_per_gas).map_err(|_| {
            convert::invalid_request("estimated priority fee exceeds owned integer width")
        })?,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Shared executor ownership and policy preservation without a provider Clone bound.

    use std::sync::Arc;

    use super::*;

    #[test]
    fn clone_shares_a_non_clone_provider_and_preserves_execution_policy() {
        struct NonCloneProvider;
        let sender = Address::new([7; 20]);
        let policy = ReceiptPolicy::new(3, Duration::from_secs(9)).unwrap();
        let executor = AlloyExecutor::new(NonCloneProvider, sender, policy);
        let cloned = executor.clone();
        assert!(Arc::ptr_eq(
            &executor.read().provider_arc(),
            &cloned.read().provider_arc()
        ));
        assert_eq!(cloned.sender(), sender);
        assert_eq!(cloned.receipt_policy, policy);
    }
}
