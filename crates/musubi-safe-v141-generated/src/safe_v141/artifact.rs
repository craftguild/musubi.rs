//! Embedded artifact bytes and aggregate input/provenance identity.

use musubi_core::ExternalArtifactCoherence;
use musubi_core::{Bytes, ContractArtifact};
/// Canonical ABI JSON embedded in this crate.
pub const ABI_JSON: &[u8] = include_bytes!("../../assets/safe_v141/abi.json");
/// Explicit reviewed compiler-artifact capability class.
pub const ARTIFACT_KIND: musubi_core::ArtifactKind = musubi_core::ArtifactKind::Deployable;
/// Returns bytecode, identity, references, and unverified external coherence provenance.
#[must_use]
pub fn artifact() -> ContractArtifact {
    ContractArtifact { id: "safe_v141".into(), creation: musubi_core::CreationBytecode::new(Bytes::new(include_bytes!("../../assets/safe_v141/creation.bin").to_vec()), vec![]).expect("validated generated creation artifact"), runtime: Some(musubi_core::RuntimeBytecode::new(Bytes::new(include_bytes!("../../assets/safe_v141/runtime.bin").to_vec()), vec![], vec![]).expect("validated generated runtime artifact")), provenance: musubi_core::ArtifactProvenance { canonical_abi_sha256: musubi_core::FixedBytes::new([32, 91, 8, 173, 226, 209, 240, 184, 120, 205, 243, 175, 83, 255, 22, 170, 94, 67, 180, 41, 104, 181, 184, 129, 180, 195, 14, 82, 126, 225, 7, 79]), raw_abi_sha256: musubi_core::FixedBytes::new([121, 73, 57, 171, 21, 226, 144, 224, 227, 20, 241, 163, 23, 3, 153, 113, 113, 97, 31, 240, 237, 3, 54, 7, 0, 174, 238, 83, 67, 147, 187, 196]), creation_template_keccak256: musubi_core::FixedBytes::new([189, 217, 227, 131, 58, 139, 223, 15, 107, 67, 18, 224, 123, 200, 81, 175, 194, 152, 213, 54, 144, 160, 10, 40, 202, 222, 93, 232, 182, 172, 15, 54]), runtime_template_keccak256: Some(musubi_core::FixedBytes::new([31, 226, 223, 133, 43, 163, 41, 157, 101, 52, 239, 65, 110, 239, 164, 6, 229, 108, 237, 153, 91, 202, 136, 106, 183, 165, 83, 230, 208, 197, 225, 196])), ast_sha256: None, metadata_sha256: Some(musubi_core::FixedBytes::new([89, 176, 37, 83, 67, 50, 234, 233, 209, 211, 0, 224, 201, 193, 36, 40, 48, 83, 125, 118, 47, 154, 150, 122, 30, 147, 197, 207, 63, 169, 255, 181])), storage_layout_sha256: None, generator: crate::generator().clone(), runtime: crate::runtime_identity(), generated_abi_version: 2, external_artifact_coherence: ExternalArtifactCoherence::Unverified } }
}
