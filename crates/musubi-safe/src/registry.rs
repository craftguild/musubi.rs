//! Reviewed Safe 1.4.1 deployment registry loaded from a generated pinned asset.

use std::sync::OnceLock;

use musubi_core::{Address, Hash};
use serde::Deserialize;

/// Reviewed `safe-smart-account` Git revision.
pub const SMART_ACCOUNT_REVISION: &str = "bf943f80fec5ac647159d26161446ac5d716a294";
/// SHA-256 of `contracts/Safe.sol` fetched at [`SMART_ACCOUNT_REVISION`].
pub const SMART_ACCOUNT_SAFE_SOURCE_SHA256: Hash = Hash::new([
    239, 211, 48, 30, 213, 198, 191, 82, 47, 179, 167, 164, 59, 228, 162, 115, 65, 230, 93, 255,
    109, 31, 238, 4, 248, 86, 127, 48, 74, 103, 227, 101,
]);
/// Reviewed `safe-deployments` Git revision.
pub const DEPLOYMENT_REVISION: &str = "5bb0ebd7150a777f39bec4733e4d799c4b637b49";
const REGISTRY_JSON: &str = include_str!("../assets/safe-v1.4.1-registry.json");

/// One canonical EVM deployment reviewed at the pinned deployment revision.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedContract {
    /// Upstream contract name.
    pub contract_name: String,
    /// Canonical CREATE2 deployment address.
    pub address: Address,
    /// Keccak-256 hash of canonical deployed runtime code.
    pub runtime_code_hash: Hash,
    /// SHA-256 digest of the complete pinned upstream deployment JSON.
    pub upstream_asset_sha256: Hash,
}

/// Canonical Safe 1.4.1 EVM contracts required by the adapter contract.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentContracts {
    /// Safe 1.4.1 singleton implementation.
    pub singleton: ReviewedContract,
    /// Safe 1.4.1 proxy factory.
    pub proxy_factory: ReviewedContract,
    /// Safe 1.4.1 compatibility fallback handler.
    pub compatibility_fallback_handler: ReviewedContract,
}

/// Generated registry tied to reviewed source and deployment revisions.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafeV141Registry {
    /// Generated registry schema version.
    pub schema_version: u32,
    /// Safe protocol version covered by every entry.
    pub safe_version: String,
    /// Exact `safe-smart-account` source revision.
    pub smart_account_revision: String,
    /// SHA-256 of the npm-retained `Safe.sol`, verified against that revision.
    pub smart_account_safe_source_sha256: Hash,
    /// Exact `safe-deployments` registry revision.
    pub deployment_revision: String,
    /// Reviewed canonical EVM contract identities.
    pub contracts: DeploymentContracts,
}

/// Generated registry integrity error.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SafeV141RegistryError {
    /// The committed generated asset is malformed or no longer matches its pinned contract.
    #[error("generated Safe 1.4.1 registry asset failed integrity validation")]
    InvalidGeneratedAsset,
}

/// Returns the parsed and integrity-checked Safe 1.4.1 registry.
///
/// This identifies reviewed official deployments; it does not assert that arbitrary code at the
/// same address is official. Callers must fetch runtime code and compare its hash explicitly.
///
/// # Errors
///
/// Returns [`SafeV141RegistryError`] if the generated asset is malformed or its identity fields
/// differ from the compile-time pins.
pub fn safe_v141_registry() -> Result<&'static SafeV141Registry, SafeV141RegistryError> {
    static REGISTRY: OnceLock<Result<SafeV141Registry, SafeV141RegistryError>> = OnceLock::new();
    REGISTRY
        .get_or_init(|| parse_registry_json(REGISTRY_JSON))
        .as_ref()
        .map_err(|error| *error)
}

fn parse_registry_json(input: &str) -> Result<SafeV141Registry, SafeV141RegistryError> {
    let registry: SafeV141Registry =
        serde_json::from_str(input).map_err(|_| SafeV141RegistryError::InvalidGeneratedAsset)?;
    let valid_identity = registry.schema_version == 2
        && registry.safe_version == "1.4.1"
        && registry.smart_account_revision == SMART_ACCOUNT_REVISION
        && registry.smart_account_safe_source_sha256 == SMART_ACCOUNT_SAFE_SOURCE_SHA256
        && registry.deployment_revision == DEPLOYMENT_REVISION;
    let valid_contracts = pinned_contract_matches(
        &registry.contracts.singleton,
        "Safe",
        [
            65, 103, 92, 9, 159, 50, 52, 27, 248, 75, 252, 83, 130, 175, 83, 77, 245, 199, 70, 26,
        ],
        [
            31, 226, 223, 133, 43, 163, 41, 157, 101, 52, 239, 65, 110, 239, 164, 6, 229, 108, 237,
            153, 91, 202, 136, 106, 183, 165, 83, 230, 208, 197, 225, 196,
        ],
        [
            222, 239, 174, 211, 82, 186, 252, 158, 27, 116, 197, 229, 62, 64, 176, 89, 142, 57,
            115, 21, 194, 111, 206, 154, 132, 163, 77, 153, 103, 155, 241, 153,
        ],
    ) && pinned_contract_matches(
        &registry.contracts.proxy_factory,
        "SafeProxyFactory",
        [
            78, 29, 207, 122, 212, 228, 96, 207, 211, 7, 145, 204, 196, 249, 200, 164, 248, 32,
            236, 103,
        ],
        [
            80, 195, 205, 196, 7, 71, 80, 167, 169, 116, 32, 74, 113, 108, 153, 158, 221, 55, 72,
            47, 144, 118, 8, 217, 96, 178, 176, 37, 238, 11, 51, 23,
        ],
        [
            178, 120, 95, 52, 7, 70, 53, 107, 200, 61, 109, 127, 64, 107, 34, 51, 4, 16, 82, 173,
            34, 87, 87, 139, 122, 239, 105, 211, 5, 203, 39, 107,
        ],
    ) && pinned_contract_matches(
        &registry.contracts.compatibility_fallback_handler,
        "CompatibilityFallbackHandler",
        [
            253, 7, 50, 220, 158, 48, 63, 9, 252, 239, 58, 115, 136, 173, 16, 168, 52, 89, 236, 153,
        ],
        [
            124, 96, 7, 165, 215, 17, 206, 168, 223, 213, 217, 31, 89, 64, 236, 41, 199, 242, 0,
            254, 81, 30, 177, 252, 19, 151, 179, 103, 175, 60, 66, 249,
        ],
        [
            214, 82, 164, 215, 192, 236, 182, 139, 103, 54, 49, 107, 201, 76, 251, 33, 206, 125,
            15, 3, 201, 225, 233, 250, 106, 167, 56, 231, 239, 224, 86, 53,
        ],
    );
    if valid_identity && valid_contracts {
        Ok(registry)
    } else {
        Err(SafeV141RegistryError::InvalidGeneratedAsset)
    }
}

fn pinned_contract_matches(
    contract: &ReviewedContract,
    name: &str,
    address: [u8; 20],
    runtime_code_hash: [u8; 32],
    upstream_asset_sha256: [u8; 32],
) -> bool {
    contract.contract_name == name
        && contract.address == Address::new(address)
        && contract.runtime_code_hash == Hash::new(runtime_code_hash)
        && contract.upstream_asset_sha256 == Hash::new(upstream_asset_sha256)
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{REGISTRY_JSON, parse_registry_json};

    #[test]
    fn every_contract_identity_field_is_compile_time_pinned() {
        let original: Value = serde_json::from_str(REGISTRY_JSON).unwrap();
        for contract in [
            "singleton",
            "proxy_factory",
            "compatibility_fallback_handler",
        ] {
            for (field, replacement) in [
                ("contract_name", "Changed"),
                ("address", "0x1111111111111111111111111111111111111111"),
                (
                    "runtime_code_hash",
                    "0x1111111111111111111111111111111111111111111111111111111111111111",
                ),
                (
                    "upstream_asset_sha256",
                    "1111111111111111111111111111111111111111111111111111111111111111",
                ),
            ] {
                let mut mutated = original.clone();
                mutated["contracts"][contract][field] = Value::String(replacement.into());
                assert!(parse_registry_json(&mutated.to_string()).is_err());
            }
        }
    }

    #[test]
    fn every_registry_identity_field_is_compile_time_pinned() {
        let original: Value = serde_json::from_str(REGISTRY_JSON).unwrap();
        for (field, replacement) in [
            ("safe_version", "1.4.2"),
            (
                "smart_account_revision",
                "1111111111111111111111111111111111111111",
            ),
            (
                "smart_account_safe_source_sha256",
                "1111111111111111111111111111111111111111111111111111111111111111",
            ),
            (
                "deployment_revision",
                "1111111111111111111111111111111111111111",
            ),
        ] {
            let mut mutated = original.clone();
            mutated[field] = Value::String(replacement.into());
            assert!(parse_registry_json(&mutated.to_string()).is_err());
        }
        let mut mutated = original;
        mutated["schema_version"] = Value::from(3);
        assert!(parse_registry_json(&mutated.to_string()).is_err());
    }
}
