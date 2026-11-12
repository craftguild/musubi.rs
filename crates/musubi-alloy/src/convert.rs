//! Lossless conversions between kit-owned values and Alloy RPC values.

use alloy_eips::{BlockId as RpcBlockId, BlockNumberOrTag, RpcBlockHash};
use alloy_primitives::{Address as RpcAddress, B256, Bytes as RpcBytes, TxKind, U256 as RpcU256};
use alloy_rpc_types_eth::{
    Filter, Log as RpcLog, TransactionInput, TransactionReceipt,
    TransactionRequest as RpcTransactionRequest,
};
use musubi_core::{
    Address, BlockId, Bytes, CallRequest, Hash, Log, LogFilter, Receipt, TopicFilter,
    TransactionRequest, U256,
};
use musubi_runtime::{BackendError, BackendErrorKind};

pub(crate) fn address(value: Address) -> RpcAddress {
    RpcAddress::from(value.into_array())
}

pub(crate) fn hash(value: Hash) -> B256 {
    B256::from(value.into_array())
}

pub(crate) fn uint(value: U256) -> RpcU256 {
    RpcU256::from_be_bytes(value.to_be_bytes())
}

pub(crate) fn block(value: &BlockId) -> RpcBlockId {
    match value {
        BlockId::Latest => RpcBlockId::Number(BlockNumberOrTag::Latest),
        BlockId::Pending => RpcBlockId::Number(BlockNumberOrTag::Pending),
        BlockId::Earliest => RpcBlockId::Number(BlockNumberOrTag::Earliest),
        BlockId::Number(number) => RpcBlockId::Number(BlockNumberOrTag::Number(*number)),
        BlockId::Hash(value) => RpcBlockId::Hash(RpcBlockHash::from(hash(*value))),
    }
}

pub(crate) fn call_request(value: &CallRequest) -> RpcTransactionRequest {
    RpcTransactionRequest {
        from: value.from.map(address),
        to: Some(
            value
                .to
                .map_or(TxKind::Create, |to| TxKind::Call(address(to))),
        ),
        gas: value.gas,
        value: Some(uint(value.value)),
        input: TransactionInput::new(RpcBytes::copy_from_slice(value.input.as_ref())),
        ..RpcTransactionRequest::default()
    }
}

pub(crate) fn transaction_request(
    value: &TransactionRequest,
) -> Result<RpcTransactionRequest, BackendError> {
    Ok(RpcTransactionRequest {
        from: value.from.map(address),
        to: Some(
            value
                .to
                .map_or(TxKind::Create, |to| TxKind::Call(address(to))),
        ),
        gas_price: optional_fee(value.gas_price)?,
        max_fee_per_gas: optional_fee(value.max_fee_per_gas)?,
        max_priority_fee_per_gas: optional_fee(value.max_priority_fee_per_gas)?,
        gas: value.gas,
        value: Some(uint(value.value)),
        input: TransactionInput::new(RpcBytes::copy_from_slice(value.input.as_ref())),
        nonce: value.nonce,
        chain_id: value.chain_id,
        ..RpcTransactionRequest::default()
    })
}

fn optional_fee(value: Option<U256>) -> Result<Option<u128>, BackendError> {
    value
        .map(|fee| {
            fee.try_into_u128()
                .map_err(|_| invalid_request("transaction fee exceeds Alloy's 128-bit RPC field"))
        })
        .transpose()
}

pub(crate) fn log_filter(value: LogFilter) -> Result<Filter, BackendError> {
    if value.topics.len() > 4 {
        return Err(invalid_request(
            "Ethereum log filters support at most four topics",
        ));
    }
    let mut filter = filter_blocks(value.from_block, value.to_block)?;
    if !value.addresses.is_empty() {
        filter = filter.address(value.addresses.into_iter().map(address).collect::<Vec<_>>());
    }
    for (index, topic) in value.topics.into_iter().enumerate() {
        filter.topics[index] = match topic {
            TopicFilter::Any => continue,
            TopicFilter::Exact(value) => hash(value).into(),
            TopicFilter::OneOf(values) if values.is_empty() => {
                return Err(invalid_request(
                    "a topic alternative list must not be empty",
                ));
            }
            TopicFilter::OneOf(values) => values.into_iter().map(hash).collect::<Vec<_>>().into(),
        };
    }
    Ok(filter)
}

fn filter_blocks(from: Option<BlockId>, to: Option<BlockId>) -> Result<Filter, BackendError> {
    match (&from, &to) {
        (Some(BlockId::Hash(left)), Some(BlockId::Hash(right))) if left == right => {
            Ok(Filter::new().at_block_hash(hash(*left)))
        }
        (Some(BlockId::Hash(value)), None) | (None, Some(BlockId::Hash(value))) => {
            Ok(Filter::new().at_block_hash(hash(*value)))
        }
        (Some(BlockId::Hash(_)), _) | (_, Some(BlockId::Hash(_))) => Err(invalid_request(
            "a block-hash log filter cannot be combined with a range",
        )),
        _ => {
            let mut filter = Filter::new();
            if let Some(value) = from {
                filter = filter.from_block(block_tag(&value));
            }
            if let Some(value) = to {
                filter = filter.to_block(block_tag(&value));
            }
            Ok(filter)
        }
    }
}

fn block_tag(value: &BlockId) -> BlockNumberOrTag {
    match value {
        BlockId::Latest => BlockNumberOrTag::Latest,
        BlockId::Pending => BlockNumberOrTag::Pending,
        BlockId::Earliest => BlockNumberOrTag::Earliest,
        BlockId::Number(number) => BlockNumberOrTag::Number(*number),
        BlockId::Hash(_) => unreachable!("hash block IDs are handled before range conversion"),
    }
}

pub(crate) fn log(value: &RpcLog) -> Log {
    Log {
        address: Address::new(value.address().into_array()),
        topics: value
            .topics()
            .iter()
            .map(|topic| Hash::new(topic.0))
            .collect(),
        data: Bytes::new(value.inner.data.data.to_vec()),
        block_hash: value.block_hash.map(|digest| Hash::new(digest.0)),
        transaction_hash: value.transaction_hash.map(|digest| Hash::new(digest.0)),
        log_index: value.log_index,
        removed: value.removed,
    }
}

pub(crate) fn receipt(value: &TransactionReceipt) -> Receipt {
    Receipt {
        transaction_hash: Hash::new(value.transaction_hash.0),
        block_hash: value.block_hash.map(|digest| Hash::new(digest.0)),
        block_number: value.block_number,
        contract_address: value
            .contract_address
            .map(|address| Address::new(address.into_array())),
        gas_used: value.gas_used,
        success: Some(value.status()),
        logs: value.logs().iter().map(log).collect(),
    }
}

pub(crate) fn invalid_request(message: &'static str) -> BackendError {
    BackendError::new(BackendErrorKind::InvalidRequest, message)
}

pub(crate) fn rpc_error(operation: &'static str) -> BackendError {
    BackendError::new(
        BackendErrorKind::Rpc,
        format!("Alloy RPC operation `{operation}` failed"),
    )
}
