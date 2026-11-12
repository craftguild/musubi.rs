//! Generated artifact, bytecode-reference, digest, and runtime identity source rendering.

use musubi_core::{ArtifactKind, FixedBytes};

use crate::{ContractIr, RenderError, render::render_error, render_source::BoundedSource};

pub(crate) fn render_artifact(
    source: &mut BoundedSource,
    contract: &ContractIr,
) -> Result<(), RenderError> {
    match contract.kind {
        ArtifactKind::Deployable => deployable_source(source, contract),
        ArtifactKind::Interface => interface_source(source, contract),
    }
}

fn deployable_source(source: &mut BoundedSource, contract: &ContractIr) -> Result<(), RenderError> {
    let creation = contract
        .creation
        .as_ref()
        .ok_or_else(|| render_error("deployable IR has no creation bytecode"))?;
    source.formatted(format_args!(
        "/// Canonical ABI JSON embedded in this crate.\npub const ABI_JSON: &[u8] = include_bytes!(\"../../assets/{0}/abi.json\");\n/// Explicit reviewed compiler-artifact capability class.\npub const ARTIFACT_KIND: musubi_core::ArtifactKind = musubi_core::ArtifactKind::Deployable;\n/// Returns bytecode, identity, references, and unverified external coherence provenance.\n#[must_use]\npub fn artifact() -> ContractArtifact {{\n    ContractArtifact {{ id: {0:?}.into(), creation: musubi_core::CreationBytecode::new(Bytes::new(include_bytes!(\"../../assets/{0}/creation.bin\").to_vec()), ",
        contract.id
    ))?;
    links_expr(source, &creation.links)?;
    source.push(").expect(\"validated generated creation artifact\"), runtime: ")?;
    if let Some(runtime_bytecode) = &contract.runtime {
        source.formatted(format_args!("Some(musubi_core::RuntimeBytecode::new(Bytes::new(include_bytes!(\"../../assets/{}/runtime.bin\").to_vec()), ", contract.id))?;
        links_expr(source, &runtime_bytecode.links)?;
        source.push(", ")?;
        immutables_expr(source, &runtime_bytecode.immutables)?;
        source.push(").expect(\"validated generated runtime artifact\"))")?;
    } else {
        source.push("None")?;
    }
    source.formatted(format_args!(
        ", provenance: musubi_core::ArtifactProvenance {{ canonical_abi_sha256: musubi_core::FixedBytes::new({:?}), raw_abi_sha256: musubi_core::FixedBytes::new({:?}), creation_template_keccak256: musubi_core::FixedBytes::new({:?}), runtime_template_keccak256: ",
        contract.canonical_abi_sha256.into_array(),
        contract.raw_abi_sha256.into_array(),
        creation.template_hash().into_array()
    ))?;
    option_hash(
        source,
        contract
            .runtime
            .as_ref()
            .map(musubi_core::RuntimeBytecode::template_hash),
    )?;
    source.push(", ast_sha256: ")?;
    option_hash(source, contract.auxiliary_digests.ast)?;
    source.push(", metadata_sha256: ")?;
    option_hash(source, contract.auxiliary_digests.metadata)?;
    source.push(", storage_layout_sha256: ")?;
    option_hash(source, contract.auxiliary_digests.storage_layout)?;
    source.formatted(format_args!(
        ", generator: crate::generator().clone(), runtime: crate::runtime_identity(), generated_abi_version: 2, external_artifact_coherence: ExternalArtifactCoherence::Unverified }} }}\n}}\n"
    ))
}

fn interface_source(source: &mut BoundedSource, contract: &ContractIr) -> Result<(), RenderError> {
    source.formatted(format_args!(
        "/// Canonical ABI JSON embedded in this crate.\npub const ABI_JSON: &[u8] = include_bytes!(\"../../assets/{0}/abi.json\");\n/// Explicit reviewed compiler-artifact capability class.\npub const ARTIFACT_KIND: musubi_core::ArtifactKind = musubi_core::ArtifactKind::Interface;\n/// Returns ABI identity and unverified external coherence provenance.\n#[must_use]\npub fn interface_artifact() -> musubi_core::InterfaceArtifact {{\n    musubi_core::InterfaceArtifact {{ id: {0:?}.into(), canonical_abi_sha256: musubi_core::FixedBytes::new({1:?}), raw_abi_sha256: musubi_core::FixedBytes::new({2:?}), ast_sha256: ",
        contract.id,
        contract.canonical_abi_sha256.into_array(),
        contract.raw_abi_sha256.into_array()
    ))?;
    option_hash(source, contract.auxiliary_digests.ast)?;
    source.push(", metadata_sha256: ")?;
    option_hash(source, contract.auxiliary_digests.metadata)?;
    source.formatted(format_args!(
        ", generator: crate::generator().clone(), runtime: crate::runtime_identity(), generated_abi_version: 2, external_artifact_coherence: ExternalArtifactCoherence::Unverified }}\n}}\n"
    ))
}

fn links_expr(
    source: &mut BoundedSource,
    links: &[musubi_core::LinkReference],
) -> Result<(), RenderError> {
    source.push("vec![")?;
    for (index, link) in links.iter().enumerate() {
        separator(source, index)?;
        source.formatted(format_args!(
            "musubi_core::LinkReference {{ source: {:?}.into(), library: {:?}.into(), ranges: ",
            link.source, link.library
        ))?;
        ranges_expr(source, &link.ranges)?;
        source.push(" }")?;
    }
    source.push("]")
}

fn immutables_expr(
    source: &mut BoundedSource,
    values: &[musubi_core::ImmutableReference],
) -> Result<(), RenderError> {
    source.push("vec![")?;
    for (index, value) in values.iter().enumerate() {
        separator(source, index)?;
        source.formatted(format_args!(
            "musubi_core::ImmutableReference {{ id: {:?}.into(), ranges: ",
            value.id
        ))?;
        ranges_expr(source, &value.ranges)?;
        source.push(" }")?;
    }
    source.push("]")
}

fn ranges_expr(
    source: &mut BoundedSource,
    values: &[musubi_core::BytecodeRange],
) -> Result<(), RenderError> {
    source.push("vec![")?;
    for (index, value) in values.iter().enumerate() {
        separator(source, index)?;
        source.formatted(format_args!(
            "musubi_core::BytecodeRange {{ start: {}, length: {} }}",
            value.start, value.length
        ))?;
    }
    source.push("]")
}

fn option_hash(
    source: &mut BoundedSource,
    value: Option<FixedBytes<32>>,
) -> Result<(), RenderError> {
    match value {
        Some(hash) => source.formatted(format_args!(
            "Some(musubi_core::FixedBytes::new({:?}))",
            hash.into_array()
        )),
        None => source.push("None"),
    }
}

fn separator(source: &mut BoundedSource, index: usize) -> Result<(), RenderError> {
    if index != 0 {
        source.push(", ")?;
    }
    Ok(())
}
