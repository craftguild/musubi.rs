//! Versioned Safe transaction, hashing, and signature support.
//!
//! Each Safe protocol version has a distinct marker and signature union so unsupported features
//! cannot leak into older deployments.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod hash;
mod operation;
mod registry;
mod signature;
mod transaction;
mod version;

#[cfg(test)]
mod operation_tests;
#[cfg(test)]
mod signature_tests;

pub use operation::{
    SafeEncodingError, approve_hash_request, exec_transaction_request, get_transaction_hash_call,
};
pub use registry::{
    DEPLOYMENT_REVISION, DeploymentContracts, ReviewedContract, SMART_ACCOUNT_REVISION,
    SMART_ACCOUNT_SAFE_SOURCE_SHA256, SafeV141Registry, SafeV141RegistryError, safe_v141_registry,
};
pub use signature::{
    EoaRecoveryId, PackedSignatures, SafeV141Signature, SignatureError, pack_signatures,
};
pub use transaction::{SafeDomain, SafeOperation, SafeTransaction};
pub use version::{SafeV141, SafeVersionSpec};
