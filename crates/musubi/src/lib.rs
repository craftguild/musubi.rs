//! Opt-in facade for Musubi.
//!
//! Core values and runtime traits are always available. Backend and Safe adapters are enabled by
//! the `alloy` and `safe` features respectively.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Alloy-independent values and artifact descriptors.
pub use musubi_core as core;
/// Runtime codecs, operations, and backend interfaces.
pub use musubi_runtime as runtime;

#[cfg(feature = "alloy")]
/// Alloy provider and transaction adapters.
pub use musubi_alloy as alloy;
#[cfg(feature = "safe")]
/// Versioned Safe protocol support.
pub use musubi_safe as safe;
