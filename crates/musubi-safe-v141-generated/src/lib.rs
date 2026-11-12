//! Deterministically generated typed EVM contract SDK.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

musubi_runtime::assert_generated_abi!(2);

/// Manifest schema used to configure this SDK.
#[rustfmt::skip]
pub const MANIFEST_SCHEMA: &str = "musubi/config-v4";

/// SHA-256 of the exact reviewed manifest bytes, including comments.
#[rustfmt::skip]
pub const MANIFEST_SHA256: musubi_core::Sha256Digest = musubi_core::Sha256Digest::new([162, 151, 49, 233, 173, 61, 31, 75, 114, 28, 29, 25, 37, 94, 16, 182, 8, 220, 222, 68, 151, 112, 112, 135, 150, 45, 130, 143, 202, 247, 97, 172]);

/// Explicit dependency source; paths are mutable and relative to this SDK.
#[rustfmt::skip]
pub fn runtime_identity() -> musubi_core::RuntimeSource { musubi_core::RuntimeSource::Path { root: "../../.".into() } }

/// Returns the generator's recorded build observations without a connection.
///
/// This is not authentication. A hash-dirty revision does not uniquely identify
/// uncommitted contents; unknown provides no source identity. Runtime source is separate.
#[rustfmt::skip]
pub fn generator() -> &'static musubi_core::GeneratorInfo {
    static INFO: musubi_core::GeneratorInfo = musubi_core::GeneratorInfo { schema_version: ::std::borrow::Cow::Borrowed("musubi/generator-info-v1"), version: ::std::borrow::Cow::Borrowed("0.1.0"), revision: ::std::borrow::Cow::Borrowed("cb83be7fabee3c4828c7122e6831177df2faae36-dirty"), git_commit: Some(::std::borrow::Cow::Borrowed("cb83be7fabee3c4828c7122e6831177df2faae36")), dirty: Some(true), build: musubi_core::GeneratorBuild { rustc: ::std::borrow::Cow::Borrowed("rustc 1.95.0 (59807616e 2026-04-14)"), host: ::std::borrow::Cow::Borrowed("aarch64-apple-darwin"), target: ::std::borrow::Cow::Borrowed("aarch64-apple-darwin"), profile: ::std::borrow::Cow::Borrowed("debug") } };
    &INFO
}

/// Typed SDK for manifest contract `safe_proxy_v141`.
#[rustfmt::skip]
pub mod safe_proxy_v141;
/// Typed SDK for manifest contract `safe_v141`.
#[rustfmt::skip]
pub mod safe_v141;
/// Owned EIP-712 authority `safe_v141_hash`.
#[rustfmt::skip]
pub mod safe_v141_typed_data;
