//! Deterministic generated crate tree and bounded top-level rendering.

use std::path::PathBuf;

use musubi_core::GeneratorInfo;

use crate::{
    Diagnostic, DiagnosticCode, GeneratedTree, GenerationIr, render_contract_tree::contract_tree,
    render_release::release_manifest, render_source::BoundedSource,
    render_typed_data::typed_data_tree,
};

/// Deterministic rendering failure.
#[derive(Clone, Debug, thiserror::Error)]
#[error("generated crate rendering failed")]
pub struct RenderError {
    /// Stable redacted diagnostic.
    pub diagnostic: Diagnostic,
}

/// Renders a complete standalone Cargo crate without writing to disk.
///
/// # Errors
///
/// Returns [`RenderError`] if runtime source or generator information is invalid, serialization fails, or the
/// generated tree violates path/count/size limits.
pub fn render(ir: &GenerationIr, generator: &GeneratorInfo) -> Result<GeneratedTree, RenderError> {
    ir.runtime
        .validate()
        .map_err(|_| render_error("runtime source is invalid"))?;
    generator
        .validate()
        .map_err(|_| render_error("generator information is invalid"))?;
    crate::render_validation::validate(ir, generator)?;
    let mut tree = GeneratedTree::with_limits(ir.limits);
    let cargo = cargo_manifest(ir, tree.remaining_bytes())?;
    tree.insert("Cargo.toml", cargo)?;
    let root = root_source(ir, generator, tree.remaining_bytes())?;
    tree.insert("src/lib.rs", root)?;
    for contract in &ir.contracts {
        contract_tree(&mut tree, contract)?;
        let assets = PathBuf::from("assets").join(&contract.id);
        tree.insert_slice(assets.join("abi.json"), &contract.canonical_abi)?;
        if let Some(creation) = &contract.creation {
            tree.insert_slice(assets.join("creation.bin"), creation.template.as_ref())?;
        }
        if let Some(runtime_bytecode) = &contract.runtime {
            tree.insert_slice(
                assets.join("runtime.bin"),
                runtime_bytecode.template.as_ref(),
            )?;
        }
    }
    for typed in &ir.typed_data {
        typed_data_tree(&mut tree, typed)?;
    }
    let release = release_manifest(ir, generator, tree.remaining_bytes())?;
    tree.insert("musubi-release.json", release)?;
    tree.seal()?;
    Ok(tree)
}

fn cargo_manifest(ir: &GenerationIr, maximum_bytes: usize) -> Result<String, RenderError> {
    let mut source = BoundedSource::new(maximum_bytes);
    source.formatted(format_args!(
        "[package]\nname = {:?}\nversion = {:?}\nedition = \"2024\"\nrust-version = \"1.95\"\npublish = false\n\n[workspace]\n\n[dependencies]\n{}\n{}\n",
        ir.package_name,
        ir.package_version,
        crate::runtime_source::cargo_dependency(&ir.runtime, "musubi-core"),
        crate::runtime_source::cargo_dependency(&ir.runtime, "musubi-runtime")
    ))?;
    Ok(source.finish())
}

fn root_source(
    ir: &GenerationIr,
    generator: &GeneratorInfo,
    maximum_bytes: usize,
) -> Result<String, RenderError> {
    let mut source = BoundedSource::new(maximum_bytes);
    source.push("//! Deterministically generated typed EVM contract SDK.\n\n#![forbid(unsafe_code)]\n#![warn(missing_docs)]\n\nmusubi_runtime::assert_generated_abi!(2);\n\n")?;
    source.formatted(format_args!(
        "/// Manifest schema used to configure this SDK.\n#[rustfmt::skip]\npub const MANIFEST_SCHEMA: &str = {:?};\n\n/// SHA-256 of the exact reviewed manifest bytes, including comments.\n#[rustfmt::skip]\npub const MANIFEST_SHA256: musubi_core::Sha256Digest = musubi_core::Sha256Digest::new({:?});\n\n/// Explicit dependency source; paths are mutable and relative to this SDK.\n#[rustfmt::skip]\npub fn runtime_identity() -> musubi_core::RuntimeSource {{ {} }}\n\n",
        crate::manifest::CONFIG_V4, ir.manifest_sha256.as_ref(), crate::runtime_source::core_expression(&ir.runtime)
    ))?;
    crate::render_generator::accessor(&mut source, generator)?;
    for contract in &ir.contracts {
        source.formatted(format_args!(
            "/// Typed SDK for manifest contract `{}`.\n#[rustfmt::skip]\npub mod {};\n",
            contract.id, contract.module
        ))?;
    }
    for typed in &ir.typed_data {
        source.formatted(format_args!(
            "/// Owned EIP-712 authority `{}`.\n#[rustfmt::skip]\npub mod {};\n",
            typed.id, typed.module
        ))?;
    }
    Ok(source.finish())
}

pub(crate) fn render_error(message: &str) -> RenderError {
    RenderError {
        diagnostic: Diagnostic::error(DiagnosticCode::InconsistentArtifact, message),
    }
}
