//! Safe transaction and EIP-712 domain values.

use musubi_core::{Address, Bytes, U256};

/// EIP-712 domain fields used by Safe 1.4.1.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SafeDomain {
    /// Chain identifier included in replay protection.
    pub chain_id: U256,
    /// Safe proxy whose state and nonce govern the transaction.
    pub verifying_contract: Address,
}

/// Safe execution operation encoded in a transaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SafeOperation {
    /// Execute an ordinary EVM call from the Safe.
    Call = 0,
    /// Execute target code in the Safe's storage context.
    DelegateCall = 1,
}

/// Safe 1.4.1 transaction fields in canonical EIP-712 order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeTransaction {
    /// Destination account or contract.
    pub to: Address,
    /// Native value sent by the Safe.
    pub value: U256,
    /// Calldata supplied to the destination.
    pub data: Bytes,
    /// CALL or DELEGATECALL execution mode.
    pub operation: SafeOperation,
    /// Gas reserved for the inner Safe transaction.
    pub safe_tx_gas: U256,
    /// Base gas reimbursed independently of inner execution.
    pub base_gas: U256,
    /// Gas price used by Safe refund accounting.
    pub gas_price: U256,
    /// ERC-20 refund token, or zero for native currency.
    pub gas_token: Address,
    /// Refund recipient, or zero for Safe's default receiver policy.
    pub refund_receiver: Address,
    /// Safe nonce that prevents replay.
    pub nonce: U256,
}

impl Default for SafeTransaction {
    fn default() -> Self {
        Self {
            to: Address::ZERO,
            value: U256::ZERO,
            data: Bytes::default(),
            operation: SafeOperation::Call,
            safe_tx_gas: U256::ZERO,
            base_gas: U256::ZERO,
            gas_price: U256::ZERO,
            gas_token: Address::ZERO,
            refund_receiver: Address::ZERO,
            nonce: U256::ZERO,
        }
    }
}
