//! Owned EIP-712 `SafeTx` value, conversion, and hashing methods.

use super::__evm_descriptors::descriptors;
use super::EIP712Domain;
/// Owned EIP-712 `SafeTx` value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeTx {
    /// Solidity field `to`.
    pub to: musubi_core::Address,
    /// Solidity field `value`.
    pub value: musubi_core::Uint<256>,
    /// Solidity field `data`.
    pub data: musubi_core::Bytes,
    /// Solidity field `operation`.
    pub operation: musubi_core::Uint<8>,
    /// Solidity field `safeTxGas`.
    pub safe_tx_gas: musubi_core::Uint<256>,
    /// Solidity field `baseGas`.
    pub base_gas: musubi_core::Uint<256>,
    /// Solidity field `gasPrice`.
    pub gas_price: musubi_core::Uint<256>,
    /// Solidity field `gasToken`.
    pub gas_token: musubi_core::Address,
    /// Solidity field `refundReceiver`.
    pub refund_receiver: musubi_core::Address,
    /// Solidity field `nonce`.
    pub nonce: musubi_core::Uint<256>,
}

impl SafeTx {
    /// Canonical EIP-712 encodeType string.
    pub const TYPE_STRING: &'static str = "SafeTx(address to,uint256 value,bytes data,uint8 operation,uint256 safeTxGas,uint256 baseGas,uint256 gasPrice,address gasToken,address refundReceiver,uint256 nonce)";
    /// Returns the canonical EIP-712 type hash.
    #[must_use]
    pub const fn type_hash() -> musubi_core::Hash { musubi_core::FixedBytes::new([187, 131, 16, 212, 134, 54, 141, 182, 189, 111, 132, 148, 2, 253, 215, 58, 213, 61, 49, 107, 90, 75, 38, 68, 173, 110, 254, 15, 148, 18, 134, 216]) }
    /// Computes this value's strict EIP-712 struct hash.
    pub fn struct_hash(&self) -> Result<musubi_core::Hash, musubi_runtime::TypedDataError> {
        musubi_runtime::typed_data_struct_hash(&descriptors(), "SafeTx", &self.typed_data_value())
    }
    /// Computes the final domain-bound EIP-712 digest.
    pub fn digest(&self, domain: &EIP712Domain) -> Result<musubi_core::Hash, musubi_runtime::TypedDataError> {
        musubi_runtime::typed_data_digest(&descriptors(), "EIP712Domain", &domain.typed_data_value(), "SafeTx", &self.typed_data_value())
    }
    pub(super) fn typed_data_value(&self) -> musubi_core::AbiValue {
        musubi_core::AbiValue::Tuple(vec![
            musubi_core::AbiValue::Address(self.to),
            musubi_core::AbiValue::Uint(self.value.to_be_bytes()),
            musubi_core::AbiValue::Bytes(self.data.clone()),
            musubi_core::AbiValue::Uint(self.operation.to_be_bytes()),
            musubi_core::AbiValue::Uint(self.safe_tx_gas.to_be_bytes()),
            musubi_core::AbiValue::Uint(self.base_gas.to_be_bytes()),
            musubi_core::AbiValue::Uint(self.gas_price.to_be_bytes()),
            musubi_core::AbiValue::Address(self.gas_token),
            musubi_core::AbiValue::Address(self.refund_receiver),
            musubi_core::AbiValue::Uint(self.nonce.to_be_bytes()),
        ])
    }
}
