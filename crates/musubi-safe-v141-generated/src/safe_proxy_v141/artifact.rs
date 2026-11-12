//! Embedded artifact bytes and aggregate input/provenance identity.

use musubi_core::ExternalArtifactCoherence;
use musubi_core::{Bytes, ContractArtifact};
/// Canonical ABI JSON embedded in this crate.
pub const ABI_JSON: &[u8] = include_bytes!("../../assets/safe_proxy_v141/abi.json");
/// Explicit reviewed compiler-artifact capability class.
pub const ARTIFACT_KIND: musubi_core::ArtifactKind = musubi_core::ArtifactKind::Deployable;
/// Returns bytecode, identity, references, and unverified external coherence provenance.
#[must_use]
pub fn artifact() -> ContractArtifact {
    ContractArtifact { id: "safe_proxy_v141".into(), creation: musubi_core::CreationBytecode::new(Bytes::new(include_bytes!("../../assets/safe_proxy_v141/creation.bin").to_vec()), vec![]).expect("validated generated creation artifact"), runtime: Some(musubi_core::RuntimeBytecode::new(Bytes::new(include_bytes!("../../assets/safe_proxy_v141/runtime.bin").to_vec()), vec![], vec![]).expect("validated generated runtime artifact")), provenance: musubi_core::ArtifactProvenance { canonical_abi_sha256: musubi_core::FixedBytes::new([209, 16, 32, 246, 253, 239, 51, 177, 109, 14, 244, 37, 204, 242, 77, 174, 231, 169, 17, 237, 206, 110, 186, 174, 152, 205, 63, 61, 112, 116, 49, 142]), raw_abi_sha256: musubi_core::FixedBytes::new([209, 16, 32, 246, 253, 239, 51, 177, 109, 14, 244, 37, 204, 242, 77, 174, 231, 169, 17, 237, 206, 110, 186, 174, 152, 205, 63, 61, 112, 116, 49, 142]), creation_template_keccak256: musubi_core::FixedBytes::new([24, 86, 224, 238, 8, 57, 157, 116, 224, 234, 11, 3, 173, 202, 33, 10, 234, 222, 111, 116, 137, 105, 172, 2, 60, 220, 180, 221, 98, 220, 175, 95]), runtime_template_keccak256: Some(musubi_core::FixedBytes::new([215, 212, 8, 235, 205, 153, 178, 183, 11, 228, 62, 32, 37, 61, 109, 146, 168, 234, 143, 171, 41, 189, 59, 231, 245, 91, 16, 3, 35, 49, 251, 76])), ast_sha256: None, metadata_sha256: Some(musubi_core::FixedBytes::new([247, 35, 85, 220, 152, 116, 153, 105, 37, 135, 176, 66, 137, 125, 37, 176, 143, 146, 101, 58, 179, 92, 54, 61, 158, 180, 130, 29, 252, 58, 30, 12])), storage_layout_sha256: None, generator: crate::generator().clone(), runtime: crate::runtime_identity(), generated_abi_version: 2, external_artifact_coherence: ExternalArtifactCoherence::Unverified } }
}
