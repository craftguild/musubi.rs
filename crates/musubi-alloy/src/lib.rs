//! Alloy-specific provider and wallet adapters.
//!
//! Alloy may appear in this crate's public constructor surface, but owned runtime values are used
//! at every interface shared with generated SDKs.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod convert;
mod executor;
mod read;

pub use executor::{AlloyExecutor, ReceiptPolicy};
pub use read::AlloyRead;
