//! Fixture surface for downstream dependency resolution.

use musubi_core::Address;
use musubi_runtime::BackendErrorKind;

/// Names owned SDK and runtime types in a consumer graph with a newer Alloy dependency.
pub fn downstream_surface(address: Address) -> (String, BackendErrorKind) {
    (address.to_string(), BackendErrorKind::InvalidRequest)
}
