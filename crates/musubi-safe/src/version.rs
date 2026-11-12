//! Sealed Safe version marker separating incompatible protocol generations.

use musubi_core::Hash;

use crate::{SafeDomain, SafeTransaction, SafeV141Signature, hash};

mod sealed {
    pub trait Sealed {}
}

/// Protocol behavior implemented by one explicitly reviewed Safe version.
pub trait SafeVersionSpec: sealed::Sealed {
    /// Signature union accepted by this Safe version.
    type Signature;

    /// Human-readable Safe protocol version.
    const VERSION: &'static str;

    /// Computes the version-specific Safe transaction digest.
    #[must_use]
    fn transaction_hash(domain: SafeDomain, transaction: &SafeTransaction) -> Hash;
}

/// Marker for the reviewed Safe 1.4.1 protocol.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SafeV141;

impl sealed::Sealed for SafeV141 {}

impl SafeVersionSpec for SafeV141 {
    type Signature = SafeV141Signature;

    const VERSION: &'static str = "1.4.1";

    fn transaction_hash(domain: SafeDomain, transaction: &SafeTransaction) -> Hash {
        hash::transaction_hash(domain, transaction)
    }
}

impl SafeV141 {
    /// Computes a Safe 1.4.1 EIP-712 transaction hash.
    #[must_use]
    pub fn transaction_hash(domain: SafeDomain, transaction: &SafeTransaction) -> Hash {
        <Self as SafeVersionSpec>::transaction_hash(domain, transaction)
    }
}
