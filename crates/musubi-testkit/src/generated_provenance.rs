//! Independent reconstruction of the current generated Rust provenance boundary.

use musubi_core::{RuntimeSource, Sha256Digest};

pub(crate) fn root_source(
    schema: &str,
    digest: &Sha256Digest,
    runtime: &RuntimeSource,
    generator: &musubi_core::GeneratorInfo,
) -> String {
    let runtime = match runtime {
        RuntimeSource::Path { root } => {
            format!("musubi_core::RuntimeSource::Path {{ root: {root:?}.into() }}")
        }
        RuntimeSource::Git { url, revision } => format!(
            "musubi_core::RuntimeSource::Git {{ url: {url:?}.into(), revision: {revision:?}.into() }}"
        ),
    };
    format!(
        "/// Manifest schema used to configure this SDK.\n#[rustfmt::skip]\npub const MANIFEST_SCHEMA: &str = {schema:?};\n\n/// SHA-256 of the exact reviewed manifest bytes, including comments.\n#[rustfmt::skip]\npub const MANIFEST_SHA256: musubi_core::Sha256Digest = musubi_core::Sha256Digest::new({:?});\n\n/// Explicit dependency source; paths are mutable and relative to this SDK.\n#[rustfmt::skip]\npub fn runtime_identity() -> musubi_core::RuntimeSource {{ {runtime} }}\n\n",
        digest.as_ref()
    ) + &crate::generated_generator::source(generator)
}
