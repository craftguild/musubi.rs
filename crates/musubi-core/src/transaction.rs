//! Backend-neutral requests, logs, and transaction receipts.

use serde::{Deserialize, Serialize};

use crate::{Address, Bytes, Hash, Uint};

/// 256-bit value used for balances, fees, and gas quantities.
pub type U256 = Uint<256>;

/// A block selection accepted by read backends.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockId {
    /// Latest canonical block.
    Latest,
    /// Pending state when supported by the backend.
    Pending,
    /// Earliest available block.
    Earliest,
    /// Explicit block number.
    Number(u64),
    /// Explicit block hash.
    Hash(Hash),
}

/// Backend-neutral `eth_call` and gas-estimation request.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CallRequest {
    /// Optional sender used for call context.
    pub from: Option<Address>,
    /// Destination contract. `None` describes contract creation.
    pub to: Option<Address>,
    /// Calldata or init code.
    pub input: Bytes,
    /// Native value attached to the call.
    pub value: U256,
    /// Optional gas cap.
    pub gas: Option<u64>,
}

/// Backend-neutral unsigned transaction whose missing policy fields belong to an executor.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransactionRequest {
    /// Optional sender; executors may derive it from their signer.
    pub from: Option<Address>,
    /// Destination contract or `None` for creation.
    pub to: Option<Address>,
    /// Calldata or init code.
    pub input: Bytes,
    /// Native value attached to the transaction.
    pub value: U256,
    /// Optional gas limit.
    pub gas: Option<u64>,
    /// Optional nonce.
    pub nonce: Option<u64>,
    /// Optional legacy gas price.
    pub gas_price: Option<U256>,
    /// Optional EIP-1559 maximum fee.
    pub max_fee_per_gas: Option<U256>,
    /// Optional EIP-1559 priority fee.
    pub max_priority_fee_per_gas: Option<U256>,
    /// Optional replay-protection chain ID.
    pub chain_id: Option<u64>,
}

/// Topic constraints for one log topic position.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TopicFilter {
    /// Match any value at this position.
    Any,
    /// Match one exact topic.
    Exact(Hash),
    /// Match any listed topic.
    OneOf(Vec<Hash>),
}

/// Backend-neutral Ethereum log filter.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct LogFilter {
    /// Contract addresses to include; empty means any address.
    pub addresses: Vec<Address>,
    /// Positional topic filters.
    pub topics: Vec<TopicFilter>,
    /// Inclusive starting block.
    pub from_block: Option<BlockId>,
    /// Inclusive ending block.
    pub to_block: Option<BlockId>,
}

/// Backend-neutral Ethereum log.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Log {
    /// Contract that emitted the log.
    pub address: Address,
    /// Indexed topics in wire order.
    pub topics: Vec<Hash>,
    /// Non-indexed event payload.
    pub data: Bytes,
    /// Containing block hash when mined.
    pub block_hash: Option<Hash>,
    /// Containing transaction hash when known.
    pub transaction_hash: Option<Hash>,
    /// Log index within the block when known.
    pub log_index: Option<u64>,
    /// Whether a reorganization removed this log.
    pub removed: bool,
}

/// Backend-neutral transaction receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Receipt {
    /// Transaction hash.
    pub transaction_hash: Hash,
    /// Containing block hash when mined.
    pub block_hash: Option<Hash>,
    /// Containing block number when mined.
    pub block_number: Option<u64>,
    /// Created contract address for deployment transactions.
    pub contract_address: Option<Address>,
    /// Total gas consumed by this transaction.
    pub gas_used: u64,
    /// EVM status when available.
    pub success: Option<bool>,
    /// Logs emitted by the transaction.
    pub logs: Vec<Log>,
}
