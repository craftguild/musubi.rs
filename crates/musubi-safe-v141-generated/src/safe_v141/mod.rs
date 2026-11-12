//! Contract facade, address selection, clients, and aggregate event/revert accessors.

pub mod types;
#[path = "artifact.rs"]
mod __evm_artifact;
pub use __evm_artifact::*;

#[path = "operations/v_e_r_s_i_o_n_cde4a074aa9416345955647537c8e834.rs"]
mod __evm_operations_v_e_r_s_i_o_n_cde4a074aa9416345955647537c8e834;
#[path = "operations/add_owner_with_threshold_cc0d88bc8bee437e811cd872872319c8.rs"]
mod __evm_operations_add_owner_with_threshold_cc0d88bc8bee437e811cd872872319c8;
#[path = "operations/approve_hash_8e93d54188d3f00a6e76e74298d492b0.rs"]
mod __evm_operations_approve_hash_8e93d54188d3f00a6e76e74298d492b0;
#[path = "operations/approved_hashes_512d95ff798cca72ee01d5a1d8befb47.rs"]
mod __evm_operations_approved_hashes_512d95ff798cca72ee01d5a1d8befb47;
#[path = "operations/change_threshold_9fbff559c1e42c7dced5b8c8648d317c.rs"]
mod __evm_operations_change_threshold_9fbff559c1e42c7dced5b8c8648d317c;
#[path = "operations/check_n_signatures_52352ed8c7be22741051cd28b350dcbc.rs"]
mod __evm_operations_check_n_signatures_52352ed8c7be22741051cd28b350dcbc;
#[path = "operations/check_signatures_abc013ead0af4098b2ba7451abd12a06.rs"]
mod __evm_operations_check_signatures_abc013ead0af4098b2ba7451abd12a06;
#[path = "operations/disable_module_88c48ea8eb8d3c03a7969d522673cb1b.rs"]
mod __evm_operations_disable_module_88c48ea8eb8d3c03a7969d522673cb1b;
#[path = "operations/domain_separator_461ef608a4663b6d70ae02719c304282.rs"]
mod __evm_operations_domain_separator_461ef608a4663b6d70ae02719c304282;
#[path = "operations/enable_module_c205497b39f7e9939810387a9f55dfd7.rs"]
mod __evm_operations_enable_module_c205497b39f7e9939810387a9f55dfd7;
#[path = "operations/encode_transaction_data_095cdc07b8895c41033ed3a5911b3114.rs"]
mod __evm_operations_encode_transaction_data_095cdc07b8895c41033ed3a5911b3114;
#[path = "operations/exec_transaction_7fc9efe77fb90b5bfe3b65c934252dfc.rs"]
mod __evm_operations_exec_transaction_7fc9efe77fb90b5bfe3b65c934252dfc;
#[path = "operations/exec_transaction_from_module_9c2aff26a9287ff37b9ec50704a1d309.rs"]
mod __evm_operations_exec_transaction_from_module_9c2aff26a9287ff37b9ec50704a1d309;
#[path = "operations/exec_transaction_from_module_return_data_42089baf17f8c071d9413b6fa8b642af.rs"]
mod __evm_operations_exec_transaction_from_module_return_data_42089baf17f8c071d9413b6fa8b642af;
#[path = "operations/get_chain_id_1f75dab939a800069b87f59c11d024b0.rs"]
mod __evm_operations_get_chain_id_1f75dab939a800069b87f59c11d024b0;
#[path = "operations/get_modules_paginated_44a9888e72d30e4083b296ab9223caae.rs"]
mod __evm_operations_get_modules_paginated_44a9888e72d30e4083b296ab9223caae;
#[path = "operations/get_owners_328246ce402605273c932f39aca330d8.rs"]
mod __evm_operations_get_owners_328246ce402605273c932f39aca330d8;
#[path = "operations/get_storage_at_9f89a63383610c18b1348ab35783b248.rs"]
mod __evm_operations_get_storage_at_9f89a63383610c18b1348ab35783b248;
#[path = "operations/get_threshold_1b40d913e12f8f5b1bd97b9518f535e8.rs"]
mod __evm_operations_get_threshold_1b40d913e12f8f5b1bd97b9518f535e8;
#[path = "operations/get_transaction_hash_bcc4e12845d2cdfc6c06565c4bd7d767.rs"]
mod __evm_operations_get_transaction_hash_bcc4e12845d2cdfc6c06565c4bd7d767;
#[path = "operations/is_module_enabled_49d246489a4246a36bccfe9654b8f3ec.rs"]
mod __evm_operations_is_module_enabled_49d246489a4246a36bccfe9654b8f3ec;
#[path = "operations/is_owner_bde7ab829711c847eb7cc88242fdaba0.rs"]
mod __evm_operations_is_owner_bde7ab829711c847eb7cc88242fdaba0;
#[path = "operations/nonce_a0067875eb2e8b7a978a8e2f1dcf3a77.rs"]
mod __evm_operations_nonce_a0067875eb2e8b7a978a8e2f1dcf3a77;
#[path = "operations/remove_owner_ff5f7f48a2d5d7c45306e74d3b5e7ead.rs"]
mod __evm_operations_remove_owner_ff5f7f48a2d5d7c45306e74d3b5e7ead;
#[path = "operations/set_fallback_handler_7548fc62319fc94bdd506899f6dba2f6.rs"]
mod __evm_operations_set_fallback_handler_7548fc62319fc94bdd506899f6dba2f6;
#[path = "operations/set_guard_2bb13b58dfcd8059299b0286945e5115.rs"]
mod __evm_operations_set_guard_2bb13b58dfcd8059299b0286945e5115;
#[path = "operations/setup_fcc7a62f38184b838c2b27ee13b19c13.rs"]
mod __evm_operations_setup_fcc7a62f38184b838c2b27ee13b19c13;
#[path = "operations/signed_messages_745a2d2649c905c444f6f72cd7b15cf7.rs"]
mod __evm_operations_signed_messages_745a2d2649c905c444f6f72cd7b15cf7;
#[path = "operations/simulate_and_revert_4f7dc02e929910ffe060754f1a7e4a88.rs"]
mod __evm_operations_simulate_and_revert_4f7dc02e929910ffe060754f1a7e4a88;
#[path = "operations/swap_owner_469594e6b836a2c3033264c535145cf1.rs"]
mod __evm_operations_swap_owner_469594e6b836a2c3033264c535145cf1;
#[path = "events/added_owner_403737a9bcc8cb0757ccb62646acc260.rs"]
mod __evm_events_added_owner_403737a9bcc8cb0757ccb62646acc260;
#[path = "events/approve_hash_2bef9bdbfcbe92afbcaece8680bde8a4.rs"]
mod __evm_events_approve_hash_2bef9bdbfcbe92afbcaece8680bde8a4;
#[path = "events/changed_fallback_handler_a95155234a8811434b56cbbf97b12b4d.rs"]
mod __evm_events_changed_fallback_handler_a95155234a8811434b56cbbf97b12b4d;
#[path = "events/changed_guard_fee51dfca3a9a84d5e5a434ccdf05d27.rs"]
mod __evm_events_changed_guard_fee51dfca3a9a84d5e5a434ccdf05d27;
#[path = "events/changed_threshold_59a15bac9e7baac8a2c58cb80848046d.rs"]
mod __evm_events_changed_threshold_59a15bac9e7baac8a2c58cb80848046d;
#[path = "events/disabled_module_a048d3987ae033bcd2d48ba372f523ab.rs"]
mod __evm_events_disabled_module_a048d3987ae033bcd2d48ba372f523ab;
#[path = "events/enabled_module_beaffc8be9b35b90ab668cd2501d2bcf.rs"]
mod __evm_events_enabled_module_beaffc8be9b35b90ab668cd2501d2bcf;
#[path = "events/execution_failure_c6c46933b6467a577a80509e1a294d20.rs"]
mod __evm_events_execution_failure_c6c46933b6467a577a80509e1a294d20;
#[path = "events/execution_from_module_failure_ffe11852dcdfcd2654d684d0dd1608d0.rs"]
mod __evm_events_execution_from_module_failure_ffe11852dcdfcd2654d684d0dd1608d0;
#[path = "events/execution_from_module_success_7caec4d4df2c92de75c3678b0a2c3b5a.rs"]
mod __evm_events_execution_from_module_success_7caec4d4df2c92de75c3678b0a2c3b5a;
#[path = "events/execution_success_fcf28d84781f29be4c61b5d028eb1f5d.rs"]
mod __evm_events_execution_success_fcf28d84781f29be4c61b5d028eb1f5d;
#[path = "events/removed_owner_e2f763c61fca1a9757a9f05e55923a37.rs"]
mod __evm_events_removed_owner_e2f763c61fca1a9757a9f05e55923a37;
#[path = "events/safe_received_02969b286f8303a7ee86e518747acc2d.rs"]
mod __evm_events_safe_received_02969b286f8303a7ee86e518747acc2d;
#[path = "events/safe_setup_89ea3fe134377a686e04826921bed8ce.rs"]
mod __evm_events_safe_setup_89ea3fe134377a686e04826921bed8ce;
#[path = "events/sign_msg_b1232c92260f4a123343a654345d77a1.rs"]
mod __evm_events_sign_msg_b1232c92260f4a123343a654345d77a1;
#[path = "constructor.rs"]
mod __evm_constructor;
pub use types::*;
use musubi_core::{Address, Bytes};

/// Marker for manifest contract `safe_v141`.
#[derive(Clone, Copy, Debug, Default)]
pub struct SafeV141;

/// Address-selected `SafeV141` without a backend.
#[derive(Clone, Copy, Debug)]
pub struct SafeV141At { address: Address }

/// Backend-connected `SafeV141` client.
#[derive(Clone, Debug)]
pub struct SafeV141Client<B> { address: Address, backend: B }

impl SafeV141 {
    /// Selects a deployed contract address.
    #[must_use]
    pub fn at(address: Address) -> SafeV141At { SafeV141At { address } }
    /// Returns ABI-authoritative event descriptors.
    #[must_use]
    pub fn events() -> Vec<musubi_core::EventDescriptor> { vec![__evm_events_added_owner_403737a9bcc8cb0757ccb62646acc260::descriptor(),__evm_events_approve_hash_2bef9bdbfcbe92afbcaece8680bde8a4::descriptor(),__evm_events_changed_fallback_handler_a95155234a8811434b56cbbf97b12b4d::descriptor(),__evm_events_changed_guard_fee51dfca3a9a84d5e5a434ccdf05d27::descriptor(),__evm_events_changed_threshold_59a15bac9e7baac8a2c58cb80848046d::descriptor(),__evm_events_disabled_module_a048d3987ae033bcd2d48ba372f523ab::descriptor(),__evm_events_enabled_module_beaffc8be9b35b90ab668cd2501d2bcf::descriptor(),__evm_events_execution_failure_c6c46933b6467a577a80509e1a294d20::descriptor(),__evm_events_execution_from_module_failure_ffe11852dcdfcd2654d684d0dd1608d0::descriptor(),__evm_events_execution_from_module_success_7caec4d4df2c92de75c3678b0a2c3b5a::descriptor(),__evm_events_execution_success_fcf28d84781f29be4c61b5d028eb1f5d::descriptor(),__evm_events_removed_owner_e2f763c61fca1a9757a9f05e55923a37::descriptor(),__evm_events_safe_received_02969b286f8303a7ee86e518747acc2d::descriptor(),__evm_events_safe_setup_89ea3fe134377a686e04826921bed8ce::descriptor(),__evm_events_sign_msg_b1232c92260f4a123343a654345d77a1::descriptor(),] }
    /// Returns ABI-authoritative custom error descriptors.
    #[must_use]
    pub fn errors() -> Vec<musubi_core::ErrorDescriptor> { vec![] }
}

impl SafeV141At {
    /// Connects a cheaply cloneable backend handle.
    #[must_use]
    pub fn connect<B>(self, backend: B) -> SafeV141Client<B> { SafeV141Client { address: self.address, backend } }
}

impl<B: Clone> SafeV141Client<B> {
    /// Returns the selected contract address.
    #[must_use]
    pub const fn address(&self) -> Address { self.address }
}

/// Typed standard and contract-specific revert union.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SafeV141Error {
    /// Standard `Error(string)` revert.
    Error(String),
    /// Standard `Panic(uint256)` revert.
    Panic([u8; 32]),
    /// Unrecognized revert bytes retained without interpretation.
    Unknown(Bytes),
}

impl SafeV141 {
    /// Decodes a standard or ABI-declared custom revert.
    pub fn decode_error(data: &[u8]) -> Result<SafeV141Error, musubi_runtime::CodecError> {
        match musubi_runtime::decode_revert(data, &Self::errors())? {
            musubi_runtime::DecodedRevert::Error(value) => Ok(SafeV141Error::Error(value)),
            musubi_runtime::DecodedRevert::Panic(value) => Ok(SafeV141Error::Panic(value)),
            musubi_runtime::DecodedRevert::Custom { .. } => Ok(SafeV141Error::Unknown(Bytes::new(data.to_vec()))),
            musubi_runtime::DecodedRevert::Unknown => Ok(SafeV141Error::Unknown(Bytes::new(data.to_vec()))),
        }
    }
}
