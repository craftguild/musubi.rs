//! Owned EIP-712 `EIP712Domain` value, conversion, and hashing methods.

use super::__evm_descriptors::descriptors;
/// Owned EIP-712 `EIP712Domain` value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EIP712Domain {
    /// Solidity field `chainId`.
    pub chain_id: musubi_core::Uint<256>,
    /// Solidity field `verifyingContract`.
    pub verifying_contract: musubi_core::Address,
}

impl EIP712Domain {
    /// Canonical EIP-712 encodeType string.
    pub const TYPE_STRING: &'static str = "EIP712Domain(uint256 chainId,address verifyingContract)";
    /// Returns the canonical EIP-712 type hash.
    #[must_use]
    pub const fn type_hash() -> musubi_core::Hash { musubi_core::FixedBytes::new([71, 231, 149, 52, 162, 69, 149, 46, 139, 22, 137, 58, 51, 107, 133, 163, 217, 234, 159, 168, 197, 115, 243, 216, 3, 175, 185, 42, 121, 70, 146, 24]) }
    /// Computes this value's strict EIP-712 struct hash.
    pub fn struct_hash(&self) -> Result<musubi_core::Hash, musubi_runtime::TypedDataError> {
        musubi_runtime::typed_data_struct_hash(&descriptors(), "EIP712Domain", &self.typed_data_value())
    }
    /// Computes this domain value's strict EIP-712 separator.
    pub fn domain_separator(&self) -> Result<musubi_core::Hash, musubi_runtime::TypedDataError> { Self::struct_hash(self) }
    pub(super) fn typed_data_value(&self) -> musubi_core::AbiValue {
        musubi_core::AbiValue::Tuple(vec![
            musubi_core::AbiValue::Uint(self.chain_id.to_be_bytes()),
            musubi_core::AbiValue::Address(self.verifying_contract),
        ])
    }
}
